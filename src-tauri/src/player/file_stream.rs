use std::fs::File;
use std::mem;
use std::path::PathBuf;

use log::{trace, warn};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::meta::MetadataRevision;
use symphonia::core::units::TimeBase;
use symphonia::core::{io::MediaSourceStream, probe::Hint};

use super::decode_worker::{
    DecodeWorker, DecodeWorkerToFileStreamMessage, DecodedBlock, FileStreamToDecodeWorkerMessage,
};
use super::errors::FileStreamOpenError;

const MESSAGE_BUFFER_SIZE: usize = 16384;
const MIN_BLOCK_SIZE: usize = 1024;

pub struct ReadData<'a> {
    data: &'a Vec<Vec<f32>>,
    len: usize,
    reached_end_of_file: bool,
}

impl<'a> ReadData<'a> {
    pub fn new(data: &'a Vec<Vec<f32>>, len: usize, reached_end_of_file: bool) -> Self {
        Self {
            data,
            len,
            reached_end_of_file,
        }
    }

    pub fn read_channel(&self, channel: usize) -> &[f32] {
        &self.data[channel][0..self.len]
    }

    pub fn num_channels(&self) -> usize {
        self.data.len()
    }

    pub fn num_frames(&self) -> usize {
        self.len
    }

    pub fn reached_end_of_file(&self) -> bool {
        self.reached_end_of_file
    }
}

const READ_BUFFER_SIZE: usize = 16384;

pub struct FileStream {
    message_consumer: rtrb::Consumer<DecodeWorkerToFileStreamMessage>,
    message_producer: rtrb::Producer<FileStreamToDecodeWorkerMessage>,
    blocks: Option<Box<DecodedBlock>>,
    playhead: usize,
    read_buffer: Vec<Vec<f32>>,
    stream_id: u32,
    n_frames: Option<u64>,
    time_base: Option<TimeBase>,
    metadata: Option<MetadataRevision>,
}

pub struct FileStreamMetadata {
    time_base: Option<TimeBase>,
    preferred_metadata: Option<MetadataRevision>,
}

impl FileStream {
    pub fn open<P>(file_path: P, target_sample_rate: u32) -> Result<Self, FileStreamOpenError>
    where
        P: Into<PathBuf>,
    {
        let file: PathBuf = file_path.into();
        let mut hint = Hint::new();
        if let Some(extension) = file.extension() {
            hint.with_extension(extension.to_str().unwrap());
        }

        let source = Box::new(File::open(file.clone())?);
        let mss = MediaSourceStream::new(source, Default::default());

        let mut probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &Default::default(),
            &Default::default(),
        )?;

        let mut reader = probed.format;
        let track = reader
            .default_track()
            .ok_or(FileStreamOpenError::NoTrackFound)?;
        let track_id = track.id;
        let n_frames = track.codec_params.n_frames;
        let time_base = track.codec_params.time_base;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions { verify: false })?;

        let decoded = loop {
            let packet = match reader.next_packet() {
                Ok(packet) => packet,
                Err(err) => break Err(err),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Err(symphonia::core::errors::Error::DecodeError(err)) => {
                    // Decode errors are not fatal. Print the error message and try to decode the next
                    // packet as usual.
                    warn!("decode error: {}", err);
                }
                Err(err) => break Err(err),
                Ok(decoded) => break Ok(decoded),
            }
        }?;

        trace!("First packet decoded frames: {}", decoded.frames());

        let spec = decoded.spec();
        let sample_rate = spec.rate;
        let block_size = decoded.capacity().max(MIN_BLOCK_SIZE);
        let num_channels = spec.channels.count();

        // Prefer metadata that's provided in the container format over other tags found during the
        // probe operation.
        let metadata = reader.metadata().current().cloned().or_else(|| {
            probed
                .metadata
                .get()
                .as_ref()
                .and_then(|metadata| metadata.current().cloned())
        });

        let (from_worker_producer, from_worker_consumer) =
            rtrb::RingBuffer::new(MESSAGE_BUFFER_SIZE);
        let (to_worker_producer, to_worker_consumer) = rtrb::RingBuffer::new(MESSAGE_BUFFER_SIZE);
        let worker = DecodeWorker::new(
            target_sample_rate,
            num_channels,
            block_size,
            sample_rate,
            track_id,
            reader,
            decoder,
            from_worker_producer,
            to_worker_consumer,
        );

        std::thread::spawn(move || {
            trace!("Starting decode worker for {file:?}");
            worker.run();
        });

        Ok(Self {
            message_consumer: from_worker_consumer,
            message_producer: to_worker_producer,
            blocks: None,
            playhead: 0,
            read_buffer: vec![vec![0.0; READ_BUFFER_SIZE]; num_channels],
            stream_id: 0,
            n_frames,
            time_base,
            metadata,
        })
    }

    pub fn n_frames(&self) -> Option<u64> {
        self.n_frames
    }

    pub fn time_base(&self) -> Option<&TimeBase> {
        self.time_base.as_ref()
    }

    pub fn playhead(&self) -> usize {
        self.playhead
    }

    pub fn is_ready(&mut self) -> bool {
        self.poll();
        self.blocks.is_some()
    }

    pub fn metadata(&self) -> Option<&MetadataRevision> {
        self.metadata.as_ref()
    }

    pub fn read(&mut self, frames: usize) -> Option<ReadData> {
        self.poll();
        if let Some(mut block) = self.blocks.as_mut() {
            let mut source_frames_read: usize = 0;
            let mut frames_read: usize = 0;
            let mut is_eof = false;
            let frames_to_read = frames.min(READ_BUFFER_SIZE);
            while frames_read < frames_to_read {
                let available_in_block = block.num_frames - block.playhead;
                let read_from_block = available_in_block.min(frames_to_read - frames_read);

                for (i, channel) in self.read_buffer.iter_mut().enumerate() {
                    channel[frames_read..frames_read + read_from_block].copy_from_slice(
                        &block.samples[i][block.playhead..block.playhead + read_from_block],
                    );
                }

                block.playhead += read_from_block;
                frames_read += read_from_block;
                source_frames_read += (read_from_block as f64 / block.resample_ratio) as usize;

                if read_from_block == available_in_block {
                    is_eof = block.is_eof;
                    if block.next.is_none() {
                        break;
                    }
                    block = block.next.as_mut().unwrap();
                } else {
                    break;
                }
            }
            while let Some(block) = self.blocks.as_mut() {
                // Empty EOF marker block needs to be preserved
                if block.playhead == block.num_frames && (!block.is_eof || is_eof) {
                    let next = block.next.take();
                    block.len = 1;
                    let replaced = mem::replace(&mut self.blocks, next);
                    let _ =
                        self.message_producer
                            .push(FileStreamToDecodeWorkerMessage::DisposeBlock(
                                replaced.unwrap(),
                            ));
                } else {
                    break;
                }
            }
            self.playhead += source_frames_read;
            Some(ReadData::new(&self.read_buffer, frames_read, is_eof))
        } else {
            None
        }
    }

    pub fn seek(&mut self, seek_to: usize) {
        self.poll();
        self.stream_id += 1;
        if let Ok(()) = self
            .message_producer
            .push(FileStreamToDecodeWorkerMessage::Seek(
                seek_to,
                self.stream_id,
            ))
        {
            self.playhead = seek_to;
            if let Some(block) = self.blocks.take() {
                let _ = self
                    .message_producer
                    .push(FileStreamToDecodeWorkerMessage::DisposeBlock(block));
            }
            assert!(self.blocks.is_none());
        }
    }

    fn poll(&mut self) {
        while let Ok(message) = self.message_consumer.pop() {
            match message {
                DecodeWorkerToFileStreamMessage::Block(decoded) => {
                    if decoded.stream_id != self.stream_id {
                        trace!("Ignoring block for old stream wih id {}", decoded.stream_id);
                        continue;
                    }
                    if let Some(mut block) = self.blocks.as_mut() {
                        let last_block = loop {
                            block.len += 1;
                            if block.next.is_none() {
                                break block;
                            }
                            block = block.next.as_mut().unwrap();
                        };
                        last_block.next = Some(decoded);
                    } else {
                        self.blocks = Some(decoded);
                    }
                }
            }
        }
    }
}

impl Drop for FileStream {
    fn drop(&mut self) {
        if let Some(block) = self.blocks.take() {
            let _ = self
                .message_producer
                .push(FileStreamToDecodeWorkerMessage::DisposeBlock(block));
        }
        let _ = self
            .message_producer
            .push(FileStreamToDecodeWorkerMessage::Done(
                self.read_buffer.drain(..).collect(),
            ));
    }
}

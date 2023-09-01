use std::fs::File;
use std::mem;
use std::ops::Range;
use std::path::PathBuf;

use log::{error, trace, warn};
use rubato::{FftFixedIn, Resampler};
use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::IntoSample;
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::sample::Sample;
use symphonia::core::{codecs::Decoder, formats::FormatReader, io::MediaSourceStream, probe::Hint};

use super::errors::FileStreamOpenError;

const MESSAGE_BUFFER_SIZE: usize = 16384;
const MIN_BLOCK_SIZE: usize = 1024;

fn convert_samples_any(
    input: &AudioBufferRef<'_>,
    output: &mut [Vec<f32>],
    input_range: Range<usize>,
) {
    match input {
        AudioBufferRef::U8(input) => convert_samples(input, output, input_range),
        AudioBufferRef::U16(input) => convert_samples(input, output, input_range),
        AudioBufferRef::U24(input) => convert_samples(input, output, input_range),
        AudioBufferRef::U32(input) => convert_samples(input, output, input_range),
        AudioBufferRef::S8(input) => convert_samples(input, output, input_range),
        AudioBufferRef::S16(input) => convert_samples(input, output, input_range),
        AudioBufferRef::S24(input) => convert_samples(input, output, input_range),
        AudioBufferRef::S32(input) => convert_samples(input, output, input_range),
        AudioBufferRef::F32(input) => convert_samples(input, output, input_range),
        AudioBufferRef::F64(input) => convert_samples(input, output, input_range),
    }
}

fn convert_samples<S>(input: &AudioBuffer<S>, output: &mut [Vec<f32>], input_range: Range<usize>)
where
    S: Sample + IntoSample<f32> + Sized,
{
    for (c, dst) in output.iter_mut().enumerate() {
        let src = input.chan(c);
        dst.extend(src[input_range.clone()].iter().map(|&s| s.into_sample()));
    }
}

#[derive(Debug, Clone)]
struct DecodedBlock {
    samples: Vec<Vec<f32>>,
    num_frames: usize,
    playhead: usize,
    is_eof: bool,
    resample_ratio: f64,
    stream_id: u32,
    next: Option<Box<DecodedBlock>>,
}

enum DecodeWorkerToFileStreamMessage {
    Block(Box<DecodedBlock>),
}

enum FileStreamToDecodeWorkerMessage {
    DisposeBlock(Box<DecodedBlock>),
    Seek(usize, u32),
    Done(Vec<Vec<f32>>),
}

struct DecodeWorker {
    message_producer: rtrb::Producer<DecodeWorkerToFileStreamMessage>,
    message_consumer: rtrb::Consumer<FileStreamToDecodeWorkerMessage>,
    // TODO: Needs to be SampleBuffer?
    input_buffer: Vec<Vec<f32>>,
    output_buffer: Vec<Vec<f32>>,
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    resampler: Option<FftFixedIn<f32>>,
    block_size: usize,
    track_id: u32,
    resample_ratio: f64,
    stream_id: u32,
}

impl DecodeWorker {
    // TODO: Fix clippy warning
    #[allow(clippy::too_many_arguments)]
    fn new(
        target_sample_rate: u32,
        num_channels: usize,
        block_size: usize,
        sample_rate: u32,
        track_id: u32,
        reader: Box<dyn FormatReader>,
        decoder: Box<dyn Decoder>,
        message_producer: rtrb::Producer<DecodeWorkerToFileStreamMessage>,
        message_consumer: rtrb::Consumer<FileStreamToDecodeWorkerMessage>,
    ) -> Self {
        let resample_ratio = target_sample_rate as f64 / sample_rate as f64;
        let mut maybe_resampler = if sample_rate != target_sample_rate {
            Some(
                FftFixedIn::new(
                    sample_rate as usize,
                    target_sample_rate as usize,
                    block_size,
                    2,
                    num_channels,
                )
                .expect("Failed to create resampler"),
            )
        } else {
            None
        };
        let mut input_buffer =
            Vec::from_iter((0..num_channels).map(|_| Vec::with_capacity(block_size)));
        let output_buffer = if let Some(resampler) = maybe_resampler.as_mut() {
            resampler.output_buffer_allocate(true)
        } else {
            input_buffer.clone()
        };
        if decoder.last_decoded().frames() > 0 {
            convert_samples_any(
                &decoder.last_decoded(),
                &mut input_buffer,
                0..decoder.last_decoded().frames(),
            );
        }
        DecodeWorker {
            resample_ratio,
            message_producer,
            message_consumer,
            output_buffer,
            input_buffer,
            reader,
            decoder,
            resampler: maybe_resampler,
            block_size,
            track_id,
            stream_id: 0,
        }
    }

    fn run(mut self) {
        let mut is_eof = false;
        let mut seek_delta: usize = 0;
        let result: symphonia::core::errors::Result<()> = loop {
            let mut is_done = false;
            let poll_result = loop {
                if let Ok(msg) = self.message_consumer.pop() {
                    match msg {
                        FileStreamToDecodeWorkerMessage::Done(_) => {
                            // trace!("Done message received");
                            is_done = true;
                            break Ok(());
                        }
                        FileStreamToDecodeWorkerMessage::DisposeBlock(_block) => {
                            // trace!("Disposing block with length {}", block.samples[0].len());
                        }
                        FileStreamToDecodeWorkerMessage::Seek(seek_to, stream_id) => {
                            match self.reader.seek(
                                SeekMode::Accurate,
                                SeekTo::TimeStamp {
                                    ts: seek_to as u64,
                                    track_id: self.track_id,
                                },
                            ) {
                                Err(e) => {
                                    break Err(e);
                                }
                                Ok(seeked_to) => {
                                    self.stream_id = stream_id;
                                    is_eof = false;
                                    seek_delta =
                                        (seeked_to.required_ts - seeked_to.actual_ts) as usize;
                                    trace!("Found seek delta of {}", seek_delta);
                                    self.decoder.reset();
                                    for channel in self.input_buffer.iter_mut() {
                                        channel.clear();
                                    }
                                }
                            }
                        }
                    }
                } else {
                    break Ok(());
                }
            };

            if poll_result.is_err() {
                break poll_result;
            }

            if is_done {
                break Ok(());
            }

            if is_eof || self.message_producer.is_full() {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }

            let packet = match self.reader.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof
                        && err.to_string() == "end of stream" =>
                {
                    let num_frames = self.input_buffer[0].len();
                    for channel in self.input_buffer.iter_mut() {
                        channel.resize(self.block_size, 0.0);
                        trace!("Resizing channel to {}", channel.len());
                    }
                    let samples = if let Some(resampler) = self.resampler.as_mut() {
                        resampler
                            .process_into_buffer(&self.input_buffer, &mut self.output_buffer, None)
                            .expect("Failed to resample");
                        self.output_buffer.clone()
                    } else {
                        self.input_buffer.clone()
                    };
                    for channel in self.input_buffer.iter_mut() {
                        channel.clear();
                    }
                    self.message_producer
                        .push(DecodeWorkerToFileStreamMessage::Block(Box::new(
                            DecodedBlock {
                                num_frames: (num_frames as f64 * self.resample_ratio) as usize,
                                samples,
                                resample_ratio: self.resample_ratio,
                                stream_id: self.stream_id,
                                playhead: 0,
                                is_eof: true,
                                next: None,
                            },
                        )))
                        .unwrap();
                    is_eof = true;
                    continue;
                }
                Err(err) => {
                    break Err(err);
                }
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let input_num_frames = decoded.frames();
                    if input_num_frames == 0 {
                        continue;
                    }
                    if input_num_frames < seek_delta {
                        trace!(
                            "Block of {} frames skipped for seek delta",
                            input_num_frames
                        );
                        seek_delta -= input_num_frames;
                        continue;
                    }
                    let input_frames_consumed = input_num_frames - seek_delta;
                    let mut input_offset = seek_delta;
                    if seek_delta > 0 {
                        trace!("Recuperated remaining {} frames of seek delta", seek_delta);
                    }

                    seek_delta = 0;
                    if input_frames_consumed + self.input_buffer[0].len() >= self.block_size {
                        let consume_samples = self.block_size - self.input_buffer[0].len();
                        convert_samples_any(
                            &decoded,
                            self.input_buffer.as_mut_slice(),
                            input_offset..consume_samples,
                        );
                        input_offset = consume_samples;
                        let mut output_num_frames = self.block_size;
                        let samples = if let Some(resampler) = self.resampler.as_mut() {
                            output_num_frames = resampler.output_frames_next();
                            resampler
                                .process_into_buffer(
                                    &self.input_buffer,
                                    &mut self.output_buffer,
                                    None,
                                )
                                .expect("Failed to resample");
                            self.output_buffer.clone()
                        } else {
                            self.input_buffer.clone()
                        };
                        for channel in self.input_buffer.iter_mut() {
                            channel.clear();
                        }
                        self.message_producer
                            .push(DecodeWorkerToFileStreamMessage::Block(Box::new(
                                DecodedBlock {
                                    samples,
                                    stream_id: self.stream_id,
                                    num_frames: output_num_frames,
                                    playhead: 0,
                                    is_eof: false,
                                    next: None,
                                    resample_ratio: self.resample_ratio,
                                },
                            )))
                            .unwrap();
                    }
                    convert_samples_any(
                        &decoded,
                        self.input_buffer.as_mut_slice(),
                        input_offset..input_num_frames,
                    );
                }
                Err(symphonia::core::errors::Error::DecodeError(err)) => {
                    warn!("decode error: {}", err)
                }
                Err(err) => break Err(err),
            }
        };
        if let Err(e) = result {
            error!("DecodeWorker error: {}", e);
        }
    }
}

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

        let source = Box::new(File::open(file)?);
        let mss = MediaSourceStream::new(source, Default::default());

        let probed = symphonia::default::get_probe().format(
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

        trace!("First decoded frames: {}", decoded.frames());

        let spec = decoded.spec();
        let sample_rate = spec.rate;
        let block_size = decoded.capacity().max(MIN_BLOCK_SIZE);
        let num_channels = spec.channels.count();

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
        })
    }

    pub fn n_frames(&self) -> Option<u64> {
        self.n_frames
    }

    pub fn playhead(&self) -> usize {
        self.playhead
    }

    pub fn is_ready(&mut self) -> bool {
        self.poll();
        self.blocks.is_some()
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
                if block.playhead == block.num_frames {
                    let next = block.next.take();
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
        // TODO: Don't deallocate on audio thread
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
                        continue;
                    }
                    if let Some(mut block) = self.blocks.as_mut() {
                        let last_block = loop {
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
        let _ = self
            .message_producer
            .push(FileStreamToDecodeWorkerMessage::Done(
                self.read_buffer.drain(..).collect(),
            ));
    }
}

use std::fs::File;
use std::ops::Range;
use std::path::PathBuf;

use log::{error, trace, warn};
use rubato::{FftFixedIn, VecResampler};
use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::IntoSample;
use symphonia::core::sample::Sample;
use symphonia::core::{codecs::Decoder, formats::FormatReader, io::MediaSourceStream, probe::Hint};

use super::errors::FileStreamOpenError;

const MESSAGE_BUFFER_SIZE: usize = 16384;

struct FileStreamServer {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
}

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
    num_channels: usize,
    num_frames: usize,
    start_frame: usize,
    playhead: usize,
    is_eof: bool,
    next: Option<Box<DecodedBlock>>,
}

enum DecodeWorkerToFileStreamMessage {
    Block(Box<DecodedBlock>),
}

struct DecodeWorker {
    message_producer: rtrb::Producer<DecodeWorkerToFileStreamMessage>,
    // TODO: Needs to be SampleBuffer?
    input_buffer: Vec<Vec<f32>>,
    output_buffer: Vec<Vec<f32>>,
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    resampler: Option<FftFixedIn<f32>>,
    playhead: usize,
    num_channels: usize,
    block_size: usize,
    track_id: u32,
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
    ) -> Self {
        let mut maybe_resampler = if sample_rate != target_sample_rate {
            Some(
                FftFixedIn::new(
                    sample_rate as usize,
                    target_sample_rate as usize,
                    block_size,
                    2,
                    num_channels,
                )
                .expect("Faile to create resampler"),
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
            trace!("Initial output buffer len: {}", output_buffer[0].len());
        }
        DecodeWorker {
            message_producer,
            input_buffer,
            output_buffer,
            reader,
            decoder,
            resampler: maybe_resampler,
            playhead: 0,
            num_channels,
            block_size,
            track_id,
        }
    }

    fn run(mut self) {
        let res = loop {
            if self.message_producer.is_full() {
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
                                samples,
                                num_frames: samples[0].len(),
                                num_channels: self.num_channels,
                                start_frame: self.playhead,
                                playhead: 0,
                                is_eof: true,
                                next: None,
                            },
                        )))
                        .unwrap();
                    break Ok(());
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
                    let num_frames = decoded.frames();
                    if num_frames == 0 {
                        continue;
                    }
                    let consume_samples =
                        (self.block_size - self.input_buffer[0].len()).min(num_frames);
                    if num_frames + self.input_buffer[0].len() >= self.block_size {
                        convert_samples_any(
                            &decoded,
                            self.input_buffer.as_mut_slice(),
                            0..consume_samples,
                        );
                        let samples = if let Some(resampler) = self.resampler.as_mut() {
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
                                    num_channels: self.num_channels,
                                    num_frames,
                                    start_frame: self.playhead,
                                    playhead: 0,
                                    is_eof: true,
                                    next: None,
                                },
                            )))
                            .unwrap();
                        self.playhead += self.block_size;
                    }
                    convert_samples_any(
                        &decoded,
                        self.input_buffer.as_mut_slice(),
                        consume_samples..num_frames,
                    );
                }
                Err(symphonia::core::errors::Error::DecodeError(err)) => {
                    warn!("decode error: {}", err)
                }
                Err(err) => break Err(err),
            }
        };
        if let Err(e) = res {
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
    blocks: Option<Box<DecodedBlock>>,
    playhead: usize,
    read_buffer: Vec<Vec<f32>>,
    num_channels: usize,
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
        let block_size = decoded.capacity();
        let num_channels = spec.channels.count();

        let (message_producer, message_consumer) = rtrb::RingBuffer::new(MESSAGE_BUFFER_SIZE);
        let worker = DecodeWorker::new(
            target_sample_rate,
            num_channels,
            block_size,
            sample_rate,
            track_id,
            reader,
            decoder,
            message_producer,
        );

        std::thread::spawn(move || {
            worker.run();
        });

        Ok(Self {
            message_consumer,
            blocks: None,
            playhead: 0,
            read_buffer: vec![vec![0.0; READ_BUFFER_SIZE]; num_channels],
            num_channels,
        })
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
                    // TODO: Don't deallocate on audio thread
                    self.blocks = block.next.take();
                } else {
                    break;
                }
            }
            if frames_read > 0 {
                self.playhead += frames_read;
                Some(ReadData::new(&self.read_buffer, frames_read, is_eof))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn poll(&mut self) {
        while let Ok(message) = self.message_consumer.pop() {
            match message {
                DecodeWorkerToFileStreamMessage::Block(decoded) => {
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

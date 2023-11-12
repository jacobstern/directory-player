use std::ops::Range;

use log::{error, trace, warn};
use rubato::{FftFixedIn, Resampler};
use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::Decoder;
use symphonia::core::conv::IntoSample;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::sample::Sample;

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
pub struct DecodedBlock {
    pub samples: Vec<Vec<f32>>,
    pub num_frames: usize,
    pub playhead: usize,
    pub is_eof: bool,
    pub resample_ratio: f64,
    pub stream_id: u32,
    pub next: Option<Box<DecodedBlock>>,
    /// Number of blocks in the linked list
    pub len: usize,
}

impl Drop for DecodedBlock {
    fn drop(&mut self) {
        // Avoid stack overflow for large linked list
        let mut curr = self.next.take();
        while let Some(mut next) = curr {
            curr = next.next.take();
        }
    }
}

pub enum DecodeWorkerToFileStreamMessage {
    Block(Box<DecodedBlock>),
}

pub enum FileStreamToDecodeWorkerMessage {
    DisposeBlock(Box<DecodedBlock>),
    Seek(usize, u32),
    Done(Vec<Vec<f32>>),
}

pub struct DecodeWorker {
    message_producer: rtrb::Producer<DecodeWorkerToFileStreamMessage>,
    message_consumer: rtrb::Consumer<FileStreamToDecodeWorkerMessage>,
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
    pub fn new(
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
            trace!("Will resample from {sample_rate} to {target_sample_rate}");
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
            trace!("Will not resample");
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

    pub fn run(mut self) {
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
                        FileStreamToDecodeWorkerMessage::DisposeBlock(block) => {
                            if block.len > 1 {
                                trace!("Disposing block with len {}", block.len);
                            }
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
                                    if let Some(resampler) = self.resampler.as_mut() {
                                        resampler.reset();
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
                    trace!("Resizing buffer from {} to {}", num_frames, self.block_size);
                    for channel in self.input_buffer.iter_mut() {
                        channel.resize(self.block_size, 0.0);
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
                                len: 1,
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
                trace!("Ignoring packet with wrong track_id");
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    // TODO: Copy interleaved samples to output directly

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
                                    len: 1,
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

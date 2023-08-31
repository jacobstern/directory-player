use std::fs::File;
use std::path::PathBuf;

use log::{error, warn};
use symphonia::core::{
    audio::{AudioBufferRef, SampleBuffer},
    codecs::{Decoder, DecoderOptions},
    formats::{FormatReader, Packet},
    io::MediaSourceStream,
    probe::Hint,
};

use super::{errors::FileStreamOpenError, resampler::Resampler};

const MESSAGE_BUFFER_SIZE: usize = 16384;

struct FileStreamServer {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
}

struct DecodedBlock {
    planar_samples: Vec<f32>,
    num_channels: usize,
    packet_info: PacketInfo,
    next: Option<Box<DecodedBlock>>,
}

enum DecodeWorkerToFileStreamMessage {
    Block(Box<DecodedBlock>),
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PacketInfo {
    ts: u64,
    dur: u64,
    trim_start: u32,
    trim_end: u32,
}

impl PacketInfo {
    fn from_packet(packet: &Packet) -> Self {
        PacketInfo {
            ts: packet.ts,
            dur: packet.dur,
            trim_start: packet.trim_start,
            trim_end: packet.trim_end,
        }
    }
}

struct DecodeWorker {
    message_producer: rtrb::Producer<DecodeWorkerToFileStreamMessage>,
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    resampler: Option<Resampler>,
    playhead: usize,
    num_channels: usize,
    sample_rate: u32,
    target_sample_rate: u32,
    track_id: u32,
}

impl DecodeWorker {
    fn new(
        target_sample_rate: u32,
        track_id: u32,
        reader: Box<dyn FormatReader>,
        decoder: Box<dyn Decoder>,
        resampler: Option<Resampler>,
        message_producer: rtrb::Producer<DecodeWorkerToFileStreamMessage>,
    ) -> Self {
        // Decoder has already decoded the first packet
        let first = decoder.last_decoded();
        let spec = first.spec();
        let num_channels = spec.channels.count();
        let sample_rate = spec.rate;
        DecodeWorker {
            message_producer,
            reader,
            decoder,
            resampler,
            playhead: 0,
            num_channels,
            sample_rate,
            target_sample_rate,
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
                    self.message_producer
                        .push(DecodeWorkerToFileStreamMessage::Eof)
                        // Should not fail as we checked for is_full() above.
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
                    self.message_producer
                        .push(DecodeWorkerToFileStreamMessage::Block(Box::new(
                            make_block(
                                PacketInfo::from_packet(&packet),
                                decoded,
                                &mut self.resampler,
                            ),
                        )))
                        .unwrap();
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

fn make_block(
    packet_info: PacketInfo,
    decoded: AudioBufferRef,
    maybe_resampler: &mut Option<Resampler>,
) -> DecodedBlock {
    let num_channels = decoded.spec().channels.count();

    let planar_samples: Vec<f32> = if let Some(resampler) = maybe_resampler.as_mut() {
        let resampled = resampler.resample(decoded).unwrap();
        resampled.to_vec()
    } else {
        let mut sample_buffer = SampleBuffer::new(decoded.frames() as u64, *decoded.spec());
        sample_buffer.copy_planar_ref(decoded);
        sample_buffer.samples().to_vec()
    };

    DecodedBlock {
        packet_info,
        planar_samples,
        num_channels,
        next: None,
    }
}

pub struct FileStream {
    message_consumer: rtrb::Consumer<DecodeWorkerToFileStreamMessage>,
    blocks: Option<Box<DecodedBlock>>,
    is_eof: bool,
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

        let (decoded, packet_info) = loop {
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
                Ok(decoded) => break Ok((decoded, PacketInfo::from_packet(&packet))),
            }
        }?;

        let spec = decoded.spec();
        let mut resampler = if spec.rate != target_sample_rate {
            Some(Resampler::new(
                *spec,
                target_sample_rate as usize,
                decoded.capacity() as u64,
            ))
        } else {
            None
        };

        let blocks = if decoded.frames() > 0 {
            Some(Box::new(make_block(packet_info, decoded, &mut resampler)))
        } else {
            None
        };

        let (message_producer, message_consumer) = rtrb::RingBuffer::new(MESSAGE_BUFFER_SIZE);
        let worker = DecodeWorker::new(
            target_sample_rate,
            track_id,
            reader,
            decoder,
            resampler,
            message_producer,
        );
        std::thread::spawn(move || {
            worker.run();
        });

        Ok(Self {
            message_consumer,
            blocks,
            is_eof: false,
        })
    }

    pub fn is_ready(&mut self) -> bool {
        self.poll();
        self.blocks.is_some()
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
                DecodeWorkerToFileStreamMessage::Eof => {
                    self.is_eof = true;
                }
            }
        }
    }
}

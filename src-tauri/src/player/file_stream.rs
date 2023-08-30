use std::fs::File;
use std::path::PathBuf;

use log::warn;
use symphonia::core::{
    audio::{AudioBufferRef, SampleBuffer},
    codecs::{Decoder, DecoderOptions},
    formats::{FormatOptions, FormatReader},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

use super::errors::FileStreamOpenError;

struct FileStreamServer {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
}

struct DecodedBlock {
    sample_buffer: SampleBuffer<f32>,
    offset: u64,
    next: Option<Box<DecodedBlock>>,
}

pub struct FileStream {
    target_sample_rate: u32,
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

        let first_packet = loop {
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

        todo!("Implement FileStream::new");
    }
}

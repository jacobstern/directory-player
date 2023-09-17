use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::trace;
use rtrb::{Consumer, Producer};

use crate::player::process::Process;
use crate::player::{ManagerToProcessMsg, ProcessToManagerMsg};

pub struct Output {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub buffer_size: u32,
}

const PREFERRED_BUFFER_SIZE: u32 = 1024;
const PREFERRED_SAMPLE_RATE: u32 = 44100;

impl Output {
    pub fn new(
        to_manager_tx: Producer<ProcessToManagerMsg>,
        from_manager_rx: Consumer<ManagerToProcessMsg>,
    ) -> Output {
        // Setup cpal audio output

        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("no output device available");
        let default_config = device.default_output_config().unwrap();

        // The only other property that could be relevant is
        // the buffer size range, but that seems unlikely to cause
        // problems.
        let preferred_config = device.supported_output_configs().unwrap().find(|c| {
            c.max_sample_rate().0 >= PREFERRED_SAMPLE_RATE
                && c.min_sample_rate().0 <= PREFERRED_SAMPLE_RATE
        });

        let buffer_size_range = preferred_config
            .as_ref()
            .map_or(default_config.buffer_size(), |value| value.buffer_size());
        let buffer_size = match buffer_size_range {
            cpal::SupportedBufferSize::Unknown => PREFERRED_BUFFER_SIZE,
            cpal::SupportedBufferSize::Range { max, min: _ } => PREFERRED_BUFFER_SIZE.min(*max),
        };
        let sample_rate = preferred_config
            .as_ref()
            .map_or(default_config.sample_rate(), |value| {
                cpal::SampleRate(value.max_sample_rate().0.min(PREFERRED_SAMPLE_RATE))
            });

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Fixed(buffer_size),
        };

        let mut process = Process::new(to_manager_tx, from_manager_rx);

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| process.process(data),
                move |err| {
                    eprintln!("{}", err);
                },
                None,
            )
            .unwrap();

        stream.play().unwrap();

        trace!("Stream sample rate: {:?}", sample_rate.0);

        Output {
            _stream: stream,
            sample_rate: sample_rate.0,
            buffer_size,
        }
    }
}

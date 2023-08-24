use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer};

use crate::player::process::Process;
use crate::player::{GuiToProcessMsg, ProcessToGuiMsg};

pub struct Output {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub buffer_size: u32,
}

const PREFERRED_BUFFER_SIZE: u32 = 1024;

impl Output {
    pub fn new(
        to_gui_tx: Producer<ProcessToGuiMsg>,
        from_gui_rx: Consumer<GuiToProcessMsg>,
    ) -> Output {
        // Setup cpal audio output

        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("no output device available");

        let default_config = device.default_output_config().unwrap();
        let sample_rate = default_config.sample_rate();
        let buffer_size_range = default_config.buffer_size();
        let buffer_size = match buffer_size_range {
            cpal::SupportedBufferSize::Unknown => PREFERRED_BUFFER_SIZE,
            cpal::SupportedBufferSize::Range { max, min: _ } => PREFERRED_BUFFER_SIZE.min(*max),
        };

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Fixed(buffer_size),
        };

        let mut process = Process::new(to_gui_tx, from_gui_rx);

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

        Output {
            _stream: stream,
            sample_rate: sample_rate.0,
            buffer_size,
        }
    }
}

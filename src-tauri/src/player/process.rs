use creek::read::ReadError;
use creek::{Decoder, ReadDiskStream, SeekMode, SymphoniaDecoder};
use log::error;
use rtrb::{Consumer, Producer};
use rubato::Resampler;

use crate::player::{GuiToProcessMsg, ProcessToGuiMsg};

use super::ProcessResampler;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Paused,
    Playing,
}

pub struct Process {
    read_disk_stream: Option<ReadDiskStream<SymphoniaDecoder>>,
    resampler: Option<ProcessResampler>,

    to_gui_tx: Producer<ProcessToGuiMsg>,
    from_gui_rx: Consumer<GuiToProcessMsg>,

    playback_state: PlaybackState,
    had_cache_miss_last_cycle: bool,

    gain: f32,

    fatal_error: bool,
}

impl Process {
    pub fn new(
        to_gui_tx: Producer<ProcessToGuiMsg>,
        from_gui_rx: Consumer<GuiToProcessMsg>,
    ) -> Self {
        Self {
            read_disk_stream: None,
            resampler: None,

            to_gui_tx,
            from_gui_rx,

            playback_state: PlaybackState::Paused,
            had_cache_miss_last_cycle: false,

            gain: 0.0,

            fatal_error: false,
        }
    }

    pub fn process(&mut self, data: &mut [f32]) {
        if self.fatal_error {
            silence(data);
            return;
        }

        if let Err(e) = self.try_process(data) {
            if matches!(e, ReadError::FatalError(_)) {
                self.fatal_error = true;
            }

            error!("{:?}", e);
            silence(data);
        }
    }

    fn try_process(
        &mut self,
        mut data: &mut [f32],
    ) -> Result<(), ReadError<<SymphoniaDecoder as Decoder>::FatalError>> {
        // Process messages from GUI.
        while let Ok(msg) = self.from_gui_rx.pop() {
            match msg {
                GuiToProcessMsg::StartPlayback(read_disk_stream, resampler) => {
                    self.read_disk_stream = Some(read_disk_stream);
                    self.set_resampler(resampler);
                    self.playback_state = PlaybackState::Playing;
                }
                GuiToProcessMsg::Pause => {
                    self.playback_state = PlaybackState::Paused;
                }
                GuiToProcessMsg::Resume => {
                    self.playback_state = PlaybackState::Playing;
                }
                GuiToProcessMsg::SeekTo(pos) => {
                    if let Some(read_disk_stream) = &mut self.read_disk_stream {
                        read_disk_stream.seek(pos, SeekMode::Auto)?;
                    }
                    let _ = self.to_gui_tx.push(ProcessToGuiMsg::DidSeek);
                }
                GuiToProcessMsg::SetGain(gain) => {
                    self.gain = gain;
                }
            }
        }

        let mut cache_missed_this_cycle = false;
        let mut reached_end_of_file = false;

        if self.playback_state == PlaybackState::Paused {
            silence(data);
        } else if let Some(read_disk_stream) = &mut self.read_disk_stream {
            let num_channels = read_disk_stream.info().num_channels;

            if let Some(ProcessResampler {
                resampler,
                in_buffer,
                out_buffer,
            }) = self.resampler.as_mut()
            {
                let requested_frames = resampler.input_frames_next();
                let mut decoded_frames = 0;
                let output_frames = resampler.output_frames_next();

                while decoded_frames < requested_frames {
                    if !read_disk_stream.is_ready()? {
                        cache_missed_this_cycle = true;
                        let _ = self.to_gui_tx.push(ProcessToGuiMsg::Buffering);
                        break;
                    }

                    let read_data = read_disk_stream.read(requested_frames - decoded_frames)?;
                    let chunk_frames = read_data.num_frames();

                    for channel in 0..num_channels {
                        in_buffer[channel as usize][decoded_frames..decoded_frames + chunk_frames]
                            .copy_from_slice(read_data.read_channel(channel as usize));
                    }

                    decoded_frames += read_data.num_frames();

                    if read_data.reached_end_of_file() {
                        reached_end_of_file = true;
                        break;
                    }
                }

                if decoded_frames < requested_frames {
                    for channel in 0..num_channels {
                        for sample in
                            &mut in_buffer[channel as usize][decoded_frames..requested_frames]
                        {
                            *sample = 0.0;
                        }
                    }
                }

                // TODO: Error handling
                resampler
                    .process_into_buffer(in_buffer, out_buffer, None)
                    .expect("Resampling failure");

                if num_channels == 1 {
                    for i in 0..output_frames {
                        data[i * 2] = out_buffer[0][i];
                        data[i * 2 + 1] = out_buffer[0][i];
                    }
                } else {
                    // Test
                    for i in 0..output_frames {
                        data[i * 2] = out_buffer[0][i];
                        data[i * 2 + 1] = out_buffer[1][i];
                    }
                }

                for sample in data.iter_mut() {
                    *sample *= self.gain;
                }
            } else {
                while !data.is_empty() {
                    if !read_disk_stream.is_ready()? {
                        cache_missed_this_cycle = true;
                        let _ = self.to_gui_tx.push(ProcessToGuiMsg::Buffering);
                        break;
                    }

                    let read_frames = data.len() / 2;
                    let read_data = read_disk_stream.read(read_frames)?;
                    let chunk_frames = read_data.num_frames();

                    if read_data.num_channels() == 1 {
                        let ch = read_data.read_channel(0);

                        for i in 0..chunk_frames {
                            data[i * 2] = ch[i];
                            data[i * 2 + 1] = ch[i];
                        }
                    } else if read_data.num_channels() == 2 {
                        let ch1 = read_data.read_channel(0);
                        let ch2 = read_data.read_channel(1);

                        for i in 0..chunk_frames {
                            data[i * 2] = ch1[i];
                            data[i * 2 + 1] = ch2[i];
                        }
                    }

                    for sample in &mut data[0..chunk_frames * 2] {
                        *sample *= self.gain;
                    }

                    data = &mut data[chunk_frames * 2..];

                    if read_data.reached_end_of_file() {
                        reached_end_of_file = true;
                        break;
                    }
                }

                // Fill silence if we have reached the end of the stream
                silence(data);
            }

            let _ = self.to_gui_tx.push(if reached_end_of_file {
                ProcessToGuiMsg::PlaybackEnded
            } else {
                ProcessToGuiMsg::Progress(read_disk_stream.playhead())
            });
        } else {
            silence(data);
        }

        if reached_end_of_file {
            self.read_disk_stream = None;
            self.playback_state = PlaybackState::Paused;
            self.set_resampler(None);
        }

        // When the cache misses, the buffer is filled with silence. So the next
        // buffer after the cache miss is starting from silence. To avoid an audible
        // pop, apply a ramping gain from 0 up to unity.

        // TODO: Fix this to have a more reasonable behavior

        // if self.had_cache_miss_last_cycle {
        //     let buffer_size = data.len() as f32;
        //     for (i, sample) in data.iter_mut().enumerate() {
        //         *sample *= i as f32 / buffer_size;
        //     }
        // }

        self.had_cache_miss_last_cycle = cache_missed_this_cycle;

        Ok(())
    }

    fn set_resampler(&mut self, resampler: Option<ProcessResampler>) {
        if let Some(resampler) = self.resampler.take() {
            let _ = self
                .to_gui_tx
                .push(ProcessToGuiMsg::DisposeResamplerBuffers(resampler));
        }
        self.resampler = resampler;
    }
}

fn silence(data: &mut [f32]) {
    for sample in data.iter_mut() {
        *sample = 0.0;
    }
}

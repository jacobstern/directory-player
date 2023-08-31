use creek::read::ReadError;
use creek::{Decoder, ReadDiskStream, SeekMode, SymphoniaDecoder};
use log::error;
use rtrb::{Consumer, Producer};
use rubato::Resampler;

use crate::player::{GuiToProcessMsg, ProcessToGuiMsg};

use super::file_stream::FileStream;
use super::{file_stream, ProcessResampler};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Paused,
    Playing,
}

pub struct Process {
    file_stream: Option<FileStream>,
    // file_stream: Option<ReadDiskStream<SymphoniaDecoder>>,
    // resampler: Option<ProcessResampler>,
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
            file_stream: None,
            // resampler: None,
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
                // GuiToProcessMsg::StartPlayback(file_stream, resampler) => {
                //     self.file_stream = Some(file_stream);
                //     self.set_resampler(resampler);
                //     self.playback_state = PlaybackState::Playing;
                // }
                GuiToProcessMsg::StartPlayback(file_stream) => {
                    self.file_stream = Some(file_stream);
                    self.playback_state = PlaybackState::Playing;
                }
                GuiToProcessMsg::Pause => {
                    self.playback_state = PlaybackState::Paused;
                }
                GuiToProcessMsg::Resume => {
                    self.playback_state = PlaybackState::Playing;
                }
                // GuiToProcessMsg::SeekTo(pos) => {
                //     if let Some(file_stream) = &mut self.file_stream {
                //         file_stream.seek(pos, SeekMode::Auto)?;
                //     }
                //     let _ = self.to_gui_tx.push(ProcessToGuiMsg::DidSeek);
                // }
                GuiToProcessMsg::SetGain(gain) => {
                    self.gain = gain;
                }
            }
        }

        let mut cache_missed_this_cycle = false;
        let mut reached_end_of_file = false;

        if self.playback_state == PlaybackState::Paused {
            silence(data);
        } else if let Some(file_stream) = &mut self.file_stream {
            while !data.is_empty() {
                if !file_stream.is_ready() {
                    cache_missed_this_cycle = true;
                    let _ = self.to_gui_tx.push(ProcessToGuiMsg::Buffering);
                    break;
                }

                let read_frames = data.len() / 2;
                let read_data = file_stream
                    .read(read_frames)
                    .expect("there to be available data to read");
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

            let _ = self.to_gui_tx.push(if reached_end_of_file {
                ProcessToGuiMsg::PlaybackEnded
            } else {
                ProcessToGuiMsg::Progress(file_stream.playhead())
            });
        } else {
            silence(data);
        }

        if reached_end_of_file {
            self.file_stream = None;
            self.playback_state = PlaybackState::Paused;
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
}

fn silence(data: &mut [f32]) {
    for sample in data.iter_mut() {
        *sample = 0.0;
    }
}

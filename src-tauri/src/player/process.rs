use creek::read::ReadError;
use creek::{Decoder, ReadDiskStream, SeekMode, SymphoniaDecoder};
use log::error;
use rtrb::{Consumer, Producer};

use crate::player::{GuiToProcessMsg, ProcessToGuiMsg};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Paused,
    Playing,
}

pub struct Process {
    read_disk_stream: Option<ReadDiskStream<SymphoniaDecoder>>,

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
            to_gui_tx,
            from_gui_rx,

            playback_state: PlaybackState::Paused,
            had_cache_miss_last_cycle: false,

            gain: 0.17,

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
                GuiToProcessMsg::StartPlayback(read_disk_stream) => {
                    self.playback_state = PlaybackState::Paused;
                    self.read_disk_stream = Some(read_disk_stream);
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
                }
            }
        }

        let mut cache_missed_this_cycle = false;
        if let Some(read_disk_stream) = &mut self.read_disk_stream {
            if self.playback_state == PlaybackState::Paused {
                silence(data);
                return Ok(());
            }

            let mut reached_end_of_file = false;

            while !data.is_empty() {
                if !read_disk_stream.is_ready()? {
                    cache_missed_this_cycle = true;
                    let _ = self.to_gui_tx.push(ProcessToGuiMsg::Buffering);
                    break;
                }

                let read_frames = data.len() / 2;
                // NOTE: Might want to report doc bug for this function
                let read_data = read_disk_stream.read(read_frames)?;

                if read_data.num_channels() == 1 {
                    let ch = read_data.read_channel(0);

                    for i in 0..read_data.num_frames() {
                        data[i * 2] = ch[i] * self.gain;
                        data[i * 2 + 1] = ch[i] * self.gain;
                    }
                } else if read_data.num_channels() == 2 {
                    let ch1 = read_data.read_channel(0);
                    let ch2 = read_data.read_channel(1);

                    for i in 0..read_data.num_frames() {
                        data[i * 2] = ch1[i] * self.gain;
                        data[i * 2 + 1] = ch2[i] * self.gain;
                    }
                }

                data = &mut data[read_data.num_frames() * 2..];

                if read_data.reached_end_of_file() {
                    self.playback_state = PlaybackState::Paused;
                    reached_end_of_file = true;
                    break;
                }
            }

            // Fill silence if we have reached the end of the stream
            silence(data);

            let _ = self.to_gui_tx.push(if reached_end_of_file {
                ProcessToGuiMsg::PlaybackEnded
            } else {
                ProcessToGuiMsg::PlaybackPos(read_disk_stream.playhead())
            });
        } else {
            // Output silence until file is received.
            silence(data);
        }

        // When the cache misses, the buffer is filled with silence. So the next
        // buffer after the cache miss is starting from silence. To avoid an audible
        // pop, apply a ramping gain from 0 up to unity.
        if self.had_cache_miss_last_cycle {
            let buffer_size = data.len() as f32;
            for (i, sample) in data.iter_mut().enumerate() {
                *sample *= i as f32 / buffer_size;
            }
        }

        self.had_cache_miss_last_cycle = cache_missed_this_cycle;
        Ok(())
    }
}

fn silence(data: &mut [f32]) {
    for sample in data.iter_mut() {
        *sample = 0.0;
    }
}

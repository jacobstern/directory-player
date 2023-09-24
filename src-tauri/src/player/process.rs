// use creek::read::ReadError;
// use creek::{Decoder, ReadDiskStream, SeekMode, SymphoniaDecoder};
use log::error;
use rtrb::{Consumer, Producer};

use crate::player::{ManagerToProcessMsg, ProcessToManagerMsg};

use super::{file_stream::FileStream, StartPlaybackState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessPlaybackState {
    Paused,
    Playing,
}

struct Stream {
    file_stream: FileStream,
    playback_id: u64,
}

pub struct Process {
    stream: Option<Stream>,
    to_gui_tx: Producer<ProcessToManagerMsg>,
    from_gui_rx: Consumer<ManagerToProcessMsg>,
    playback_state: ProcessPlaybackState,
    gain: f32,
    fatal_error: bool,
}

impl Process {
    pub fn new(
        to_gui_tx: Producer<ProcessToManagerMsg>,
        from_gui_rx: Consumer<ManagerToProcessMsg>,
    ) -> Self {
        Self {
            stream: None,
            to_gui_tx,
            from_gui_rx,
            playback_state: ProcessPlaybackState::Paused,
            gain: 0.0,
            fatal_error: false,
        }
    }

    pub fn process(&mut self, data: &mut [f32]) {
        // TODO: Make sure the various ring buffers are not backed up
        if self.fatal_error {
            silence(data);
            return;
        }

        if let Err(e) = self.try_process(data) {
            error!("{:?}", e);
            silence(data);
        }
    }

    fn try_process(&mut self, mut data: &mut [f32]) -> symphonia::core::errors::Result<()> {
        while let Ok(msg) = self.from_gui_rx.pop() {
            match msg {
                ManagerToProcessMsg::StartPlayback(
                    playback_id,
                    file_stream,
                    start_playback_state,
                ) => {
                    self.stream = Some(Stream {
                        file_stream,
                        playback_id,
                    });
                    self.playback_state = match start_playback_state {
                        StartPlaybackState::Playing => ProcessPlaybackState::Playing,
                        StartPlaybackState::Paused => ProcessPlaybackState::Paused,
                    };
                }
                ManagerToProcessMsg::Stop => {
                    self.stream = None;
                    self.playback_state = ProcessPlaybackState::Paused;
                }
                ManagerToProcessMsg::Pause => {
                    self.playback_state = ProcessPlaybackState::Paused;
                }
                ManagerToProcessMsg::Resume => {
                    self.playback_state = ProcessPlaybackState::Playing;
                }
                ManagerToProcessMsg::SeekTo(pos) => {
                    if let Some(Stream {
                        file_stream,
                        playback_id,
                    }) = &mut self.stream
                    {
                        file_stream.seek(pos);
                        let _ = self.to_gui_tx.push(ProcessToManagerMsg::PlaybackPos(
                            *playback_id,
                            file_stream.playhead(),
                        ));
                    }
                }
                ManagerToProcessMsg::SetGain(gain) => {
                    self.gain = gain;
                }
            }
        }

        let mut reached_end_of_file = false;

        if self.playback_state == ProcessPlaybackState::Paused {
            silence(data);
        } else if let Some(Stream {
            file_stream,
            playback_id,
        }) = &mut self.stream
        {
            while !data.is_empty() {
                if !file_stream.is_ready() {
                    // Buffering...
                    break;
                }

                let read_frames = data.len() / 2;
                let read_data = file_stream
                    .read(read_frames)
                    .expect("Expected there to be available data to read");
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

            let _ = self.to_gui_tx.push(ProcessToManagerMsg::PlaybackPos(
                *playback_id,
                file_stream.playhead(),
            ));
            if reached_end_of_file {
                let _ = self
                    .to_gui_tx
                    .push(ProcessToManagerMsg::PlaybackEnded(*playback_id));
            }
        } else {
            silence(data);
        }

        if reached_end_of_file {
            self.stream = None;
            self.playback_state = ProcessPlaybackState::Paused;
        }

        // TODO: Fade in/out audio when buffering?

        Ok(())
    }
}

fn silence(data: &mut [f32]) {
    for sample in data.iter_mut() {
        *sample = 0.0;
    }
}

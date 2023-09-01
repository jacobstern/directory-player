use std::{sync::mpsc, thread, time::Duration};

// use creek::{ReadDiskStream, ReadStreamOptions, SymphoniaDecoder};
use log::{error, info, warn};
use rtrb::RingBuffer;
use rubato::{FftFixedOut, Resampler};

use crate::player::{file_stream::FileStream, ProcessResampler, TrackInfo};

use super::{output::Output, GuiToProcessMsg, PlayerEvent, ProcessToGuiMsg};

pub enum ManagerCommand {
    StartPlayback(Vec<String>),
    Pause,
    Progress(usize),
    PlaybackEnded,
    Buffering,
    Resume,
    SetVolume(f64),
    SeekTo(usize),
    DidSeek,
}

#[derive(Clone)]
struct Queue<T> {
    elements: Vec<T>,
    index: usize,
}

impl<T> Queue<T> {
    pub fn from_vec(elements: Vec<T>) -> Option<Queue<T>> {
        if elements.is_empty() {
            None
        } else {
            Some(Queue { elements, index: 0 })
        }
    }

    pub fn current(&self) -> &T {
        &self.elements[self.index]
    }

    pub fn go_next(self) -> Option<Queue<T>> {
        if self.index + 1 < self.elements.len() {
            Some(Queue {
                index: self.index + 1,
                elements: self.elements,
            })
        } else {
            None
        }
    }
}

fn gain_for_volume(volume: f64) -> f32 {
    let clamped = volume.max(0_f64).min(100_f64);
    let normalized = clamped / 100.0;
    let amp = normalized.powf(2.7);
    (amp as f32).min(1.0)
}

const NUM_CACHE_BLOCKS: usize = 20;

pub struct PlaybackManager {
    output: Output,
    to_process_tx: rtrb::Producer<GuiToProcessMsg>,
    command_rx: mpsc::Receiver<ManagerCommand>,
    stream_frame_count: Option<u64>,
    queue: Option<Queue<String>>,
    event_tx: tokio::sync::mpsc::Sender<PlayerEvent>,
}

impl PlaybackManager {
    pub fn new(
        event_tx: tokio::sync::mpsc::Sender<PlayerEvent>,
        command_tx: mpsc::Sender<ManagerCommand>,
        command_rx: mpsc::Receiver<ManagerCommand>,
    ) -> PlaybackManager {
        let (to_gui_tx, mut from_process_rx) = RingBuffer::<ProcessToGuiMsg>::new(256);
        let (to_process_tx, from_gui_rx) = RingBuffer::<GuiToProcessMsg>::new(64);
        let output = Output::new(to_gui_tx, from_gui_rx);

        thread::spawn({
            let tx = command_tx.clone();
            move || {
                let mut failed_to_send = false;
                while !failed_to_send {
                    while let Ok(msg) = from_process_rx.pop() {
                        let manager_command = match msg {
                            ProcessToGuiMsg::Buffering => Some(ManagerCommand::Buffering),
                            ProcessToGuiMsg::Progress(pos) => Some(ManagerCommand::Progress(pos)),
                            ProcessToGuiMsg::PlaybackEnded => Some(ManagerCommand::PlaybackEnded),
                            // ProcessToGuiMsg::DidSeek => Some(ManagerCommand::DidSeek),
                            // Special message, just deallocate the resource
                            // ProcessToGuiMsg::DisposeResamplerBuffers(_) => None,
                        };
                        if let Some(command) = manager_command {
                            let result = tx.send(command);
                            failed_to_send = result.is_err();
                            if failed_to_send {
                                break;
                            }
                        }
                    }
                    if !failed_to_send {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });

        PlaybackManager {
            output,
            to_process_tx,
            command_rx,
            stream_frame_count: None,
            queue: None,
            event_tx,
        }
    }

    pub fn run(mut self) {
        while let Ok(msg) = self.command_rx.recv() {
            match msg {
                ManagerCommand::StartPlayback(file_paths) => {
                    self.queue = Queue::from_vec(file_paths);
                    if let Some(queue) = self.queue.as_ref() {
                        self.start_stream(queue.current().to_owned());
                    }
                }
                ManagerCommand::Pause => self
                    .to_process_tx
                    .push(GuiToProcessMsg::Pause)
                    .unwrap_or_else(|_| {
                        warn!("Failed to send pause message to audio thread");
                    }),
                ManagerCommand::Resume => self
                    .to_process_tx
                    .push(GuiToProcessMsg::Resume)
                    .unwrap_or_else(|_| {
                        warn!("Failed to send resume message to audio thread");
                    }),
                ManagerCommand::Buffering => {
                    // debug!("Buffering...");
                }
                ManagerCommand::Progress(pos) => self
                    .event_tx
                    .blocking_send(PlayerEvent::Progress(pos))
                    .unwrap_or_else(|e| {
                        error!("Failed to send Progress event with {e:?}");
                    }),
                ManagerCommand::DidSeek => self
                    .event_tx
                    .blocking_send(PlayerEvent::DidSeek)
                    .unwrap_or_else(|e| {
                        error!("Failed to send Seek event with {e:?}");
                    }),
                ManagerCommand::PlaybackEnded => {
                    self.queue = self.queue.and_then(|queue| queue.go_next());
                    if let Some(queue) = self.queue.as_ref() {
                        self.start_stream(queue.current().to_owned());
                    }
                }
                ManagerCommand::SetVolume(volume) => {
                    let gain = gain_for_volume(volume);
                    info!("Setting gain {gain:?}");
                    self.to_process_tx
                        .push(GuiToProcessMsg::SetGain(gain))
                        .unwrap_or_else(|_| {
                            warn!("Failed to send gain message to audio thread");
                        })
                }
                ManagerCommand::SeekTo(offset) => {
                    self.to_process_tx
                        .push(GuiToProcessMsg::SeekTo(offset))
                        .unwrap_or_else(|_| {
                            error!("Failed to send seek message to audio thread");
                        });
                }
            }
        }
    }

    fn start_stream(&mut self, path: String) {
        info!("Starting stream for {:?}", path);

        // TODO: Handle open failure
        let file_stream = FileStream::open(path.clone(), self.output.sample_rate)
            .expect("Failed to create the file stream");
        let n_frames = file_stream.n_frames();

        self.to_process_tx
            .push(GuiToProcessMsg::StartPlayback(file_stream))
            .unwrap_or_else(|_| error!("Failed to send message to start playback to audio thread"));

        self.stream_frame_count = n_frames;
        if let Some(n) = n_frames {
            self.event_tx
                .blocking_send(PlayerEvent::Track(TrackInfo {
                    path,
                    duration: n as usize,
                }))
                .unwrap_or_else(|e| {
                    error!("Failed to send Track event with {e:?}");
                });
        }
    }
}

use std::{path::Path, sync::mpsc, thread, time::Duration};

use log::{error, info, warn};
use rtrb::RingBuffer;
use symphonia::core::units::{Time, TimeBase};

use crate::player::{file_stream::FileStream, PlaybackFile, TrackInfo};

use super::{
    errors::FileStreamOpenError, output::Output, GuiToProcessMsg, PlaybackState, PlayerEvent,
    ProcessToGuiMsg,
};

const STREAM_SEEK_BACK_THRESHOLD_SECONDS: u8 = 3;

pub enum ManagerCommand {
    StartPlayback(Vec<String>),
    Pause,
    Progress(usize),
    PlaybackEnded,
    Resume,
    SetVolume(f64),
    SeekTo(usize),
    OpenFileStreamError(u64, String, FileStreamOpenError),
    OpenFileStream(u64, String, FileStream),
    SkipForward,
    SkipBack,
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

    pub fn has_previous(&self) -> bool {
        self.index > 0
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

    pub fn go_previous_clamped(self) -> Queue<T> {
        if self.has_previous() {
            Queue {
                index: self.index - 1,
                elements: self.elements,
            }
        } else {
            self
        }
    }
}

struct StreamTimingInternal {
    time_base: TimeBase,
    n_frames: u64,
    pos: usize,
}

fn gain_for_volume(volume: f64) -> f32 {
    let clamped = volume.max(0_f64).min(100_f64);
    let normalized = clamped / 100.0;
    let amp = normalized.powf(2.7);
    (amp as f32).min(1.0)
}

pub struct PlaybackManager {
    output: Output,
    to_process_tx: rtrb::Producer<GuiToProcessMsg>,
    command_rx: mpsc::Receiver<ManagerCommand>,
    command_tx: mpsc::Sender<ManagerCommand>,
    queue: Option<Queue<String>>,
    event_tx: tokio::sync::mpsc::Sender<PlayerEvent>,
    current_playback_id: Option<u64>,
    next_playback_id: u64,
    playback_state: PlaybackState,
    stream_timing: Option<StreamTimingInternal>,
}

fn poll_process_to_gui_message(
    command_tx: mpsc::Sender<ManagerCommand>,
    mut from_process_rx: rtrb::Consumer<ProcessToGuiMsg>,
) {
    let mut failed_to_send = false;
    while !failed_to_send {
        let mut progress: Option<usize> = None;
        while let Ok(msg) = from_process_rx.pop() {
            let manager_command = match msg {
                ProcessToGuiMsg::PlaybackEnded => Some(ManagerCommand::PlaybackEnded),
                ProcessToGuiMsg::PlaybackPos(pos) => {
                    progress = Some(pos);
                    None
                }
            };
            if let Some(command) = manager_command {
                let result = command_tx.send(command);
                failed_to_send = result.is_err();
                if failed_to_send {
                    break;
                }
            }
        }
        if let Some(pos) = progress {
            let result = command_tx.send(ManagerCommand::Progress(pos));
            failed_to_send = result.is_err();
        }
        if !failed_to_send {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

impl PlaybackManager {
    pub fn new(
        event_tx: tokio::sync::mpsc::Sender<PlayerEvent>,
        command_tx: mpsc::Sender<ManagerCommand>,
        command_rx: mpsc::Receiver<ManagerCommand>,
    ) -> PlaybackManager {
        let (to_gui_tx, from_process_rx) = RingBuffer::<ProcessToGuiMsg>::new(256);
        let (to_process_tx, from_gui_rx) = RingBuffer::<GuiToProcessMsg>::new(64);
        let output = Output::new(to_gui_tx, from_gui_rx);

        thread::spawn({
            let tx = command_tx.clone();
            move || {
                poll_process_to_gui_message(tx, from_process_rx);
            }
        });

        PlaybackManager {
            output,
            to_process_tx,
            command_rx,
            command_tx,
            queue: None,
            event_tx,
            current_playback_id: None,
            next_playback_id: 0,
            playback_state: PlaybackState::Stopped,
            stream_timing: None,
        }
    }

    pub fn run(mut self) {
        while let Ok(msg) = self.command_rx.recv() {
            match msg {
                ManagerCommand::StartPlayback(file_paths) => {
                    self.queue = Queue::from_vec(file_paths);
                    if let Some(queue) = self.queue.as_ref() {
                        self.start_playback(queue.current().to_owned());
                    }
                }
                ManagerCommand::Pause => {
                    self.to_process_tx
                        .push(GuiToProcessMsg::Pause)
                        .unwrap_or_else(|_| {
                            error!("Failed to send pause message to audio thread");
                        });
                    self.update_playback_state(PlaybackState::Paused);
                }
                ManagerCommand::Resume => {
                    self.to_process_tx
                        .push(GuiToProcessMsg::Resume)
                        .unwrap_or_else(|_| {
                            error!("Failed to send resume message to audio thread");
                        });
                    self.update_playback_state(PlaybackState::Playing);
                }
                ManagerCommand::Progress(pos) => {
                    self.progress(pos);
                }
                ManagerCommand::PlaybackEnded => {
                    // TODO: Check for current_playback_id
                    self.play_next();
                }
                ManagerCommand::SetVolume(volume) => {
                    let gain = gain_for_volume(volume);
                    self.to_process_tx
                        .push(GuiToProcessMsg::SetGain(gain))
                        .unwrap_or_else(|_| {
                            error!("Failed to send gain message to audio thread");
                        })
                }
                ManagerCommand::SeekTo(offset) => {
                    self.to_process_tx
                        .push(GuiToProcessMsg::SeekTo(offset))
                        .unwrap_or_else(|_| {
                            error!("Failed to send seek message to audio thread");
                        });
                }
                ManagerCommand::OpenFileStream(playback_id, path, file_stream) => {
                    self.open_file_stream(playback_id, path, file_stream);
                }
                ManagerCommand::OpenFileStreamError(playback_id, path, e) => {
                    if Some(playback_id) != self.current_playback_id {
                        info!(
                            "Ignoring open stream error for {:?} as it is no longer the current playback",
                            path
                        );
                        continue;
                    }

                    // TODO: Surface errors to the UI
                    error!("Failed to open file stream for {path:?}: {e:?}");

                    self.play_next();
                }
                ManagerCommand::SkipForward => {
                    self.skip_forward();
                }
                ManagerCommand::SkipBack => {
                    self.skip_back();
                }
            }
        }
    }

    fn open_file_stream(&mut self, playback_id: u64, path: String, file_stream: FileStream) {
        if Some(playback_id) != self.current_playback_id {
            info!(
                "Ignoring stream for {:?} as it is no longer the current playback",
                path
            );
            return;
        }

        let n_frames = file_stream.n_frames();
        let time_base = file_stream.time_base();

        if let Some(n_frames) = n_frames {
            self.event_tx
                .blocking_send(PlayerEvent::Track(TrackInfo {
                    path,
                    duration: n_frames as usize,
                }))
                .unwrap_or_else(|e| {
                    error!("Failed to send Track event with {e:?}");
                });

            if let Some(time_base) = time_base {
                self.stream_timing = Some(StreamTimingInternal {
                    time_base: *time_base,
                    n_frames,
                    pos: 0,
                });
            }
        }

        self.update_playback_state(PlaybackState::Playing);
        self.to_process_tx
            .push(GuiToProcessMsg::StartPlayback(file_stream))
            .unwrap_or_else(|_| warn!("Failed to send message to start playback to audio thread"));
    }

    fn skip_forward(&mut self) {
        self.play_next();
    }

    fn skip_back(&mut self) {
        let has_previous = self
            .queue
            .as_ref()
            .map_or(false, |queue| queue.has_previous());
        let is_early_in_stream = self.stream_timing.as_ref().map_or(false, |timing| {
            timing.time_base.calc_time(timing.pos as u64)
                < Time::from_ss(STREAM_SEEK_BACK_THRESHOLD_SECONDS, 0).unwrap()
        });

        if is_early_in_stream && has_previous {
            self.queue = self.queue.take().map(|queue| {
                let previous = queue.go_previous_clamped();
                self.start_playback(previous.current().to_owned());
                previous
            });
        } else {
            self.to_process_tx
                .push(GuiToProcessMsg::SeekTo(0))
                .unwrap_or_else(|_| {
                    error!("Failed to send seek message to audio thread for skip back");
                });
        }
    }

    fn progress(&mut self, pos: usize) {
        if let Some(timing) = self.stream_timing.as_mut() {
            // TODO: New event type
            timing.pos = pos;
        }
        self.event_tx
            .blocking_send(PlayerEvent::Progress(pos))
            .unwrap_or_else(|e| {
                error!("Failed to send Progress event with {e:?}");
            });
    }

    fn play_next(&mut self) {
        self.queue = self.queue.take().and_then(|queue| queue.go_next());
        if let Some(queue) = self.queue.as_ref() {
            self.start_playback(queue.current().to_owned());
        } else {
            self.current_playback_id = None;
            self.stream_timing = None;
            self.to_process_tx
                .push(GuiToProcessMsg::Stop)
                .unwrap_or_else(|_| {
                    warn!("Failed to send stop message to audio thread");
                });
            self.try_send_event(PlayerEvent::PlaybackFileChange(None));
            self.update_playback_state(PlaybackState::Stopped);
        }
    }

    fn start_playback(&mut self, path: String) {
        info!("Starting stream for {:?}", path);
        self.update_playback_state(PlaybackState::Playing);

        let playback_id = self.next_playback_id;

        self.next_playback_id += 1;
        self.current_playback_id = Some(playback_id);

        let os_path = Path::new(&path);
        let file_name = os_path.file_name().unwrap().to_str().unwrap().to_owned();

        self.try_send_event(PlayerEvent::PlaybackFileChange(Some(PlaybackFile {
            path: path.clone(),
            name: file_name,
        })));

        let output_sample_rate = self.output.sample_rate;
        let tx = self.command_tx.clone();
        thread::spawn(
            move || match FileStream::open(path.clone(), output_sample_rate) {
                Ok(file_stream) => tx.send(ManagerCommand::OpenFileStream(
                    playback_id,
                    path,
                    file_stream,
                )),
                Err(e) => tx.send(ManagerCommand::OpenFileStreamError(playback_id, path, e)),
            },
        );
    }

    fn update_playback_state(&mut self, playback_state: PlaybackState) {
        if self.playback_state != playback_state {
            self.playback_state = playback_state;
            self.try_send_event(PlayerEvent::PlaybackStateChange(playback_state));
        }
    }

    fn try_send_event(&mut self, event: PlayerEvent) {
        if let Err(e) = self.event_tx.blocking_send(event.clone()) {
            // TODO: Decide on error log level policy
            warn!("Failed to send {event:?} from the manager with {e:?}");
        }
    }
}

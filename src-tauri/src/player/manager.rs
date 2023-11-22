use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

use base64::{engine::general_purpose, Engine};
use log::{error, info, warn};
use random_color::RandomColor;

use rtrb::RingBuffer;
use serde::{Deserialize, Serialize};
use symphonia::core::{
    meta::{StandardTagKey, StandardVisualKey, Value},
    units::{Time, TimeBase},
};

use crate::player::{file_stream::FileStream, queue::Queue, PlaybackFile, StreamMetadata};

use super::{
    errors::FileStreamOpenError, output::Output, queue::GoNextMode, ManagerToProcessMsg,
    PlaybackState, PlayerEvent, ProcessToManagerMsg, StartPlaybackState, StreamMetadataVisual,
    StreamTiming,
};

const STREAM_SEEK_BACK_THRESHOLD_SECONDS_PART: u8 = 3;

fn gen_album_color(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    RandomColor::new().seed(hasher.finish()).to_rgba_string()
}

pub enum ManagerCommand {
    StartPlayback(Vec<String>, usize),
    Pause,
    Stop,
    Progress(u64, usize),
    PlaybackEnded(u64),
    Resume,
    SetVolume(f64),
    SeekTo(usize),
    OpenFileStreamError(u64, String, FileStreamOpenError),
    OpenFileStream(u64, String, FileStream),
    SkipForward,
    SkipBack,
    SetShuffle(ShuffleMode),
    SetRepeat(RepeatMode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StreamTimingInternal {
    time_base: TimeBase,
    n_frames: u64,
    pos: usize,
}

impl StreamTimingInternal {
    pub fn as_stream_timing(&self) -> StreamTiming {
        let duration_time = self.time_base.calc_time(self.n_frames);
        StreamTiming {
            duration: self.n_frames,
            pos: self.pos,
            duration_seconds: duration_time.seconds,
        }
    }
}

fn gain_for_volume(volume: f64) -> f32 {
    let clamped = volume.max(0_f64).min(100_f64);
    let normalized = clamped / 100.0;
    let amp = normalized.powf(2.7);
    (amp as f32).min(1.0)
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ShuffleMode {
    NotEnabled,
    Enabled,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RepeatMode {
    None,
    RepeatAll,
    RepeatOne,
}

pub struct PlaybackManager {
    output: Output,
    to_process_tx: rtrb::Producer<ManagerToProcessMsg>,
    command_rx: mpsc::Receiver<ManagerCommand>,
    command_tx: mpsc::Sender<ManagerCommand>,
    queue: Option<Queue<String>>,
    event_tx: tokio::sync::mpsc::Sender<PlayerEvent>,
    current_playback_id: Option<u64>,
    next_playback_id: u64,
    playback_state: PlaybackState,
    stream_timing: Option<StreamTimingInternal>,
    shuffle_mode: ShuffleMode,
    repeat_mode: RepeatMode,
}

fn poll_process_to_gui_message(
    command_tx: mpsc::Sender<ManagerCommand>,
    mut from_process_rx: rtrb::Consumer<ProcessToManagerMsg>,
) {
    let mut failed_to_send = false;
    while !failed_to_send {
        // TODO: Come up with a better way to debounce/throttle these messages across the system
        let mut debounced_progress_message: Option<ManagerCommand> = None;
        while let Ok(msg) = from_process_rx.pop() {
            match msg {
                ProcessToManagerMsg::PlaybackEnded(playback_id) => {
                    failed_to_send = command_tx
                        .send(ManagerCommand::PlaybackEnded(playback_id))
                        .is_err();
                    if failed_to_send {
                        break;
                    }
                }
                ProcessToManagerMsg::PlaybackPos(playback_id, pos) => {
                    debounced_progress_message = Some(ManagerCommand::Progress(playback_id, pos));
                }
            };
        }
        if let Some(message) = debounced_progress_message {
            failed_to_send = command_tx.send(message).is_err();
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
        let (to_manager_tx, from_process_rx) = RingBuffer::<ProcessToManagerMsg>::new(256);
        let (to_process_tx, from_manager_rx) = RingBuffer::<ManagerToProcessMsg>::new(64);
        let output = Output::new(to_manager_tx, from_manager_rx);

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
            shuffle_mode: ShuffleMode::NotEnabled,
            repeat_mode: RepeatMode::None,
        }
    }

    pub fn run(mut self) {
        while let Ok(msg) = self.command_rx.recv() {
            match msg {
                ManagerCommand::StartPlayback(file_paths, start_index) => {
                    self.start_playback_impl(file_paths, start_index);
                }
                ManagerCommand::Pause => {
                    self.to_process_tx
                        .push(ManagerToProcessMsg::Pause)
                        .unwrap_or_else(|_| {
                            error!("Failed to send pause message to audio thread");
                        });
                    self.set_playback_state(PlaybackState::Paused);
                }
                ManagerCommand::Stop => {
                    self.stop_impl();
                }
                ManagerCommand::Resume => {
                    self.to_process_tx
                        .push(ManagerToProcessMsg::Resume)
                        .unwrap_or_else(|_| {
                            error!("Failed to send resume message to audio thread");
                        });
                    self.set_playback_state(PlaybackState::Playing);
                }
                ManagerCommand::Progress(playback_id, pos) => {
                    self.progress_impl(playback_id, pos);
                }
                ManagerCommand::PlaybackEnded(playback_id) => {
                    self.playback_ended_impl(playback_id);
                }
                ManagerCommand::SetVolume(volume) => {
                    let gain = gain_for_volume(volume);
                    self.to_process_tx
                        .push(ManagerToProcessMsg::SetGain(gain))
                        .unwrap_or_else(|_| {
                            error!("Failed to send gain message to audio thread");
                        })
                }
                ManagerCommand::SeekTo(offset) => {
                    self.seek_to_impl(offset);
                }
                ManagerCommand::OpenFileStream(playback_id, path, file_stream) => {
                    self.open_file_stream_impl(playback_id, path, file_stream);
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
                    self.skip_forward_impl();
                }
                ManagerCommand::SkipBack => {
                    self.skip_back_impl();
                }
                ManagerCommand::SetShuffle(shuffle_mode) => {
                    self.set_shuffle_mode_impl(shuffle_mode);
                }
                ManagerCommand::SetRepeat(repeat_mode) => {
                    self.set_repeat_impl(repeat_mode);
                }
            }
        }
    }

    fn set_shuffle_mode_impl(&mut self, shuffle_mode: ShuffleMode) {
        if shuffle_mode == self.shuffle_mode {
            return;
        }
        if shuffle_mode == ShuffleMode::Enabled {
            self.queue = self.queue.take().map(|queue| queue.to_shuffled());
        } else {
            self.queue = self.queue.take().map(|queue| queue.to_unshuffled());
        }
        self.shuffle_mode = shuffle_mode;
    }

    fn set_repeat_impl(&mut self, repeat_mode: RepeatMode) {
        self.repeat_mode = repeat_mode;
    }

    fn start_playback_impl(&mut self, file_paths: Vec<String>, start_index: usize) {
        self.queue = if self.shuffle_mode == ShuffleMode::Enabled {
            Queue::from_iter_shuffled(file_paths, start_index)
        } else {
            Queue::from_iter(file_paths, start_index)
        };
        if let Some(queue) = self.queue.as_ref() {
            self.start_playback(queue.current().to_owned());
        }
    }

    fn playback_ended_impl(&mut self, playback_id: u64) {
        if self.current_playback_id != Some(playback_id) {
            return;
        }
        self.play_next();
    }

    fn open_file_stream_impl(&mut self, playback_id: u64, path: String, file_stream: FileStream) {
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
            if let Some(time_base) = time_base {
                self.set_stream_timing(Some(StreamTimingInternal {
                    time_base: *time_base,
                    n_frames,
                    pos: 0,
                }));
            }
        }

        let parent_path = Path::new(&path)
            .parent()
            .map(|path| path.to_str().unwrap())
            .unwrap_or(&path);
        let meta: StreamMetadata = file_stream
            .metadata()
            .map(|metadata| {
                let tags = metadata.tags();
                let track_title_tag = tags
                    .into_iter()
                    .find(|tag| tag.std_key == Some(StandardTagKey::TrackTitle));
                let track_title = track_title_tag.as_ref().and_then(|tag| {
                    if let Value::String(s) = tag.value.clone() {
                        Some(s)
                    } else {
                        None
                    }
                });
                let artist_tag = tags
                    .into_iter()
                    .find(|tag| tag.std_key == Some(StandardTagKey::Artist));
                let artist = artist_tag.as_ref().and_then(|tag| {
                    if let Value::String(s) = tag.value.clone() {
                        Some(s)
                    } else {
                        None
                    }
                });
                let album_cover_visual = metadata
                    .visuals()
                    .into_iter()
                    .find(|visual| visual.usage == Some(StandardVisualKey::FrontCover));
                let album_cover = album_cover_visual.map(|visual| StreamMetadataVisual {
                    media_type: visual.media_type.to_owned(),
                    data_base64: general_purpose::URL_SAFE.encode(visual.data.as_ref()),
                });
                StreamMetadata {
                    track_title,
                    artist,
                    album_cover,
                    fallback_color: gen_album_color(&parent_path),
                }
            })
            .unwrap_or(StreamMetadata {
                track_title: None,
                artist: None,
                album_cover: None,
                fallback_color: gen_album_color(&parent_path),
            });
        self.try_send_event(PlayerEvent::StreamMetadataChange(Some(meta)));

        assert_ne!(self.playback_state, PlaybackState::Stopped);

        let start_playback_state = if self.playback_state == PlaybackState::Paused {
            StartPlaybackState::Paused
        } else {
            StartPlaybackState::Playing
        };

        self.to_process_tx
            .push(ManagerToProcessMsg::StartPlayback(
                playback_id,
                file_stream,
                start_playback_state,
            ))
            .unwrap_or_else(|_| warn!("Failed to send message to start playback to audio thread"));
    }

    fn skip_forward_impl(&mut self) {
        self.play_next();
    }

    fn skip_back_impl(&mut self) {
        let has_previous = self
            .queue
            .as_ref()
            .map_or(false, |queue| queue.has_previous());
        let is_early_in_stream = self.stream_timing.as_ref().map_or(false, |timing| {
            timing.time_base.calc_time(timing.pos as u64)
                < Time::from_ss(STREAM_SEEK_BACK_THRESHOLD_SECONDS_PART, 0).unwrap()
        });

        if is_early_in_stream && has_previous {
            let previous = self
                .queue
                .as_mut()
                .map(|queue| queue.go_previous_clamped().to_owned());
            if let Some(path) = previous {
                self.start_playback(path);
            }
        } else if self.stream_timing.as_ref().map_or(0, |timing| timing.pos) > 0 {
            // TODO: Technically, we can have a stream position without the timing data structure
            // but this is not currently done since the UI won't make use of it. Is it worth
            // splitting out the pos field?
            self.to_process_tx
                .push(ManagerToProcessMsg::SeekTo(0))
                .unwrap_or_else(|_| {
                    error!("Failed to send seek message to audio thread for skip back");
                });
        }
    }

    fn progress_impl(&mut self, playback_id: u64, pos: usize) {
        if self.current_playback_id != Some(playback_id) {
            return;
        }
        if let Some(stream_timing) = self.stream_timing.as_ref() {
            let updated = StreamTimingInternal {
                pos,
                ..*stream_timing
            };
            self.set_stream_timing(Some(updated));
        }
    }

    fn stop_impl(&mut self) {
        self.stop_playback();
    }

    fn seek_to_impl(&mut self, offset: usize) {
        if let Some(stream_timing) = self.stream_timing.as_ref() {
            if (offset as u64) < stream_timing.n_frames {
                self.to_process_tx
                    .push(ManagerToProcessMsg::SeekTo(offset))
                    .unwrap_or_else(|_| {
                        error!("Failed to send seek message to audio thread");
                    });
            } else {
                self.play_next();
            }
        }
    }

    fn play_next(&mut self) {
        if self.repeat_mode == RepeatMode::RepeatOne {
            if let Some(path) = self.queue.as_ref().map(|queue| queue.current().to_owned()) {
                self.start_playback(path);
            }
            return;
        }
        let next = self
            .queue
            .as_mut()
            .and_then(|queue| {
                queue.go_next(if self.repeat_mode == RepeatMode::RepeatAll {
                    GoNextMode::RepeatAll
                } else {
                    GoNextMode::Default
                })
            })
            .map(|path| path.to_owned());
        if let Some(path) = next {
            self.start_playback(path);
        } else {
            self.stop_playback();
            self.queue = None;
        }
    }

    fn stop_playback(&mut self) {
        self.current_playback_id = None;

        self.to_process_tx
            .push(ManagerToProcessMsg::Stop)
            .unwrap_or_else(|_| {
                warn!("Failed to send stop message to audio thread");
            });

        self.try_send_event(PlayerEvent::PlaybackFileChange(None));
        self.set_stream_timing(None);
        self.set_playback_state(PlaybackState::Stopped);
        self.try_send_event(PlayerEvent::StreamMetadataChange(None));
    }

    fn start_playback(&mut self, path: String) {
        self.to_process_tx
            .push(ManagerToProcessMsg::Stop)
            .unwrap_or_else(|_| {
                warn!("Failed to send stop message to audio thread when starting a new playback");
            });

        info!("Starting stream for {:?}", path);
        self.set_stream_timing(None);
        self.set_playback_state(PlaybackState::Playing);
        self.try_send_event(PlayerEvent::StreamMetadataChange(None));

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

    fn set_playback_state(&mut self, playback_state: PlaybackState) {
        if self.playback_state != playback_state {
            self.playback_state = playback_state;
            self.try_send_event(PlayerEvent::PlaybackStateChange(playback_state));
        }
    }

    fn set_stream_timing(&mut self, stream_timing: Option<StreamTimingInternal>) {
        if self.stream_timing != stream_timing {
            let stream_timing_payload =
                stream_timing.as_ref().map(|value| value.as_stream_timing());
            self.stream_timing = stream_timing;
            self.try_send_event(PlayerEvent::StreamTimingChange(stream_timing_payload));
        }
    }

    fn try_send_event(&mut self, event: PlayerEvent) {
        if let Err(e) = self.event_tx.blocking_send(event.clone()) {
            // TODO: Decide on error log level policy
            warn!("Failed to send {event:?} from the manager with {e:?}");
        }
    }
}

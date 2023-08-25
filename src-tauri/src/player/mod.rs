use std::{cmp::Ordering, thread, time::Duration};

use creek::{Decoder, ReadDiskStream, ReadStreamOptions, SymphoniaDecoder};
use log::{info, warn};
use rtrb::RingBuffer;
use rubato::{FftFixedOut, Resampler, VecResampler};
use std::sync::mpsc;

use self::output::Output;

mod output;
mod process;

type ResampleBuffer = Vec<Vec<f32>>;

struct ProcessResampler {
    resampler: FftFixedOut<f32>,
    in_buffer: ResampleBuffer,
    out_buffer: ResampleBuffer,
}

#[allow(clippy::large_enum_variant)]
pub enum GuiToProcessMsg {
    StartPlayback(ReadDiskStream<SymphoniaDecoder>, Option<ProcessResampler>),
    Pause,
    Resume,
    SetGain(f32),
    SeekTo(usize),
}

pub enum ProcessToGuiMsg {
    PlaybackPos(usize),
    Buffering,
    PlaybackEnded,
    DisposeResamplerBuffers(ProcessResampler),
}

enum ManagerCommand {
    StartPlayback(Vec<String>),
    Pause,
    PlaybackPos(usize),
    PlaybackEnded,
    Buffering,
    Resume,
    SetVolume(f64),
}

fn gain_for_volume(volume: f64) -> f32 {
    let clamped = volume.max(0_f64).min(100_f64);
    let normalized = clamped / 100.0;
    let amp = normalized.powf(2.7);
    (amp as f32).min(1.0)
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

const NUM_CACHE_BLOCKS: usize = 20;

#[allow(unused)]
const CACHE_SIZE: usize = NUM_CACHE_BLOCKS * SymphoniaDecoder::DEFAULT_BLOCK_SIZE;

struct PlaybackManager {
    output: Output,
    to_process_tx: rtrb::Producer<GuiToProcessMsg>,
    command_rx: mpsc::Receiver<ManagerCommand>,
    stream_frame_count: Option<usize>,
    queue: Option<Queue<String>>,
}

impl PlaybackManager {
    pub fn new(
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
                            ProcessToGuiMsg::PlaybackPos(pos) => {
                                Some(ManagerCommand::PlaybackPos(pos))
                            }
                            ProcessToGuiMsg::PlaybackEnded => Some(ManagerCommand::PlaybackEnded),
                            // Special message, just deallocate the resource
                            ProcessToGuiMsg::DisposeResamplerBuffers(_) => None,
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
                ManagerCommand::PlaybackPos(_pos) => {
                    // debug!("Played up to frame {_pos:?}");
                }
                ManagerCommand::PlaybackEnded => {
                    self.queue = self.queue.and_then(|queue| queue.go_next());
                    if let Some(queue) = self.queue.as_ref() {
                        self.start_stream(queue.current().to_owned());
                    }
                }
                ManagerCommand::SetVolume(volume) => {
                    let gain = gain_for_volume(volume);
                    self.to_process_tx
                        .push(GuiToProcessMsg::SetGain(gain))
                        .unwrap_or_else(|_| {
                            warn!("Failed to send gain message to audio thread");
                        })
                }
            }
        }
    }

    fn start_stream(&mut self, path: String) {
        // Setup read stream -------------------------------------------------------------

        let opts = ReadStreamOptions {
            // The number of prefetch blocks in a cache block. This will cause a cache to be
            // used whenever the stream is seeked to a frame in the range:
            //
            // `[cache_start, cache_start + (num_cache_blocks * block_size))`
            //
            // If this is 0, then the cache is only used when seeked to exactly `cache_start`.
            num_cache_blocks: NUM_CACHE_BLOCKS,

            // The maximum number of caches that can be active in this stream. Keep in mind each
            // cache uses some memory (but memory is only allocated when the cache is created).
            //
            // The default is `1`.
            num_caches: 1,

            ..Default::default()
        };

        info!("Starting stream for {:?}", path);

        let mut read_stream = ReadDiskStream::<SymphoniaDecoder>::new(path, 0, opts).unwrap();

        let file_info = read_stream.info();

        let mut process_resampler: Option<ProcessResampler> = None;
        if let Some(sample_rate) = file_info.sample_rate {
            if sample_rate != self.output.sample_rate {
                let num_channels = file_info.num_channels;
                let resampler: FftFixedOut<f32> = FftFixedOut::new(
                    sample_rate as usize,
                    self.output.sample_rate as usize,
                    self.output.buffer_size as usize,
                    // TODO: Investigate this parameter
                    2,
                    num_channels as usize,
                )
                // TODO: Error handling
                .expect("Failed to initialize resampler");
                let in_buffer = Resampler::input_buffer_allocate(&resampler, true);
                let out_buffer = Resampler::output_buffer_allocate(&resampler, true);
                process_resampler = Some(ProcessResampler {
                    resampler,
                    in_buffer,
                    out_buffer,
                });
            }
        }

        self.stream_frame_count = Some(file_info.num_frames);

        // Cache the start of the file into cache with index `0`.
        let _ = read_stream.cache(0, 0);

        // Tell the stream to seek to the beginning of file. This will also alert the stream to the existence
        // of the cache with index `0`.
        read_stream.seek(0, Default::default()).unwrap();

        self.to_process_tx
            .push(GuiToProcessMsg::StartPlayback(
                read_stream,
                process_resampler,
            ))
            .unwrap();
    }
}

pub struct Player {
    command_tx: mpsc::Sender<ManagerCommand>,
}

impl Player {
    pub fn new() -> Player {
        let (command_tx, rx) = mpsc::channel();
        thread::spawn({
            let tx = command_tx.clone();
            // TODO: Fix up builder pattern for PlaybackManager
            move || PlaybackManager::new(tx, rx).run()
        });
        Player { command_tx }
    }

    pub fn start_playback(&mut self, file_paths: &[String]) {
        self.command_tx
            .send(ManagerCommand::StartPlayback(Vec::from(file_paths)))
            .unwrap_or_else(|_| {
                warn!("Failed to send start playback command to the manager");
            })
    }

    pub fn pause(&mut self) {
        self.command_tx
            .send(ManagerCommand::Pause)
            .unwrap_or_else(|_| warn!("Failed to send pause command to the manager"))
    }

    pub fn play(&mut self) {
        self.command_tx
            .send(ManagerCommand::Resume)
            .unwrap_or_else(|_| warn!("Failed to send resume command to the manager"))
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.command_tx
            .send(ManagerCommand::SetVolume(volume))
            .unwrap_or_else(|_| warn!("Failed to send volume command to the manager"))
    }
}

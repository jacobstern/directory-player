use std::{thread, time::Duration};

use creek::{Decoder, ReadDiskStream, ReadStreamOptions, SymphoniaDecoder};
use log::{info, warn};
use rtrb::RingBuffer;
use std::sync::mpsc;

mod output;
mod process;

#[allow(clippy::large_enum_variant)]
pub enum GuiToProcessMsg {
    StartPlayback(ReadDiskStream<SymphoniaDecoder>),
    Pause,
    Stop,
    Restart,
    SeekTo(usize),
}

pub enum ProcessToGuiMsg {
    PlaybackPos(usize),
    Buffering,
    PlaybackEnded,
}

enum ManagerCommand {
    StartPlayback(Vec<String>),
    Pause,
    PlaybackPos(usize),
    PlaybackEnded,
    Buffering,
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
    _stream: cpal::Stream,
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
        let cpal_stream = output::start_stream(to_gui_tx, from_gui_rx);

        thread::spawn({
            let tx = command_tx.clone();
            move || {
                let mut failed_to_send = false;
                while !failed_to_send {
                    while let Ok(msg) = from_process_rx.pop() {
                        let result = tx.send(match msg {
                            ProcessToGuiMsg::Buffering => ManagerCommand::Buffering,
                            ProcessToGuiMsg::PlaybackPos(pos) => ManagerCommand::PlaybackPos(pos),
                            ProcessToGuiMsg::PlaybackEnded => ManagerCommand::PlaybackEnded,
                        });
                        failed_to_send = result.is_err();
                        if failed_to_send {
                            break;
                        }
                    }
                    if !failed_to_send {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });

        PlaybackManager {
            _stream: cpal_stream,
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
                ManagerCommand::Buffering => {}
                ManagerCommand::PlaybackPos(_pos) => {}
                ManagerCommand::PlaybackEnded => {
                    self.queue = self.queue.and_then(|queue| queue.go_next());
                    if let Some(queue) = self.queue.as_ref() {
                        self.start_stream(queue.current().to_owned());
                    }
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
            num_caches: 2,
            ..Default::default()
        };

        info!("Starting stream for {:?}", path);

        // Open the read stream.
        let mut read_stream = ReadDiskStream::<SymphoniaDecoder>::new(path, 0, opts).unwrap();

        // Cache the start of the file into cache with index `0`.
        let _ = read_stream.cache(0, 0);

        // Tell the stream to seek to the beginning of file. This will also alert the stream to the existence
        // of the cache with index `0`.
        read_stream.seek(0, Default::default()).unwrap();

        // Wait until the buffer is filled before sending it to the process thread.
        // read_stream.block_until_ready().unwrap();

        self.stream_frame_count = Some(read_stream.info().num_frames);

        self.to_process_tx
            .push(GuiToProcessMsg::StartPlayback(read_stream))
            .unwrap();
    }
}

// pub async fn run_manager()

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
}

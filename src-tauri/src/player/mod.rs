use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use creek::{Decoder, ReadDiskStream, ReadStreamOptions, SymphoniaDecoder};
use log::warn;
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
}

enum ManagerCommand {
    StartPlayback(String),
    Pause,
    PlaybackPos(usize),
    Buffering,
}

struct PlaybackManager {
    stream: cpal::Stream,
    to_process_tx: rtrb::Producer<GuiToProcessMsg>,
    from_process_rx: rtrb::Consumer<ProcessToGuiMsg>,
    command_tx: mpsc::Sender<ManagerCommand>,
    command_rx: mpsc::Receiver<ManagerCommand>,
    stream_frame_count: Option<usize>,
    cache_size: usize,
}

impl PlaybackManager {
    pub fn new(
        command_tx: mpsc::Sender<ManagerCommand>,
        command_rx: mpsc::Receiver<ManagerCommand>,
    ) -> PlaybackManager {
        let (to_gui_tx, from_process_rx) = RingBuffer::<ProcessToGuiMsg>::new(256);
        let (to_process_tx, from_gui_rx) = RingBuffer::<GuiToProcessMsg>::new(64);
        let cpal_stream = output::start_stream(to_gui_tx, from_gui_rx);

        PlaybackManager {
            stream: cpal_stream,
            to_process_tx,
            from_process_rx,
            command_tx,
            command_rx,
            stream_frame_count: None,
            cache_size: 0,
        }
    }

    pub fn run(mut self) {
        let mut from_process_rx = self.from_process_rx;

        thread::spawn({
            let tx = self.command_tx.clone();
            move || {
                let mut failed_to_send = false;
                while !failed_to_send {
                    while let Ok(msg) = from_process_rx.pop() {
                        match msg {
                            ProcessToGuiMsg::Buffering => {
                                if tx.send(ManagerCommand::Buffering).is_err() {
                                    failed_to_send = true;
                                    break;
                                }
                            }
                            ProcessToGuiMsg::PlaybackPos(pos) => {
                                if tx.send(ManagerCommand::PlaybackPos(pos)).is_err() {
                                    failed_to_send = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });

        while let Ok(msg) = self.command_rx.recv() {
            match msg {
                ManagerCommand::StartPlayback(file_path) => {
                    // Setup read stream -------------------------------------------------------------

                    let opts = ReadStreamOptions {
                        // The number of prefetch blocks in a cache block. This will cause a cache to be
                        // used whenever the stream is seeked to a frame in the range:
                        //
                        // `[cache_start, cache_start + (num_cache_blocks * block_size))`
                        //
                        // If this is 0, then the cache is only used when seeked to exactly `cache_start`.
                        num_cache_blocks: 20,

                        // The maximum number of caches that can be active in this stream. Keep in mind each
                        // cache uses some memory (but memory is only allocated when the cache is created).
                        //
                        // The default is `1`.
                        num_caches: 2,
                        ..Default::default()
                    };

                    self.cache_size = opts.num_cache_blocks * SymphoniaDecoder::DEFAULT_BLOCK_SIZE;

                    // Open the read stream.
                    let mut read_stream =
                        ReadDiskStream::<SymphoniaDecoder>::new(file_path, 0, opts).unwrap();

                    // Cache the start of the file into cache with index `0`.
                    let _ = read_stream.cache(0, 0);

                    // Tell the stream to seek to the beginning of file. This will also alert the stream to the existence
                    // of the cache with index `0`.
                    read_stream.seek(0, Default::default()).unwrap();

                    // Wait until the buffer is filled before sending it to the process thread.
                    // read_stream.block_until_ready().unwrap();

                    // ------------------------------------------------------------------------------

                    self.stream_frame_count = Some(read_stream.info().num_frames);

                    self.to_process_tx
                        .push(GuiToProcessMsg::StartPlayback(read_stream))
                        .unwrap();
                }
                ManagerCommand::Pause => self
                    .to_process_tx
                    .push(GuiToProcessMsg::Pause)
                    .unwrap_or_else(|_| {
                        warn!("Failed to send pause message to audio thread");
                    }),
                ManagerCommand::Buffering => {}
                ManagerCommand::PlaybackPos(_pos) => {}
            }
        }
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
            move || PlaybackManager::new(tx, rx).run()
        });
        Player { command_tx }
    }

    pub fn start_playback(&mut self, file_paths: &[String]) {
        let file_path = file_paths[0].to_owned();
        self.command_tx
            .send(ManagerCommand::StartPlayback(file_path))
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

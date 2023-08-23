use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use creek::{Decoder, ReadDiskStream, ReadStreamOptions, SymphoniaDecoder};
use log::warn;
use rtrb::RingBuffer;
use std::sync::mpsc;

mod output;
mod process;

#[allow(clippy::large_enum_variant)]
pub enum GuiToProcessMsg {
    UseStream(ReadDiskStream<SymphoniaDecoder>),
    PlayResume,
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
    Play(String),
}

struct PlaybackManager {
    stream: cpal::Stream,
    to_process_tx: rtrb::Producer<GuiToProcessMsg>,
    command_rx: mpsc::Receiver<ManagerCommand>,
}

impl PlaybackManager {
    pub fn new(command_rx: mpsc::Receiver<ManagerCommand>) -> PlaybackManager {
        let (to_gui_tx, _from_process_rx) = RingBuffer::<ProcessToGuiMsg>::new(256);
        let (to_process_tx, from_gui_rx) = RingBuffer::<GuiToProcessMsg>::new(64);
        let cpal_stream = output::create_stream(to_gui_tx, from_gui_rx);

        PlaybackManager {
            stream: cpal_stream,
            to_process_tx,
            command_rx,
        }
    }

    pub fn run(&mut self) {
        while let Ok(msg) = self.command_rx.recv() {
            match msg {
                ManagerCommand::Play(file_path) => {
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

                    // This is how to calculate the total size of a cache block.
                    let cache_size = opts.num_cache_blocks * SymphoniaDecoder::DEFAULT_BLOCK_SIZE;

                    // Open the read stream.
                    let mut read_stream =
                        ReadDiskStream::<SymphoniaDecoder>::new(file_path, 0, opts).unwrap();

                    // Cache the start of the file into cache with index `0`.
                    let _ = read_stream.cache(0, 0);

                    // Tell the stream to seek to the beginning of file. This will also alert the stream to the existence
                    // of the cache with index `0`.
                    read_stream.seek(0, Default::default()).unwrap();

                    // Wait until the buffer is filled before sending it to the process thread.
                    read_stream.block_until_ready().unwrap();

                    // ------------------------------------------------------------------------------

                    let num_frames = read_stream.info().num_frames;

                    self.to_process_tx
                        .push(GuiToProcessMsg::UseStream(read_stream))
                        .unwrap();
                    self.to_process_tx
                        .push(GuiToProcessMsg::PlayResume)
                        .unwrap();
                }
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
        thread::spawn(move || PlaybackManager::new(rx).run());
        Player { command_tx }
    }

    pub fn start_playback(&mut self, file_paths: &[String]) {
        let file_path = file_paths[0].to_owned();
        self.command_tx
            .send(ManagerCommand::Play(file_path))
            .unwrap_or_else(|_| {
                warn!("Failed to send play command to the manager");
            })
    }
}

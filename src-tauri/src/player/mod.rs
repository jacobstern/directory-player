use std::thread;

// use creek::{ReadDiskStream, SymphoniaDecoder};
use log::{error, warn};
use rubato::FftFixedOut;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

use self::{
    file_stream::FileStream,
    manager::{ManagerCommand, PlaybackManager},
};

mod errors;
mod file_stream;
mod manager;
mod output;
mod process;
mod resampler;

pub type ResampleBuffer = Vec<Vec<f32>>;

pub struct ProcessResampler {
    resampler: FftFixedOut<f32>,
    in_buffer: ResampleBuffer,
    out_buffer: ResampleBuffer,
}

#[allow(clippy::large_enum_variant)]
pub enum GuiToProcessMsg {
    StartPlayback(FileStream),
    Pause,
    Resume,
    SetGain(f32),
    SeekTo(usize),
}

#[allow(clippy::large_enum_variant)]
pub enum ProcessToGuiMsg {
    Progress(usize),
    Buffering,
    PlaybackEnded,
    // DisposeResamplerBuffers(ProcessResampler),
    // DidSeek,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrackInfo {
    pub path: String,
    pub duration: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayerEvent {
    Progress(usize),
    Track(TrackInfo),
    DidSeek,
}

pub struct Player {
    command_tx: mpsc::Sender<ManagerCommand>,
}

impl Player {
    pub fn new(event_tx: tokio::sync::mpsc::Sender<PlayerEvent>) -> Player {
        let (command_tx, rx) = mpsc::channel();
        thread::spawn({
            let tx = command_tx.clone();
            move || PlaybackManager::new(event_tx, tx, rx).run()
        });
        Player { command_tx }
    }

    pub fn start_playback(&mut self, file_paths: &[String]) {
        self.command_tx
            .send(ManagerCommand::StartPlayback(Vec::from(file_paths)))
            .unwrap_or_else(|_| {
                warn!("Failed to send start playback command to the manager");
            });
    }

    pub fn pause(&mut self) {
        self.command_tx
            .send(ManagerCommand::Pause)
            .unwrap_or_else(|_| warn!("Failed to send pause command to the manager"));
    }

    pub fn play(&mut self) {
        self.command_tx
            .send(ManagerCommand::Resume)
            .unwrap_or_else(|_| warn!("Failed to send resume command to the manager"));
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.command_tx
            .send(ManagerCommand::SetVolume(volume))
            .unwrap_or_else(|_| warn!("Failed to send volume command to the manager"));
    }

    pub fn seek(&mut self, offset: usize) {
        self.command_tx
            .send(ManagerCommand::SeekTo(offset))
            .unwrap_or_else(|_| error!("Failed to send seek command to the manager"));
    }
}

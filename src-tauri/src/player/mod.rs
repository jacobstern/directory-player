use std::thread;

use log::warn;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

use self::{
    file_stream::FileStream,
    manager::{ManagerCommand, PlaybackManager},
};

mod decode_worker;
mod errors;
mod file_stream;
mod manager;
mod output;
mod process;

pub enum StartPlaybackState {
    Playing,
    Paused,
}

pub enum ManagerToProcessMsg {
    StartPlayback(FileStream, StartPlaybackState),
    Pause,
    Resume,
    Stop,
    SetGain(f32),
    SeekTo(usize),
}

#[derive(Debug)]
pub enum ProcessToManagerMsg {
    PlaybackPos(usize),
    PlaybackEnded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaybackFile {
    path: String,
    name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StreamTiming {
    pub duration: u64,
    pub pos: usize,
    pub duration_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayerEvent {
    PlaybackFileChange(Option<PlaybackFile>),
    PlaybackStateChange(PlaybackState),
    StreamTimingChange(Option<StreamTiming>),
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

    pub fn start_playback(&mut self, file_paths: &[String], start_index: usize) {
        self.command_tx
            .send(ManagerCommand::StartPlayback(
                Vec::from(file_paths),
                start_index,
            ))
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
            .unwrap_or_else(|_| warn!("Failed to send seek command to the manager"));
    }

    pub fn skip_forward(&mut self) {
        self.command_tx
            .send(ManagerCommand::SkipForward)
            .unwrap_or_else(|_| warn!("Failed to send skip forward command to the manager"));
    }

    pub fn skip_back(&mut self) {
        self.command_tx
            .send(ManagerCommand::SkipBack)
            .unwrap_or_else(|_| warn!("Failed to send skip back command to the manager"));
    }
}

use std::{fs::File, thread};

use log::{error, warn};
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

pub enum GuiToProcessMsg {
    StartPlayback(FileStream),
    Pause,
    Resume,
    Stop,
    SetGain(f32),
    SeekTo(usize),
}

#[derive(Debug)]
pub enum ProcessToGuiMsg {
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
pub struct TrackInfo {
    pub path: String,
    pub duration: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaybackFile {
    path: String,
    name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlayerEvent {
    Progress(usize),
    Track(TrackInfo),
    PlaybackFileChange(Option<PlaybackFile>),
    PlaybackStateChange(PlaybackState),
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
            .unwrap_or_else(|_| warn!("Failed to send seek command to the manager"));
    }

    pub fn skip_forward(&mut self) {
        self.command_tx
            .send(ManagerCommand::SkipForward)
            .unwrap_or_else(|_| warn!("Failed to send skip forward command to the manager"));
    }
}

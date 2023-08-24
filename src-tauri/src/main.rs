// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod player;

use player::Player;
use serde::Serialize;
use std::fs::{DirEntry, ReadDir};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
enum TreeviewItem {
    Directory {
        name: String,
        path: String,
        children: Vec<TreeviewItem>,
        #[serde(rename = "isExpanded")]
        is_expanded: bool,
    },
    File {
        name: String,
        path: String,
        #[serde(rename = "canPlay")]
        can_play: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TreeviewView {
    listing: Vec<TreeviewItem>,
}

struct PlayerState(Arc<Mutex<Player>>);

struct FileNameAndPath {
    name: String,
    path: String,
}

fn get_utf8_name_and_path(entry: &DirEntry) -> Option<FileNameAndPath> {
    let path = entry.path().to_str()?.to_owned();
    let name = entry.file_name().to_str()?.to_owned();
    Some(FileNameAndPath { name, path })
}

fn treeview_item_name_and_path(item: &TreeviewItem) -> FileNameAndPath {
    match item {
        TreeviewItem::Directory { name, path, .. } => FileNameAndPath {
            name: name.clone(),
            path: path.clone(),
        },
        TreeviewItem::File { name, path, .. } => FileNameAndPath {
            name: name.clone(),
            path: path.clone(),
        },
    }
}

fn build_directory_listing(entries: ReadDir) -> Vec<TreeviewItem> {
    // TODO: Handle and surface IO errors
    let mut listing: Vec<TreeviewItem> = Vec::new();
    for entry in entries.flatten() {
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                let valid = get_utf8_name_and_path(&entry);
                if let Some(FileNameAndPath { name, path }) = valid {
                    listing.push(TreeviewItem::Directory {
                        name,
                        path,
                        children: vec![],
                        is_expanded: false,
                    });
                }
            } else if file_type.is_file() {
                let parsed = get_utf8_name_and_path(&entry);
                if let Some(FileNameAndPath { name, path }) = parsed {
                    if name.starts_with('.') {
                        continue;
                    }
                    let can_play = [".mp3", ".flac", ".wav", ".ogg"]
                        .iter()
                        .any(|ext| name.ends_with(ext));
                    listing.push(TreeviewItem::File {
                        name,
                        path,
                        can_play,
                    });
                }
            }
        }
    }
    listing.sort_unstable_by(|a, b| {
        treeview_item_name_and_path(a)
            .name
            .cmp(&treeview_item_name_and_path(b).name)
    });
    listing
}

#[tauri::command]
fn treeview_get_view() -> TreeviewView {
    let path = Path::new("/Users/jacob/Library/CloudStorage/OneDrive-Personal/Music");
    let entries = path.read_dir().unwrap();
    let listing = build_directory_listing(entries);
    TreeviewView { listing }
}

#[tauri::command]
fn treeview_expand_directory(directory_path: String) -> TreeviewItem {
    let path = Path::new(&directory_path);
    let entries = path.read_dir().unwrap();
    let listing = build_directory_listing(entries);
    let name = path.file_name().unwrap().to_str().unwrap().to_owned();
    TreeviewItem::Directory {
        name,
        path: directory_path,
        children: listing,
        is_expanded: true,
    }
}

#[tauri::command]
fn treeview_collapse_directory(directory_path: String) -> TreeviewItem {
    let path = Path::new(&directory_path);
    let name = path.file_name().unwrap().to_str().unwrap().to_owned();
    TreeviewItem::Directory {
        name,
        path: directory_path,
        children: vec![],
        is_expanded: false,
    }
}

#[tauri::command]
fn player_play(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().play();
}

#[tauri::command]
fn player_pause(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().pause();
}

#[tauri::command]
fn player_start_playback(file_paths: Vec<String>, player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().start_playback(&file_paths);
}

#[tauri::command]
fn player_set_volume(volume: f64, player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().set_volume(volume);
}

#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    window.get_window("main").unwrap().show().unwrap();
}

fn main() {
    let player = Player::new();
    let shared_player = Arc::new(Mutex::new(player));
    tauri::Builder::default()
        .manage(PlayerState(shared_player))
        .invoke_handler(tauri::generate_handler![
            treeview_get_view,
            treeview_expand_directory,
            treeview_collapse_directory,
            player_play,
            player_pause,
            player_start_playback,
            player_set_volume,
            show_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

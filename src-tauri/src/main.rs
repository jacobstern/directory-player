// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde;
use serde::Serialize;
use std::fs::ReadDir;
use std::path::Path;
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

fn build_directory_listing(read: ReadDir) -> Vec<TreeviewItem> {
    // TODO: Handle and surface IO errors
    let mut listing: Vec<TreeviewItem> = Vec::new();
    for result in read {
        if let Ok(entry) = result {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    listing.push(TreeviewItem::Directory {
                        name: entry.file_name().to_str().unwrap().to_owned(),
                        path: entry.path().to_str().unwrap().to_owned(),
                        children: vec![],
                        is_expanded: false,
                    });
                } else if file_type.is_file() {
                    let name = entry.file_name().to_str().unwrap().to_owned();
                    let can_play = vec![".mp3", ".flac", ".wav"]
                        .iter()
                        .any(|ext| name.ends_with(ext));
                    listing.push(TreeviewItem::File {
                        name,
                        path: entry.path().to_str().unwrap().to_owned(),
                        can_play,
                    });
                }
            }
        }
    }
    listing
}

#[tauri::command]
fn treeview_get_view() -> TreeviewView {
    let path = Path::new("/Users/jacob/Library/CloudStorage/OneDrive-Personal/Music");
    let read = path.read_dir().unwrap();
    let listing = build_directory_listing(read);
    TreeviewView { listing }
}

#[tauri::command]
fn treeview_expand_directory(directory_path: String) -> TreeviewItem {
    let path = Path::new(&directory_path);
    let read = path.read_dir().unwrap();
    let listing = build_directory_listing(read);
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
fn player_play() {
    todo!();
}

#[tauri::command]
fn player_pause() {
    todo!();
}

#[tauri::command]
fn player_start_playback(file_paths: Vec<String>) {
    todo!();
}

#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    window.get_window("main").unwrap().show().unwrap();
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            treeview_get_view,
            treeview_expand_directory,
            treeview_collapse_directory,
            player_play,
            player_pause,
            player_start_playback,
            show_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

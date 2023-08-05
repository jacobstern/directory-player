// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde;
use serde::Serialize;
use std::fs::{DirEntry, ReadDir};
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

struct FileNameAndPath {
    name: String,
    path: String,
}

fn parse_name_and_path(entry: &DirEntry) -> Option<FileNameAndPath> {
    let name = entry.file_name().to_str()?.to_owned();
    let path = entry.path().to_str()?.to_owned();
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
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let parsed = parse_name_and_path(&entry);
                    if let Some(FileNameAndPath { name, path }) = parsed {
                        listing.push(TreeviewItem::Directory {
                            name,
                            path,
                            children: vec![],
                            is_expanded: false,
                        });
                    }
                } else if file_type.is_file() {
                    let parsed = parse_name_and_path(&entry);
                    if let Some(FileNameAndPath { name, path }) = parsed {
                        if name.starts_with(".") {
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
    }
    listing.sort_unstable_by(|a, b| {
        treeview_item_name_and_path(&a)
            .name
            .partial_cmp(&treeview_item_name_and_path(&b).name)
            .unwrap()
    });
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
    gstreamer::init().expect("failed to initialize GStreamer");
    println!("{}", gstreamer::version_string());

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

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum TreeviewItem {
    Directory {
        path: String,
        name: String,
        is_expanded: bool,
        children: Vec<TreeviewItem>,
    },
    File {
        path: String,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TreeviewView {
    listing: Vec<TreeviewItem>,
}

#[tauri::command]
fn treeview_get_view() -> TreeviewView {
    let mut listing: Vec<TreeviewItem> = Vec::new();
    let path = Path::new("/Users/jacob/Library/CloudStorage/OneDrive-Personal/Music");
    let contents = path.read_dir().unwrap();
    for result in contents {
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
                    listing.push(TreeviewItem::File {
                        name: entry.file_name().to_str().unwrap().to_owned(),
                        path: entry.path().to_str().unwrap().to_owned(),
                    });
                }
            }
        }
    }
    TreeviewView { listing }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![treeview_get_view])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

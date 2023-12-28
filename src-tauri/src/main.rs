// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod player;

use log::warn;
use player::{Player, PlayerEvent, RepeatMode, ShuffleMode};
use serde::Serialize;
use std::{process::Command, sync::Mutex};
use tauri::{
    async_runtime, AboutMetadata, AppHandle, CustomMenuItem, Manager, Menu, MenuItem, Submenu,
};

struct PlayerState(Mutex<Player>);

#[tauri::command]
fn player_play(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().play();
}

#[tauri::command]
fn player_pause(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().pause();
}

#[tauri::command]
fn player_start_playback(
    file_paths: Vec<String>,
    start_index: usize,
    player_state: tauri::State<PlayerState>,
) {
    player_state
        .0
        .lock()
        .unwrap()
        .start_playback(&file_paths, start_index);
}

#[tauri::command]
fn player_set_volume(volume: f64, player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().set_volume(volume);
}

#[tauri::command]
fn player_seek(offset: usize, player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().seek(offset);
}

#[tauri::command]
fn player_skip_forward(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().skip_forward();
}

#[tauri::command]
fn player_skip_back(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().skip_back();
}

#[tauri::command]
fn player_stop(player_state: tauri::State<PlayerState>) {
    player_state.0.lock().unwrap().stop();
}

#[tauri::command]
fn player_set_shuffle_mode(player_state: tauri::State<PlayerState>, shuffle_mode: ShuffleMode) {
    player_state
        .0
        .lock()
        .unwrap()
        .set_shuffle_mode(shuffle_mode);
}

#[tauri::command]
fn player_set_repeat_mode(player_state: tauri::State<PlayerState>, repeat_mode: RepeatMode) {
    player_state.0.lock().unwrap().set_repeat_mode(repeat_mode);
}

#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    window.get_window("main").unwrap().show().unwrap();
}

/// Show a file in its containing folder a la "Reveal in Finder" in VS Code.
///
/// Source: https://github.com/tauri-apps/tauri/issues/4062#issuecomment-1338048169
#[tauri::command]
fn show_in_folder(path: String) {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path]) // The comma after select is not a typo
            .spawn()
            .unwrap();
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").args(["-R", &path]).spawn().unwrap();
    }
}

fn try_emit_all<S: Serialize + Clone>(app_handle: &AppHandle, event: &str, payload: S) {
    app_handle.emit_all(event, payload).unwrap_or_else(|e| {
        warn!("Failed to emit {event} with {e:?}");
    });
}

async fn poll_player_events(
    app_handle: AppHandle,
    mut player_event_rx: async_runtime::Receiver<PlayerEvent>,
) {
    while let Some(msg) = player_event_rx.recv().await {
        match msg {
            PlayerEvent::PlaybackFileChange(file) => {
                try_emit_all(&app_handle, "player://playback-file-change", file);
            }
            PlayerEvent::PlaybackStateChange(state) => {
                try_emit_all(&app_handle, "player://playback-state-change", state);
            }
            PlayerEvent::StreamTimingChange(timing) => {
                try_emit_all(&app_handle, "player://stream-timing-change", timing);
            }
            PlayerEvent::StreamMetadataChange(metadata) => {
                try_emit_all(&app_handle, "player://stream-metadata-change", metadata);
            }
        }
    }
}

fn build_menu(app_name: &str) -> Menu {
    let file_menu = Menu::new()
        .add_item(CustomMenuItem::new("open", "Open Folder...").accelerator("CommandOrControl+O"));
    let edit_menu = Menu::new()
        .add_native_item(MenuItem::Undo)
        .add_native_item(MenuItem::Redo)
        .add_native_item(MenuItem::Separator)
        .add_native_item(MenuItem::Cut)
        .add_native_item(MenuItem::Copy)
        .add_native_item(MenuItem::Paste)
        .add_native_item(MenuItem::SelectAll);
    Menu::new()
        .add_submenu(Submenu::new(
            app_name,
            Menu::new()
                .add_native_item(MenuItem::About(
                    app_name.to_string(),
                    AboutMetadata::default(),
                ))
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Services)
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Hide)
                .add_native_item(MenuItem::HideOthers)
                .add_native_item(MenuItem::ShowAll)
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Quit),
        ))
        .add_submenu(Submenu::new("File", file_menu))
        .add_submenu(Submenu::new("Edit", edit_menu))
}

fn main() {
    let (player_event_tx, player_event_rx) = async_runtime::channel(1024);
    let player = Player::new(player_event_tx);
    let menu = build_menu("directory-player");

    tauri::Builder::default()
        .menu(menu)
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_fs_watch::init())
        .plugin(tauri_plugin_context_menu::init())
        .manage(PlayerState(Mutex::new(player)))
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            player_play,
            player_pause,
            player_stop,
            player_start_playback,
            player_set_volume,
            player_seek,
            player_skip_forward,
            player_skip_back,
            player_set_shuffle_mode,
            player_set_repeat_mode,
            show_in_folder
        ])
        .setup(|app| {
            async_runtime::spawn(poll_player_events(app.handle(), player_event_rx));
            Ok(())
        })
        .on_menu_event(|event| {
            event
                .window()
                .emit("app://menu-event", event.menu_item_id())
                .unwrap_or_else(|_| {
                    warn!("Failed to emit menu-event");
                });
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

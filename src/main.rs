mod media_status;
mod ui;

use async_std::{prelude::StreamExt, task};

use media_status::{TrackInfo, WindowsMediaEventListener};

use crate::ui::Ui;

fn copy_current_track_info_to_clipboard() -> Result<(), String> {
    match TrackInfo::get_current_track() {
        Some(track) => {
            // Format a string
            let np_str = format!("{} - {} [{}]", track.artist, track.title, track.album);
            println!("{}", np_str);
            // Copy to clipboard
            clipboard_win::set_clipboard_string(&np_str)
                .or(Err("Clipboard setting failed".to_string()))
        }
        None => Err("No currently playing track".to_string()),
    }
}

async fn run() {
    let listener = WindowsMediaEventListener::new().unwrap();

    let mut ui = Ui::new();

    while let Some(ev) = listener.get_media_events().next().await {
        match ev {
            media_status::MediaEvent::SourceAppChanged(aumid) => {
                ui.update_source(aumid);
            }
            media_status::MediaEvent::PlayStatusChanged(status) => {
                ui.update_status(Some(status));
            }
            media_status::MediaEvent::PositionChanged { current, length } => {
                ui.update_timeline(Some(current), Some(length));
            }
            media_status::MediaEvent::InfoChanged(info) => {
                ui.update_track(Some(info));
            }
        }
    }
}

fn main() {
    if std::env::args().find(|s| s == "monitor").is_some() {
        task::block_on(run());
    } else {
        copy_current_track_info_to_clipboard().unwrap();
        println!("Copied to clipboard");
    }
}

mod media_status;
mod ui;

use async_std::{prelude::StreamExt, task};

use media_status::WindowsMediaEventListener;

use crate::ui::Ui;

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
    task::block_on(run())
}

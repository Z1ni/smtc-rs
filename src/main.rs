mod media_status;

use async_std::{prelude::StreamExt, task};

use media_status::WindowsMediaEventListener;

async fn run() {
    let listener = WindowsMediaEventListener::new().unwrap();

    while let Some(ev) = listener.get_media_events().next().await {
        todo!();
    }
}

fn main() {
    task::block_on(run())
}

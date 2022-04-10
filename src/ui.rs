use std::{io::Write, time::Duration};

use crate::media_status::{PlayStatus, TrackInfo};

pub struct Ui {
    aumid: Option<String>,
    info: Option<TrackInfo>,
    status: Option<PlayStatus>,
    position: Option<Duration>,
    length: Option<Duration>,
}

impl Ui {
    pub fn new() -> Ui {
        // Reserve two lines of space
        println!();

        // Move back to the first line
        print!("\x1B[1A");
        // Save cursor position
        print!("\x1B[s");

        Ui {
            aumid: None,
            info: None,
            status: None,
            position: None,
            length: None,
        }
    }

    pub fn update_track(&mut self, track: Option<TrackInfo>) {
        self.info = track;
        self.print_ui();
    }

    pub fn update_timeline(&mut self, position: Option<Duration>, length: Option<Duration>) {
        self.position = position;
        self.length = length;
        self.print_ui();
    }

    pub fn update_status(&mut self, status: Option<PlayStatus>) {
        self.status = status;
        self.print_ui();
    }

    pub fn update_source(&mut self, aumid: Option<String>) {
        self.aumid = aumid;
        self.print_ui();
    }

    fn to_mm_ss(dur: &Duration) -> (u64, u64) {
        let total_sec = dur.as_secs();
        let min = total_sec / 60;
        let sec = total_sec % 60;

        (min, sec)
    }

    fn to_status_char(status_opt: &Option<PlayStatus>) -> char {
        match status_opt {
            /* media_status::PlayStatus::Playing => '\u{23F5}', // '⏵︎'
            media_status::PlayStatus::Paused => '\u{23F8}',  // '⏸︎'
            media_status::PlayStatus::Stopped => '\u{23F9}', // '⏹︎' */
            Some(PlayStatus::Playing) => '>',
            Some(PlayStatus::Paused) => '|',
            Some(PlayStatus::Stopped) => '_',
            _ => '?',
        }
    }

    fn print_ui(&self) {
        // Restore cursor position to the beginning of the title line
        print!("\x1B[u");
        // Erase the line
        print!("\x1B[2K");
        // Print out info
        if let Some(aumid) = &self.aumid {
            if let Some(track) = &self.info {
                print!(
                    "[{}] {} - {} [{}]\r",
                    aumid, track.artist, track.title, track.album
                );
            }
        } else {
            println!("No active playback session");
            print!("\x1B[2K");
            std::io::stdout().flush().unwrap();
            return;
        }

        // Move one line down to the status line
        print!("\x1B[1B");
        // Erase the line
        print!("\x1B[2K");

        let has_pos = self.position.is_some();
        let has_len = self.length.is_some();

        let status_char = Ui::to_status_char(&self.status);

        if !has_pos || !has_len {
            print!("{} No position/length info", status_char);
            std::io::stdout().flush().unwrap();
            return;
        }

        let position = self.position.unwrap();
        let length = self.length.unwrap();

        let pos_per = position.as_nanos() as f32 / length.as_nanos() as f32;

        let (cur_m, cur_s) = Ui::to_mm_ss(&position);
        let (len_m, len_s) = Ui::to_mm_ss(&length);

        let left_pad = std::cmp::max((pos_per * 50.0).ceil() as i32 - 1, 0) as u32;

        print!(
            "{} {:02}:{:02} ({:3.0} %) [",
            status_char,
            cur_m,
            cur_s,
            pos_per * 100.0
        );
        for _ in 0..left_pad {
            print!("=");
        }
        print!(">");
        for _ in 0..50 - left_pad - 1 {
            print!(" ");
        }
        print!("] {:02}:{:02}\r", len_m, len_s);

        std::io::stdout().flush().unwrap();
    }
}

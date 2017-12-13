extern crate mpris;
use mpris::{PlayerFinder, Metadata, PlaybackStatus, Progress};
use std::io::{stdout, Write};
use std::time::Duration;

fn reset_line() {
    print!("\r\x1b[K");
}

fn print_duration(duration: Duration) {
    let secs = duration.as_secs();
    let whole_hours = secs / (60 * 60);

    let secs = secs - whole_hours * 60 * 60;
    let whole_minutes = secs / 60;

    let secs = secs - whole_minutes * 60;

    print!("{:02}:{:02}:{:02}", whole_hours, whole_minutes, secs)
}

fn print_time(duration: Option<Duration>) {
    match duration {
        Some(duration) => print_duration(duration),
        None => print!("??:??:??"),
    }
}

fn print_artist(metadata: &Metadata) {
    if let Some(ref artists) = metadata.artists {
        if artists.len() > 0 {
            print!("{}", artists.join(" + "));
            return;
        }
    }

    print!("Unknown artist");
}

fn print_title(metadata: &Metadata) {
    if let Some(ref title) = metadata.title {
        print!("{}", title);
        return;
    }

    print!("Unknown title");
}

fn print_playback_status(progress: &Progress) {
    match progress.playback_status {
        PlaybackStatus::Playing => print!("▶"),
        PlaybackStatus::Paused => print!("▮▮"),
        PlaybackStatus::Stopped => print!("◼"),
    }
}

fn main() {
    let player = PlayerFinder::new().unwrap().find_active().unwrap();
    let identity = player.identity();

    let mut progress_tracker = player.track_progress(100).unwrap();
    loop {
        let progress = progress_tracker.tick();

        reset_line();
        print_playback_status(&progress);
        print!("\t");
        print_artist(&progress.metadata);
        print!(" - ");
        print_title(&progress.metadata);
        print!(" [");
        if progress.supports_position() {
            print_time(Some(progress.position()));
        } else {
            print_time(None);
        }
        print!(" / ");
        print_time(progress.length());
        print!("] ({})", identity);
        stdout().flush().unwrap();
    }
}

use std::io::{stdout, Write};
use std::time::Duration;

use mpris::{LoopStatus, Metadata, PlaybackStatus, PlayerFinder, Progress, ProgressTick};

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
    if let Some(artists) = metadata.artists() {
        if !artists.is_empty() {
            print!("{}", artists.join(" + "));
            return;
        }
    }

    print!("Unknown artist");
}

fn print_title(metadata: &Metadata) {
    print!("{}", metadata.title().unwrap_or("Unknown title"));
}

fn print_playback_status(progress: &Progress) {
    match progress.playback_status() {
        PlaybackStatus::Playing => print!("▶"),
        PlaybackStatus::Paused => print!("▮▮"),
        PlaybackStatus::Stopped => print!("◼"),
    }
}

fn print_shuffle_status(progress: &Progress) {
    if progress.shuffle() {
        print!("🔀");
    } else {
        print!(" ");
    }
}

fn print_loop_status(progress: &Progress) {
    match progress.loop_status() {
        LoopStatus::None => print!(" "),
        LoopStatus::Track => print!("🔂"),
        LoopStatus::Playlist => print!("🔁"),
    }
}

fn main() {
    let player = PlayerFinder::new().unwrap().find_active().unwrap();
    let identity = player.identity();

    let mut progress_tracker = player.track_progress(100).unwrap();
    loop {
        let ProgressTick { progress, .. } = progress_tracker.tick();

        reset_line();
        print_playback_status(progress);
        print_shuffle_status(progress);
        print_loop_status(progress);
        print!("\t");
        print_artist(progress.metadata());
        print!(" - ");
        print_title(progress.metadata());
        print!(" [");
        if identity != "Spotify" {
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

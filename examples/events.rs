use mpris::PlayerFinder;
use std::time::{Duration, Instant};

fn main() {
    let player = PlayerFinder::new()
        .expect("Could not connect to D-Bus")
        .find_active()
        .expect("Could not find active player");

    println!(
        "Showing event stream for player {}...\n(Exit with Ctrl-C)\n",
        player.identity()
    );

    let events = player.events().expect("Could not start event stream");
    let start = Instant::now();

    for event in events {
        match event {
            Ok(event) => println!("{}: {:#?}", format_elapsed(start.elapsed()), event),
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }

    println!("Event stream ended.");
}

fn format_elapsed(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let minutes = seconds / 60;
    let seconds_left = seconds - (60 * minutes);
    let ms = duration.subsec_millis();
    format!("{:02}:{:02}.{:3}", minutes, seconds_left, ms)
}

extern crate mpris;

use mpris::{Event, PlayerFinder};

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
    for event in events {
        match event {
            Ok(event) => println!("{:#?}", event),
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }

    println!("Event stream ended.");
}

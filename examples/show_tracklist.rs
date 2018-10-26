extern crate failure;
extern crate mpris;

use failure::{Error, ResultExt};
use mpris::PlayerFinder;

fn main() {
    match print_track_list() {
        Ok(_) => {}
        Err(error) => {
            println!("Error: {}", error);
            for (i, cause) in error.iter_causes().enumerate() {
                print!("{}", "  ".repeat(i + 1));
                println!("Caused by: {}", cause);
            }
            std::process::exit(1);
        }
    }
}

fn print_track_list() -> Result<(), Error> {
    let player_finder = PlayerFinder::new().context("Could not connect to D-Bus")?;

    let player = player_finder
        .find_active()
        .context("Could not find any player")?;

    println!(
        "Found {identity} (on bus {bus_name})",
        bus_name = player.bus_name(),
        identity = player.identity(),
    );

    let track_list = player
        .get_track_list()
        .context("Could not get track list for player")?;

    println!("Track list:\n");
    let iter = track_list
        .metadata_iter(&player)
        .context("Could not load metadata for tracks")?;

    for metadata in iter {
        println!("{:#?}", metadata);
    }

    Ok(())
}

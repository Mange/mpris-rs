use anyhow::{Context, Result};
use mpris::PlayerFinder;

fn main() {
    match print_track_list() {
        Ok(_) => {}
        Err(error) => {
            println!("Error: {}", error);
            for (i, cause) in error.chain().skip(1).enumerate() {
                print!("{}", "  ".repeat(i + 1));
                println!("Caused by: {}", cause);
            }
            std::process::exit(1);
        }
    }
}

fn print_track_list() -> Result<()> {
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
        .checked_get_track_list()
        .context("Could not get track list for player")?;

    let track_list = match track_list {
        Some(tracks) => tracks,
        None => {
            println!("Player does not support the TrackList interface.");
            return Ok(());
        }
    };

    println!("Track list:\n");
    let iter = track_list
        .metadata_iter(&player)
        .context("Could not load metadata for tracks")?;

    for metadata in iter {
        println!("{:#?}", metadata);
    }

    Ok(())
}

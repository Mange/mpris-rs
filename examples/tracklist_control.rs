extern crate failure;
extern crate mpris;

use failure::{format_err, Error, ResultExt};
use mpris::{Player, PlayerFinder};

fn main() {
    match run() {
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

fn run() -> Result<(), Error> {
    use std::io::stdin;

    let player_finder = PlayerFinder::new().context("Could not connect to D-Bus")?;

    let player = player_finder
        .find_active()
        .context("Could not find any player")?;

    println!(
        "Found {identity} (on bus {bus_name})",
        bus_name = player.bus_name(),
        identity = player.identity(),
    );

    if !player.supports_track_lists() {
        println!("Player does not support TrackList");
        return Ok(());
    }

    let mut answer = String::new();

    loop {
        println!("What to do? [q]uit, [g]oto, [l]ist >");
        answer.clear();
        stdin().read_line(&mut answer)?;
        match answer.trim() {
            "q" | "Q" => break,
            "l" | "L" => print_track_list(&player)?,
            "g" | "G" => goto_track(&player)?,
            _ => println!("I don't understand \"{}\"", answer),
        }
    }

    Ok(())
}

fn print_track_list(player: &Player) -> Result<(), Error> {
    let track_list = player
        .get_track_list()
        .context("Could not get track list for player")?;

    println!("Track list:\n");
    let iter = track_list
        .metadata_iter(&player)
        .context("Could not load metadata for tracks")?;

    for (index, metadata) in iter.enumerate() {
        let title = metadata.title().unwrap_or("Unknown title");
        let artist = metadata
            .artists()
            .map(|list| list.join(", "))
            .unwrap_or_else(|| "Unknown artist".into());

        println!("{}. {} - {}", index + 1, artist, title);
    }

    Ok(())
}

fn goto_track(player: &Player) -> Result<(), Error> {
    use std::io::stdin;

    let track_list = player
        .get_track_list()
        .context("Could not get track list for player")?;
    let len = track_list.len();
    println!("Select track index [1-{}, q] > ", len);

    let mut answer = String::new();
    stdin().read_line(&mut answer)?;
    let answer = answer.trim();

    if answer != "q" {
        let number: usize = answer.parse::<usize>().context("Not a valid number")?;
        let track_id = track_list
            .get(number.saturating_sub(1))
            .ok_or_else(|| format_err!("Not a valid position"))?;
        player.go_to(track_id).context("Failed to change track")?;
    }

    Ok(())
}

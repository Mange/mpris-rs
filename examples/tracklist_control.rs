use anyhow::{anyhow, Context, Error, Result};
use mpris::{Player, PlayerFinder, TrackID};

fn main() {
    match run() {
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

fn prompt_string(message: &str) -> Result<String> {
    use std::io::stdin;
    let mut answer = String::new();

    println!("{}", message);
    stdin().read_line(&mut answer)?;

    Ok(String::from(answer.trim()))
}

fn run() -> Result<()> {
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

    loop {
        let answer = prompt_string("What to do? [q]uit, [g]oto, [l]ist, [a]dd, [r]emove >")?;
        match answer.as_str() {
            "q" | "Q" => break,
            "l" | "L" => print_track_list(&player).context("Failed to list tracks")?,
            "g" | "G" => goto_track(&player).context("Failed to change track")?,
            "a" | "A" => add_track(&player).context("Failed to add track")?,
            "r" | "R" => remove_track(&player).context("Failed to remove track")?,
            _ => println!("I don't understand \"{}\"", answer),
        }
    }

    Ok(())
}

fn print_track_list(player: &Player) -> Result<()> {
    let track_list = player.get_track_list()?;

    println!("Track list:\n");
    let iter = track_list
        .metadata_iter(player)
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

fn select_track(player: &Player, lower_bound: usize) -> Result<Option<TrackID>> {
    let track_list = player
        .get_track_list()
        .context("Could not get track list for player")?;
    let len = track_list.len();
    let answer = prompt_string(&format!(
        "Select track index [{}-{}, q] > ",
        lower_bound, len
    ))?;

    if answer.is_empty() || answer == "q" {
        return Ok(None);
    }

    let number: usize = answer.parse::<usize>().context("Not a valid number")?;
    if number == 0 {
        return Ok(None);
    }

    let track_id = track_list
        .get(number - 1)
        .ok_or_else(|| anyhow!("Not a valid position"))?;

    Ok(Some(track_id.clone()))
}

fn goto_track(player: &Player) -> Result<()> {
    match select_track(player, 1) {
        Ok(Some(track_id)) => player.go_to(&track_id).map_err(Error::from),
        Ok(None) => Ok(()),
        Err(err) => Err(err.context("Failed to select track")),
    }
}

fn remove_track(player: &Player) -> Result<()> {
    match select_track(player, 1) {
        Ok(Some(track_id)) => player.remove_track(&track_id).map_err(Error::from),
        Ok(None) => Ok(()),
        Err(err) => Err(err.context("Failed to select track")),
    }
}

fn add_track(player: &Player) -> Result<()> {
    println!("NOTE: To add local media, start with the \"file://\" protocol. E.x. \"file:///path/to/file.mp3\"");
    let uri = prompt_string("Enter URI (or nothing to cancel) > ")?;
    if uri.is_empty() {
        return Ok(());
    }

    println!(
        "Will be inserted after selected track. Select no track (0) to insert at the beginning."
    );
    match select_track(player, 0) {
        Ok(Some(track_id)) => player
            .add_track(&uri, &track_id, false)
            .map_err(Error::from),
        Ok(None) => player.add_track_at_start(&uri, false).map_err(Error::from),
        Err(err) => Err(err)
            .context("Failed to select track")
            .map_err(Error::from),
    }
}

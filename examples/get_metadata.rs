extern crate mpris;

use mpris::{PlayerFinder, Metadata};

fn main() {
    match retrieve_metadata() {
        Ok(metadata) => {
            println!("Metadata:\n{:#?}", metadata);
        }
        Err(error) => {
            println!("ERROR: {}", error);
            std::process::exit(1);
        }
    }
}

fn retrieve_metadata() -> Result<Metadata, String> {
    let player_finder = PlayerFinder::new().map_err(|e| {
        format!("Could not connect to DBus: {}", e)
    })?;

    let player = player_finder.find_active().map_err(|e| {
        format!("Could not find any player: {}", e)
    })?;

    println!("Found player on bus {}", player.bus_name());

    player.get_metadata().map_err(|e| {
        format!("Could not get metadata for player: {}", e)
    })
}

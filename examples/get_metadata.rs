extern crate failure;
extern crate mpris;

use failure::{Error, ResultExt};
use mpris::PlayerFinder;

fn main() {
    match print_metadata() {
        Ok(_) => {}
        Err(error) => {
            for (i, cause) in error.iter_chain().enumerate() {
                if i == 0 {
                    println!("Error: {}", cause);
                } else {
                    println!("Caused by: {}", cause);
                }
            }
            std::process::exit(1);
        }
    }
}

#[allow(deprecated)]
fn print_metadata() -> Result<(), Error> {
    let player_finder = PlayerFinder::new().context("Could not connect to D-Bus")?;

    let player = player_finder
        .find_active()
        .context("Could not find any player")?;

    println!(
        "Found {identity} (on bus {bus_name})",
        bus_name = player.bus_name(),
        identity = player.identity(),
    );

    let metadata = player
        .get_metadata()
        .context("Could not get metadata for player")?;

    println!("Metadata:\n{:#?}\n", metadata);
    println!(
        "\nRest of the metadata (emulated raw data):\n{:#?}",
        metadata.rest_hash()
    );

    println!(
        "\nRaw metadata value:\n{:#?}",
        player
            .get_metadata_hash()
            .context("Could not fetch raw metadata hash")?
    );

    Ok(())
}

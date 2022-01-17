use anyhow::{Context, Result};
use mpris::PlayerFinder;

fn main() {
    match print_metadata() {
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

fn print_metadata() -> Result<()> {
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

    Ok(())
}

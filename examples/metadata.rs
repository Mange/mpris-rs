use mpris::{Mpris, Player};
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mpris = Mpris::new().await?;
    let mut total = 0;

    for player in mpris.players().await? {
        print_metadata(player).await?;
        total += 1;
    }

    if total == 0 {
        println!("No players found");
    }

    Ok(())
}

async fn print_metadata(player: Player<'_>) -> Result<(), Box<dyn Error>> {
    println!(
        "Player: {} ({})",
        player.identity().await?,
        player.bus_name()
    );
    let metadata = player.metadata().await?;
    println!("Metadata:\n{:#?}", metadata);
    Ok(())
}

use mpris::{Mpris, MprisError, Player};

#[async_std::main]
async fn main() -> Result<(), MprisError> {
    let mpris = Mpris::new().await?;
    let mut total = 0;

    for player in mpris.all_players().await? {
        print_metadata(player).await?;
        total += 1;
    }

    if total == 0 {
        println!("No players found");
    }

    Ok(())
}

async fn print_metadata(player: Player) -> Result<(), MprisError> {
    println!(
        "Player: {} ({})",
        player.identity().await?,
        player.bus_name()
    );

    let metadata = player.metadata().await?;
    println!("Metadata:\n{:#?}", metadata);

    Ok(())
}

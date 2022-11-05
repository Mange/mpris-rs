use mpris::Mpris;
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mpris = Mpris::new().await?;
    for player in mpris.players().await {
        println!("{:?}", player);
    }
    Ok(())
}

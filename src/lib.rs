use zbus::Connection;

mod player;
mod proxies;

use player::Player;

pub struct Mpris {
    connection: Connection,
}

impl Mpris {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }

    pub async fn players(&self) -> Result<Vec<Player>, Box<dyn std::error::Error>> {
        player::all(&self.connection).await
    }
}

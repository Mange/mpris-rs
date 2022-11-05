use zbus::Connection;

use crate::{
    proxies::{DBusProxy, MprisPlayerProxy},
    Mpris,
};

pub(crate) const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";
// pub(crate) const MPRIS2_PATH: &str = "/org/mpris/MediaPlayer2";

pub struct Player<'a> {
    proxy: MprisPlayerProxy<'a>, // Lifetime is for the connection, which we own.
}

impl<'a> Player<'a> {
    pub async fn new(
        mpris: &'a Mpris,
        bus_name: String,
    ) -> Result<Player<'a>, Box<dyn std::error::Error>> {
        Ok(Player {
            proxy: MprisPlayerProxy::builder(&mpris.connection)
                .destination(bus_name)?
                .build()
                .await?,
        })
    }

    pub(crate) async fn new_from_connection(
        connection: Connection,
        bus_name: String,
    ) -> Result<Player<'a>, Box<dyn std::error::Error>> {
        Ok(Player {
            proxy: MprisPlayerProxy::builder(&connection)
                .destination(bus_name)?
                .build()
                .await?,
        })
    }

    pub async fn identity(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.proxy.identity().await?)
    }
}

impl<'a> std::fmt::Debug for Player<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("bus_name", &self.proxy.inner().destination().to_string())
            .finish()
    }
}

pub(crate) async fn all(
    connection: &Connection,
) -> Result<Vec<Player>, Box<dyn std::error::Error>> {
    let connection = connection.clone();
    let proxy = DBusProxy::new(&connection).await?;
    let names = proxy.list_names().await?;

    let mut players = Vec::with_capacity(names.len());
    for name in names.into_iter() {
        if name.starts_with(MPRIS2_PREFIX) {
            players.push(Player::new_from_connection(connection.clone(), name).await?);
        }
    }

    Ok(players)
}

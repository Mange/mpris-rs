use std::collections::HashMap;

use zbus::{names::BusName, zvariant::Value, Connection};

use crate::{
    proxies::{DBusProxy, MediaPlayer2Proxy, PlayerProxy},
    Metadata, Mpris,
};

pub(crate) const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";
// pub(crate) const MPRIS2_PATH: &str = "/org/mpris/MediaPlayer2";

pub struct Player<'conn> {
    mp2_proxy: MediaPlayer2Proxy<'conn>,
    player_proxy: PlayerProxy<'conn>,
}

impl<'conn> Player<'conn> {
    pub async fn new<B>(
        mpris: &'conn Mpris,
        bus_name: BusName<'static>,
    ) -> Result<Player<'conn>, Box<dyn std::error::Error>> {
        Player::new_from_connection(mpris.connection.clone(), bus_name).await
    }

    pub(crate) async fn new_from_connection(
        connection: Connection,
        bus_name: BusName<'static>,
    ) -> Result<Player<'conn>, Box<dyn std::error::Error>> {
        let mp2_proxy = MediaPlayer2Proxy::builder(&connection)
            .destination(bus_name.clone())?
            .build()
            .await?;

        let player_proxy = PlayerProxy::builder(&connection)
            .destination(bus_name.clone())?
            .build()
            .await?;

        Ok(Player {
            mp2_proxy,
            player_proxy,
        })
    }

    pub async fn identity(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.mp2_proxy.identity().await?)
    }

    pub async fn metadata(&self) -> Result<Metadata, Box<dyn std::error::Error>> {
        Ok(self.player_proxy.metadata().await?.into())
    }

    pub fn bus_name(&self) -> &str {
        self.mp2_proxy.bus_name()
    }
}

impl<'a> std::fmt::Debug for Player<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("bus_name", &self.bus_name())
            .finish()
    }
}

pub(crate) async fn all(
    connection: &Connection,
) -> Result<Vec<Player>, Box<dyn std::error::Error>> {
    let connection = connection.clone();
    let proxy = DBusProxy::new(&connection).await?;
    let names = proxy.list_names().await?;

    let mut players = Vec::new();
    for name in names.into_iter() {
        if name.starts_with(MPRIS2_PREFIX) {
            if let Ok(bus_name) = name.try_into() {
                players.push(Player::new_from_connection(connection.clone(), bus_name).await?);
            }
        }
    }

    Ok(players)
}

extern crate dbus;

use dbus::{Connection, Message, BusType, arg};

use prelude::*;
use player::Player;

const LIST_NAMES_TIMEOUT_MS: i32 = 500;

pub struct PlayerFinder {
    connection: Connection,
}

impl PlayerFinder {
    pub fn new() -> Result<Self> {
        Ok(PlayerFinder {
            connection: Connection::get_private(BusType::Session)?,
        })
    }

    pub fn find_all(&self) -> Result<Vec<Player>> {
        self.all_player_buses()?
            .into_iter()
            .map(|bus_name| Player::new(&self.connection, bus_name))
            .collect()
    }

    pub fn find_active(&self) -> Result<Player> {
        if let Some(bus_name) = self.active_player_bus()? {
            Player::new(&self.connection, bus_name)
        } else {
            Err(ErrorKind::NoPlayerFound.into())
        }
    }


    fn active_player_bus(&self) -> Result<Option<String>> {
        // Right now, we just pick the first of the players. Is there some way to select this more
        // intelligently?
        Ok(self.all_player_buses()?.into_iter().nth(0))
    }

    fn all_player_buses(&self) -> Result<Vec<String>> {
        let list_names = Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "ListNames",
        ).unwrap();
        let reply = self.connection.send_with_reply_and_block(
            list_names,
            LIST_NAMES_TIMEOUT_MS,
        )?;

        let names: arg::Array<&str, _> = reply.get1().ok_or_else(|| {
            ErrorKind::DBusCallError(String::from("Could not get ListNames reply"))
        })?;

        Ok(
            names
                .into_iter()
                .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
                .map(|str_ref| str_ref.to_owned())
                .collect(),
        )
    }
}

extern crate dbus;

use dbus::{Connection, Message, BusType, arg};

use prelude::*;
use player::{Player, MPRIS2_PREFIX};

const LIST_NAMES_TIMEOUT_MS: i32 = 500;

/// Used to find `Player`s running on a DBUS connection.
pub struct PlayerFinder {
    connection: Connection,
}

impl PlayerFinder {
    /// Creates a new `PlayerFinder` with a new default DBUS connection.
    ///
    /// Use `for_connection` if you want to provide the DBUS connection yourself.
    pub fn new() -> Result<Self> {
        Ok(PlayerFinder::for_connection(
            Connection::get_private(BusType::Session)?,
        ))
    }

    /// Create a new `PlayerFinder` with the given connection.
    ///
    /// Use `new` if you want a new default connection rather than manually managing the DBUS
    /// connection.
    pub fn for_connection(connection: Connection) -> Self {
        PlayerFinder { connection: connection }
    }

    /// Find all available `Player`s in the connection.
    pub fn find_all(&self) -> Result<Vec<Player>> {
        self.all_player_buses()?
            .into_iter()
            .map(|bus_name| Player::new(&self.connection, bus_name))
            .collect()
    }

    /// Try to find the "active" player in the connection.
    ///
    /// MPRIS does not have the concept of "active" and all players are treated the same, even if
    /// only one of the players are currently playing something.
    ///
    /// This method will try to determine which player a user is most likely to use.
    ///
    /// **NOTE:** Currently this method is very naive and just returns the first player. This
    /// behavior can change later without a major version change, so don't rely on that behavior.
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
                .filter(|name| name.starts_with(MPRIS2_PREFIX))
                .map(|str_ref| str_ref.to_owned())
                .collect(),
        )
    }
}

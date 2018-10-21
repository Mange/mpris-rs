extern crate dbus;

use std::rc::Rc;

use dbus::{arg, BusType, Connection, Message};

use super::DBusError;
use player::{Player, DEFAULT_TIMEOUT_MS, MPRIS2_PATH, MPRIS2_PREFIX};
use pooled_connection::PooledConnection;

const LIST_NAMES_TIMEOUT_MS: i32 = 500;

/// This enum encodes possible error cases that could happen when finding players.
#[derive(Fail, Debug)]
pub enum FindingError {
    /// No player was found matching the requirements of the calling method.
    #[fail(display = "No player found")]
    NoPlayerFound,

    /// Finding failed due to an underlying D-Bus error.
    #[fail(display = "{}", _0)]
    DBusError(#[cause] DBusError),
}

impl From<dbus::Error> for FindingError {
    fn from(error: dbus::Error) -> Self {
        FindingError::DBusError(error.into())
    }
}

impl From<DBusError> for FindingError {
    fn from(error: DBusError) -> Self {
        FindingError::DBusError(error)
    }
}

/// Used to find `Player`s running on a D-Bus connection.
#[derive(Debug)]
pub struct PlayerFinder {
    connection: Rc<PooledConnection>,
}

impl PlayerFinder {
    /// Creates a new `PlayerFinder` with a new default D-Bus connection.
    ///
    /// Use `for_connection` if you want to provide the D-Bus connection yourself.
    pub fn new() -> Result<Self, DBusError> {
        Ok(PlayerFinder::for_connection(Connection::get_private(
            BusType::Session,
        )?))
    }

    /// Create a new `PlayerFinder` with the given connection.
    ///
    /// Use `new` if you want a new default connection rather than manually managing the D-Bus
    /// connection.
    pub fn for_connection(connection: Connection) -> Self {
        PlayerFinder {
            connection: Rc::new(connection.into()),
        }
    }

    /// Find all available `Player`s in the connection.
    pub fn find_all<'a>(&self) -> Result<Vec<Player<'a>>, FindingError> {
        self.all_player_buses()
            .map_err(FindingError::from)?
            .into_iter()
            .map(|bus_name| {
                Player::for_pooled_connection(
                    Rc::clone(&self.connection),
                    bus_name.into(),
                    MPRIS2_PATH.into(),
                    DEFAULT_TIMEOUT_MS,
                ).map_err(FindingError::from)
            }).collect()
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
    pub fn find_active<'a>(&self) -> Result<Player<'a>, FindingError> {
        if let Some(bus_name) = self.active_player_bus()? {
            Player::for_pooled_connection(
                Rc::clone(&self.connection),
                bus_name.into(),
                MPRIS2_PATH.into(),
                DEFAULT_TIMEOUT_MS,
            ).map_err(FindingError::from)
        } else {
            Err(FindingError::NoPlayerFound)
        }
    }

    fn active_player_bus(&self) -> Result<Option<String>, FindingError> {
        // Right now, we just pick the first of the players. Is there some way to select this more
        // intelligently?
        Ok(self.all_player_buses()?.into_iter().nth(0))
    }

    fn all_player_buses(&self) -> Result<Vec<String>, DBusError> {
        let list_names = Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "ListNames",
        ).unwrap();

        let reply = self
            .connection
            .underlying()
            .send_with_reply_and_block(list_names, LIST_NAMES_TIMEOUT_MS)?;

        let names: arg::Array<&str, _> = reply.read1().map_err(DBusError::from)?;

        Ok(names
            .filter(|name| name.starts_with(MPRIS2_PREFIX))
            .map(|str_ref| str_ref.to_owned())
            .collect())
    }
}

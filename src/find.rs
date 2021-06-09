use failure::Fail;

use std::rc::Rc;

use dbus::ffidisp::{BusType, Connection};
use dbus::{arg, Message};

use super::DBusError;
use crate::player::{Player, DEFAULT_TIMEOUT_MS, MPRIS2_PATH, MPRIS2_PREFIX};
use crate::pooled_connection::PooledConnection;
use crate::PlaybackStatus;

const LIST_NAMES_TIMEOUT_MS: i32 = 500;

/// This enum encodes possible error cases that could happen when finding players.
#[derive(Fail, Debug)]
pub enum FindingError {
    /// No player was found matching the requirements of the calling method.
    #[fail(display = "No player found")]
    NoPlayerFound,

    /// Finding failed due to an underlying [`DBusError`].
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

/// Used to find [`Player`]s running on a D-Bus connection.
///
/// All find results are sorted in alphabetical order.
#[derive(Debug)]
pub struct PlayerFinder {
    connection: Rc<PooledConnection>,
}

impl PlayerFinder {
    /// Creates a new [`PlayerFinder`] with a new default D-Bus connection.
    ///
    /// Use [`for_connection`](Self::for_connection) if you want to provide the D-Bus connection yourself.
    pub fn new() -> Result<Self, DBusError> {
        Ok(PlayerFinder::for_connection(Connection::get_private(
            BusType::Session,
        )?))
    }

    /// Create a new [`PlayerFinder`] with the given connection.
    ///
    /// Use [`new`](Self::new) if you want a new default connection rather than manually managing the D-Bus
    /// connection.
    pub fn for_connection(connection: Connection) -> Self {
        PlayerFinder {
            connection: Rc::new(connection.into()),
        }
    }

    /// Find all available [`Player`]s in the connection.
    pub fn find_all<'b>(&self) -> Result<Vec<Player<'b>>, FindingError> {
        self.all_player_buses()
            .map_err(FindingError::from)?
            .into_iter()
            .map(|bus_name| {
                Player::for_pooled_connection(
                    Rc::clone(&self.connection),
                    bus_name.into(),
                    MPRIS2_PATH.into(),
                    DEFAULT_TIMEOUT_MS,
                )
                .map_err(FindingError::from)
            })
            .collect()
    }

    /// Return the first found [`Player`] regardless of state.
    pub fn find_first<'b>(&self) -> Result<Player<'b>, FindingError> {
        let busses = self.all_player_buses()?;
        if let Some(bus_name) = busses.into_iter().next() {
            Player::for_pooled_connection(
                Rc::clone(&self.connection),
                bus_name.into(),
                MPRIS2_PATH.into(),
                DEFAULT_TIMEOUT_MS,
            )
            .map_err(FindingError::from)
        } else {
            Err(FindingError::NoPlayerFound)
        }
    }

    /// Try to find the "active" [`Player`] in the connection.
    ///
    /// This method will try to determine which player a user is most likely to use. First it will look for a player with
    /// the playback status [`Playing`](PlaybackStatus::Playing), then for a [`Paused`](PlaybackStatus::Paused), then one with
    /// track metadata, after that it will just return the first it finds. [`NoPlayerFound`](FindingError::NoPlayerFound) is returned
    /// only if there is no player on the DBus.
    pub fn find_active<'b>(&self) -> Result<Player<'b>, FindingError> {
        let mut players: Vec<Player> = self.find_all()?;

        // Return Error if no players
        if players.is_empty() {
            return Err(FindingError::NoPlayerFound);
        }

        // Look for "Playing"
        for (n, player) in players.iter().enumerate() {
            if let PlaybackStatus::Playing = player.get_playback_status()? {
                return Ok(players.remove(n));
            }
        }

        // Look for "Paused"
        for (n, player) in players.iter().enumerate() {
            if let PlaybackStatus::Paused = player.get_playback_status()? {
                return Ok(players.remove(n));
            }
        }

        // Look for player with metadata
        for (n, player) in players.iter().enumerate() {
            if !player.get_metadata()?.as_hashmap().is_empty() {
                return Ok(players.remove(n));
            }
        }

        // Finally just return any player
        Ok(players.remove(0))
    }

    /// Find a [`Player`] by it's MPRIS [`Identity`][identity]. Returns [`NoPlayerFound`](FindingError::NoPlayerFound) if no direct match found.
    ///
    /// [identity]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Identity
    pub fn find_by_name<'b>(&self, name: &str) -> Result<Player<'b>, FindingError> {
        let players = self.find_all()?;
        for player in players {
            if player.identity() == name {
                return Ok(player);
            }
        }
        Err(FindingError::NoPlayerFound)
    }

    fn all_player_buses(&self) -> Result<Vec<String>, DBusError> {
        let list_names = Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "ListNames",
        )
        .unwrap();

        let reply = self
            .connection
            .underlying()
            .send_with_reply_and_block(list_names, LIST_NAMES_TIMEOUT_MS)?;

        let names: arg::Array<'_, &str, _> = reply.read1().map_err(DBusError::from)?;

        let mut all_busses = names
            .filter(|name| name.starts_with(MPRIS2_PREFIX))
            .map(|str_ref| str_ref.to_owned())
            .collect::<Vec<String>>();
        all_busses.sort_by_key(|a| a.to_lowercase());
        Ok(all_busses)
    }
}

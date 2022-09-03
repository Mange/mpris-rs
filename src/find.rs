use thiserror::Error;

use std::rc::Rc;

use dbus::ffidisp::{BusType, Connection};
use dbus::{arg, Message};

use super::DBusError;
use crate::player::{Player, DEFAULT_TIMEOUT_MS, MPRIS2_PREFIX};
use crate::pooled_connection::PooledConnection;
use crate::PlaybackStatus;

const LIST_NAMES_TIMEOUT_MS: i32 = 500;

/// This enum encodes possible error cases that could happen when finding players.
#[derive(Debug, Error)]
pub enum FindingError {
    /// No player was found matching the requirements of the calling method.
    #[error("No player found")]
    NoPlayerFound,

    /// Finding failed due to an underlying [`DBusError`].
    #[error("{}", 0)]
    DBusError(#[from] DBusError),
}

impl From<dbus::Error> for FindingError {
    fn from(error: dbus::Error) -> Self {
        FindingError::DBusError(error.into())
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
    pub fn find_all(&self) -> Result<Vec<Player>, FindingError> {
        self.all_player_buses()
            .map_err(FindingError::from)?
            .into_iter()
            .map(|bus_name| {
                Player::for_pooled_connection(
                    Rc::clone(&self.connection),
                    bus_name,
                    DEFAULT_TIMEOUT_MS,
                )
                .map_err(FindingError::from)
            })
            .collect()
    }

    /// Return the first found [`Player`] regardless of state.
    pub fn find_first(&self) -> Result<Player, FindingError> {
        let busses = self.all_player_buses()?;
        if let Some(bus_name) = busses.into_iter().next() {
            Player::for_pooled_connection(Rc::clone(&self.connection), bus_name, DEFAULT_TIMEOUT_MS)
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
    pub fn find_active(&self) -> Result<Player, FindingError> {
        let mut players: Vec<Player> = self.find_all()?;

        match self.find_active_player_index(&players)? {
            Some(index) => Ok(players.remove(index)),
            None => Err(FindingError::NoPlayerFound),
        }
    }

    /// Finds the index of an "active" player. Follows the order mentioned in [`find_active`](Self::find_active).
    fn find_active_player_index(&self, players: &[Player]) -> Result<Option<usize>, DBusError> {
        if players.is_empty() {
            return Ok(None);
        } else if players.len() == 1 {
            return Ok(Some(0));
        }

        let mut first_paused: Option<usize> = None;
        let mut first_with_track: Option<usize> = None;

        for (index, player) in players.iter().enumerate() {
            let player_status = player.get_playback_status()?;

            if player_status == PlaybackStatus::Playing {
                return Ok(Some(index));
            }

            if first_paused.is_none() && player_status == PlaybackStatus::Paused {
                first_paused.replace(index);
            }

            if first_with_track.is_none() && !player.get_metadata()?.is_empty() {
                first_with_track.replace(index);
            }
        }

        Ok(first_paused.or(first_with_track).or(Some(0)))
    }

    /// Find a [`Player`] by it's MPRIS [`Identity`][identity]. Returns [`NoPlayerFound`](FindingError::NoPlayerFound) if no direct match found.
    ///
    /// [identity]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Identity
    pub fn find_by_name(&self, name: &str) -> Result<Player, FindingError> {
        let players = self.find_all()?;
        for player in players {
            if player.identity() == name {
                return Ok(player);
            }
        }
        Err(FindingError::NoPlayerFound)
    }

    /// Returns all of the MPRIS DBus paths
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

    /// Returns a [`PlayerIter`] iterator, or an [`DBusError`] if there was a problem with the D-Bus
    ///
    /// For more details see [`PlayerIter`] documentation
    pub fn iter_players(&self) -> Result<PlayerIter, DBusError> {
        let buses = self.all_player_buses()?;
        Ok(PlayerIter::new(buses, self.connection.clone()))
    }
}

/// An iterator that lazily iterates over all of the found [`Player`]s. Useful for efficiently searching for a specific player.
///
/// Created by calling [`PlayerFinder::iter_players`]
///
/// Note that this iterator will not keep checking what players are connected after it's been created. A player might quit or
/// a new one might connect at a later time, this will result in an error or the player not being present respectively.
/// If you want to make sure the data is "fresh" you'll either have to make a new PlayerIter whenever you want to get new data or
/// use [`PlayerFinder::find_all`] which will immediately return a [`Vec`] with all the [`Player`]s that were connected at that point.
#[derive(Debug)]
pub struct PlayerIter {
    buses: std::vec::IntoIter<String>,
    connection: Rc<PooledConnection>,
}

impl PlayerIter {
    fn new(buses: Vec<String>, connection: Rc<PooledConnection>) -> Self {
        Self {
            buses: buses.into_iter(),
            connection,
        }
    }
}

impl Iterator for PlayerIter {
    type Item = Result<Player, DBusError>;

    fn next(&mut self) -> Option<Self::Item> {
        let bus = self.buses.next()?;
        Some(Player::for_pooled_connection(
            self.connection.clone(),
            bus,
            DEFAULT_TIMEOUT_MS,
        ))
    }
}

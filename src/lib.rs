// #![warn(missing_docs)]
#![warn(clippy::print_stdout)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unreachable_pub,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::stream::{FusedStream, Stream, TryStreamExt};
use zbus::{
    names::{BusName, WellKnownName},
    Connection,
};

mod duration;
pub mod errors;
mod metadata;
mod player;
mod playlist;
mod proxies;

use errors::*;

use crate::proxies::DBusProxy;
pub use duration::MprisDuration;
pub use errors::MprisError;
pub use metadata::{Metadata, MetadataIter, MetadataValue, TrackID};
pub use player::Player;
pub use playlist::{Playlist, PlaylistOrdering};

pub(crate) const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";

type PlayerFuture = Pin<Box<dyn Future<Output = Result<Player, MprisError>> + Send>>;

#[derive(Clone)]
pub struct Mpris {
    connection: Connection,
    dbus_proxy: DBusProxy<'static>,
}

impl Mpris {
    pub async fn new() -> Result<Self, MprisError> {
        let connection = Connection::session().await?;
        let dbus_proxy = DBusProxy::new(&connection).await?;

        Ok(Self {
            connection,
            dbus_proxy,
        })
    }

    pub async fn new_from_connection(connection: Connection) -> Result<Self, MprisError> {
        let dbus_proxy = DBusProxy::new(&connection).await?;
        Ok(Self {
            connection,
            dbus_proxy,
        })
    }

    pub fn get_connection(&self) -> Connection {
        self.connection.clone()
    }

    pub fn get_executor(&self) -> &'static zbus::Executor {
        self.connection.executor()
    }

    pub async fn find_first(&self) -> Result<Option<Player>, MprisError> {
        match self.all_player_bus_names().await?.into_iter().next() {
            Some(bus) => Ok(Some(
                Player::new_from_connection(self.connection.clone(), bus).await?,
            )),
            None => Ok(None),
        }
    }

    pub async fn find_active(&self) -> Result<Option<Player>, MprisError> {
        let mut players = self.into_stream().await?;
        if players.is_terminated() {
            return Ok(None);
        }

        let mut first_paused: Option<Player> = None;
        let mut first_with_track: Option<Player> = None;
        let mut first_found: Option<Player> = None;

        while let Some(player) = players.try_next().await? {
            let player_status = player.playback_status().await?;

            if player_status == PlaybackStatus::Playing {
                return Ok(Some(player));
            }

            if first_paused.is_none() && player_status == PlaybackStatus::Paused {
                first_paused.replace(player);
            } else if first_with_track.is_none() && !player.raw_metadata().await?.is_empty() {
                first_with_track.replace(player);
            } else if first_found.is_none() {
                first_found.replace(player);
            }
        }

        Ok(first_paused.or(first_with_track).or(first_found))
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Player>, MprisError> {
        let mut players = self.into_stream().await?;
        if players.is_terminated() {
            return Ok(None);
        }
        while let Some(player) = players.try_next().await? {
            if player.identity().await?.to_lowercase() == name.to_lowercase() {
                return Ok(Some(player));
            }
        }
        Ok(None)
    }

    pub async fn all_players(&self) -> Result<Vec<Player>, MprisError> {
        let bus_names = self.all_player_bus_names().await?;
        let mut players = Vec::with_capacity(bus_names.len());
        for player_name in bus_names {
            players.push(Player::new_from_connection(self.connection.clone(), player_name).await?);
        }
        Ok(players)
    }

    async fn all_player_bus_names(&self) -> Result<Vec<BusName<'static>>, MprisError> {
        let mut names: Vec<BusName> = self
            .dbus_proxy
            .list_names()
            .await?
            .into_iter()
            .filter(|name| name.starts_with(MPRIS2_PREFIX))
            // We got the bus name from the D-Bus server so unchecked is fine
            .map(|name| BusName::from(WellKnownName::from_string_unchecked(name)))
            .collect();
        names.sort_unstable_by_key(|n| n.to_lowercase());
        Ok(names)
    }

    pub async fn into_stream(&self) -> Result<PlayerStream, MprisError> {
        let buses = self.all_player_bus_names().await?;
        Ok(PlayerStream::new(self.connection.clone(), buses))
    }
}

impl Debug for Mpris {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mpris")
            .field("connection", &format_args!("Connection {{ .. }}"))
            .finish_non_exhaustive()
    }
}

pub struct PlayerStream {
    connection: Connection,
    buses: VecDeque<BusName<'static>>,
    cur_future: Option<PlayerFuture>,
}

impl PlayerStream {
    pub fn new(connection: Connection, buses: Vec<BusName<'static>>) -> Self {
        let buses = VecDeque::from(buses);
        Self {
            connection,
            buses,
            cur_future: None,
        }
    }
}

impl Stream for PlayerStream {
    type Item = Result<Player, MprisError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.cur_future.as_mut() {
                Some(fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(player) => {
                        self.cur_future = None;
                        self.buses.pop_front();
                        return Poll::Ready(Some(player));
                    }
                    Poll::Pending => return Poll::Pending,
                },
                None => match self.buses.front() {
                    Some(bus) => {
                        self.cur_future = Some(Box::pin(Player::new_from_connection(
                            self.connection.clone(),
                            bus.clone(),
                        )))
                    }
                    None => return Poll::Ready(None),
                },
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let l = self.buses.len();
        (l, Some(l))
    }
}

impl FusedStream for PlayerStream {
    fn is_terminated(&self) -> bool {
        self.buses.is_empty()
    }
}

impl Debug for PlayerStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerStream")
            .field("connection", &format_args!("Connection {{ .. }}"))
            .field("buses", &self.buses)
            .field(
                "cur_future",
                &self.cur_future.as_ref().map(|_| &self.buses[0]),
            )
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
/// The [`Player`]'s playback status
///
/// See: [MPRIS2 specification about `PlaybackStatus`][playback_status]
///
/// [playback_status]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Enum:Playback_Status
pub enum PlaybackStatus {
    /// A track is currently playing.
    Playing,
    /// A track is currently paused.
    Paused,
    /// There is no track currently playing.
    Stopped,
}

impl ::std::str::FromStr for PlaybackStatus {
    type Err = InvalidPlaybackStatus;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "Playing" => Ok(Self::Playing),
            "Paused" => Ok(Self::Paused),
            "Stopped" => Ok(Self::Stopped),
            _ => Err(InvalidPlaybackStatus::from(string)),
        }
    }
}

impl PlaybackStatus {
    pub fn as_str(&self) -> &str {
        match self {
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Stopped => "Stopped",
        }
    }
}

impl Display for PlaybackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
/// A [`Player`]'s looping status.
///
/// See: [MPRIS2 specification about `Loop_Status`][loop_status]
///
/// [loop_status]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Enum:Loop_Status
pub enum LoopStatus {
    /// The playback will stop when there are no more tracks to play
    None,

    /// The current track will start again from the begining once it has finished playing
    Track,

    /// The playback loops through a list of tracks
    Playlist,
}

impl ::std::str::FromStr for LoopStatus {
    type Err = InvalidLoopStatus;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "None" => Ok(LoopStatus::None),
            "Track" => Ok(LoopStatus::Track),
            "Playlist" => Ok(LoopStatus::Playlist),
            _ => Err(InvalidLoopStatus::from(string)),
        }
    }
}

impl LoopStatus {
    pub fn as_str(&self) -> &str {
        match self {
            LoopStatus::None => "None",
            LoopStatus::Track => "Track",
            LoopStatus::Playlist => "Playlist",
        }
    }
}

impl Display for LoopStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod status_enums_tests {
    use super::*;

    #[test]
    fn valid_playback_status() {
        assert_eq!("Playing".parse(), Ok(PlaybackStatus::Playing));
        assert_eq!("Paused".parse(), Ok(PlaybackStatus::Paused));
        assert_eq!("Stopped".parse(), Ok(PlaybackStatus::Stopped));
    }

    #[test]
    fn invalid_playback_status() {
        assert_eq!(
            "".parse::<PlaybackStatus>(),
            Err(InvalidPlaybackStatus::from(""))
        );
        assert_eq!(
            "playing".parse::<PlaybackStatus>(),
            Err(InvalidPlaybackStatus::from("playing"))
        );
        assert_eq!(
            "wrong".parse::<PlaybackStatus>(),
            Err(InvalidPlaybackStatus::from("wrong"))
        );
    }

    #[test]
    fn playback_status_as_str() {
        assert_eq!(PlaybackStatus::Playing.as_str(), "Playing");
        assert_eq!(PlaybackStatus::Paused.as_str(), "Paused");
        assert_eq!(PlaybackStatus::Stopped.as_str(), "Stopped");
    }

    #[test]
    fn valid_loop_status() {
        assert_eq!("None".parse(), Ok(LoopStatus::None));
        assert_eq!("Track".parse(), Ok(LoopStatus::Track));
        assert_eq!("Playlist".parse(), Ok(LoopStatus::Playlist));
    }

    #[test]
    fn invalid_loop_status() {
        assert_eq!("".parse::<LoopStatus>(), Err(InvalidLoopStatus::from("")));
        assert_eq!(
            "track".parse::<LoopStatus>(),
            Err(InvalidLoopStatus::from("track"))
        );
        assert_eq!(
            "wrong".parse::<LoopStatus>(),
            Err(InvalidLoopStatus::from("wrong"))
        );
    }

    #[test]
    fn loop_status_as_str() {
        assert_eq!(LoopStatus::None.as_str(), "None");
        assert_eq!(LoopStatus::Track.as_str(), "Track");
        assert_eq!(LoopStatus::Playlist.as_str(), "Playlist");
    }
}

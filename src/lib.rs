#![warn(clippy::print_stdout, missing_docs, clippy::todo)]
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

//! # mpris
//!
//! `mpris` is a library for dealing with [MPRIS2][spec]-compatible media players over D-Bus.
//!
//! This would mostly apply to the Linux-ecosystem which is a heavy user of D-Bus.
//!
//! ## Getting started
//!
//! Some hints on how to use this library:
//!
//! 1. Look at the examples under `examples/` in the repo
//! 2. Look at the [`Mpris`] struct
//! 3. Get the first player and make it start playing:
//! ```no_run
//! use mpris::Mpris;
//!
//! #[async_std::main]
//! async fn main() {
//!     let mpris = Mpris::new().await.expect("couldn't connect to D-Bus");
//!     let player = match mpris.find_first().await {
//!         Ok(result) => match result {
//!             Some(player) => player,
//!             None => {
//!                 println!("No player found");
//!                 return;
//!             }
//!         },
//!         Err(err) => {
//!             println!("Error occured: {:?}", err);
//!             return;
//!         }
//!     };
//!     match player.play().await {
//!         Ok(_) => println!("Made the player play"),
//!         Err(err) => println!("Error occured: {:?}", err),
//!     }
//! }
//! ```
//!
//! ## Runtime compatibility
//!
//! The examples will be using [`async_std`][std] but this library does not require a specific
//! runtime. When used with a runtime that isn't [`Tokio`][tokio] it will spawn a thread for the
//! background tasks. If you want to prevent that you should "tick" the internal executor with your
//! runtime like this:
//!
//! ```no_run
//! use mpris::Mpris;
//! use zbus::connection::Builder;
//!
//! #[async_std::main]
//! async fn main() {
//!     let conn = Builder::session()
//!         .unwrap()
//!         .internal_executor(false) // The important part
//!         .build()
//!         .await
//!         .unwrap();
//!     let c = conn.clone();
//!     async_std::task::spawn(async move {
//!         loop {
//!             c.executor().tick().await;
//!         }
//!     });
//!     let mpris = Mpris::new_from_connection(conn).await.unwrap();
//!
//!     // The rest of your code here
//! }
//! ```
//!
//! [spec]: https://specifications.freedesktop.org/mpris-spec/latest/
//! [std]: https://docs.rs/async-std/latest/async_std/
//! [tokio]: https://docs.rs/tokio/latest/tokio/

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
pub mod metadata;
mod player;
mod playlist;
mod proxies;

use errors::*;

use crate::proxies::DBusProxy;
pub use duration::MprisDuration;
#[doc(inline)]
pub use errors::MprisError;
#[doc(inline)]
pub use metadata::{Metadata, MetadataValue, TrackID};
pub use player::Player;
pub use playlist::{Playlist, PlaylistOrdering};

pub(crate) const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";

type PlayerFuture = Pin<Box<dyn Future<Output = Result<Player, MprisError>> + Send>>;

/// The main struct of the library. Used to find [`Player`]s on a D-Bus connection.
///
/// All find methods first sort alphabetically by the players' [well known Bus Name][busname] and if
/// any of the [`Player`]s fails to initialize they will immediately return with an [`Err`]. If you
/// want to get all of the [`Player`]s even if one fails to initialize you should use
/// [`stream_players()`][Self::stream_players] and handle the errors.
///
/// # Find methods return types
/// - <code>[Ok]\([Some]\([Player]\)\)</code>: No error happened and a [`Player`] was found
/// - <code>[Ok]\([None]\)</code>: No error happened but no [`Player`] was found
/// - <code>[Err]\([MprisError]\)</code>: Error occurred while searching, most likely while
///   communicating with the D-Bus server
///
/// [busname]: https://dbus.freedesktop.org/doc/dbus-tutorial.html#bus-names
#[derive(Clone)]
pub struct Mpris {
    connection: Connection,
    pub(crate) dbus_proxy: DBusProxy<'static>,
}

impl Mpris {
    /// Creates a new [`Mpris`] struct by connecting to the session D-Bus server.
    ///
    /// Use [`new_from_connection`](Self::new_from_connection) if you want to provide the D-Bus
    /// connection yourself.
    pub async fn new() -> Result<Self, MprisError> {
        let connection = Connection::session().await?;
        let dbus_proxy = DBusProxy::new(&connection).await?;

        Ok(Self {
            connection,
            dbus_proxy,
        })
    }

    /// Creates a new [`Mpris`] struct with the given connection.
    ///
    /// See [here](crate#runtime-compatibility) for why you would want to use a custom
    /// [`Connection`].
    ///
    /// Use [`new`](Self::new) if you don't have a need to provide the D-Bus connection yourself.
    pub async fn new_from_connection(connection: Connection) -> Result<Self, MprisError> {
        let dbus_proxy = DBusProxy::new(&connection).await?;
        Ok(Self {
            connection,
            dbus_proxy,
        })
    }

    /// Gets the [`Connection`] that is used.
    pub fn get_connection(&self) -> Connection {
        self.connection.clone()
    }

    /// Gets a reference to the [`Connection`] that is used.
    pub fn get_connection_ref(&self) -> &Connection {
        &self.connection
    }

    // Will be used later
    #[allow(dead_code)]
    /// Gets the internal executor for the [`Connection`]. Can be used to spawn tasks.
    pub(crate) fn get_executor(&self) -> &'static zbus::Executor {
        self.connection.executor()
    }

    /// Returns the first found [`Player`] regardless of state.
    pub async fn find_first(&self) -> Result<Option<Player>, MprisError> {
        match self.all_player_bus_names().await?.into_iter().next() {
            Some(bus) => Ok(Some(Player::new(self, bus).await?)),
            None => Ok(None),
        }
    }

    /// Tries to find the "active" [`Player`] in the connection.
    ///
    /// This method will try to determine which player a user is most likely to use. First it will
    /// look for a player with the playback status [`Playing`](PlaybackStatus::Playing), then for a
    /// [`Paused`](PlaybackStatus::Paused), then one with any track metadata, after that it will
    /// just return the first it finds.
    pub async fn find_active(&self) -> Result<Option<Player>, MprisError> {
        let mut players = self.stream_players().await?;
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

    /// Looks for a [`Player`] by it's MPRIS [`Identity`][identity].
    ///
    /// See also [`Player::identity()`].
    ///
    /// [identity]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Identity
    pub async fn find_by_name(
        &self,
        name: &str,
        case_sensitive: bool,
    ) -> Result<Option<Player>, MprisError> {
        let mut players = self.stream_players().await?;
        if players.is_terminated() {
            return Ok(None);
        }
        while let Some(player) = players.try_next().await? {
            let identity = player.identity().await?;
            if case_sensitive {
                if identity == name {
                    return Ok(Some(player));
                }
            } else if identity.to_lowercase() == name.to_lowercase() {
                return Ok(Some(player));
            }
        }
        Ok(None)
    }

    /// Finds all available [`Player`]s in the connection.
    ///
    /// Will return an empty [`Vec`] if there are no players.
    pub async fn all_players(&self) -> Result<Vec<Player>, MprisError> {
        let bus_names = self.all_player_bus_names().await?;
        let mut players = Vec::with_capacity(bus_names.len());
        for player_name in bus_names {
            players.push(Player::new(self, player_name).await?);
        }
        Ok(players)
    }

    /// Gets all of the BusNames that start with the [`MPRIS2_PREFIX`]
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

    /// Creates a [`PlayerStream`] which implements the [`Stream`] trait.
    ///
    /// For more details see [`PlayerStream`]'s documentation.
    pub async fn stream_players(&self) -> Result<PlayerStream, MprisError> {
        let buses = self.all_player_bus_names().await?;
        Ok(PlayerStream::new(self, buses))
    }
}

impl Debug for Mpris {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mpris")
            .field("connection", &format_args!("Connection {{ .. }}"))
            .finish_non_exhaustive()
    }
}

/// Lazily returns the [`Player`]s on the connection.
///
/// Implements the [`Stream`] trait which is the async version of [`Iterator`]. It is recommended to
/// use the [`futures_util`] or [`futures_lite`][lite] crate which provide useful traits for streams.
///
/// Note that [`PlayerStream`] will only yield the [`Player`]s that were present when it was created.
///
/// ```no_run
/// use futures_util::StreamExt;
/// use mpris::Mpris;
///
/// #[async_std::main]
/// async fn main() {
///     let mpris = Mpris::new().await.unwrap();
///     let mut stream = mpris.stream_players().await.unwrap();
///
///     while let Some(result) = stream.next().await {
///         match result {
///             Ok(player) => {}, // Do something with Player
///             Err(err) => {}, // Deal with the error
///         }
///     }
/// }
/// ```
///
/// [lite]: https://docs.rs/futures-lite/latest/futures_lite/
pub struct PlayerStream {
    connection: Connection,
    dbus_proxy: DBusProxy<'static>,
    buses: VecDeque<BusName<'static>>,
    cur_future: Option<PlayerFuture>,
}

impl PlayerStream {
    /// Creates a new [`PlayerStream`].
    ///
    /// There should be no need to use this directly and instead you should use
    /// [`Mpris::stream_players`].
    pub fn new(mpris: &Mpris, buses: Vec<BusName<'static>>) -> Self {
        let buses = VecDeque::from(buses);
        Self {
            connection: mpris.get_connection(),
            dbus_proxy: mpris.dbus_proxy.clone(),
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
                        self.cur_future = Some(Box::pin(Player::new_internal(
                            self.connection.clone(),
                            self.dbus_proxy.clone(),
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

/// The [`Player`]'s playback status.
///
/// See: [MPRIS2 specification about `PlaybackStatus`][playback_status]
///
/// [playback_status]:
/// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Enum:Playback_Status
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
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
    /// Returns it's value as a <code>&[str]</code>
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

/// A [`Player`]'s looping status.
///
/// See: [MPRIS2 specification about `Loop_Status`][loop_status]
///
/// [loop_status]:
/// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Enum:Loop_Status
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum LoopStatus {
    /// The playback will stop when there are no more tracks to play.
    None,

    /// The current track will start again from the beginning once it has finished playing.
    Track,

    /// The playback loops through a list of tracks.
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
    /// Returns it's value as a <code>&[str]</code>
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

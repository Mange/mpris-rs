#![warn(missing_docs)]
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

//!
//! # mpris
//!
//! `mpris` is an idiomatic library for dealing with [MPRIS2][spec]-compatible media players over D-Bus.
//!
//! This would mostly apply to the Linux-ecosystem which is a heavy user of D-Bus.
//!
//! ## Getting started
//!
//! Some hints on how to use this library:
//!
//! 1. Look at the examples under `examples/`.
//! 2. Look at the [`PlayerFinder`] struct.
//!
//! [spec]: https://specifications.freedesktop.org/mpris-spec/latest/

use thiserror::Error;

mod extensions;

#[allow(unreachable_pub)]
mod generated;

mod event;
mod find;
mod metadata;
mod player;
mod pooled_connection;
mod progress;
mod track_list;

pub use crate::event::{Event, EventError, PlayerEvents};
pub use crate::find::{FindingError, PlayerFinder, PlayerIter};
pub use crate::metadata::Metadata;
pub use crate::metadata::Value as MetadataValue;
pub use crate::metadata::ValueKind as MetadataValueKind;
pub use crate::player::Player;
pub use crate::progress::{Progress, ProgressError, ProgressTick, ProgressTracker};
pub use crate::track_list::{TrackID, TrackList, TrackListError};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(missing_docs)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

/// [`PlaybackStatus`] had an invalid string value.
#[derive(Debug, Error)]
#[error("PlaybackStatus must be one of Playing, Paused, Stopped, but was {0}")]
pub struct InvalidPlaybackStatus(String);

impl ::std::str::FromStr for PlaybackStatus {
    type Err = InvalidPlaybackStatus;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        use crate::PlaybackStatus::*;

        match string {
            "Playing" => Ok(Playing),
            "Paused" => Ok(Paused),
            "Stopped" => Ok(Stopped),
            other => Err(InvalidPlaybackStatus(other.to_string())),
        }
    }
}

/// [`LoopStatus`] had an invalid string value.
#[derive(Debug, Error)]
#[error("LoopStatus must be one of None, Track, Playlist, but was {0}")]
pub struct InvalidLoopStatus(String);

impl ::std::str::FromStr for LoopStatus {
    type Err = InvalidLoopStatus;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "None" => Ok(LoopStatus::None),
            "Track" => Ok(LoopStatus::Track),
            "Playlist" => Ok(LoopStatus::Playlist),
            other => Err(InvalidLoopStatus(other.to_string())),
        }
    }
}

impl LoopStatus {
    fn dbus_value(self) -> String {
        String::from(match self {
            LoopStatus::None => "None",
            LoopStatus::Track => "Track",
            LoopStatus::Playlist => "Playlist",
        })
    }
}

/// Something went wrong when communicating with the D-Bus. This could either be an underlying
/// D-Bus library problem, or that the other side did not conform to the expected protocols.
#[derive(Debug, Error)]
pub enum DBusError {
    /// An error occurred while talking to the D-Bus.
    #[error("D-Bus call failed: {0}")]
    TransportError(#[from] dbus::Error),

    /// Failed to parse an enum from a string value received from the [`Player`]. This means that the
    /// [`Player`] replied with unexpected data.
    #[error("Failed to parse enum value: {0}")]
    EnumParseError(String),

    /// A D-Bus method call did not pass arguments of the correct type. This means that the [`Player`]
    /// replied with unexpected data.
    #[error("D-Bus call failed: {0}")]
    TypeMismatchError(#[from] dbus::arg::TypeMismatchError),

    /// Some other unexpected error occurred.
    #[error("Unexpected error: {0}")]
    Miscellaneous(String),
}

impl From<InvalidPlaybackStatus> for DBusError {
    fn from(error: InvalidPlaybackStatus) -> Self {
        DBusError::EnumParseError(error.to_string())
    }
}

impl From<InvalidLoopStatus> for DBusError {
    fn from(error: InvalidLoopStatus) -> Self {
        DBusError::EnumParseError(error.to_string())
    }
}

#![warn(missing_docs)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//!
//! # mpris
//!
//! `mpris` is an idiomatic library for dealing with MPRIS2-compatible media players over D-Bus.
//!
//! This would mostly apply to the Linux-ecosystem which is a heavy user of D-Bus.
//!
//! ## Getting started
//!
//! Some hints on how to use this library:
//!
//! 1. Look at the examples under `examples/`.
//! 2. Look at the `PlayerFinder` struct.
//!

// Rust currently has a false-positive on unused_imports for proc macro crates:
// If it's imported with #[macro_use] it triggers the "Unused imports" lint.
// If you remove #[macro_use], then the custom derives stop working with a recommendation to add it
// again.
//
// Allowing unused_imports on this statement gets rid of the warning.
#[allow(unused_imports)]
#[macro_use]
extern crate failure_derive;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate enum_kinds;

#[macro_use]
extern crate derive_is_enum_variant;

#[macro_use]
extern crate from_variants;

extern crate dbus;

mod extensions;
mod generated;

mod event;
mod find;
mod metadata;
mod player;
mod pooled_connection;
mod progress;
mod track_list;

pub use event::{Event, EventError};
pub use find::{FindingError, PlayerFinder};
pub use metadata::{Metadata, Value as MetadataValue, ValueKind as MetadataValueKind};
pub use player::Player;
pub use progress::{Progress, ProgressError, ProgressTick, ProgressTracker};
pub use track_list::{TrackID, TrackList, TrackListError};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(missing_docs)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
/// A Player's looping status.
///
/// See: [MPRIS2 specification about
/// `Loop_Status`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Enum:Loop_Status)
pub enum LoopStatus {
    /// The playback will stop when there are no more tracks to play
    None,

    /// The current track will start again from the begining once it has finished playing
    Track,

    /// The playback loops through a list of tracks
    Playlist,
}

/// `PlaybackStatus` had an invalid string value.
#[derive(Fail, Debug)]
#[fail(
    display = "PlaybackStatus must be one of Playing, Paused, Stopped, but was {}",
    _0
)]
pub struct InvalidPlaybackStatus(String);

impl ::std::str::FromStr for PlaybackStatus {
    type Err = InvalidPlaybackStatus;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        use PlaybackStatus::*;

        match string {
            "Playing" => Ok(Playing),
            "Paused" => Ok(Paused),
            "Stopped" => Ok(Stopped),
            other => Err(InvalidPlaybackStatus(other.to_string())),
        }
    }
}

/// `LoopStatus` had an invalid string value.
#[derive(Fail, Debug)]
#[fail(
    display = "LoopStatus must be one of None, Track, Playlist, but was {}",
    _0
)]
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
#[derive(Fail, Debug)]
pub enum DBusError {
    /// An error occurred while talking to the D-Bus.
    #[fail(display = "D-Bus call failed: {}", _0)]
    TransportError(#[cause] dbus::Error),

    /// Failed to parse an enum from a string value received from the player. This means that the
    /// player replied with unexpected data.
    #[fail(display = "Failed to parse enum value: {}", _0)]
    EnumParseError(String),

    /// A D-Bus method call did not pass arguments of the correct type. This means that the player
    /// replied with unexpected data.
    #[fail(display = "D-Bus call failed: {}", _0)]
    TypeMismatchError(#[cause] dbus::arg::TypeMismatchError),

    /// Some other unexpected error occurred.
    #[fail(display = "Unexpected error: {}", _0)]
    Miscellaneous(String),
}

impl From<dbus::Error> for DBusError {
    fn from(error: dbus::Error) -> Self {
        DBusError::TransportError(error)
    }
}

impl From<dbus::arg::TypeMismatchError> for DBusError {
    fn from(error: dbus::arg::TypeMismatchError) -> Self {
        DBusError::TypeMismatchError(error)
    }
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

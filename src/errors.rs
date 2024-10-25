use std::fmt::Display;

/// [`PlaybackStatus`][crate::PlaybackStatus] had an invalid string value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidPlaybackStatus(pub(crate) String);

/// [`LoopStatus`][crate::LoopStatus] had an invalid string value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidLoopStatus(pub(crate) String);

/// [`TrackID`][crate::metadata::TrackID] had an invalid ObjectPath.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidTrackID(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidMprisDuration(pub(crate) String);

impl InvalidMprisDuration {
    pub(crate) fn new_too_big() -> Self {
        Self("can't create MprisDuration, value too big".to_string())
    }

    pub(crate) fn new_negative() -> Self {
        Self("can't create MprisDuration, value is negative".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidMetadataValue(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidMetadata(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidPlaylist(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidPlaylistOrdering(pub(crate) String);

macro_rules! impl_display {
    ($error:ty) => {
        impl Display for $error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $error {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $error {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }
    };
}

impl_display!(InvalidPlaybackStatus);
impl_display!(InvalidLoopStatus);
impl_display!(InvalidTrackID);
impl_display!(InvalidMprisDuration);
impl_display!(InvalidMetadataValue);
impl_display!(InvalidMetadata);
impl_display!(InvalidPlaylist);
impl_display!(InvalidPlaylistOrdering);

#[derive(Debug, PartialEq, Clone)]
pub enum MprisError {
    /// An error occurred while talking to the D-Bus.
    DbusError(zbus::Error),

    /// Failed to parse an enum from a string value received from the [`Player`][crate::Player].
    /// This means that the [`Player`][crate::Player] replied with unexpected data.
    ParseError(String),

    /// The player doesn't implement the required interface/method/signal
    Unsupported,

    /// One of the given arguments has an invalid value
    InvalidArgument(String),

    /// Some other unexpected error occurred.
    Miscellaneous(String),
}

impl MprisError {
    pub(crate) fn track_id_is_no_track() -> Self {
        Self::InvalidArgument(
            "/org/mpris/MediaPlayer2/TrackList/NoTrack is not a valid value".to_owned(),
        )
    }
}

impl From<zbus::Error> for MprisError {
    fn from(value: zbus::Error) -> Self {
        match value {
            zbus::Error::InterfaceNotFound | zbus::Error::Unsupported => Self::Unsupported,
            _ => todo!(),
        }
    }
}

impl From<InvalidPlaybackStatus> for MprisError {
    fn from(value: InvalidPlaybackStatus) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<InvalidLoopStatus> for MprisError {
    fn from(value: InvalidLoopStatus) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<InvalidTrackID> for MprisError {
    fn from(value: InvalidTrackID) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<InvalidMprisDuration> for MprisError {
    fn from(value: InvalidMprisDuration) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<InvalidMetadata> for MprisError {
    fn from(value: InvalidMetadata) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<InvalidPlaylistOrdering> for MprisError {
    fn from(value: InvalidPlaylistOrdering) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<InvalidPlaylist> for MprisError {
    fn from(value: InvalidPlaylist) -> Self {
        Self::ParseError(value.0)
    }
}

impl From<String> for MprisError {
    fn from(value: String) -> Self {
        Self::Miscellaneous(value)
    }
}

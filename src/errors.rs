use std::fmt::Display;

pub use zbus::Error;

#[rustfmt::skip]
macro_rules! generate_error {
    ($error:ident, $source:ident) => {
        #[doc=concat!(
            "Error for when [`",
            stringify!($source),
            "`](crate::",
            stringify!($source),
            ") ",
            "failed to be created."
            )
        ]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $error(pub(crate) String);

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

generate_error!(InvalidPlaybackStatus, PlaybackStatus);
generate_error!(InvalidLoopStatus, LoopStatus);
generate_error!(InvalidTrackID, TrackID);
generate_error!(InvalidMprisDuration, MprisDuration);
generate_error!(InvalidMetadataValue, MetadataValue);
generate_error!(InvalidMetadata, Metadata);
generate_error!(InvalidPlaylist, Playlist);
generate_error!(InvalidPlaylistOrdering, PlaylistOrdering);

impl InvalidMprisDuration {
    pub(crate) fn new_too_big() -> Self {
        Self("can't create MprisDuration, value too big".to_string())
    }

    pub(crate) fn new_negative() -> Self {
        Self("can't create MprisDuration, value is negative".to_string())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MprisError {
    /// An error occurred while talking to the D-Bus.
    DbusError(Error),

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

impl From<Error> for MprisError {
    fn from(value: Error) -> Self {
        match value {
            Error::InterfaceNotFound | Error::Unsupported => Self::Unsupported,
            _ => Self::DbusError(value),
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

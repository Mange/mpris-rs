extern crate dbus;
use super::{DBusError, Metadata, Player};
use std::fmt;

/// Represents [the MPRIS `Track_Id` type][track_id].
///
/// ```rust
/// use mpris::TrackID;
/// let no_track = TrackID::from("/org/mpris/MediaPlayer2/TrackList/NoTrack");
/// ```
///
/// **Note:** There is currently no good way to retrieve values for this through the `mpris`
/// library. You will have to manually retrieve them through D-Bus until implemented.
///
/// # Panics
///
/// Trying to construct a `TrackID` from a string that is not a valid D-Bus Path will result in a
/// panic.
///
/// [track_id]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Simple-Type:Track_Id
#[derive(Debug, Clone, PartialEq)]
pub struct TrackID<'a>(pub(crate) dbus::Path<'a>);

/// Represents a MediaPlayer2.TrackList.
///
/// This type offers an iterator of the track's metadata, when provided a `Player` instance that
/// matches the list.
///
/// See [MediaPlayer2.TrackList
/// interface](https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html)
#[derive(Debug)]
pub struct TrackList<'a> {
    ids: Vec<TrackID<'a>>,
}

impl<'a, T> From<T> for TrackID<'a>
where
    T: Into<dbus::Path<'a>>,
{
    fn from(value: T) -> TrackID<'a> {
        TrackID(value.into())
    }
}

impl<'a> fmt::Display for TrackID<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> TrackID<'a> {
    /// Create a new `TrackID` from a string-like entity.
    ///
    /// This is not something you should normally do as the IDs are temporary and will only work if
    /// the Player knows about it.
    ///
    /// However, creating `TrackID`s manually can help with test setup, comparisons, etc.
    ///
    /// # Example
    /// ```rust
    /// use mpris::TrackID;
    /// let id = TrackID::new("/dbus/path/id").expect("Parse error");
    /// ```
    pub fn new<S: Into<Vec<u8>>>(s: S) -> Result<Self, String> {
        dbus::Path::new(s).map(TrackID)
    }
}

impl<'a> From<Vec<TrackID<'a>>> for TrackList<'a> {
    fn from(ids: Vec<TrackID<'a>>) -> Self {
        TrackList { ids }
    }
}

impl<'a> From<Vec<dbus::Path<'a>>> for TrackList<'a> {
    fn from(ids: Vec<dbus::Path<'a>>) -> Self {
        TrackList {
            ids: ids.into_iter().map(TrackID::from).collect(),
        }
    }
}

impl<'a> TrackList<'a> {
    /// Iterates the tracks in the tracklist, returning a tuple of TrackID and Metadata for that
    /// track.
    ///
    /// If metadata loading fails, then a DBusError will be returned instead.
    pub fn metadata_iter(
        &self,
        player: &Player,
    ) -> Result<impl Iterator<Item = (&TrackID<'a>, Metadata)>, DBusError> {
        Ok(self.ids.iter().zip(player.get_tracks_metadata(&self.ids)?))
    }
}

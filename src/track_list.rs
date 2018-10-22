extern crate dbus;
use super::{DBusError, Metadata, Player};
use std::cell::RefCell;
use std::collections::HashMap;
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
/// TrackLists cache metadata about tracks so multiple iterations should be fast. It also enables
/// signals received from the Player to pre-populate metadata and to keep everything up to date.
///
/// See [MediaPlayer2.TrackList
/// interface](https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html)
#[derive(Debug)]
pub struct TrackList<'a> {
    ids: Vec<TrackID<'a>>,
    metadata_cache: RefCell<HashMap<String, Metadata>>,
}

#[derive(Debug)]
pub struct MetadataIter {
    order: Vec<String>,
    metadata: HashMap<String, Metadata>,
    current: usize,
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

    /// Returns a `&str` variant of the ID.
    pub fn as_str(&self) -> &str {
        &*self.0
    }
}

impl<'a> From<Vec<TrackID<'a>>> for TrackList<'a> {
    fn from(ids: Vec<TrackID<'a>>) -> Self {
        TrackList {
            metadata_cache: RefCell::new(HashMap::with_capacity(ids.len())),
            ids,
        }
    }
}

impl<'a> From<Vec<dbus::Path<'a>>> for TrackList<'a> {
    fn from(ids: Vec<dbus::Path<'a>>) -> Self {
        TrackList {
            metadata_cache: RefCell::new(HashMap::with_capacity(ids.len())),
            ids: ids.into_iter().map(TrackID::from).collect(),
        }
    }
}

impl<'a> TrackList<'a> {
    /// Iterates the tracks in the tracklist, returning a tuple of TrackID and Metadata for that
    /// track.
    ///
    /// If metadata loading fails, then a DBusError will be returned instead.
    pub fn metadata_iter(&self, player: &Player) -> Result<MetadataIter, DBusError> {
        self.complete_cache(player)?;
        let metadata: HashMap<_, _> = self.metadata_cache.clone().into_inner();
        let ids: Vec<_> = self.ids.iter().map(TrackID::to_string).collect();

        Ok(MetadataIter {
            current: 0,
            order: ids,
            metadata,
        })
    }

    /// Clears all cache and reloads metadata for all tracks.
    ///
    /// Cache will be replaced *after* the new metadata has been loaded, so on load errors the
    /// cache will still be maintained.
    pub fn reload_cache(&self, player: &Player) -> Result<(), DBusError> {
        let id_metadata = self
            .ids
            .iter()
            .map(TrackID::to_string)
            .zip(player.get_tracks_metadata(&self.ids)?);
        let mut cache = self.metadata_cache.borrow_mut();
        *cache = id_metadata.collect();
        Ok(())
    }

    /// Fill in any holes in the cache so that each track on the list has a cached Metadata entry.
    ///
    /// If all tracks already have a cache entry, then this will do nothing.
    pub fn complete_cache(&self, player: &Player) -> Result<(), DBusError> {
        let ids: Vec<_> = self
            .ids_without_cache()
            .into_iter()
            .map(Clone::clone)
            .collect();
        if !ids.is_empty() {
            let metadata = player.get_tracks_metadata(&ids)?;
            let mut cache = self.metadata_cache.borrow_mut();
            for (metadata, id) in metadata.into_iter().zip(ids.into_iter()) {
                cache.insert(id.to_string(), metadata);
            }
        }
        Ok(())
    }

    fn ids_without_cache(&self) -> Vec<&TrackID<'a>> {
        let cache = &*self.metadata_cache.borrow();
        self.ids
            .iter()
            .filter(|id| !cache.contains_key(id.as_str()))
            .collect()
    }
}

impl Iterator for MetadataIter {
    type Item = Metadata;

    fn next(&mut self) -> Option<Self::Item> {
        match self.order.get(self.current) {
            Some(next_id) => {
                self.current += 1;
                // In case of race conditions with cache population, emit a simple Metadata without
                // any interesting data in it.
                Some(
                    self.metadata
                        .remove(next_id)
                        .unwrap_or_else(|| Metadata::new(next_id.clone())),
                )
            }
            None => None,
        }
    }
}

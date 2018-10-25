extern crate dbus;
use super::{DBusError, Metadata, Player};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::iter::{FromIterator, IntoIterator};

/// Represents [the MPRIS `Track_Id` type][track_id].
///
/// ```rust
/// use mpris::TrackID;
/// let no_track = TrackID::new("/org/mpris/MediaPlayer2/TrackList/NoTrack").unwrap();
/// ```
///
/// TrackIDs must be valid D-Bus object paths according to the spec.
///
/// # Errors
///
/// Trying to construct a `TrackID` from a string that is not a valid D-Bus Path will fail.
///
/// ```rust
/// # use mpris::TrackID;
/// let result = TrackID::new("invalid track ID");
/// assert!(result.is_err());
/// ```
///
/// [track_id]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Simple-Type:Track_Id
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct TrackID(pub(crate) String);

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
#[derive(Debug, Default)]
pub struct TrackList {
    ids: Vec<TrackID>,
    metadata_cache: RefCell<HashMap<TrackID, Metadata>>,
}

#[derive(Debug)]
pub struct MetadataIter {
    order: Vec<TrackID>,
    metadata: HashMap<TrackID, Metadata>,
    current: usize,
}

impl<'a> From<dbus::Path<'a>> for TrackID {
    fn from(path: dbus::Path<'a>) -> TrackID {
        TrackID(path.to_string())
    }
}

impl From<TrackID> for String {
    fn from(id: TrackID) -> String {
        id.0
    }
}

impl fmt::Display for TrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TrackID {
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
    pub fn new<S: Into<String>>(id: S) -> Result<Self, String> {
        let id = id.into();
        // Validate the ID by constructing a dbus::Path.
        if let Err(error) = dbus::Path::new(id.as_str()) {
            Err(error)
        } else {
            Ok(TrackID(id))
        }
    }

    /// Returns a `&str` variant of the ID.
    pub fn as_str(&self) -> &str {
        &*self.0
    }

    pub(crate) fn as_path(&self) -> dbus::Path {
        // All inputs to this class should be validated to work with dbus::Path, so unwrapping
        // should be safe here.
        dbus::Path::new(self.as_str()).unwrap()
    }
}

impl From<Vec<TrackID>> for TrackList {
    fn from(ids: Vec<TrackID>) -> Self {
        TrackList {
            metadata_cache: RefCell::new(HashMap::with_capacity(ids.len())),
            ids,
        }
    }
}

impl<'a> From<Vec<dbus::Path<'a>>> for TrackList {
    fn from(ids: Vec<dbus::Path<'a>>) -> Self {
        ids.into_iter().map(TrackID::from).collect()
    }
}

impl FromIterator<TrackID> for TrackList {
    fn from_iter<I: IntoIterator<Item = TrackID>>(iter: I) -> Self {
        TrackList::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl TrackList {
    /// Returns the number of tracks on the list.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Iterates the tracks in the tracklist, returning a tuple of TrackID and Metadata for that
    /// track.
    ///
    /// If metadata loading fails, then a DBusError will be returned instead.
    pub fn metadata_iter(&self, player: &Player) -> Result<MetadataIter, DBusError> {
        self.complete_cache(player)?;
        let metadata: HashMap<_, _> = self.metadata_cache.clone().into_inner();
        let ids = self.ids.clone();

        Ok(MetadataIter {
            current: 0,
            order: ids,
            metadata,
        })
    }

    /// Reloads the tracklist from the given player. This can be compared with loading a new track
    /// list, but in this case the metadata cache can be maintained for tracks that remain on the
    /// list.
    ///
    /// Cache for tracks that are no longer part of the player's tracklist will be removed.
    pub fn reload(&mut self, player: &Player) -> Result<(), DBusError> {
        self.ids = player.get_track_list()?.ids;
        self.clear_extra_cache();
        Ok(())
    }

    /// Clears all cache and reloads metadata for all tracks.
    ///
    /// Cache will be replaced *after* the new metadata has been loaded, so on load errors the
    /// cache will still be maintained.
    pub fn reload_cache(&self, player: &Player) -> Result<(), DBusError> {
        let id_metadata = self
            .ids
            .iter()
            .cloned()
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
                cache.insert(id, metadata);
            }
        }
        Ok(())
    }

    fn ids_without_cache(&self) -> Vec<&TrackID> {
        let cache = &*self.metadata_cache.borrow();
        self.ids
            .iter()
            .filter(|id| !cache.contains_key(id))
            .collect()
    }

    fn clear_extra_cache(&mut self) {
        // &mut self means that no other reference to self exists, so it should always be safe to
        // mutably borrow the cache.
        let mut cache = self.metadata_cache.borrow_mut();

        // For each id in the list, move the cache out into a new HashMap, then replace the old
        // one with the new. Only ids on the list will therefore be present on the new list.
        let new_cache: HashMap<TrackID, Metadata> = self
            .ids
            .iter()
            .flat_map(|id| match cache.remove(&id) {
                Some(value) => Some((id.clone(), value)),
                None => None,
            }).collect();

        *cache = new_cache;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn track_id(s: &str) -> TrackID {
        TrackID::new(s).expect("Failed to parse a TrackID fixture")
    }

    mod track_list {
        use super::*;

        #[test]
        fn it_inserts_after_given_id() {
            let first = track_id("/path/1");
            let third = track_id("/path/3");

            let mut list = TrackList {
                ids: vec![first, third],
                metadata_cache: RefCell::new(HashMap::new()),
            };

            let metadata = Metadata::new("/path/new");
            list.insert(&track_id("/path/1"), metadata);

            assert_eq!(list.len(), 3);
            assert_eq!(
                &list.ids,
                &[
                    track_id("/path/1"),
                    track_id("/path/new"),
                    track_id("/path/3")
                ]
            );
            assert_eq!(
                list.ids_without_cache(),
                vec![&track_id("/path/1"), &track_id("/path/3")],
            );
        }

        #[test]
        fn it_inserts_at_end_on_missing_id() {
            let first = track_id("/path/1");
            let third = track_id("/path/3");

            let mut list = TrackList {
                ids: vec![first, third],
                metadata_cache: RefCell::new(HashMap::new()),
            };

            let metadata = Metadata::new("/path/new");
            list.insert(&track_id("/path/missing"), metadata);

            assert_eq!(list.len(), 3);
            assert_eq!(
                &list.ids,
                &[
                    track_id("/path/1"),
                    track_id("/path/3"),
                    track_id("/path/new"),
                ]
            );
            assert_eq!(
                list.ids_without_cache(),
                vec![&track_id("/path/1"), &track_id("/path/3")],
            );
        }
    }
}
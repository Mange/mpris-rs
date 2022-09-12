use super::{DBusError, Metadata, Player};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::iter::{FromIterator, IntoIterator};
use thiserror::Error;

pub(crate) const NO_TRACK: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";

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
/// Trying to construct a [`TrackID`] from a string that is not a valid D-Bus Path will fail.
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

/// Represents a [`MediaPlayer2.TrackList`][track_list].
///
/// This type offers an iterator of the track's metadata, when provided a [`Player`] instance that
/// matches the list.
///
/// TrackLists cache metadata about tracks so multiple iterations should be fast. It also enables
/// signals received from the Player to pre-populate metadata and to keep everything up to date.
///
/// [track_list]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html
#[derive(Debug, Default)]
pub struct TrackList {
    ids: Vec<TrackID>,
    metadata_cache: RefCell<HashMap<TrackID, Metadata>>,
}

/// TrackList-related errors.
///
/// This is mostly [`DBusError`] with the extra possibility of borrow errors of the internal metadata
/// cache.
#[derive(Debug, Error)]
pub enum TrackListError {
    /// Something went wrong with the D-Bus communication. See the [`DBusError`] type.
    #[error("D-Bus communication failed: {0}")]
    DBusError(#[from] DBusError),

    /// Something went wrong with the borrowing logic for the internal cache. Perhaps you have
    /// multiple borrowed references to the cache live at the same time, for example because of
    /// multiple iterations?
    #[error("Could not borrow cache: {0}")]
    BorrowError(String),
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

impl<'a> From<&'a TrackID> for TrackID {
    fn from(id: &'a TrackID) -> Self {
        TrackID(id.0.clone())
    }
}

impl From<TrackID> for String {
    fn from(id: TrackID) -> String {
        id.0
    }
}

impl<'a> From<&'a TrackID> for dbus::Path<'a> {
    fn from(id: &'a TrackID) -> dbus::Path<'a> {
        id.as_path()
    }
}

impl fmt::Display for TrackID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TrackID {
    /// Create a new [`TrackID`] from a string-like entity.
    ///
    /// This is not something you should normally do as the IDs are temporary and will only work if
    /// the Player knows about it.
    ///
    /// However, creating [`TrackID`]s manually can help with test setup, comparisons, etc.
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

    /// Return a new [`TrackID`] that matches the MPRIS standard for the "No track" sentinel value.
    ///
    /// Some APIs takes this in order to signal a missing value for a track, for example by saying
    /// that no specific track is playing, or that a track should be added at the start of the
    /// list instead of after a specific track.
    ///
    /// The actual path is "/org/mpris/MediaPlayer2/TrackList/NoTrack".
    ///
    /// This value is only valid in some cases. Make sure to read the [MPRIS specification before
    /// you use this manually][track_id].
    ///
    /// [track_id]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Simple-Type:Track_Id
    pub fn no_track() -> Self {
        TrackID(NO_TRACK.into())
    }

    /// Returns a `&str` variant of the ID.
    pub fn as_str(&self) -> &str {
        &*self.0
    }

    pub(crate) fn as_path(&self) -> dbus::Path<'_> {
        // All inputs to this class should be validated to work with [`dbus::Path`], so unwrapping
        // should be safe here.
        dbus::Path::new(self.as_str()).unwrap()
    }
}

impl From<Vec<TrackID>> for TrackList {
    fn from(ids: Vec<TrackID>) -> Self {
        TrackList::new(ids)
    }
}

impl<'a> From<Vec<dbus::Path<'a>>> for TrackList {
    fn from(ids: Vec<dbus::Path<'a>>) -> Self {
        ids.into_iter().map(TrackID::from).collect()
    }
}

impl FromIterator<TrackID> for TrackList {
    fn from_iter<I: IntoIterator<Item = TrackID>>(iter: I) -> Self {
        TrackList::new(iter.into_iter().collect())
    }
}

impl TrackList {
    /// Construct a new [`TrackList`] without any existing cache.
    pub fn new(ids: Vec<TrackID>) -> TrackList {
        TrackList {
            metadata_cache: RefCell::new(HashMap::with_capacity(ids.len())),
            ids,
        }
    }

    /// Get a list of [`TrackID`]s that are part of this [`TrackList`]. The order matters.
    pub fn ids(&self) -> &[TrackID] {
        self.ids.as_ref()
    }

    /// Returns the number of tracks on the list.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// If the tracklist is empty or not.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Return the [`TrackID`] of the index. Out-of-bounds will result in [`None`].
    pub fn get(&self, index: usize) -> Option<&TrackID> {
        self.ids.get(index)
    }

    /// Insert a new track (via its metadata) after another one. If the provided ID cannot be found
    /// on the list, it will be inserted at the end.
    ///
    /// **NOTE:** This is *not* something that will affect a player's actual tracklist; this is
    /// strictly for client-side representation. Use this if you want to maintain your own instance
    /// of [`TrackList`] or to feed your code with test fixtures.
    pub fn insert(&mut self, after: &TrackID, metadata: Metadata) {
        let new_id = match metadata.track_id() {
            Some(val) => val,
            // Cannot insert ID if there is no ID in the metadata.
            None => return,
        };

        let index = self.index_of_id(after).unwrap_or_else(|| self.ids.len());

        // Vec::insert inserts BEFORE the given index, but we need to insert *after* the index.
        if index >= self.ids.len() {
            self.ids.push(new_id.clone());
        } else {
            self.ids.insert(index + 1, new_id.clone());
        }

        self.change_metadata(|cache| cache.insert(new_id, metadata));
    }

    /// Removes a track from the list and metadata cache.
    ///
    /// **Note:** If the same id is present multiple times, all of them will be removed.
    pub fn remove(&mut self, id: &TrackID) {
        self.ids.retain(|existing_id| existing_id != id);

        self.change_metadata(|cache| cache.remove(id));
    }

    /// Clears the entire list and cache.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.change_metadata(|cache| cache.clear());
    }

    /// Replace the contents with the contents of the provided list. Cache will be reused when
    /// possible.
    pub fn replace(&mut self, other: TrackList) {
        self.ids = other.ids;
        let other_cache = other.metadata_cache.into_inner();

        self.change_metadata(|self_cache| {
            // Will overwrite existing keys on conflicts; e.g. the newer cache wins.
            self_cache.extend(other_cache.into_iter());
        });
    }

    /// Adds/updates the metadata cache for a track (as identified by [`Metadata::track_id`]).
    ///
    /// The metadata will be added to the cache even if the [`TrackID`] isn't part of the list, but
    /// will be cleaned out again after the next cache cleanup unless the track in question have
    /// been added to the list before then.
    ///
    /// If provided metadata does not contain a [`TrackID`], the metadata will be discarded.
    pub fn add_metadata(&mut self, metadata: Metadata) {
        if let Some(id) = metadata.track_id() {
            self.change_metadata(|cache| cache.insert(id.to_owned(), metadata));
        }
    }

    /// Replaces a track on the list with a new entry. The new metadata could contain a new track
    /// ID, and will in that case replace the old ID on the tracklist.
    ///
    /// The new ID (which *might* be identical to the old ID) will be returned by this method.
    ///
    /// If the old ID cannot be found, the metadata will be discarded and [`None`] will be returned.
    ///
    /// If provided metadata does not contain a [`TrackID`], the metadata will be discarded and
    /// [`None`] will be returned.
    pub fn replace_track_metadata(
        &mut self,
        old_id: &TrackID,
        new_metadata: Metadata,
    ) -> Option<TrackID> {
        if let Some(new_id) = new_metadata.track_id() {
            if let Some(index) = self.index_of_id(old_id) {
                self.ids[index] = new_id.to_owned();
                self.change_metadata(|cache| cache.insert(new_id.to_owned(), new_metadata));

                return Some(new_id);
            }
        }

        None
    }

    /// Iterates the tracks in the tracklist, returning a tuple of [`TrackID`] and [`Metadata`] for that
    /// track.
    ///
    /// [`Metadata`] will be loaded from the provided player when not present in the metadata cache.
    /// If metadata loading fails, then a [`DBusError`] will be returned instead of the iterator.
    pub fn metadata_iter(&self, player: &Player) -> Result<MetadataIter, TrackListError> {
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
    pub fn reload(&mut self, player: &Player) -> Result<(), TrackListError> {
        self.ids = player.get_track_list()?.ids;
        self.clear_extra_cache();
        Ok(())
    }

    /// Clears all cache and reloads metadata for all tracks.
    ///
    /// Cache will be replaced *after* the new metadata has been loaded, so on load errors the
    /// cache will still be maintained.
    pub fn reload_cache(&self, player: &Player) -> Result<(), TrackListError> {
        let id_metadata = self
            .ids
            .iter()
            .cloned()
            .zip(player.get_tracks_metadata(&self.ids)?);

        // We only have a &self reference, so fail if we cannot borrow.
        let mut cache = self.metadata_cache.try_borrow_mut()?;
        *cache = id_metadata.collect();

        Ok(())
    }

    /// Fill in any holes in the cache so that each track on the list has a cached [`Metadata`] entry.
    ///
    /// If all tracks already have a cache entry, then this will do nothing.
    pub fn complete_cache(&self, player: &Player) -> Result<(), TrackListError> {
        let ids: Vec<_> = self
            .ids_without_cache()
            .into_iter()
            .map(Clone::clone)
            .collect();
        if !ids.is_empty() {
            let metadata = player.get_tracks_metadata(&ids)?;

            // We only have a &self reference, so fail if we cannot borrow.
            let mut cache = self.metadata_cache.try_borrow_mut()?;

            for info in metadata.into_iter() {
                if let Some(id) = info.track_id() {
                    cache.insert(id, info);
                }
            }
        }
        Ok(())
    }

    /// Change metadata cache. As this requires a `&mut self`, the borrow is guaranteed to work.
    fn change_metadata<T, F>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut HashMap<TrackID, Metadata>) -> T,
    {
        let mut cache = self.metadata_cache.borrow_mut(); // Safe. &mut self reference.
        f(&mut *cache)
    }

    fn ids_without_cache(&self) -> Vec<&TrackID> {
        let cache = &*self.metadata_cache.borrow();
        self.ids
            .iter()
            .filter(|id| !cache.contains_key(id))
            .collect()
    }

    fn clear_extra_cache(&mut self) {
        let ids: Vec<TrackID> = self.ids().iter().map(TrackID::from).collect();

        self.change_metadata(|cache| {
            // For each id in the list, move the cache out into a new HashMap, then replace the old
            // one with the new. Only ids on the list will therefore be present on the new list.
            let new_cache: HashMap<TrackID, Metadata> = ids
                .iter()
                .flat_map(|id| cache.remove(id).map(|value| (id.to_owned(), value)))
                .collect();

            *cache = new_cache;
        });
    }

    fn index_of_id(&self, id: &TrackID) -> Option<usize> {
        self.ids
            .iter()
            .enumerate()
            .find(|(_, item_id)| *item_id == id)
            .map(|(index, _)| index)
    }
}

impl PartialEq<TrackList> for TrackList {
    fn eq(&self, other: &TrackList) -> bool {
        self.ids.eq(&other.ids)
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

impl From<::std::cell::BorrowMutError> for TrackListError {
    fn from(error: ::std::cell::BorrowMutError) -> TrackListError {
        TrackListError::BorrowError(format!("Could not borrow mutably: {}", error))
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

        #[test]
        fn it_inserts_at_end_on_empty() {
            let mut list = TrackList::default();

            let metadata = Metadata::new("/path/new");
            list.insert(&track_id("/path/missing"), metadata);

            assert_eq!(list.len(), 1);
            assert_eq!(&list.ids, &[track_id("/path/new")]);
            assert!(list.ids_without_cache().is_empty());
        }
    }
}

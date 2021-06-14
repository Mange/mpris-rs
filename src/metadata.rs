mod value;
pub use self::value::{Value, ValueKind};
use super::TrackID;

use std::collections::HashMap;
use std::time::Duration;

/// A structured representation of the [`Player`](crate::player::Player) metadata.
///
/// * [Read more about the MPRIS2 `Metadata_Map` type.][metadata_map]
/// * [Read MPRIS v2 metadata guidelines][metadata_guidelines]
///
/// [metadata_map]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Mapping:Metadata_Map
/// [metadata_guidelines]: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
#[derive(Debug, Default, Clone)]
pub struct Metadata {
    values: HashMap<String, Value>,
}

impl Metadata {
    /// Create a new [`Metadata`] struct with a given `track_id`.
    ///
    /// This is mostly useful for test fixtures and other places where you want to work with mock
    /// data.
    pub fn new<S>(track_id: S) -> Self
    where
        S: Into<String>,
    {
        let mut values = HashMap::with_capacity(1);
        values.insert(
            String::from("mpris:trackid"),
            Value::String(track_id.into()),
        );

        Metadata { values }
    }

    /// Get a value from the metadata by key name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use mpris::{Metadata, MetadataValue};
    /// # let mut metadata = Metadata::new(String::from("1234"));
    /// # let key_name = "foo";
    /// if let Some(MetadataValue::String(name)) = metadata.get("xesam:composer") {
    ///     println!("Composed by: {}", name);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// The track ID.
    ///
    /// If the [`TrackID`] could not be parsed as a proper [`TrackID`], [`None`] will be returned.
    ///
    /// Based on `mpris:trackid`
    /// > A unique identity for this track within the context of an MPRIS object.
    ///
    pub fn track_id(&self) -> Option<TrackID> {
        self.get("mpris:trackid")
            .and_then(Value::as_str)
            .and_then(|v| TrackID::new(v).ok())
    }

    /// A list of artists of the album the track appears on.
    ///
    /// Based on `xesam:albumArtist`
    /// > The album artist(s).
    pub fn album_artists(&self) -> Option<Vec<&str>> {
        self.get("xesam:albumArtist").and_then(Value::as_str_array)
    }

    /// The name of the album the track appears on.
    ///
    /// Based on `xesam:album`
    /// > The album name.
    pub fn album_name(&self) -> Option<&str> {
        self.get("xesam:album").and_then(Value::as_str)
    }

    /// An URL to album art of the current track.
    ///
    /// Based on `mpris:artUrl`
    /// > The location of an image representing the track or album. Clients should not assume this
    /// > will continue to exist when the media player stops giving out the URL.
    pub fn art_url(&self) -> Option<&str> {
        self.get("mpris:artUrl").and_then(Value::as_str)
    }

    /// A list of artists of the track.
    ///
    /// Based on `xesam:artist`
    /// > The track artist(s).
    pub fn artists(&self) -> Option<Vec<&str>> {
        self.get("xesam:artist").and_then(Value::as_str_array)
    }

    /// Based on `xesam:autoRating`
    /// > An automatically-generated rating, based on things such as how often it has been played.
    /// > This should be in the range 0.0 to 1.0.
    pub fn auto_rating(&self) -> Option<f64> {
        self.get("xesam:autoRating").and_then(Value::as_f64)
    }

    /// Based on `xesam:discNumber`
    /// > The disc number on the album that this track is from.
    pub fn disc_number(&self) -> Option<i32> {
        self.get("xesam:discNumber").and_then(Value::as_i32)
    }

    /// The duration of the track, in microseconds
    ///
    /// Based on `mpris:length`
    /// > The duration of the track in microseconds.
    pub fn length_in_microseconds(&self) -> Option<u64> {
        match self.get("mpris:length") {
            Some(Value::I64(len)) => Some(*len as u64),
            Some(Value::U64(len)) => Some(*len),
            Some(_) => None,
            None => None,
        }
    }

    /// The duration of the track, as a [`Duration`]
    ///
    /// Based on `mpris:length`.
    pub fn length(&self) -> Option<Duration> {
        use crate::extensions::DurationExtensions;
        self.length_in_microseconds().map(Duration::from_micros_ext)
    }

    /// The name of the track.
    ///
    /// Based on `xesam:title`
    /// > The track title.
    pub fn title(&self) -> Option<&str> {
        self.get("xesam:title").and_then(Value::as_str)
    }

    /// The track number on the disc of the album the track appears on.
    ///
    /// Based on `xesam:trackNumber`
    /// > The track number on the album disc.
    pub fn track_number(&self) -> Option<i32> {
        self.get("xesam:trackNumber").and_then(Value::as_i32)
    }

    /// A URL to the media being played.
    ///
    /// Based on `xesam:url`
    /// > The location of the media file.
    pub fn url(&self) -> Option<&str> {
        self.get("xesam:url").and_then(Value::as_str)
    }

    /// Returns an owned [`HashMap`] of borrowed values from this [`Metadata`]. Useful if you need a
    /// mutable hash but don't have ownership of [`Metadata`] or want to consume it.
    ///
    /// If you want to convert to a [`HashMap`], use [`Into::into`](std::convert::Into::into) instead.
    pub fn as_hashmap(&self) -> HashMap<&str, &Value> {
        self.iter().collect()
    }

    /// Iterate all metadata keys and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate all metadata keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    /// Returns [`true`] if there is no metadata
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl IntoIterator for Metadata {
    type Item = (String, Value);
    type IntoIter = std::collections::hash_map::IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

// Disable implicit_hasher; suggested code fix does not compile. I think this might be a false
// positive, but I'm not sure.
#[cfg_attr(feature = "cargo-clippy", allow(clippy::implicit_hasher))]
impl From<Metadata> for HashMap<String, Value> {
    fn from(metadata: Metadata) -> Self {
        metadata.values
    }
}

impl From<HashMap<String, Value>> for Metadata {
    fn from(values: HashMap<String, Value>) -> Self {
        Metadata { values }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_creates_new_metadata() {
        let metadata = Metadata::new("/foo");
        assert_eq!(metadata.track_id(), Some(TrackID::new("/foo").unwrap()));
    }

    #[test]
    fn it_supports_blank_metadata() {
        let metadata = Metadata::from(HashMap::new());
        assert_eq!(metadata.track_id(), None);
    }

    #[test]
    fn it_builds_values_hash() {
        let mut input_hash: HashMap<String, Value> = HashMap::new();
        input_hash.insert(String::from("xesam:trackNumber"), Value::from(42));

        let metadata = Metadata::from(input_hash.clone());
        let output_hash = metadata.as_hashmap();

        assert_eq!(input_hash.get("xesam:trackNumber"), Some(&Value::I32(42)));
        assert_eq!(output_hash.get("xesam:trackNumber"), Some(&&Value::I32(42)));
    }

    #[test]
    fn it_has_iterators() {
        let mut input_hash: HashMap<String, Value> = HashMap::new();
        input_hash.insert(String::from("xesam:trackNumber"), Value::from(42));
        let metadata = Metadata::from(input_hash);

        let keys: Vec<&str> = metadata.keys().collect();
        assert_eq!(keys, vec!["xesam:trackNumber"]);

        let keyvals: Vec<(&str, &Value)> = metadata.iter().collect();
        assert_eq!(keyvals, vec![("xesam:trackNumber", &Value::I32(42))]);

        for (key, val) in metadata {
            assert_eq!(key, String::from("xesam:trackNumber"));
            assert_eq!(val, Value::I32(42));
        }
    }
}

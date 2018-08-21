extern crate dbus;

mod value;
pub use self::value::{Value, ValueKind};

use std::collections::HashMap;
use std::time::Duration;

use dbus::arg::{RefArg, Variant};

/// A structured representation of the `Player` metadata.
///
/// * [Read more about the MPRIS2 `Metadata_Map`
/// type.](https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Mapping:Metadata_Map)
/// * [Read MPRIS v2 metadata guidelines](https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/)
#[derive(Debug, Default, Clone)]
pub struct Metadata {
    values: HashMap<String, Value>,
}

impl Metadata {
    /// Create a new `Metadata` struct with a given `track_id`.
    ///
    /// This is mostly useful for test fixtures and other places where you want to work with mock
    /// data.
    pub fn new<S>(track_id: S) -> Self where S: Into<String> {
        let mut values = HashMap::with_capacity(1);
        values.insert(String::from("mpris:trackid"), Value::String(track_id.into()));

        Metadata {
            values
        }
    }

    pub(crate) fn new_from_dbus(
        metadata: HashMap<String, Variant<Box<RefArg + 'static>>>,
    ) -> Metadata {
        Metadata {
            values: metadata.into_iter().flat_map(|(key, variant)| {
                match Value::from_variant(variant) {
                    Some(value) => Some((key, value)),
                    None => None,
                }
            }).collect(),
        }
    }

    /// Get a value from the metadata by key name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate mpris;
    /// # extern crate dbus;
    /// # use mpris::{Metadata, MetadataValue};
    /// # fn main() {
    /// # let mut metadata = Metadata::new(String::from("1234"));
    /// # let key_name = "foo";
    /// if let Some(MetadataValue::String(name)) = metadata.get("xesam:composer") {
    ///     println!("Composed by: {}", name);
    /// }
    /// # }
    /// ```
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// The track ID.
    ///
    /// Based on `mpris:trackid`
    /// > A unique identity for this track within the context of an MPRIS object.
    ///
    pub fn track_id(&self) -> Option<&str> {
        self.get("mpris:trackid").and_then(Value::as_str)
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
        self.get("mpris:length").and_then(Value::as_u64)
    }

    /// The duration of the track, as a `Duration`
    ///
    /// Based on `mpris:length`.
    pub fn length(&self) -> Option<Duration> {
        use extensions::DurationExtensions;
        self.length_in_microseconds()
            .map(Duration::from_micros_ext)
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
}

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
        let metadata = Metadata::new("foo");
        assert_eq!(metadata.track_id(), Some("foo"));
    }

    #[test]
    fn it_supports_blank_metadata() {
        let metadata = Metadata::new_from_dbus(HashMap::new());
        assert_eq!(metadata.track_id(), None);
    }

    mod values {
        use super::*;

        fn metadata_with_value<S>(key: S, value: Variant<Box<RefArg + 'static>>) -> Metadata
        where
            S: Into<String>,
        {
            let mut values = HashMap::with_capacity(1);
            values.insert(key.into(), value);

            Metadata::new_from_dbus(values)
        }

        #[test]
        fn it_supports_string_values() {
            let data = String::from("The string value");
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), "The string value".into());

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_i64_values() {
            let data = 42i64;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::I64(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_i32() {
            let data = 42i32;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::I32(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_i16() {
            let data = 42i16;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::I16(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_u64() {
            let data = 42u64;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::U64(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_u32() {
            let data = 42u32;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::U32(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_u16() {
            let data = 42u16;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::U16(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_u8() {
            let data = 42u8;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::U8(42));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_f64_values() {
            let data = 42.0f64;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::F64(42.0));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_bool_values() {
            let data = true;
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::Bool(true));

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_supports_array_of_strings() {
            let data: Vec<String> = vec![String::from("foo"), String::from("bar")];
            let metadata = metadata_with_value("arr", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert(
                "arr".into(),
                Value::Array(vec![
                    Value::String(String::from("foo")),
                    Value::String(String::from("bar")),
                ]),
            );

            assert_eq!(metadata.values, expected_hash);
        }

        #[test]
        fn it_stores_unknown_types() {
            let data = dbus::Path::default();
            let metadata = metadata_with_value("foo", Variant(Box::new(data)));

            let mut expected_hash: HashMap<String, Value> = HashMap::new();
            expected_hash.insert("foo".into(), Value::Unsupported);

            assert_eq!(metadata.values, expected_hash);
        }
    }
}

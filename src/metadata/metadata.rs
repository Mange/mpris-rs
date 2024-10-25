use std::{collections::HashMap, iter::FusedIterator};

use super::{MetadataValue, TrackID};
use crate::{errors::InvalidMetadata, MprisDuration};

type RawMetadata = HashMap<String, MetadataValue>;

/// Macro that auto implements useful things for Metadata without needing to repeat the fields every time
/// while preserving documentation for the fields
///
/// The generated things are:
/// - `Metadata::new()`
/// - `Metadata::is_empty()`
/// - `Metadata::get_metadata_key()` which lets you get the key for a given field as a str
/// - `TryFrom<HashMap<String, MetadataValue>> for Metadata`
/// - `Metadata::from_raw_lossy()` which is similar to the above but wrong types just get discarded
/// - `From<Metadata> for HashMap<String, MetadataValue>`
/// - `IntoIterator for Metadata`
///
/// The macro expects a structure like this
/// ```text
/// #[derive(Debug, Clone, Default, PartialEq)]
/// struct Example {
///     "key" => key_field: Vec<String>,
///     "prefix:otherKey" => other_key_field: String,
///     [...]
///     "last_key" => last_key_field: f64,
///     field_name_for_hashmap,
/// }
/// ```
macro_rules! gen_metadata_struct {
    ($(#[$struct_meta:meta])*
     struct $name:ident {
        $($(#[$field_meta:meta])*
          $key:literal => $field:ident : $type:ty),*, $others_name:ident $(,)?
    }) => {

        // Creates the actual struct
        $(#[$struct_meta])*
        pub struct $name {
            $(
            $(#[$field_meta])*
            #[doc=""]
            #[doc=stringify!($key)]
            pub $field: Option<$type>
            ),*,
            pub $others_name: RawMetadata,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    $($field: None),*,
                    $others_name: HashMap::new(),
                }
            }

            pub fn is_empty(&self) -> bool {
                $(self.$field.is_none())&&*
                && self.$others_name.is_empty()
            }

            pub fn get_metadata_key(&self, field: &str) -> Option<&str> {
                match field {
                    $(stringify!($field) => Some($key)),*,
                    _ => None
                }
            }

            pub fn from_raw_lossy(mut raw: RawMetadata) -> Self {
                Self {
                    $($field: raw.remove($key).and_then(|v| <$type>::try_from(v).ok())),*,
                    $others_name: raw
                }
            }
        }

        impl IntoIterator for $name {
            type Item = (String, Option<MetadataValue>);
            type IntoIter = MetadataIter;

            fn into_iter(mut self) -> Self::IntoIter {
                // Turns the fields into Vec<&'static str, Option<MetadataValue>> with they key as the str
                let fields = vec![
                    $(($key, self.$field.take().map(MetadataValue::from))),*
                ];
                MetadataIter::new(fields, self.$others_name)
            }
        }

        // From<Metadata> for HashMap<String, MetadataValue>
        // Simply adds the fields to the HashMap using the specified key
        impl From<$name> for RawMetadata {
            fn from(mut value: $name) -> Self {
                let mut map = value.$others_name;
                $(if let Some(v) = value.$field.take() {
                    map.insert(String::from($key), MetadataValue::from(v));
                })*
                map
            }
        }

        // TryFrom<HashMap<String, MetadataValue>> for Metadata
        // Removes the given key from the HashMap tries to turn it into the target type.
        // Fails if MetadataValue is of the wrong type for the field or if mpris:trackid" is missing
        impl TryFrom<RawMetadata> for $name {
            type Error = InvalidMetadata;
            fn try_from(mut raw: RawMetadata) -> Result<Self, Self::Error> {
                if raw.is_empty() {
                    return Ok(Self::new());
                } else if !raw.contains_key("mpris:trackid") {
                    return Err(InvalidMetadata::from("metadata doesn't contain the mpris:trackid key"));
                }

                Ok(Self {
                    $(
                    $field: {
                        match raw.remove($key).map(<$type>::try_from) {
                            Some(v) => Some(v.map_err(|e| InvalidMetadata::from(format!("{} for {}", e.0, $key)))?),
                            None => None,
                        }
                    }
                    ),*,
                    $others_name: raw
                })
            }
        }
}}

gen_metadata_struct!(
    #[derive(Debug, Clone, Default, PartialEq)]
    #[cfg_attr(feature = "serde",
               derive(serde::Serialize, serde::Deserialize),
               serde(into = "RawMetadata", try_from = "RawMetadata")
               )
    ]
    struct Metadata {
        "xesam:albumArtist" => album_artists: Vec<String>,
        "xesam:album" => album_name: String,
        "mpris:artUrl" => art_url: String,
        "xesam:artist" => artists: Vec<String>,
        "xesam:audioBPM" => audio_bpm: u64,
        "xesam:autoRating" => auto_rating: f64,
        "xesam:comment" => comments: Vec<String>,
        "xesam:composer" => composers: Vec<String>,
        "xesam:contentCreated" => content_created: String,
        "xesam:discNumber" => disc_number: u64,
        "xesam:firstUsed" => first_used: String,
        "xesam:genre" => genres: Vec<String>,
        "xesam:lastUsed" => last_used: String,
        "mpris:length" => length: MprisDuration,
        "xesam:lyricist" => lyricists: Vec<String>,
        "xesam:asText" => lyrics: String,
        "xesam:title" => title: String,
        "mpris:trackid" => track_id: TrackID,
        "xesam:trackNumber" => track_number: u64,
        "xesam:url" => url: String,
        "xesam:useCount" => use_count: u64,
        "xesam:userRating" => user_rating: f64,
        others,
    }
);

#[derive(Debug)]
pub struct MetadataIter {
    values: std::vec::IntoIter<(&'static str, Option<MetadataValue>)>,
    map: std::collections::hash_map::IntoIter<String, MetadataValue>,
}

impl MetadataIter {
    fn new(fields: Vec<(&'static str, Option<MetadataValue>)>, map: RawMetadata) -> Self {
        Self {
            values: fields.into_iter(),
            map: map.into_iter(),
        }
    }
}

impl Iterator for MetadataIter {
    type Item = (String, Option<MetadataValue>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.values.next() {
            Some((k, v)) => Some((k.to_string(), v)),
            None => self.map.next().map(|(k, v)| (k, Some(v))),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let l = self.values.len() + self.map.len();
        (l, Some(l))
    }
}

impl ExactSizeIterator for MetadataIter {}
impl FusedIterator for MetadataIter {}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn empty_new_default() {
        let empty = Metadata {
            album_artists: None,
            album_name: None,
            art_url: None,
            artists: None,
            audio_bpm: None,
            auto_rating: None,
            comments: None,
            composers: None,
            content_created: None,
            disc_number: None,
            first_used: None,
            genres: None,
            last_used: None,
            length: None,
            lyricists: None,
            lyrics: None,
            title: None,
            track_id: None,
            track_number: None,
            url: None,
            use_count: None,
            user_rating: None,
            others: HashMap::new(),
        };
        assert_eq!(empty, Metadata::default());
        assert_eq!(empty, Metadata::new())
    }

    #[test]
    fn is_empty() {
        let mut m = Metadata::new();
        assert!(m.is_empty());

        let mut field = m.clone();
        field.disc_number = Some(0);
        assert!(!field.is_empty());

        m.others
            .insert("test".to_string(), MetadataValue::Boolean(false));
        assert!(!m.is_empty());

        m.others.remove("test");
        assert!(m.is_empty());
    }

    #[test]
    fn default_back_and_forth() {
        let original = Metadata::new();
        assert_eq!(
            Metadata::try_from(RawMetadata::from(original.clone())),
            Ok(original)
        )
    }

    #[test]
    fn try_from_raw() {
        let raw_metadata: RawMetadata = HashMap::from_iter([
            ("xesam:albumArtist".to_string(), vec![String::new()].into()),
            ("xesam:album".to_string(), String::new().into()),
            ("mpris:artUrl".to_string(), String::new().into()),
            ("xesam:artist".to_string(), vec![String::new()].into()),
            ("xesam:audioBPM".to_string(), 0_i64.into()),
            ("xesam:autoRating".to_string(), 0.0.into()),
            ("xesam:comment".to_string(), vec![String::new()].into()),
            ("xesam:composer".to_string(), vec![String::new()].into()),
            ("xesam:contentCreated".to_string(), String::new().into()),
            ("xesam:discNumber".to_string(), 0_i64.into()),
            ("xesam:firstUsed".to_string(), String::new().into()),
            ("xesam:genre".to_string(), vec![String::new()].into()),
            ("xesam:lastUsed".to_string(), String::new().into()),
            ("mpris:length".to_string(), MprisDuration::default().into()),
            ("xesam:lyricist".to_string(), vec![String::new()].into()),
            ("xesam:asText".to_string(), String::new().into()),
            ("xesam:title".to_string(), String::new().into()),
            ("mpris:trackid".to_string(), TrackID::no_track().into()),
            ("xesam:trackNumber".to_string(), 0_i64.into()),
            ("xesam:url".to_string(), String::new().into()),
            ("xesam:useCount".to_string(), 0_i64.into()),
            ("xesam:userRating".to_string(), 0.0.into()),
            ("other".to_string(), MetadataValue::Unsupported),
        ]);
        let meta = Metadata::try_from(raw_metadata);
        let manual_meta = Metadata {
            album_artists: Some(vec![String::new()]),
            album_name: Some(String::new()),
            art_url: Some(String::new()),
            artists: Some(vec![String::new()]),
            audio_bpm: Some(0),
            auto_rating: Some(0.0),
            comments: Some(vec![String::new()]),
            composers: Some(vec![String::new()]),
            content_created: Some(String::new()),
            disc_number: Some(0),
            first_used: Some(String::new()),
            genres: Some(vec![String::new()]),
            last_used: Some(String::new()),
            length: Some(MprisDuration::default()),
            lyricists: Some(vec![String::new()]),
            lyrics: Some(String::new()),
            title: Some(String::new()),
            track_id: Some(TrackID::no_track()),
            track_number: Some(0),
            url: Some(String::new()),
            use_count: Some(0),
            user_rating: Some(0.0),
            others: HashMap::from_iter([(String::from("other"), MetadataValue::Unsupported)]),
        };

        assert_eq!(meta, Ok(manual_meta));
    }

    #[test]
    fn try_from_raw_fail() {
        let mut map = HashMap::new();

        // Wrong type
        map.insert("xesam:autoRating".to_string(), true.into());
        let m = Metadata::try_from(map.clone());
        assert!(m.is_err());

        // Correct type but no TrackID
        map.insert("xesam:autoRating".to_string(), 0.0.into());
        let m = Metadata::try_from(map.clone());
        assert!(m.is_err());

        map.insert("mpris:trackid".to_string(), TrackID::no_track().into());
        let m = Metadata::try_from(map);
        assert!(m.is_ok());
    }

    #[test]
    fn equality() {
        let mut first = Metadata::new();
        first.auto_rating = Some(0.0);
        first.others.insert(String::from("test"), true.into());

        let mut second = Metadata::new();
        second.auto_rating = Some(0.0);
        assert_ne!(first, second.clone());

        second.others.insert(String::from("test"), true.into());
        assert_eq!(first, second);
    }
}

#[cfg(test)]
mod metadata_iterator_tests {
    use super::*;

    #[test]
    fn empty() {
        let iter = Metadata::new().into_iter();
        let (left, right) = iter.size_hint();
        assert_eq!(Some(left), right);
        assert_eq!(left, 22);

        for (_, v) in iter {
            assert!(v.is_none());
        }
    }
}

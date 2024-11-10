use std::{collections::HashMap, iter::FusedIterator};

use zbus::zvariant::OwnedValue;

use super::{MetadataValue, TrackID};
use crate::{errors::InvalidMetadata, MprisDuration};

/// HashMap returned from DBus
type DBusMetadata = HashMap<String, OwnedValue>;
type InnerRawMetadata = HashMap<String, MetadataValue>;

/// A struct that represents the raw version of [`Metadata`].
///
/// It's a simple wrapper around <code>[HashMap]<[String], [MetadataValue]></code>. It should act
/// like a [`HashMap`] but it can be easily converted into and from one using the [`From`] traits or
/// [`into_inner()`][Self::into_inner].
///
/// Can be obtained from [`Player::raw_metadata()`][crate::Player::raw_metadata].
#[derive(Clone, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct RawMetadata(InnerRawMetadata);

impl RawMetadata {
    /// Creates a new empty [`RawMetadata`].
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Turns [`RawMetadata`] into <code>[HashMap]<[String], [MetadataValue]></code>.
    pub fn into_inner(self) -> InnerRawMetadata {
        self.0
    }
}

impl std::ops::Deref for RawMetadata {
    type Target = InnerRawMetadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RawMetadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DBusMetadata> for RawMetadata {
    fn from(value: DBusMetadata) -> Self {
        Self(
            value
                .into_iter()
                .map(|(k, v)| (k, MetadataValue::from(v)))
                .collect(),
        )
    }
}

impl From<RawMetadata> for InnerRawMetadata {
    fn from(value: RawMetadata) -> Self {
        value.0
    }
}

impl From<InnerRawMetadata> for RawMetadata {
    fn from(value: InnerRawMetadata) -> Self {
        Self(value)
    }
}

impl FromIterator<(String, MetadataValue)> for RawMetadata {
    fn from_iter<T: IntoIterator<Item = (String, MetadataValue)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl IntoIterator for RawMetadata {
    type Item = (String, MetadataValue);

    type IntoIter = std::collections::hash_map::IntoIter<String, MetadataValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl PartialEq<InnerRawMetadata> for RawMetadata {
    fn eq(&self, other: &InnerRawMetadata) -> bool {
        &self.0 == other
    }
}

impl std::fmt::Debug for RawMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Macro that auto implements useful things for Metadata without needing to repeat the fields every time
/// while preserving documentation for the fields
///
/// The generated things are:
/// - `Metadata::new()`
/// - `Metadata::is_empty()`
/// - `Metadata::is_valid()'
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
            #[doc=concat!("The `", stringify!($key), "` field from the guidelines.")]
                pub $field: Option<$type>
            ),*,
            /// The rest of the metadata not specified in the guidelines.
            pub $others_name: RawMetadata,
        }

        impl $name {
            /// Creates a new empty [`Metadata`].
            ///
            /// Same as using `Metadata::default()`.
            pub fn new() -> Self {
                Self {
                    $($field: None),*,
                    $others_name: RawMetadata::new(),
                }
            }

            /// Checks if it contains any metadata.
            pub fn is_empty(&self) -> bool {
                $(self.$field.is_none())&&*
                && self.$others_name.is_empty()
            }

            /// Checks if it's valid.
            ///
            /// See [here][Self#validity] for the requirements.
            pub fn is_valid(&self) -> bool {
                if self.is_empty() {
                    true
                } else {
                    self.track_id.is_some()
                }
            }

            /// Gets the field name as specified in the [guide] for the given [`Metadata`] field.
            ///
            /// Returns [`None`] if such a [`Metadata`] field doesn't exist.
            /// ```
            /// use mpris::Metadata;
            ///
            /// assert_eq!(Metadata::get_metadata_key("lyrics"), Some("xesam:asText"));
            /// assert_eq!(Metadata::get_metadata_key("invalid_field"), None);
            /// ```
            /// [guide]: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
            pub fn get_metadata_key(field: &str) -> Option<&str> {
                match field {
                    $(stringify!($field) => Some($key)),*,
                    _ => None
                }
            }

            /// Lossily converts from [`RawMetadata`] into [`Metadata`]
            ///
            /// Similar to <code>[TryFrom]<[RawMetadata]></code> but the requirements mentioned
            /// [here][Self#validity] are not checked so it can't fail. If a field mentioned in the
            /// [guidelines] has the wrong type it will be turned into [`None`]. This is useful when
            /// used together with [`Player::raw_metadata()`][crate::Player::raw_metadata] in the
            /// case your player doesn't follow the [guidelines] and for example sends over some
            /// data in the wrong type.
            /// ```no_run
            /// use mpris::{Metadata, Mpris};
            ///
            /// async_std::task::block_on(async {
            ///     let mpris = Mpris::new().await.unwrap();
            ///     let player = mpris.find_first().await.unwrap().unwrap();
            ///     let meta = Metadata::from_raw_lossy(player.raw_metadata().await.unwrap());
            /// })
            /// ```
            ///
            /// ```
            /// use std::collections::HashMap;
            /// use mpris::{Metadata, metadata::RawMetadata};
            ///
            /// let mut raw_meta = RawMetadata::new();
            /// // Wrong type for title
            /// raw_meta.insert(String::from("xesam:title"), 0_u64.into());
            /// raw_meta.insert(String::from("xesam:comment"), String::from("Some comment").into());
            /// let wrong_meta = Metadata::from_raw_lossy(raw_meta);
            ///
            /// // Creates fine even though the field had the wrong type and track_id is not present
            /// assert!(!wrong_meta.is_valid());
            /// assert!(wrong_meta.track_id.is_none());
            /// // Wrong value got turned into None
            /// assert!(wrong_meta.title.is_none())
            /// ```
            ///
            /// [guidelines]: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
            pub fn from_raw_lossy(mut raw: RawMetadata) -> Self {
                Self {
                    $($field: raw.remove($key).and_then(|v| <$type>::try_from(v).ok())),*,
                    $others_name: raw
                }
            }
        }

        impl IntoIterator for $name {
            type Item = (String, Option<MetadataValue>);
            type IntoIter = MetadataIntoIter;

            fn into_iter(mut self) -> Self::IntoIter {
                // Turns the fields into Vec<&'static str, Option<MetadataValue>> with they key as the str
                let fields = vec![
                    $(($key, self.$field.take().map(MetadataValue::from))),*
                ];
                MetadataIntoIter::new(fields, self.$others_name)
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
    /// A struct that represents metadata for a track.
    ///
    /// It follows the [MPRIS v2 metadata guidelines][guide]. It can be obtained from
    /// [`Player::metadata()`][crate::Player::metadata] but it can also be created from any
    /// [`RawMetadata`] that meets the requirements mentioned below. The metadata fields included in
    /// the guidelines are assigned to struct fields for easier access and are type checked while
    /// all other metadata fields are held in the [`others`][Self::others] field as [`RawMetadata`].
    ///
    /// # Validity
    ///
    /// For [`Metadata`] to be valid it has to be empty or it needs to at least contain the
    /// `"mpris:trackid"` field ([`track_id`][Metadata::track_id]) which has to be a valid
    /// [`TrackID`]. All the other fields are optional but they need to be the right type.
    /// <code>[TryFrom]<[RawMetadata]></code> will fail if these requirements are not met. The
    /// [`others`][Self::others] field is not checked in any way.
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use mpris::{Metadata, TrackID, metadata::RawMetadata};
    ///
    /// let mut raw_meta = RawMetadata::new();
    ///
    /// // Empty is valid
    /// assert!(Metadata::try_from(raw_meta.clone()).is_ok());
    ///
    /// // Adding any fields without adding track_id will fail
    /// raw_meta.insert(String::from("some_field"), String::from("Some value").into());
    /// assert!(Metadata::try_from(raw_meta.clone()).is_err());
    ///
    /// // A valid track_id is present but a field from the guidelines has a wrong type
    /// raw_meta.insert(
    ///     String::from("mpris:trackid"),
    ///     TrackID::try_from("/valid/path").unwrap().into(),
    /// );
    /// raw_meta.insert(String::from("xesam:trackNumber"), String::new().into());
    /// assert!(Metadata::try_from(raw_meta.clone()).is_err());
    ///
    /// // If we remove the invalid type it will be valid
    /// raw_meta.remove("xesam:trackNumber");
    /// assert!(Metadata::try_from(raw_meta).is_ok());
    /// ```
    ///
    /// # Miscellaneous features
    ///
    /// - Can be turned into [`RawMetadata`] using <code>[Into]<[RawMetadata]></code>
    /// - Implements [`IntoIterator`], see [`MetadataIntoIter`] for details
    /// - Can be lossily converted from [`RawMetadata`] by using
    ///   [`from_raw_lossy()`][Self::from_raw_lossy]
    ///
    /// [guide]: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
    /// [object_path]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-marshaling-object-path
    #[derive(Debug, Clone, Default, PartialEq)]
    #[cfg_attr(
        feature = "serde",
        derive(serde::Serialize, serde::Deserialize),
        serde(into = "RawMetadata", try_from = "RawMetadata")
    )]
    struct Metadata {
        /// The album artist(s).
        "xesam:albumArtist" => album_artists: Vec<String>,
        /// The album name.
        "xesam:album" => album_name: String,
        /// The location of an image representing the track or album. Clients should not assume this
        /// will continue to exist when the media player stops giving out the URL.
        "mpris:artUrl" => art_url: String,
        /// The track artist(s).
        "xesam:artist" => artists: Vec<String>,
        /// The speed of the music, in beats per minute.
        "xesam:audioBPM" => audio_bpm: u64,
        /// An automatically-generated rating, based on things such as how often it has been played.
        /// This should be in the range 0.0 to 1.0.
        "xesam:autoRating" => auto_rating: f64,
        /// A (list of) freeform comment(s).
        "xesam:comment" => comments: Vec<String>,
        /// The composer(s) of the track.
        "xesam:composer" => composers: Vec<String>,
        /// When the track was created. Usually only the year component will be useful.
        "xesam:contentCreated" => content_created: String,
        /// The disc number on the album that this track is from.
        "xesam:discNumber" => disc_number: u64,
        /// When the track was first played.
        "xesam:firstUsed" => first_used: String,
        /// The genre(s) of the track.
        "xesam:genre" => genres: Vec<String>,
        /// When the track was last played.
        "xesam:lastUsed" => last_used: String,
        /// The duration of the track in microseconds.
        "mpris:length" => length: MprisDuration,
        /// The lyricist(s) of the track.
        "xesam:lyricist" => lyricists: Vec<String>,
        /// The track lyrics.
        "xesam:asText" => lyrics: String,
        /// The track title.
        "xesam:title" => title: String,
        /// A unique identity for this track within the context of an MPRIS object.
        "mpris:trackid" => track_id: TrackID,
        /// The track number on the album disc.
        "xesam:trackNumber" => track_number: u64,
        /// The location of the media file.
        "xesam:url" => url: String,
        /// The number of times the track has been played.
        "xesam:useCount" => use_count: u64,
        /// A user-specified rating. This should be in the range 0.0 to 1.0.
        "xesam:userRating" => user_rating: f64,
        others,
    }
);

/// [`Iterator`] over the fields of [`Metadata`].
///
/// Yields the field name as a [`String`] and <code>[Option]<[MetadataValue]></code> containing the
/// value if present. The [`RawMetadata`] from the [`others`][Metadata::others] field is also
/// included and values from it will always be <code>[Some]\([MetadataValue]\)</code>.
/// ```
/// use mpris::Metadata;
///
/// let meta = Metadata::new();
/// for (field, value) in meta {
///     // Do something   
/// }
/// ```
/// **Note**: the field names from this iterator are the actual field names of the [`Metadata`]
/// struct. If you want the field names from the [guidelines] you can use
/// [`get_metadata_key()`][Metadata::get_metadata_key] together with this iterator:
/// ```
/// use mpris::Metadata;
///
/// let meta = Metadata::new();
/// for (field, value) in meta {
///     let field_key = match Metadata::get_metadata_key(&field) {
///         Some(s) => s.to_string(),
///         None => field,
///     };
///     // Do something
/// }
/// ```
///
/// [guidelines]: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
#[derive(Debug)]
pub struct MetadataIntoIter {
    values: std::vec::IntoIter<(&'static str, Option<MetadataValue>)>,
    map: std::collections::hash_map::IntoIter<String, MetadataValue>,
}

impl MetadataIntoIter {
    fn new(fields: Vec<(&'static str, Option<MetadataValue>)>, map: RawMetadata) -> Self {
        Self {
            values: fields.into_iter(),
            map: map.into_iter(),
        }
    }
}

impl Iterator for MetadataIntoIter {
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

impl ExactSizeIterator for MetadataIntoIter {}
impl FusedIterator for MetadataIntoIter {}

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
            others: RawMetadata::new(),
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
    fn is_valid() {
        let mut meta = Metadata::new();
        assert!(meta.is_valid());
        meta.album_name = Some(String::from("Album Name"));
        assert!(!meta.is_valid());
        meta.track_id = Some(TrackID::no_track());
        assert!(meta.is_valid());
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
        let raw_metadata = RawMetadata::from_iter([
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
            others: RawMetadata::from_iter([(String::from("other"), MetadataValue::Unsupported)]),
        };

        assert_eq!(meta, Ok(manual_meta));
    }

    #[test]
    fn try_from_raw_fail() {
        let mut map = RawMetadata::new();

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

#[cfg(test)]
mod raw_metadata {
    use super::*;

    #[test]
    fn new_is_default() {
        let manual = RawMetadata(HashMap::new());
        assert_eq!(manual, RawMetadata::new());
        assert_eq!(manual, RawMetadata::default());
    }

    #[test]
    fn deref() {
        let mut meta = RawMetadata::new();
        assert!(meta.is_empty());
        meta.insert(String::from("Key"), MetadataValue::Unsupported);
        assert_eq!(meta["Key"], MetadataValue::Unsupported);
        assert!(!meta.is_empty());
        meta.clear();
        assert!(meta.is_empty())
    }

    #[test]
    fn from_hash_map() {
        let map: HashMap<String, OwnedValue> =
            HashMap::from_iter([(String::from("Some"), OwnedValue::from(0_u64))]);
        let meta = RawMetadata::from(map);
        assert_eq!(meta.get("Some"), Some(&MetadataValue::UnsignedInt(0)));
        assert_eq!(meta.get("Other"), None);
    }

    #[test]
    fn iter_and_eq() {
        let values = [
            (String::from("Bool"), MetadataValue::Boolean(false)),
            (String::from("Number"), MetadataValue::UnsignedInt(0)),
            (
                String::from("String"),
                MetadataValue::String(String::from("Value")),
            ),
        ];
        let meta = RawMetadata::from_iter(values.clone());
        let map = HashMap::from_iter(values);
        assert_eq!(meta, map);
    }
}

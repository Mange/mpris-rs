extern crate dbus;
use dbus::arg::{Variant, RefArg, cast};
use std::collections::HashMap;

/// This enum encodes possible error cases that could happen when loading metadata from a player.
#[derive(Fail, Debug)]
pub enum MetadataError {
    /// The `trackId` field could not be read from the player metadata.
    ///
    /// Non-conforming implementations of the MPRIS2 protocol might omit the required
    /// `trackId` field, which would then return this error.
    #[fail(display = "TrackId missing from metadata")]
    TrackIdMissing,

    /// Metadata reading failed due to an underlying D-Bus error.
    #[fail(display = "{}", _0)]
    DBusError(#[cause] super::DBusError),
}

impl From<dbus::Error> for MetadataError {
    fn from(error: dbus::Error) -> Self {
        MetadataError::DBusError(error.into())
    }
}

type Result<T> = ::std::result::Result<T, MetadataError>;

/// A structured representation of the `Player` metadata.
///
/// * [Read more about the MPRIS2 `Metadata_Map`
/// type.](https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Mapping:Metadata_Map)
/// * [Read MPRIS v2 metadata guidelines](https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/)
#[derive(Debug)]
pub struct Metadata {
    /// The track ID.
    ///
    /// Based on `mpris:trackId`
    /// > A unique identity for this track within the context of an MPRIS object.
    pub track_id: String,

    /// A list of artists of the album the track appears on.
    ///
    /// Based on `xesam:albumArtist`
    /// > The album artist(s).
    pub album_artists: Option<Vec<String>>,

    /// The name of the album the track appears on.
    ///
    /// Based on `xesam:album`
    /// > The album name.
    pub album_name: Option<String>,

    /// An URL to album art of the current track.
    ///
    /// Based on `mpris:artUrl`
    /// > The location of an image representing the track or album. Clients should not assume this
    /// > will continue to exist when the media player stops giving out the URL.
    pub art_url: Option<String>,

    /// A list of artists of the track.
    ///
    /// Based on `xesam:artist`
    /// > The track artist(s).
    pub artists: Option<Vec<String>>,

    /// Based on `xesam:autoRating`
    /// > An automatically-generated rating, based on things such as how often it has been played.
    /// > This should be in the range 0.0 to 1.0.
    pub auto_rating: Option<f64>,

    /// Based on `xesam:discNumber`
    /// > The disc number on the album that this track is from.
    pub disc_number: Option<i32>,

    /// The duration of the track, in microseconds
    ///
    /// Based on `mpris:length`
    /// > The duration of the track in microseconds.
    pub length_in_microseconds: Option<u64>,

    /// The name of the track.
    ///
    /// Based on `xesam:title`
    /// > The track title.
    pub title: Option<String>,

    /// The track number on the disc of the album the track appears on.
    ///
    /// Based on `xesam:trackNumber`
    /// > The track number on the album disc.
    pub track_number: Option<i32>,

    /// A URL to the media being played.
    ///
    /// Based on `xesam:url`
    /// > The location of the media file.
    pub url: Option<String>,

    /// Remaining metadata that has not been parsed into one of the other fields of the `Metadata`,
    /// if any.
    ///
    /// As an example, if the media player exposed `xesam:composer`, then you could read that
    /// String like this:
    ///
    /// ```rust
    /// # extern crate mpris;
    /// # extern crate dbus;
    /// # use mpris::Metadata;
    /// # fn main() {
    /// # let metadata = Metadata::new(String::from("1234"));
    /// use dbus::arg::RefArg;
    /// if let Some(name) = metadata.rest.get("xesam:composer").and_then(|v| v.as_str()) {
    ///     println!("Composed by: {}", name)
    /// }
    /// # }
    /// ```
    pub rest: HashMap<String, Variant<Box<RefArg>>>,
}

impl Metadata {
    /// Create a new `Metadata` struct with a given `track_id`.
    ///
    /// This is mostly useful for test fixtures and other places where you want to work with mock
    /// data.
    pub fn new(track_id: String) -> Self {
        let mut builder = MetadataBuilder::new();
        builder.track_id = Some(track_id);
        builder.finish().unwrap()
    }

    pub(crate) fn new_from_dbus(
        metadata: HashMap<String, Variant<Box<RefArg>>>,
    ) -> Result<Metadata> {
        MetadataBuilder::build_from_metadata(metadata)
    }
}

#[derive(Debug, Default)]
struct MetadataBuilder {
    track_id: Option<String>,

    album_artists: Option<Vec<String>>,
    album_name: Option<String>,
    art_url: Option<String>,
    artists: Option<Vec<String>>,
    auto_rating: Option<f64>,
    disc_number: Option<i32>,
    length_in_microseconds: Option<u64>,
    title: Option<String>,
    track_number: Option<i32>,
    url: Option<String>,

    rest: HashMap<String, Variant<Box<RefArg>>>,
}

fn cast_string_vec(value: &Variant<Box<RefArg>>) -> Option<Vec<String>> {
    value.0.as_iter().map(
        |arr| arr.flat_map(cast_string).collect(),
    )
}

fn cast_string<T: RefArg + ?Sized>(value: &T) -> Option<String> {
    value.as_str().map(String::from)
}

impl MetadataBuilder {
    fn build_from_metadata(metadata: HashMap<String, Variant<Box<RefArg>>>) -> Result<Metadata> {
        let mut builder = MetadataBuilder::new();

        for (key, value) in metadata {
            match key.as_ref() {
                "mpris:trackid" => builder.track_id = cast_string(&value),
                "mpris:length" => builder.length_in_microseconds = cast(&value.0).cloned(),
                "mpris:artUrl" => builder.art_url = cast_string(&value),
                "xesam:title" => builder.title = cast_string(&value),
                "xesam:albumArtist" => builder.album_artists = cast_string_vec(&value),
                "xesam:artist" => builder.artists = cast_string_vec(&value),
                "xesam:url" => builder.url = cast_string(&value),
                "xesam:album" => builder.album_name = cast_string(&value),
                "xesam:discNumber" => builder.disc_number = cast(&value.0).cloned(),
                "xesam:trackNumber" => builder.track_number = cast(&value.0).cloned(),
                "xesam:autoRating" => builder.auto_rating = cast(&value.0).cloned(),
                _ => builder.add_rest(key, value),
            };
        }

        builder.finish()
    }

    fn new() -> Self {
        MetadataBuilder::default()
    }

    fn add_rest(&mut self, key: String, value: Variant<Box<RefArg>>) {
        self.rest.insert(key, value);
    }

    fn finish(self) -> Result<Metadata> {
        match self.track_id {
            Some(track_id) => {
                Ok(Metadata {
                    track_id: track_id,

                    album_artists: self.album_artists,
                    album_name: self.album_name,
                    art_url: self.art_url,
                    artists: self.artists,
                    auto_rating: self.auto_rating,
                    disc_number: self.disc_number,
                    length_in_microseconds: self.length_in_microseconds,
                    title: self.title,
                    track_number: self.track_number,
                    url: self.url,

                    rest: self.rest,
                })
            }
            None => Err(MetadataError::TrackIdMissing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_creates_new_metadata() {
        let metadata = Metadata::new(String::from("foo"));
        assert_eq!(metadata.track_id, "foo");
    }
}

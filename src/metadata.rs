extern crate dbus;

use std::collections::HashMap;
use std::time::Duration;

use dbus::arg::{cast, RefArg, Variant};

use super::DBusError;

/// A structured representation of the `Player` metadata.
///
/// * [Read more about the MPRIS2 `Metadata_Map`
/// type.](https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Mapping:Metadata_Map)
/// * [Read MPRIS v2 metadata guidelines](https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/)
#[derive(Debug)]
pub struct Metadata {
    track_id: String,
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
    ) -> Result<Metadata, DBusError> {
        MetadataBuilder::build_from_metadata(metadata)
    }

    /// The track ID.
    ///
    /// Based on `mpris:trackId`
    /// > A unique identity for this track within the context of an MPRIS object.
    pub fn track_id(&self) -> &str {
        &self.track_id
    }

    /// A list of artists of the album the track appears on.
    ///
    /// Based on `xesam:albumArtist`
    /// > The album artist(s).
    pub fn album_artists(&self) -> Option<&Vec<String>> {
        self.album_artists.as_ref()
    }

    /// The name of the album the track appears on.
    ///
    /// Based on `xesam:album`
    /// > The album name.
    pub fn album_name(&self) -> Option<&str> {
        self.album_name.as_ref().map(String::as_ref)
    }

    /// An URL to album art of the current track.
    ///
    /// Based on `mpris:artUrl`
    /// > The location of an image representing the track or album. Clients should not assume this
    /// > will continue to exist when the media player stops giving out the URL.
    pub fn art_url(&self) -> Option<&str> {
        self.art_url.as_ref().map(String::as_ref)
    }

    /// A list of artists of the track.
    ///
    /// Based on `xesam:artist`
    /// > The track artist(s).
    pub fn artists(&self) -> Option<&Vec<String>> {
        self.artists.as_ref()
    }

    /// Based on `xesam:autoRating`
    /// > An automatically-generated rating, based on things such as how often it has been played.
    /// > This should be in the range 0.0 to 1.0.
    pub fn auto_rating(&self) -> Option<f64> {
        self.auto_rating
    }

    /// Based on `xesam:discNumber`
    /// > The disc number on the album that this track is from.
    pub fn disc_number(&self) -> Option<i32> {
        self.disc_number
    }

    /// The duration of the track, in microseconds
    ///
    /// Based on `mpris:length`
    /// > The duration of the track in microseconds.
    pub fn length_in_microseconds(&self) -> Option<u64> {
        self.length_in_microseconds
    }

    /// The duration of the track, as a `Duration`
    ///
    /// Based on `mpris:length`.
    pub fn length(&self) -> Option<Duration> {
        use extensions::DurationExtensions;
        self.length_in_microseconds
            .clone()
            .map(Duration::from_micros_ext)
    }

    /// The name of the track.
    ///
    /// Based on `xesam:title`
    /// > The track title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(String::as_str)
    }

    /// The track number on the disc of the album the track appears on.
    ///
    /// Based on `xesam:trackNumber`
    /// > The track number on the album disc.
    pub fn track_number(&self) -> Option<i32> {
        self.track_number
    }

    /// A URL to the media being played.
    ///
    /// Based on `xesam:url`
    /// > The location of the media file.
    pub fn url(&self) -> Option<&str> {
        self.url.as_ref().map(String::as_str)
    }

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
    /// if let Some(name) = metadata.rest().get("xesam:composer").and_then(|v| v.as_str()) {
    ///     println!("Composed by: {}", name)
    /// }
    /// # }
    /// ```
    pub fn rest(&self) -> &HashMap<String, Variant<Box<RefArg>>> {
        &self.rest
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
    value
        .0
        .as_iter()
        .map(|arr| arr.flat_map(cast_string).collect())
}

fn cast_string<T: RefArg + ?Sized>(value: &T) -> Option<String> {
    value.as_str().map(String::from)
}

impl MetadataBuilder {
    fn build_from_metadata(
        metadata: HashMap<String, Variant<Box<RefArg>>>,
    ) -> Result<Metadata, DBusError> {
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

    fn finish(self) -> Result<Metadata, DBusError> {
        match self.track_id {
            Some(track_id) => Ok(Metadata {
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
            }),
            None => Err(DBusError::new(
                "TrackId is missing from metadata; client is not conforming to MPRIS-2",
            )),
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

use std::collections::HashMap;

use dbus::MessageItem;

use prelude::*;

/// A structured representation of the `Player` metadata.
///
/// * [Read more about the MPRIS2 `Metadata_Map`
/// type.](https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Mapping:Metadata_Map)
/// * [Read MPRIS v2 metadata guidelines](https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/)
#[derive(Debug, Clone)]
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
    /// > The location of an image representing the track or album. Clients should not assume this will continue to exist when the media player stops giving out the URL.
    pub art_url: Option<String>,

    /// A list of artists of the track.
    ///
    /// Based on `xesam:artist`
    /// > The track artist(s).
    pub artists: Option<Vec<String>>,

    /// Based on `xesam:autoRating`
    /// > An automatically-generated rating, based on things such as how often it has been played. This should be in the range 0.0 to 1.0.
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
    /// # use dbus::MessageItem;
    /// # fn main() {
    /// # let metadata = Metadata::new(String::from("1234"));
    /// if let Some(&MessageItem::Str(ref name)) = metadata.rest.get("xesam:composer") {
    ///     println!("Composed by: {}", name)
    /// }
    /// # }
    /// ```
    pub rest: HashMap<String, MessageItem>,
}

impl Metadata {
    pub fn new(track_id: String) -> Self {
        let mut builder = MetadataBuilder::new();
        builder.track_id = Some(track_id);
        builder.finish().unwrap()
    }

    pub(crate) fn new_from_message_item(metadata: MessageItem) -> Result<Metadata> {
        MetadataBuilder::build_from_metadata(metadata)
    }
}

#[derive(Debug, Clone, Default)]
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

    rest: HashMap<String, MessageItem>,
}

impl MetadataBuilder {
    fn build_from_metadata(metadata: MessageItem) -> Result<Metadata> {
        let mut builder = MetadataBuilder::new();

        for (key, value) in metadata.as_dict_array("metadata")? {
            let key: Result<&str> = key.inner().map_err(
                |_| "Dictionary key was not a String".into(),
            );
            let key = key?;

            use dbus::MessageItem::*;
            match (key, value) {
                ("mpris:trackid", Str(track_id)) => builder.track_id = Some(track_id),
                ("mpris:length", UInt64(length)) => builder.length_in_microseconds = Some(length),
                ("mpris:artUrl", Str(art_url)) => builder.art_url = Some(art_url),
                ("xesam:title", Str(title)) => builder.title = Some(title),
                ("xesam:albumArtist", artists @ Array(_, _)) => {
                    builder.album_artists = Some(artists.as_string_array("xesam:albumArtist")?)
                }
                ("xesam:artist", artists @ Array(_, _)) => {
                    builder.artists = Some(artists.as_string_array("xesam:artist")?)
                }
                ("xesam:url", Str(url)) => builder.url = Some(url),
                ("xesam:album", Str(album)) => builder.album_name = Some(album),
                ("xesam:discNumber", Int32(disc_number)) => builder.disc_number = Some(disc_number),
                ("xesam:trackNumber", Int32(track_number)) => {
                    builder.track_number = Some(track_number)
                }
                ("xesam:autoRating", Double(auto_rating)) => {
                    builder.auto_rating = Some(auto_rating)
                }
                (key, value) => builder.add_rest(key, value),
            };
        }

        builder.finish()
    }

    fn new() -> Self {
        MetadataBuilder::default()
    }

    fn add_rest(&mut self, key: &str, value: MessageItem) {
        self.rest.insert(key.to_owned(), value);
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
            None => Err(ErrorKind::TrackIdMissing.into()),
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

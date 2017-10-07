use std::collections::HashMap;

use dbus::MessageItem;

use prelude::*;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub track_id: String,

    pub album_artists: Option<Vec<String>>,
    pub album_name: Option<String>,
    pub art_url: Option<String>,
    pub artists: Option<Vec<String>>,
    pub auto_rating: Option<f64>,
    pub disc_number: Option<i32>,
    pub length_in_microseconds: Option<u64>,
    pub title: Option<String>,
    pub track_number: Option<i32>,
    pub url: Option<String>,

    pub rest: HashMap<String, MessageItem>,
}

impl Metadata {
    pub fn new_from_message_item(metadata: MessageItem) -> Result<Metadata> {
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

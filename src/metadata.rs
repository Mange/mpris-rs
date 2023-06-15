use std::{collections::HashMap, ops::Deref};

use zbus::zvariant::{OwnedValue, Value};

#[derive(Debug)]
pub struct Metadata {
    pub raw: HashMap<String, OwnedValue>,

    pub track_id: Option<TrackID>,
}

fn build_metadata(raw: HashMap<String, OwnedValue>) -> Metadata {
    let track_id = raw
        .get("mpris:trackid")
        .and_then(|v| TrackID::try_from(v).ok());

    Metadata { raw, track_id }
}

impl From<HashMap<String, OwnedValue>> for Metadata {
    fn from(metadata: HashMap<String, OwnedValue>) -> Self {
        build_metadata(metadata)
    }
}

impl<'a> From<HashMap<String, Value<'a>>> for Metadata {
    fn from(metadata: HashMap<String, Value<'a>>) -> Self {
        let mut owned: HashMap<String, OwnedValue> = HashMap::with_capacity(metadata.len());

        for (k, v) in metadata.into_iter() {
            owned.insert(k.to_owned(), v.to_owned());
        }

        build_metadata(owned)
    }
}

impl<'a> From<&HashMap<String, Value<'a>>> for Metadata {
    fn from(metadata: &HashMap<String, Value<'a>>) -> Self {
        let mut owned: HashMap<String, OwnedValue> = HashMap::with_capacity(metadata.len());

        for (k, v) in metadata.iter() {
            owned.insert(k.to_owned(), v.to_owned());
        }

        build_metadata(owned)
    }
}

#[derive(Debug)]
pub struct TrackID(String);

impl TryFrom<&str> for TrackID {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() || value == "/org/mpris/MediaPlayer2/TrackList/NoTrack" {
            Err(())
        } else {
            Ok(TrackID(value.to_owned()))
        }
    }
}

impl TryFrom<OwnedValue> for TrackID {
    type Error = ();

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        match value.deref() {
            Value::Str(s) => s.as_str().try_into(),
            Value::ObjectPath(path) => path.as_str().try_into(),
            _ => Err(()),
        }
    }
}

impl TryFrom<&OwnedValue> for TrackID {
    type Error = ();

    fn try_from(value: &OwnedValue) -> Result<Self, Self::Error> {
        match value.deref() {
            Value::Str(s) => s.as_str().try_into(),
            Value::ObjectPath(path) => path.as_str().try_into(),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for TrackID {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Str(s) => s.as_str().try_into(),
            Value::ObjectPath(path) => path.as_str().try_into(),
            _ => Err(()),
        }
    }
}

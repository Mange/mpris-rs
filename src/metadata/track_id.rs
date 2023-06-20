use std::ops::Deref;

use zbus::zvariant::{OwnedValue, Value};

use super::MetadataValue;

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

impl TryFrom<MetadataValue> for TrackID {
    type Error = ();

    fn try_from(value: MetadataValue) -> Result<Self, Self::Error> {
        match value {
            MetadataValue::String(s) => s.as_str().try_into(),
            _ => Err(()),
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

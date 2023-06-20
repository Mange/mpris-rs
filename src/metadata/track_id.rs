use std::ops::Deref;

use zbus::zvariant::{OwnedValue, Value};

use super::MetadataValue;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
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

impl Deref for TrackID {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_tokens, Token};

    #[test]
    fn test_serialization() {
        let track_id = TrackID("/foo/bar".to_owned());
        assert_tokens(&track_id, &[Token::String("/foo/bar")]);
    }

    #[test]
    fn test_deserialization() {
        let track_id = TrackID("/foo/bar".to_owned());
        assert_de_tokens(&track_id, &[Token::String("/foo/bar")]);
    }
}

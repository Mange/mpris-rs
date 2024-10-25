use std::ops::Deref;

use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use super::MetadataValue;
use crate::errors::InvalidTrackID;

const NO_TRACK: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
pub struct TrackID(OwnedObjectPath);

impl TrackID {
    pub fn new(id: String) -> Result<Self, InvalidTrackID> {
        Self::try_from(id)
    }

    pub fn no_track() -> Self {
        // We know it's a valid path so it's safe to skip the check
        Self(OwnedObjectPath::from(
            ObjectPath::from_static_str_unchecked(NO_TRACK),
        ))
    }

    pub fn is_no_track(&self) -> bool {
        self.as_str() == NO_TRACK
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_object_path(&self) -> ObjectPath {
        self.0.as_ref()
    }
}

impl AsRef<OwnedObjectPath> for TrackID {
    fn as_ref(&self) -> &OwnedObjectPath {
        &self.0
    }
}

fn check_start<T>(s: T) -> Result<T, InvalidTrackID>
where
    T: Deref<Target = str>,
{
    if s.starts_with("/org/mpris") && s.deref() != NO_TRACK {
        Err(InvalidTrackID::from(
            "TrackID can't start with \"/org/mpris\"",
        ))
    } else {
        Ok(s)
    }
}

impl Ord for TrackID {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for TrackID {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<&str> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match OwnedObjectPath::try_from(check_start(value)?) {
            Ok(o) => Ok(Self(o)),
            Err(e) => {
                if let zbus::zvariant::Error::Message(s) = e {
                    Err(InvalidTrackID(s))
                } else {
                    // ObjectValue only creates Serde errors which get converted into
                    // zbus::zvariant::Error::Message
                    unreachable!("ObjectPath should only return Message errors")
                }
            }
        }
    }
}

impl TryFrom<String> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match OwnedObjectPath::try_from(check_start(value)?) {
            Ok(o) => Ok(Self(o)),
            Err(e) => {
                if let zbus::zvariant::Error::Message(s) = e {
                    Err(InvalidTrackID(s))
                } else {
                    // ObjectValue only creates Serde errors which get converted into
                    // zbus::zvariant::Error::Message
                    unreachable!("ObjectPath should only return Message errors")
                }
            }
        }
    }
}

impl TryFrom<MetadataValue> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: MetadataValue) -> Result<Self, Self::Error> {
        match value {
            MetadataValue::String(s) => s.try_into(),
            MetadataValue::Strings(mut s) if s.len() == 1 => {
                s.pop().expect("length should be 1").try_into()
            }
            _ => Err(InvalidTrackID(String::from("not a string"))),
        }
    }
}

impl TryFrom<OwnedValue> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        Self::try_from(Value::from(value))
    }
}

impl TryFrom<Value<'_>> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Str(s) => Self::try_from(s.as_str()),
            Value::ObjectPath(path) => Self::try_from(path),
            _ => Err(InvalidTrackID::from("not a String or ObjectPath")),
        }
    }
}

impl TryFrom<OwnedObjectPath> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: OwnedObjectPath) -> Result<Self, Self::Error> {
        match check_start(value.as_str()) {
            Ok(_) => Ok(Self(value)),
            Err(e) => Err(e),
        }
    }
}

impl TryFrom<ObjectPath<'_>> for TrackID {
    type Error = InvalidTrackID;

    fn try_from(value: ObjectPath) -> Result<Self, Self::Error> {
        Ok(Self(check_start(value)?.into()))
    }
}

impl Deref for TrackID {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TrackID> for ObjectPath<'_> {
    fn from(value: TrackID) -> Self {
        value.0.into_inner()
    }
}

impl From<TrackID> for OwnedObjectPath {
    fn from(value: TrackID) -> Self {
        value.0
    }
}

impl From<TrackID> for String {
    fn from(value: TrackID) -> Self {
        value.0.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_track() {
        let track = TrackID::no_track();
        let manual = TrackID(OwnedObjectPath::from(
            ObjectPath::from_static_str_unchecked(NO_TRACK),
        ));

        assert!(track.is_no_track());
        assert!(manual.is_no_track());
        assert_eq!(track, manual);
    }

    #[test]
    fn check_start_test() {
        assert!(check_start("/some/path").is_ok());
        assert!(check_start("A").is_ok());
        assert!(check_start("").is_ok());
        assert!(check_start("/org/mpris").is_err());
        assert!(check_start("/org/mpris/more/path").is_err());
        assert!(check_start(NO_TRACK).is_ok());
    }

    #[test]
    fn valid_track_id() {
        assert_eq!(
            TrackID::try_from("/"),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/")
            )))
        );
        assert_eq!(
            TrackID::try_from("/some/path"),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/some/path")
            )))
        );

        assert_eq!(
            TrackID::try_from("/".to_string()),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/")
            )))
        );
        assert_eq!(
            TrackID::try_from("/some/path".to_string()),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/some/path")
            )))
        );
    }

    #[test]
    fn invalid_track_id() {
        assert!(TrackID::try_from("").is_err());
        assert!(TrackID::try_from("//some/path").is_err());
        assert!(TrackID::try_from("/some.path").is_err());
        assert!(TrackID::try_from("path").is_err());
        assert!(TrackID::try_from("/org/mpris").is_err());

        assert!(TrackID::try_from("".to_string()).is_err());
        assert!(TrackID::try_from("//some/path".to_string()).is_err());
        assert!(TrackID::try_from("/some.path".to_string()).is_err());
        assert!(TrackID::try_from("path".to_string()).is_err());
        assert!(TrackID::try_from("/org/mpris".to_string()).is_err());
    }

    #[test]
    fn from_object_path() {
        assert_eq!(
            TrackID::try_from(ObjectPath::from_str_unchecked("/valid/path")),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/valid/path")
            )))
        );
        assert_eq!(
            TrackID::try_from(OwnedObjectPath::from(ObjectPath::from_str_unchecked(
                "/valid/path"
            ))),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/valid/path")
            )))
        );

        assert!(TrackID::try_from(ObjectPath::from_str_unchecked("/org/mpris")).is_err());
        assert!(
            TrackID::try_from(OwnedObjectPath::from(ObjectPath::from_str_unchecked(
                "/org/mpris"
            )))
            .is_err()
        );
    }

    #[test]
    fn from_metadata_value() {
        assert!(TrackID::try_from(MetadataValue::Boolean(true)).is_err());
        assert!(TrackID::try_from(MetadataValue::Float(0.0)).is_err());
        assert!(TrackID::try_from(MetadataValue::SignedInt(0)).is_err());
        assert!(TrackID::try_from(MetadataValue::UnsignedInt(0)).is_err());
        assert_eq!(
            TrackID::try_from(MetadataValue::String(String::from("/valid/path"))),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/valid/path")
            )))
        );
        assert!(TrackID::try_from(MetadataValue::Strings(vec![])).is_err());
        assert_eq!(
            TrackID::try_from(MetadataValue::Strings(vec![String::from("/valid/path")])),
            Ok(TrackID(OwnedObjectPath::from(
                ObjectPath::from_str_unchecked("/valid/path")
            )))
        );
        assert!(
            TrackID::try_from(MetadataValue::Strings(vec![String::from("/valid/path"); 2]))
                .is_err()
        );
        assert!(TrackID::try_from(MetadataValue::Unsupported).is_err());
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

    #[test]
    fn test_serialization() {
        let track_id = TrackID::try_from("/foo/bar").unwrap();
        assert_ser_tokens(&track_id, &[Token::Str("/foo/bar")]);
    }

    #[test]
    fn test_deserialization() {
        let track_id = TrackID::try_from("/foo/bar").unwrap();
        assert_de_tokens(&track_id, &[Token::Str("/foo/bar")]);
    }
}

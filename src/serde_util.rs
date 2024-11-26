use serde::{de::IgnoredAny, Deserialize, Deserializer, Serializer};
use zbus::zvariant::OwnedObjectPath;

/// Serializes OwnedObjectPath into simple string
pub(crate) fn serialize_owned_object_path<S>(
    object: &OwnedObjectPath,
    ser: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(object.as_str())
}

/// Takes anything and returns a unit
pub(crate) fn deser_no_fail<'de, D>(d: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    IgnoredAny::deserialize(d).map(|_| ())
}

/// Deals with Option<String>
pub(crate) mod option_string {
    use super::*;

    pub(crate) fn serialize<S>(object: &Option<String>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ser.serialize_str(match object {
            Some(s) => s,
            None => "",
        })
    }

    pub(crate) fn deserialize<'de, D>(deser: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deser)?;
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }
}

#[cfg(feature = "serde")]
use serde::Serializer;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

use crate::{InvalidPlaylist, InvalidPlaylistOrdering};

/// A data structure describing a playlist.
///
/// It represents a [Playlist][playlist] type from the [MediaPlayer2.Playlists][interface] interface.
/// It contains:
/// - the unique id of the playlist: a [valid D-Bus object path][object_path] which, unlike
///   [`TrackID`][crate::TrackID], is not tied to the player's current track list and should
///   stay the same even if the playlist gets edited
/// - the name of the playlist
/// - an optional icon url
///
/// It can be obtain from [`Player::active_playlist()`][crate::Player::active_playlist] and
/// [`Player::get_playlists()`][crate::Player::get_playlists].
///
/// **Note**: currently the name and icon url will not get updated if they get changed. If they
/// need to be up to date you should fetch the playlists again.
///
/// [interface]: https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html
/// [playlist]: https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html#Struct:Playlist
/// [object_path]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-marshaling-object-path
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Playlist {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_owned_object_path")
    )]
    /// Unique playlist identifier
    id: OwnedObjectPath,
    name: String,
    #[cfg_attr(feature = "serde", serde(default))]
    icon: Option<String>,
}

impl Playlist {
    /// Tries to create a new [`Playlist`].
    ///
    /// The MPRIS spec does not provide a way to create new playlists so you can't use this to
    /// create a new one but creating [`Playlist`] manually is useful for example when you save the
    /// id to a file and use that to quickly change to a playlist without the need to fetch them
    /// first.
    ///
    /// Will return [`Err`] if the given `id` is not a valid Object Path. See the [struct
    /// documentation for details][Playlist].
    ///
    /// **Note**: the icon [`String`] should be a valid url but it is not actually checked
    ///
    /// See also [`new_from_object_path()`][Self::new_from_object_path]
    pub fn new(id: String, name: String, icon: Option<String>) -> Result<Self, InvalidPlaylist> {
        match OwnedObjectPath::try_from(id) {
            Ok(o) => Ok(Self { id: o, name, icon }),
            Err(e) => Err(InvalidPlaylist::from(e.to_string())),
        }
    }
    /// Creates a new [`Playlist`]
    ///
    /// Almost the same as [`new()`][Self::new] but uses a [`OwnedObjectPath`] instead of a
    /// [`String`] so it can't fail.
    pub fn new_from_object_path(id: OwnedObjectPath, name: String, icon: Option<String>) -> Self {
        Self { id, name, icon }
    }

    /// Gets the name of the playlist
    ///
    /// **Note**: as mentioned in the struct documentation this value might not be correct if the
    /// player changed the name of this playlist.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Gets the icon url if present
    ///
    /// **Note**: as mentioned in the struct documentation this value might not be correct if the
    /// player changed the icon of this playlist.
    pub fn get_icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    /// Gets the `id` as a borrowed [`ObjectPath`]
    pub fn get_id(&self) -> ObjectPath<'_> {
        self.id.as_ref()
    }

    /// Gets the `id` as a &[`str`]
    pub fn get_id_as_str(&self) -> &str {
        self.id.as_str()
    }
}

impl std::fmt::Debug for Playlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playlist")
            .field("id", &self.get_id_as_str())
            .field("name", &self.name)
            .field("icon", &self.icon)
            .finish()
    }
}

#[cfg(feature = "serde")]
pub(crate) fn serialize_owned_object_path<S>(
    object: &OwnedObjectPath,
    ser: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(object.as_str())
}

impl From<(OwnedObjectPath, String, String)> for Playlist {
    fn from(value: (OwnedObjectPath, String, String)) -> Self {
        let icon = if value.2.is_empty() {
            None
        } else {
            Some(value.2)
        };
        Self {
            id: value.0,
            name: value.1,
            icon,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Specifies the ordering of returned playlists.
pub enum PlaylistOrdering {
    /// Alphabetical ordering by name, ascending.
    Alphabetical, /* Alphabetical */

    /// Ordering by creation date, oldest first.
    CreationDate, /* Created */

    /// Ordering by last modified date, oldest first.
    ModifiedDate, /* Modified */

    ///Ordering by date of last playback, oldest first.
    LastPlayDate, /* Played */

    /// A user-defined ordering.
    ///
    /// Some media players may allow users to order playlists as they wish. This ordering allows playlists to be retreived in that order.
    UserDefined, /* User */
}

impl PlaylistOrdering {
    /// Returns the string value that's used on the D-Bus.
    ///
    /// See [`as_str()`][Self::as_str()] if you want the name of the enum variant.
    pub fn as_str_value(&self) -> &str {
        match self {
            PlaylistOrdering::Alphabetical => "Alphabetical",
            PlaylistOrdering::CreationDate => "Created",
            PlaylistOrdering::ModifiedDate => "Modified",
            PlaylistOrdering::LastPlayDate => "Played",
            PlaylistOrdering::UserDefined => "User",
        }
    }

    /// Returns the name of the enum variant as a <code>&[str]</code>
    ///
    /// See [`as_str_value()`][Self::as_str_value] if you want the actual D-Bus value.
    pub fn as_str(&self) -> &str {
        match self {
            PlaylistOrdering::Alphabetical => "Alphabetical",
            PlaylistOrdering::CreationDate => "CreationDate",
            PlaylistOrdering::ModifiedDate => "ModifiedDate",
            PlaylistOrdering::LastPlayDate => "LastPlayDate",
            PlaylistOrdering::UserDefined => "UserDefined",
        }
    }
}

impl std::fmt::Display for PlaylistOrdering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PlaylistOrdering {
    type Err = InvalidPlaylistOrdering;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Alphabetical" => Ok(Self::Alphabetical),
            "Created" => Ok(Self::CreationDate),
            "Modified" => Ok(Self::ModifiedDate),
            "Played" => Ok(Self::LastPlayDate),
            "User" => Ok(Self::UserDefined),
            _ => Err(InvalidPlaylistOrdering::from(
                r#"expected "Alphabetical", "Created", "Modified", "Played" or "User""#,
            )),
        }
    }
}

#[cfg(test)]
mod playlist_ordering_tests {
    use super::*;

    #[test]
    fn parsing() {
        assert_eq!("Alphabetical".parse(), Ok(PlaylistOrdering::Alphabetical));
        assert_eq!("Created".parse(), Ok(PlaylistOrdering::CreationDate));
        assert_eq!("Modified".parse(), Ok(PlaylistOrdering::ModifiedDate));
        assert_eq!("Played".parse(), Ok(PlaylistOrdering::LastPlayDate));
        assert_eq!("User".parse(), Ok(PlaylistOrdering::UserDefined));

        assert!("alphabetical".parse::<PlaylistOrdering>().is_err());
        assert!("created".parse::<PlaylistOrdering>().is_err());
        assert!("modified".parse::<PlaylistOrdering>().is_err());
        assert!("played".parse::<PlaylistOrdering>().is_err());
        assert!("user".parse::<PlaylistOrdering>().is_err());
        assert!("wrong".parse::<PlaylistOrdering>().is_err());
        assert!("".parse::<PlaylistOrdering>().is_err())
    }

    #[test]
    fn as_str() {
        assert_eq!(
            PlaylistOrdering::Alphabetical.as_str_value(),
            "Alphabetical"
        );
        assert_eq!(PlaylistOrdering::CreationDate.as_str_value(), "Created");
        assert_eq!(PlaylistOrdering::ModifiedDate.as_str_value(), "Modified");
        assert_eq!(PlaylistOrdering::LastPlayDate.as_str_value(), "Played");
        assert_eq!(PlaylistOrdering::UserDefined.as_str_value(), "User");
    }

    #[test]
    fn display() {
        assert_eq!(&PlaylistOrdering::Alphabetical.to_string(), "Alphabetical");
        assert_eq!(&PlaylistOrdering::CreationDate.to_string(), "CreationDate");
        assert_eq!(&PlaylistOrdering::LastPlayDate.to_string(), "LastPlayDate");
        assert_eq!(&PlaylistOrdering::UserDefined.to_string(), "UserDefined");
    }
}

#[cfg(test)]
mod playlist_tests {
    use super::*;

    #[test]
    fn new() {
        let manual = Playlist {
            id: ObjectPath::from_string_unchecked(String::from("/valid/path")).into(),
            name: String::from("TestName"),
            icon: Some(String::from("TestIcon")),
        };
        let new = Playlist::new(
            String::from("/valid/path"),
            String::from("TestName"),
            Some(String::from("TestIcon")),
        );
        assert_eq!(new, Ok(manual));
    }

    #[test]
    fn gets() {
        let mut new = Playlist::new_from_object_path(
            ObjectPath::from_string_unchecked(String::from("/valid/path")).into(),
            String::from("TestName"),
            Some(String::from("TestIcon")),
        );
        assert_eq!(new.get_name(), "TestName");
        assert_eq!(new.get_icon(), Some("TestIcon"));
        assert_eq!(new.get_id(), ObjectPath::from_str_unchecked("/valid/path"));
        assert_eq!(new.get_id_as_str(), "/valid/path");

        new.icon = None;
        assert_eq!(new.get_icon(), None);
    }
}

#[cfg(all(test, feature = "serde"))]
mod playlist_serde_tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_de_tokens_error, assert_tokens, Token};

    #[test]
    fn serialization() {
        let mut playlist = Playlist::new_from_object_path(
            ObjectPath::from_string_unchecked(String::from("/valid/path")).into(),
            String::from("TestName"),
            Some(String::from("TestIcon")),
        );
        assert_tokens(
            &playlist,
            &[
                Token::Struct {
                    name: "Playlist",
                    len: 3,
                },
                Token::Str("id"),
                Token::String("/valid/path"),
                Token::Str("name"),
                Token::String("TestName"),
                Token::Str("icon"),
                Token::Some,
                Token::String("TestIcon"),
                Token::StructEnd,
            ],
        );

        playlist.icon = None;
        assert_tokens(
            &playlist,
            &[
                Token::Struct {
                    name: "Playlist",
                    len: 3,
                },
                Token::Str("id"),
                Token::String("/valid/path"),
                Token::Str("name"),
                Token::String("TestName"),
                Token::Str("icon"),
                Token::None,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn deser_default() {
        let playlist = Playlist::new_from_object_path(
            ObjectPath::from_str_unchecked("/valid/path").into(),
            String::from("TestName"),
            None,
        );
        assert_de_tokens(
            &playlist,
            &[
                Token::Struct {
                    name: "Playlist",
                    len: 3,
                },
                Token::Str("id"),
                Token::String("/valid/path"),
                Token::Str("name"),
                Token::String("TestName"),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn deser_invalid_path() {
        assert_de_tokens_error::<Playlist>(
            &[
                Token::Struct {
                    name: "Playlist",
                    len: 3,
                },
                Token::Str("id"),
                Token::String("invalid/path"),
            ],
            "invalid value: character `i`, expected /",
        );
    }
}

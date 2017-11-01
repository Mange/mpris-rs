use dbus::{Connection, BusName, Props, Message};

use prelude::*;
use metadata::Metadata;
use super::PlaybackStatus;

// TODO: Make this a config on the Player instead.
const DEFAULT_TIMEOUT: i32 = 500; // milliseconds

pub(crate) const MPRIS2_PREFIX: &'static str = "org.mpris.MediaPlayer2.";
pub(crate) const MPRIS2_INTERFACE: &'static str = "org.mpris.MediaPlayer2";
pub(crate) const PLAYER_INTERFACE: &'static str = "org.mpris.MediaPlayer2.Player";
pub(crate) const MPRIS2_PATH: &'static str = "/org/mpris/MediaPlayer2";

/// A MPRIS-compatible player.
///
/// You can query this player about the currently playing media, or control it.
///
/// The `Player` is valid for the `'conn` (DBUS "connection") lifetime.
///
/// **See:** [MPRIS2 MediaPlayer2.Player Specification][spec]
/// [spec]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html
pub struct Player<'conn> {
    connection: &'conn Connection,
    bus_name: BusName<'conn>,
    identity: String,
    player_props: Props<'conn>,
}

impl<'conn> Player<'conn> {
    /// Create a new `Player` using a DBUS connection and a bus name.
    ///
    /// If no player is running on this bus name an `Err` will be returned.
    pub fn new<B>(connection: &'conn Connection, bus_name: B) -> Result<Player<'conn>>
    where
        B: Into<BusName<'conn>>,
    {
        let bus_name = bus_name.into();

        let parent_props = Props::new(
            connection,
            bus_name.clone(),
            MPRIS2_PATH,
            MPRIS2_INTERFACE,
            DEFAULT_TIMEOUT,
        );

        let player_props = Props::new(
            connection,
            bus_name.clone(),
            MPRIS2_PATH,
            PLAYER_INTERFACE,
            DEFAULT_TIMEOUT,
        );

        let identity = parent_props
            .get("Identity")
            .map_err(|e| e.into())
            .and_then(|v| v.as_string("Identity"))?;

        Ok(Player {
            connection: connection,
            bus_name: bus_name,
            identity: identity,
            player_props: player_props,
        })
    }

    /// Returns the player's DBUS bus name.
    pub fn bus_name(&self) -> &str {
        &self.bus_name
    }

    /// Returns the player's MPRIS `Identity`.
    ///
    /// This is usually the application's name, like `Spotify`.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Query the player for current metadata.
    ///
    /// See `Metadata` for more information about what is included here.
    pub fn get_metadata(&self) -> Result<Metadata> {
        self.player_props
            .get("Metadata")
            .map_err(|e| e.into())
            .and_then(Metadata::new_from_message_item)
    }

    /// Send a `PlayPause` signal to the player.
    ///
    /// See: [MPRIS2 specification about `PlayPause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause)
    pub fn play_pause(&self) -> Result<()> {
        let message = self.player_message("PlayPause");
        let _ = self.connection.send_with_reply_and_block(
            message,
            DEFAULT_TIMEOUT,
        )?;
        Ok(())
    }

    /// Send a `Play` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Play`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Play)
    pub fn play(&self) -> Result<()> {
        self.send_player_void_message("Play")
    }

    /// Send a `Pause` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Pause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Pause)
    pub fn pause(&self) -> Result<()> {
        self.send_player_void_message("Pause")
    }

    /// Send a `Stop` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Stop`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop)
    pub fn stop(&self) -> Result<()> {
        self.send_player_void_message("Stop")
    }

    /// Send a `Next` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Next`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Next)
    pub fn next(&self) -> Result<()> {
        self.send_player_void_message("Next")
    }

    /// Send a `Previous` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Previous`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Previous)
    pub fn previous(&self) -> Result<()> {
        self.send_player_void_message("Previous")
    }

    /// Sends a `PlayPause` signal to the player, if the player indicates that it can pause.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `PlayPause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause)
    pub fn checked_play_pause(&self) -> Result<bool> {
        if self.can_pause()? {
            self.play_pause().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Sends a `Play` signal to the player, if the player indicates that it can play.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Play`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Play)
    pub fn checked_play(&self) -> Result<bool> {
        if self.can_play()? {
            self.play().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Sends a `Pause` signal to the player, if the player indicates that it can pause.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Pause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Pause)
    pub fn checked_pause(&self) -> Result<bool> {
        if self.can_pause()? {
            self.pause().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Sends a `Stop` signal to the player, if the player indicates that it can stop.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Stop`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop)
    pub fn checked_stop(&self) -> Result<bool> {
        if self.can_stop()? {
            self.stop().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Sends a `Next` signal to the player, if the player indicates that it can go to the next
    /// media.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Next`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Next)
    pub fn checked_next(&self) -> Result<bool> {
        if self.can_go_next()? {
            self.next().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Sends a `Previous` signal to the player, if the player indicates that it can go to a
    /// previous media.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Previous`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Previous)
    pub fn checked_previous(&self) -> Result<bool> {
        if self.can_go_previous()? {
            self.previous().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Queries the player to see if it can be controlled or not.
    ///
    /// See: [MPRIS2 specification about `CanControl`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl)
    pub fn can_control(&self) -> Result<bool> {
        self.player_bool_property("CanControl")
    }

    /// Queries the player to see if it can go to next or not.
    ///
    /// See: [MPRIS2 specification about `CanGoNext`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoNext)
    pub fn can_go_next(&self) -> Result<bool> {
        self.player_bool_property("CanGoNext")
    }

    /// Queries the player to see if it can go to previous or not.
    ///
    /// See: [MPRIS2 specification about `CanGoPrevious`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoPrevious)
    pub fn can_go_previous(&self) -> Result<bool> {
        self.player_bool_property("CanGoPrevious")
    }

    /// Queries the player to see if it can pause.
    ///
    /// See: [MPRIS2 specification about `CanPause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPause)
    pub fn can_pause(&self) -> Result<bool> {
        self.player_bool_property("CanPause")
    }

    /// Queries the player to see if it can play.
    ///
    /// See: [MPRIS2 specification about `CanPlay`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPlay)
    pub fn can_play(&self) -> Result<bool> {
        self.player_bool_property("CanPlay")
    }

    /// Queries the player to see if it can seek within the media.
    ///
    /// See: [MPRIS2 specification about `CanSeek`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanSeek)
    pub fn can_seek(&self) -> Result<bool> {
        self.player_bool_property("CanSeek")
    }

    /// Queries the player to see if it can stop.
    ///
    /// MPRIS2 defines [the `Stop` message to only work when the player can be controlled][stop], so that
    /// is the property used for this method.
    ///
    /// See: [MPRIS2 specification about `CanControl`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl)
    /// [stop]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop
    pub fn can_stop(&self) -> Result<bool> {
        self.can_control()
    }

    /// Query the player for current playback status.
    pub fn get_playback_status(&self) -> Result<PlaybackStatus> {
        let raw = self.player_props
            .get("PlaybackStatus")
            .map_err(|e| e.into())
            .and_then(|v| v.as_string("PlaybackStatus"))?;

        match raw.as_ref() {
            "Playing" => Ok(PlaybackStatus::Playing),
            "Paused" => Ok(PlaybackStatus::Paused),
            "Stopped" => Ok(PlaybackStatus::Stopped),
            other => Err(format!("Not a valid PlaybackStatus: {}", other).into()),
        }
    }

    fn player_message(&self, member_name: &'static str) -> Message {
        // Unwrap result as it should never panic:
        // 1. self.bus_name must be valid as it's been used before to initialize Player instance.
        // 2. The strings for the path and the interface are valid identifiers.
        // 3. The member name will always be a hard-coded string that should be verified as valid
        //    identifiers. Making it <'static> further helps to reinforce that the method name
        //    should be in the source code and not generated at runtime.
        Message::new_method_call(
            self.bus_name.clone(),
            MPRIS2_PATH,
            PLAYER_INTERFACE,
            member_name,
        ).unwrap()
    }

    fn send_player_void_message(&self, member_name: &'static str) -> Result<()> {
        let message = self.player_message(member_name);
        let _ = self.connection.send_with_reply_and_block(
            message,
            DEFAULT_TIMEOUT,
        )?;
        Ok(())
    }

    fn player_bool_property(&self, property_name: &'static str) -> Result<bool> {
        self.player_props
            .get(property_name)
            .map_err(|e| e.into())
            .and_then(|v| v.as_bool(property_name))
    }
}

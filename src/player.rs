use dbus::{Connection, BusName, Props, Message};

use prelude::*;
use metadata::Metadata;
use super::PlaybackStatus;

const DEFAULT_TIMEOUT: i32 = 500; // milliseconds

pub struct Player<'conn> {
    connection: &'conn Connection,
    bus_name: BusName<'conn>,
    identity: String,
    player_props: Props<'conn>,
}

impl<'conn> Player<'conn> {
    pub fn new<B>(connection: &'conn Connection, bus_name: B) -> Result<Player<'conn>>
    where
        B: Into<BusName<'conn>>,
    {
        let bus_name = bus_name.into();

        let parent_props = Props::new(
            connection,
            bus_name.clone(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2",
            DEFAULT_TIMEOUT,
        );

        let player_props = Props::new(
            connection,
            bus_name.clone(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
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

    pub fn bus_name(&self) -> &str {
        &self.bus_name
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn get_metadata(&self) -> Result<Metadata> {
        self.player_props
            .get("Metadata")
            .map_err(|e| e.into())
            .and_then(Metadata::new_from_message_item)
    }

    pub fn play_pause(&self) -> Result<()> {
        let message = self.player_message("PlayPause");
        let _ = self.connection.send_with_reply_and_block(
            message,
            DEFAULT_TIMEOUT,
        )?;
        Ok(())
    }

    pub fn play(&self) -> Result<()> {
        self.send_player_void_message("Play")
    }

    pub fn pause(&self) -> Result<()> {
        self.send_player_void_message("Pause")
    }

    pub fn stop(&self) -> Result<()> {
        self.send_player_void_message("Stop")
    }

    pub fn next(&self) -> Result<()> {
        self.send_player_void_message("Next")
    }

    pub fn previous(&self) -> Result<()> {
        self.send_player_void_message("Previous")
    }

    pub fn checked_play_pause(&self) -> Result<bool> {
        if self.can_pause()? {
            self.play_pause().map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn checked_play(&self) -> Result<bool> {
        if self.can_play()? {
            self.play().map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn checked_pause(&self) -> Result<bool> {
        if self.can_pause()? {
            self.pause().map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn checked_stop(&self) -> Result<bool> {
        if self.can_stop()? {
            self.stop().map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn checked_next(&self) -> Result<bool> {
        if self.can_go_next()? {
            self.next().map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn checked_previous(&self) -> Result<bool> {
        if self.can_go_previous()? {
            self.previous().map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn can_control(&self) -> Result<bool> {
        self.player_bool_property("CanControl")
    }

    pub fn can_go_next(&self) -> Result<bool> {
        self.player_bool_property("CanGoNext")
    }

    pub fn can_go_previous(&self) -> Result<bool> {
        self.player_bool_property("CanGoPrevious")
    }

    pub fn can_pause(&self) -> Result<bool> {
        self.player_bool_property("CanPause")
    }

    pub fn can_play(&self) -> Result<bool> {
        self.player_bool_property("CanPlay")
    }

    pub fn can_seek(&self) -> Result<bool> {
        self.player_bool_property("CanSeek")
    }

    pub fn can_stop(&self) -> Result<bool> {
        self.can_control()
    }

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
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
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

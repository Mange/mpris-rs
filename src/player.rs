extern crate dbus;

use std::rc::Rc;
use std::time::Duration;

use dbus::{BusName, ConnPath, Connection, Path};

use super::{DBusError, LoopStatus, PlaybackStatus, TrackID};
use extensions::DurationExtensions;
use generated::OrgMprisMediaPlayer2;
use generated::OrgMprisMediaPlayer2Player;
use metadata::Metadata;
use pooled_connection::PooledConnection;
use progress::ProgressTracker;

pub(crate) const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub(crate) const MPRIS2_PATH: &str = "/org/mpris/MediaPlayer2";

/// When D-Bus connection is managed for you, use this timeout while communicating with a Player.
pub const DEFAULT_TIMEOUT_MS: i32 = 500; // ms

/// A MPRIS-compatible player.
///
/// You can query this player about the currently playing media, or control it.
///
/// **See:** [MPRIS2 MediaPlayer2.Player Specification][spec]
/// [spec]: <https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html>
#[derive(Debug)]
pub struct Player<'a> {
    connection: Rc<PooledConnection>,
    bus_name: BusName<'a>,
    unique_name: String,
    identity: String,
    path: Path<'a>,
    timeout_ms: i32,
}

impl<'a> Player<'a> {
    /// Create a new `Player` using a D-Bus connection and address information.
    ///
    /// If no player is running on this bus name an `Err` will be returned.
    pub fn new<B, P>(
        connection: Connection,
        bus_name: B,
        path: P,
        timeout_ms: i32,
    ) -> Result<Player<'a>, DBusError>
    where
        B: Into<BusName<'a>>,
        P: Into<Path<'a>>,
    {
        Player::for_pooled_connection(
            Rc::new(connection.into()),
            bus_name.into(),
            path.into(),
            timeout_ms,
        )
    }

    pub(crate) fn for_pooled_connection(
        pooled_connection: Rc<PooledConnection>,
        bus_name: BusName<'a>,
        path: Path<'a>,
        timeout_ms: i32,
    ) -> Result<Player<'a>, DBusError> {
        let identity = {
            let connection_path =
                pooled_connection.with_path(bus_name.clone(), path.clone(), timeout_ms);
            connection_path.get_identity()?
        };

        let unique_name = pooled_connection
            .determine_unique_name(&*bus_name)
            .ok_or_else(|| {
                DBusError::new(
                    "Could not determine player's unique name. Did it exit during initialization?",
                )
            })?;

        Ok(Player {
            connection: pooled_connection,
            bus_name: bus_name,
            unique_name: unique_name,
            identity: identity,
            path: path,
            timeout_ms: timeout_ms,
        })
    }

    /// Returns the current D-Bus communication timeout (in milliseconds).
    ///
    /// When querying D-Bus the call should not block longer than this, and will instead fail the
    /// query if no response has been received in this time.
    ///
    /// You can change this using `set_dbus_timeout_ms`.
    pub fn dbus_timeout_ms(&self) -> i32 {
        self.timeout_ms
    }

    /// Change the D-Bus communication timeout.
    pub fn set_dbus_timeout_ms(&mut self, timeout_ms: i32) {
        self.timeout_ms = timeout_ms;
    }

    /// Returns the player's D-Bus bus name.
    pub fn bus_name(&self) -> &BusName {
        &self.bus_name
    }

    /// Returns the player's unique D-Bus bus name (usually something like `:1.1337`).
    pub fn unique_name(&self) -> &str {
        &self.unique_name
    }

    /// Returns the player's MPRIS `Identity`.
    ///
    /// This is usually the application's name, like `Spotify`.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the player's MPRIS `position` as a `Duration` since the start of the media.
    pub fn get_position(&self) -> Result<Duration, DBusError> {
        self.get_position_in_microseconds()
            .map(|us| Duration::from_micros_ext(us))
    }

    /// Returns the player's MPRIS `position` as a count of microseconds since the start of the
    /// media.
    pub fn get_position_in_microseconds(&self) -> Result<u64, DBusError> {
        self.connection_path()
            .get_position()
            .map(|p| p as u64)
            .map_err(|e| e.into())
    }

    /// Sets the position of the current track to the given position (as a `Duration`).
    ///
    /// Current `TrackID` must be provided to avoid race conditions with the player, in case it
    /// changes tracks while the signal is being sent.
    ///
    /// **Note:** There is currently no good way to retrieve the current `TrackID` through the
    /// `mpris` library. You will have to manually retrieve it through D-Bus until implemented.
    ///
    /// See: [MPRIS2 specification about `SetPosition`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:SetPosition)
    pub fn set_position<'id, ID>(&self, track_id: ID, position: &Duration) -> Result<(), DBusError>
    where
        ID: Into<TrackID<'id>>,
    {
        self.set_position_in_microseconds(track_id, position.as_micros())
    }

    /// Sets the position of the current track to the given position (in microseconds).
    ///
    /// Current `TrackID` must be provided to avoid race conditions with the player, in case it
    /// changes tracks while the signal is being sent.
    ///
    /// **Note:** There is currently no good way to retrieve the current `TrackID` through the
    /// `mpris` library. You will have to manually retrieve it through D-Bus until implemented.
    ///
    /// See: [MPRIS2 specification about `SetPosition`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:SetPosition)
    pub fn set_position_in_microseconds<'id, ID>(
        &self,
        track_id: ID,
        position_in_us: u64,
    ) -> Result<(), DBusError>
    where
        ID: Into<TrackID<'id>>,
    {
        self.connection_path()
            .set_position(track_id.into().0, position_in_us as i64)
            .map_err(|e| e.into())
    }

    /// Returns the player's MPRIS (playback) `rate` as a factor.
    ///
    /// 1.0 would mean normal rate, while 2.0 would mean twice the playback speed.
    pub fn get_playback_rate(&self) -> Result<f32, DBusError> {
        self.connection_path()
            .get_rate()
            .map(|p| p as f32)
            .map_err(|e| e.into())
    }

    /// Query the player for current metadata.
    ///
    /// See `Metadata` for more information about what is included here.
    pub fn get_metadata(&self) -> Result<Metadata, DBusError> {
        self.connection_path()
            .get_metadata()
            .map_err(|e| e.into())
            .and_then(Metadata::new_from_dbus)
    }

    /// Returns a new `ProgressTracker` for the player.
    ///
    /// Use this if you want to monitor a player in order to show close-to-realtime information
    /// about it.
    pub fn track_progress(&self, interval_ms: u32) -> Result<ProgressTracker, DBusError> {
        ProgressTracker::new(self, interval_ms)
    }

    pub(crate) fn connection(&self) -> &PooledConnection {
        &self.connection
    }

    /// Send a `PlayPause` signal to the player.
    ///
    /// See: [MPRIS2 specification about `PlayPause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause)
    pub fn play_pause(&self) -> Result<(), DBusError> {
        self.connection_path().play_pause().map_err(|e| e.into())
    }

    /// Send a `Play` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Play`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Play)
    pub fn play(&self) -> Result<(), DBusError> {
        self.connection_path().play().map_err(|e| e.into())
    }

    /// Send a `Pause` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Pause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Pause)
    pub fn pause(&self) -> Result<(), DBusError> {
        self.connection_path().pause().map_err(|e| e.into())
    }

    /// Send a `Stop` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Stop`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop)
    pub fn stop(&self) -> Result<(), DBusError> {
        self.connection_path().stop().map_err(|e| e.into())
    }

    /// Send a `Next` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Next`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Next)
    pub fn next(&self) -> Result<(), DBusError> {
        self.connection_path().next().map_err(|e| e.into())
    }

    /// Send a `Previous` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Previous`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Previous)
    pub fn previous(&self) -> Result<(), DBusError> {
        self.connection_path().previous().map_err(|e| e.into())
    }

    /// Send a `Seek` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Seek`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek)
    pub fn seek(&self, offset_in_microseconds: i64) -> Result<(), DBusError> {
        self.connection_path()
            .seek(offset_in_microseconds)
            .map_err(|e| e.into())
    }

    /// Tell the player to seek forwards.
    ///
    /// See: `seek` method on `Player`.
    pub fn seek_forwards(&self, offset: &Duration) -> Result<(), DBusError> {
        self.seek(offset.as_micros() as i64)
    }

    /// Tell the player to seek backwards.
    ///
    /// See: `seek` method on `Player`.
    pub fn seek_backwards(&self, offset: &Duration) -> Result<(), DBusError> {
        self.seek(-(offset.as_micros() as i64))
    }

    /// Sends a `PlayPause` signal to the player, if the player indicates that it can pause.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `PlayPause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause)
    pub fn checked_play_pause(&self) -> Result<bool, DBusError> {
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
    pub fn checked_play(&self) -> Result<bool, DBusError> {
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
    pub fn checked_pause(&self) -> Result<bool, DBusError> {
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
    pub fn checked_stop(&self) -> Result<bool, DBusError> {
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
    pub fn checked_next(&self) -> Result<bool, DBusError> {
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
    pub fn checked_previous(&self) -> Result<bool, DBusError> {
        if self.can_go_previous()? {
            self.previous().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Sends a `Seek` signal to the player, if the player indicates that it can seek.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Seek`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek)
    pub fn checked_seek(&self, offset_in_microseconds: i64) -> Result<bool, DBusError> {
        if self.can_seek()? {
            self.seek(offset_in_microseconds).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Seeks the player forwards, if the player indicates that it can seek.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Seek`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek)
    pub fn checked_seek_forwards(&self, offset: &Duration) -> Result<bool, DBusError> {
        if self.can_seek()? {
            self.seek_forwards(offset).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Seeks the player backwards, if the player indicates that it can seek.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Seek`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek)
    pub fn checked_seek_backwards(&self, offset: &Duration) -> Result<bool, DBusError> {
        if self.can_seek()? {
            self.seek_backwards(offset).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Queries the player to see if it can be controlled or not.
    ///
    /// See: [MPRIS2 specification about `CanControl`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl)
    pub fn can_control(&self) -> Result<bool, DBusError> {
        self.connection_path()
            .get_can_control()
            .map_err(|e| e.into())
    }

    /// Queries the player to see if it can go to next or not.
    ///
    /// See: [MPRIS2 specification about `CanGoNext`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoNext)
    pub fn can_go_next(&self) -> Result<bool, DBusError> {
        self.connection_path()
            .get_can_go_next()
            .map_err(|e| e.into())
    }

    /// Queries the player to see if it can go to previous or not.
    ///
    /// See: [MPRIS2 specification about `CanGoPrevious`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoPrevious)
    pub fn can_go_previous(&self) -> Result<bool, DBusError> {
        self.connection_path()
            .get_can_go_previous()
            .map_err(|e| e.into())
    }

    /// Queries the player to see if it can pause.
    ///
    /// See: [MPRIS2 specification about `CanPause`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPause)
    pub fn can_pause(&self) -> Result<bool, DBusError> {
        self.connection_path().get_can_pause().map_err(|e| e.into())
    }

    /// Queries the player to see if it can play.
    ///
    /// See: [MPRIS2 specification about `CanPlay`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPlay)
    pub fn can_play(&self) -> Result<bool, DBusError> {
        self.connection_path().get_can_play().map_err(|e| e.into())
    }

    /// Queries the player to see if it can seek within the media.
    ///
    /// See: [MPRIS2 specification about `CanSeek`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanSeek)
    pub fn can_seek(&self) -> Result<bool, DBusError> {
        self.connection_path().get_can_seek().map_err(|e| e.into())
    }

    /// Queries the player to see if it can stop.
    ///
    /// MPRIS2 defines [the `Stop` message to only work when the player can be controlled][stop], so that
    /// is the property used for this method.
    ///
    /// See: [MPRIS2 specification about `CanControl`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl)
    /// [stop]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop
    pub fn can_stop(&self) -> Result<bool, DBusError> {
        self.can_control()
    }

    /// Query the player for current playback status.
    pub fn get_playback_status(&self) -> Result<PlaybackStatus, DBusError> {
        self.connection_path()
            .get_playback_status()?
            .parse()
            .map_err(DBusError::from)
    }

    /// Query player for the state of the "Shuffle" setting.
    ///
    /// See: [MPRIS2 specification about
    /// `Shuffle`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle)
    pub fn get_shuffle(&self) -> Result<bool, DBusError> {
        self.connection_path()
            .get_shuffle()
            .map_err(DBusError::from)
    }

    /// Set the "Shuffle" setting of the player.
    ///
    /// See: [MPRIS2 specification about
    /// `Shuffle`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle)
    pub fn set_shuffle(&self, state: bool) -> Result<(), DBusError> {
        self.connection_path()
            .set_shuffle(state)
            .map_err(DBusError::from)
    }

    /// Set the "Shuffle" setting of the player, if the player indicates that it can be controlled.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about
    /// `Shuffle`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle)
    pub fn checked_set_shuffle(&self, state: bool) -> Result<bool, DBusError> {
        if self.can_control()? {
            self.set_shuffle(state)
                .map(|_| true)
                .map_err(DBusError::from)
        } else {
            Ok(false)
        }
    }

    /// Query the player for the current loop status.
    ///
    /// See: [MPRIS2 specification about
    /// `LoopStatus`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus)
    pub fn get_loop_status(&self) -> Result<LoopStatus, DBusError> {
        self.connection_path()
            .get_loop_status()?
            .parse()
            .map_err(DBusError::from)
    }

    /// Set the loop status of the player.
    ///
    /// See: [MPRIS2 specification about
    /// `LoopStatus`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus)
    pub fn set_loop_status(&self, status: LoopStatus) -> Result<(), DBusError> {
        self.connection_path()
            .set_loop_status(status.dbus_value())
            .map_err(DBusError::from)
    }

    /// Set the loop status of the player, if the player indicates that it can be controlled.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about
    /// `LoopStatus`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus)
    pub fn checked_set_loop_status(&self, status: LoopStatus) -> Result<bool, DBusError> {
        if self.can_control()? {
            self.set_loop_status(status)
                .map(|_| true)
                .map_err(DBusError::from)
        } else {
            Ok(false)
        }
    }

    /// Get the volume of the player.
    ///
    /// Volume should be between 0.0 and 1.0. Above 1.0 is possible, but not
    /// recommended.
    ///
    /// See: [MPRIS2 specification about
    /// `Volume`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume)
    pub fn get_volume(&self) -> Result<f64, DBusError> {
        self.connection_path().get_volume().map_err(DBusError::from)
    }

    /// Set the volume of the player.
    ///
    /// Volume should be between 0.0 and 1.0. Above 1.0 is possible, but not
    /// recommended.
    ///
    /// See: [MPRIS2 specification about
    /// `Volume`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume)
    pub fn set_volume(&self, value: f64) -> Result<(), DBusError> {
        self.connection_path()
            .set_volume(value.min(0.0))
            .map_err(DBusError::from)
    }

    /// Set the volume of the player, if the player indicates that it can be
    /// controlled.
    ///
    /// Volume should be between 0.0 and 1.0. Above 1.0 is possible, but not
    /// recommended.
    ///
    /// See: [MPRIS2 specification about
    /// `Volume`](https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume)
    pub fn set_volume_checked(&self, value: f64) -> Result<bool, DBusError> {
        if self.can_control()? {
            self.set_volume(value).map(|_| true)
        } else {
            Ok(false)
        }
    }

    fn connection_path(&self) -> ConnPath<&Connection> {
        // TODO: Can we create this only once? Maybe using the Once type, or a RefCell?
        self.connection
            .with_path(self.bus_name.clone(), self.path.clone(), self.timeout_ms)
    }
}

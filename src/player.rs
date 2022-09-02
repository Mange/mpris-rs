use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use dbus::ffidisp::{ConnPath, Connection};
use dbus::strings::{BusName, Path};

use super::{DBusError, LoopStatus, MetadataValue, PlaybackStatus, TrackID, TrackList};
use crate::event::PlayerEvents;
use crate::extensions::DurationExtensions;
use crate::generated::OrgMprisMediaPlayer2;
use crate::generated::OrgMprisMediaPlayer2Player;
use crate::metadata::Metadata;
use crate::pooled_connection::{MprisEvent, PooledConnection};
use crate::progress::ProgressTracker;

pub(crate) const MPRIS2_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub(crate) const MPRIS2_PATH: &str = "/org/mpris/MediaPlayer2";

/// When D-Bus connection is managed for you, use this timeout while communicating with a Player.
pub(crate) const DEFAULT_TIMEOUT_MS: i32 = 500; // ms

/// A MPRIS-compatible player.
///
/// You can query this player about the currently playing media, or control it.
///
/// **See:** [MPRIS2 MediaPlayer2.Player Specification][spec].
///
/// [spec]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html
#[derive(Debug)]
pub struct Player {
    connection: Rc<PooledConnection>,
    bus_name: String,
    unique_name: String,
    identity: String,
    timeout_ms: i32,
    has_tracklist_interface: bool,
}

impl Player {
    /// Create a new [`Player`] using a D-Bus connection and address information.
    ///
    /// If no player is running on this bus name an [`Err`] will be returned.
    pub fn new(
        connection: Connection,
        bus_name: String,
        timeout_ms: i32,
    ) -> Result<Player, DBusError> {
        Player::for_pooled_connection(Rc::new(connection.into()), bus_name, timeout_ms)
    }

    pub(crate) fn for_pooled_connection(
        pooled_connection: Rc<PooledConnection>,
        bus_name: String,
        timeout_ms: i32,
    ) -> Result<Player, DBusError> {
        let path: Path = MPRIS2_PATH.into();
        let bus: BusName = bus_name.as_str().into();
        let identity = {
            let connection_path =
                pooled_connection.with_path(bus.clone(), path.clone(), timeout_ms);
            connection_path.identity()?
        };

        let unique_name = pooled_connection
            .determine_unique_name(&bus_name)
            .ok_or_else(|| {
                DBusError::Miscellaneous(String::from(
                    "Could not determine player's unique name. Did it exit during initialization?",
                ))
            })?;

        let has_tracklist_interface = {
            let connection_path = pooled_connection.with_path(bus, path, timeout_ms);
            has_tracklist_interface(connection_path).unwrap_or(false)
        };

        Ok(Player {
            connection: pooled_connection,
            bus_name,
            unique_name,
            identity,
            timeout_ms,
            has_tracklist_interface,
        })
    }

    /// Returns the current D-Bus communication timeout (in milliseconds).
    ///
    /// When querying D-Bus the call should not block longer than this, and will instead fail the
    /// query if no response has been received in this time.
    ///
    /// You can change this using [`set_dbus_timeout_ms`](Self::set_dbus_timeout_ms).
    pub fn dbus_timeout_ms(&self) -> i32 {
        self.timeout_ms
    }

    /// Change the D-Bus communication timeout.
    pub fn set_dbus_timeout_ms(&mut self, timeout_ms: i32) {
        self.timeout_ms = timeout_ms;
    }

    /// Returns the player's D-Bus bus name.
    pub fn bus_name(&self) -> &str {
        &self.bus_name
    }

    /// Returns the player name part of the player's D-Bus bus name.
    /// This is the part after "org.mpris.MediaPlayer2.", not including the instance part.
    ///
    /// See: [MPRIS2 specification about bus names][bus_names].
    ///
    /// [bus_names]: https://specifications.freedesktop.org/mpris-spec/latest/#Bus-Name-Policy
    pub fn bus_name_player_name_part(&self) -> &str {
        self.bus_name()
            .trim_start_matches(MPRIS2_PREFIX)
            .split('.') // Remove the "instance" part
            .next()
            .unwrap()
    }

    /// Returns the player's unique D-Bus bus name (usually something like `:1.1337`).
    pub fn unique_name(&self) -> &str {
        &self.unique_name
    }

    /// Returns the player's MPRIS [`Identity`][identity].
    ///
    /// This is usually the application's name, like `Spotify`.
    ///
    /// [identity]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Identity
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Checks if the Player implements the `org.mpris.MediaPlayer2.TrackList` interface.
    pub fn supports_track_lists(&self) -> bool {
        self.has_tracklist_interface
    }

    /// Returns the player's `DesktopEntry` property, if supported.
    ///
    /// See: [MPRIS2 specification about `DesktopEntry`][desktop_entry].
    ///
    /// [desktop_entry]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:DesktopEntry
    pub fn get_desktop_entry(&self) -> Result<Option<String>, DBusError> {
        handle_optional_property(self.connection_path().desktop_entry())
    }

    /// Returns the player's `SupportedMimeTypes` property.
    ///
    /// See: [MPRIS2 specification about `SupportedMimeTypes`][mime_types].
    ///
    /// [mime_types]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:SupportedMimeTypes
    pub fn get_supported_mime_types(&self) -> Result<Vec<String>, DBusError> {
        self.connection_path()
            .supported_mime_types()
            .map_err(|e| e.into())
    }

    /// Returns the player's `SupportedUriSchemes` property.
    ///
    /// See: [MPRIS2 specification about `SupportedUriSchemes`][schemes].
    ///
    /// [schemes]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:SupportedUriSchemes
    pub fn get_supported_uri_schemes(&self) -> Result<Vec<String>, DBusError> {
        self.connection_path()
            .supported_uri_schemes()
            .map_err(|e| e.into())
    }

    /// Returns the player's `HasTrackList` property.
    ///
    /// See: [MPRIS2 specification about `HasTrackList`][track_list].
    ///
    /// [track_list]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:HasTrackList
    pub fn get_has_track_list(&self) -> Result<bool, DBusError> {
        self.connection_path()
            .has_track_list()
            .map_err(|e| e.into())
    }

    /// Returns the player's MPRIS `position` as a [`Duration`] since the start of the media.
    pub fn get_position(&self) -> Result<Duration, DBusError> {
        self.get_position_in_microseconds()
            .map(Duration::from_micros_ext)
    }

    /// Gets the "Position" setting, if the player indicates that it supports it.
    ///
    /// Return [`Some`] containing the current value of the position. If the setting is not
    /// supported, return [`None`]
    pub fn checked_get_position(&self) -> Result<Option<Duration>, DBusError> {
        if self.has_position()? {
            Ok(Some(self.get_position()?))
        } else {
            Ok(None)
        }
    }

    /// Returns the player's MPRIS `position` as a count of microseconds since the start of the
    /// media.
    pub fn get_position_in_microseconds(&self) -> Result<u64, DBusError> {
        self.connection_path()
            .position()
            .map(|p| p as u64)
            .map_err(|e| e.into())
    }

    /// Sets the position of the current track to the given position (as a [`Duration`]).
    ///
    /// Current [`TrackID`] must be provided to avoid race conditions with the player, in case it
    /// changes tracks while the signal is being sent.
    ///
    /// **Note:** There is currently no good way to retrieve the current [`TrackID`] through the
    /// `mpris` library. You will have to manually retrieve it through D-Bus until implemented.
    ///
    /// See: [MPRIS2 specification about `SetPosition`][set_position].
    ///
    /// [set_position]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:SetPosition
    pub fn set_position(&self, track_id: TrackID, position: &Duration) -> Result<(), DBusError> {
        self.set_position_in_microseconds(track_id, DurationExtensions::as_micros(position))
    }

    /// Set the "Position" setting of the player, if the player indicates that it supports the
    /// "Position" setting and can be controlled.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Position`][position].
    ///
    /// [position]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Position
    pub fn checked_set_position(
        &self,
        track_id: TrackID,
        position: &Duration,
    ) -> Result<bool, DBusError> {
        if self.can_control()? && self.has_position()? {
            self.set_position(track_id, position)
                .map(|_| true)
                .map_err(DBusError::from)
        } else {
            Ok(false)
        }
    }

    /// Sets the position of the current track to the given position (in microseconds).
    ///
    /// Current [`TrackID`] must be provided to avoid race conditions with the player, in case it
    /// changes tracks while the signal is being sent.
    ///
    /// **Note:** There is currently no good way to retrieve the current [`TrackID`] through the
    /// `mpris` library. You will have to manually retrieve it through D-Bus until implemented.
    ///
    /// See: [MPRIS2 specification about `SetPosition`][set_position].
    ///
    /// [set_position]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:SetPosition
    pub fn set_position_in_microseconds(
        &self,
        track_id: TrackID,
        position_in_us: u64,
    ) -> Result<(), DBusError> {
        self.connection_path()
            .set_position(track_id.as_path(), position_in_us as i64)
            .map_err(|e| e.into())
    }

    /// Returns the player's MPRIS (playback) `rate` as a factor.
    ///
    /// 1.0 would mean normal rate, while 2.0 would mean twice the playback speed.
    pub fn get_playback_rate(&self) -> Result<f64, DBusError> {
        self.connection_path().rate().map_err(|e| e.into())
    }

    /// Gets the "Rate" setting, if the player indicates that it supports it.
    ///
    /// Returns [`Some`] containing the current value of the rate setting. If the setting is not
    /// supported, returns [`None`]
    pub fn checked_get_playback_rate(&self) -> Result<Option<f64>, DBusError> {
        if self.has_playback_rate()? {
            Ok(Some(self.get_playback_rate()?))
        } else {
            Ok(None)
        }
    }

    /// Sets the player's MPRIS (playback) `rate` as a factor.
    ///
    /// 1.0 would mean normal rate, while 2.0 would mean twice the playback speed.
    ///
    /// It is not allowed to try to set playback rate to a value outside of the supported range.
    /// [`get_valid_playback_rate_range`](Self::get_valid_playback_rate_range) returns a [`Range<f64>`] that encodes the maximum and
    /// minimum values.
    ///
    /// You must not set rate to 0.0; instead call [`pause`](Self::pause).
    ///
    /// See: [MPRIS2 specification about `Rate`][rate].
    ///
    /// [rate]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Rate
    pub fn set_playback_rate(&self, rate: f64) -> Result<(), DBusError> {
        self.connection_path().set_rate(rate).map_err(|e| e.into())
    }

    /// Set the playback rate of the player, if the player indicates that supports it and that it
    /// can be controlled.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Rate`][rate].
    ///
    /// [rate]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Rate
    pub fn checked_set_playback_rate(&self, rate: f64) -> Result<bool, DBusError> {
        if self.can_control()? && self.has_playback_rate()? {
            self.set_playback_rate(rate)
                .map(|_| true)
                .map_err(DBusError::from)
        } else {
            Ok(false)
        }
    }

    /// Gets the minimum allowed value for playback rate.
    ///
    /// See: [MPRIS2 specification about `MinimumRate`][min_rate].
    ///
    /// [min_rate]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:MinimumRate
    pub fn get_minimum_playback_rate(&self) -> Result<f64, DBusError> {
        self.connection_path().minimum_rate().map_err(|e| e.into())
    }

    /// Gets the maximum allowed value for playback rate.
    ///
    /// See: [MPRIS2 specification about `MaximumRate`][max_rate].
    ///
    /// [max_rate]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:MaximumRate
    pub fn get_maximum_playback_rate(&self) -> Result<f64, DBusError> {
        self.connection_path().maximum_rate().map_err(|e| e.into())
    }

    /// Gets the minimum-maximum allowed value range for playback rate.
    ///
    /// See: [`get_minimum_playback_rate`](Self::get_minimum_playback_rate)
    /// and [`get_maximum_playback_rate`](Self::get_maximum_playback_rate).
    pub fn get_valid_playback_rate_range(&self) -> Result<Range<f64>, DBusError> {
        self.get_minimum_playback_rate()
            .and_then(|min| self.get_maximum_playback_rate().map(|max| min..max))
    }

    /// Query the player for current metadata.
    ///
    /// See [`Metadata`] for more information about what is included here.
    pub fn get_metadata(&self) -> Result<Metadata, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        let connection_path = self.connection_path();

        Properties::get::<HashMap<String, MetadataValue>>(
            &connection_path,
            "org.mpris.MediaPlayer2.Player",
            "Metadata",
        )
        .map(Metadata::from)
        .map_err(DBusError::from)
    }

    /// Query the player for the current tracklist.
    ///
    /// **Note:** It's more expensive to rebuild this each time rather than trying to keep the same
    /// [`TrackList`] updated. See [`TrackList::reload`].
    ///
    /// See [`checked_get_track_list`](Self::checked_get_track_list) to automatically detect players not supporting track lists.
    pub fn get_track_list(&self) -> Result<TrackList, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        let connection_path = self.connection_path();

        Properties::get::<Vec<Path<'_>>>(
            &connection_path,
            "org.mpris.MediaPlayer2.TrackList",
            "Tracks",
        )
        .map(TrackList::from)
        .map_err(DBusError::from)
    }

    /// Query the player for the current tracklist.
    ///
    /// **Note:** It's more expensive to rebuild this each time rather than trying to keep the same
    /// [`TrackList`] updated. See [`TrackList::reload`].
    ///
    /// See [`get_track_list`](Self::get_track_list) and [`supports_track_lists`](Self::supports_track_lists)
    /// if you want to manually handle compatibility checks.
    pub fn checked_get_track_list(&self) -> Result<Option<TrackList>, DBusError> {
        if self.supports_track_lists() {
            self.get_track_list().map(Some)
        } else {
            Ok(None)
        }
    }

    /// Query the player to see if it allows changes to its TrackList.
    ///
    /// Will return [`Err`] if Player isn't supporting the [`TrackList`] interface.
    ///
    /// See [`checked_can_edit_tracks`](Self::checked_can_edit_tracks) to automatically detect players not supporting track lists.
    ///
    /// See: [MPRIS2 specification about `CanEditTracks`][can_edit].
    ///
    /// [can_edit]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Property:CanEditTracks
    pub fn can_edit_tracks(&self) -> Result<bool, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;
        let connection_path = self.connection_path();

        Properties::get::<bool>(
            &connection_path,
            "org.mpris.MediaPlayer2.TrackList",
            "CanEditTracks",
        )
        .map_err(DBusError::from)
    }

    /// Query the player to see if it allows changes to its TrackList.
    ///
    /// Will return [`false`] if [`Player`] isn't supporting the `TrackList` interface.
    ///
    /// See [`can_edit_tracks`](Self::can_edit_tracks) and [`supports_track_lists`](Self::supports_track_lists)
    /// if you want to manually handle compatibility checks.
    ///
    /// See: [MPRIS2 specification about `CanEditTracks`][can_edit].
    ///
    /// [can_edit]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Property:CanEditTracks
    pub fn checked_can_edit_tracks(&self) -> bool {
        if self.supports_track_lists() {
            self.can_edit_tracks().unwrap_or(false)
        } else {
            false
        }
    }

    /// Query the player for metadata for the given [`TrackID`]s.
    ///
    /// This is used by the [`TrackList`] type to iterator metadata for the tracks in the track list.
    ///
    /// See: [MediaPlayer2.TrackList.GetTracksMetadata][get_meta].
    ///
    /// [get_meta]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:GetTracksMetadata
    pub fn get_tracks_metadata(&self, track_ids: &[TrackID]) -> Result<Vec<Metadata>, DBusError> {
        use dbus::arg::IterAppend;
        let connection_path = self.connection_path();

        let mut method = connection_path.method_call_with_args(
            &"org.mpris.MediaPlayer2.TrackList".into(),
            &"GetTracksMetadata".into(),
            |msg| {
                let mut i = IterAppend::new(msg);
                i.append(track_ids.iter().map(|id| id.as_path()).collect::<Vec<_>>());
            },
        )?;
        method.as_result()?;
        let mut i = method.iter_init();
        let metadata: Vec<::std::collections::HashMap<String, MetadataValue>> = i.read()?;

        if metadata.len() == track_ids.len() {
            Ok(metadata.into_iter().map(Metadata::from).collect())
        } else {
            Err(DBusError::Miscellaneous(format!(
                "Expected {} tracks, but got {} tracks returned.",
                track_ids.len(),
                metadata.len()
            )))
        }
    }

    /// Query the player for metadata for a single [`TrackID`].
    ///
    /// Note that [`get_tracks_metadata`](Self::get_tracks_metadata) with a list is more effective if you have more than a
    /// single [`TrackID`] to load.
    ///
    /// See: [MediaPlayer2.TrackList.GetTracksMetadata][get_meta].
    ///
    /// [get_meta]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:GetTracksMetadata
    pub fn get_track_metadata(&self, track_id: &TrackID) -> Result<Metadata, DBusError> {
        self.get_tracks_metadata(&[track_id.clone()])
            .and_then(|mut result| {
                result.pop().map(Ok).unwrap_or_else(|| {
                    Err(DBusError::Miscellaneous(format!(
                        "Player gave no Metadata for {}",
                        track_id
                    )))
                })
            })
    }

    /// Returns a new [`ProgressTracker`] for the player.
    ///
    /// Use this if you want to monitor a player in order to show close-to-realtime information
    /// about it.
    ///
    /// It is built like a blocking "frame limiter" where it returns at an approximately fixed
    /// interval with the most up-to-date information. It's mostly appropriate when trying to
    /// render something like a progress bar, or information about the current track.
    ///
    /// See: [`events`](Self::events) for an alternative approach.
    pub fn track_progress(&self, interval_ms: u32) -> Result<ProgressTracker<'_>, DBusError> {
        ProgressTracker::new(self, interval_ms)
    }

    /// Returns a [`PlayerEvents`] iterator, or an [`DBusError`] if there was a problem with the D-Bus
    /// connection to the player.
    ///
    /// This iterator will block until an event for the current player is emitted. This is a lot
    /// more bare-bones than [`track_progress`](Self::track_progress), but it's also something that makes it easier
    /// for you to translate events into your own application's domain events and only deal with
    /// actual changes.
    ///
    /// You could implement your own progress tracker on top of this, but it's probably not
    /// appropriate to render a live progress bar using this iterator as the progress bar will
    /// remain frozen until the next event is emitted and the iterator returns.
    ///
    /// See: [`track_progress`](Self::track_progress) for an alternative approach.
    pub fn events(&self) -> Result<PlayerEvents, DBusError> {
        PlayerEvents::new(self)
    }

    /// Returns true if the bus of this player is still occupied in the connection, or put in
    /// another way: If there's a process still listening on messages on this bus.
    ///
    /// If the player that you are controlling / querying has shut down, then this would return
    /// false. You can use this to do graceful restarts, begin looking for another player, etc.
    pub fn is_running(&self) -> bool {
        self.connection()
            .name_has_owner(self.bus_name.to_string())
            .unwrap_or(false)
    }

    pub(crate) fn connection(&self) -> &PooledConnection {
        &self.connection
    }

    /// Send a `PlayPause` signal to the player.
    ///
    /// See: [MPRIS2 specification about `PlayPause`][play_pause]
    ///
    /// [play_pause]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause
    pub fn play_pause(&self) -> Result<(), DBusError> {
        self.connection_path().play_pause().map_err(|e| e.into())
    }

    /// Send a `Play` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Play`][play].
    ///
    /// [play]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Play
    pub fn play(&self) -> Result<(), DBusError> {
        self.connection_path().play().map_err(|e| e.into())
    }

    /// Send a `Pause` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Pause`][pause].
    ///
    /// [pause]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Pause
    pub fn pause(&self) -> Result<(), DBusError> {
        self.connection_path().pause().map_err(|e| e.into())
    }

    /// Send a `Stop` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Stop`][stop].
    ///
    /// [stop]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop
    pub fn stop(&self) -> Result<(), DBusError> {
        self.connection_path().stop().map_err(|e| e.into())
    }

    /// Send a `Next` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Next`][next].
    ///
    /// [next]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Next
    pub fn next(&self) -> Result<(), DBusError> {
        self.connection_path().next().map_err(|e| e.into())
    }

    /// Send a `Previous` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Previous`][prev].
    ///
    /// [prev]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Previous
    pub fn previous(&self) -> Result<(), DBusError> {
        self.connection_path().previous().map_err(|e| e.into())
    }

    /// Send a `Seek` signal to the player.
    ///
    /// See: [MPRIS2 specification about `Seek`][seek].
    ///
    /// [seek]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek
    pub fn seek(&self, offset_in_microseconds: i64) -> Result<(), DBusError> {
        self.connection_path()
            .seek(offset_in_microseconds)
            .map_err(|e| e.into())
    }

    /// Tell the player to seek forwards.
    ///
    /// See: [`seek`](Self::seek) method.
    pub fn seek_forwards(&self, offset: &Duration) -> Result<(), DBusError> {
        self.seek(DurationExtensions::as_micros(offset) as i64)
    }

    /// Send a `Raise` signal to the player.
    ///
    /// > Brings the media player's user interface to the front using any appropriate mechanism
    /// > available.
    /// >
    /// > The media player may be unable to control how its user interface is displayed, or it may
    /// > not have a graphical user interface at all. In this case, the CanRaise property is false
    /// > and this method does nothing.
    ///
    /// See: [MPRIS2 specification about `Raise`][raise] and the [`can_raise`](Self::can_raise) method.
    ///
    /// [raise]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Method:Raise
    pub fn raise(&self) -> Result<(), DBusError> {
        self.connection_path().raise().map_err(|e| e.into())
    }

    /// Send a `Raise` signal to the player, if it supports it.
    ///
    /// See: [`can_raise`](Self::can_raise) and [`raise`](Self::raise) methods.
    pub fn checked_raise(&self) -> Result<bool, DBusError> {
        if self.can_raise()? {
            self.raise().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Send a `Quit` signal to the player.
    ///
    /// > Causes the media player to stop running.
    /// >
    /// > The media player may refuse to allow clients to shut it down. In this case, the CanQuit
    /// > property is false and this method does nothing.
    /// >
    /// > Note: Media players which can be D-Bus activated, or for which there is no sensibly easy
    /// > way to terminate a running instance (via the main interface or a notification area icon for
    /// > example) should allow clients to use this method. Otherwise, it should not be needed.
    /// >
    /// > If the media player does not have a UI, this should be implemented.
    ///
    /// See: [MPRIS2 specification about `Quit`][quit] and the [`can_quit`](Self::can_quit) method.
    ///
    /// [quit]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Method:Quit
    pub fn quit(&self) -> Result<(), DBusError> {
        self.connection_path().quit().map_err(|e| e.into())
    }

    /// Send a `Quit` signal to the player, if it supports it.
    ///
    /// See: [`can_quit`](Self::can_quit) and [`quit`](Self::quit) methods.
    pub fn checked_quit(&self) -> Result<bool, DBusError> {
        if self.can_quit()? {
            self.quit().map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Tell the player to seek backwards.
    ///
    /// See: [`seek`](Self::seek) method.
    pub fn seek_backwards(&self, offset: &Duration) -> Result<(), DBusError> {
        self.seek(-(DurationExtensions::as_micros(offset) as i64))
    }

    /// Go to a specific track on the [`Player`]'s [`TrackList`].
    ///
    /// If the given [`TrackID`] is not part of the player's [`TrackList`], it will have no effect.
    ///
    /// Requires the player to implement the `TrackList` interface.
    ///
    /// See: [MPRIS2 specification about `GoTo`][go_to]
    ///
    /// [go_to]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:GoTo
    pub fn go_to(&self, track_id: &TrackID) -> Result<(), DBusError> {
        use crate::generated::OrgMprisMediaPlayer2TrackList;

        self.connection_path()
            .go_to(track_id.into())
            .map_err(DBusError::from)
    }

    /// Add a URI to the TrackList and optionally set it as current.
    ///
    /// It is placed after the specified [`TrackID`], if supported by the player.
    ///
    /// Requires the player to implement the `TrackList` interface.
    ///
    /// See: [MPRIS2 specification about `AddTrack`][add_track].
    ///
    /// [add_track]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:AddTrack
    pub fn add_track(
        &self,
        uri: &str,
        after: &TrackID,
        set_as_current: bool,
    ) -> Result<(), DBusError> {
        use crate::generated::OrgMprisMediaPlayer2TrackList;

        self.connection_path()
            .add_track(uri, after.into(), set_as_current)
            .map_err(DBusError::from)
    }

    /// Add a URI to the start of the TrackList and optionally set it as current.
    ///
    /// Requires the player to implement the `TrackList` interface.
    ///
    /// See: [MPRIS2 specification about `AddTrack`][add_track].
    ///
    /// [add_track]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:AddTrack
    pub fn add_track_at_start(&self, uri: &str, set_as_current: bool) -> Result<(), DBusError> {
        use crate::generated::OrgMprisMediaPlayer2TrackList;

        self.connection_path()
            .add_track(uri, crate::track_list::NO_TRACK.into(), set_as_current)
            .map_err(DBusError::from)
    }

    /// Remove an item from the TrackList.
    ///
    /// Requires the player to implement the `TrackList` interface.
    ///
    /// See: [MPRIS2 specification about `RemoveTrack`][remove].
    ///
    /// [remove]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:RemoveTrack
    pub fn remove_track(&self, track_id: &TrackID) -> Result<(), DBusError> {
        use crate::generated::OrgMprisMediaPlayer2TrackList;

        self.connection_path()
            .remove_track(track_id.into())
            .map_err(DBusError::from)
    }

    /// Sends a `PlayPause` signal to the player, if the player indicates that it can pause.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `PlayPause`][play_pause]
    ///
    /// [play_pause]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause
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
    /// See: [MPRIS2 specification about `Play`][play].
    ///
    /// [play]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Play
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
    /// See: [MPRIS2 specification about `Pause`][pause].
    ///
    /// [pause]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Pause
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
    /// See: [MPRIS2 specification about `Stop`][stop].
    ///
    /// [stop]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop
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
    /// See: [MPRIS2 specification about `Next`][next].
    ///
    /// [next]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Next
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
    /// See: [MPRIS2 specification about `Previous`][prev].
    ///
    /// [prev]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Previous
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
    /// See: [MPRIS2 specification about `Seek`][seek].
    ///
    /// [seek]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek
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
    /// See: [MPRIS2 specification about `Seek`][seek].
    ///
    /// [seek]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek
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
    /// See: [MPRIS2 specification about `Seek`][seek].
    ///
    /// [seek]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek
    pub fn checked_seek_backwards(&self, offset: &Duration) -> Result<bool, DBusError> {
        if self.can_seek()? {
            self.seek_backwards(offset).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Queries the player to see if it can be raised or not.
    ///
    /// See: [MPRIS2 specification about `CanRaise`][can_raise] and the [`raise`](Self::raise) method.
    ///
    /// [can_raise]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:CanRaise
    pub fn can_raise(&self) -> Result<bool, DBusError> {
        self.connection_path().can_raise().map_err(|e| e.into())
    }

    /// Queries the player to see if it can be asked to quit.
    ///
    /// See: [MPRIS2 specification about `CanQuit`][can_quit] and the [`quit`](Self::quit) method.
    ///
    /// [can_quit]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:CanQuit
    pub fn can_quit(&self) -> Result<bool, DBusError> {
        self.connection_path().can_quit().map_err(|e| e.into())
    }

    /// Queries the player to see if it can be asked to entrer fullscreen.
    ///
    /// This property was added in MPRIS 2.2, and not all players will implement it. This method
    /// will try to detect this case and fall back to `Ok(false)`.
    ///
    /// It is up to you to decide if you want to ignore errors caused by this method or not.
    ///
    /// See: [MPRIS2 specification about `CanSetFullscreen`][can_full] and the [`set_fullscreen`](Self::set_fullscreen) method.
    ///
    /// [can_full]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:CanSetFullscreen
    pub fn can_set_fullscreen(&self) -> Result<bool, DBusError> {
        handle_optional_property(self.connection_path().can_set_fullscreen())
            .map(|o| o.unwrap_or(false))
    }

    /// Queries the player to see if it can be controlled or not.
    ///
    /// See: [MPRIS2 specification about `CanControl`][can_control].
    ///
    /// [can_control]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl
    pub fn can_control(&self) -> Result<bool, DBusError> {
        self.connection_path().can_control().map_err(|e| e.into())
    }

    /// Queries the player to see if it can go to next or not.
    ///
    /// See: [MPRIS2 specification about `CanGoNext`][can_next].
    ///
    /// [can_next]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoNext
    pub fn can_go_next(&self) -> Result<bool, DBusError> {
        self.connection_path().can_go_next().map_err(|e| e.into())
    }

    /// Queries the player to see if it can go to previous or not.
    ///
    /// See: [MPRIS2 specification about `CanGoPrevious`][can_prev].
    ///
    /// [can_prev]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoPrevious
    pub fn can_go_previous(&self) -> Result<bool, DBusError> {
        self.connection_path()
            .can_go_previous()
            .map_err(|e| e.into())
    }

    /// Queries the player to see if it can pause.
    ///
    /// See: [MPRIS2 specification about `CanPause`][can_pause]
    ///
    /// [can_pause]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPause
    pub fn can_pause(&self) -> Result<bool, DBusError> {
        self.connection_path().can_pause().map_err(|e| e.into())
    }

    /// Queries the player to see if it can play.
    ///
    /// See: [MPRIS2 specification about `CanPlay`][can_play].
    ///
    /// [can_play]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPlay
    pub fn can_play(&self) -> Result<bool, DBusError> {
        self.connection_path().can_play().map_err(|e| e.into())
    }

    /// Queries the player to see if it can seek within the media.
    ///
    /// See: [MPRIS2 specification about `CanSeek`][can_seek].
    ///
    /// [can_seek]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanSeek
    pub fn can_seek(&self) -> Result<bool, DBusError> {
        self.connection_path().can_seek().map_err(|e| e.into())
    }

    /// Queries the player to see if it can stop.
    ///
    /// MPRIS2 defines [the `Stop` message to only work when the player can be controlled][can_stop], so that
    /// is the property used for this method.
    ///
    /// See: [MPRIS2 specification about `CanControl`][can_control].
    ///
    /// [can_stop]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop
    /// [can_control]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl
    pub fn can_stop(&self) -> Result<bool, DBusError> {
        self.can_control()
    }

    /// Queries the player to see if it currently supports/allows changing playback rate.
    pub fn can_set_playback_rate(&self) -> Result<bool, DBusError> {
        self.get_valid_playback_rate_range()
            .map(|range| range.start < 1.0 || range.end > 1.0)
    }

    /// Queries the player to see if it supports the "Shuffle" setting
    pub fn can_shuffle(&self) -> Result<bool, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        self.connection_path()
            .get_all("org.mpris.MediaPlayer2.Player")
            .map(|props| props.contains_key("Shuffle"))
            .map_err(DBusError::from)
    }

    /// Queries the player to see if it supports the "LoopStatus" setting
    pub fn can_loop(&self) -> Result<bool, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        self.connection_path()
            .get_all("org.mpris.MediaPlayer2.Player")
            .map(|props| props.contains_key("LoopStatus"))
            .map_err(DBusError::from)
    }

    /// Queries the player to see if it supports the "Rate" setting
    pub fn has_playback_rate(&self) -> Result<bool, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        self.connection_path()
            .get_all("org.mpris.MediaPlayer2.Player")
            .map(|props| props.contains_key("Rate"))
            .map_err(DBusError::from)
    }

    /// Queries the player to see if it supports the "Position" setting
    pub fn has_position(&self) -> Result<bool, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        self.connection_path()
            .get_all("org.mpris.MediaPlayer2.Player")
            .map(|props| props.contains_key("Position"))
            .map_err(DBusError::from)
    }

    /// Queries the player to see if it supports the "Volume" setting
    pub fn has_volume(&self) -> Result<bool, DBusError> {
        use dbus::ffidisp::stdintf::org_freedesktop_dbus::Properties;

        self.connection_path()
            .get_all("org.mpris.MediaPlayer2.Player")
            .map(|props| props.contains_key("Volume"))
            .map_err(DBusError::from)
    }

    /// Query the player for current fullscreen state.
    ///
    /// This property was added in MPRIS 2.2, and not all players will implement it. This method
    /// will try to detect this case and fall back to `Ok(None)`.
    ///
    /// It is up to you to decide if you want to ignore errors caused by this method or not.
    ///
    /// See: [MPRIS2 specification about `Fullscreen`][full] and the [`can_set_fullscreen`](Self::can_set_fullscreen) method.
    ///
    /// [full]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Fullscreen
    pub fn get_fullscreen(&self) -> Result<Option<bool>, DBusError> {
        handle_optional_property(self.connection_path().fullscreen())
    }

    /// Asks the player to change fullscreen state.
    ///
    /// If method call succeeded, `Ok(true)` will be returned.
    ///
    /// This property was added in MPRIS 2.2, and not all players will implement it. This method
    /// will try to detect this case and fall back to `Ok(false)`.
    ///
    /// Other errors will be returned as [`Err`].
    ///
    /// See: [MPRIS2 specification about `Fullscreen`][full] and the [`can_set_fullscreen`](Self::can_set_fullscreen) method.
    ///
    /// [full]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Fullscreen
    pub fn set_fullscreen(&self, new_state: bool) -> Result<bool, DBusError> {
        handle_optional_property(self.connection_path().set_fullscreen(new_state))
            .map(|o| o.is_some())
    }

    /// Query the player for current playback status.
    pub fn get_playback_status(&self) -> Result<PlaybackStatus, DBusError> {
        self.connection_path()
            .playback_status()?
            .parse()
            .map_err(DBusError::from)
    }

    /// Query player for the state of the "Shuffle" setting.
    ///
    /// See: [MPRIS2 specification about `Shuffle`][shuffle].
    ///
    /// [shuffle]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle
    pub fn get_shuffle(&self) -> Result<bool, DBusError> {
        self.connection_path().shuffle().map_err(DBusError::from)
    }

    /// Gets the "Shuffle" setting, if the player indicates that it supports it.
    ///
    /// Return [`Some`] containing the current value of the shuffle setting. If the setting is not
    /// supported, will return [`None`]
    pub fn checked_get_shuffle(&self) -> Result<Option<bool>, DBusError> {
        if self.can_shuffle()? {
            Ok(Some(self.get_shuffle()?))
        } else {
            Ok(None)
        }
    }

    /// Set the "Shuffle" setting of the player.
    ///
    /// See: [MPRIS2 specification about `Shuffle`][shuffle].
    ///
    /// [shuffle]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle
    pub fn set_shuffle(&self, state: bool) -> Result<(), DBusError> {
        self.connection_path()
            .set_shuffle(state)
            .map_err(DBusError::from)
    }

    /// Set the "Shuffle" setting of the player, if the player indicates that it supports the
    /// "Shuffle" setting and can be controlled.
    ///
    /// Returns a [`bool`] to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Shuffle`][shuffle].
    ///
    /// [shuffle]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle
    pub fn checked_set_shuffle(&self, state: bool) -> Result<bool, DBusError> {
        if self.can_control()? && self.can_shuffle()? {
            self.set_shuffle(state)
                .map(|_| true)
                .map_err(DBusError::from)
        } else {
            Ok(false)
        }
    }

    /// Query the player for the current loop status.
    ///
    /// See: [MPRIS2 specification about  `LoopStatus`][loop_status].
    ///
    /// [loop_status]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus
    pub fn get_loop_status(&self) -> Result<LoopStatus, DBusError> {
        self.connection_path()
            .loop_status()?
            .parse()
            .map_err(DBusError::from)
    }

    /// Gets the "LoopStatus" setting, if the player indicates that it supports it.
    ///
    /// Returns [`Some`] containing the current value of the loop setting. If the setting is not
    /// supported, returns [`None`]
    pub fn checked_get_loop_status(&self) -> Result<Option<LoopStatus>, DBusError> {
        if self.can_loop()? {
            Ok(Some(self.get_loop_status()?))
        } else {
            Ok(None)
        }
    }

    /// Set the loop status of the player.
    ///
    /// See: [MPRIS2 specification about  `LoopStatus`][loop_status].
    ///
    /// [loop_status]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus
    pub fn set_loop_status(&self, status: LoopStatus) -> Result<(), DBusError> {
        self.connection_path()
            .set_loop_status(status.dbus_value())
            .map_err(DBusError::from)
    }

    /// Set the loop status of the player, if the player indicates that supports it and that it can
    /// be controlled.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about  `LoopStatus`][loop_status].
    ///
    /// [loop_status]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus
    pub fn checked_set_loop_status(&self, status: LoopStatus) -> Result<bool, DBusError> {
        if self.can_control()? && self.can_loop()? {
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
    /// See: [MPRIS2 specification about `Volume`][vol].
    ///
    /// [vol]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume
    pub fn get_volume(&self) -> Result<f64, DBusError> {
        self.connection_path().volume().map_err(DBusError::from)
    }

    /// Gets the "Volume" setting, if the player indicates that it supports it.
    ///
    /// Returns [`Some`] containing the current value of the position. If the setting is not
    /// supported, returns [`None`]
    pub fn checked_get_volume(&self) -> Result<Option<f64>, DBusError> {
        if self.has_volume()? {
            Ok(Some(self.get_volume()?))
        } else {
            Ok(None)
        }
    }

    /// Set the volume of the player.
    ///
    /// Volume should be between 0.0 and 1.0. Above 1.0 is possible, but not
    /// recommended.
    ///
    /// See: [MPRIS2 specification about `Volume`][vol].
    ///
    /// [vol]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume
    pub fn set_volume(&self, value: f64) -> Result<(), DBusError> {
        self.connection_path()
            .set_volume(value.max(0.0))
            .map_err(DBusError::from)
    }

    /// Set the "Volume" setting of the player, if the player indicates that it supports the
    /// "Volume" setting and can be controlled.
    ///
    /// Returns a boolean to show if the signal was sent or not.
    ///
    /// See: [MPRIS2 specification about `Volume`][vol].
    ///
    /// [vol]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume
    pub fn checked_set_volume(&self, volume: f64) -> Result<bool, DBusError> {
        if self.can_control()? && self.has_volume()? {
            self.set_volume(volume)
                .map(|_| true)
                .map_err(DBusError::from)
        } else {
            Ok(false)
        }
    }

    /// Set the volume of the player, if the player indicates that it can be
    /// controlled.
    ///
    /// Volume should be between 0.0 and 1.0. Above 1.0 is possible, but not
    /// recommended.
    ///
    /// See: [MPRIS2 specification about `Volume`][vol].
    ///
    /// [vol]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume
    pub fn set_volume_checked(&self, value: f64) -> Result<bool, DBusError> {
        if self.can_control()? {
            self.set_volume(value).map(|_| true)
        } else {
            Ok(false)
        }
    }

    fn connection_path(&self) -> ConnPath<'_, &Connection> {
        self.connection.with_path(
            self.bus_name.as_str().into(),
            MPRIS2_PATH.into(),
            self.timeout_ms,
        )
    }

    /// Blocks until player gets an event on the bus.
    ///
    /// Other player events will also be recorded, but will not cause this function to return. Note
    /// that this will block forever if player is not running. Make sure to check that the player
    /// is running before calling this method!
    pub(crate) fn process_events_blocking_until_received(&self) {
        while !self.connection.has_pending_events(&self.unique_name) {
            self.connection.process_events_blocking_until_received();
        }
    }

    /// Return any events that are pending (for this player) on the connection.
    pub(crate) fn pending_events(&self) -> Vec<MprisEvent> {
        self.connection.pending_events(&self.unique_name)
    }
}

fn handle_optional_property<T>(result: Result<T, dbus::Error>) -> Result<Option<T>, DBusError> {
    if let Err(ref error) = result {
        if let Some(error_name) = error.name() {
            if error_name == "org.freedesktop.DBus.Error.InvalidArgs" {
                // This property was likely just missing, which means that the player has not
                // implemented it.
                return Ok(None);
            }
        }
    }

    result.map(Some).map_err(|e| e.into())
}

/// Checks if the Player implements the `org.mpris.MediaPlayer2.TrackList` interface.
fn has_tracklist_interface(connection: ConnPath<'_, &Connection>) -> Result<bool, DBusError> {
    // Get the introspection XML and look for the substring instead of parsing the XML. Yeah,
    // pretty dirty, but it's also a lot faster and doesn't require a huge XML library as a
    // dependency either.
    //
    // It's probably accurate enough.

    use dbus::ffidisp::stdintf::OrgFreedesktopDBusIntrospectable;
    let xml: String = connection.introspect()?;
    Ok(xml.contains("org.mpris.MediaPlayer2.TrackList"))
}

use std::collections::HashMap;

use futures_util::{join, try_join};
use zbus::{names::BusName, Connection};

use crate::{
    metadata::{MetadataValue, RawMetadata},
    playlist::PlaylistsInterface,
    proxies::{DBusProxy, MediaPlayer2Proxy, PlayerProxy, TrackListProxy},
    LoopStatus, Metadata, Mpris, MprisDuration, MprisError, PlaybackStatus, Playlist,
    PlaylistOrdering, TrackID, MPRIS2_PREFIX,
};

/// Struct that represents a player connected to the D-Bus server. Can be used to query and control
/// a player.
///
/// The easiest way to create a [`Player`] is to use one of [`Mpris`][crate::Mpris]'s find methods,
/// see the [crate documentation][crate#getting-started] for examples.
///
/// # Bus Name
///
/// ## Well-know
///
/// The [`Player`] created through [`Mpris`][crate::Mpris] is bound to it's "well-known" Bus Name
/// and not it's "unique" Bus Name (see [here][bus] for details) which means that an instance of
/// [`Player`] can still be used even after the player restarts.
/// ```no_run
/// use mpris::Mpris;
///
/// #[async_std::main]
/// async fn main() {
///     let mpris = Mpris::new().await.unwrap();
///     let vlc = mpris
///         .find_by_name("VLC media player", false)
///         .await
///         .unwrap()
///         .unwrap();
///     // VLC restarts here
///     vlc.play().await.expect("will not panic");
/// }
/// ```
///
/// ## Unique
///
/// If you want to create a [`Player`] that's bound to a unique name you can use
/// [`new()`][Player::new] directly. In that case the instance of [`Player`] will stop working if
/// the player disconnects from the D-Bus for any reason.
/// ```no_run
/// use mpris::{Mpris, Player};
/// use zbus::names::BusName;
/// #[async_std::main]
/// async fn main() {
///     let mpris = Mpris::new().await.unwrap();
///     // A "unique" Bus Name
///     let unique_name = BusName::try_from(":1.123").unwrap();
///     let vlc = Player::new(&mpris, unique_name)
///         .await
///         .unwrap();
///   
///     // VLC restarts here
///     vlc.play().await.expect("will panic")
/// }
/// ```
///
/// # Interfaces
///
/// The [MPRIS specification][spec] contains 4 interfaces:
///
/// ## [org.mpris.MediaPlayer2][mp2]
///
/// <details> <summary>Index of methods for this interface</summary>
///
/// ### Methods
/// - `Raise`: [`raise()`][Self::raise]
/// - `Quit`: [`quit()`][Self::quit]
///
/// ### Properties
/// - `CanQuit`: [`can_quit()`][Self::can_quit]
/// - `CanRaise`: [`can_raise()`][Self::can_raise]
/// - `DesktopEntry`: [`desktop_entry()`][Self::desktop_entry]
/// - `Fullscreen`: [`get_fullscreen()`][Self::get_fullscreen] /
///   [`set_fullscreen()`][Self::set_fullscreen]
/// - `CanSetFullscreen`: [`can_set_fullscreen()`][Self::can_set_fullscreen]
/// - `HasTrackList`: [`has_track_list()`][Self::has_track_list]
/// - `Identify`: [`identity()`][Self::identity]
/// - `SupportedMimeTypes`: [`supported_mime_types()`][Self::supported_mime_types]
/// - `SupportedUriSchemes`: [`supported_uri_schemes()`][Self::supported_uri_schemes]
///
/// </details>
///
/// ## [org.mpris.MediaPlayer2.Player][player]
///
/// > This interface implements the methods for querying and providing basic control over what is
/// > currently playing.
///
/// <details> <summary>Index of methods for this interface</summary>
///
/// ### Methods
/// - `Next`: [`next()`][Self::next]
/// - `Previous`: [`previous()`][Self::previous]
/// - `Play`: [`play()`][Self::play]
/// - `Pause`: [`pause()`][Self::pause]
/// - `PlayPause`: [`play_pause()`][Self::play_pause]
/// - `Stop`: [`stop()`][Self::stop]
/// - `Seek`: [`seek()`][Self::seek] / [`seek_forwards()`][Self::seek_forwards] /
///   [`seek_backwards()`][Self::seek_backwards]
/// - `SetPosition`: [`set_position()`][Self::set_position]
/// - `OpenUri`: [`open_uri()`][Self::open_uri]
///
/// ### Properties
/// - `CanControl`: [`can_control()`][Self::can_control]
/// - `CanGoNext`: [`can_go_next()`][Self::can_go_next]
/// - `CanGoPrevious`: [`can_go_previous()`][Self::can_go_previous]
/// - `CanPlay`: [`can_play()`][Self::can_play]
/// - `CanPause`: [`can_pause()`][Self::can_pause]
/// - `CanSeek`: [`can_seek()`][Self::can_seek]
/// - `LoopStatus`: [`get_loop_status()`][Self::get_loop_status] /
///   [`set_loop_status()`][Self::set_loop_status]
/// - `MaximumRate`: [`maximum_rate()`][Self::maximum_rate]
/// - `MinimumRate`: [`minimum_rate()`][Self::minimum_rate]
/// - `Metadata`: [`metadata()`][Self::metadata] / [`raw_metadata()`][Self::raw_metadata]
/// - `PlaybackStatus`: [`playback_status()`][Self::playback_status]
/// - `Position`: [`get_position()`][Self::get_position]
/// - `Rate`: [`get_playback_rate()`][Self::get_playback_rate] /
///   [`set_playback_rate()`][Self::set_playback_rate]
/// - `Shuffle`: [`get_shuffle()`][Self::get_shuffle] / [`set_shuffle()`][Self::set_shuffle]
/// - `Volume`: [`get_volume()`][Self::get_volume] / [`set_volume()`][Self::set_volume]
///
/// </details>
///
/// ## [org.mpris.MediaPlayer2.TrackList][tracklist]
///
/// **This is an optional interface.**
///
/// > Provides access to a short list of tracks which were recently played or will be played
/// > shortly. This is intended to provide context to the currently-playing track, rather than
/// > giving complete access to the media player's playlist.
/// >
/// > Example use cases are the list of tracks from the same album as the currently playing song or
/// > the Rhythmbox play queue.
/// >
/// > Each track in the tracklist has a unique identifier. The intention is that this uniquely
/// > identifies the track within the scope of the tracklist. In particular, if a media item (a
/// > particular music file, say) occurs twice in the track list, each occurrence should have a
/// > different identifier. If a track is removed from the middle of the playlist, it should not
/// > affect the track ids of any other tracks in the tracklist.
/// >
/// > As a result, the traditional track identifiers of URLs and position in the playlist cannot be
/// > used. Any scheme which satisfies the uniqueness requirements is valid, as clients should not
/// > make any assumptions about the value of the track id beyond the fact that it is a unique
/// > identifier.
/// >
/// > Note that the (memory and processing) burden of implementing the TrackList interface and
/// > maintaining unique track ids for the playlist can be mitigated by only exposing a subset of
/// > the playlist when it is very long (the 20 or so tracks around the currently playing track, for
/// > example). This is a recommended practice as the tracklist interface is not designed to enable
/// > browsing through a large list of tracks, but rather to provide clients with context about the
/// > currently playing track.
///
/// <details> <summary>Index of methods for this interface</summary>
///
/// ### Methods
/// - `AddTrack`: [`add_track()`][Self::add_track]
/// - `GetTracksMetadata`: [`get_tracks_metadata()`][Self::get_tracks_metadata]
/// - `GoTo`: [`go_to()`][Self::go_to]
/// - `RemoveTrack`: [`remove_track()`][Self::remove_track]
///
/// ### Properties
/// - `CanEditTracks`: [`can_edit_tracks()`][Self::can_edit_tracks]
/// - `Tracks`: [`tracks()`][Self::tracks]
///
/// </details>
///
/// ## [org.mpris.MediaPlayer2.Playlists][playlists]
///
/// **This is an optional interface.**
///
/// > Provides access to the media player's playlists.
///
/// <details> <summary>Index of methods for this interface</summary>
///
/// ### Methods
/// - `ActivatePlaylist`: [`activate_playlist()`][Self::activate_playlist]
/// - `GetPlaylists`: [`get_playlists()`][Self::get_playlists]
///
/// ### Properties
/// - `ActivePlaylist`: [`active_playlist()`][Self::active_playlist]
/// - `Orderings`: [`orderings()`][Self::orderings]
/// - `Orderings`: [`playlist_count()`][Self::playlist_count]
///
/// </details>
///
/// ## Support checking
///
/// Interfaces are checked by calling one of their methods only when [`Player`] gets created. This
/// means that if for some reason the player implements an interface at a later point the created
/// [`Player`] will not know about it.
///
/// Support for the 2 optional interfaces can be checked with:
/// - [`supports_track_list_interface()`][Self::supports_track_list_interface]
/// - [`supports_playlists_interface()`][Self::supports_playlists_interface]
///
/// <div class="warning">Note that just because an interface is implemented it doesn't mean that the
/// player implemented it correctly, even in the case of the required ones. Methods might be missing
/// or it might return wrong data. This library is not trying to correct those mistakes and will
/// simply return an error.</div>
///
/// [spec]: https://specifications.freedesktop.org/mpris-spec/latest
/// [mp2]: https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html
/// [player]: https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html
/// [tracklist]: https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html
/// [playlists]: https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html
/// [bus]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-names-bus
#[derive(Clone)]
pub struct Player {
    bus_name: BusName<'static>,
    dbus_proxy: DBusProxy<'static>,
    mp2_proxy: MediaPlayer2Proxy<'static>,
    player_proxy: PlayerProxy<'static>,
    track_list_proxy: Option<TrackListProxy<'static>>,
    playlist_interface: Option<PlaylistsInterface>,
}

impl Player {
    /// Creates a new [`Player`] for the given [`Connection`] and [`BusName`].
    ///
    /// In most cases there is no need to create [`Player`]s directly, instead you should create
    /// them through [`Mpris`][crate::Mpris]. Doing it this way however allows you to bind the
    /// [`Player`] to a unique Bus Name. See [this][Self#bus-name] for a simple explanation.
    pub async fn new(mpris: &Mpris, bus_name: BusName<'static>) -> Result<Player, MprisError> {
        Self::new_internal(mpris.get_connection(), mpris.dbus_proxy.clone(), bus_name).await
    }

    pub(crate) async fn new_internal(
        conn: Connection,
        dbus_proxy: DBusProxy<'static>,
        bus_name: BusName<'static>,
    ) -> Result<Self, MprisError> {
        let (mp2_proxy, player_proxy, track_list_proxy, playlists_interface) = try_join!(
            MediaPlayer2Proxy::new(&conn, bus_name.clone()),
            PlayerProxy::new(&conn, bus_name.clone()),
            TrackListProxy::new(&conn, bus_name.clone()),
            PlaylistsInterface::new(&conn, bus_name.clone()),
        )?;

        let (track_list, playlists) = join!(
            track_list_proxy.can_edit_tracks(),
            playlists_interface.playlist_count(),
        );

        Ok(Player {
            bus_name,
            dbus_proxy,
            mp2_proxy,
            player_proxy,
            track_list_proxy: if track_list.is_ok() {
                Some(track_list_proxy)
            } else {
                None
            },
            playlist_interface: if playlists.is_ok() {
                Some(playlists_interface)
            } else {
                None
            },
        })
    }

    /// Returns the player's [`HasTrackList`][track_list] property.
    ///
    /// **Note**: this property is reported by the player and might not be accurate. A better way to
    /// check is [`supports_track_list_interface()`][Self::supports_track_list_interface]
    ///
    /// [track_list]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:HasTrackList
    pub async fn has_track_list(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.has_track_list().await?)
    }

    /// Checks if the [`Player`] has support for the
    /// [`TrackList`][Self#orgmprismediaplayer2tracklist] interface.
    ///
    /// See the [interfaces section for more details][Self#interfaces]
    pub fn supports_track_list_interface(&self) -> bool {
        self.track_list_proxy.is_some()
    }

    /// Checks if the [`Player`] has support for the
    /// [`Playlists`][Self#orgmprismediaplayer2playlists] interface.
    ///
    /// See the [interfaces section for more details][Self#interfaces]
    pub fn supports_playlists_interface(&self) -> bool {
        self.playlist_interface.is_some()
    }

    /// Queries the player for current metadata.
    ///
    /// See [`Metadata`] for more information.
    pub async fn metadata(&self) -> Result<Metadata, MprisError> {
        Ok(self.raw_metadata().await?.try_into()?)
    }

    /// Queries the player for current metadata and returns it as is.
    ///
    /// Similar to [`metadata()`][Self::metadata] but doesn't perform any checks or conversions. See
    /// [`Metadata::from_raw_lossy()`] for details.
    pub async fn raw_metadata(&self) -> Result<RawMetadata, MprisError> {
        let data = self.player_proxy.metadata().await?;
        let raw: HashMap<String, MetadataValue> =
            data.into_iter().map(|(k, v)| (k, v.into())).collect();
        Ok(raw)
    }

    /// Checks if the player is still connected.
    ///
    /// Simply pings the player and checks if it responds.
    ///
    /// This could return [`Err`] even if the player is connected but something with the connection
    /// went wrong.
    pub async fn is_running(&self) -> Result<bool, MprisError> {
        match self.mp2_proxy.ping().await {
            Ok(_) => Ok(true),
            Err(e) => match e {
                zbus::Error::MethodError(e_name, _, _)
                    if e_name == "org.freedesktop.DBus.Error.ServiceUnknown" =>
                {
                    Ok(false)
                }
                _ => Err(e.into()),
            },
        }
    }

    /// Returns the Bus Name of the [`Player`].
    ///
    /// See also: [`bus_name_trimmed()`][Self::bus_name_trimmed] and
    /// [`unique_bus_name()`][Self::unique_bus_name].
    pub fn bus_name(&self) -> &str {
        self.bus_name.as_str()
    }

    /// Returns the Unique Bus Name of the [`Player`].
    ///
    /// If it returns [`None`] then no player is currently connected.
    ///
    /// If you just want to check if the player is connected you should use
    /// [`is_running()`][Self::is_running] instead.
    ///
    /// See also: [`bus_name()`][Self::bus_name].
    pub async fn unique_bus_name(&self) -> Result<Option<String>, MprisError> {
        // If Player is bound to a unique name there's no need to check.
        if let BusName::Unique(unique_name) = &self.bus_name {
            Ok(Some(unique_name.to_string()))
        } else {
            match self.dbus_proxy.get_name_owner(&self.bus_name).await {
                Ok(name) => Ok(Some(name.to_string())),
                Err(e) => match e {
                    zbus::Error::MethodError(e_name, _, _)
                        if e_name == "org.freedesktop.DBus.Error.NameHasNoOwner" =>
                    {
                        Ok(None)
                    }
                    _ => Err(e.into()),
                },
            }
        }
    }

    /// Returns the player name part of the player's D-Bus bus name with the MPRIS2 prefix trimmed.
    ///
    /// Examples:
    /// - `org.mpris.MediaPlayer2.io.github.celluloid_player.Celluloid` ->
    ///   `io.github.celluloid_player.Celluloid`
    /// - `org.mpris.MediaPlayer2.Spotify.` -> `Spotify`
    /// - `org.mpris.MediaPlayer2.mpv.instance123` -> `mpv.instance123`
    ///
    /// See also: [`bus_name()`][Self::bus_name].
    pub fn bus_name_trimmed(&self) -> &str {
        self.bus_name().trim_start_matches(MPRIS2_PREFIX)
    }

    /// Sends a `Quit` signal to the player.
    ///
    /// > Causes the media player to stop running.
    /// >
    /// > The media player may refuse to allow clients to shut it down. In this case, the CanQuit
    /// > property is false and this method does nothing.
    ///
    /// See also: [MPRIS2 specification about `Quit`][quit] and [`can_quit()`][Self::can_quit].
    ///
    /// [quit]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Method:Quit
    pub async fn quit(&self) -> Result<(), MprisError> {
        Ok(self.mp2_proxy.quit().await?)
    }

    /// Queries the player to see if it can be asked to quit.
    ///
    ///  > If false, calling Quit will have no effect, and may raise a NotSupported error. If true,
    ///  > calling Quit will cause the media application to attempt to quit (although it may still
    ///  > be prevented from quitting by the user, for example).
    ///
    /// See also: [MPRIS2 specification about `CanQuit`][can_quit] and [`quit()`][Self::quit].
    ///
    /// [can_quit]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:CanQuit
    pub async fn can_quit(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.can_quit().await?)
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
    /// See also: [MPRIS2 specification about `Raise`][raise] and [`can_raise()`][Self::can_raise].
    ///
    /// [raise]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Method:Raise
    pub async fn raise(&self) -> Result<(), MprisError> {
        Ok(self.mp2_proxy.raise().await?)
    }

    /// Queries the player to see if it can be raised or not.
    ///
    /// > If false, calling Raise will have no effect, and may raise a NotSupported error. If true,
    /// > calling Raise will cause the media application to attempt to bring its user interface to
    /// > the front, although it may be prevented from doing so (by the window manager, for
    /// > example).
    ///
    /// See also: [MPRIS2 specification about `CanRaise`][can_raise] and [`raise()`][Self::raise]
    ///
    /// [can_raise]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:CanRaise
    pub async fn can_raise(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.can_raise().await?)
    }
    /// Returns the player's [`DesktopEntry`][entry] property, if supported.
    ///
    /// > The basename of an installed .desktop file which complies with the Desktop entry
    /// > specification, with the ".desktop" extension stripped.
    /// >
    /// > Example: The desktop entry file is "/usr/share/applications/vlc.desktop", and this
    /// > property contains "vlc"
    ///
    /// [entry]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:DesktopEntry
    pub async fn desktop_entry(&self) -> Result<String, MprisError> {
        Ok(self.mp2_proxy.desktop_entry().await?)
    }

    /// Returns the player's MPRIS [`Identity`][identity].
    ///
    /// > A friendly name to identify the media player to users.
    /// >
    /// > This should usually match the name found in .desktop files
    /// >
    /// > (eg: "VLC media player").
    ///
    /// [identity]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Identity
    pub async fn identity(&self) -> Result<String, MprisError> {
        Ok(self.mp2_proxy.identity().await?)
    }

    /// Returns the player's [`SupportedMimeTypes`][mime] property.
    ///
    /// > The mime-types supported by the media player.
    /// >
    /// > Mime-types should be in the standard format (eg: audio/mpeg or application/ogg).
    ///
    /// [mime]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:SupportedMimeTypes
    pub async fn supported_mime_types(&self) -> Result<Vec<String>, MprisError> {
        Ok(self.mp2_proxy.supported_mime_types().await?)
    }

    /// Returns the player's [`SupportedUriSchemes`][uri] property.
    ///
    /// > The URI schemes supported by the media player.
    /// >
    /// > This can be viewed as protocols supported by the player in almost all cases. Almost every
    /// > media player will include support for the "file" scheme. Other common schemes are "http"
    /// > and "rtsp".
    /// >
    /// > Note that URI schemes should be lower-case.
    ///
    /// [uri]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:SupportedUriSchemes
    pub async fn supported_uri_schemes(&self) -> Result<Vec<String>, MprisError> {
        Ok(self.mp2_proxy.supported_uri_schemes().await?)
    }

    /// Returns the player's [`Fullscreen`][full] property.
    ///
    /// <div class="warning">This property was added in MPRIS 2.2, and not all players will
    /// implement it.</div>
    ///
    /// > Whether the media player is occupying the fullscreen.
    /// >
    /// > This is typically used for videos. A value of true indicates that the media player is
    /// > taking up the full screen.
    /// >
    /// > Media centre software may well have this value fixed to true
    ///
    /// See also: [`set_fullscreen()`][Self::set_fullscreen] and
    /// [`can_set_fullscreen()`][Self::can_set_fullscreen].
    ///
    /// [full]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Fullscreen
    pub async fn get_fullscreen(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.fullscreen().await?)
    }

    /// Asks the player to set the [`Fullscreen`][full] property.
    ///
    /// <div class="warning">This property was added in MPRIS 2.2, and not all players will
    /// implement it.</div>
    ///
    /// See [`get_fullscreen`][Self::get_fullscreen()] and
    /// [`can_set_fullscreen()`][Self::can_set_fullscreen] for more information
    ///
    /// [full]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:Fullscreen
    pub async fn set_fullscreen(&self, value: bool) -> Result<(), MprisError> {
        Ok(self.mp2_proxy.set_fullscreen(value).await?)
    }

    /// Queries the player to see if it can be asked to enter fullscreen.
    ///
    /// <div class="warning">This property was added in MPRIS 2.2, and not all players will
    /// implement it.</div>
    ///
    /// > If false, attempting to set Fullscreen will have no effect, and may raise an error. If
    /// > true, attempting to set Fullscreen will not raise an error, and (if it is different from
    /// > the current value) will cause the media player to attempt to enter or exit fullscreen
    /// > mode.
    /// >
    /// > Note that the media player may be unable to fulfil the request. In this case, the value
    /// > will not change. If the media player knows in advance that it will not be able to fulfil
    /// > the request, however, this property should be false.
    ///
    /// See also: [MPRIS2 specification about `CanSetFullscreen`][can_full],
    /// [`get_fullscreen()`][Self::get_fullscreen] and [`set_fullscreen()`][Self::set_fullscreen].
    ///
    /// [can_full]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html#Property:CanSetFullscreen
    pub async fn can_set_fullscreen(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.can_set_fullscreen().await?)
    }

    /// Queries the player to see if it can be controlled or not.
    ///
    /// > Whether the media player may be controlled over this interface.
    /// >
    /// > This property is not expected to change, as it describes an intrinsic capability of the
    /// > implementation.
    /// >
    /// > If this is false, clients should assume that all properties on the
    /// > [`org.mpris.MediaPlayer2.Player`][Self#orgmprismediaplayer2player] interface are read-only
    /// > (and will raise errors if writing to them is attempted), no methods are implemented and
    /// > all other properties starting with "Can" are also false.
    ///
    /// See also: [MPRIS2 specification about `CanControl`][control].
    ///
    /// [control]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanControl
    pub async fn can_control(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_control().await?)
    }

    /// Sends a [`Next`][next] signal to the player.
    ///
    /// > Skips to the next track in the tracklist.
    /// >
    /// > If there is no next track (and endless playback and track repeat are both off), stop
    /// > playback.
    /// >
    /// > If playback is paused or stopped, it remains that way.
    /// >
    /// > If CanGoNext is false, attempting to call this method should have no effect.
    ///
    /// See also: [`can_go_next()`][Self::can_go_next].
    ///
    /// [next]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Next
    pub async fn next(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.next().await?)
    }

    /// Queries the player to see if it can go to next.
    ///
    /// > Whether the client can call the Next method on this interface and expect the current track
    /// > to change.
    /// >
    /// > If it is unknown whether a call to Next will be successful (for example, when streaming
    /// > tracks), this property should be set to true.
    /// >
    /// > If CanControl is false, this property should also be false.
    ///
    /// See also: [MPRIS2 specification about `CanGoNext`][can_next] and [`next()`][Self::next].
    ///
    /// [can_next]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoNext
    pub async fn can_go_next(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_go_next().await?)
    }

    /// Sends a [`Previous`][prev] signal to the player.
    ///
    /// > Skips to the previous track in the tracklist.
    /// >
    /// > If there is no previous track (and endless playback and track repeat are both off), stop
    /// > playback.
    /// >
    /// > If playback is paused or stopped, it remains that way.
    /// >
    /// > If CanGoPrevious is false, attempting to call this method should have no effect.
    ///
    /// See also: [`can_go_previous()`][Self::can_go_previous].
    ///
    /// [prev]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Previous
    pub async fn previous(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.previous().await?)
    }

    /// Queries the player to see if it can go to previous or not.
    ///
    /// > Whether the client can call the Previous method on this interface and expect the current
    /// > track to change.
    /// >
    /// > If it is unknown whether a call to Previous will be successful (for example, when
    /// > streaming tracks), this property should be set to true.
    /// >
    /// > If CanControl is false, this property should also be false.
    ///
    /// See also: [MPRIS2 specification about `CanGoPrevious`][can_prev] and [`previous()`][Self::previous].
    ///
    /// [can_prev]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanGoPrevious
    pub async fn can_go_previous(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_go_previous().await?)
    }

    /// Sends a [`Play`][play] signal to the player.
    ///
    /// > Starts or resumes playback.
    /// >
    /// > If already playing, this has no effect.
    /// >
    /// > If paused, playback resumes from the current position.
    /// >
    /// > If there is no track to play, this has no effect.
    /// >
    /// > If CanPlay is false, attempting to call this method should have no effect.
    ///
    /// See also: [`can_play()`][Self::can_play].
    ///
    /// [play]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Play
    pub async fn play(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.play().await?)
    }

    /// Queries the player to see if it can play.
    ///
    /// > Whether playback can be started using Play or PlayPause.
    /// >
    /// > Note that this is related to whether there is a "current track": the value should not depend on whether the track is currently paused or playing. In fact, if a track is currently playing (and CanControl is true), this should be true.
    /// >
    /// > If CanControl is false, this property should also be false.
    ///
    /// See also: [MPRIS2 specification about `CanPlay`][can_play] and [`play()`][Self::play].
    ///
    /// [can_play]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPlay
    pub async fn can_play(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_play().await?)
    }

    /// Sends a [`Pause`][pause] signal to the player.
    ///
    /// > Pauses playback.
    /// >
    /// > If playback is already paused, this has no effect.
    /// >
    /// > Calling Play after this should cause playback to start again from the same position.
    /// >
    /// > If CanPause is false, attempting to call this method should have no effect.
    ///
    /// See also: [`can_pause()`][Self::can_pause].
    ///
    /// [pause]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Pause
    pub async fn pause(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.pause().await?)
    }

    /// Queries the player to see if it can pause.
    ///
    /// > Whether playback can be paused using Pause or PlayPause.
    /// >
    /// > Note that this is an intrinsic property of the current track: its value should not depend
    /// > on whether the track is currently paused or playing. In fact, if playback is currently
    /// > paused (and CanControl is true), this should be true.
    /// >
    /// > If CanControl is false, this property should also be false.
    ///
    /// See also: [MPRIS2 specification about `CanPause`][can_pause] and [`pause()`][Self::pause].
    ///
    /// [can_pause]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanPause
    pub async fn can_pause(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_pause().await?)
    }

    /// Sends a [`PlayPause`][play_pause] signal to the player.
    ///
    /// > Pauses playback.
    /// >
    /// > If playback is already paused, resumes playback.
    /// >
    /// > If playback is stopped, starts playback.
    /// >
    /// > If CanPause is false, attempting to call this method should have no effect and raise an
    /// > error.
    ///
    /// See also: [`can_pause()`][Self::can_pause].
    ///
    /// [play_pause]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:PlayPause
    pub async fn play_pause(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.play_pause().await?)
    }

    /// Sends a [`Stop`][stop] signal to the player.
    ///
    /// > Stops playback.
    /// >
    /// > If playback is already stopped, this has no effect.
    /// >
    /// > Calling Play after this should cause playback to start again from the beginning of the
    /// > track.
    /// >
    /// > If CanControl is false, attempting to call this method should have no effect and raise an
    /// > error.
    ///
    /// [stop]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Stop
    pub async fn stop(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.stop().await?)
    }

    /// Sends a [`Seek`][seek] signal to the player.
    ///
    /// > Seeks forward in the current track by the specified number of microseconds.
    /// >
    /// > A negative value seeks back. If this would mean seeking back further than the start of the
    /// > track, the position is set to 0.
    /// >
    /// > If the value passed in would mean seeking beyond the end of the track, acts like a call to
    /// > Next.
    /// >
    /// > If the CanSeek property is false, this has no effect.
    ///
    /// See also: [`can_seek()`][Self::can_seek], [`seek_forwards()`][Self::seek_forwards] and
    /// [`seek_backwards()`][Self::seek_backwards].
    ///
    /// [seek]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:Seek
    pub async fn seek(&self, offset_in_microseconds: i64) -> Result<(), MprisError> {
        Ok(self.player_proxy.seek(offset_in_microseconds).await?)
    }

    /// Tells the player to seek forwards.
    ///
    /// Similar to [`seek()`][Self::seek] but can only seek forwards and uses the more convenient
    /// [`MprisDuration`] as an argument.
    ///
    /// See also: [`seek_backwards()`][Self::seek_backwards]
    pub async fn seek_forwards(&self, offset: MprisDuration) -> Result<(), MprisError> {
        Ok(self.player_proxy.seek(offset.into()).await?)
    }

    /// Tells the player to seek backwards.
    ///
    /// Similar to [`seek()`][Self::seek] but can only seek backwards and uses the more convenient
    /// [`MprisDuration`] as an argument.
    ///
    /// See also: [`seek_forwards()`][Self::seek_forwards]
    pub async fn seek_backwards(&self, offset: MprisDuration) -> Result<(), MprisError> {
        Ok(self.player_proxy.seek(-i64::from(offset)).await?)
    }

    /// Queries the player to see if it can seek within the media.
    ///
    /// > Whether the client can control the playback position using Seek and SetPosition. This may
    /// > be different for different tracks.
    /// >
    /// > If CanControl is false, this property should also be false.
    ///
    /// See also: [MPRIS2 specification about `CanSeek`][can_seek], [`seek()`][Self::seek],
    /// [`seek_forwards()`][Self::seek_forwards], [`seek_backwards()`][Self::seek_backwards].
    ///
    /// [can_seek]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:CanSeek
    pub async fn can_seek(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_seek().await?)
    }

    /// Gets the player's MPRIS [`Position`][position] as a [`MprisDuration`] since the start of the
    /// media.
    ///
    /// > The current track position in microseconds, between 0 and the 'mpris:length' metadata
    /// > entry (see [`Metadata`][Metadata]).
    ///
    /// [position]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Position
    pub async fn get_position(&self) -> Result<MprisDuration, MprisError> {
        Ok(self.player_proxy.position().await?.try_into()?)
    }

    /// Sets the position of the current track to the given position (as a [`MprisDuration`]).
    ///
    /// Current [`TrackID`] must be provided to avoid race conditions with the player, in case it
    /// changes tracks while the signal is being sent. The special
    /// [`"NoTrack"`][TrackID::NO_TRACK] value is not allowed.
    ///
    /// To obtain the current [`TrackID`] you can use [`metadata()`][Self::metadata]
    ///```no_run
    /// #[async_std::main]
    /// async fn main() {
    ///     use mpris::Mpris;
    ///     use std::time::Duration;
    ///
    ///     let mpris = Mpris::new().await.unwrap();
    ///     let player = mpris.find_active().await.unwrap().unwrap();
    ///     let track_id = player.metadata().await.unwrap().track_id.unwrap();
    ///     let _ = player
    ///         .set_position(&track_id, Duration::from_secs(5).try_into().unwrap())
    ///         .await;
    /// }
    ///```
    ///
    /// > If the CanSeek property is false, this has no effect.
    ///
    /// See also: [MPRIS2 specification about `SetPosition`][set_position] and
    /// [`can_seek()`][Self::can_seek].
    ///
    /// [set_position]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:SetPosition
    pub async fn set_position(
        &self,
        track_id: &TrackID,
        position: MprisDuration,
    ) -> Result<(), MprisError> {
        if track_id.is_no_track() {
            return Err(MprisError::track_id_is_no_track());
        }
        Ok(self
            .player_proxy
            .set_position(track_id.as_ref(), position.into())
            .await?)
    }

    /// Gets the player's current loop status.
    ///
    /// See also: [MPRIS2 specification about `LoopStatus`][loop_status] and [`LoopStatus`][LoopStatus].
    ///
    /// [loop_status]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus
    pub async fn get_loop_status(&self) -> Result<LoopStatus, MprisError> {
        Ok(self.player_proxy.loop_status().await?.parse()?)
    }

    /// Sets the loop status of the player.
    ///
    /// > If CanControl is false, attempting to set this property should have no effect and raise an
    /// > error.
    ///
    /// See also: [MPRIS2 specification about `LoopStatus`][loop_status] and [`LoopStatus`][LoopStatus].
    ///
    /// [loop_status]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:LoopStatus
    pub async fn set_loop_status(&self, loop_status: LoopStatus) -> Result<(), MprisError> {
        Ok(self
            .player_proxy
            .set_loop_status(loop_status.as_str())
            .await?)
    }

    /// Gets the player's current playback status.
    ///
    /// See also: [MPRIS2 specification about `PlaybackStatus`][playback] and
    /// [`PlaybackStatus`][PlaybackStatus].
    ///
    /// [playback]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:PlaybackStatus
    pub async fn playback_status(&self) -> Result<PlaybackStatus, MprisError> {
        Ok(self.player_proxy.playback_status().await?.parse()?)
    }

    /// Signals the player to open the given `uri`.
    ///
    /// The argument's uri scheme should be in
    /// [`supported_uri_schemes()`][Self::supported_uri_schemes] and the mime-type should be in
    /// [`supported_mime_types()`][Self::supported_mime_types] but note that this method does not
    /// check for that. It's up to you to verify that.
    ///
    /// > If the playback is stopped, starts playing
    /// >
    /// > If the uri scheme or the mime-type of the uri to open is not supported, this method does
    /// > nothing and may raise an error. In particular, if the list of available uri schemes is
    /// > empty, this method may not be implemented.
    /// >
    /// > Clients should not assume that the Uri has been opened as soon as this method returns.
    /// > They should wait until the mpris:trackid field in the Metadata property changes.
    /// >
    /// > If the media player implements the [TrackList
    /// > interface][Self#orgmprismediaplayer2tracklist], then the opened track should be made part
    /// > of the tracklist.
    ///
    /// See also: [MPRIS2 specification about `OpenUri`][uri].
    ///
    /// [uri]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Method:OpenUri
    pub async fn open_uri(&self, uri: &str) -> Result<(), MprisError> {
        Ok(self.player_proxy.open_uri(uri).await?)
    }

    /// Gets the minimum allowed value for playback rate.
    ///
    /// > The minimum value which the Rate property can take. Clients should not attempt to set the
    /// > Rate property below this value.
    /// >
    /// > Note that even if this value is 0.0 or negative, clients should not attempt to set the
    /// > Rate property to 0.0.
    /// >
    /// > This value should always be 1.0 or less.
    ///
    /// See also: [MPRIS2 specification about `MinimumRate`][min_rate] and
    /// [`set_playback_rate()`][Self::set_playback_rate].
    ///
    /// [min_rate]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:MinimumRate
    pub async fn maximum_rate(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.maximum_rate().await?)
    }

    /// Gets the maximum allowed value for playback rate.
    ///
    /// > The maximum value which the Rate property can take. Clients should not attempt to set the
    /// > Rate property above this value.
    /// >
    /// > This value should always be 1.0 or greater.
    ///
    /// See also: [MPRIS2 specification about `MaximumRate`][max_rate] and
    /// [`set_playback_rate()`][Self::set_playback_rate].
    ///
    /// [max_rate]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:MaximumRate
    pub async fn minimum_rate(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.minimum_rate().await?)
    }

    /// Returns the player's MPRIS (playback) [`rate`][rate] as a factor.
    ///
    /// 1.0 would mean normal rate, while 2.0 would mean twice the playback speed.
    ///
    /// See also: [`set_playback_rate()`][Self::set_playback_rate].
    ///
    /// [rate]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Rate
    pub async fn get_playback_rate(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.rate().await?)
    }

    /// Sets the player's MPRIS (playback) [`rate`][rate] as a factor.
    ///
    /// > The value must fall in the range described by [`MinimumRate`][Self::minimum_rate] and
    /// > [`MaximumRate`][Self::maximum_rate], and must not be 0.0.
    /// >
    /// > Not all values may be accepted by the media player. It is left to media player
    /// > implementations to decide how to deal with values they cannot use; they may either ignore
    /// > them or pick a "best fit" value. Clients are recommended to only use sensible fractions or
    /// > multiples of 1 (eg: 0.5, 0.25, 1.5, 2.0, etc).
    ///
    /// **Note**: this method does not check if the argument is between the minimum and maximum.
    ///
    /// See also: [`get_playback_rate()`][Self::get_playback_rate].
    ///
    /// [rate]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Rate
    pub async fn set_playback_rate(&self, rate: f64) -> Result<(), MprisError> {
        if rate == 0.0 {
            return Err(MprisError::InvalidArgument("rate can't be 0.0".to_string()));
        }
        Ok(self.player_proxy.set_rate(rate).await?)
    }

    /// Gets the player's [`Shuffle`][shuffle] property.
    ///
    /// >  A value of false indicates that playback is progressing linearly through a playlist,
    /// >  while true means playback is progressing through a playlist in some other order.
    ///
    /// See also: [`set_shuffle()`][Self::set_shuffle].
    ///
    /// [shuffle]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle
    pub async fn get_shuffle(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.shuffle().await?)
    }

    /// Sets the [`Shuffle`][shuffle] property of the player.
    ///
    /// > If CanControl is false, attempting to set this property should have no effect and raise an
    /// > error.
    ///
    /// See also: [`get_shuffle()`][Self::get_shuffle].
    ///
    /// [shuffle]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Shuffle
    pub async fn set_shuffle(&self, shuffle: bool) -> Result<(), MprisError> {
        Ok(self.player_proxy.set_shuffle(shuffle).await?)
    }

    /// Gets the [`Volume`][vol] of the player.
    ///
    /// See also: [`set_volume()`][Self::set_volume].
    ///
    /// [vol]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume
    pub async fn get_volume(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.volume().await?)
    }

    /// Sets the [`Volume`][vol] of the player.
    ///
    /// Volume should be between 0.0 and 1.0. Above 1.0 is possible, but not
    /// recommended. Negative values will be turned into 0.0
    ///
    /// See also: [`get_volume()`][Self::get_volume].
    ///
    /// [vol]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html#Property:Volume
    pub async fn set_volume(&self, volume: f64) -> Result<(), MprisError> {
        Ok(self.player_proxy.set_volume(volume).await?)
    }

    /// Shortcut to check if `self.playlist_proxy` is Some
    fn check_playlist_support(&self) -> Result<&PlaylistsInterface, MprisError> {
        match &self.playlist_interface {
            Some(proxy) => Ok(proxy),
            None => Err(MprisError::Unsupported),
        }
    }

    /// Tries to update the given [`Playlist`].
    ///
    /// Returns [`true`] if the given [`Playlist`] was found, [`false`] if [`Player`] isn't aware of
    /// that playlist. Running [`get_playlists()`][Self::get_playlists] will refresh the list of
    /// known playlists.
    ///
    /// Can only fail if the interface is not implemented.
    pub fn update_playlist(&self, playlist: &mut Playlist) -> Result<bool, MprisError> {
        Ok(self
            .check_playlist_support()?
            .update_playlist_struct(playlist))
    }

    /// Signals the player to activate a given [`Playlist`].
    ///
    /// > Starts playing the given playlist.
    /// >
    /// > It is up to the media player whether this completely replaces the current tracklist, or
    /// > whether it is merely inserted into the tracklist and the first track starts. For example,
    /// > if the media player is operating in a "jukebox" mode, it may just append the playlist to
    /// > the list of upcoming tracks, and skip to the first track in the playlist.
    ///
    /// See also: [MPRIS2 specification about `ActivatePlaylist`][activate] and
    /// [`get_playlists()`][Self::get_playlists]
    ///
    /// [activate]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html#Method:ActivatePlaylist
    pub async fn activate_playlist(&self, playlist: &Playlist) -> Result<(), MprisError> {
        self.check_playlist_support()?
            .activate_playlist(playlist)
            .await
    }

    /// Gets the [`Playlist`]s of the player.
    ///
    /// `start_index` and `max_count` allow for pagination of the playlists in case the player has a
    /// lot of them.
    /// The given [`PlaylistOrdering`] should be in the return value of
    /// [`orderings()`][Self::orderings] but this method does not check for that.
    ///
    /// See also: [MPRIS2 specification about `GetPlaylists`][get_playlists].
    ///
    /// [get_playlists]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html#Method:GetPlaylists
    pub async fn get_playlists(
        &self,
        start_index: u32,
        max_count: u32,
        order: PlaylistOrdering,
        reverse_order: bool,
    ) -> Result<Vec<Playlist>, MprisError> {
        self.check_playlist_support()?
            .get_playlists(start_index, max_count, order, reverse_order)
            .await
    }

    /// Clears the stored [`Playlist`]s metadata.
    ///
    /// Players don't signal when a [`Playlist`] gets removed meaning that if you use a [`Player`]
    /// instance for a long time and edit playlists often the internal playlist list will keep
    /// getting bigger with pointless data. This method lets you clear it if it becomes an issue.
    ///
    /// It's recommended to run [`get_playlists()`][Self::get_playlists] after clearing.
    ///
    /// Can only fail if the interface is not implemented.
    pub fn clear_playlists_data(&self) -> Result<(), MprisError> {
        self.check_playlist_support()?.clear();
        Ok(())
    }

    /// Gets the currently active [`Playlist`] if any.
    ///
    /// > Note that this may not have a value even after ActivatePlaylist is called with a valid
    /// > playlist id as ActivatePlaylist implementations have the option of simply inserting the
    /// > contents of the playlist into the current tracklist.
    ///
    /// See also: [MPRIS2 specification about `ActivePlaylist`][active].
    ///
    /// [active]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html#Property:ActivePlaylist
    pub async fn active_playlist(&self) -> Result<Option<Playlist>, MprisError> {
        self.check_playlist_support()?.active_playlist().await
    }

    /// Gets the [`PlaylistOrdering`]s the player supports.
    ///
    /// > The available orderings. At least one must be offered.
    ///
    /// See also: [MPRIS2 specification about `Orderings`][orderings].
    ///
    /// [orderings]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html#Property:Orderings
    pub async fn orderings(&self) -> Result<Vec<PlaylistOrdering>, MprisError> {
        self.check_playlist_support()?.orderings().await
    }

    /// Gets the number of available playlists.
    ///
    /// See also: [MPRIS2 specification about `PlaylistCount`][count].
    ///
    /// [count]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Playlists_Interface.html#Property:PlaylistCount
    pub async fn playlist_count(&self) -> Result<u32, MprisError> {
        self.check_playlist_support()?.playlist_count().await
    }

    /// Shortcut to check if `self.track_list_proxy` is Some
    fn check_track_list_support(&self) -> Result<&TrackListProxy, MprisError> {
        match &self.track_list_proxy {
            Some(proxy) => Ok(proxy),
            None => Err(MprisError::Unsupported),
        }
    }

    /// Queries the player to see if it allows changes to its `TrackList`.
    ///
    /// > If false, calling AddTrack or RemoveTrack will have no effect, and may raise a
    /// > NotSupported error.
    ///
    /// See also: [MPRIS2 specification about `CanEditTracks`][can_edit],
    /// [`add_track()`][Self::add_track] and [`remove_track()`][Self::remove_track] .
    ///
    /// [can_edit]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Property:CanEditTracks
    pub async fn can_edit_tracks(&self) -> Result<bool, MprisError> {
        Ok(self.check_track_list_support()?.can_edit_tracks().await?)
    }

    /// Gets the tracks in the current `TrackList`
    ///
    /// > An array which contains the identifier of each track in the tracklist, in order.
    ///
    /// See also: [MPRIS2 specification about `Tracks`][tracks].
    ///
    /// [tracks]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Property:Tracks
    pub async fn tracks(&self) -> Result<Vec<TrackID>, MprisError> {
        let result = self.check_track_list_support()?.tracks().await?;
        let mut track_ids = Vec::with_capacity(result.len());
        for r in result {
            track_ids.push(TrackID::from(r));
        }
        Ok(track_ids)
    }

    /// Adds a `uri` to the `TrackList` and optionally set it as current.
    ///
    /// The `uri` argument's uri scheme should be in
    /// [`supported_uri_schemes()`][Self::supported_uri_schemes] and the mime-type should be in
    /// [`supported_mime_types()`][Self::supported_mime_types] but note that this method does not
    /// check for that. It's up to you to verify that.
    ///
    /// It is placed after the specified [`TrackID`], if supported by the player. If [`None`] is
    /// used then it will be inserted as the first song.
    ///
    /// > If the CanEditTracks property is false, this has no effect.
    /// >
    /// > Note: Clients should not assume that the track has been added at the time when this method
    /// > returns.
    ///
    /// See also: [MPRIS2 specification about `AddTrack`][add_track] and
    /// [`can_edit_tracks()`][Self::can_edit_tracks].
    ///
    /// [add_track]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:AddTrack
    pub async fn add_track(
        &self,
        uri: &str,
        after_track: Option<&TrackID>,
        set_as_current: bool,
    ) -> Result<(), MprisError> {
        let after = if let Some(track_id) = after_track {
            track_id
        } else {
            &TrackID::no_track()
        };
        Ok(self
            .check_track_list_support()?
            .add_track(uri, after.as_ref(), set_as_current)
            .await?)
    }

    /// Removes an item from the TrackList.
    ///
    /// The special [`"NoTrack"`][TrackID::NO_TRACK] value is not allowed as the argument.
    ///
    /// > If the track is not part of this tracklist, this has no effect.
    /// >
    /// > If the CanEditTracks property is false, this has no effect.
    /// >
    /// > Note: Clients should not assume that the track has been removed at the time when this
    /// > method returns.
    ///
    /// See also: [MPRIS2 specification about `RemoveTrack`][remove] and
    /// [`can_edit_tracks()`][Self::can_edit_tracks].
    ///
    /// [remove]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:RemoveTrack
    pub async fn remove_track(&self, track_id: &TrackID) -> Result<(), MprisError> {
        if track_id.is_no_track() {
            return Err(MprisError::track_id_is_no_track());
        }
        Ok(self
            .check_track_list_support()?
            .remove_track(track_id.as_ref())
            .await?)
    }

    /// Go to a specific track on the [`Player`]'s `TrackList`.
    ///
    /// If the given [`TrackID`] is not part of the player's `TrackList` it will have no effect.
    /// The special [`"NoTrack"`][TrackID::NO_TRACK] value is not allowed as the argument.
    ///
    /// See also: [MPRIS2 specification about `GoTo`][go_to].
    ///
    /// [go_to]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:GoTo
    pub async fn go_to(&self, track_id: &TrackID) -> Result<(), MprisError> {
        if track_id.is_no_track() {
            return Err(MprisError::track_id_is_no_track());
        }
        Ok(self
            .check_track_list_support()?
            .go_to(track_id.as_ref())
            .await?)
    }

    /// Gets the [`Metadata`] for the given [`TrackID`]s.
    ///
    /// Will fail if any of the tracks has invalid metadata.
    ///
    /// See also: [MPRIS2 specification about `GetTracksMetadata`][get_meta].
    ///
    /// [get_meta]:
    /// https://specifications.freedesktop.org/mpris-spec/latest/Track_List_Interface.html#Method:GetTracksMetadata
    pub async fn get_tracks_metadata(
        &self,
        tracks: &[TrackID],
    ) -> Result<Vec<Metadata>, MprisError> {
        let result = self
            .check_track_list_support()?
            .get_tracks_metadata(&tracks.iter().map(|t| t.as_ref()).collect::<Vec<_>>())
            .await?;

        let mut metadata = Vec::with_capacity(tracks.len());
        for meta in result {
            let raw: HashMap<String, MetadataValue> = meta
                .into_iter()
                .map(|(k, v)| (k, MetadataValue::from(v)))
                .collect();
            metadata.push(Metadata::try_from(raw)?);
        }
        Ok(metadata)
    }
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("bus_name", &self.bus_name())
            .field("track_list", &self.track_list_proxy.is_some())
            .field("playlist", &self.playlist_interface.is_some())
            .finish_non_exhaustive()
    }
}

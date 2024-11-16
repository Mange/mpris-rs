use std::collections::HashMap;

use zbus::names::{BusName, OwnedUniqueName};
use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use crate::{metadata::RawMetadata, MprisDuration, MprisError, TrackID};
use crate::{LoopStatus, PlaybackStatus, Playlist, PlaylistOrdering};

#[proxy(
    default_service = "org.freedesktop.DBus",
    interface = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus",
    gen_blocking = false
)]
pub(crate) trait DBus {
    fn list_names(&self) -> Result<Vec<String>, MprisError>;

    fn get_name_owner(&self, bus_name: &BusName<'_>) -> Result<OwnedUniqueName, MprisError>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub(crate) trait MediaPlayer2 {
    /// Quit method
    fn quit(&self) -> Result<(), MprisError>;

    /// Raise method
    fn raise(&self) -> Result<(), MprisError>;

    /// CanQuit property
    #[zbus(property)]
    fn can_quit(&self) -> Result<bool, MprisError>;

    /// CanRaise property
    #[zbus(property)]
    fn can_raise(&self) -> Result<bool, MprisError>;

    /// DesktopEntry property
    #[zbus(property)]
    fn desktop_entry(&self) -> Result<String, MprisError>;

    /// HasTrackList property
    #[zbus(property)]
    fn has_track_list(&self) -> Result<bool, MprisError>;

    /// Identity property
    #[zbus(property)]
    fn identity(&self) -> Result<String, MprisError>;

    /// SupportedMimeTypes property
    #[zbus(property)]
    fn supported_mime_types(&self) -> Result<Vec<String>, MprisError>;

    /// SupportedUriSchemes property
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Result<Vec<String>, MprisError>;

    #[zbus(property)]
    fn fullscreen(&self) -> Result<bool, MprisError>;

    #[zbus(property)]
    fn set_fullscreen(&self, value: bool) -> Result<(), MprisError>;

    #[zbus(property)]
    fn can_set_fullscreen(&self) -> Result<bool, MprisError>;
}

impl MediaPlayer2Proxy<'_> {
    pub(crate) async fn ping(&self) -> Result<(), MprisError> {
        self.inner()
            .connection()
            .call_method(
                Some(self.0.destination()),
                self.0.path(),
                Some("org.freedesktop.DBus.Peer"),
                "Ping",
                &(),
            )
            .await
            .map(|_| ())
            .map_err(MprisError::from)
    }
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub(crate) trait Player {
    /// Next method
    fn next(&self) -> Result<(), MprisError>;

    /// OpenUri method
    fn open_uri(&self, uri: &str) -> Result<(), MprisError>;

    /// Pause method
    fn pause(&self) -> Result<(), MprisError>;

    /// Play method
    fn play(&self) -> Result<(), MprisError>;

    /// PlayPause method
    fn play_pause(&self) -> Result<(), MprisError>;

    /// Previous method
    fn previous(&self) -> Result<(), MprisError>;

    /// Seek method
    fn seek(&self, offset: i64) -> Result<(), MprisError>;

    /// SetPosition method
    fn set_position(&self, track_id: &TrackID, position: MprisDuration) -> Result<(), MprisError>;

    /// Stop method
    fn stop(&self) -> Result<(), MprisError>;

    /// Seeked signal
    #[zbus(signal)]
    fn seeked(&self, position: MprisDuration) -> Result<(), MprisError>;

    /// CanControl property
    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> Result<bool, MprisError>;

    /// CanGoNext property
    #[zbus(property)]
    fn can_go_next(&self) -> Result<bool, MprisError>;

    /// CanGoPrevious property
    #[zbus(property)]
    fn can_go_previous(&self) -> Result<bool, MprisError>;

    /// CanPause property
    #[zbus(property)]
    fn can_pause(&self) -> Result<bool, MprisError>;

    /// CanPlay property
    #[zbus(property)]
    fn can_play(&self) -> Result<bool, MprisError>;

    /// CanSeek property
    #[zbus(property)]
    fn can_seek(&self) -> Result<bool, MprisError>;

    /// LoopStatus property
    #[zbus(property)]
    fn loop_status(&self) -> Result<LoopStatus, MprisError>;

    #[zbus(property)]
    fn set_loop_status(&self, value: LoopStatus) -> Result<(), MprisError>;

    /// MaximumRate property
    #[zbus(property)]
    fn maximum_rate(&self) -> Result<f64, MprisError>;

    /// Metadata property
    #[zbus(property)]
    fn metadata(&self) -> Result<RawMetadata, MprisError>;

    /// MinimumRate property
    #[zbus(property)]
    fn minimum_rate(&self) -> Result<f64, MprisError>;

    /// PlaybackStatus property
    #[zbus(property)]
    fn playback_status(&self) -> Result<PlaybackStatus, MprisError>;

    /// Position property
    #[zbus(property(emits_changed_signal = "const"))]
    fn position(&self) -> Result<MprisDuration, MprisError>;

    /// Rate property
    #[zbus(property)]
    fn rate(&self) -> Result<f64, MprisError>;

    #[zbus(property)]
    fn set_rate(&self, value: f64) -> Result<(), MprisError>;

    /// Shuffle property
    #[zbus(property)]
    fn shuffle(&self) -> Result<bool, MprisError>;

    #[zbus(property)]
    fn set_shuffle(&self, value: bool) -> Result<(), MprisError>;

    /// Volume property
    #[zbus(property)]
    fn volume(&self) -> Result<f64, MprisError>;

    #[zbus(property)]
    fn set_volume(&self, value: f64) -> Result<(), MprisError>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Playlists",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub(crate) trait Playlists {
    /// ActivatePlaylist method
    fn activate_playlist(&self, playlist_id: &OwnedObjectPath) -> Result<(), MprisError>;

    /// GetPlaylists method
    fn get_playlists(
        &self,
        index: u32,
        max_count: u32,
        order: &str,
        reverse_order: bool,
    ) -> Result<Vec<Playlist>, MprisError>;

    #[zbus(signal)]
    fn playlist_changed(&self, playlist: Playlist) -> Result<(), MprisError>;

    /// ActivePlaylist property
    #[zbus(property, name = "ActivePlaylist")]
    fn _active_playlist_inner(
        &self,
    ) -> Result<(bool, (OwnedObjectPath, String, String)), MprisError>;

    /// Orderings property
    #[zbus(property)]
    fn orderings(&self) -> Result<Vec<PlaylistOrdering>, MprisError>;

    /// PlaylistCount property
    #[zbus(property)]
    fn playlist_count(&self) -> Result<u32, MprisError>;
}

impl PlaylistsProxy<'_> {
    pub(crate) async fn active_playlist(&self) -> Result<Option<Playlist>, MprisError> {
        Ok(match self._active_playlist_inner().await? {
            (true, data) => Some(Playlist::from(data)),
            _ => None,
        })
    }
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.TrackList",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub trait TrackList {
    /// AddTrack method
    fn add_track(
        &self,
        uri: &str,
        after_track: &TrackID,
        set_as_current: bool,
    ) -> Result<(), MprisError>;

    // There is no way to implement zvariant::Type for MetadataValue while keeping the current serde
    // implementation. If you try to do it then returning RawMetadata here will cause a infinite
    // recursion that ends with a stack overflow. To avoid that the function just gets wrapped
    /// GetTracksMetadata method
    fn _get_tracks_metadata(
        &self,
        track_ids: &[TrackID],
    ) -> Result<Vec<HashMap<String, OwnedValue>>, MprisError>;

    /// GoTo method
    fn go_to(&self, track_id: &TrackID) -> Result<(), MprisError>;

    /// RemoveTrack method
    fn remove_track(&self, track_id: &TrackID) -> Result<(), MprisError>;

    /// TrackAdded signal
    #[zbus(signal)]
    fn track_added(&self, metadata: RawMetadata, after_track: TrackID) -> Result<(), MprisError>;

    /// TrackListReplaced signal
    #[zbus(signal)]
    fn track_list_replaced(
        &self,
        track_ids: Vec<TrackID>,
        current_track: TrackID,
    ) -> Result<(), MprisError>;

    /// TrackMetadataChanged signal
    #[zbus(signal)]
    fn track_metadata_changed(
        &self,
        track_id: TrackID,
        metadata: RawMetadata,
    ) -> Result<(), MprisError>;

    /// TrackRemoved signal
    #[zbus(signal)]
    fn track_removed(&self, track_id: TrackID) -> Result<(), MprisError>;

    /// CanEditTracks property
    #[zbus(property)]
    fn can_edit_tracks(&self) -> Result<bool, MprisError>;

    /// Tracks property
    #[zbus(property(emits_changed_signal = "invalidates"))]
    fn tracks(&self) -> Result<Vec<TrackID>, MprisError>;
}

impl TrackListProxy<'_> {
    pub(crate) async fn get_tracks_metadata(
        &self,
        tracks: &[TrackID],
    ) -> Result<Vec<RawMetadata>, MprisError> {
        Ok(self
            ._get_tracks_metadata(tracks)
            .await?
            .into_iter()
            .map(RawMetadata::from)
            .collect())
    }
}

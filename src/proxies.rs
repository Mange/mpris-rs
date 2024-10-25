use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

#[proxy(
    default_service = "org.freedesktop.DBus",
    interface = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus",
    gen_blocking = false
)]
pub(crate) trait DBus {
    fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub(crate) trait MediaPlayer2 {
    /// Quit method
    fn quit(&self) -> zbus::Result<()>;

    /// Raise method
    fn raise(&self) -> zbus::Result<()>;

    /// CanQuit property
    #[zbus(property)]
    fn can_quit(&self) -> zbus::Result<bool>;

    /// CanRaise property
    #[zbus(property)]
    fn can_raise(&self) -> zbus::Result<bool>;

    /// DesktopEntry property
    #[zbus(property)]
    fn desktop_entry(&self) -> zbus::Result<String>;

    /// HasTrackList property
    #[zbus(property)]
    fn has_track_list(&self) -> zbus::Result<bool>;

    /// Identity property
    #[zbus(property)]
    fn identity(&self) -> zbus::Result<String>;

    /// SupportedMimeTypes property
    #[zbus(property)]
    fn supported_mime_types(&self) -> zbus::Result<Vec<String>>;

    /// SupportedUriSchemes property
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> zbus::Result<Vec<String>>;
}

impl MediaPlayer2Proxy<'_> {
    pub(crate) async fn ping(&self) -> zbus::Result<()> {
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
            .and(Ok(()))
    }
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub(crate) trait Player {
    /// Next method
    fn next(&self) -> zbus::Result<()>;

    /// OpenUri method
    fn open_uri(&self, uri: &str) -> zbus::Result<()>;

    /// Pause method
    fn pause(&self) -> zbus::Result<()>;

    /// Play method
    fn play(&self) -> zbus::Result<()>;

    /// PlayPause method
    fn play_pause(&self) -> zbus::Result<()>;

    /// Previous method
    fn previous(&self) -> zbus::Result<()>;

    /// Seek method
    fn seek(&self, offset: i64) -> zbus::Result<()>;

    /// SetPosition method
    fn set_position(&self, track_id: &OwnedObjectPath, position: i64) -> zbus::Result<()>;

    /// Stop method
    fn stop(&self) -> zbus::Result<()>;

    /// StopAfterCurrent method
    fn stop_after_current(&self) -> zbus::Result<()>;

    /// Seeked signal
    #[zbus(signal)]
    fn seeked(&self, position: i64) -> zbus::Result<()>;

    /// CanControl property
    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> zbus::Result<bool>;

    /// CanGoNext property
    #[zbus(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;

    /// CanGoPrevious property
    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;

    /// CanPause property
    #[zbus(property)]
    fn can_pause(&self) -> zbus::Result<bool>;

    /// CanPlay property
    #[zbus(property)]
    fn can_play(&self) -> zbus::Result<bool>;

    /// CanSeek property
    #[zbus(property)]
    fn can_seek(&self) -> zbus::Result<bool>;

    /// LoopStatus property
    #[zbus(property)]
    fn loop_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn set_loop_status(&self, value: &str) -> zbus::Result<()>;

    /// MaximumRate property
    #[zbus(property)]
    fn maximum_rate(&self) -> zbus::Result<f64>;

    /// Metadata property
    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, Value>>;

    /// MinimumRate property
    #[zbus(property)]
    fn minimum_rate(&self) -> zbus::Result<f64>;

    /// PlaybackStatus property
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    /// Position property
    #[zbus(property(emits_changed_signal = "const"))]
    fn position(&self) -> zbus::Result<i64>;

    /// Rate property
    #[zbus(property)]
    fn rate(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn set_rate(&self, value: f64) -> zbus::Result<()>;

    /// Shuffle property
    #[zbus(property)]
    fn shuffle(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_shuffle(&self, value: bool) -> zbus::Result<()>;

    /// Volume property
    #[zbus(property)]
    fn volume(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn set_volume(&self, value: f64) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Playlists",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
pub(crate) trait Playlists {
    /// ActivatePlaylist method
    fn activate_playlist(&self, playlist_id: &ObjectPath<'_>) -> zbus::Result<()>;

    /// GetPlaylists method
    fn get_playlists(
        &self,
        index: u32,
        max_count: u32,
        order: &str,
        reverse_order: bool,
    ) -> zbus::Result<Vec<(OwnedObjectPath, String, String)>>;

    #[zbus(signal)]
    fn playlist_changed(&self) -> zbus::Result<Vec<(OwnedObjectPath, String, String)>>;

    /// ActivePlaylist property
    #[zbus(property)]
    fn active_playlist(&self) -> zbus::Result<(bool, (OwnedObjectPath, String, String))>;

    /// Orderings property
    #[zbus(property)]
    fn orderings(&self) -> zbus::Result<Vec<String>>;

    /// PlaylistCount property
    #[zbus(property)]
    fn playlist_count(&self) -> zbus::Result<u32>;
}

use std::collections::HashMap;

use futures_util::try_join;
use zbus::{names::BusName, Connection};

use crate::{
    metadata::MetadataValue,
    proxies::{MediaPlayer2Proxy, PlayerProxy, PlaylistsProxy, TrackListProxy},
    LoopStatus, Metadata, Mpris, MprisDuration, MprisError, PlaybackStatus, Playlist,
    PlaylistOrdering, TrackID, MPRIS2_PREFIX,
};

pub struct Player {
    bus_name: BusName<'static>,
    mp2_proxy: MediaPlayer2Proxy<'static>,
    player_proxy: PlayerProxy<'static>,
    playlist_proxy: Option<PlaylistsProxy<'static>>,
    track_list_proxy: Option<TrackListProxy<'static>>,
}

impl Player {
    pub async fn new(mpris: &Mpris, bus_name: BusName<'static>) -> Result<Player, MprisError> {
        Player::new_from_connection(mpris.connection.clone(), bus_name).await
    }

    pub(crate) async fn new_from_connection(
        connection: Connection,
        bus_name: BusName<'static>,
    ) -> Result<Player, MprisError> {
        let (mp2_proxy, player_proxy, playlist_proxy, track_list_proxy) = try_join!(
            MediaPlayer2Proxy::new(&connection, bus_name.clone()),
            PlayerProxy::new(&connection, bus_name.clone()),
            PlaylistsProxy::new(&connection, bus_name.clone()),
            TrackListProxy::new(&connection, bus_name.clone()),
        )?;

        let playlist = playlist_proxy.playlist_count().await.is_ok();
        let track_list = track_list_proxy.can_edit_tracks().await.is_ok();
        Ok(Player {
            bus_name,
            mp2_proxy,
            player_proxy,
            playlist_proxy: if playlist { Some(playlist_proxy) } else { None },
            track_list_proxy: if track_list {
                Some(track_list_proxy)
            } else {
                None
            },
        })
    }

    pub async fn supports_track_list(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.has_track_list().await?)
    }

    pub fn supports_playlist_interface(&self) -> bool {
        self.playlist_proxy.is_some()
    }

    pub fn supports_track_list_interface(&self) -> bool {
        self.track_list_proxy.is_some()
    }

    pub async fn metadata(&self) -> Result<Metadata, MprisError> {
        Ok(self.raw_metadata().await?.try_into()?)
    }

    pub async fn raw_metadata(&self) -> Result<HashMap<String, MetadataValue>, MprisError> {
        let data = self.player_proxy.metadata().await?;
        let raw: HashMap<String, MetadataValue> =
            data.into_iter().map(|(k, v)| (k, v.into())).collect();
        Ok(raw)
    }

    pub async fn is_running(&self) -> Result<bool, MprisError> {
        match self.mp2_proxy.ping().await {
            Ok(_) => Ok(true),
            Err(e) => {
                if let zbus::Error::MethodError(ref err_name, _, _) = e {
                    if err_name.as_str() == "org.freedesktop.DBus.Error.ServiceUnknown" {
                        Ok(false)
                    } else {
                        Err(e.into())
                    }
                } else {
                    Err(e.into())
                }
            }
        }
    }

    pub fn bus_name(&self) -> &str {
        self.bus_name.as_str()
    }

    pub fn bus_name_trimmed(&self) -> &str {
        self.bus_name().trim_start_matches(MPRIS2_PREFIX)
    }

    pub async fn quit(&self) -> Result<(), MprisError> {
        Ok(self.mp2_proxy.quit().await?)
    }

    pub async fn can_quit(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.can_quit().await?)
    }

    pub async fn raise(&self) -> Result<(), MprisError> {
        Ok(self.mp2_proxy.raise().await?)
    }

    pub async fn can_raise(&self) -> Result<bool, MprisError> {
        Ok(self.mp2_proxy.can_raise().await?)
    }

    pub async fn desktop_entry(&self) -> Result<String, MprisError> {
        Ok(self.mp2_proxy.desktop_entry().await?)
    }

    pub async fn identity(&self) -> Result<String, MprisError> {
        Ok(self.mp2_proxy.identity().await?)
    }

    pub async fn supported_mime_types(&self) -> Result<Vec<String>, MprisError> {
        Ok(self.mp2_proxy.supported_mime_types().await?)
    }

    pub async fn supported_uri_schemes(&self) -> Result<Vec<String>, MprisError> {
        Ok(self.mp2_proxy.supported_uri_schemes().await?)
    }

    pub async fn can_control(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_control().await?)
    }

    pub async fn next(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.next().await?)
    }

    pub async fn can_go_next(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_go_next().await?)
    }

    pub async fn previous(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.previous().await?)
    }

    pub async fn can_go_previous(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_go_previous().await?)
    }

    pub async fn play(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.play().await?)
    }

    pub async fn can_play(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_play().await?)
    }

    pub async fn pause(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.pause().await?)
    }

    pub async fn can_pause(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_pause().await?)
    }

    pub async fn play_pause(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.play_pause().await?)
    }

    pub async fn stop(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.stop().await?)
    }

    pub async fn stop_after_current(&self) -> Result<(), MprisError> {
        Ok(self.player_proxy.stop_after_current().await?)
    }

    pub async fn seek(&self, offset_in_microseconds: i64) -> Result<(), MprisError> {
        Ok(self.player_proxy.seek(offset_in_microseconds).await?)
    }

    pub async fn seek_forwards(&self, offset: MprisDuration) -> Result<(), MprisError> {
        Ok(self.player_proxy.seek(offset.into()).await?)
    }

    pub async fn seek_backwards(&self, offset: MprisDuration) -> Result<(), MprisError> {
        Ok(self.player_proxy.seek(-i64::from(offset)).await?)
    }

    pub async fn can_seek(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.can_seek().await?)
    }

    pub async fn get_position(&self) -> Result<MprisDuration, MprisError> {
        Ok(self.player_proxy.position().await?.try_into()?)
    }

    pub async fn set_position(
        &self,
        track_id: &TrackID,
        position: MprisDuration,
    ) -> Result<(), MprisError> {
        Ok(self
            .player_proxy
            .set_position(track_id.as_ref(), position.into())
            .await?)
    }

    pub async fn get_loop_status(&self) -> Result<LoopStatus, MprisError> {
        Ok(self.player_proxy.loop_status().await?.parse()?)
    }

    pub async fn set_loop_status(&self, loop_status: LoopStatus) -> Result<(), MprisError> {
        Ok(self
            .player_proxy
            .set_loop_status(loop_status.as_str())
            .await?)
    }

    pub async fn playback_status(&self) -> Result<PlaybackStatus, MprisError> {
        Ok(self.player_proxy.playback_status().await?.parse()?)
    }

    pub async fn open_uri(&self, uri: &str) -> Result<(), MprisError> {
        Ok(self.player_proxy.open_uri(uri).await?)
    }

    pub async fn maximum_rate(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.maximum_rate().await?)
    }

    pub async fn minimum_rate(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.minimum_rate().await?)
    }

    pub async fn get_playback_rate(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.rate().await?)
    }

    pub async fn set_playback_rate(&self, rate: f64) -> Result<(), MprisError> {
        Ok(self.player_proxy.set_rate(rate).await?)
    }

    pub async fn get_shuffle(&self) -> Result<bool, MprisError> {
        Ok(self.player_proxy.shuffle().await?)
    }

    pub async fn set_shuffle(&self, shuffle: bool) -> Result<(), MprisError> {
        Ok(self.player_proxy.set_shuffle(shuffle).await?)
    }

    pub async fn get_volume(&self) -> Result<f64, MprisError> {
        Ok(self.player_proxy.volume().await?)
    }

    pub async fn set_volume(&self, volume: f64) -> Result<(), MprisError> {
        Ok(self.player_proxy.set_volume(volume).await?)
    }

    fn check_playlist_support(&self) -> Result<&PlaylistsProxy, MprisError> {
        match &self.playlist_proxy {
            Some(proxy) => Ok(proxy),
            None => Err(MprisError::Unsupported),
        }
    }

    pub async fn activate_playlist(&self, playlist: &Playlist) -> Result<(), MprisError> {
        Ok(self
            .check_playlist_support()?
            .activate_playlist(&playlist.get_id())
            .await?)
    }

    pub async fn get_playlists(
        &self,
        start_index: u32,
        max_count: u32,
        order: PlaylistOrdering,
        reverse_order: bool,
    ) -> Result<Vec<Playlist>, MprisError> {
        Ok(self
            .check_playlist_support()?
            .get_playlists(start_index, max_count, order.as_str_value(), reverse_order)
            .await?
            .into_iter()
            .map(Playlist::from)
            .collect())
    }

    pub async fn active_playlist(&self) -> Result<Option<Playlist>, MprisError> {
        let result = self.check_playlist_support()?.active_playlist().await?;
        if result.0 {
            Ok(Some(Playlist::from(result.1)))
        } else {
            Ok(None)
        }
    }

    pub async fn orderings(&self) -> Result<Vec<PlaylistOrdering>, MprisError> {
        let result = self.check_playlist_support()?.orderings().await?;
        let mut orderings = Vec::with_capacity(result.len());
        for s in result {
            orderings.push(s.parse()?);
        }
        Ok(orderings)
    }

    pub async fn playlist_count(&self) -> Result<u32, MprisError> {
        Ok(self.check_playlist_support()?.playlist_count().await?)
    }

    fn check_track_list_support(&self) -> Result<&TrackListProxy, MprisError> {
        match &self.track_list_proxy {
            Some(proxy) => Ok(proxy),
            None => Err(MprisError::Unsupported),
        }
    }

    pub async fn can_edit_tracks(&self) -> Result<bool, MprisError> {
        Ok(self.check_track_list_support()?.can_edit_tracks().await?)
    }

    pub async fn tracks(&self) -> Result<Vec<TrackID>, MprisError> {
        let result = self.check_track_list_support()?.tracks().await?;
        let mut track_ids = Vec::with_capacity(result.len());
        for r in result {
            track_ids.push(TrackID::try_from(r)?);
        }
        Ok(track_ids)
    }

    pub async fn add_track(
        &self,
        url: &str,
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
            .add_track(url, after.as_ref(), set_as_current)
            .await?)
    }

    pub async fn remove_track(&self, track_id: &TrackID) -> Result<(), MprisError> {
        if track_id.is_no_track() {
            return Err(MprisError::track_id_is_no_track());
        }
        Ok(self
            .check_track_list_support()?
            .remove_track(track_id.as_ref())
            .await?)
    }

    pub async fn go_to(&self, track_id: &TrackID) -> Result<(), MprisError> {
        if track_id.is_no_track() {
            return Err(MprisError::track_id_is_no_track());
        }
        Ok(self
            .check_track_list_support()?
            .go_to(track_id.as_ref())
            .await?)
    }

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
            .field("playlist_proxy", &self.playlist_proxy.is_some())
            .finish()
    }
}

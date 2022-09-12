use super::{
    DBusError, LoopStatus, Metadata, PlaybackStatus, Player, Progress, TrackID, TrackList,
    TrackListError,
};
use crate::pooled_connection::MprisEvent;
use thiserror::Error;

/// Represents a change in [`Player`] state.
///
/// Note that this does not include position changes (seeking in a track or normal progress of time
/// for playing media).
#[derive(Debug)]
pub enum Event {
    /// [`Player`] was shut down / quit.
    PlayerShutDown,

    /// [`Player`] was paused.
    Paused,

    /// [`Player`] started playing media.
    Playing,

    /// [`Player`] was stopped.
    Stopped,

    /// Loop status of [`Player`] was changed. New [`LoopStatus`] is provided.
    LoopingChanged(LoopStatus),

    /// Shuffle status of [`Player`] was changed. New shuffle status is provided.
    ShuffleToggled(bool),

    /// [`Player`]'s volume was changed. The new volume is provided.
    VolumeChanged(f64),

    /// [`Player`]'s playback rate was changed. New playback rate is provided.
    PlaybackRateChanged(f64),

    /// [`Player`]'s track changed. [`Metadata`] of the new track is provided.
    TrackChanged(Metadata),

    /// [`Player`] seeked (changed position in the current track).
    ///
    /// This will only be emitted when the player in question emits this signal. Some players do
    /// not support this signal. If you want to accurately detect seeking, you'll have to query
    /// the player's position yourself at some intervals.
    Seeked {
        /// The new position, in microseconds.
        position_in_us: u64,
    },

    /// A new track was added to the [`TrackList`].
    TrackAdded(TrackID),

    /// A track was removed from the [`TrackList`].
    TrackRemoved(TrackID),

    /// A track on the [`TrackList`] had its metadata changed.
    ///
    /// This could also mean that a entry on the playlist completely changed; including the ID.
    TrackMetadataChanged {
        /// The id of the track *before* the change.
        ///
        /// Only use this ID if you are keeping track of track IDs somewhere. The ID might no
        /// longer be valid for the player, so loading metadata for it might fail.
        ///
        /// **Note:** This can be the same as the `new_id`.
        old_id: TrackID,

        /// The id of the track *after* the change.
        ///
        /// Use this ID if you intend to read metadata or anything else as the `old_id` may no
        /// longer be valid.
        ///
        /// **Note:** This can be the same as the `old_id`.
        new_id: TrackID,
    },

    /// The track list was replaced.
    TrackListReplaced,
}

/// Errors that can occur while processing event streams.
#[derive(Debug, Error)]
pub enum EventError {
    /// Something went wrong with the D-Bus communication. See the [`DBusError`] type.
    #[error("D-Bus communication failed: {0}")]
    DBusError(#[from] DBusError),

    /// Something went wrong with the track list. See the [`TrackListError`] type.
    #[error("TrackList could not be refreshed: {0}")]
    TrackListError(#[from] TrackListError),
}

/// Iterator that blocks forever until the player has an [`Event`].
///
/// Iteration will stop if player stops running. If the player was running before this iterator
/// blocks, one last [`Event::PlayerShutDown`] event will be emitted before stopping iteration.
///
/// If multiple events are found between processing D-Bus events then all of them will be iterated
/// in rapid succession before processing more events.
#[derive(Debug)]
pub struct PlayerEvents<'a> {
    /// [`Player`] to watch.
    player: &'a Player,

    /// Queued up events found after the last signal.
    buffer: Vec<Event>,

    /// Used to diff older state to find events.
    last_progress: Progress,

    /// Current tracklist of the player. Will be kept up to date.
    track_list: Option<TrackList>,
}

impl PlayerEvents<'_> {
    pub(crate) fn new(player: &Player) -> Result<PlayerEvents, DBusError> {
        let progress = Progress::from_player(player)?;
        Ok(PlayerEvents {
            player,
            buffer: Vec::new(),
            last_progress: progress,
            track_list: player.checked_get_track_list()?,
        })
    }

    /// Current tracklist of the player. Will be kept up to date.
    pub fn track_list(&self) -> Option<&TrackList> {
        self.track_list.as_ref()
    }

    fn read_events(&mut self) -> Result<(), EventError> {
        self.player.process_events_blocking_until_received();

        let mut new_progress: Option<Progress> = None;
        let mut reload_track_list = false;

        for event in self.player.pending_events().into_iter() {
            match event {
                MprisEvent::PlayerQuit => {
                    self.buffer.push(Event::PlayerShutDown);
                    return Ok(());
                }
                MprisEvent::PlayerPropertiesChanged => {
                    if new_progress.is_none() {
                        new_progress = Some(Progress::from_player(self.player)?);
                    }
                }
                MprisEvent::Seeked { position_in_us } => {
                    self.buffer.push(Event::Seeked { position_in_us })
                }
                MprisEvent::TrackListPropertiesChanged => {
                    reload_track_list = true;
                }
                MprisEvent::TrackListReplaced { ids } => {
                    if let Some(ref mut list) = self.track_list {
                        list.replace(ids.into_iter().map(TrackID::from).collect());
                    }
                    self.buffer.push(Event::TrackListReplaced);
                }
                MprisEvent::TrackAdded { after_id, metadata } => {
                    if let Some(id) = metadata.track_id() {
                        if let Some(ref mut list) = self.track_list {
                            list.insert(&after_id, metadata);
                        }
                        self.buffer.push(Event::TrackAdded(id));
                    }
                }
                MprisEvent::TrackRemoved { id } => {
                    if let Some(ref mut list) = self.track_list {
                        list.remove(&id);
                    }
                    self.buffer.push(Event::TrackRemoved(id));
                }
                MprisEvent::TrackMetadataChanged { old_id, metadata } => {
                    if let Some(ref mut list) = self.track_list {
                        if let Some(new_id) = list.replace_track_metadata(&old_id, metadata) {
                            self.buffer
                                .push(Event::TrackMetadataChanged { old_id, new_id });
                        }
                    }
                }
            }
        }

        if let Some(progress) = new_progress {
            self.detect_playback_status_events(&progress);
            self.detect_loop_status_events(&progress);
            reload_track_list |= self.detect_shuffle_events(&progress);
            self.detect_volume_events(&progress);
            self.detect_playback_rate_events(&progress);
            self.detect_metadata_events(&progress);
            self.last_progress = progress;
        }

        if reload_track_list && self.track_list.is_some() {
            if let Some(new_tracks) = self.player.checked_get_track_list()? {
                match self.track_list {
                    Some(ref mut list) => list.replace(new_tracks),
                    None => self.track_list = Some(new_tracks),
                }
                self.buffer.push(Event::TrackListReplaced);
            }
        }

        Ok(())
    }

    fn detect_playback_status_events(&mut self, new_progress: &Progress) {
        match new_progress.playback_status() {
            status if self.last_progress.playback_status() == status => {}
            PlaybackStatus::Playing => self.buffer.push(Event::Playing),
            PlaybackStatus::Paused => self.buffer.push(Event::Paused),
            PlaybackStatus::Stopped => self.buffer.push(Event::Stopped),
        }
    }

    fn detect_loop_status_events(&mut self, new_progress: &Progress) {
        let loop_status = new_progress.loop_status();
        if self.last_progress.loop_status() != loop_status {
            self.buffer.push(Event::LoopingChanged(loop_status));
        }
    }

    fn detect_shuffle_events(&mut self, new_progress: &Progress) -> bool {
        let status = new_progress.shuffle();
        if self.last_progress.shuffle() != status {
            self.buffer.push(Event::ShuffleToggled(status));
            true
        } else {
            false
        }
    }

    fn detect_volume_events(&mut self, new_progress: &Progress) {
        let volume = new_progress.current_volume();
        if is_different_float(self.last_progress.current_volume(), volume) {
            self.buffer.push(Event::VolumeChanged(volume));
        }
    }

    fn detect_playback_rate_events(&mut self, new_progress: &Progress) {
        let rate = new_progress.playback_rate();
        if is_different_float(self.last_progress.playback_rate(), rate) {
            self.buffer.push(Event::PlaybackRateChanged(rate));
        }
    }

    fn detect_metadata_events(&mut self, new_progress: &Progress) {
        let new_metadata = new_progress.metadata();
        let old_metadata = self.last_progress.metadata();

        // As a workaround for Players not setting a valid track ID, we also check against the URL
        // Title and artists are checked to detect changes for streams (radios) because track ID and URL don't change.
        // Title is checked first because most radios set title to `Artist - Title` and have the station name in artists.

        if old_metadata.track_id() != new_metadata.track_id()
            || old_metadata.url() != new_metadata.url()
            || old_metadata.title() != new_metadata.title()
            || old_metadata.artists() != new_metadata.artists()
        {
            self.buffer.push(Event::TrackChanged(new_metadata.clone()));
        }
    }
}

fn is_different_float(a: f64, b: f64) -> bool {
    (a - b).abs() >= ::std::f64::EPSILON
}

impl<'a> Iterator for PlayerEvents<'a> {
    type Item = Result<Event, EventError>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.buffer.is_empty() {
            // Stop iteration when player is not running. Why beat a dead horse?
            if !self.player.is_running() {
                return None;
            }

            match self.read_events() {
                Ok(_) => {}
                Err(err) => return Some(Err(err)),
            };
        }

        let event = self.buffer.remove(0);
        Some(Ok(event))
    }
}

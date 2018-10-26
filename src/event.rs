use super::{
    DBusError, LoopStatus, Metadata, PlaybackStatus, Player, Progress, TrackID, TrackList,
    TrackListError,
};
use pooled_connection::MprisEvent;

/// Represents a change in Player state.
///
/// Note that this does not include position changes (seeking in a track or normal progress of time
/// for playing media).
#[derive(Debug)]
pub enum Event {
    /// Player was shut down / quit.
    PlayerShutDown,

    /// Player was paused.
    Paused,

    /// Player started playing media.
    Playing,

    /// Player was stopped.
    Stopped,

    /// Loop status of player was changed. New loop status is provided.
    LoopingChanged(LoopStatus),

    /// Shuffle status of player was changed. New shuffle status is provided.
    ShuffleToggled(bool),

    /// Player's volume was changed. The new volume is provided.
    VolumeChanged(f64),

    /// Player's playback rate was changed. New playback rate is provided.
    PlaybackRateChanged(f64),

    /// Player's track changed. Metadata of the new track is provided.
    TrackChanged(Metadata),

    /// Player seeked (changed position in the current track).
    ///
    /// This will only be emitted when the player in question emits this signal. Some players do
    /// not support this signal. If you want to accurately detect seeking, you'll have to query
    /// the player's position yourself at some intervals.
    Seeked {
        /// The new position, in microseconds.
        position_in_us: u64,
    },

    /// A new track was added to the TrackList.
    TrackAdded(TrackID),

    /// A track was removed from the TrackList.
    TrackRemoved(TrackID),

    /// A track on the TrackList had its metadata changed.
    TrackMetadataChanged(TrackID),

    /// The track list was replaced.
    TrackListReplaced,
}

/// Errors that can occur while processing event streams.
#[derive(Debug, Fail)]
pub enum EventError {
    /// Something went wrong with the D-Bus communication. See the `DBusError` type.
    #[fail(display = "D-Bus communication failed")]
    DBusError(#[cause] DBusError),

    /// Something went wrong with the track list. See the `TrackListError` type.
    #[fail(display = "TrackList could not be refreshed")]
    TrackListError(#[cause] TrackListError),
}

/// Iterator that blocks forever until the player has an event.
///
/// Iteration will stop if player stops running. If the player was running before this iterator
/// blocks, one last `Event::PlayerShutDown` event will be emitted before stopping iteration.
///
/// If multiple events are found between processing D-Bus events then all of them will be iterated
/// in rapid succession before processing more events.
#[derive(Debug)]
pub struct PlayerEvents<'a> {
    /// Player to watch.
    player: &'a Player<'a>,

    /// Queued up events found after the last signal.
    buffer: Vec<Event>,

    /// Used to diff older state to find events.
    last_progress: Progress,

    /// Current tracklist of the player. Will be kept up to date.
    track_list: TrackList,
}

impl<'a> PlayerEvents<'a> {
    pub fn new(player: &'a Player<'a>) -> Result<PlayerEvents<'a>, DBusError> {
        let progress = Progress::from_player(player)?;
        Ok(PlayerEvents {
            player,
            buffer: Vec::new(),
            last_progress: progress,
            track_list: player.get_track_list()?,
        })
    }

    pub fn track_list(&self) -> &TrackList {
        &self.track_list
    }

    fn read_events(&mut self) -> Result<(), EventError> {
        self.player.process_events_blocking_until_received();

        let mut new_progress: Option<Progress> = None;

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
                    self.track_list.reload(&self.player)?;
                    self.buffer.push(Event::TrackListReplaced);
                }
                MprisEvent::TrackListReplaced { ids } => {
                    self.track_list
                        .replace(ids.into_iter().map(TrackID::from).collect());
                    self.buffer.push(Event::TrackListReplaced);
                }
                MprisEvent::TrackAdded { after_id, metadata } => {
                    if let Some(id) = metadata.track_id() {
                        self.track_list.insert(&after_id, metadata);
                        self.buffer.push(Event::TrackAdded(id));
                    }
                }
                MprisEvent::TrackRemoved { id } => {
                    self.track_list.remove(&id);
                    self.buffer.push(Event::TrackRemoved(id));
                }
                MprisEvent::TrackMetadataChanged { id, metadata } => {
                    self.track_list.update_metadata(&id, metadata);
                    self.buffer.push(Event::TrackMetadataChanged(id));
                }
            }
        }

        if let Some(progress) = new_progress {
            self.detect_playback_status_events(&progress);
            self.detect_loop_status_events(&progress);
            self.detect_shuffle_events(&progress);
            self.detect_volume_events(&progress);
            self.detect_playback_rate_events(&progress);
            self.detect_metadata_events(&progress);
            self.last_progress = progress;
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

    fn detect_shuffle_events(&mut self, new_progress: &Progress) {
        let status = new_progress.shuffle();
        if self.last_progress.shuffle() != status {
            self.buffer.push(Event::ShuffleToggled(status));
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
        let metadata = new_progress.metadata();
        if self.last_progress.metadata().track_id() != metadata.track_id() {
            self.buffer.push(Event::TrackChanged(metadata.clone()));
        }
    }
}

fn is_different_float(a: f64, b: f64) -> bool {
    (a - b).abs() < ::std::f64::EPSILON
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

impl From<TrackListError> for EventError {
    fn from(error: TrackListError) -> EventError {
        EventError::TrackListError(error)
    }
}

impl From<DBusError> for EventError {
    fn from(error: DBusError) -> EventError {
        EventError::DBusError(error)
    }
}

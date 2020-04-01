use std::time::{Duration, Instant};

use super::{DBusError, LoopStatus, PlaybackStatus, TrackList, TrackListError};
use crate::extensions::DurationExtensions;
use crate::metadata::Metadata;
use crate::player::Player;
use crate::pooled_connection::MprisEvent;

/// Struct containing information about current progress of a Player.
///
/// It has access to the metadata of the current track, as well as information about the current
/// position of the track.
///
/// It is up to you to decide on how outdated information you want to rely on when implementing
/// progress rendering.
#[derive(Debug)]
pub struct Progress {
    metadata: Metadata,
    playback_status: PlaybackStatus,
    shuffle: bool,
    loop_status: LoopStatus,

    /// When this Progress was constructed, in order to calculate how old it is.
    instant: Instant,

    position: Duration,
    rate: f64,
    current_volume: f64,
}

/// Controller for calculating `Progress` and maintaining a `TrackList` (if supported) for a given `Player`.
///
/// Call the `tick` method to get the most current `Progress` data.
#[derive(Debug)]
pub struct ProgressTracker<'a> {
    player: &'a Player<'a>,
    track_list: Option<TrackList>,
    interval: Duration,
    last_tick: Instant,
    last_progress: Progress,
}

/// Return value of `ProgressTracker::tick`, which gives details about the latest refresh.
#[derive(Debug)]
pub struct ProgressTick<'a> {
    /// `true` if `Player` quit. This likely means that the player is no longer running.
    ///
    /// If the player is no longer running, then fetching new data will not be possible,
    /// so they will all be reused (`progress_changed` and `track_list_changed` should all be
    /// `false`).
    pub player_quit: bool,

    /// `true` if `Progress` data changed (beyond the calculated `position`)
    ///
    /// **Examples:**
    ///
    /// * Playback status changed
    /// * Metadata changed for the track
    /// * Volume was decreased
    pub progress_changed: bool,

    /// `true` if `TrackList` data changed. This will always be `false` if player does not support
    /// track lists.
    ///
    /// **Examples:**
    ///
    /// * Track was added
    /// * Track was removed
    /// * Metadata changed for a track
    pub track_list_changed: bool,

    /// The current `Progress` from the `ProgressTracker`. `progress_changed` tells you if this was
    /// reused from the last tick or if it's a new one.
    pub progress: &'a Progress,

    /// The current `TrackList` from the `ProgressTracker`. `track_list_changed` tells you if this was
    /// changed since the last tick.
    pub track_list: Option<&'a TrackList>,
}

/// Errors that can occur while refreshing progress.
#[derive(Debug, Fail)]
pub enum ProgressError {
    /// Something went wrong with the D-Bus communication. See the `DBusError` type.
    #[fail(display = "D-Bus communication failed")]
    DBusError(#[cause] DBusError),

    /// Something went wrong with the track list. See the `TrackListError` type.
    #[fail(display = "TrackList could not be refreshed")]
    TrackListError(#[cause] TrackListError),
}

impl<'a> ProgressTracker<'a> {
    /// Construct a new `ProgressTracker` for the provided `Player`.
    ///
    /// The `interval_ms` value is the desired time between ticks when calling the `tick` method.
    /// See `tick` for more information about that.
    ///
    /// You probably want to use `Player::track_progress` instead of this method.
    ///
    /// # Errors
    ///
    /// Returns an error in case Player metadata or state retrieval over DBus fails.
    pub fn new(player: &'a Player<'a>, interval_ms: u32) -> Result<Self, DBusError> {
        Ok(ProgressTracker {
            player,
            interval: Duration::from_millis(u64::from(interval_ms)),
            last_tick: Instant::now(),
            last_progress: Progress::from_player(player)?,
            track_list: player.checked_get_track_list()?,
        })
    }

    /// Returns a `ProgressTick` at each interval, or as close to each interval as possible.
    ///
    /// The returned struct contains borrows of the current data along with booleans telling you if
    /// the underlying data changed or not. See `ProgressTick` for more information about that.
    ///
    /// If there is time left until the next interval window, then the tracker will process DBus
    /// events to determine if something changed (and potentially perform a full refresh of the
    /// data). If there is no time left, then the previous data will be reused.
    ///
    /// If refreshing failed for some reason the old data will be reused.
    ///
    /// It is recommended to call this inside a loop to maintain your progress display.
    ///
    /// ## On reusing data
    ///
    /// `Progress` can be reused until something about the player changes, like track or playback
    /// status. As long as nothing changes, `Progress` can accurately determine playback position
    /// from timing data.
    ///
    /// In addition, `TrackList` will maintain a cache of track metadata so as long as the list
    /// remains static if should be cheap to read from it.
    ///
    /// You can use the `bool`s in the `ProgressTick` to perform optimizations as they tell you if
    /// any data has changed. If all of them are `false` you don't have to treat any of the data as
    /// dirty.
    ///
    /// The calculated `Progress::position` might still change depending on the player state, so if
    /// you want to show the track position you might still want to refresh that part.
    ///
    /// # Examples
    ///
    /// Simple progress bar of track position:
    ///
    /// ```rust,no_run
    /// # use mpris::{PlayerFinder, Metadata, PlaybackStatus, Progress};
    /// use mpris::ProgressTick;
    /// # use std::time::Duration;
    /// # fn update_progress_bar(_: Duration) { }
    /// # let player = PlayerFinder::new().unwrap().find_active().unwrap();
    /// #
    /// // Re-render progress bar every 100ms
    /// let mut progress_tracker = player.track_progress(100).unwrap();
    /// loop {
    ///     let ProgressTick {progress, ..} = progress_tracker.tick();
    ///     update_progress_bar(progress.position());
    /// }
    /// ```
    ///
    /// Using the `progress_changed` `bool`:
    ///
    /// ```rust,no_run
    /// # use mpris::PlayerFinder;
    /// use mpris::ProgressTick;
    /// # use std::time::Duration;
    /// # fn update_progress_bar(_: Duration) { }
    /// # fn reset_progress_bar(_: Duration, _: Option<Duration>) { }
    /// # fn update_track_title(_: Option<&str>) { }
    /// #
    /// # let player = PlayerFinder::new().unwrap().find_active().unwrap();
    /// #
    /// // Refresh every 100ms
    /// let mut progress_tracker = player.track_progress(100).unwrap();
    /// loop {
    ///     let ProgressTick {progress, progress_changed, ..} = progress_tracker.tick();
    ///     if progress_changed {
    ///         update_track_title(progress.metadata().title());
    ///         reset_progress_bar(progress.position(), progress.length());
    ///     } else {
    ///         update_progress_bar(progress.position());
    ///     }
    /// }
    /// ```
    ///
    /// Showing only the track list until the player quits:
    ///
    /// ```rust,no_run
    /// # use mpris::{PlayerFinder, TrackList};
    /// use mpris::ProgressTick;
    /// # fn render_track_list(_: &TrackList) { }
    /// # fn render_track_list_unavailable() { }
    /// #
    /// # let player = PlayerFinder::new().unwrap().find_active().unwrap();
    /// #
    /// // Refresh every 10 seconds
    /// let mut progress_tracker = player.track_progress(10_000).unwrap();
    /// loop {
    ///     let ProgressTick {track_list, track_list_changed, player_quit, ..} = progress_tracker.tick();
    ///     if player_quit {
    ///         break;
    ///     } else if track_list_changed {
    ///         if let Some(list) = track_list {
    ///             render_track_list(list);
    ///         } else {
    ///             render_track_list_unavailable();
    ///         }
    ///     }
    /// }
    /// ```
    pub fn tick(&mut self) -> ProgressTick {
        let mut player_quit = false;
        let mut progress_changed = false;
        let mut track_list_changed = false;
        let old_shuffle = self.last_progress.shuffle;

        // Calculate time left until we're expected to return with new data.
        let time_left = self
            .interval
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        // Refresh events if we're not late.
        if time_left > Duration::from_millis(0) {
            self.player
                .connection()
                .process_events_blocking_for(time_left);
        }

        // Process events that are queued up for us
        for event in self.player.pending_events().into_iter() {
            match event {
                MprisEvent::PlayerQuit => {
                    player_quit = true;
                    break;
                }
                MprisEvent::PlayerPropertiesChanged | MprisEvent::Seeked { .. } => {
                    if !progress_changed {
                        progress_changed |= self.refresh_player();
                    }
                }
                MprisEvent::TrackListPropertiesChanged => {
                    track_list_changed |= self.refresh_track_list();
                }
                MprisEvent::TrackListReplaced { ids } => {
                    if let Some(ref mut list) = self.track_list {
                        list.replace(ids.into_iter().collect());
                    }
                    track_list_changed = true;
                }
                MprisEvent::TrackAdded { after_id, metadata } => {
                    if let Some(ref mut list) = self.track_list {
                        list.insert(&after_id, metadata);
                    }
                    track_list_changed = true;
                }
                MprisEvent::TrackRemoved { id } => {
                    if let Some(ref mut list) = self.track_list {
                        list.remove(&id);
                    }
                    track_list_changed = true;
                }
                MprisEvent::TrackMetadataChanged { old_id, metadata } => {
                    if let Some(ref mut list) = self.track_list {
                        list.replace_track_metadata(&old_id, metadata);
                    }
                    track_list_changed = true;
                }
            }
        }

        if old_shuffle != self.last_progress.shuffle {
            // Shuffle changed, which means that the tracklist is likely to have been changed too.
            // Do a reload, even if track_list_changed was true so the correct order is loaded even
            // if only a in-place change took place before.
            track_list_changed |= self.refresh_track_list();
        }

        self.last_tick = Instant::now();
        ProgressTick {
            progress: &self.last_progress,
            track_list: self.track_list.as_ref(),
            player_quit,
            progress_changed,
            track_list_changed,
        }
    }

    /// Force a refresh right now.
    ///
    /// This will ignore the interval and perform a refresh anyway. The new `Progress` will be
    /// saved, and the `TrackList` will be refreshed.
    ///
    /// # Errors
    ///
    /// Returns an error if the refresh failed.
    pub fn force_refresh(&mut self) -> Result<(), ProgressError> {
        self.last_progress = Progress::from_player(self.player)?;
        if let Some(ref mut list) = self.track_list {
            list.reload(&self.player)?;
        }
        Ok(())
    }

    fn refresh_player(&mut self) -> bool {
        if let Ok(progress) = Progress::from_player(self.player) {
            self.last_progress = progress;
            return true;
        }
        false
    }

    fn refresh_track_list(&mut self) -> bool {
        match self.track_list {
            Some(ref mut list) => list.reload(&self.player).is_ok(),
            None => false,
        }
    }
}

impl Progress {
    pub(crate) fn from_player<'a>(player: &'a Player<'a>) -> Result<Progress, DBusError> {
        Ok(Progress {
            metadata: player.get_metadata()?,
            playback_status: player.get_playback_status()?,
            shuffle: player.checked_get_shuffle()?.unwrap_or(false),
            loop_status: player.get_loop_status()?,
            rate: player.get_playback_rate()?,
            position: player.get_position()?,
            current_volume: player.get_volume()?,
            instant: Instant::now(),
        })
    }

    /// The track metadata at the point in time that this Progress was constructed.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// The playback status at the point in time that this Progress was constructed.
    pub fn playback_status(&self) -> PlaybackStatus {
        self.playback_status
    }

    /// The shuffle status at the point in time that this Progress was constructed.
    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// The loop status at the point in time that this Progress was constructed.
    pub fn loop_status(&self) -> LoopStatus {
        self.loop_status
    }

    /// The playback rate at the point in time that this Progress was constructed.
    pub fn playback_rate(&self) -> f64 {
        self.rate
    }

    /// Returns the length of the current track as a `Duration`.
    pub fn length(&self) -> Option<Duration> {
        self.metadata.length()
    }

    /// Returns the current position of the current track as a `Duration`.
    ///
    /// This method will calculate the expected position of the track at the instant of the
    /// invocation using the `initial_position` and knowledge of how long ago that position was
    /// determined.
    ///
    /// **Note:** Some players might not support this and will return a bad position. Spotify is
    /// one such example. There is no reliable way of detecting problematic players, so it will be
    /// up to your client to check for this.
    ///
    /// One way of doing this is to query the `initial_position` for two measures with the
    /// `Playing` `PlaybackStatus` and if both are `0`, then it is likely that this client does not
    /// support positions.
    pub fn position(&self) -> Duration {
        self.position + self.elapsed()
    }

    /// Returns the position that the current track was at when the `Progress` was created.
    ///
    /// This is the number that was returned for the `Position` property in the MPRIS2 interface.
    pub fn initial_position(&self) -> Duration {
        self.position
    }

    /// The instant where this `Progress` was recorded.
    ///
    /// See: `age`.
    pub fn created_at(&self) -> &Instant {
        &self.instant
    }

    /// Returns the age of the data as a `Duration`.
    ///
    /// If the `Progress` has a high age it is more likely to be out of date.
    pub fn age(&self) -> Duration {
        self.instant.elapsed()
    }

    /// Returns the player's volume as it was at the time of refresh.
    ///
    /// See: `Player::get_volume`.
    pub fn current_volume(&self) -> f64 {
        self.current_volume
    }

    fn elapsed(&self) -> Duration {
        let elapsed_ms = match self.playback_status {
            PlaybackStatus::Playing => {
                DurationExtensions::as_millis(&self.age()) as f64 * self.rate
            }
            _ => 0.0,
        };
        Duration::from_millis(elapsed_ms as u64)
    }
}

impl From<TrackListError> for ProgressError {
    fn from(error: TrackListError) -> ProgressError {
        ProgressError::TrackListError(error)
    }
}

impl From<DBusError> for ProgressError {
    fn from(error: DBusError) -> ProgressError {
        ProgressError::DBusError(error)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_progresses_position_when_playing_at_microseconds() {
        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Playing,
            shuffle: false,
            loop_status: LoopStatus::None,
            rate: 1.0,
            position: Duration::from_micros_ext(1),
            current_volume: 0.0,
            instant: Instant::now(),
        };

        assert_eq!(progress.initial_position(), Duration::from_micros_ext(1));
        assert!(progress.position() >= progress.initial_position());
    }

    #[test]
    fn it_does_not_progress_when_paused() {
        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Paused,
            shuffle: false,
            loop_status: LoopStatus::None,
            rate: 1.0,
            position: Duration::from_micros_ext(1336),
            current_volume: 0.0,
            instant: Instant::now() - Duration::from_millis(500),
        };

        assert_eq!(progress.position(), progress.initial_position());
    }
}

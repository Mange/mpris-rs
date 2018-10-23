use std::time::{Duration, Instant};

use super::{DBusError, LoopStatus, PlaybackStatus, TrackList};
use extensions::DurationExtensions;
use metadata::Metadata;
use player::Player;

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

/// Controller for calculating `Progress` and maintaining a `TrackList` for a given `Player`.
///
/// Call the `tick` method to get the most current `Progress` data.
#[derive(Debug)]
pub struct ProgressTracker<'a, 'b> {
    player: &'a Player<'a>,
    track_list: TrackList<'b>,
    interval: Duration,
    last_tick: Instant,
    last_progress: Progress,
}

/// Return value of `ProgressTracker::tick`, which gives details about the latest refresh.
#[derive(Debug)]
pub struct ProgressTick<'a> {
    /// `true` if `Progress` data changed (beyond the calculated `position`)
    ///
    /// **Examples:**
    ///
    /// * Playback status changed
    /// * Metadata changed for the track
    /// * Volume was decreased
    pub progress_changed: bool,

    /// `true` if `TrackList` data changed.
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
    pub track_list: &'a TrackList<'a>,
}

impl<'a, 'b> ProgressTracker<'a, 'b> {
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
            track_list: player.get_track_list()?,
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
    /// Showing only the track list:
    ///
    /// ```rust,no_run
    /// # use mpris::{PlayerFinder, TrackList};
    /// use mpris::ProgressTick;
    /// # fn render_track_list(_: &TrackList) { }
    /// #
    /// # let player = PlayerFinder::new().unwrap().find_active().unwrap();
    /// #
    /// // Refresh every 10 seconds
    /// let mut progress_tracker = player.track_progress(10_000).unwrap();
    /// loop {
    ///     let ProgressTick {track_list, track_list_changed, ..} = progress_tracker.tick();
    ///     if track_list_changed {
    ///         render_track_list(track_list);
    ///     }
    /// }
    /// ```
    pub fn tick(&mut self) -> ProgressTick {
        let mut did_refresh = false;

        // Calculate time left until we're expected to return with new data.
        let time_left = self
            .interval
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        // Refresh events if we're not late.
        if time_left > Duration::from_millis(0) {
            self.player.connection().process_events_blocking(time_left);
        }

        // If we got a new event since the last time we ticked, then reload fresh data.
        if self
            .player
            .connection()
            .is_bus_updated_after(self.player.unique_name(), &self.last_tick)
        {
            // TODO: Right now the track list and the progress are always reloaded at the same
            // times. Processing events should separate them and tell you which ones needs to be
            // refreshed instead.
            did_refresh = self.refresh();
        }

        self.last_tick = Instant::now();
        ProgressTick {
            progress: &self.last_progress,
            track_list: &self.track_list,
            progress_changed: did_refresh,
            track_list_changed: did_refresh,
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
    pub fn force_refresh(&mut self) -> Result<(), DBusError> {
        self.last_progress = Progress::from_player(self.player)?;
        self.track_list.reload(&self.player)?;
        Ok(())
    }

    fn refresh(&mut self) -> bool {
        if let Ok(progress) = Progress::from_player(self.player) {
            self.last_progress = progress;
            return true;
        }
        false
    }
}

impl Progress {
    pub(crate) fn from_player<'a>(player: &'a Player<'a>) -> Result<Progress, DBusError> {
        Ok(Progress {
            metadata: player.get_metadata()?,
            playback_status: player.get_playback_status()?,
            shuffle: player.get_shuffle()?,
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

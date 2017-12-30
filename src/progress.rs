use super::PlaybackStatus;
use metadata::Metadata;
use player::Player;
use std::time::{Duration, Instant};
use extensions::DurationExtensions;
use failure::Error;

/// Struct containing information about current progress of a Player.
///
/// It has access to the metadata of the current track, as well as information about the current
/// position of the track.
///
/// It is up to you to decide on how outdated information you want to rely on when implementing
/// progress rendering.
#[derive(Debug)]
pub struct Progress {
    /// The track metadata at the point in time that this Progress was constructed.
    pub metadata: Metadata,
    /// The playback status at the point in time that this Progress was constructed.
    pub playback_status: PlaybackStatus,

    /// When this Progress was constructed, in order to calculate how old it is.
    instant: Instant,

    position_in_microseconds: u64,
    rate: f32,
    is_spotify: bool,
}

/// Controller for calculating Progress for a given Player.
///
/// Call the `tick` method to get the most current Progress data.
#[derive(Debug)]
pub struct ProgressTracker<'a> {
    player: &'a Player<'a>,
    interval: Duration,
    last_tick: Instant,
    last_progress: Progress,
}

impl<'a> ProgressTracker<'a> {
    /// Construct a new ProgressTracker for the provided Player.
    ///
    /// The `interval_ms` value is the desired time between ticks when calling the `tick` method.
    /// See `tick` for more information about that.
    ///
    /// You probably want to use `Player::track_progress` instead of this method.
    ///
    /// # Errors
    ///
    /// Returns an error in case Player metadata or state retrieval over DBus fails.
    pub fn new(player: &'a Player<'a>, interval_ms: u32) -> Result<Self, Error> {
        Ok(ProgressTracker {
            player: player,
            interval: Duration::from_millis(u64::from(interval_ms)),
            last_tick: Instant::now(),
            last_progress: Progress::from_player(player)?,
        })
    }

    /// Returns a (`Progress`, `bool`) pair at each interval, or as close to each interval as
    /// possible.
    ///
    /// The `bool` is `true` if the `Progress` was refreshed, or `false` if the old `Progress` was
    /// reused.
    ///
    /// If there is time left until the next interval window, then the tracker will process DBus
    /// events to determine if something changed (and potentially perform a full refresh). If there
    /// is no time left, then the previous `Progress` will be returned again.
    ///
    /// If refreshing failed for some reason the old `Progress` will be returned.
    ///
    /// It is recommended to call this inside a loop to maintain your progress display.
    ///
    /// ## On reusing `Progress` instances
    ///
    /// `Progress` can be reused until something about the player changes, like track or playback
    /// status. As long as nothing changes, `Progress` can accurately determine playback position
    /// from timing data.
    ///
    /// You can use the returned `bool` in order to perform similar optimizations yourself, as a
    /// `false` value means that nothing (except potentially `position`) changed.
    ///
    /// # Examples
    ///
    /// Simple progress tracker:
    ///
    /// ```rust,no_run
    /// # use mpris::{PlayerFinder, Metadata, PlaybackStatus, Progress};
    /// # use std::time::Duration;
    /// # fn update_progress_bar(_: Duration) { }
    /// # let player = PlayerFinder::new().unwrap().find_active().unwrap();
    /// #
    /// // Refresh every 100ms
    /// let mut progress_tracker = player.track_progress(100).unwrap();
    /// loop {
    ///     let (progress, _) = progress_tracker.tick();
    ///     update_progress_bar(progress.position());
    /// }
    /// ```
    ///
    /// Using the `was_refreshed` `bool`:
    ///
    /// ```rust,no_run
    /// # use mpris::PlayerFinder;
    /// # use std::time::Duration;
    /// # fn update_progress_bar(_: Duration) { }
    /// # fn reset_progress_bar(_: Duration, _: Option<Duration>) { }
    /// # fn update_track_title(_: &Option<String>) { }
    /// #
    /// # let player = PlayerFinder::new().unwrap().find_active().unwrap();
    /// #
    /// // Refresh every 100ms
    /// let mut progress_tracker = player.track_progress(100).unwrap();
    /// loop {
    ///     let (progress, was_changed) = progress_tracker.tick();
    ///     if was_changed {
    ///         update_track_title(&progress.metadata.title);
    ///         reset_progress_bar(progress.position(), progress.length());
    ///     } else {
    ///         update_progress_bar(progress.position());
    ///     }
    /// }
    /// ```
    pub fn tick(&mut self) -> (&Progress, bool) {
        let mut did_refresh = false;

        // Calculate time left until we're expected to return with new data.
        let time_left = self.interval
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        // Refresh events if we're not late.
        if time_left > Duration::from_millis(0) {
            self.player.connection().process_events_blocking(time_left);
        }

        // If we got a new event since the last time we ticked, then reload fresh data.
        if let Some(last_event_at) =
            self.player.connection().last_event_for_unique_name(
                self.player.unique_name(),
            )
        {
            if last_event_at > self.last_tick {
                did_refresh = self.refresh();
            }
        }

        (self.progress(), did_refresh)
    }

    /// Force a refresh right now.
    ///
    /// This will ignore the interval and perform a refresh anyway, storing the result as the last
    /// `Progress` value.
    ///
    /// # Errors
    ///
    /// Returns an error if the refresh failed.
    pub fn force_refresh(&mut self) -> Result<(), Error> {
        Progress::from_player(self.player).map(|progress| { self.last_progress = progress; })
    }

    fn progress(&mut self) -> &Progress {
        self.last_tick = Instant::now();
        &self.last_progress
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
    fn from_player<'a>(player: &'a Player<'a>) -> Result<Progress, Error> {
        Ok(Progress {
            metadata: player.get_metadata()?,
            playback_status: player.get_playback_status()?,
            rate: player.get_playback_rate()?,
            position_in_microseconds: player.get_position_in_microseconds()?,
            is_spotify: player.identity() == "Spotify",
            instant: Instant::now(),
        })
    }

    /// Returns the length of the current track as a `Duration`.
    pub fn length(&self) -> Option<Duration> {
        self.metadata.length_in_microseconds.map(
            Duration::from_micros_ext,
        )
    }

    /// Returns the current position of the current track as a `Duration`.
    ///
    /// This method will calculate the expected position of the track at the instant of the
    /// invocation using the `initial_position` and knowledge of how long ago that position was
    /// determined.
    ///
    /// **Note:** Some players might not support this. Spotify is one such example. You can test
    /// for known problem players using the `supports_position` method.
    pub fn position(&self) -> Duration {
        self.initial_position() + self.elapsed()
    }

    /// Returns the position that the current track was at when the `Progress` was created.
    pub fn initial_position(&self) -> Duration {
        Duration::from_micros_ext(self.position_in_microseconds)
    }

    /// Returns `false` if the current player is known to not support the `position` field.
    ///
    /// You can optionally use this in order to display an undetermined position, as an example.
    pub fn supports_position(&self) -> bool {
        // Spotify does not support position at this time. It always returns 0, no matter what.
        // Still make sure it's 0 in case Spotify later starts to support it.
        !(self.is_spotify && self.position_in_microseconds == 0)
    }

    /// Returns the age of the data as a `Duration`.
    ///
    /// If the `Progress` has a high age it is more likely to be out of date.
    pub fn age(&self) -> Duration {
        self.instant.elapsed()
    }

    fn elapsed(&self) -> Duration {
        let elapsed_ms = match self.playback_status {
            PlaybackStatus::Playing => self.age().as_millis() as f32 * self.rate,
            _ => 0.0,
        };
        Duration::from_millis(elapsed_ms as u64)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_does_not_support_position_when_player_is_spotify() {
        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Playing,
            rate: 1.0,
            position_in_microseconds: 0,
            instant: Instant::now(),
            is_spotify: true,
        };

        assert!(!progress.supports_position());

        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Playing,
            rate: 1.0,
            position_in_microseconds: 0,
            instant: Instant::now(),
            is_spotify: false,
        };

        assert!(progress.supports_position());
    }

    #[test]
    fn it_progresses_position_when_playing_at_microseconds() {
        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Playing,
            rate: 1.0,
            position_in_microseconds: 1,
            instant: Instant::now(),
            is_spotify: false,
        };

        assert_eq!(progress.initial_position(), Duration::from_micros_ext(1));
        assert!(progress.position() >= progress.initial_position());
    }

    #[test]
    fn it_does_not_progress_when_paused() {
        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Paused,
            rate: 1.0,
            position_in_microseconds: 1336,
            instant: Instant::now() - Duration::from_millis(500),
            is_spotify: false,
        };

        assert_eq!(progress.position(), progress.initial_position());
    }
}

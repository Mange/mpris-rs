use super::PlaybackStatus;
use dbus::Connection;
use metadata::Metadata;
use player::Player;
use prelude::*;
use std::ops::Deref;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Progress {
    pub metadata: Metadata,
    pub playback_status: PlaybackStatus,
    instant: Instant,
    position_in_microseconds: u64,
    rate: f32,
    is_spotify: bool,
}

#[derive(Debug)]
pub struct ProgressTracker<'a> {
    player: &'a Player<'a>,
    interval: Duration,
    last_tick: Instant,
    last_progress: Progress,
}

pub(crate) trait DurationExtensions {
    // Rust beta has a from_micros function that is unstable.
    fn from_micros_ext(u64) -> Duration;
    fn as_millis(&self) -> u64;
}

impl DurationExtensions for Duration {
    fn from_micros_ext(micros: u64) -> Duration {
        Duration::from_millis(micros / 1000)
    }

    fn as_millis(&self) -> u64 {
        self.as_secs() * 1000 + (self.subsec_nanos() / 1000 / 1000) as u64
    }
}

impl<'a> ProgressTracker<'a> {
    pub fn new(
        player: &'a Player<'a>,
        interval_ms: u32,
        metadata: Metadata,
        playback_status: PlaybackStatus,
    ) -> Result<Self> {
        Ok(ProgressTracker {
            player: player,
            interval: Duration::from_millis(interval_ms as u64),
            last_tick: Instant::now(),
            last_progress: Progress::from_player(player)?,
        })
    }

    fn progress(&mut self) -> &Progress {
        self.last_tick = Instant::now();
        &self.last_progress
    }

    fn refresh(&mut self) {
        if let Ok(progress) = Progress::from_player(self.player) {
            self.last_progress = progress;
        }
    }

    pub fn tick(&mut self) -> (&Progress, bool) {
        let mut did_refresh = false;

        // Calculate time left until we're expected to return with new data.
        let time_left = self.interval
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or(Duration::from_millis(0));

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
                did_refresh = true;
                self.refresh();
            }
        }

        return (self.progress(), did_refresh);
    }
}

impl Progress {
    fn from_player<'a>(player: &'a Player<'a>) -> Result<Progress> {
        Ok(Progress {
            metadata: player.get_metadata()?,
            playback_status: player.get_playback_status()?,
            rate: player.get_playback_rate()?,
            position_in_microseconds: player.get_position_in_microseconds()?,
            is_spotify: player.identity() == "Spotify",
            instant: Instant::now(),
        })
    }

    pub fn length(&self) -> Option<Duration> {
        self.metadata.length_in_microseconds.map(
            Duration::from_micros_ext,
        )
    }

    pub fn position(&self) -> Duration {
        self.initial_position() + self.elapsed()
    }

    pub fn initial_position(&self) -> Duration {
        Duration::from_micros_ext(self.position_in_microseconds)
    }

    fn elapsed(&self) -> Duration {
        let elapsed_ms = match self.playback_status {
            PlaybackStatus::Playing => self.instant.elapsed().as_millis() as f32 * self.rate,
            _ => 0.0,
        };
        Duration::from_millis(elapsed_ms as u64)
    }

    pub fn supports_position(&self) -> bool {
        // Spotify does not support position at this time. It always returns 0, no matter what.
        // Still make sure it's 0 in case Spotify later starts to support it.
        !(self.is_spotify && self.position_in_microseconds == 0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_calculates_whole_millis_from_durations() {
        let duration = Duration::new(5, 543_210_000);
        assert_eq!(duration.as_millis(), 5543);
    }

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

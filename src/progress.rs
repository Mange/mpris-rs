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
pub struct ProgressTracker<'conn, C: 'conn + Deref<Target = Connection>> {
    player: &'conn Player<'conn, C>,
    interval: Duration,
    last_tick: Instant,
    last_progress: Progress,
}

trait DurationExtensions {
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

impl<'conn, C: 'conn + Deref<Target = Connection>> ProgressTracker<'conn, C> {
    pub fn new(
        player: &'conn Player<'conn, C>,
        interval_ms: u32,
        metadata: Metadata,
        playback_status: PlaybackStatus,
    ) -> Result<Self> {
        player.connection().add_match(
            "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'",
        )?;
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

    pub fn tick(&mut self) -> &Progress {
        let mut should_refresh = false;

        // Is time already up?
        if self.last_tick.elapsed() >= self.interval {
            return self.progress();
        }

        // Try to read messages util time is up. Keep going with smaller and smaller windows until
        // our time is up.
        loop {
            let ms_left = self.interval
                .checked_sub(self.last_tick.elapsed())
                .map(|d| d.as_millis())
                .unwrap_or(0);
            // Don't bother if we have very little time left
            if ms_left < 2 {
                break;
            }
            match self.player.connection().incoming(ms_left as u32).next() {
                Some(_) => {
                    // If it's a matching message, we should refresh.
                    // TODO: Don't refresh on all messages.
                    should_refresh = true;
                }
                None => {
                    // Time is up. No more messages.
                    break;
                }
            }
        }

        if should_refresh {
            self.refresh();
        }

        return self.progress();
    }
}

impl Progress {
    fn from_player<'conn, C: 'conn + Deref<Target = Connection>>(
        player: &Player<'conn, C>,
    ) -> Result<Progress> {
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

    pub fn position(&self) -> Option<Duration> {
        self.initial_position().and_then(
            |pos| pos.checked_add(self.elapsed()),
        )
    }

    pub fn initial_position(&self) -> Option<Duration> {
        if self.supports_position() {
            Some(Duration::from_micros_ext(self.position_in_microseconds))
        } else {
            None
        }
    }

    fn elapsed(&self) -> Duration {
        let elapsed_ms = match self.playback_status {
            PlaybackStatus::Playing => self.instant.elapsed().as_millis() as f32 * self.rate,
            _ => 0.0,
        };
        Duration::from_millis(elapsed_ms as u64)
    }

    fn supports_position(&self) -> bool {
        // Spotify does not support position at this time. It always returns 0, no matter what.
        !self.is_spotify
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
    fn it_has_no_position_when_player_is_spotify() {
        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Playing,
            rate: 1.0,
            position_in_microseconds: 1337,
            instant: Instant::now(),
            is_spotify: true,
        };

        assert!(progress.initial_position().is_none());
        assert!(progress.position().is_none());

        let progress = Progress {
            metadata: Metadata::new(String::from("id")),
            playback_status: PlaybackStatus::Playing,
            rate: 1.0,
            position_in_microseconds: 1337,
            instant: Instant::now(),
            is_spotify: false,
        };

        assert!(progress.initial_position().is_some());
        assert!(progress.position().is_some());
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

        assert_eq!(
            progress.initial_position().unwrap(),
            Duration::from_micros_ext(1)
        );
        assert!(progress.position().unwrap() >= progress.initial_position().unwrap());
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

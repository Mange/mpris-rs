use std::time::Duration;

pub(crate) trait DurationExtensions {
    // Rust beta has a from_micros function that is unstable.
    fn from_micros_ext(u64) -> Duration;
    fn as_millis(&self) -> u64;
    fn as_micros(&self) -> u64;
}

impl DurationExtensions for Duration {
    fn from_micros_ext(micros: u64) -> Duration {
        Duration::from_millis(micros / 1000)
    }

    fn as_millis(&self) -> u64 {
        self.as_secs() * 1000 + (self.subsec_nanos() / 1000 / 1000) as u64
    }

    fn as_micros(&self) -> u64 {
        self.as_secs() * 1000 * 1000 + (self.subsec_nanos() / 1000) as u64
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
    fn it_calculates_whole_micros_from_durations() {
        let duration = Duration::new(5, 543_210_000);
        assert_eq!(duration.as_micros(), 5_543_210);
    }
}

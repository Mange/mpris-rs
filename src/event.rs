use super::{DBusError, PlaybackStatus, Player, Progress};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Paused,
    Playing,
    Stopped,
}

#[derive(Debug)]
pub struct PlayerEvents<'a> {
    player: &'a Player<'a>,

    buffer: Vec<Event>,

    last_progress: Progress,
}

impl<'a> PlayerEvents<'a> {
    pub fn new(player: &'a Player<'a>) -> Result<PlayerEvents<'a>, DBusError> {
        let progress = Progress::from_player(player)?;
        Ok(PlayerEvents {
            player,
            buffer: Vec::new(),
            last_progress: progress,
        })
    }

    fn read_events(&mut self) -> Result<(), DBusError> {
        self.player.process_events_blocking_until_dirty();

        let new_progress = Progress::from_player(self.player)?;

        //
        // Detect changes
        //
        match new_progress.playback_status() {
            status if self.last_progress.playback_status() == status => {}
            PlaybackStatus::Playing => self.buffer.push(Event::Playing),
            PlaybackStatus::Paused => self.buffer.push(Event::Paused),
            PlaybackStatus::Stopped => self.buffer.push(Event::Stopped),
        }

        self.last_progress = new_progress;
        Ok(())
    }
}

impl<'a> Iterator for PlayerEvents<'a> {
    type Item = Result<Event, DBusError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_events() {
            Ok(_) => {}
            Err(err) => return Some(Err(err)),
        };

        debug_assert!(
            !self.buffer.is_empty(),
            "Internal Events buffer is empty, which should never happen!"
        );
        let event = self.buffer.remove(0);
        Some(Ok(event))
    }
}

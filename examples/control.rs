extern crate mpris;
extern crate termion;

use std::borrow::Cow;
use std::io::{stdout, Stdout, Write};
use std::time::Duration;

use mpris::{LoopStatus, PlaybackStatus, Player, PlayerFinder, Progress, ProgressTracker};
use termion::color;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;

const REFRESH_INTERVAL: u32 = 100; // ms

type Screen = AlternateScreen<RawTerminal<Stdout>>;

#[derive(Clone, Copy)]
enum Action {
    Quit,
    PlayPause,
    Stop,
    Next,
    Previous,
    SeekForwards,
    SeekBackwards,
    ToggleShuffle,
    CycleLoopStatus,
}

const ACTIONS: &[Action] = &[
    Action::PlayPause,
    Action::Stop,
    Action::Next,
    Action::Previous,
    Action::ToggleShuffle,
    Action::CycleLoopStatus,
    Action::SeekForwards,
    Action::SeekBackwards,
    Action::Quit,
];

impl Action {
    fn from_key(key: termion::event::Key) -> Option<Action> {
        use Action::*;
        use termion::event::Key;

        match key {
            Key::Ctrl('c') | Key::Esc | Key::Char('q') => Some(Quit),
            Key::Char(' ') => Some(PlayPause),
            Key::Char('s') => Some(Stop),
            Key::Char('n') => Some(Next),
            Key::Char('p') => Some(Previous),
            Key::Char('z') => Some(ToggleShuffle),
            Key::Char('x') => Some(CycleLoopStatus),
            Key::Left => Some(SeekForwards),
            Key::Right => Some(SeekBackwards),
            _ => None,
        }
    }

    fn key_name(&self) -> &'static str {
        match *self {
            Action::Quit => "q",
            Action::PlayPause => "Space",
            Action::Stop => "s",
            Action::Next => "n",
            Action::Previous => "p",
            Action::ToggleShuffle => "z",
            Action::CycleLoopStatus => "x",
            Action::SeekForwards => "Left",
            Action::SeekBackwards => "Right",
        }
    }

    fn description(&self) -> &'static str {
        match *self {
            Action::Quit => "Quit",
            Action::PlayPause => "Toggle play/pause",
            Action::Stop => "Stop",
            Action::Next => "Next media",
            Action::Previous => "Previous media",
            Action::ToggleShuffle => "Toggle shuffle",
            Action::CycleLoopStatus => "Cycle loop status",
            Action::SeekForwards => "Seek 5s forward",
            Action::SeekBackwards => "Seek 5s backward",
        }
    }

    fn is_enabled(&self, player: &Player) -> bool {
        match *self {
            Action::Quit => true,
            Action::PlayPause => player.can_pause().unwrap_or(false),
            Action::Stop => player.can_stop().unwrap_or(false),
            Action::Next => player.can_go_next().unwrap_or(false),
            Action::Previous => player.can_go_previous().unwrap_or(false),
            Action::ToggleShuffle => player.can_control().unwrap_or(false),
            Action::CycleLoopStatus => player.can_control().unwrap_or(false),
            Action::SeekForwards | Action::SeekBackwards => player.can_seek().unwrap_or(false),
        }
    }

    fn should_exit(&self) -> bool {
        match *self {
            Action::Quit => true,
            _ => false,
        }
    }
}

struct App<'a> {
    player: &'a Player<'a>,
    progress_tracker: ProgressTracker<'a>,
    stdin: termion::AsyncReader,
    screen: Screen,
}

impl<'a> App<'a> {
    fn main_loop(&mut self) {
        let mut should_continue = true;

        // Start on true so first iteration refreshes everything; after that it will be set to
        // false until something causes a change.
        let mut should_refresh = true;

        while should_continue {
            while let Some(action) = self.next_action() {
                self.perform_action(action);
                if action.should_exit() {
                    should_continue = false;
                }
                should_refresh = true;
            }

            self.tick_progress_and_refresh(should_refresh);
            should_refresh = false;
        }
    }

    fn next_action(&mut self) -> Option<Action> {
        (&mut self.stdin)
            .keys()
            .next()
            .and_then(|result| result.ok())
            .and_then(Action::from_key)
    }

    fn perform_action(&mut self, action: Action) {
        match action {
            Action::Quit => (),
            Action::PlayPause => control_player(self.player.play_pause()),
            Action::Stop => control_player(self.player.stop()),
            Action::Next => control_player(self.player.next()),
            Action::Previous => control_player(self.player.previous()),
            Action::ToggleShuffle => control_player(toggle_shuffle(self.player)),
            Action::CycleLoopStatus => control_player(cycle_loop_status(self.player)),
            Action::SeekBackwards => {
                control_player(self.player.seek_backwards(&Duration::new(5, 0)))
            }
            Action::SeekForwards => control_player(self.player.seek_forwards(&Duration::new(5, 0))),
        };
    }

    fn tick_progress_and_refresh(&mut self, should_refresh: bool) {
        let supports_position = self.supports_position();
        let (progress, was_changed) = self.progress_tracker.tick();

        // Dirty tracking to keep CPU usage lower. In case nothing happened since the last refresh,
        // only update the progress bar.
        //
        // If player doesn't support position handling, don't even try to refresh the progress bar
        // if no event took place.
        if was_changed || should_refresh {
            clear_screen(&mut self.screen);
            print_instructions(&mut self.screen, self.player);
            print_track_info(&mut self.screen, progress);
            print_progress_bar(&mut self.screen, progress, supports_position);
        } else if supports_position {
            clear_progress_bar(&mut self.screen);
            print_progress_bar(&mut self.screen, progress, supports_position);
        }

        self.screen.flush().unwrap();
    }

    fn supports_position(&self) -> bool {
        self.player.identity() != "Spotify"
    }
}

fn print_instructions(screen: &mut Screen, player: &Player) {
    let bold = termion::style::Bold;
    let nobold = termion::style::NoBold;

    write!(
        screen,
        "{}Instructions for controlling {}:{}\r\n",
        bold,
        player.identity(),
        nobold,
    ).unwrap();

    for action in ACTIONS {
        let is_enabled = action.is_enabled(player);

        if is_enabled {
            write!(screen, "{}", color::Fg(color::White)).unwrap();
        } else {
            write!(screen, "{}", color::Fg(color::LightBlack)).unwrap();
        };

        write!(
            screen,
            "  {bold}{key:5}{nobold} - {description}",
            bold = bold,
            nobold = nobold,
            key = action.key_name(),
            description = action.description(),
        ).unwrap();

        if !is_enabled {
            write!(screen, " (not supported)").unwrap();
        }

        write!(
            screen,
            "{nostatecolor}\r\n",
            nostatecolor = color::Fg(color::Reset),
        ).unwrap();
    }
    write!(screen, "\r\n").unwrap();
}

fn control_player(result: Result<(), mpris::DBusError>) {
    result.expect("Could not control player");
}

fn toggle_shuffle(player: &Player) -> Result<(), mpris::DBusError> {
    player.set_shuffle(!player.get_shuffle()?)
}

fn cycle_loop_status(player: &Player) -> Result<(), mpris::DBusError> {
    let current_status = player.get_loop_status()?;
    let next_status = match current_status {
        LoopStatus::None => LoopStatus::Playlist,
        LoopStatus::Playlist => LoopStatus::Track,
        LoopStatus::Track => LoopStatus::None,
    };
    player.set_loop_status(next_status)
}

fn print_track_info(screen: &mut Screen, progress: &Progress) {
    let metadata = &(progress.metadata);

    let artist_string: Cow<str> = metadata
        .artists
        .as_ref()
        .map(|artists| Cow::Owned(artists.join(" + ")))
        .unwrap_or_else(|| Cow::Borrowed("Unknown artist"));

    let title_string: Cow<str> = metadata
        .title
        .as_ref()
        .map(|s| Cow::Owned(s.clone()))
        .unwrap_or_else(|| Cow::Borrowed("Unkown title"));

    let playback_string = match progress.playback_status {
        PlaybackStatus::Playing => format!("{}▶", color::Fg(color::Green)),
        PlaybackStatus::Paused => format!("{}▮▮", color::Fg(color::LightBlack)),
        PlaybackStatus::Stopped => format!("{}◼", color::Fg(color::Red)),
    };

    let shuffle_string = if progress.shuffle {
        format!("{}🔀", color::Fg(color::Green))
    } else {
        format!("{}🔀", color::Fg(color::LightBlack))
    };

    let loop_string = match progress.loop_status {
        LoopStatus::None => format!("{}🔁", color::Fg(color::LightBlack)),
        LoopStatus::Playlist => format!("{}🔁", color::Fg(color::Green)),
        LoopStatus::Track => format!("{}🔂", color::Fg(color::Yellow)),
    };

    write!(
        screen,
        "{playback} {shuffle} {loop} {color_reset} {blue}{bold}{artist}{nobold} - {title}{color_reset}\r\n",
        playback = playback_string,
        shuffle = shuffle_string,
        loop = loop_string,
        blue = color::Fg(color::Blue),
        color_reset = color::Fg(color::Reset),
        bold = termion::style::Bold,
        nobold = termion::style::NoBold,
        artist = artist_string,
        title = title_string,
    ).unwrap();
}

fn print_progress_bar(screen: &mut Screen, progress: &Progress, supports_position: bool) {
    let position_string: Cow<str> = if supports_position {
        Cow::Owned(format_duration(progress.position()))
    } else {
        Cow::Borrowed("??:??:??")
    };

    let length_string: Cow<str> = progress
        .length()
        .map(|s| Cow::Owned(format_duration(s)))
        .unwrap_or_else(|| Cow::Borrowed("??:??:??"));

    write!(
        screen,
        "{position} / {length}\r\n",
        position = position_string,
        length = length_string,
    ).unwrap();
}

fn clear_progress_bar(screen: &mut Screen) {
    write!(
        screen,
        "{}\r{}",
        termion::cursor::Up(1),
        termion::clear::CurrentLine,
    ).unwrap();
}

fn clear_screen(screen: &mut Screen) {
    write!(
        screen,
        "{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1)
    ).unwrap();
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let whole_hours = secs / (60 * 60);

    let secs = secs - whole_hours * 60 * 60;
    let whole_minutes = secs / 60;

    let secs = secs - whole_minutes * 60;

    format!("{:02}:{:02}:{:02}", whole_hours, whole_minutes, secs)
}

fn main() {
    let player = PlayerFinder::new()
        .unwrap()
        .find_active()
        .expect("Could not find a running player");
    let progress_tracker = player
        .track_progress(REFRESH_INTERVAL)
        .expect("Could not determine progress of player");

    let stdout = stdout().into_raw_mode().unwrap();

    let mut app = App {
        player: &player,
        progress_tracker: progress_tracker,
        screen: AlternateScreen::from(stdout),
        stdin: termion::async_stdin(),
    };

    app.main_loop();
}

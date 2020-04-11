use mpris::{PlaybackStatus, PlayerFinder};

fn main() {
    match play_pause() {
        Ok(playback_status) => match playback_status {
            PlaybackStatus::Playing => println!("Player is now playing."),
            PlaybackStatus::Paused => println!("Player is now paused."),
            PlaybackStatus::Stopped => println!("Player is stopped."),
        },
        Err(error) => {
            println!("ERROR: {}", error);
            std::process::exit(1);
        }
    }
}

fn play_pause() -> Result<PlaybackStatus, String> {
    let player_finder =
        PlayerFinder::new().map_err(|e| format!("Could not connect to D-Bus: {}", e))?;

    let player = player_finder
        .find_active()
        .map_err(|e| format!("Could not find any player: {}", e))?;

    let toggled = player
        .checked_play_pause()
        .map_err(|e| format!("Could not control player: {}", e))?;

    if toggled {
        // Give the player some time to respond to the message and update its properties. The
        // play_pause() call will wait for a reply, but the player might not update the properties
        // before replying.
        std::thread::sleep(std::time::Duration::from_millis(50));

        player
            .get_playback_status()
            .map_err(|e| format!("Could not get playback status: {}", e))
    } else {
        // Could not toggle play/pause status. This happens when the media cannot be paused, which
        // could be because of any number of reasons including:
        //   - No media is playing
        //   - Media is streaming and does not allow pause
        Err(String::from("Media cannot be paused"))
    }
}

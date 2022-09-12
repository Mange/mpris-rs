use anyhow::{Context, Result};
use mpris::{DBusError, Player, PlayerFinder};
use std::borrow::Cow;

const VALUE_INDENTATION: usize = 25;

trait CustomDisplay {
    fn string_for_display(&self) -> Cow<'_, str>;
}

fn main() {
    match print_capabilities_for_all_players() {
        Ok(_) => {}
        Err(error) => {
            println!("Error: {}", error);
            for (i, cause) in error.chain().skip(1).enumerate() {
                print!("{}", "  ".repeat(i + 1));
                println!("Caused by: {}", cause);
            }
            std::process::exit(1);
        }
    }
}

fn print_capabilities_for_all_players() -> Result<()> {
    for player in PlayerFinder::new()
        .context("Failed to connect to D-Bus")?
        .find_all()
        .context("Could not fetch list of players")?
    {
        print_capabilities_for_player(player)?;
        println!();
    }

    Ok(())
}

fn print_capabilities_for_player(player: Player) -> Result<()> {
    println!(
        ">> Player: {} ({})",
        player.identity(),
        player.unique_name()
    );

    println!();
    println!("\t─── MediaPlayer2 ───");
    print_value("CanQuit", player.can_quit());
    print_value("CanRaise", player.can_raise());
    print_value("CanSetFullscreen", player.can_set_fullscreen());
    print_value("HasTrackList", player.get_has_track_list());
    print_value("SupportedMimeTypes", player.get_supported_mime_types());
    print_value("SupportedUriSchemes", player.get_supported_uri_schemes());

    println!();
    println!("\t─── MediaPlayer2.Player ───");
    print_value("CanControl", player.can_control());
    print_value("CanGoNext", player.can_go_next());
    print_value("CanGoPrevious", player.can_go_previous());
    print_value("CanLoop", player.can_loop());
    print_value("CanPause", player.can_pause());
    print_value("CanPlay", player.can_play());
    print_value("CanSeek", player.can_seek());
    print_value("CanSetPaybackRate", player.can_set_playback_rate());
    print_value("CanShuffle", player.can_shuffle());
    print_value("CanStop", player.can_stop());
    print_value("HasPlaybackRate", player.has_playback_rate());
    print_value("HasPosition", player.has_position());
    print_value("HasVolume", player.has_volume());

    print_value("Rate", player.get_playback_rate());
    print_value("MaximumRate", player.get_maximum_playback_rate());
    print_value("MinimumRate", player.get_minimum_playback_rate());

    println!();
    println!("\t─── MediaPlayer2.TrackList ───");
    if player.supports_track_lists() {
        print_value("CanEditTracks", player.can_edit_tracks());
    } else {
        println!("\tPlayer does not support TrackList interface!\n\tNote how they fail.\n");
        print_value("CanEditTracks", player.can_edit_tracks());

        println!(
            "\n\tYou can used the \"Checked\" variants to hide\n\terror handling for these cases:"
        );
        print_value("CheckedCanEditTracks", player.checked_can_edit_tracks());
    }

    Ok(())
}

fn print_value<T: CustomDisplay>(name: &str, value: T) {
    println!(
        "\t{1:>0$}:\t{2}",
        VALUE_INDENTATION,
        name,
        value.string_for_display()
    );
}

impl CustomDisplay for bool {
    fn string_for_display(&self) -> Cow<'_, str> {
        match self {
            true => "✔ Yes".into(),
            false => "✖ No".into(),
        }
    }
}

impl CustomDisplay for f64 {
    fn string_for_display(&self) -> Cow<'_, str> {
        format!("{:.3}", self).into()
    }
}

impl CustomDisplay for String {
    fn string_for_display(&self) -> Cow<'_, str> {
        self.into()
    }
}

impl CustomDisplay for DBusError {
    fn string_for_display(&self) -> Cow<'_, str> {
        format!("Error: {}", self).into()
    }
}

impl<T> CustomDisplay for Vec<T>
where
    T: CustomDisplay,
{
    fn string_for_display(&self) -> Cow<'_, str> {
        let mut buf = String::new();
        for val in self {
            if buf.is_empty() {
                buf.push_str(&val.string_for_display());
            } else {
                buf.push_str(&format!(
                    "\n\t{1:>0$} \t{2}",
                    VALUE_INDENTATION,
                    "",
                    val.string_for_display()
                ));
            }
        }
        buf.into()
    }
}

impl<T, E> CustomDisplay for Result<T, E>
where
    T: CustomDisplay,
    E: CustomDisplay,
{
    fn string_for_display(&self) -> Cow<'_, str> {
        match self {
            Ok(val) => val.string_for_display(),
            Err(err) => err.string_for_display(),
        }
    }
}

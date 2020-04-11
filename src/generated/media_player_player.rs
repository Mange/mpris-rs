#![allow(unknown_lints)]
#![allow(clippy::all)]
#![allow(missing_debug_implementations,
        missing_copy_implementations,
        trivial_casts,
        trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces,
        unused_qualifications,
        unused_imports)]
// This code was autogenerated with dbus-codegen-rust, see https://github.com/diwic/dbus-rs

use dbus;
use dbus::arg;
use dbus::ffidisp;

pub trait OrgMprisMediaPlayer2Player {
    fn next(&self) -> Result<(), dbus::Error>;
    fn previous(&self) -> Result<(), dbus::Error>;
    fn pause(&self) -> Result<(), dbus::Error>;
    fn play_pause(&self) -> Result<(), dbus::Error>;
    fn stop(&self) -> Result<(), dbus::Error>;
    fn play(&self) -> Result<(), dbus::Error>;
    fn seek(&self, offset: i64) -> Result<(), dbus::Error>;
    fn set_position(&self, track_id: dbus::Path<'_>, position: i64) -> Result<(), dbus::Error>;
    fn open_uri(&self, uri: &str) -> Result<(), dbus::Error>;
    fn get_playback_status(&self) -> Result<String, dbus::Error>;
    fn get_loop_status(&self) -> Result<String, dbus::Error>;
    fn set_loop_status(&self, value: String) -> Result<(), dbus::Error>;
    fn get_rate(&self) -> Result<f64, dbus::Error>;
    fn set_rate(&self, value: f64) -> Result<(), dbus::Error>;
    fn get_shuffle(&self) -> Result<bool, dbus::Error>;
    fn set_shuffle(&self, value: bool) -> Result<(), dbus::Error>;
    fn get_metadata(
        &self,
    ) -> Result<
        ::std::collections::HashMap<String, arg::Variant<Box<dyn arg::RefArg + 'static>>>,
        dbus::Error,
    >;
    fn get_volume(&self) -> Result<f64, dbus::Error>;
    fn set_volume(&self, value: f64) -> Result<(), dbus::Error>;
    fn get_position(&self) -> Result<i64, dbus::Error>;
    fn get_minimum_rate(&self) -> Result<f64, dbus::Error>;
    fn get_maximum_rate(&self) -> Result<f64, dbus::Error>;
    fn get_can_go_next(&self) -> Result<bool, dbus::Error>;
    fn get_can_go_previous(&self) -> Result<bool, dbus::Error>;
    fn get_can_play(&self) -> Result<bool, dbus::Error>;
    fn get_can_pause(&self) -> Result<bool, dbus::Error>;
    fn get_can_seek(&self) -> Result<bool, dbus::Error>;
    fn get_can_control(&self) -> Result<bool, dbus::Error>;
}

impl<'a, C: ::std::ops::Deref<Target = ffidisp::Connection>> OrgMprisMediaPlayer2Player
    for ffidisp::ConnPath<'a, C>
{
    fn next(&self) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "Next", ())
    }

    fn previous(&self) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "Previous", ())
    }

    fn pause(&self) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "Pause", ())
    }

    fn play_pause(&self) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "PlayPause", ())
    }

    fn stop(&self) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "Stop", ())
    }

    fn play(&self) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "Play", ())
    }

    fn seek(&self, offset: i64) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "Seek", (offset,))
    }

    fn set_position(&self, track_id: dbus::Path<'_>, position: i64) -> Result<(), dbus::Error> {
        self.method_call(
            "org.mpris.MediaPlayer2.Player",
            "SetPosition",
            (track_id, position),
        )
    }

    fn open_uri(&self, uri: &str) -> Result<(), dbus::Error> {
        self.method_call("org.mpris.MediaPlayer2.Player", "OpenUri", (uri,))
    }

    fn get_playback_status(&self) -> Result<String, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "PlaybackStatus",
        )
    }

    fn get_loop_status(&self) -> Result<String, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "LoopStatus",
        )
    }

    fn get_rate(&self) -> Result<f64, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Rate",
        )
    }

    fn get_shuffle(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Shuffle",
        )
    }

    fn get_metadata(
        &self,
    ) -> Result<
        ::std::collections::HashMap<String, arg::Variant<Box<dyn arg::RefArg + 'static>>>,
        dbus::Error,
    > {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Metadata",
        )
    }

    fn get_volume(&self) -> Result<f64, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Volume",
        )
    }

    fn get_position(&self) -> Result<i64, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Position",
        )
    }

    fn get_minimum_rate(&self) -> Result<f64, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "MinimumRate",
        )
    }

    fn get_maximum_rate(&self) -> Result<f64, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "MaximumRate",
        )
    }

    fn get_can_go_next(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "CanGoNext",
        )
    }

    fn get_can_go_previous(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "CanGoPrevious",
        )
    }

    fn get_can_play(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "CanPlay",
        )
    }

    fn get_can_pause(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "CanPause",
        )
    }

    fn get_can_seek(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "CanSeek",
        )
    }

    fn get_can_control(&self) -> Result<bool, dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::get(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "CanControl",
        )
    }

    fn set_loop_status(&self, value: String) -> Result<(), dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::set(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "LoopStatus",
            value,
        )
    }

    fn set_rate(&self, value: f64) -> Result<(), dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::set(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Rate",
            value,
        )
    }

    fn set_shuffle(&self, value: bool) -> Result<(), dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::set(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Shuffle",
            value,
        )
    }

    fn set_volume(&self, value: f64) -> Result<(), dbus::Error> {
        <Self as ffidisp::stdintf::org_freedesktop_dbus::Properties>::set(
            &self,
            "org.mpris.MediaPlayer2.Player",
            "Volume",
            value,
        )
    }
}

#[derive(Debug)]
pub struct OrgMprisMediaPlayer2PlayerSeeked {
    pub position: i64,
}

impl arg::AppendAll for OrgMprisMediaPlayer2PlayerSeeked {
    fn append(&self, i: &mut arg::IterAppend<'_>) {
        arg::RefArg::append(&self.position, i);
    }
}

impl arg::ReadAll for OrgMprisMediaPlayer2PlayerSeeked {
    fn read(i: &mut arg::Iter<'_>) -> Result<Self, arg::TypeMismatchError> {
        Ok(OrgMprisMediaPlayer2PlayerSeeked {
            position: i.read()?,
        })
    }
}

impl dbus::message::SignalArgs for OrgMprisMediaPlayer2PlayerSeeked {
    const NAME: &'static str = "Seeked";
    const INTERFACE: &'static str = "org.mpris.MediaPlayer2.Player";
}

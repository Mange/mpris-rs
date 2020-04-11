// The following modules have been automatically generated from the MPRIS standard.
// You may regenerate them by running ./script/generate-mpris-interface.sh
mod media_player;
mod media_player_player;
mod media_player_playlists;
mod media_player_tracklist;

// Re-export items used by the codebase here
pub use self::media_player::OrgMprisMediaPlayer2;
pub use self::media_player_player::{OrgMprisMediaPlayer2Player, OrgMprisMediaPlayer2PlayerSeeked};
pub use self::media_player_tracklist::OrgMprisMediaPlayer2TrackList;

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet.

## [v2.0.0-rc3] - 2022-09-12

**Important:** Now using Rust 2018 edition.

### Fixed

* Track change detection for some non-conforming players (e.g. Spotify). -
  [Stephan Henrichs (Kilobyte22)][Kilobyte22]
* Error on progress tracker for players that do not support shuffling. -
  [Stephan Henrichs (Kilobyte22)][Kilobyte22]
* Events not added for streams - [Kanjirito][Kanjirito]
* Incorrect error messages when using the `Display` trait [Kanjirito][Kanjirito]

### Added

* `Player::can_shuffle` and `Player::checked_get_shuffle`. - [Stephan Henrichs
  (Kilobyte22)][Kilobyte22]
* Iterator methods to `Metadata`:
  * `impl IntoIterator`
  * `iter()`
  * `keys()`
* `Player::bus_name_player_name_part` - [Koen Bolhuis
  (InputUsername)](https://github.com/InputUsername)
* `Metadata::as_hashmap(&self)` which returns a simple borrowed hashmap.
* More `Player::has_*`, `Player::can_*`, and `Player::checked_*` methods -
  [Harrison Thorne (harrisonthorne)][harrisonthorne]
  * `Player::has_volume`, `Player::checked_get_volume`,
    `Player::checked_set_volume`
  * `Player::has_position`, `Player::checked_get_position`,
    `Player::checked_set_position`
  * `Player::has_playback_rate`, `Player::checked_get_playback_rate`,
    `Player::checked_set_playback_rate`
  * `Player::can_loop`, `Player::checked_get_loop_status`
* `PlayerIter` that iterates over all of the players [Kanjirito][Kanjirito]
* `PlayerFinder.player_timeout_ms` field that changes the DBUS timeout value for
  all new `Player`s [Kanjirito][Kanjirito]

### Changed

* Now using Rust 2018 edition.
* `Player::checked_set_shuffle` also checks `::can_shuffle`. - [Stephan
  Henrichs (Kilobyte22)][Kilobyte22]
* `Player::checked_set_loop_status` also checks `::can_loop` - [Harrison Thorne
  (harrisonthorne)][harrisonthorne]
* `Progress` default values uses `checked_get_*` functions - [Harrison
  Thorne (harrisonthorne)][harrisonthorne]
* Documentation was made easier to navigate - [Kanjirito][Kanjirito]
* Use `thiserror` & `anyhow` instead of unmaintained `failure` - [fengalin][fengalin]
* Removed `Player` lifetime [Kanjirito][Kanjirito]
* All `PlayerFinder` find methods switched to using `PlayerIter` [Kanjirito][Kanjirito]

## [v2.0.0-rc2] - 2020-02-15

This is a RC for 2.0.0. If no major problems are discovered, this version will
be re-labeled as 2.0.0. If issues are found, they will be fixed in subsequent
versions.

### Changed

- `PlayerEvents` is now properly exposed from the crate root.
- Now using `dbus` 0.8.1.
- This might require a bump of Rust version.

## [v2.0.0-rc1] - 2019-02-06

This is a RC for 2.0.0. If no major problems are discovered, this version will
be re-labeled as 2.0.0. If issues are found, they will be fixed in subsequent
versions.

### Changed

- This library now only supports "latest stable" version of Rust. Hopefully
  this can be changed the day it is possible to mark minimum version in the
  crate manifest.
- Some methods have a different error type to add more context to the errors
  that can happen. See `TrackListError` and `ProgressError`.
- `ProgressTracker::tick` now returns a `ProgressTick` instead of a `bool`.
  - `ProgressTick` contains information about tracklist (if player supports
    it), and what parts have changed.

### Fixed

- Emitted `Event::TrackChanged` events now contains full metadata.
- Compilation warnings caused by newer Rust versions (up to 1.28) have been
  fixed.
- `Player::set_volume` is fixed (always set to 0 previously)
- Detection of volume and playback rate changes using `PlayerEvents` iterator now works.
- Loading of length of a track now works in more clients. #40

### Added

- A new version of `Metadata` that should be much easier to use with extra
  metadata values, or to populate for tests.
- A full implementation of all properties and methods on the
  `org.mpris.MediaPlayer2` interface.
- Support for the `Seeked` signal in the blocking `PlayerEvents` iterator.
- Support for TrackList signals in `PlayerEvents` iterator.
- A new `TrackList` struct, which keeps track of `Metadata` for tracks.
  - `Progress` provides an up-to-date `TrackList` if the player supports it.
  - You can manually maintain this for your `PlayerEvents` iterator if you wish.
- Support for loading `Metadata` for a specific `TrackID`.
- `TrackListError` is an error type for problems with tracklists.
- `ProgressError` is an error type for problems with progress tracking.
- `Player::can_edit_tracks`.
- `Player::checked_can_edit_tracks`.
- `Player::supports_track_lists`.
- A new example called "Capabilities" that shows capabilities of running
  players.

### Removed

- All deprecated items in [v1.1.1] have been removed.

## [v1.1.1] - 2019-01-04

### Fixed

- Loading of length of a track now works in more clients. #40

## [v1.1.0] - 2018-08-18

### Added

- `Player::events(&self)` returns a blocking iterator of player events.
  - Use this to block single-threaded apps until something happens and then
    react on this event.
  - This is not suitable if you want to render a progress bar as it will only
    return when something changes; if you want to render the information at a
    regular update interval then keep using `Player::track_progress(&self)`
    instead.
- `MetadataValue` type, for dynamically types metadata values. This will
  replace the raw DBus values in `Metadata` in version 2.0.
- `Player::get_metadata_hash` which returns a `Result<HashMap<String,
  MetadataValue>, DBusError>`.
- `Metadata::rest_hash` which converts values in the `rest` hash into
  `MetadataValue`s, where possible. This is closer to how `Metadata` will work
  in 2.0.
- `Progress::playback_rate` returns the playback rate at the time of
  measurement.
- `Player::is_running` checks if a player is still running. Use this to detect
  players shutting down.

### Changed

- `Metadata` can now be constructed with empty metadata; `track_id` will then be the empty string.
  * Some players (like VLC) without any tracks on its play queue emits empty
    metadata, which would cause this library to return an error instead of an
    empty metadata.
- `Metadata` now implements `Default`.

### Deprecated

- `Metadata::rest` is deprecated; version 2.0 will have a method that returns
  `MetadataValue`s instead.
- `Player::get_metadata_hash` is added as deprecated. It will likely be merged
  into `Metadata` in version 2.0, but presents a way to get all supported
  metadata values where `Metadata::rest` might not.

## [v1.0.0] - 2018-01-19

### Added

- `TrackID` struct added.
- `Player` can now query and change `Shuffle` status.
- `Player` can now query and change `LoopStatus`.
- `Player` can now change playback rate.
- `Player` can now query for valid playback rates and if it supports setting
  rates at all.
- `Player` can now control volume.
- `Player` can now query for current position as a `std::time::Duration` and
  not just a microsecond count.
- `Player` can set position, if a valid `TrackID` is given.
  - Note: This library has no way of querying for valid `TrackID`s right now.

### Changed

- `failure` replaces `error_chain` for error handling.
  - All errors now implements the `failure::Fail` trait, and methods return
    more fine-grained `Result`s.
- All fields on `Progress` and `Metadata` are now methods instead.
- Playback rate is now `f64` instead of `f32`.

### Removed

- The `supports_progress` method is removed from `Progress`.
  - This is better left to clients to do themselves as this library cannot
    guarantee anything anyway.

## 0.1.0 - 2017-12-29

[Unreleased]: https://github.com/Mange/mpris-rs/compare/v2.0.0-rc3...HEAD
[v2.0.0-rc3]: https://github.com/Mange/mpris-rs/compare/v2.0.0-rc2...v2.0.0-rc3
[v2.0.0-rc2]: https://github.com/Mange/mpris-rs/compare/v2.0.0-rc1...v2.0.0-rc2
[v2.0.0-rc1]: https://github.com/Mange/mpris-rs/compare/v1.1.0...v2.0.0-rc1
[v1.1.1]: https://github.com/Mange/mpris-rs/compare/v1.1.0...v1.1.1
[v1.1.0]: https://github.com/Mange/mpris-rs/compare/v1.0.0...v1.1.0
[v1.0.0]: https://github.com/Mange/mpris-rs/compare/v0.1.0...v1.0.0

[Kilobyte22]: https://github.com/Kilobyte22
[harrisonthorne]: https://github.com/harrisonthorne
[Kanjirito]: https://github.com/Kanjirito
[fengalin]: https://github.com/fengalin

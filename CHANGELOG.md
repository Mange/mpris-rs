# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/Mange/mpris-rs/compare/v1.0.0...HEAD
[v1.0.0]: https://github.com/Mange/mpris-rs/compare/v0.1.0...v1.0.0

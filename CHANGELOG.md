# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Player` can now query and change `Shuffle` status.
- `Player` can now query and change `LoopStatus`.

### Changed

- `failure` replaces `error_chain` for error handling.
  - All errors now implements the `failure::Fail` trait, and methods return more fine-grained `Result`s.

## 0.1.0 - 2017-12-29

[Unreleased]: https://github.com/Mange/mpris-rs/compare/v0.1.0...HEAD

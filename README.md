# mpris

> A Rust library for dealing with [MPRIS2][mpris2]-compatible players over
> D-Bus.

**NOTE:** This is still under development and not ready for use yet.

## What is MPRIS2?

> The Media Player Remote Interfacing Specification is a standard D-Bus
> interface which aims to provide a common programmatic API for controlling
> media players.
>
> It provides a mechanism for discovery, querying and basic playback control of
> compliant media players, as well as a tracklist interface which is used to
> add context to the active media item.

From [*About*, in the MPRIS2 specification][mpris-about].

Basically, you can use it to control media players on your computer. This is
most commonly used to build media player applets, UIs or to pause other players
before your own software performs some action.

You can also use it in order to query metadata about what is currently playing,
or *if* something is playing.

## How to use

```rust
extern crate mpris;

use mpris::PlayerFinder;

// Pauses currently playing media and prints metadata information about that
// media.
// If no player is running, exits with an error.
fn main() {
  let player = PlayerFinder::new()
    .expect("Could not connect to D-Bus")
    .find_active()
    .expect("Could not find any player");

  player.pause().expect("Could not pause");

  let metadata = player.get_metadata().expect("Could not get metadata for player");
  println!("{:#?}", metadata);
}
```

## License

Copyright 2017 Magnus Bergmark

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

[mpris2]: https://specifications.freedesktop.org/mpris-spec/latest/
[mpris-about]: https://specifications.freedesktop.org/mpris-spec/latest/#About

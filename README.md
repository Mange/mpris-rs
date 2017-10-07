# mpris

> A Rust library for dealing with [MPRIS2][mpris2]-compatible players over DBus.

**NOTE:** This is still under development and not ready for use yet.

## How to use

```rust
extern crate mpris;

use mpris::PlayerFinder;

fn main() {
  let player = PlayerFinder::new()
    .expect("Could not connect to DBus")
    .find_active()
    .expect("Could not find any player");

  player.pause().expect("Could not pause");

  let metadata = player.get_metadata().expect("Could not get metadata for player");
  println!("{:#?}", metadata);
}
```

[mpris2]: https://specifications.freedesktop.org/mpris-spec/latest/


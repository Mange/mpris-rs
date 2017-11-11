# mpris

> A Rust library for dealing with [MPRIS2][mpris2]-compatible players over D-Bus.

**NOTE:** This is still under development and not ready for use yet.

## How to use

```rust
extern crate mpris;

use mpris::PlayerFinder;

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


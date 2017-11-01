#!/usr/bin/env bash
# Regenerates the MPRIS interface code using `dbus-codegen-rust`.
set -e

root="$(readlink -f "$(dirname "$0")/..")"
if [[ ! -d "$root" ]]; then
  echo "Could not find root $root"
  exit 1
fi

if ! hash dbus-codegen-rust 2> /dev/null; then
  echo "You must have dbus-codegen-rust installed to run this script."
  exit 1
fi

player=$1
if [[ -z $player ]]; then
  echo "I need a running player to introspect. Enter a player name on the bus."
  echo "(Example: 'spotify' for org.mpris.MediaPlayer2.spotify)"
  echo -n "> "
  read -r player
fi

if [[ -z $player ]]; then
  echo "No player selected. Aborting."
  exit 1
fi

dest="$root/src/generated"
mkdir -p "$dest"

echo "Generating code... "
cat <<EOF > "$dest/mod.rs"
#![allow(unknown_lints)]
#![allow(clippy)]
EOF
dbus-codegen-rust -d "org.mpris.MediaPlayer2.${player}" -p "/org/mpris/MediaPlayer2" -m None >> "$dest/mod.rs"

echo "Formatting code... "
rustfmt --write-mode replace "$dest/mod.rs" 2> /dev/null || true

echo "Done."

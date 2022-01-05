#!/usr/bin/env bash
# Regenerates the MPRIS interface code using `dbus-codegen-rust`.
set -e

root="$(readlink -f "$(dirname "$0")/..")"
if [[ ! -d "$root" ]]; then
  echo "Could not find root $root"
  exit 1
fi

if ! hash dbus-codegen-rust 2> /dev/null; then
  echo "Could not find dbus-codegen-rust binary. Do you want to install it using Cargo?"
  echo -n "[Yn] > "
  read -r c
  if [[ $c == "y" || $c == "Y" ]]; then
    cargo install dbus-codegen
  else
    exit 1
  fi
fi

dest="$root/src/generated"

for spec in "$root"/mpris-spec/spec/org.mpris.*.xml; do
  basename=$(
    basename "$spec" | \
      sed -r 's/org\.mpris\.MediaPlayer2(\.(\w+))?\.xml/media_player_\2.rs/; s/_\.rs$/\.rs/' | \
      tr '[:upper:]' '[:lower:]'
  )
  dest_file="${dest}/${basename}"
  echo "Generating code from $(basename "${spec}") to ${basename}…"

  cat <<EOF > "$dest_file"
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![allow(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    unused_imports
)]
EOF
  dbus-codegen-rust -m None -c ffidisp < "$spec" >> "$dest_file"

  rustfmt ${dest_file}
done

echo "Done."

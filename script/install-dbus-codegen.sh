#!/usr/bin/env bash
# Installs dbus-codegen-rust in the `target` directory.
set -e

root="$(readlink -f "$(dirname "$0")/..")"
if [[ ! -d "$root" ]]; then
  echo "Could not find root $root"
  exit 1
fi

target="$root/target/rust-dbus"
if [[ ! -d "$target" ]]; then
  git clone https://github.com/diwic/dbus-rs.git "$target"
else
  (cd "$target" && git pull --rebase)
fi

(cd "$target/dbus-codegen" && cargo install)

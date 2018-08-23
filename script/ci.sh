#!/bin/sh
# Run this on CI, inside a X11 instance.

set -ex

export RUST_BACKTRACE=1

if [ -z "$DBUS_SESSION_BUS_ADDRESS" ]; then
  echo "Starting dbus"
  # dbus-launch will quote values itself, so quoting the string will actually
  # not set values correctly.
  # shellcheck disable=SC2046
  export $(dbus-launch)
fi

cargo build --verbose &&
  cargo test --verbose &&
  cargo doc --no-deps

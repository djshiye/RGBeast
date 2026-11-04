#!/bin/sh
# Invoked by Meson: builds the workspace with Cargo and copies the daemon
# (rgbeastd) into OUTDIR. The app is Python and needs no build.
set -eu
MESON_BUILD_ROOT="$1"
MESON_SOURCE_ROOT="$2"
OUTDIR="$3"
BUILDTYPE="$4"
OFFLINE="$5"

case "$OUTDIR" in
    /*) ;;
    *) OUTDIR="$MESON_BUILD_ROOT/$OUTDIR" ;;
esac

export CARGO_TARGET_DIR="$MESON_BUILD_ROOT/target"
# CARGO_HOME is left alone: developers keep their ~/.cargo registry, and the
# RPM build exports its own vendored one.

cd "$MESON_SOURCE_ROOT"

ARGS=""
if [ "$OFFLINE" = "offline" ]; then ARGS="$ARGS --offline"; fi

if [ "$BUILDTYPE" = "release" ] || [ "$BUILDTYPE" = "plain" ]; then
    cargo build --release --workspace $ARGS
    PROFILE=release
else
    cargo build --workspace $ARGS
    PROFILE=debug
fi
cp "$CARGO_TARGET_DIR/$PROFILE/rgbeastd" "$OUTDIR/rgbeastd"

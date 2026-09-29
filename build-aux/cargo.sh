#!/bin/sh
# Invoked by Meson: builds the workspace with Cargo and copies both binaries
# (rgbeast, rgbeastd) into OUTDIR.
set -eu
MESON_BUILD_ROOT="$1"
MESON_SOURCE_ROOT="$2"
OUTDIR="$3"
BUILDTYPE="$4"
OFFLINE="$5"
LOCALEDIR="${6:-}"
if [ -n "$LOCALEDIR" ]; then export RGBEAST_LOCALEDIR="$LOCALEDIR"; fi

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
cp "$CARGO_TARGET_DIR/$PROFILE/rgbeast" "$OUTDIR/rgbeast"
cp "$CARGO_TARGET_DIR/$PROFILE/rgbeastd" "$OUTDIR/rgbeastd"

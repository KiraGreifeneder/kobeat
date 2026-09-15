#!/bin/sh
# Starts kobeatd and kobeatui on the Kobo.
#
# Kobo's stripped-down init does not bring up the loopback interface by
# default. kobeatui is served on 127.0.0.1 (and kobeatd/kobeatui talk to
# each other over a unix socket, which is unaffected, but the web UI itself
# still needs `lo` up for the browser to reach it). If `lo` is down, every
# connection attempt (browser, curl, ncat, etc.) simply times out with no
# error on either side. Symptom: kobeatui logs "listening on
# http://0.0.0.0:3000" and then nothing else ever happens, even though the
# process itself is completely healthy.
#
# `ip addr show lo` on a broken device shows no `UP,LOWER_UP` flags and no
# `inet 127.0.0.1/8` line. This script fixes that unconditionally before
# starting the binaries, so the fix survives every reboot without relying
# on manually running `ip link set lo up` over SSH each time.

set -e

KOBEAT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="${KOBEAT_LOG_DIR:-$KOBEAT_DIR}"

# Bring up loopback if it isn't already, and make sure it has an address.
if ! ip addr show lo | grep -q "inet 127.0.0.1/8"; then
    ip addr add 127.0.0.1/8 dev lo 2>/dev/null || true
fi
if ! ip addr show lo | grep -q "LOWER_UP"; then
    ip link set lo up
fi

cd "$KOBEAT_DIR"

RUST_LOG="${RUST_LOG:-info}" ./kobeatd >>"$LOG_DIR/kobeatd.log" 2>&1 &
RUST_LOG="${RUST_LOG:-info}" ./kobeatui >>"$LOG_DIR/kobeatui.log" 2>&1 &

wait

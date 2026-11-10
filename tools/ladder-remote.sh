#!/usr/bin/env bash
# The compile half of the Industrial Foregoing slice-1 ladder, path-agnostic so
# it runs on either box (`ports/tools/remote-slot.sh run … -- bash
# ports/industrialforegoing/tools/ladder-remote.sh`): fmt, clippy, the pure
# tests, then stage → check → test → build against the pinned Horizon.
#
# The red run was captured separately (`evidence/industrialforegoing/test-red.log`
# from the stubbed runtime); this script only runs the implementation.
set -uo pipefail
export LC_ALL=C

PORT=$(cd "$(dirname "$0")/.." && pwd)
EV=$PORT/../_shared/evidence/industrialforegoing
mkdir -p "$EV"
cd "$PORT" || exit 1

fail() {
    echo "LADDER-ABORT: $*" >&2
    exit 1
}

step() { # step NAME COMMAND...
    local name=$1
    shift
    echo "== $name"
    "$@" >"$EV/$name.log" 2>&1
    local rc=$?
    echo "   exit $rc"
    return $rc
}

if [ ! -f Cargo.lock ]; then
    step 00-lockfile cargo generate-lockfile || fail "generate-lockfile"
fi

step fmt-write cargo fmt --all || fail "cargo fmt"
step fmt cargo fmt --all -- --check || fail "cargo fmt --check"
step clippy cargo clippy --all-targets --locked --offline -- -D warnings || fail "clippy"
step test-green cargo test --locked --offline || fail "cargo test"

# The staged check compiles the content in with `native_data!`, which reads
# `native-data/<id>/` — the planner stages it from mixins/<id>/data/, so the
# content half must be built and copied there first.
step build-hmod env CARGO_INCREMENTAL=0 tools/build_hmod.sh || fail "hmod"
step stage python3 tools/horizon.py stage || fail "stage"
step horizon-check python3 tools/horizon.py check || fail "horizon check"
step horizon-test python3 tools/horizon.py test || fail "horizon test"
step horizon-build python3 tools/horizon.py build || fail "horizon build"

echo "LADDER-REMOTE-OK"

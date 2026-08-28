#!/usr/bin/env bash
# Build the genuine CMake artifact (the `driver` executable) for one
# (OP, REPEAT) configuration, out-of-source so nothing under c_src/ is touched.
#
# usage: build_c_exe.sh <OP> <REPEAT>
# prints the path of the produced executable
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OP="${1:-add}"
REPEAT="${2:-5}"
B="$ROOT/cbuild/exe/${OP}_${REPEAT}"
mkdir -p "$B"
cmake -S "$ROOT/c_src" -B "$B" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      "-DOP=${OP}" "-DREPEAT=${REPEAT}" >"$B/configure.log" 2>&1
cmake --build "$B" >"$B/build.log" 2>&1
echo "$B/driver"

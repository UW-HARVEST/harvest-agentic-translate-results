#!/bin/bash
# Preprocess a libsodium C source file exactly as the reference CMake build does,
# then strip everything that came from system headers, so you see only the
# libsodium code that is actually compiled (all #ifdef HAVE_* already resolved).
#
# Usage: tools/cpp.sh c_src/libsodium/sodium/utils.c
set -e
W=$HARVEST_WORKDIR
INC="-I $W/c_src/libsodium/include -I $W/c_src/libsodium/include/sodium"
for d in "$W"/c_src/libsodium/*/; do INC="$INC -I ${d%/}"; done
INC="$INC -I $(dirname "$1")"
gcc -std=c99 -E $INC "$1" 2>/dev/null | awk -v root="$W/c_src" '
  /^# [0-9]+ "/ {
    f = $3; gsub(/"/, "", f);
    keep = (index(f, root) == 1);
    if (keep && f != prev) { print "\n/* ===== " f " ===== */"; prev = f }
    next
  }
  keep { print }
'

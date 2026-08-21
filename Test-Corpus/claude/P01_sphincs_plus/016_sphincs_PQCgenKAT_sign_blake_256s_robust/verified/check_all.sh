#!/bin/bash
# `cargo check` every valid (HASH_BACKEND, THASH, SECPAR) feature combination.
# The three axes come from Cargo.toml's [features], which mirror the CMake cache
# variables in c_src/CMakeLists.txt:
#   HASH_BACKEND -> haraka | sha2 | shake | blake
#   THASH        -> robust | simple
#   SECPAR       -> 128s | 128f | 192s | 192f | 256s | 256f
# => 4 * 2 * 6 = 48 combinations.  Also checks the bare default feature set.
set -u
R="$(cd "$(dirname "$0")" && pwd)"
export CARGO_NET_OFFLINE=true
LOG="${TMPDIR:-/var/tmp}/check_all.log"
: > "$LOG"
cd "$R" || exit 1
n=0; bad=0
run() { # $1 = description, rest = cargo args
  local d="$1"; shift
  n=$((n+1))
  if cargo check --all-targets "$@" >>"$LOG" 2>&1; then
    printf 'OK   %s\n' "$d"
  else
    printf 'FAIL %s\n' "$d"; bad=$((bad+1))
    cargo check --all-targets "$@" 2>&1 | grep -E '^(error|warning)' | head -10
  fi
}
run "default features" 
for bk in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      run "$bk $th $sp" --no-default-features --features "$bk $th $sp"
    done
  done
done
echo "check_all: $n configurations, $bad failed"
[ "$bad" -eq 0 ]

#!/bin/bash
# Builds the Rust cdylib for every valid feature combination and stashes each
# resulting .so under rbuild/<backend>-<thash>-<secpar>/libsphincs_core_det.so
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation" || exit 1
mkdir -p "$ROOT/rbuild"

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
THASHES="${THASHES:-robust simple}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"

fail=0
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      combo="$b-$t-$s"
      d="$ROOT/rbuild/$combo"
      mkdir -p "$d"
      if cargo build --release --quiet --no-default-features --features "$b,$t,$s" 2>"$d/build.log"; then
        cp target/release/libsphincs_core_det.so "$d/" && echo "ok   $combo"
      else
        echo "FAIL $combo"; tail -5 "$d/build.log"; fail=1
      fi
    done
  done
done
exit $fail

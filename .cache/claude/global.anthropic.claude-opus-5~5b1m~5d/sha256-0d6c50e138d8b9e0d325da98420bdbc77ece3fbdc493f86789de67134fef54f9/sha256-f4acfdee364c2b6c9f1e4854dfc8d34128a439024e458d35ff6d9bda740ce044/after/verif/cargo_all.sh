#!/usr/bin/env bash
# Run a cargo subcommand for every valid feature combination
# (one hash backend x one thash variant x one security parameter = 48 combos).
#
#   ./verif/cargo_all.sh check
#   ./verif/cargo_all.sh build --release
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
SUB="$1"; shift || true

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
THASHES="${THASHES:-robust simple}"

fail=0
for b in $BACKENDS; do
  for s in $SECPARS; do
    for t in $THASHES; do
      combo="$b,$t,$s"
      out=$(cd "$CRATE" && CARGO_TARGET_DIR="target/$b-$s-$t" \
            timeout 600 cargo "$SUB" --no-default-features --features "$combo" "$@" 2>&1)
      if [ $? -eq 0 ]; then
        w=$(printf '%s' "$out" | grep -c 'warning')
        echo "OK   $combo (warnings: $w)"
      else
        echo "FAIL $combo"
        printf '%s\n' "$out" | grep -E '^(error|warning)' | head -20
        fail=1
      fi
    done
  done
done
exit $fail

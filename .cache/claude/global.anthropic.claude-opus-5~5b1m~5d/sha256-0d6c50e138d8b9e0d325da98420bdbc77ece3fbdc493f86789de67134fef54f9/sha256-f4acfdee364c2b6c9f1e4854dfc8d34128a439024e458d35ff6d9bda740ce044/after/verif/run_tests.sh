#!/usr/bin/env bash
# Run the differential test-suite for every (or a selected) feature combination.
#
#   ./verif/run_tests.sh                      # all 48 combos
#   BACKENDS=blake SECPARS=128f ./verif/run_tests.sh
#   ./verif/run_tests.sh --test diff_backend  # extra args go to `cargo test`
#
# `cargo test` does NOT refresh the cdylib, so the shared object is built
# explicitly first — the tests dlopen it.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"

BACKENDS="${BACKENDS:-haraka sha2 shake blake}"
SECPARS="${SECPARS:-128s 128f 192s 192f 256s 256f}"
THASHES="${THASHES:-robust simple}"
TIMEOUT="${TIMEOUT:-600}"

fail=0
for b in $BACKENDS; do
  for s in $SECPARS; do
    for t in $THASHES; do
      combo="$b,$t,$s"
      tag="$b-$s-$t"
      export CARGO_TARGET_DIR="target/$tag"
      cd "$CRATE" || exit 1
      if ! timeout "$TIMEOUT" cargo build --offline --release \
             --no-default-features --features "$combo" > "$ROOT/verif/last_build.log" 2>&1; then
        echo "BUILDFAIL $tag"; tail -20 "$ROOT/verif/last_build.log"; fail=1; continue
      fi
      out=$(timeout "$TIMEOUT" cargo test --offline --release \
              --no-default-features --features "$combo" "$@" 2>&1)
      rc=$?
      summary=$(printf '%s\n' "$out" | grep -E '^test result:' | tr '\n' '|')
      if [ $rc -eq 0 ]; then
        echo "PASS $tag  $summary"
      else
        echo "FAIL $tag  $summary"
        printf '%s\n' "$out" | grep -vE '^(   Compiling|    Finished|     Running|warning)' | tail -40
        fail=1
      fi
    done
  done
done
exit $fail

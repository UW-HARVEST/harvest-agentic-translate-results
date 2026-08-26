#!/usr/bin/env bash
# Negative control for the differential suite.
#
# A suite that passes is only meaningful if it can FAIL. This injects known-wrong
# variants into the Rust translation one at a time, records which tests catch
# each one, and always restores the original source.
#
# Usage: ./mutation_check.sh
set -uo pipefail
cd "$(dirname "$0")"

LIB=src/lib.rs
BAK="$(mktemp)"
cp "$LIB" "$BAK"
restore() { cp "$BAK" "$LIB"; rm -f "$BAK"; }
trap restore EXIT INT TERM

ORIGINAL='let result: c_int = x | !y;'
grep -qF "$ORIGINAL" "$LIB" || { echo "FATAL: cannot find the expression to mutate in $LIB"; exit 2; }

# mutant name | replacement expression
MUTANTS=(
  "drop_complement|let result: c_int = x | y;"
  "and_instead_of_or|let result: c_int = x & !y;"
  "xor_instead_of_or|let result: c_int = x ^ !y;"
  "complement_x_not_y|let result: c_int = !x | y;"
  "off_by_one|let result: c_int = (x | !y).wrapping_add(1);"
  "swap_operands|let result: c_int = y | !x;"
)

printf '%-22s %8s %8s\n' MUTANT CAUGHT SURVIVED
printf '%-22s %8s %8s\n' ---------------------- -------- --------

OVERALL=0
for m in "${MUTANTS[@]}"; do
  name="${m%%|*}"
  expr="${m#*|}"
  cp "$BAK" "$LIB"
  python3 - "$LIB" "$ORIGINAL" "$expr" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
assert old in s, "anchor missing"
open(path, 'w').write(s.replace(old, new))
PY

  log="$(mktemp)"
  # Build the mutated cdylib up front and hand it to the harness via
  # DRIVER_RUST_SO. Otherwise the harness's own nested `cargo build` prints
  # "Finished ..." *during* the run, interleaving with the first `test NAME ...`
  # line of each binary and corrupting the tallies.
  timeout 600 cargo build --offline --lib --target-dir target/ffi-so >/dev/null 2>&1
  timeout 600 cargo test --offline --no-run >/dev/null 2>&1
  # --no-fail-fast is essential: without it cargo stops after the first failing
  # test target, so phases C and D would never run and the tally would only
  # reflect phase B.
  DRIVER_RUST_SO="$PWD/target/ffi-so/debug/libdriver.so" \
    timeout 600 cargo test --offline --no-fail-fast -- --test-threads=1 >"$log" 2>&1
  caught=$(grep -cE '^test [a-z0-9_]+ \.\.\. FAILED$' "$log")
  passed=$(grep -cE '^test [a-z0-9_]+ \.\.\. ok$' "$log")
  printf '%-22s %8s %8s\n' "$name" "$caught" "$passed"
  if (( caught == 0 )); then
    echo "  >>> NOT DETECTED by any test — the suite has a blind spot here!"
    OVERALL=1
  fi
  grep -oE '^test [a-z0-9_]+ \.\.\. ok$' "$log" | sed 's/^test /    survived: /; s/ \.\.\. ok$//'
  rm -f "$log"
done

restore
trap - EXIT INT TERM
echo
echo "original source restored:"
grep -n 'let result' "$LIB"
if (( OVERALL == 0 )); then echo "NEGATIVE CONTROL OK: every mutant was detected"; else echo "NEGATIVE CONTROL FAILED"; fi
exit $OVERALL

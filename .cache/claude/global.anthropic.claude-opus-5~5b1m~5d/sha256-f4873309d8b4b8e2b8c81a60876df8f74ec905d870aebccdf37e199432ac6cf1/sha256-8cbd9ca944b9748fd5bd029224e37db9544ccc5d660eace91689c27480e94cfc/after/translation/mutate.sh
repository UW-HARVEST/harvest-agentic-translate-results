#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Matching symbols and green tests are only meaningful if the tests would go RED
# on a wrong translation. This script injects known-wrong behaviour into
# src/lib.rs one mutation at a time, rebuilds the cdylib (cargo test does NOT
# rebuild a cdylib on its own), and reports whether the suite caught it.
#
# Usage: ./mutate.sh
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

BAK=$(mktemp -d)/lib.rs
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; echo "src/lib.rs restored"' EXIT

rebuild() {
  cargo build --offline >/dev/null 2>&1 && cargo build --offline --release >/dev/null 2>&1
}

# Applies one replacement to a pristine copy of src/lib.rs.
apply() {
  restore
  OLD="$1" NEW="$2" python3 -c '
import os, sys
p = "src/lib.rs"; s = open(p).read()
o, n = os.environ["OLD"], os.environ["NEW"]
if o not in s:
    sys.exit("PATTERN NOT FOUND: " + repr(o))
open(p, "w").write(s.replace(o, n, 1))'
}

# Runs the suite; prints CAUGHT if any test failed, SURVIVED otherwise.
check() {
  local name="$1"
  if ! rebuild; then printf '  %-52s \033[33mBUILD-ERROR\033[0m\n' "$name"; return; fi
  local out; out=$(timeout 600 cargo test --offline 2>&1)
  local failed; failed=$(printf '%s' "$out" | grep -cE '^test .* FAILED$')
  local passed; passed=$(printf '%s' "$out" | grep -oP '\d+(?= passed)' | awk '{s+=$1} END{print s+0}')
  if [ "$failed" -gt 0 ] || printf '%s' "$out" | grep -q 'error:'; then
    printf '  %-52s \033[32mCAUGHT\033[0m (%s tests failed)\n' "$name" "$failed"
  else
    printf '  %-52s \033[31mSURVIVED\033[0m (%s passed)\n' "$name" "$passed"
  fi
}

printf '\n\033[1mBaseline (unmutated) — must be all green\033[0m\n'
restore; check "no mutation"

printf '\n\033[1mMutants — each must be CAUGHT\033[0m\n'

apply 'if truncated >= 2147483648.0 || truncated < -2147483648.0 {
        return c_int::MIN;
    }' 'if truncated >= 2147483648.0 { return c_int::MAX; }
    if truncated < -2147483648.0 { return c_int::MIN; }'
check "M1 saturating cast instead of INT_MIN indefinite"

apply '.abs() > 0.000001f64' '.abs() > 0.0000001f64'
check "M2 goodB2G threshold 1e-6 -> 1e-7"

apply 'printf(FMT_STR_NL.as_ptr() as *const c_char, line);' 'printf(line);'
check "M3 printLine uses line as the format string"

apply 'if value.is_nan() {
        return c_int::MIN;
    }' 'if value.is_nan() { return 0; }'
check "M4 NaN converts to 0 instead of INT_MIN"

apply 'let truncated = value.trunc();' 'let truncated = value.floor();'
check "M5 floor instead of truncate-toward-zero"

apply 'data = 2.0f32;' 'data = 4.0f32;'
check "M6 goodG2B constant 2.0 -> 4.0"

apply 'b"Finished good()\0"' 'b"Finished good()!\0"'
check "M7 driver label typo"

# The C divides `100.0` (a *double*) by the widened float, so the quotient has
# f64 precision. Rounding the quotient to f32 changes the truncated result for
# ~2.7% of all f32 inputs, so this must be caught.
apply 'fn c_double_to_int(value: c_double) -> c_int {' 'fn c_double_to_int(value: c_double) -> c_int {
    let value = value as f32 as f64; // MUTANT: quotient rounded to f32'
check "M8 quotient rounded to f32 precision"

apply 'if !line.is_null() {' 'if line.is_null() {'
check "M9 printLine NULL guard inverted"

apply 'printf(FMT_INT_NL.as_ptr() as *const c_char, intNumber);' 'printf(b"%u\n\0".as_ptr() as *const c_char, intNumber);'
check "M10 printIntLine uses %u instead of %d"

printf '\n\033[1mKnown semantic equivalents — SURVIVED is CORRECT here\033[0m\n'
apply '.abs() > 0.000001f64' '.abs() >= 0.000001f64'
check "E1 gt -> gte at the 1e-6 threshold"
echo "     (no f32 widens to exactly 1e-6 — 1e-6f as f64 is 9.99999997475e-7 —"
echo "      so the two comparisons are provably equivalent for all f32 inputs)"

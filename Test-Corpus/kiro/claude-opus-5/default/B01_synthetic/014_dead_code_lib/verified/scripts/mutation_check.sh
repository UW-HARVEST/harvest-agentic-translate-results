#!/usr/bin/env bash
# Mutation check: deliberately break the Rust translation in ways that mirror
# realistic mis-translations, and confirm the differential suite CATCHES each
# one. A suite that passes on a mutant is not testing anything.
#
# The C in c_src/ is never touched. src/lib.rs is restored on every exit path.
set -uo pipefail
cd "$(dirname "$0")/.."

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

FMT='FMT_STR_NL.as_ptr()'
NULLCHK='if line != ptr::null() {'
PRINTF='c_printf(FMT_STR_NL.as_ptr() as \*const c_char, line);'

# name | sed expression
MUTANTS=(
  "M1 NULL treated as empty string|s|${NULLCHK}|if true { let line = if line.is_null() { c\"\".as_ptr() } else { line };|"
  "M2 empty string also rejected|s|${NULLCHK}|if !line.is_null() \&\& unsafe { *line } != 0 {|"
  "M3 payload used as format string|s|${PRINTF}|c_printf(line);|"
  "M4 bad() also calls helperBad()|s|print_line_lit(b\"bad()\\\\0\");|print_line_lit(b\"bad()\\\\0\"); helperBad();|"
  "M5 driver() prints bad-banner first|s|print_line_lit(b\"Calling good()...\\\\0\");|print_line_lit(b\"Calling bad()...\\\\0\");|"
  "M6 output truncated at 4096 bytes|s|${FMT}|b\"%.4096s\\\\n\\\\0\".as_ptr()|"
  "M7 trailing newline dropped|s|${FMT}|b\"%s\\\\0\".as_ptr()|"
)

pass=0; fail=0
for entry in "${MUTANTS[@]}"; do
  name=${entry%%|*}
  expr=${entry#*|}
  cp "$BAK" "$SRC"
  if ! sed -i "$expr" "$SRC"; then
    echo "!! $name: sed failed"; fail=$((fail+1)); continue
  fi
  if cmp -s "$BAK" "$SRC"; then
    echo "!! $name: mutation did not apply (pattern not found) — check the script"
    fail=$((fail+1)); continue
  fi
  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    echo "~~ $name: mutant does not compile (counts as caught by the compiler)"
    pass=$((pass+1)); continue
  fi
  if timeout 300 cargo test --release --test differential -- --test-threads=1 >/dev/null 2>&1; then
    echo "!! $name: SUITE PASSED ON A MUTANT — the tests are blind to this bug"
    fail=$((fail+1))
  else
    echo "OK $name: caught"
    pass=$((pass+1))
  fi
done

restore; trap - EXIT
timeout 300 cargo build --release >/dev/null 2>&1
echo
echo "mutants caught: $pass    escaped: $fail"
[ "$fail" -eq 0 ]

#!/usr/bin/env bash
# Copyright 2025 MIT Lincoln Laboratory
# SPDX-License-Identifier: MIT
#
# Negative control for the differential suite (mutation testing).
#
# A passing test suite proves nothing unless it can fail. This script injects one
# realistic translation bug at a time into src/, runs the full suite, and asserts
# that the suite REJECTS the mutant. Any mutant that survives marks a real hole in
# the tests.
#
# The original sources are always restored, including on interrupt.

set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

BACKUP=$(mktemp -d "${TMPDIR:-/tmp}/driver_negctl.XXXXXX")
cp src/lib.rs src/main.rs "$BACKUP/"
restore() { cp "$BACKUP/lib.rs" src/lib.rs; cp "$BACKUP/main.rs" src/main.rs; }
trap 'restore; rm -rf "$BACKUP"' EXIT INT TERM

LOG=$BACKUP/run.log
SURVIVORS=0
TOTAL=0

# Each mutant: "description|file|sed-expression"
MUTANTS=(
  "bitwise OR becomes XOR|src/lib.rs|s/let result: c_int = x | !y;/let result: c_int = x ^ !y;/"
  "bitwise NOT dropped|src/lib.rs|s/let result: c_int = x | !y;/let result: c_int = x | y;/"
  "operands swapped|src/lib.rs|s/let result: c_int = x | !y;/let result: c_int = y | !x;/"
  "trailing newline from puts(\"\") dropped|src/lib.rs|s/let _ = writeln!(out);/let _ = write!(out, \"\");/"
  "SIGPIPE left ignored by the Rust runtime|src/main.rs|s/driver::restore_default_sigpipe();//"
  "vertical tab no longer counts as whitespace|src/lib.rs|s/b' ' | b'\\\\t' | b'\\\\n' | b'\\\\x0b' | b'\\\\x0c' | b'\\\\r'/b' ' | b'\\\\t' | b'\\\\n' | b'\\\\x0c' | b'\\\\r'/"
  "leading '+' no longer consumed|src/lib.rs|s/Some(b'+') => {/Some(b'@') => {/"
  "strtol clamp done at int width instead of long|src/lib.rs|s/i64::MAX$/i32::MAX as i64/"
  "long->int truncation replaced by saturation|src/lib.rs|s/\\*out = as_long as i32;/*out = as_long.clamp(i32::MIN as i64, i32::MAX as i64) as i32;/"
  "digit test accepts letters too|src/lib.rs|s/if !c.is_ascii_digit() {/if !c.is_ascii_alphanumeric() {/"
  "stdin slurped eagerly instead of lazily|src/lib.rs|s#let mut input = Scanner::new(stdin.lock());#let mut all = Vec::new(); { use std::io::Read as _; let _ = std::io::stdin().read_to_end(\\&mut all); } let mut input = Scanner::new(std::io::Cursor::new(all));#"
)

printf '\n\033[1mNegative control: %d mutants\033[0m\n\n' "${#MUTANTS[@]}"

for entry in "${MUTANTS[@]}"; do
  IFS='|' read -r desc file expr <<<"$entry"
  TOTAL=$((TOTAL + 1))
  restore
  sed -i "$expr" "$file"

  if ! git diff --quiet -- "$file" 2>/dev/null && ! diff -q "$BACKUP/$(basename "$file")" "$file" >/dev/null; then
    : # mutation applied
  elif diff -q "$BACKUP/$(basename "$file")" "$file" >/dev/null; then
    printf '  \033[33mSKIP\033[0m %-55s (sed matched nothing)\n' "$desc"
    continue
  fi

  if timeout 600 cargo test --offline >"$LOG" 2>&1; then
    # The mutant compiled and every test still passed -> the suite is blind to it.
    printf '  \033[31mSURVIVED\033[0m %-51s <-- TEST GAP\n' "$desc"
    SURVIVORS=$((SURVIVORS + 1))
  else
    if grep -q "^error\[E\|^error: could not compile" "$LOG"; then
      printf '  \033[33mNO-COMPILE\033[0m %-49s (rejected by rustc)\n' "$desc"
    else
      killers=$(grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' "$LOG" | wc -l | tr -d ' ')
      printf '  \033[32mKILLED\033[0m %-53s (%s failing test(s))\n' "$desc" "$killers"
    fi
  fi
done

restore
printf '\n'
if ((SURVIVORS)); then
  printf '\033[31m%d of %d mutants SURVIVED - the suite has gaps.\033[0m\n' "$SURVIVORS" "$TOTAL"
  exit 1
fi
printf '\033[32mAll %d mutants were rejected.\033[0m\n' "$TOTAL"
exit 0

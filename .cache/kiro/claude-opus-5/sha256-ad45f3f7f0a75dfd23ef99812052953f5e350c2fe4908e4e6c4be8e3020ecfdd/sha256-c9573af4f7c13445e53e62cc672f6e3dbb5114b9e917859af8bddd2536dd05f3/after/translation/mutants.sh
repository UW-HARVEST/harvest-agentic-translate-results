#!/usr/bin/env bash
# Mutation sweep: prove the differential suite actually discriminates.
#
# For each mutation below: apply it to the Rust sources, rebuild the .so, run
# the suite, and require at least one test to FAIL.  A mutation that survives
# means the suite has a blind spot there.
#
# Usage: ./mutants.sh            (all mutations)
#        ./mutants.sh <index>    (one mutation)
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp -d)
cp -r src "$BAK/src"
restore() { rm -rf src; cp -r "$BAK/src" src; }
trap 'restore; rm -rf "$BAK"' EXIT

# file : python-replacement (old ->|- new), applied once
MUTANTS=(
  # --- pure numeric / utf ---
  "src/utf.rs|Runeerror|0xFFFC"
  "src/utf.rs|if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {|if !p.is_null() && c >= *p.add(0) && c < *p.add(1) {"
  "src/jsvalue.rs|if n > INT_MAX as f64 { return INT_MAX; }|if n > INT_MAX as f64 { return INT_MIN; }"
  "src/jsdtoa.rs|while i < 1 {|while i < 2 {"
  # --- regexp engine ---
  "src/regexp.rs|unsafe fn canon(c: Rune) -> c_int {|unsafe fn canon(c: Rune) -> c_int { if c == 'z' as Rune { return 'z' as c_int; }"
  "src/regexp.rs|if depth > REG_MAXREC {|if depth > REG_MAXREC - 1 {"
  # --- interpreter / builtins ---
  "src/jsarray.rs|if from < 0 { from = 0; }|if from < 0 { from = 1; }"
  "src/jsmath.rs|if x > 0.0 && x < 0.5 { return 0.0; }|if x > 0.0 && x <= 0.5 { return 0.0; }"
  "src/jsdate.rs|y % 400 == 0|y % 500 == 0"
  "src/jsrepr.rs|if c < ' ' as c_int {|if c < 0x1f as c_int {"
  "src/jsbuiltin.rs|if radix == 0 {|if radix == 0 || radix == 10 {"
  "src/jsstate.rs|if (*J).trytop == JS_TRYLIMIT {|if (*J).trytop == JS_TRYLIMIT - 1 {"
  "src/jsparse.rs|if (*\$J).astdepth > JS_ASTLIMIT {|if (*\$J).astdepth > JS_ASTLIMIT - 1 {"
  "src/jsrun.rs|if size >= (*J).memlimit {|if size > (*J).memlimit {"
  "src/jsintern.rs|if n as c_int > JS_STRLIMIT {|if n as c_int > JS_STRLIMIT + 1 {"
  # --- error messages, one per module ---
  "src/jsnumber.rs|invalid radix|invalid radiX"
  "src/jslex.rs|number with leading zero|number with a leading zero"
  "src/jsstring.rs|not a string|not a String"
  "src/jsobject.rs|not an object|not an Object"
  "src/jsproperty.rs|object is non-extensible|object is non extensible"
  "src/json.rs|cyclic object value|cyclic-object value"
  "src/jsfunction.rs|not a function|not a Function"
  "src/jsboolean.rs|not a boolean|not a Boolean"
  "src/jsregexp.rs|regular expression: |regular expression:: "
  "src/jserror.rs|not an object|not an Object"
  "src/jsgc.rs|garbage collected|garbage-collected"
)

run_suite() {
  local log="$1"
  : > "$log"
  for t in $(ls tests/*.rs | xargs -n1 basename | sed 's/\.rs$//'); do
    timeout 600 cargo test --release --test "$t" -- --test-threads=1 >>"$log" 2>&1
  done
  grep -c 'test result: FAILED' "$log" || true
}

only="${1:-}"
i=0
survived=0
for m in "${MUTANTS[@]}"; do
  file="${m%%|*}"; rest="${m#*|}"; old="${rest%%|*}"; new="${rest#*|}"
  i=$((i+1))
  if [ -n "$only" ] && [ "$only" != "$i" ]; then continue; fi
  restore
  python3 - "$file" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if old not in s:
    print(f"SKIP: pattern not found in {path}: {old!r}")
    sys.exit(3)
open(path, 'w').write(s.replace(old, new, 1))
PY
  rc=$?
  if [ $rc -eq 3 ]; then echo "mutant $i: PATTERN MISSING ($file)"; continue; fi
  if ! timeout 600 cargo build --release >/dev/null 2>&1; then
    echo "mutant $i: build failed (counts as detected)"
    continue
  fi
  n=$(run_suite "/tmp/mutant-$i.log")
  if [ "$n" -gt 0 ]; then
    echo "mutant $i KILLED by $n test binaries : $file : $old -> $new"
  else
    echo "mutant $i SURVIVED (BLIND SPOT)     : $file : $old -> $new"
    survived=$((survived+1))
  fi
done

restore
timeout 600 cargo build --release >/dev/null 2>&1
echo "survivors: $survived"
[ "$survived" -eq 0 ]

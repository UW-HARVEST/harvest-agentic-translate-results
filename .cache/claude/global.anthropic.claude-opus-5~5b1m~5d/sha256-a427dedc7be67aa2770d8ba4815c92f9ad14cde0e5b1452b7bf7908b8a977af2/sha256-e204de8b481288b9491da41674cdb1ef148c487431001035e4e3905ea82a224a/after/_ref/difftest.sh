#!/bin/bash
# Differential test harness: compares the C reference with the Rust translation.
# Usage: difftest.sh <name> <input-file>
C_BIN="$PWD/_ref/driver"
R_BIN="$PWD/translation/target/release/driver"
name="$1"
in="$2"
d=$(mktemp -d "$PWD/_ref/tmp.XXXXXX")
"$C_BIN" < "$in" > "$d/c.out" 2> "$d/c.err"; cec=$?
"$R_BIN" < "$in" > "$d/r.out" 2> "$d/r.err"; rec=$?
ok=1
if ! cmp -s "$d/c.out" "$d/r.out"; then
  echo "=== [$name] STDOUT DIFF ==="
  diff <(cat -A "$d/c.out") <(cat -A "$d/r.out") | head -40
  ok=0
fi
if ! cmp -s "$d/c.err" "$d/r.err"; then
  echo "=== [$name] STDERR DIFF ==="
  diff <(cat -A "$d/c.err") <(cat -A "$d/r.err") | head -20
  ok=0
fi
if [ "$cec" != "$rec" ]; then
  echo "=== [$name] EXIT DIFF: c=$cec r=$rec ==="
  ok=0
fi
if [ "$ok" = 1 ]; then
  echo "PASS [$name] ($(wc -c < "$d/c.out") bytes stdout, $(wc -c < "$d/c.err") bytes stderr)"
else
  echo "FAIL [$name]"
fi
rm -rf "$d"
[ "$ok" = 1 ]

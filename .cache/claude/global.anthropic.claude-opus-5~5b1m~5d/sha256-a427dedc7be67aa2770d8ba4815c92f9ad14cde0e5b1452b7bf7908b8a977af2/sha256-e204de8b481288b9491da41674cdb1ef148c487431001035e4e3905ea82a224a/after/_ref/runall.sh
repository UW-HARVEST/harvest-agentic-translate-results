#!/bin/bash
# Run every generated case against both binaries and report differences.
W=$HARVEST_WORKDIR
C_BIN="$W/_ref/driver"
R_BIN="$W/translation/target/release/driver"
OUTD="$W/_ref/out"
mkdir -p "$OUTD"
pass=0
fail=0
for in in "$W"/_ref/cases/*; do
  name=$(basename "$in")
  "$C_BIN" < "$in" > "$OUTD/$name.c.out" 2> "$OUTD/$name.c.err"; cec=$?
  "$R_BIN" < "$in" > "$OUTD/$name.r.out" 2> "$OUTD/$name.r.err"; rec=$?
  ok=1
  if ! cmp -s "$OUTD/$name.c.out" "$OUTD/$name.r.out"; then
    echo "### [$name] STDOUT DIFF"
    diff <(cat -A "$OUTD/$name.c.out") <(cat -A "$OUTD/$name.r.out") | head -30
    ok=0
  fi
  if ! cmp -s "$OUTD/$name.c.err" "$OUTD/$name.r.err"; then
    echo "### [$name] STDERR DIFF"
    diff <(cat -A "$OUTD/$name.c.err") <(cat -A "$OUTD/$name.r.err") | head -20
    ok=0
  fi
  if [ "$cec" != "$rec" ]; then
    echo "### [$name] EXIT DIFF: c=$cec r=$rec"
    ok=0
  fi
  if [ "$ok" = 1 ]; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL $name"; fi
done
echo "-----------------------------"
echo "PASS: $pass  FAIL: $fail"
[ "$fail" = 0 ]

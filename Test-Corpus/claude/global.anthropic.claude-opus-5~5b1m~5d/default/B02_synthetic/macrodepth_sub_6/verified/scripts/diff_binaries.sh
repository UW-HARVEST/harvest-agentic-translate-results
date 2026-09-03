#!/usr/bin/env bash
# Differential check of the *whole program* (mdmain.c vs src/main.rs) for every
# canonical (OP, REPEAT) configuration and a set of argument vectors.
set -u
cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"
OUT="${TMPDIR:-/tmp}/bindiff"
mkdir -p "$OUT"

ARGSETS=(
  ""            # argc == 1
  "7"           # argc == 2
  "3 4"
  "0 0"
  "-5 9"
  "2147483647 1"
  "-2147483648 -1"
  "2147483647 2147483647"
  "abc def"
  "  12x 0034"
  "99999999999999999999 -99999999999999999999"
  "+8 -8"
  "5 6 7"       # argc == 4 (extra args ignored)
)

fail=0
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    gcc -O2 -DOP="$op" -DREPEAT="$rep" -o "$OUT/cdriver" \
        "$ROOT/c_src/src/mdcore.c" "$ROOT/c_src/src/mdmain.c" || { echo "cc fail $op $rep"; fail=1; continue; }
    cargo build --quiet --release --no-default-features --features "$op,$rep" \
      || { echo "rust build fail $op $rep"; fail=1; continue; }
    cp target/release/driver "$OUT/rdriver"
    for a in "${ARGSETS[@]}"; do
      # shellcheck disable=SC2086
      co=$("$OUT/cdriver" $a 2>"$OUT/ce"); cs=$?
      # shellcheck disable=SC2086
      ro=$("$OUT/rdriver" $a 2>"$OUT/re"); rs=$?
      ce=$(sed "s|$OUT/cdriver|PROG|g" "$OUT/ce")
      re=$(sed "s|$OUT/rdriver|PROG|g" "$OUT/re")
      if [ "$co" != "$ro" ] || [ "$cs" != "$rs" ] || [ "$ce" != "$re" ]; then
        echo "DIFF op=$op rep=$rep args=[$a]"
        echo "  C : status=$cs out=<$co> err=<$ce>"
        echo "  R : status=$rs out=<$ro> err=<$re>"
        fail=1
      fi
    done
  done
done
echo "binary diff done fail=$fail"
exit $fail

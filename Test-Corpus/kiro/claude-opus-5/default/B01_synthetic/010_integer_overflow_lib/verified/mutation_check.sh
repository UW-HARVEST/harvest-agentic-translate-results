#!/usr/bin/env bash
# Sanity check that the differential suite has teeth.
#
# Builds deliberately WRONG variants of the C library in /tmp (c_src is never
# touched) and points the suite at them via DRIVER_RUST_SO. Every mutant MUST be
# detected; a mutant that passes means the suite has a blind spot.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
SRC="$ROOT/c_src/src/driver.c"
INC="$ROOT/c_src/include"
MUT="${TMPDIR:-/tmp}/driver_mutants.$$"
trap 'rm -rf "$MUT"' EXIT
mkdir -p "$MUT"

# name|description|sed expression
MUTANTS=(
  "m1|driver adds 2 instead of 1|s/data + 1/data + 2/"
  "m2|uppercase hex format %02X|s/%02x/%02X/"
  "m3|param is unsigned char (kills sign-extension)|s/void printHexCharLine (char charHex)/void printHexCharLine (unsigned char charHex)/"
  "m4|param is int (skips narrowing the arg register)|s/void printHexCharLine (char charHex)/void printHexCharLine (int charHex)/"
  "m5|field width 2 dropped from format|s/%02x/%x/"
  "m6|newline dropped from format|s/%02x\\\\n/%02x/"
  "m7|driver subtracts 1 instead of adding|s/data + 1/data - 1/"
)

fail=0
for entry in "${MUTANTS[@]}"; do
  IFS='|' read -r name desc expr <<<"$entry"
  sed "$expr" "$SRC" >"$MUT/$name.c"
  if cmp -s "$MUT/$name.c" "$SRC"; then
    echo "SKIP $name: sed produced an unchanged file (mutation expression stale)"
    fail=1
    continue
  fi
  if ! gcc -shared -fPIC -I "$INC" -o "$MUT/lib$name.so" "$MUT/$name.c" 2>/dev/null; then
    echo "SKIP $name: mutant did not compile"
    fail=1
    continue
  fi

  out=$(DRIVER_RUST_SO="$MUT/lib$name.so" timeout 600 cargo test --test differential -q 2>&1)
  if [ $? -eq 0 ]; then
    printf 'BLIND    %-3s %-50s <-- suite did NOT detect this\n' "$name" "$desc"
    fail=1
  else
    printf 'detected %-3s %-50s (%s rows failed)\n' \
      "$name" "$desc" "$(grep -c '^FAILED' <<<"$out")"
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "All mutants detected: the differential suite is sensitive to behaviour changes."
else
  echo "One or more mutants escaped detection."
fi
exit "$fail"

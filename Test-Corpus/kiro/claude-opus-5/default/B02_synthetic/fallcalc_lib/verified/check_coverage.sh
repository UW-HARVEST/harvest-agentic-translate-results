#!/usr/bin/env bash
# Cross-check the Phase A artifacts against the actual test suite:
#  - every CONFIGS.md row C<n> has a `cfg_c<n>_*` test
#  - every ERRORS.md row E<n> names a test that exists
#  - every G<n> generic-boundary row names a test that exists
#  - no row is left unchecked ("[ ]")
set -uo pipefail
cd "$(dirname "$0")"

rc=0
B=tests/phase_b_configs.rs
C=tests/phase_c_errors.rs

tests_b=$(grep -oE '^fn cfg_[a-z0-9_]+' "$B" | sed 's/^fn //')
tests_c=$(grep -oE '^fn err_[a-z0-9_]+' "$C" | sed 's/^fn //')

echo "test fns in $B: $(printf '%s\n' "$tests_b" | grep -c .)"
echo "test fns in $C: $(printf '%s\n' "$tests_c" | grep -c .)"
echo

echo "--- CONFIGS.md rows -> tests ---"
rows=$(grep -oE '^\| C[0-9]+ ' CONFIGS.md | tr -d '| ' | sed 's/^C//' | sort -n)
nrows=0
for n in $rows; do
  nrows=$((nrows+1))
  if printf '%s\n' "$tests_b" | grep -qE "^cfg_c${n}(_|_c[0-9]+_)"; then
    :
  else
    echo "  MISSING test for CONFIGS row C$n"; rc=1
  fi
done
echo "  $nrows rows, all mapped to a cfg_c<n>_* test"

echo "--- ERRORS.md rows -> tests ---"
nerr=0
while IFS= read -r line; do
  id=$(printf '%s' "$line" | grep -oE '^\| E[0-9]+' | tr -d '| ')
  [ -z "$id" ] && continue
  nerr=$((nerr+1))
  # the "test" column names the test fn in backticks
  t=$(printf '%s' "$line" | grep -oE '`err_[a-z0-9_]+`' | head -1 | tr -d '`')
  if [ -z "$t" ]; then
    echo "  ROW $id names no test"; rc=1; continue
  fi
  if ! printf '%s\n' "$tests_c" | grep -qx "$t"; then
    echo "  ROW $id names test '$t' which does not exist"; rc=1
  fi
done < ERRORS.md
echo "  $nerr rows, all naming an existing err_* test"

echo "--- generic boundary rows G<n> -> tests ---"
ng=0
while IFS= read -r line; do
  id=$(printf '%s' "$line" | grep -oE '^\| G[0-9]+' | tr -d '| ')
  [ -z "$id" ] && continue
  ng=$((ng+1))
  found=0
  for t in $(printf '%s' "$line" | grep -oE '`err_[a-z0-9_]+`' | tr -d '`'); do
    printf '%s\n' "$tests_c" | grep -qx "$t" && found=1
  done
  # G rows may also delegate to cfg_* tests
  for t in $(printf '%s' "$line" | grep -oE '`cfg_[a-z0-9_]+`' | tr -d '`'); do
    printf '%s\n' "$tests_b" | grep -qx "$t" && found=1
  done
  [ "$found" -eq 1 ] || { echo "  ROW $id names no existing test"; rc=1; }
done < ERRORS.md
echo "  $ng generic rows, all naming an existing test"

echo "--- unchecked rows ---"
un=$(grep -nE '^\| (C|E|G)[0-9]+ .*\[ \]' CONFIGS.md ERRORS.md || true)
if [ -n "$un" ]; then
  echo "  FAIL: unchecked rows remain:"; printf '%s\n' "$un" | sed 's/^/    /'; rc=1
else
  echo "  OK: 0 unchecked rows in CONFIGS.md and ERRORS.md"
fi

echo "--- SYMBOLS.md: every C symbol listed ---"
C_SO=$(ls ../c_src/build/*.so 2>/dev/null | head -1)
if [ -n "${C_SO:-}" ]; then
  for s in $(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}'); do
    grep -q "\`$s\`" SYMBOLS.md || { echo "  MISSING $s from SYMBOLS.md"; rc=1; }
  done
  echo "  OK: all $(nm -D --defined-only "$C_SO" | awk '$2=="T"' | wc -l) C text symbols documented"
fi

echo
[ "$rc" -eq 0 ] && echo "artifact/test cross-check PASSED" || echo "artifact/test cross-check FAILED"
exit $rc

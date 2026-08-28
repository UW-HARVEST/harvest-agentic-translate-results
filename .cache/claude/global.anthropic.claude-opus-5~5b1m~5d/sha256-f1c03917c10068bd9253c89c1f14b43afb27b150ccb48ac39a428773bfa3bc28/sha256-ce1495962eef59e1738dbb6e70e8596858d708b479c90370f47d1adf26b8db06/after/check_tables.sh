#!/bin/bash
# Mechanical gate for Phases A-C: every row in CONFIGS.md / ERRORS.md must name
# at least one test that (a) really exists in tests/ and (b) really passed, and
# every cfg_*/err_* test must be referenced by some row. Catches a table row that
# was checked off without a test, and a test that drifted out of the tables.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
CRATE="$ROOT/translation"
rc=0

PASSED="${TMPDIR:-/tmp}/passed.txt"
( cd "$CRATE" && timeout 600 cargo test 2>&1 ) \
  | grep -oE '^test [A-Za-z0-9_]+ \.\.\. ok' | awk '{print $2}' | sort -u > "$PASSED"
echo "tests that passed: $(wc -l < "$PASSED")"

# --- every row references an existing, passing test --------------------------
for f in CONFIGS.md ERRORS.md; do
  n_rows=0; n_bad=0
  while IFS= read -r line; do
    # data rows only: start with "| <number> |"
    [[ "$line" =~ ^\|[[:space:]]*[0-9]+[[:space:]]*\| ]] || continue
    num=$(printf '%s' "$line" | sed -E 's/^\|[[:space:]]*([0-9]+).*/\1/')
    n_rows=$((n_rows+1))
    # collect `code spans` that look like test names
    mapfile -t refs < <(printf '%s' "$line" | grep -oE '`(cfg|err|sym)_[A-Za-z0-9_]+`' | tr -d '`' | sort -u)
    if [ "${#refs[@]}" -eq 0 ]; then
      echo "  $f row $num: NO test referenced"; n_bad=$((n_bad+1)); continue
    fi
    ok=0
    for r in "${refs[@]}"; do
      if grep -qx "$r" "$PASSED"; then ok=1; else
        echo "  $f row $num: referenced test '$r' did not exist / did not pass"
        n_bad=$((n_bad+1))
      fi
    done
    [ "$ok" -eq 1 ] || { echo "  $f row $num: no passing test"; n_bad=$((n_bad+1)); }
    # the row must be checked off
    printf '%s' "$line" | grep -q '\[x\]' || { echo "  $f row $num: not checked off"; n_bad=$((n_bad+1)); }
  done < "$CRATE/$f"
  if [ "$n_bad" -eq 0 ]; then
    echo "$f: all $n_rows rows map to a passing, checked-off test"
  else
    echo "$f: $n_bad problem(s) across $n_rows rows"; rc=1
  fi
done

# --- every test is referenced by some row ------------------------------------
unref=0
while read -r t; do
  case "$t" in cfg_*|err_*) ;; *) continue ;; esac
  if ! grep -qF "$t" "$CRATE/CONFIGS.md" "$CRATE/ERRORS.md"; then
    echo "  test '$t' is not referenced by any table row"; unref=$((unref+1))
  fi
done < "$PASSED"
[ "$unref" -eq 0 ] && echo "every cfg_/err_ test is referenced by a table row" \
                   || { echo "$unref unreferenced test(s)"; rc=1; }

[ "$rc" -eq 0 ] && echo "TABLE GATE PASSED" || echo "TABLE GATE FAILED"
exit "$rc"

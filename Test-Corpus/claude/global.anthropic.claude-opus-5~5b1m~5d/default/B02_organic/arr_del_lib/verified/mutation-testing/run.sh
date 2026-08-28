#!/usr/bin/env bash
# Suite-sensitivity check: inject one deliberate bug into src/lib.rs at a time,
# rebuild the cdylib, run the whole differential suite, and report whether the
# suite noticed.  Always restores src/lib.rs afterwards.
#
#   usage: mutation-testing/run.sh [MUTANT_NAME ...]      (default: all)
set -u
cd "$(dirname "$0")/.."
BK=$(mktemp) ; cp src/lib.rs "$BK"
restore() { cp "$BK" src/lib.rs; timeout 300 cargo build --release >/dev/null 2>&1; }
trap 'restore; rm -f "$BK"' EXIT

NAMES="${*:-$(python3 - <<'PY'
import re
print(' '.join(re.findall(r'"(M\d+_\w+)"', open('mutation-testing/mutate.py').read())))
PY
)}"
for M in $NAMES; do
  cp "$BK" src/lib.rs
  R=$(BK="$BK" python3 mutation-testing/mutate.py "$M" 2>&1)
  [ "$R" = "MUT-APPLIED" ] || { echo "$M : SKIP ($R)"; continue; }
  timeout 300 cargo build --release >/dev/null 2>&1 || { echo "$M : BUILD-FAILED"; continue; }
  OUT=$(timeout 300 cargo test --release 2>&1)
  RC=$?
  SUM=$(printf '%s\n' "$OUT" | grep -E "^test .* FAILED|SIGSEGV|SIGABRT|WATCHDOG" | sort -u | head -3)
  if [ "$RC" -ne 0 ]; then
    echo "$M : CAUGHT"; [ -n "$SUM" ] && printf '%s\n' "$SUM" | sed 's/^/        /'
  else
    echo "$M : *** NOT CAUGHT ***"
  fi
done

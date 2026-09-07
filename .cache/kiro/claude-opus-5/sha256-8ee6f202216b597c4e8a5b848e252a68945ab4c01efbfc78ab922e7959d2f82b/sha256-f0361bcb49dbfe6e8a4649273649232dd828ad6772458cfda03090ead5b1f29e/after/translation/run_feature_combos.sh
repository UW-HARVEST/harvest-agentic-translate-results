#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY cargo feature
# combination. Feature names are extracted from Cargo.toml rather than
# hard-coded, so this stays correct if features are added later.
set -uo pipefail
cd "$(dirname "$0")"

# --- extract feature names from the [features] table (excluding "default") ---
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', s, re.M | re.S)
if not m:
    print('', end='')
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(' '.join(n for n in names if n != 'default'), end='')
PY
)

# --- build the list of combinations to test ---
declare -a COMBOS_DESC=()
declare -a COMBOS_ARGS=()

add() { COMBOS_DESC+=("$1"); COMBOS_ARGS+=("$2"); }

add "default"                ""
add "no-default-features"    "--no-default-features"
add "all-features"           "--all-features"

if [ -n "$FEATURES" ]; then
  read -ra FARR <<< "$FEATURES"
  n=${#FARR[@]}
  # full power set of the declared features, with default features off
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FARR[i]}"; fi
    done
    add "no-default+{${combo:-<none>}}" "--no-default-features${combo:+ --features $combo}"
  done
else
  echo "note: Cargo.toml declares no [features] table — 'default',"
  echo "      '--no-default-features' and '--all-features' are the same build."
fi

# --- run ---
rc=0
for i in "${!COMBOS_DESC[@]}"; do
  desc="${COMBOS_DESC[$i]}"
  args="${COMBOS_ARGS[$i]}"
  printf '=== combo: %-28s args: %s\n' "$desc" "${args:-<none>}"

  # The tests dlopen the RELEASE cdylib, so it must be rebuilt per combination.
  if ! timeout 600 cargo build --release $args >/tmp/fc_build.log 2>&1; then
    echo "    BUILD FAILED"; tail -20 /tmp/fc_build.log; rc=1; continue
  fi
  if ! timeout 600 cargo check $args >/tmp/fc_check.log 2>&1; then
    echo "    CHECK FAILED"; tail -20 /tmp/fc_check.log; rc=1; continue
  fi
  if timeout 600 cargo test $args >/tmp/fc_test.log 2>&1; then
    echo "    PASS  ($(grep -ho '[0-9]\+ passed' /tmp/fc_test.log | awk '{s+=$1} END {print s}') tests)"
  else
    echo "    TEST FAILED"; grep -E '^(test result|---- |thread)' /tmp/fc_test.log | head -30; rc=1
  fi
done

echo
[ "$rc" -eq 0 ] && echo "ALL FEATURE COMBINATIONS PASS" || echo "FAILURES — see above"
exit "$rc"

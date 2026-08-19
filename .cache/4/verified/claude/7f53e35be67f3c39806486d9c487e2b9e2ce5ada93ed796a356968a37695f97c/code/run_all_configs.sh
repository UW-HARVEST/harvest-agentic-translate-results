#!/usr/bin/env bash
# Enumerates every valid cargo feature combination from Cargo.toml and runs
# `cargo check` + the full differential test suite for each of them.
#
# `Cargo.toml` declares no `[features]`, so the power set is the single empty
# combination; the loop below derives that mechanically instead of assuming it,
# and additionally covers the `--all-features` / default-features invocations.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

# --- enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S | re.M)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name and name != 'default':
                print(name)
PY
)

echo "declared features: ${#FEATURES[@]} -> ${FEATURES[*]:-<none>}"

# --- build the power set of feature combinations ---------------------------
COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

rc=0
run() { # run <label> <extra cargo args...>
  local label="$1"; shift
  echo "=============================================================="
  echo "== $label"
  echo "=============================================================="
  local out
  out=$(timeout 600 cargo check --offline --all-targets "$@" 2>&1)
  if [[ $? -ne 0 ]]; then
    echo "$out" | tail -n 30; echo "CHECK FAILED: $label"; rc=1; return
  fi
  echo "$out" | grep -E "^(warning|error)" | sort | uniq -c || true
  out=$(timeout 600 cargo test --offline "$@" 2>&1)
  if [[ $? -ne 0 ]]; then
    echo "$out" | tail -n 40; echo "TESTS FAILED: $label"; rc=1; return
  fi
  echo "$out" | grep -E "^(test result|running|error|warning: unused)" || true
}

for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    run "--no-default-features (empty feature set)" --no-default-features
  else
    run "--no-default-features --features $combo" --no-default-features --features "$combo"
  fi
done

run "default features"
run "--all-features" --all-features

echo
if (( rc == 0 )); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $rc

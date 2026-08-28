#!/usr/bin/env bash
# Differential verification of translation/ against c_src/.
#
#   1. enumerate every valid Cargo feature combination
#   2. `cargo check` each one
#   3. build the C reference shared library
#   4. run the differential test suite for each combination, debug and release
#
# Usage: ./verify.sh [DIFF_ITERS]
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
iters="${1:-}"
log=/tmp/verify-translation.log
: >"$log"
fail=0

note() { printf '%s\n' "$*"; }
run() { # run <label> <cmd...>
  local label="$1"; shift
  printf '\n===== %s =====\n%s\n' "$label" "$*" >>"$log"
  if timeout 600 "$@" >>"$log" 2>&1; then
    note "  PASS  $label"
  else
    note "  FAIL  $label  (see $log)"
    fail=1
  fi
}

# ---------------------------------------------------------------------------
# 1. feature combinations
# ---------------------------------------------------------------------------
mapfile -t all_lines < <(
  python3 - "$root/translation/Cargo.toml" <<'PY'
import sys, re
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if '=' in line:
            name = line.split('=', 1)[0].strip().strip('"')
            if name != 'default':
                names.append(name)
print('\n'.join(names))
PY
)
features=()
for f in "${all_lines[@]}"; do
  [ -n "$f" ] && features+=("$f")
done

combos=()
if [ "${#features[@]}" -eq 0 ]; then
  note "Cargo.toml declares no [features]; the crate has a single configuration."
  combos=("")
else
  n=${#features[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${features[b]}")
    done
    combos+=("$(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi
note "Feature combinations to verify: ${#combos[@]}"

# ---------------------------------------------------------------------------
# 2. cargo check
# ---------------------------------------------------------------------------
note ""
note "cargo check:"
for combo in "${combos[@]}"; do
  if [ -z "$combo" ]; then
    run "check --no-default-features" cargo check --manifest-path "$root/translation/Cargo.toml" \
      --no-default-features --all-targets
    run "check (default features)" cargo check --manifest-path "$root/translation/Cargo.toml" \
      --all-targets
  else
    run "check --features $combo" cargo check --manifest-path "$root/translation/Cargo.toml" \
      --no-default-features --features "$combo" --all-targets
  fi
done

# ---------------------------------------------------------------------------
# 3. C reference library
# ---------------------------------------------------------------------------
note ""
note "C reference library:"
mkdir -p "$root/c_src/build"
run "cmake configure" cmake -S "$root/c_src" -B "$root/c_src/build" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run "cmake build" cmake --build "$root/c_src/build"

# ---------------------------------------------------------------------------
# 4. differential tests
# ---------------------------------------------------------------------------
note ""
note "differential tests:"
[ -n "$iters" ] && export DIFF_ITERS="$iters"
for combo in "${combos[@]}"; do
  for profile in debug release; do
    args=(cargo test --manifest-path "$root/translation/Cargo.toml")
    [ "$profile" = release ] && args+=(--release)
    if [ -z "$combo" ]; then
      label="test $profile (default features)"
    else
      args+=(--no-default-features --features "$combo")
      label="test $profile --features $combo"
    fi
    run "$label" "${args[@]}"
  done
done

note ""
if [ "$fail" -eq 0 ]; then
  note "ALL CHECKS PASSED"
else
  note "FAILURES PRESENT - inspect $log"
fi
exit "$fail"

#!/usr/bin/env bash
# Full verification driver: builds the C reference, enumerates every Cargo
# feature combination, and runs the differential suite + symbol diff for each,
# in both the dev and release profiles.
#
# Usage: translation/scripts/verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$CRATE/.." && pwd)"
FAIL=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   [ok]   %s\n' "$*"; }
bad()  { printf '   [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "1. Build the C reference shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "C .so built" || bad "C build failed"

C_SO="$(find "$ROOT/c_src/build" -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] && ok "C .so: $C_SO" || bad "no C .so found"

# ---------------------------------------------------------------------------
step "2. Enumerate feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  python3 - "$CRATE/Cargo.toml" <<'PY'
import re, sys
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
for n in names:
    print(n)
PY
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  ok "no [features] table -> the only configuration is the default (empty) one"
  COMBOS+=("default:")
  COMBOS+=("no-default:")
else
  ok "features: ${FEATURES[*]}"
  COMBOS+=("default:")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("no-default:$(IFS=,; echo "${sel[*]}")")
  done
fi
ok "${#COMBOS[@]} combination(s) to verify"

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" profile="$2"
  local kind="${combo%%:*}" feats="${combo#*:}"
  local -a fargs=()
  [ "$kind" = "no-default" ] && fargs+=(--no-default-features)
  [ -n "$feats" ] && fargs+=(--features "$feats")
  local -a pargs=()
  [ "$profile" = "release" ] && pargs+=(--release)

  local label="[$profile] ${kind}${feats:+ +$feats}"

  ( cd "$CRATE" && timeout 300 cargo check "${fargs[@]}" "${pargs[@]}" >/dev/null 2>&1 ) \
    && ok "$label cargo check" || bad "$label cargo check"

  ( cd "$CRATE" && timeout 300 cargo build --lib "${fargs[@]}" "${pargs[@]}" >/dev/null 2>&1 ) \
    && ok "$label cargo build" || bad "$label cargo build"

  local out
  out="$(cd "$CRATE" && timeout 600 cargo test "${fargs[@]}" "${pargs[@]}" 2>&1)"
  if grep -q 'test result: FAILED' <<<"$out"; then
    bad "$label cargo test"
    grep -E 'panicked|test result: FAILED' <<<"$out" | head -10
  else
    local passed
    passed="$(grep -oE '[0-9]+ passed' <<<"$out" | awk '{s+=$1} END {print s}')"
    ok "$label cargo test — $passed assertions/tests passed"
  fi

  # Symbol parity for the artifact this combination produced.
  local r_so="$CRATE/target/$([ "$profile" = release ] && echo release || echo debug)/libdiv_euclid_lib.so"
  if [ -f "$r_so" ] && [ -n "$C_SO" ]; then
    local missing
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
      <(nm -D --defined-only "$r_so"  | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u))"
    if [ -z "$missing" ]; then
      ok "$label symbol diff empty"
    else
      bad "$label missing symbols: $(tr '\n' ' ' <<<"$missing")"
    fi
  else
    bad "$label missing artifact $r_so"
  fi
}

step "3. Verify every combination in both profiles"
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    run_combo "$combo" "$profile"
  done
done

# ---------------------------------------------------------------------------
step "4. Result"
if [ "$FAIL" -eq 0 ]; then
  printf '   \033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '   \033[31mFAILURES PRESENT\033[0m\n'
fi
exit "$FAIL"

#!/usr/bin/env bash
# Enumerates every Cargo feature combination and runs the full verification
# matrix (build + symbol diff + Phase B + Phase C) for each one.
#
# Usage:  ./scripts/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")/.."
CRATE="$PWD"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; }
bad()  { printf '  FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 300 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 300 cmake --build . >/dev/null ) \
  && ok "libdriver.so (C)" || { bad "C build"; exit 1; }

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# Features listed under a [features] section, excluding the `default` key.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); if ($0 != "default") print }' Cargo.toml
)

# Power set of FEATURES, always including the plain default build.
COMBOS=("default")
if ((${#FEATURES[@]} > 0)); then
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do (((mask >> i) & 1)) && sel+=("${FEATURES[i]}"); done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
else
  # No [features] section: --no-default-features is the same build, but run it
  # anyway so the claim in CONFIGS.md is actually exercised.
  COMBOS+=("--no-default-features")
fi
printf '  %d combination(s):\n' "${#COMBOS[@]}"
printf '    %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
declare -a SYMSETS=()
for combo in "${COMBOS[@]}"; do
  flags=()
  [[ $combo != default ]] && read -r -a flags <<<"$combo"

  step "Combination: $combo"

  timeout 600 cargo build --release "${flags[@]}" >/dev/null 2>&1 \
    && ok "cargo build --release" || { bad "cargo build ($combo)"; continue; }

  RS_SO="$CRATE/target/release/libdriver.so"

  # -- symbol parity -------------------------------------------------------
  c_syms=$(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort)
  r_syms=$(nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort)
  if [[ "$c_syms" == "$r_syms" ]]; then
    ok "nm -D symbol diff is empty ($(wc -l <<<"$c_syms") symbols)"
  else
    bad "symbol diff ($combo):"
    diff <(echo "$c_syms") <(echo "$r_syms") | sed 's/^/        /'
  fi
  SYMSETS+=("$(md5sum <<<"$r_syms" | cut -d' ' -f1)")

  # -- Phase B / C / D tests ----------------------------------------------
  for t in symbol_parity phase_b_configs phase_c_errors; do
    if timeout 600 cargo test "${flags[@]}" --test "$t" -- --test-threads=1 >/tmp/vt.$$.log 2>&1; then
      ok "$t ($(grep -oP '\d+(?= passed)' /tmp/vt.$$.log | head -1) tests)"
    else
      bad "$t ($combo)"
      tail -30 /tmp/vt.$$.log | sed 's/^/        /'
    fi
  done
  rm -f /tmp/vt.$$.log
done

# ---------------------------------------------------------------------------
step "Cross-combination consistency"
if [[ $(printf '%s\n' "${SYMSETS[@]}" | sort -u | wc -l) -eq 1 ]]; then
  ok "all combinations export an identical symbol set"
else
  bad "combinations export different symbol sets"
fi

step "Result"
if ((fail)); then
  echo "  VERIFICATION FAILED"
  exit 1
fi
echo "  ALL COMBINATIONS VERIFIED"

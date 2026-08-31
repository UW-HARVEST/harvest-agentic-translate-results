#!/usr/bin/env bash
# Verifies the Rust translation against the C ground truth for every build-time
# configuration and both cargo profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/verify_driver
mkdir -p "$LOG"
rc=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ OK ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; rc=1; }

# --- 1. enumerate feature combinations ------------------------------------
step "Feature combinations declared in Cargo.toml"
FEATURES=$(python3 - "$CRATE/Cargo.toml" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
if not m:
    print("", end="")
else:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            print(line.split('=')[0].strip())
PY
)
if [ -z "$FEATURES" ]; then
  echo "  none declared -> the only configuration is the empty feature set"
  COMBOS=("")
else
  # Power set of the declared features.
  mapfile -t FEATLIST <<<"$FEATURES"
  COMBOS=()
  n=${#FEATLIST[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FEATLIST[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  printf '  %s\n' "${FEATLIST[@]}"
fi
echo "  -> ${#COMBOS[@]} combination(s) to verify"

# --- 2. build the C ground truth -------------------------------------------
step "Building the C shared library"
if (cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && cmake --build .) > "$LOG/cmake.log" 2>&1; then
  ok "c_src/build/libdriver.so"
else
  bad "C build failed (see $LOG/cmake.log)"; exit 1
fi
C_SO="$ROOT/c_src/build/libdriver.so"

# --- 3-5. per-combination check, build, symbol diff, differential tests ----
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  slug="$(echo "${combo:-none}" | tr ',' '_')"
  step "Configuration: $label"

  if timeout 600 cargo check --manifest-path "$CRATE/Cargo.toml" \
        --all-targets --no-default-features ${combo:+--features "$combo"} \
        > "$LOG/check_$slug.log" 2>&1; then
    if grep -q "^warning" "$LOG/check_$slug.log"; then
      ok "cargo check (with warnings, see $LOG/check_$slug.log)"
    else
      ok "cargo check clean, no warnings"
    fi
  else
    bad "cargo check failed (see $LOG/check_$slug.log)"; continue
  fi

  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"

    if timeout 600 cargo build --manifest-path "$CRATE/Cargo.toml" $relflag \
          --no-default-features ${combo:+--features "$combo"} \
          > "$LOG/build_${slug}_$profile.log" 2>&1; then
      ok "cargo build ($profile)"
    else
      bad "cargo build ($profile) failed"; continue
    fi

    # Symbol parity: every dynamic symbol the C .so defines must also be
    # defined by the Rust .so under the exact same name.
    RUST_SO="$CRATE/target/$profile/libdriver.so"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
    if [ -z "$missing" ]; then
      ok "symbol parity ($profile): $(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u | tr '\n' ' ')"
    else
      bad "symbols exported by C but missing from Rust ($profile): $missing"
    fi

    if timeout 600 cargo test --manifest-path "$CRATE/Cargo.toml" $relflag \
          --no-default-features ${combo:+--features "$combo"} \
          > "$LOG/test_${slug}_$profile.log" 2>&1; then
      passed=$(grep -o 'test result: ok\. [0-9]* passed' "$LOG/test_${slug}_$profile.log" \
               | awk '{s+=$4} END {print s+0}')
      ok "cargo test ($profile): $passed differential tests passed"
    else
      bad "cargo test ($profile) failed (see $LOG/test_${slug}_$profile.log)"
      tail -25 "$LOG/test_${slug}_$profile.log"
    fi
  done
done

step "Result"
if [ $rc -eq 0 ]; then
  echo "  ALL CONFIGURATIONS VERIFIED"
else
  echo "  FAILURES PRESENT"
fi
exit $rc

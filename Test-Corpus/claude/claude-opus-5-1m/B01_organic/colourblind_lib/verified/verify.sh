#!/usr/bin/env bash
# Phase D driver: run every phase under EVERY feature combination and profile,
# then diff the exported symbols of the two shared objects.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$PWD
LOG_DIR=${TMPDIR:-/tmp}/cb_verify
mkdir -p "$LOG_DIR"

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 0. Enumerate feature combinations from Cargo.toml (powerset)
# ---------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
src = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != "default":
            names.append(name)
for n in names:
    print(n)
PY
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  no [features] declared -> exactly one combination (the empty set)"
  COMBOS=("")
else
  echo "  features: ${FEATURES[*]}"
  mapfile -t COMBOS < <(python3 - "${FEATURES[@]}" <<'PY'
import itertools, sys
feats = sys.argv[1:]
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(",".join(c))
PY
)
fi
echo "  ${#COMBOS[@]} combination(s) to verify"

# ---------------------------------------------------------------------------
# 1. Build the C shared library (the ground truth)
# ---------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p c_src/build
if (cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && cmake --build .) > "$LOG_DIR/cmake.log" 2>&1; then
  ok "cmake build"
else
  bad "cmake build (see $LOG_DIR/cmake.log)"
  tail -20 "$LOG_DIR/cmake.log"
  exit 1
fi
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)
echo "  C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. cargo check / build / test for every combination x profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label=${combo:-<none>}
  if [ -z "$combo" ]; then
    featflags=(--no-default-features)
  else
    featflags=(--no-default-features --features "$combo")
  fi

  note "Feature combination: $label"

  tag=${combo//,/_}; tag=${tag:-none}

  if timeout 600 cargo check "${featflags[@]}" --all-targets \
        > "$LOG_DIR/check_$tag.log" 2>&1; then
    ok "cargo check [$label]"
  else
    bad "cargo check [$label] (see $LOG_DIR/check_$tag.log)"
    tail -30 "$LOG_DIR/check_$tag.log"
  fi

  for profile in dev release; do
    if [ "$profile" = release ]; then relflag=(--release); else relflag=(); fi

    if timeout 600 cargo build "${featflags[@]}" "${relflag[@]}" --lib \
          > "$LOG_DIR/build_${tag}_$profile.log" 2>&1; then
      ok "cargo build --lib [$label/$profile]"
    else
      bad "cargo build --lib [$label/$profile]"
      tail -30 "$LOG_DIR/build_${tag}_$profile.log"
      continue
    fi

    # Phases B, C and D all run from the test suite.
    if CB_TEST_FEATURES="$combo" timeout 600 cargo test "${featflags[@]}" "${relflag[@]}" \
          > "$LOG_DIR/test_${tag}_$profile.log" 2>&1; then
      passed=$(grep -c '^test .* \.\.\. ok$' "$LOG_DIR/test_${tag}_$profile.log")
      ok "cargo test [$label/$profile] — $passed tests passed"
    else
      bad "cargo test [$label/$profile] (see $LOG_DIR/test_${tag}_$profile.log)"
      grep -E '^(test .*FAILED|failures:|thread)' \
        "$LOG_DIR/test_${tag}_$profile.log" | head -30
    fi

    # ---- symbol parity for this exact artifact ----
    if [ "$profile" = release ]; then
      RUST_SO=$ROOT/target/release/libcolourblind_lib.so
    else
      RUST_SO=$ROOT/target/debug/libcolourblind_lib.so
    fi

    nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' \
      | grep -Ev '^(_ITM_|__cxa_|__gmon_|_init|_fini)' | sort -u \
      > "$LOG_DIR/c_syms.txt"
    nm -D --defined-only "$RUST_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u \
      > "$LOG_DIR/rust_syms_${tag}_$profile.txt"

    missing=$(comm -23 "$LOG_DIR/c_syms.txt" "$LOG_DIR/rust_syms_${tag}_$profile.txt")
    if [ -z "$missing" ]; then
      ok "symbol parity [$label/$profile] — $(wc -l < "$LOG_DIR/c_syms.txt") C symbol(s), 0 missing"
    else
      bad "symbol parity [$label/$profile] — missing from Rust .so:"
      echo "$missing" | sed 's/^/      /'
    fi
  done
done

# ---------------------------------------------------------------------------
# 3. Summary
# ---------------------------------------------------------------------------
note "Summary"
if [ "$fail" -eq 0 ]; then
  printf '  \033[32mALL CHECKS PASSED\033[0m across %d feature combination(s) x 2 profiles\n' \
    "${#COMBOS[@]}"
else
  printf '  \033[31mSOME CHECKS FAILED\033[0m — logs in %s\n' "$LOG_DIR"
fi
exit "$fail"

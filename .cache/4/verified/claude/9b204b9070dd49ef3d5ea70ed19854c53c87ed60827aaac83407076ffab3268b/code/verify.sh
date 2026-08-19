#!/usr/bin/env bash
# Full differential verification driver.
#
#   ./verify.sh
#
# 1. builds the C shared library
# 2. mechanically enumerates every feature combination from Cargo.toml
# 3. for each combination: cargo check, rebuild the cdylib, run every test
# 4. diffs the exported-symbol sets of the two .so files
#
# IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` lib
# target (integration tests do not link it), so an explicit `cargo build` is
# required before each test run or the tests would load a stale .so.  The test
# harness also asserts .so freshness so this can never silently regress.

set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
rc=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; rc=1; }
ok()   { printf '\033[32mok\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------------------
step "1. Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "c_src/build/libdriver.so" || { fail "C build"; exit 1; }

# ---------------------------------------------------------------------------
step "2. Enumerate feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
# Collect the feature names declared under [features] (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
NF_=${#FEATURES[@]}
echo "declared features (${NF_}): ${FEATURES[*]:-<none>}"

# Full power set of the declared features -> the list of combos to verify.
COMBOS=("")   # the empty combination (--no-default-features) is always valid
if (( NF_ > 0 )); then
  total=$(( 1 << NF_ ))
  for (( m = 1; m < total; m++ )); do
    combo=""
    for (( b = 0; b < NF_; b++ )); do
      if (( m & (1 << b) )); then combo+="${combo:+,}${FEATURES[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<no features>}'"; done

# ---------------------------------------------------------------------------
step "3. cargo check + build + test for every feature combination"
# ---------------------------------------------------------------------------
# `[profile.release] panic = "abort"` makes the release profile a genuinely
# different build configuration, so both profiles are verified.
for profile in dev release; do
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>} / $profile"
  if [[ -n "$combo" ]]; then FLAGS=(--no-default-features --features "$combo")
  else                       FLAGS=(--no-default-features); fi
  [[ $profile == release ]] && FLAGS+=(--release)

  printf '\n--- combination: %s ---\n' "$label"

  if timeout 600 cargo check --offline "${FLAGS[@]}" --all-targets >"$ROOT/.verify.log" 2>&1; then
    ok "cargo check [$label]"
  else
    fail "cargo check [$label]"; tail -30 "$ROOT/.verify.log"; continue
  fi

  # Rebuild the cdylib so the tests load a fresh .so (see note at the top).
  if timeout 600 cargo build --offline "${FLAGS[@]}" >"$ROOT/.verify.log" 2>&1; then
    ok "cargo build [$label]"
  else
    fail "cargo build [$label]"; tail -30 "$ROOT/.verify.log"; continue
  fi

  if timeout 600 cargo test --offline "${FLAGS[@]}" >"$ROOT/.verify.log" 2>&1; then
    ok "cargo test  [$label]  $(grep -hoE '[0-9]+ passed' "$ROOT/.verify.log" | awk '{s+=$1} END {print s" tests passed"}')"
  else
    fail "cargo test [$label]"
    grep -E "^(test result|failures:|\[C[0-9]+\]|\[E[0-9]+\]|\[G[0-9]+\])" "$ROOT/.verify.log" | head -40
  fi
done
done

# ---------------------------------------------------------------------------
step "4. Exported-symbol diff (C .so vs Rust .so)"
# ---------------------------------------------------------------------------
syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TDBRWVi]$/ {print $3}' | sort -u; }
syms c_src/build/libdriver.so >"$ROOT/.syms_c"
echo "C exports:    $(tr '\n' ' ' <"$ROOT/.syms_c")"

for RUST_SO in target/debug/libdriver.so target/release/libdriver.so; do
  [[ -f $RUST_SO ]] || continue
  syms "$RUST_SO" | grep -vE '^(_init|_fini|_edata|_end|__bss_start|_ITM_|__gmon_start__|__cxa_finalize|rust_|_R|_ZN|__rust|__rdl_|__rg_)' >"$ROOT/.syms_rust"
  echo "Rust exports ($RUST_SO): $(tr '\n' ' ' <"$ROOT/.syms_rust")"
  MISSING=$(comm -23 "$ROOT/.syms_c" "$ROOT/.syms_rust")
  if [[ -z "$MISSING" ]]; then
    ok "symbol diff is EMPTY for $RUST_SO"
  else
    fail "$RUST_SO is missing C symbols:"; echo "$MISSING"
  fi
done
rm -f "$ROOT/.syms_c" "$ROOT/.syms_rust" "$ROOT/.verify.log"

# ---------------------------------------------------------------------------
printf '\n'
if (( rc == 0 )); then printf '\033[32mALL VERIFICATION PASSED\033[0m\n'
else                   printf '\033[31mVERIFICATION FAILED\033[0m\n'; fi
exit $rc

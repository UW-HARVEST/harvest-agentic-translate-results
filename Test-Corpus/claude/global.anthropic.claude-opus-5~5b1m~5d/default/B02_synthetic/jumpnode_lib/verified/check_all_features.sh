#!/usr/bin/env bash
# Runs the full verification (Phases A-D) across EVERY cargo feature
# combination. Feature list is extracted from Cargo.toml rather than hard-coded.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
LOGS="target/logs"
mkdir -p "$LOGS"

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad() { printf '\033[31mFAIL\033[0m %s\n' "$*"; fail=1; }
good() { printf '\033[32mok\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library
# ---------------------------------------------------------------------------
note "Building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOGS/c-build.log" 2>&1 \
  || { bad "C build failed (see $LOGS/c-build.log)"; tail -20 "$LOGS/c-build.log"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
good "C .so = $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{ sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "default" && $0 != "") print }' Cargo.toml
)
note "Declared features: ${FEATURES[*]:-<none>}"

# Full power set of the declared features.
COMBOS=("")
for f in "${FEATURES[@]}"; do
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done
note "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<default/none>}'"; done

# ---------------------------------------------------------------------------
# 2. Per-combination: check, build cdylib, symbol diff, test
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  tag="${combo:-default}"
  safe="${tag//,/_}"
  note "Feature combo: '${combo:-<none>}'"

  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  if cargo check "${args[@]}" --all-targets > "$LOGS/check-$safe.log" 2>&1; then
    good "cargo check"
  else
    bad "cargo check ('$combo') — see $LOGS/check-$safe.log"
    tail -30 "$LOGS/check-$safe.log"
    continue
  fi

  TD="$PWD/target/sotest/$safe"
  if CARGO_TARGET_DIR="$TD" cargo build --release --lib "${args[@]}" \
       > "$LOGS/build-$safe.log" 2>&1; then
    good "cargo build --release --lib"
  else
    bad "cargo build ('$combo') — see $LOGS/build-$safe.log"
    tail -30 "$LOGS/build-$safe.log"
    continue
  fi
  R_SO="$TD/release/libjumpnode_lib.so"

  # --- Phase D symbol diff -------------------------------------------------
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$LOGS/sym-c.txt"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$LOGS/sym-r-$safe.txt"
  missing="$(comm -23 "$LOGS/sym-c.txt" "$LOGS/sym-r-$safe.txt")"
  extra="$(comm -13 "$LOGS/sym-c.txt" "$LOGS/sym-r-$safe.txt")"
  if [[ -z "$missing" ]]; then
    good "symbol parity: 0 C symbols missing from Rust .so"
  else
    bad "symbols exported by C but MISSING from Rust: $(echo "$missing" | tr '\n' ' ')"
  fi
  [[ -n "$extra" ]] && echo "     (Rust-only symbols: $(echo "$extra" | tr '\n' ' '))"

  # --- Phases B & C --------------------------------------------------------
  if JUMPNODE_RUST_SO="$R_SO" timeout 600 cargo test "${args[@]}" \
       > "$LOGS/test-$safe.log" 2>&1; then
    good "cargo test ($(grep -c '^test .* \.\.\. ok$' "$LOGS/test-$safe.log") tests passed)"
    grep -E '^test result:' "$LOGS/test-$safe.log" | sed 's/^/     /'
  else
    bad "cargo test ('$combo') — see $LOGS/test-$safe.log"
    grep -E '^(test result:|---- |thread .* panicked|failures:)' "$LOGS/test-$safe.log" \
      | head -40 | sed 's/^/     /'
  fi
done

note "SUMMARY"
if [[ $fail -eq 0 ]]; then
  printf '\033[32mALL FEATURE COMBINATIONS VERIFIED\033[0m\n'
else
  printf '\033[31mVERIFICATION FAILED\033[0m\n'
fi
exit $fail

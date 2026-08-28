#!/usr/bin/env bash
# Full verification gate: builds both libraries and runs Phases B/C/D against
# every feature combination and against BOTH Rust build profiles.
#
#   ./verify.sh            # normal run
#   DIFF_ITERS=200 ./verify.sh   # quick smoke run
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
CARGO="cargo --offline"
FAILED=0

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
ok "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations straight out of Cargo.toml.
step "Enumerating feature combinations from Cargo.toml"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' "$CRATE_DIR/Cargo.toml")
if [ -z "$FEATURES" ]; then
  echo "  no [features] table -> the only configurations are the default build"
  echo "  and --no-default-features (equivalent here)."
  COMBOS=("" "--no-default-features" "--all-features")
else
  COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
fi
printf '  combos: %s\n' "$(printf '[%s] ' "${COMBOS[@]}")"

# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  if $CARGO check --tests $combo >/dev/null 2>&1; then
    ok "cargo check ${combo:-<default>}"
  else
    bad "cargo check ${combo:-<default>}"
  fi
done

# ---------------------------------------------------------------------------
step "Symbol parity (nm -D)"
$CARGO build --release >/dev/null 2>&1 || { bad "release build"; exit 1; }
$CARGO build          >/dev/null 2>&1 || { bad "debug build";   exit 1; }
R_SO="$CRATE_DIR/target/release/libupdate_frame_header_lib.so"
D_SO="$CRATE_DIR/target/debug/libupdate_frame_header_lib.so"
for so in "$R_SO" "$D_SO"; do
  MISSING=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$so"  | awk '{print $NF}' | sort -u))
  if [ -z "$MISSING" ]; then
    ok "0 symbols missing from $(basename "$(dirname "$so")")/$(basename "$so")"
  else
    bad "missing symbols in $so: $MISSING"
  fi
done

# ---------------------------------------------------------------------------
# Run the differential suite for each feature combo, once against the debug
# cdylib and once against the release cdylib (the shipped artifact).
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    step "Phases B+C+D  |  features: ${combo:-<default>}  |  Rust .so: $profile"
    if [ "$profile" = release ]; then SO="$R_SO"; else SO="$D_SO"; fi
    $CARGO build          $combo >/dev/null 2>&1
    $CARGO build --release $combo >/dev/null 2>&1
    if C_LIB="$C_SO" RUST_LIB="$SO" $CARGO test $combo -- --test-threads="$(nproc)" 2>&1 \
         | grep -E "^(test result|running|error|warning: unused)|FAILED|panicked"; then
      ok "tests  features=${combo:-<default>}  so=$profile"
    else
      bad "tests  features=${combo:-<default>}  so=$profile"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Result"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAILED"

#!/usr/bin/env bash
# Full C-vs-Rust differential verification matrix.
#
#   ./run_verification.sh            # debug + release, every feature combo
#   HARVEST_FUZZ_ITERS=5000000 ./run_verification.sh
#
# Phases:
#   A  build both shared objects, dump the symbol tables
#   B  valid-path differential tests   (CONFIGS.md rows)
#   C  error-path differential tests   (ERRORS.md rows)
#   D  symbol parity + every feature combination
set -u -o pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
LOG_DIR="$CRATE_DIR/target/verification"
mkdir -p "$LOG_DIR"

FAILED=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# --------------------------------------------------------------------------
# Phase A — build the C ground truth
# --------------------------------------------------------------------------
note "Phase A: building the C shared library"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOG_DIR/cmake.log" 2>&1 &&
  cmake --build . >>"$LOG_DIR/cmake.log" 2>&1
) && ok "C .so built" || { bad "C build (see $LOG_DIR/cmake.log)"; exit 1; }

C_SO="$(find "$ROOT/c_src/build" -maxdepth 2 -name '*.so' | head -n1)"
echo "   C  .so: $C_SO"

# --------------------------------------------------------------------------
# Feature combinations (there are only two: the empty set and test_internals)
# --------------------------------------------------------------------------
# Derived from Cargo.toml so a new feature cannot silently escape the matrix.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,"");split($0,a,"=");if(a[1]!="default")print a[1]}' \
    "$CRATE_DIR/Cargo.toml"
)
echo "   declared non-default features: ${FEATURES[*]:-<none>}"

COMBOS=("--no-default-features")
for f in "${FEATURES[@]:-}"; do
  [ -n "$f" ] && COMBOS+=("--no-default-features --features $f")
done
COMBOS+=("--all-features")

for PROFILE in "" "--release"; do
  for COMBO in "${COMBOS[@]}"; do
    TAG="$(echo "profile${PROFILE:-_debug}_${COMBO}" | tr -cs '[:alnum:]' '_')"
    note "Phases B-D: cargo test $PROFILE $COMBO"

    # The cdylib must exist for the profile under test BEFORE the tests run:
    # `cargo test` does not always refresh the cdylib artifact.
    # shellcheck disable=SC2086
    if ! cargo build --offline $PROFILE $COMBO >"$LOG_DIR/build_$TAG.log" 2>&1; then
      bad "cargo build $PROFILE $COMBO (see $LOG_DIR/build_$TAG.log)"
      continue
    fi

    RUST_SO="$CRATE_DIR/target/$([ -n "$PROFILE" ] && echo release || echo debug)/libmemchra2_lib.so"
    echo "   Rust .so: $RUST_SO"
    diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort) \
         >"$LOG_DIR/symdiff_$TAG.txt"
    if grep -q '^<' "$LOG_DIR/symdiff_$TAG.txt"; then
      bad "symbols missing from the Rust .so:"; grep '^<' "$LOG_DIR/symdiff_$TAG.txt"
    else
      ok "symbol parity (0 C symbols missing from Rust)"
    fi

    # shellcheck disable=SC2086
    if timeout 600 cargo test --offline $PROFILE $COMBO -- --test-threads="$(nproc)" \
         >"$LOG_DIR/test_$TAG.log" 2>&1; then
      ok "$(grep -c '^test .* ok$' "$LOG_DIR/test_$TAG.log") tests passed"
    else
      bad "cargo test $PROFILE $COMBO (see $LOG_DIR/test_$TAG.log)"
      tail -n 40 "$LOG_DIR/test_$TAG.log"
    fi
  done
done

note "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL PHASES PASSED\033[0m — logs in %s\n' "$LOG_DIR"
else
  printf '\033[31mVERIFICATION FAILED\033[0m — logs in %s\n' "$LOG_DIR"
fi
exit "$FAILED"

#!/usr/bin/env bash
# Full C-vs-Rust differential verification driver.
#
# Sequences the steps that MUST happen in order, because `cargo test` does not
# emit cdylib artifacts on its own:
#
#   1. build the C shared object
#   2. build the Rust cdylib for a profile
#   3. run the differential test suite for that same profile
#
# ...for every profile and every feature combination, plus the symbol diff.
#
# Usage: scripts/verify_all.sh [--slow]
#   --slow   also run the #[ignore]d long-running rows
set -uo pipefail

cd "$(dirname "$0")/.."
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"
C_DIR="$ROOT/c_src"

SLOW=0
[[ "${1:-}" == "--slow" ]] && SLOW=1

FAILED=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAILED=1; }
ok()   { printf '\033[32mok: %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
step "Building C shared library"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/tmp/cbuild.log 2>&1 \
  || { tail -20 /tmp/cbuild.log; fail "C build"; exit 1; }

C_SO="$(find "$C_DIR/build" -maxdepth 1 -name '*.so' | head -1)"
[[ -n "$C_SO" ]] || { fail "no C .so produced"; exit 1; }
ok "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating feature combinations"
# Every feature name declared under [features] (excluding `default`).
FEATURES=$(awk '
  /^\[features\]/       { inf=1; next }
  /^\[/                 { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

# Build the list of combos to test. With no declared features the only
# meaningful configurations are the default build and --no-default-features.
COMBOS=()
COMBOS+=("__default__")
COMBOS+=("__none__")
if [[ -n "$FEATURES" ]]; then
  # Full powerset of declared features.
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((k=0; k<n; k++)); do
      if (( mask & (1<<k) )); then combo+="${feats[k]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi
printf 'declared features: %s\n' "${FEATURES:-<none>}"
printf 'combinations to test: %s\n' "${#COMBOS[@]}"
printf '  - %s\n' "${COMBOS[@]}"

flags_for() {
  case "$1" in
    __default__) echo "" ;;
    __none__)    echo "--no-default-features" ;;
    "")          echo "--no-default-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# ---------------------------------------------------------------------------
# 3. cargo check every combination (fast compile-error gate)
# ---------------------------------------------------------------------------
step "cargo check, all feature combinations"
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  if timeout 300 cargo check --all-targets $(flags_for "$combo") >/tmp/check.log 2>&1; then
    ok "cargo check [$combo]"
  else
    tail -20 /tmp/check.log; fail "cargo check [$combo]"
  fi
done

# ---------------------------------------------------------------------------
# 4. Build + test each (profile x feature combination)
# ---------------------------------------------------------------------------
for profile in dev release; do
  if [[ $profile == release ]]; then PF="--release"; PDIR=release; else PF=""; PDIR=debug; fi
  for combo in "${COMBOS[@]}"; do
    FF=$(flags_for "$combo")
    step "profile=$profile features=[$combo]"

    # The cdylib must be built explicitly: `cargo test` never emits it.
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build --lib $PF $FF >/tmp/build.log 2>&1; then
      tail -20 /tmp/build.log; fail "cargo build --lib [$profile/$combo]"; continue
    fi
    RUST_SO="$CRATE_DIR/target/$PDIR/libflip_horizontal_lib.so"
    [[ -f "$RUST_SO" ]] || { fail "missing $RUST_SO"; continue; }

    # Symbol parity for this exact artifact.
    diffout=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [[ -n "$diffout" ]]; then
      printf 'symbols in C .so missing from Rust .so:\n%s\n' "$diffout"
      fail "symbol parity [$profile/$combo]"
    else
      ok "symbol parity [$profile/$combo] (0 missing)"
    fi

    IGN=""
    [[ $SLOW == 1 ]] && IGN="--include-ignored"
    LOG="/tmp/difftest-$profile-${combo//[^A-Za-z0-9]/_}.log"
    # shellcheck disable=SC2086
    if timeout 600 cargo test $PF $FF -- $IGN >"$LOG" 2>&1; then
      grep -E '^test result' "$LOG" | sed 's/^/  /'
      ok "differential suite [$profile/$combo]"
    else
      echo "--- full log: $LOG ---"
      tail -40 "$LOG"
      fail "differential suite [$profile/$combo]"
    fi
  done
done

# ---------------------------------------------------------------------------
# 5. Final symbol report
# ---------------------------------------------------------------------------
step "Final symbol report"
echo "C .so defined:"
nm -D --defined-only "$C_SO" | sed 's/^/  /'
echo "Rust .so (release) defined:"
nm -D --defined-only "$CRATE_DIR/target/release/libflip_horizontal_lib.so" | sed 's/^/  /'
echo "Rust .so undefined, non-libc (must be empty):"
nm -D --undefined-only "$CRATE_DIR/target/release/libflip_horizontal_lib.so" \
  | awk '{print $NF}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^__cxa_|^statx$|^gettid$' \
  | sed 's/^/  /'

step "RESULT"
if [[ $FAILED == 0 ]]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit $FAILED

#!/usr/bin/env bash
# Full verification driver (Phases A-D).
#
# Enumerates every Cargo feature combination from Cargo.toml, and for each one
# and each build profile:
#   1. builds the C .so and the Rust cdylib,
#   2. diffs `nm -D` symbol tables (must be empty),
#   3. runs the whole differential test suite.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
WORKDIR="$(cd .. && pwd)"
RESULTS=()
FAILED=0

note()  { printf '\n\033[1;36m== %s\033[0m\n' "$*"; }
pass()  { printf '\033[1;32mPASS\033[0m %s\n' "$*"; RESULTS+=("PASS  $*"); }
fail()  { printf '\033[1;31mFAIL\033[0m %s\n' "$*"; RESULTS+=("FAIL  $*"); FAILED=1; }

# --------------------------------------------------------------------------
# 1. Build the C shared library
# --------------------------------------------------------------------------
note "Building C shared library"
mkdir -p "$WORKDIR/c_src/build" || exit 1
(
  cd "$WORKDIR/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { fail "C build"; exit 1; }

C_SO="$(find "$WORKDIR/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C .so: $C_SO"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations (powerset of [features] in Cargo.toml)
# --------------------------------------------------------------------------
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

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "No [features] declared -> single configuration."
  COMBOS+=("default:")
else
  echo "Features found: ${FEATURES[*]}"
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
  COMBOS+=("all:--all-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("nodefault+${joined}:--no-default-features --features ${joined}")
  done
fi

# --------------------------------------------------------------------------
# 3. For each combination x profile: build, diff symbols, run tests
# --------------------------------------------------------------------------
NM_C="$(mktemp)"; NM_R="$(mktemp)"
trap 'rm -f "$NM_C" "$NM_R"' EXIT
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$NM_C"

for profile in debug release; do
  PROFILE_FLAG=""; OUT="debug"
  [ "$profile" = release ] && { PROFILE_FLAG="--release"; OUT="release"; }

  for entry in "${COMBOS[@]}"; do
    label="${entry%%:*}"
    flags="${entry#*:}"
    tag="$profile/$label"

    note "$tag  (cargo flags: ${flags:-<none>})"

    # cargo check
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $PROFILE_FLAG $flags --all-targets >/dev/null 2>&1; then
      fail "$tag cargo check"
      continue
    fi

    # build the cdylib
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $PROFILE_FLAG $flags >/dev/null 2>&1; then
      fail "$tag cargo build"
      continue
    fi

    R_SO="$CRATE_DIR/target/$OUT/libcheckshift_lib.so"
    if [ ! -f "$R_SO" ]; then
      fail "$tag missing $R_SO"
      continue
    fi

    # symbol parity
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$NM_R"
    missing="$(comm -23 "$NM_C" "$NM_R")"
    if [ -n "$missing" ]; then
      fail "$tag symbol parity; missing from Rust: $(echo "$missing" | tr '\n' ' ')"
    else
      pass "$tag symbol parity ($(wc -l < "$NM_C") symbols)"
    fi

    # undefined non-libc symbols
    undef="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
      | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_' || true)"
    if [ -n "$undef" ]; then
      fail "$tag undefined non-libc symbols: $(echo "$undef" | tr '\n' ' ')"
    else
      pass "$tag no undefined non-libc symbols"
    fi

    # the differential test suite
    # shellcheck disable=SC2086
    if timeout 600 cargo test $PROFILE_FLAG $flags 2>&1 | tail -40 > "$CRATE_DIR/.last_test.log"; then
      total=$(grep -c 'test result: ok' "$CRATE_DIR/.last_test.log" || true)
      pass "$tag differential tests"
    else
      cat "$CRATE_DIR/.last_test.log"
      fail "$tag differential tests"
    fi
  done
done

# --------------------------------------------------------------------------
# 4. Summary
# --------------------------------------------------------------------------
note "SUMMARY"
for line in "${RESULTS[@]}"; do echo "  $line"; done
if [ "$FAILED" -eq 0 ]; then
  printf '\n\033[1;32mALL CHECKS PASSED\033[0m\n'
else
  printf '\n\033[1;31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAILED"

#!/usr/bin/env bash
# Phase D driver: enumerate every build configuration and verify all of them.
#
#  * enumerates the [features] powerset from Cargo.toml (mechanically),
#  * cargo check + cargo test for each combination,
#  * in BOTH the dev and release profiles (release sets panic = "abort", which is
#    a genuine build-time configuration difference),
#  * and diffs `nm -D` symbols for every resulting Rust .so against the C .so.
set -u
cd "$(dirname "$0")"

CARGO="cargo"
LOGDIR="${TMPDIR:-/tmp}"
mkdir -p "$LOGDIR"
OFFLINE="--offline"
C_SO="c_src/build/libpow.so"
fail=0

echo "=============================================================="
echo " Phase A: enumerate build configurations"
echo "=============================================================="

# Extract feature names from the [features] table, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "Cargo features declared: $n ${FEATURES[*]:-(none)}"

# Build the powerset of feature combinations. With no features declared the
# powerset is the single empty combination, i.e. --no-default-features alone.
COMBOS=()
if [ "$n" -eq 0 ]; then
  COMBOS=("")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
# The declared default feature set is also a real configuration.
COMBOS+=("__DEFAULT__")

echo "Configurations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do
  if [ "$c" = "__DEFAULT__" ]; then
    echo "  - (default features)"
  elif [ -z "$c" ]; then
    echo "  - --no-default-features (empty feature set)"
  else
    echo "  - --no-default-features --features $c"
  fi
done

if [ ! -f "$C_SO" ]; then
  echo "FATAL: $C_SO missing. Build the C library first."
  exit 1
fi

C_SYMS=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort)
echo
echo "C .so exported symbols:"
echo "$C_SYMS" | sed 's/^/  /'

for profile in dev release; do
  if [ "$profile" = "release" ]; then
    PROFILE_FLAG="--release"
    TARGET_DIR="target/release"
  else
    PROFILE_FLAG=""
    TARGET_DIR="target/debug"
  fi

  for combo in "${COMBOS[@]}"; do
    if [ "$combo" = "__DEFAULT__" ]; then
      FEATURE_FLAGS=""
      label="default-features"
    elif [ -z "$combo" ]; then
      FEATURE_FLAGS="--no-default-features"
      label="no-default-features"
    else
      FEATURE_FLAGS="--no-default-features --features $combo"
      label="no-default-features+$combo"
    fi

    echo
    echo "=============================================================="
    echo " profile=$profile  features=$label"
    echo "=============================================================="

    echo "--- cargo check ---"
    if ! timeout 600 $CARGO check $OFFLINE $PROFILE_FLAG $FEATURE_FLAGS --all-targets \
        >"$LOGDIR/pow_check.log" 2>&1; then
      echo "FAIL: cargo check ($profile/$label)"
      tail -30 "$LOGDIR/pow_check.log"
      fail=1
      continue
    fi
    if grep -qE '^(warning|error)' "$LOGDIR/pow_check.log"; then
      echo "  (diagnostics)"; grep -E '^(warning|error)' "$LOGDIR/pow_check.log" | head -10
    else
      echo "  clean"
    fi

    echo "--- cargo build (cdylib) ---"
    if ! timeout 600 $CARGO build $OFFLINE $PROFILE_FLAG $FEATURE_FLAGS \
        >"$LOGDIR/pow_build.log" 2>&1; then
      echo "FAIL: cargo build ($profile/$label)"
      tail -30 "$LOGDIR/pow_build.log"
      fail=1
      continue
    fi

    echo "--- nm -D symbol parity vs the C .so ---"
    RUST_SO="$TARGET_DIR/libpow.so"
    if [ ! -f "$RUST_SO" ]; then
      echo "FAIL: $RUST_SO not produced"
      fail=1
      continue
    fi
    RUST_SYMS=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort)
    MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$RUST_SYMS"))
    if [ -n "$MISSING" ]; then
      echo "FAIL: symbols exported by C but MISSING from Rust:"
      echo "$MISSING" | sed 's/^/    /'
      fail=1
    else
      echo "  OK: 0 missing symbols"
    fi
    # Both must bind the same versioned libm pow, or errno semantics differ.
    for want in 'pow@GLIBC' '__errno_location' 'fprintf' 'stderr'; do
      cs=$(nm -D --undefined-only "$C_SO"  | grep -c "$want")
      rs=$(nm -D --undefined-only "$RUST_SO" | grep -c "$want")
      if [ "$cs" -gt 0 ] && [ "$rs" -eq 0 ]; then
        echo "FAIL: Rust .so does not import '$want' (C does)"
        fail=1
      fi
    done
    cver=$(nm -D --undefined-only "$C_SO"  | grep -oE 'pow@GLIBC_[0-9.]+' | head -1)
    rver=$(nm -D --undefined-only "$RUST_SO" | grep -oE 'pow@GLIBC_[0-9.]+' | head -1)
    if [ "$cver" != "$rver" ]; then
      echo "FAIL: libm pow version tag differs: C='$cver' Rust='$rver'"
      fail=1
    else
      echo "  OK: both bind $cver"
    fi

    echo "--- cargo test (Phases B + C) ---"
    if timeout 600 $CARGO test $OFFLINE $PROFILE_FLAG $FEATURE_FLAGS \
        >"$LOGDIR/pow_test.log" 2>&1; then
      grep -E 'test result' "$LOGDIR/pow_test.log" | sed 's/^/  /'
    else
      echo "FAIL: cargo test ($profile/$label)"
      grep -E 'panicked|test result|^test .* FAILED|failures:' "$LOGDIR/pow_test.log" | head -40
      fail=1
    fi
  done
done

echo
echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo " ALL CONFIGURATIONS PASSED"
else
  echo " SOME CONFIGURATIONS FAILED"
fi
echo "=============================================================="
exit $fail

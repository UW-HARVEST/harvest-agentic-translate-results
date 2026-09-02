#!/usr/bin/env bash
# Phase D — feature-combination matrix.
#
# Enumerates the powerset of the features declared in Cargo.toml and runs
# `cargo check` + the full differential suite for every combination, so that no
# configuration is verified by hand. `translation` currently declares no
# [features] table, in which case the matrix is {default, --no-default-features}.
set -uo pipefail

cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"

# --- feature discovery (mechanical, from Cargo.toml) -------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=()
COMBOS+=("--all-features")                 # == default when no features exist
COMBOS+=("--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    if (( ${#sel[@]} )); then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

# --- the C library is the ground truth; build it once ------------------------
if ! ls "$ROOT"/c_src/build/*.so >/dev/null 2>&1; then
  echo "==> building the C shared library"
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || exit 1
fi
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C  .so: $C_SO"

fail=0
for combo in "${COMBOS[@]}"; do
  # The Rust .so is also verified in BOTH cargo profiles: `release` is the
  # shipped artifact, `dev` has overflow checks enabled, and the two can diverge.
  for profile in release dev; do
    if [[ $profile == release ]]; then
      flag=--release; dir=release
    else
      flag=""; dir=debug
    fi
    label="[$profile] cargo ${combo}"
    echo
    echo "===================================================================="
    echo "==> $label"
    echo "===================================================================="

    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $flag $combo --all-targets >/tmp/fc-check.log 2>&1; then
      echo "FAIL: cargo check ($label)"; tail -30 /tmp/fc-check.log; fail=1; continue
    fi
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $flag $combo >/tmp/fc-build.log 2>&1; then
      echo "FAIL: cargo build ($label)"; tail -30 /tmp/fc-build.log; fail=1; continue
    fi

    R_SO="$PWD/target/$dir/libcheckshift_lib.so"
    if [[ ! -f $R_SO ]]; then
      echo "FAIL: $R_SO was not produced"; fail=1; continue
    fi

    # Symbol parity for THIS build of the Rust .so.
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > /tmp/fc-c.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > /tmp/fc-r.txt
    missing="$(comm -23 /tmp/fc-c.txt /tmp/fc-r.txt)"
    if [[ -n $missing ]]; then
      echo "FAIL: Rust .so is missing symbols ($label):"; echo "$missing"; fail=1; continue
    fi
    echo "symbol parity: $(wc -l < /tmp/fc-c.txt) C symbols, 0 missing from Rust"

    # Phases B + C against exactly this .so. The test binaries are always built
    # in release (they are just harnesses); RUST_SO selects the library to load.
    # shellcheck disable=SC2086
    if ! C_SO="$C_SO" RUST_SO="$R_SO" timeout 600 cargo test --release $combo \
           >/tmp/fc-test.log 2>&1; then
      echo "FAIL: differential suite ($label)"; tail -60 /tmp/fc-test.log; fail=1; continue
    fi
    grep -E '^(test result|result):' /tmp/fc-test.log | sed 's/^/    /'
  done
done

echo
if (( fail )); then
  echo "FEATURE MATRIX: FAILED"
  exit 1
fi
echo "FEATURE MATRIX: all ${#COMBOS[@]} feature combination(s) x 2 cargo profile(s) passed"

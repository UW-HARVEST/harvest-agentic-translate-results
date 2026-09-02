#!/usr/bin/env bash
# Phase D — run the whole differential suite under every feature combination and
# both Rust build profiles. Feature names are extracted from Cargo.toml rather
# than hard-coded, so a future `[features]` table is picked up automatically.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

ITERS="${DIFF_ITERS:-200}"
export DIFF_ITERS="$ITERS"

# --- enumerate features ----------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' Cargo.toml | grep -vx 'default' | sort -u
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Power set of FEATURES, expressed as --features arguments. With no features the
# only combinations are the default build and --no-default-features.
COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS+=("")                          # default
  COMBOS+=("--no-default-features")
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATURES[i]}")
    done
    if (( ${#sel[@]} == 0 )); then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("")                          # plus the plain default build
fi

fail=0

for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    label="profile=$profile combo='${combo:-<default>}'"
    echo
    echo "=============================================================="
    echo ">>> cargo check   $label"
    echo "=============================================================="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo check $combo --all-targets 2>&1 | tail -n 5; then
      echo "FAIL: cargo check ($label)"; fail=1; continue
    fi

    echo ">>> building the cdylib under test ($profile)"
    build_flag=""
    [[ "$profile" == "release" ]] && build_flag="--release"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $build_flag $combo 2>&1 | tail -n 3; then
      echo "FAIL: cargo build ($label)"; fail=1; continue
    fi

    so="$here/target/$profile/libunderhanded_c_nuke_lib.so"
    if [[ ! -f "$so" ]]; then
      echo "FAIL: $so not produced ($label)"; fail=1; continue
    fi

    echo ">>> ./check_symbols.sh against $profile"
    if ! RUST_SO="$so" "$here/check_symbols.sh" | tail -n 3; then
      echo "FAIL: symbol parity ($label)"; fail=1; continue
    fi

    echo ">>> cargo test    $label  (RUST_SO=$profile, DIFF_ITERS=$ITERS)"
    # Tests always run in the dev profile (libtest needs unwinding); RUST_SO
    # selects which build of the cdylib is loaded through libloading.
    # shellcheck disable=SC2086
    if ! RUST_SO="$so" timeout 600 cargo test $combo -- --test-threads=4 2>&1 \
        | grep -E '^(test |running |error|failures|test result)' ; then
      echo "FAIL: cargo test ($label)"; fail=1; continue
    fi
    # grep swallows the exit status, so re-check explicitly.
    # shellcheck disable=SC2086
    RUST_SO="$so" timeout 600 cargo test $combo -- --test-threads=4 >/dev/null 2>&1 \
      || { echo "FAIL: cargo test ($label)"; fail=1; }
  done
done

echo
if (( fail )); then
  echo "OVERALL: FAIL"
  exit 1
fi
echo "OVERALL: PASS — all feature combinations x both profiles"

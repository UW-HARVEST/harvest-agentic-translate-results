#!/usr/bin/env bash
# Full verification sweep: every feature combination x every test target.
#
# The crate declares no [features], so the complete set of valid combinations is
# the empty set -- exercised both as the default build and explicitly with
# --no-default-features. The loop is written generically so that adding features
# later automatically extends the sweep.
set -euo pipefail
cd "$(dirname "$0")/.."

# ---- enumerate feature combinations from Cargo.toml -------------------------
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

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("")            # no features exist -> one configuration
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done

# ---- the C reference build, exactly as documented in SYMBOLS.md -------------
echo
echo "=== building the C reference (cmake) ==="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)

fail=0
for combo in "${COMBOS[@]}"; do
  for defaults in "--no-default-features" ""; do
    label="features='${combo:-<none>}' ${defaults:-<defaults>}"
    args=()
    [ -n "$defaults" ] && args+=("$defaults")
    [ -n "$combo" ] && args+=(--features "$combo")

    echo
    echo "############ cargo check  [$label] ############"
    if ! cargo check --offline --all-targets "${args[@]}"; then fail=1; continue; fi

    echo "############ cargo build  [$label] ############"
    # MUST precede cargo test: the integration tests dlopen the cdylib instead
    # of linking it, so cargo has no dependency edge that would rebuild it.
    if ! cargo build --offline --all-targets "${args[@]}"; then fail=1; continue; fi

    echo "############ symbol diff  [$label] ############"
    if ! ./scripts/symdiff.sh; then fail=1; fi

    echo "############ cargo test   [$label] ############"
    if ! cargo test --offline "${args[@]}" -- --test-threads=8; then fail=1; fi

    # The release profile turns debug assertions off and optimisations on, and
    # sets panic=abort for the cdylib, so re-run everything there too.
    echo "############ release build+test [$label] ############"
    if ! cargo build --offline --release --all-targets "${args[@]}"; then fail=1; continue; fi
    if ! RUST_SO=target/release/libfma_array.so ./scripts/symdiff.sh; then fail=1; fi
    if ! cargo test --offline --release "${args[@]}" -- --test-threads=8; then fail=1; fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"

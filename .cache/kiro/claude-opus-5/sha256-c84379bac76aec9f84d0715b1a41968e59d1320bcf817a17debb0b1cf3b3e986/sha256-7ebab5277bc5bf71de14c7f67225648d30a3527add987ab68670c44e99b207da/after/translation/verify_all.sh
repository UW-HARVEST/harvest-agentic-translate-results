#!/usr/bin/env bash
# Build the C reference library and run the differential test suite for every
# valid Cargo feature combination.
#
# `translation/Cargo.toml` declares no `[features]` and `c_src/CMakeLists.txt`
# exposes no options, so the enumeration below resolves to the single default
# configuration. The loop is written generically so that adding features later
# requires no changes here.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SRC="$ROOT/c_src"
RUST="$ROOT/translation"
TIMEOUT=600

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$RUST/Cargo.toml"
)

COMBOS=("")   # the empty string means --no-default-features with nothing extra
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && combo+=("${FEATURES[b]}")
    done
    COMBOS+=("$(
      IFS=,
      echo "${combo[*]}"
    )")
  done
fi

echo "=== feature combinations to verify: ${#COMBOS[@]} ==="
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library
# ---------------------------------------------------------------------------
echo
echo "=== building C reference library ==="
mkdir -p "$C_SRC/build"
(
  cd "$C_SRC/build" &&
    timeout $TIMEOUT cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    timeout $TIMEOUT cmake --build . >/dev/null
) || {
  echo "C build FAILED"
  exit 1
}
C_SO="$C_SRC/build/libdriver.so"
echo "built $C_SO"

# ---------------------------------------------------------------------------
# 3. cargo check, build and test each combination
# ---------------------------------------------------------------------------
status=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  if [[ -n "$combo" ]]; then
    FEATFLAGS=(--no-default-features --features "$combo")
  else
    FEATFLAGS=(--no-default-features)
  fi

  echo
  echo "############ combination: $label ############"

  for step in check build test; do
    case $step in
    check) cmd=(cargo check "${FEATFLAGS[@]}" --all-targets) ;;
    build) cmd=(cargo build --release "${FEATFLAGS[@]}") ;;
    test) cmd=(cargo test "${FEATFLAGS[@]}") ;;
    esac
    echo "--- cargo $step ---"
    if [[ $step == test ]]; then
      # Point the tests at the release cdylib just built for this combination.
      (cd "$RUST" && DRIVER_C_SO="$C_SO" \
        DRIVER_RUST_SO="$RUST/target/release/libdriver.so" \
        timeout $TIMEOUT "${cmd[@]}")
    else
      (cd "$RUST" && timeout $TIMEOUT "${cmd[@]}")
    fi
    rc=$?
    if ((rc != 0)); then
      echo "FAILED: cargo $step for '$label' (exit $rc)"
      status=1
      continue 2
    fi
  done

  # -------------------------------------------------------------------------
  # 4. Symbol export parity for this combination
  # -------------------------------------------------------------------------
  echo "--- nm -D symbol parity ---"
  csyms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRGSi]$/ {print $3}' | sort -u)
  rsyms=$(nm -D --defined-only "$RUST/target/release/libdriver.so" |
    awk '$2 ~ /^[TDBRGSi]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  if [[ -n "$missing" ]]; then
    echo "MISSING from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    status=1
  else
    echo "all C exports present in Rust .so: $(echo "$csyms" | tr '\n' ' ')"
  fi
done

echo
if ((status == 0)); then
  echo "=== ALL COMBINATIONS PASSED ==="
else
  echo "=== FAILURES PRESENT ==="
fi
exit $status

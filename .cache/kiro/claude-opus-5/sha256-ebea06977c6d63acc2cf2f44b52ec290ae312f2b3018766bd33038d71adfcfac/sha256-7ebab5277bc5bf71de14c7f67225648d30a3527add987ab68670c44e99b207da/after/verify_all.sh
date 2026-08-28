#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for EVERY build-time
# configuration.
#
#   * translation/Cargo.toml has no [features] section  -> exactly one Rust config
#   * c_src/CMakeLists.txt has no options/definitions    -> exactly one C config
#
# The script still enumerates programmatically so it keeps working if either
# side gains configuration knobs later.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
FAILED=0

note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' "$CRATE/Cargo.toml"
)

note "Rust features discovered: ${#FEATURES[@]} (${FEATURES[*]-none})"
note "CMake options discovered:"
grep -nE '^[[:space:]]*(option|add_definitions|target_compile_definitions)' "$CSRC/CMakeLists.txt" || echo "  (none)"

# Build the combination list: always include the empty combo (no features).
COMBOS=("")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
note "Feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Build the C shared library
# ---------------------------------------------------------------------------
note "Building C shared library"
mkdir -p "$CSRC/build"
(
  cd "$CSRC/build" &&
    timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 &&
    timeout 600 cmake --build . >>/tmp/cmake.log 2>&1
) || {
  echo "C build FAILED (see /tmp/cmake.log)"
  tail -20 /tmp/cmake.log
  exit 1
}
C_SO="$(find "$CSRC/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 3. cargo check / test / symbol diff per combination, in debug and release
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    fargs=(--no-default-features)
    label="<no features>"
  else
    fargs=(--no-default-features --features "$combo")
    label="$combo"
  fi

  note "cargo check  [$label]"
  if ! (cd "$CRATE" && timeout 600 cargo check "${fargs[@]}" 2>&1 | tail -5); then
    echo "CHECK FAILED for $label"
    FAILED=1
    continue
  fi

  for profile in debug release; do
    pargs=()
    [ "$profile" = release ] && pargs=(--release)

    note "cargo test [$label] ($profile)"
    if ! (cd "$CRATE" && timeout 600 cargo test "${fargs[@]}" "${pargs[@]}" 2>&1 | tail -20); then
      echo "TEST FAILED for $label ($profile)"
      FAILED=1
    fi

    note "symbol diff [$label] ($profile)"
    (cd "$CRATE" && timeout 600 cargo build --lib "${fargs[@]}" "${pargs[@]}" >/dev/null 2>&1)
    RUST_SO="$CRATE/target/$profile/libpremultiply_lib.so"
    if [ ! -f "$RUST_SO" ]; then
      RUST_SO="$CRATE/target/ffi-cdylib/$profile/libpremultiply_lib.so"
    fi
    if [ ! -f "$RUST_SO" ]; then
      echo "Rust .so not found for $label ($profile)"
      FAILED=1
      continue
    fi

    # Every dynamic symbol *defined* by the C .so must also be defined by the
    # Rust .so. Toolchain-internal symbols are excluded on both sides.
    filter='^(_init|_fini|_edata|_end|__bss_start|__.*|_ITM_.*|_Unwind_.*|rust_.*|.*\.llvm\..*)$'
    c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | grep -Ev "$filter" | sort -u)
    r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | grep -Ev "$filter" | sort -u)

    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      echo "MISSING EXPORTS in Rust .so ($label/$profile):"
      echo "$missing" | sed 's/^/  /'
      FAILED=1
    else
      echo "OK: Rust .so exports all $(echo "$c_syms" | grep -c .) C symbol(s): $(echo $c_syms)"
    fi
  done
done

note "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES DETECTED"
fi
exit "$FAILED"

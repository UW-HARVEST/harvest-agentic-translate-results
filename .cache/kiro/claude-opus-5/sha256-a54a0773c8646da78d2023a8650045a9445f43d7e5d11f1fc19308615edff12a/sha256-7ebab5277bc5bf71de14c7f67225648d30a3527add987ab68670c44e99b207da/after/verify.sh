#!/usr/bin/env bash
# Full verification matrix: build the C reference, then for every valid Cargo
# feature combination cargo-check, build the cdylib and run the differential
# tests against the C .so.
#
# Usage: ./verify.sh          (from the repository root or anywhere)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_src="$root/c_src"
rust="$root/translation"
timeout_s=600

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations declared in translation/Cargo.toml.
#    The powerset of all declared features is used; with no [features] table
#    this yields a single (empty) combination, i.e. the default configuration.
# ---------------------------------------------------------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "")
      if ($0 != "default") print
    }
  ' "$rust/Cargo.toml"
)

combos=("")
for f in "${features[@]}"; do
  existing=("${combos[@]}")
  for c in "${existing[@]}"; do
    combos+=("${c:+$c,}$f")
  done
done

step "Feature combinations (${#combos[@]})"
for c in "${combos[@]}"; do printf '  - %s\n' "${c:-<none / default>}"; done

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library.
# ---------------------------------------------------------------------------
step "Building C reference"
mkdir -p "$c_src/build"
(
  cd "$c_src/build"
  timeout $timeout_s cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1
  timeout $timeout_s cmake --build . >>/tmp/cmake.log 2>&1
) || { tail -30 /tmp/cmake.log; exit 1; }
c_so="$(find "$c_src/build" -maxdepth 1 -name 'lib*.so' | sort | tail -1)"
echo "C library: $c_so"

# ---------------------------------------------------------------------------
# 3-4. cargo check, build and test every combination.
# ---------------------------------------------------------------------------
fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<default>}"
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  step "check  $label"
  if ! (cd "$rust" && timeout $timeout_s cargo check --all-targets "${args[@]}" 2>&1 | tail -20); then
    echo "CHECK FAILED: $label"; fail=1; continue
  fi

  for profile in "" --release; do
    pname="${profile:---dev}"
    step "build+test $label ($pname)"
    # The cdylib must be built explicitly: cargo test does not build a
    # cdylib-only lib target, so the tests would otherwise load a stale .so.
    if ! (cd "$rust" && timeout $timeout_s cargo build $profile "${args[@]}" 2>&1 | tail -10); then
      echo "BUILD FAILED: $label $pname"; fail=1; continue
    fi

    rust_so="$rust/target/$([[ -n $profile ]] && echo release || echo debug)/libhsl_to_rgb_lib.so"

    # Step 8: symbol parity, also asserted from inside tests/symbols.rs.
    missing="$(comm -23 \
      <(nm -D --defined-only "$c_so"   | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u) \
      <(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u))"
    if [[ -n "$missing" ]]; then
      echo "note: symbols present in C but not Rust (toolchain symbols are filtered by tests/symbols.rs):"
      echo "$missing" | sed 's/^/    /'
    fi

    if ! (cd "$rust" && timeout $timeout_s cargo test $profile "${args[@]}" 2>&1 | tail -25); then
      echo "TEST FAILED: $label $pname"; fail=1
    fi
  done
done

step "Result"
if (( fail )); then
  echo "FAILURES — see output above"
  exit 1
fi
echo "All ${#combos[@]} feature combination(s) passed in both dev and release profiles."

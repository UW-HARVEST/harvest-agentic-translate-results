#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build-time
# configuration: each Cargo feature combination, in both the dev and release
# profiles (they hand LLVM different optimisation levels, which is exactly what
# perturbed floating-point NaN payloads during this port).
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
c_dir="$root/c_src"
rust_dir="$root/translation"
log_dir="${TMPDIR:-/tmp}/cb-verify"
mkdir -p "$log_dir"
status=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; status=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination.
# ---------------------------------------------------------------------------
step 'Feature combinations'
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "")
      if ($0 != "default") print
    }
  ' "$rust_dir/Cargo.toml"
)

# The powerset of the declared features; with none declared this is just the
# empty combination, i.e. the single default configuration.
combos=("")
for feature in "${features[@]}"; do
  for existing in "${combos[@]}"; do
    combos+=("${existing:+$existing,}$feature")
  done
done

printf 'declared features: %s\n' "${features[*]:-<none>}"
printf 'combinations to verify: %d\n' "${#combos[@]}"
for combo in "${combos[@]}"; do
  printf '  - %s\n' "${combo:-<no features>}"
done

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library.
# ---------------------------------------------------------------------------
step 'Build C reference'
(
  mkdir -p "$c_dir/build" &&
  cd "$c_dir/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
  cmake --build .
) >"$log_dir/cmake.log" 2>&1 || { fail 'C build'; tail -20 "$log_dir/cmake.log"; exit 1; }

c_so="$(find "$c_dir/build" -maxdepth 1 -name '*.so' -print -quit)"
printf 'C library: %s\n' "$c_so"

# ---------------------------------------------------------------------------
# 3. cargo check / test each combination in each profile.
# ---------------------------------------------------------------------------
for combo in "${combos[@]}"; do
  label="${combo:-default}"
  safe="${label//,/_}"

  step "cargo check --no-default-features --features '$combo'"
  if timeout 600 cargo check --manifest-path "$rust_dir/Cargo.toml" --all-targets \
      --no-default-features --features "$combo" >"$log_dir/check-$safe.log" 2>&1; then
    echo 'ok'
  else
    fail "cargo check ($label)"
    tail -30 "$log_dir/check-$safe.log"
    continue
  fi

  for profile in dev release; do
    flag=()
    [[ $profile == release ]] && flag=(--release)

    # `cargo test` builds the lib as an rlib for the unit-test harness but never
    # emits the `cdylib` artefact, so the integration tests would dlopen a stale
    # (or absent) .so. Build it explicitly first.
    step "cargo build --$profile --no-default-features --features '$combo'"
    if timeout 600 cargo build --manifest-path "$rust_dir/Cargo.toml" "${flag[@]}" \
        --no-default-features --features "$combo" \
        >"$log_dir/build-$safe-$profile.log" 2>&1; then
      echo 'ok'
    else
      fail "cargo build ($label, $profile)"
      tail -30 "$log_dir/build-$safe-$profile.log"
      continue
    fi

    step "cargo test --$profile --no-default-features --features '$combo'"
    if timeout 600 cargo test --manifest-path "$rust_dir/Cargo.toml" "${flag[@]}" \
        --no-default-features --features "$combo" \
        >"$log_dir/test-$safe-$profile.log" 2>&1; then
      grep -E '^test result' "$log_dir/test-$safe-$profile.log"
    else
      fail "cargo test ($label, $profile)"
      tail -40 "$log_dir/test-$safe-$profile.log"
    fi

    # -------------------------------------------------------------------
    # 4. Every dynamic symbol the C library exports must be exported by the
    #    Rust library under the identical name.
    # -------------------------------------------------------------------
    dir=release
    [[ $profile == dev ]] && dir=debug
    rust_so="$rust_dir/target/$dir/libcolourblind_lib.so"

    step "symbol parity ($label, $profile)"
    if [[ ! -f $rust_so ]]; then
      fail "missing $rust_so"
      continue
    fi

    # Ignore the linker-generated ELF housekeeping symbols that are an artefact
    # of the toolchain rather than part of either library's API.
    exclude='^(_init|_fini|_edata|_end|__bss_start|__.*_impl)$'
    dyn_syms() {
      nm -D --defined-only --format=posix "$1" 2>/dev/null |
        awk '{print $1}' | grep -Ev "$exclude" | sort -u
    }

    missing="$(comm -23 <(dyn_syms "$c_so") <(dyn_syms "$rust_so"))"
    if [[ -n $missing ]]; then
      fail "Rust library is missing C exports ($label, $profile):"
      printf '  %s\n' $missing
    else
      printf 'all C exports present (%s)\n' "$(dyn_syms "$c_so" | tr '\n' ' ')"
    fi
  done
done

step 'Summary'
if [[ $status -eq 0 ]]; then
  echo 'PASS: every feature combination matches the C reference.'
else
  echo 'FAIL: see messages above.'
fi
exit $status

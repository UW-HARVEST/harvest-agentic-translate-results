#!/usr/bin/env bash
# Differential-verification sweep: builds the C reference library, enumerates
# every valid Cargo feature combination, and runs `cargo check` + `cargo test`
# for each of them, in both the dev and release profiles.
#
# Usage: translation/verify_all.sh
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
timeout_s=600
fail=0

run() { # run <label> <cmd...>
  local label="$1"; shift
  printf '\n=== %s ===\n' "$label"
  if timeout "$timeout_s" "$@"; then
    printf -- '--- PASS: %s\n' "$label"
  else
    printf -- '--- FAIL: %s (exit %d)\n' "$label" "$?"
    fail=1
  fi
}

# --- 1. C reference library ------------------------------------------------
printf '=== building C reference library (default configuration) ===\n'
mkdir -p "$root/c_src/build"
(
  cd "$root/c_src/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { echo "C build failed"; exit 1; }
ls -l "$root/c_src/build/libdriver.so"

# --- 2. enumerate feature combinations -------------------------------------
# Read the [features] table of Cargo.toml; every subset of the optional features
# is a valid combination (there are no mutually exclusive backends here).
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /=/   { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' "$here/Cargo.toml"
)

combos=("")   # "" == --no-default-features with nothing enabled
n=${#features[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo+="${features[i]},"
      fi
    done
    combos+=("${combo%,}")
  done
fi

printf '\nfeatures declared in Cargo.toml: %d %s\n' "$n" "${features[*]:-(none)}"
printf 'feature combinations to verify: %d\n' "${#combos[@]}"

# --- 3. check + test every combination, in both profiles -------------------
cd "$here"
for combo in "${combos[@]}"; do
  label_combo="${combo:-<no features>}"
  for profile_flag in "" "--release"; do
    if [[ -z "$profile_flag" ]]; then
      label_profile="--dev"; profile_dir="debug"
    else
      label_profile="--release"; profile_dir="release"
    fi
    if [[ -z "$combo" ]]; then
      feat=(--no-default-features)
    else
      feat=(--no-default-features --features "$combo")
    fi
    run "cargo check [$label_combo] [$label_profile]" \
      cargo check --all-targets $profile_flag "${feat[@]}"
    # `cargo test` does not produce the cdylib for a cdylib-only crate, so the
    # artifact under test is built explicitly and pinned via DRIVER_RUST_SO.
    run "cargo build [$label_combo] [$label_profile]" \
      cargo build $profile_flag "${feat[@]}"
    DRIVER_RUST_SO="$here/target/$profile_dir/libdriver.so" \
      run "cargo test  [$label_combo] [$label_profile] (so=$profile_dir)" \
        cargo test $profile_flag "${feat[@]}"
  done
done

# Also verify the crate's declared default configuration.
for profile_flag in "" "--release"; do
  if [[ -z "$profile_flag" ]]; then profile_dir="debug"; else profile_dir="release"; fi
  run "cargo check [default features] [$profile_dir]" cargo check --all-targets $profile_flag
  run "cargo build [default features] [$profile_dir]" cargo build $profile_flag
  DRIVER_RUST_SO="$here/target/$profile_dir/libdriver.so" \
    run "cargo test  [default features] [$profile_dir]" cargo test $profile_flag
done

# --- 4. symbol parity ------------------------------------------------------
printf '\n=== dynamic symbol parity (nm -D --defined-only) ===\n'
for profile in debug release; do
  so="$here/target/$profile/libdriver.so"
  [[ -f "$so" ]] || continue
  c_syms=$(nm -D --defined-only "$root/c_src/build/libdriver.so" |
           awk '$3 !~ /^_/ {print $3}' | sort)
  r_syms=$(nm -D --defined-only "$so" | awk '$3 !~ /^_/ {print $3}' | sort)
  if [[ "$c_syms" == "$r_syms" ]]; then
    printf -- '--- PASS: %s symbol set identical to C:\n%s\n' "$profile" "$c_syms"
  else
    printf -- '--- FAIL: %s symbol set differs from C\n' "$profile"
    diff <(echo "$c_syms") <(echo "$r_syms")
    fail=1
  fi
done

printf '\n==================== %s ====================\n' \
  "$( ((fail == 0)) && echo 'ALL CONFIGURATIONS VERIFIED' || echo 'FAILURES PRESENT' )"
exit "$fail"

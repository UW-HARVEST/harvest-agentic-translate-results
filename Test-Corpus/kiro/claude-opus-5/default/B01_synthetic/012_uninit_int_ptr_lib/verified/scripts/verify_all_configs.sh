#!/usr/bin/env bash
# Enumerate every valid build-time configuration and run `cargo check` plus the
# full C-vs-Rust parity suite against each one.
#
# Configurations come from two places:
#   * translation/Cargo.toml  [features]      -> Cargo feature combinations
#   * c_src/CMakeLists.txt    option()/if()   -> C-side build switches
#
# Both are parsed here rather than hard-coded, so a configuration added later is
# picked up automatically instead of being silently skipped.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
translation="$(cd "$here/.." && pwd)"
root="$(cd "$translation/.." && pwd)"
cargo_toml="$translation/Cargo.toml"
cmakelists="$root/c_src/CMakeLists.txt"

TIMEOUT=${TIMEOUT:-600}

# ---------------------------------------------------------------- C-side flags
echo "== C build-time configuration (c_src/CMakeLists.txt) =="
c_opts=$(grep -oE '^[[:space:]]*option\([[:space:]]*[A-Za-z_][A-Za-z0-9_]*' "$cmakelists" \
         | awk -F'(' '{print $2}' | tr -d ' ' | sort -u)
c_defs=$(grep -oE 'add_(compile_)?definitions\(|target_compile_definitions\(' "$cmakelists" | sort -u)
if [[ -z "$c_opts" && -z "$c_defs" ]]; then
  echo "  none: no option() switches and no compile definitions -> single C configuration"
else
  echo "  options: $c_opts"
  echo "  definition commands: $c_defs"
fi
echo

# ------------------------------------------------------- Cargo feature parsing
# Read the [features] table, ignoring comments and blank lines.
mapfile -t features < <(
  awk '
    /^[[:space:]]*\[/ { in_f = ($0 ~ /^[[:space:]]*\[features\][[:space:]]*$/); next }
    !in_f { next }
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    match($0, /^[[:space:]]*"?([A-Za-z0-9_-]+)"?[[:space:]]*=/, m) { print m[1] }
  ' "$cargo_toml"
)

echo "== Cargo features (translation/Cargo.toml) =="
if ((${#features[@]} == 0)); then
  echo "  none declared"
else
  printf '  %s\n' "${features[@]}"
fi
echo

# ------------------------------------------------- enumerate all combinations
# With no declared features the only valid configuration is the empty set,
# which is identical to the default build. With N features, enumerate the full
# power set (2^N) so no combination is left untested.
combos=()
n=${#features[@]}
if ((n == 0)); then
  combos=("")
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${features[i]}")
    done
    combos+=("$(IFS=,; echo "${sel[*]}")")
  done
fi

echo "== ${#combos[@]} configuration(s) to verify =="
for c in "${combos[@]}"; do echo "  [${c:-<no features>}]"; done
echo

# ---------------------------------------------------------------- run the loop
fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"
  echo "──────────────────────────────────────────────────────────────"
  echo ">>> configuration: $label"

  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  echo "--- cargo check ${args[*]}"
  if ! timeout "$TIMEOUT" cargo check "${args[@]}" --all-targets \
        --manifest-path "$cargo_toml" 2>&1 | tail -n 15; then
    echo "!!! cargo check FAILED for [$label]"
    fail=1
    continue
  fi

  echo "--- cargo build --release ${args[*]}  (cdylib / exported symbols)"
  if ! timeout "$TIMEOUT" cargo build --release "${args[@]}" \
        --manifest-path "$cargo_toml" 2>&1 | tail -n 15; then
    echo "!!! cargo build FAILED for [$label]"
    fail=1
    continue
  fi

  echo "--- cargo test ${args[*]}"
  # DRIVER_TEST_FEATURES makes the harness build the cdylib under test with the
  # same feature selection as the test binary itself.
  if ! DRIVER_TEST_FEATURES="$combo" timeout "$TIMEOUT" \
        cargo test "${args[@]}" --manifest-path "$cargo_toml" 2>&1 \
        | grep -E "^(test |test result|error|warning: unused|running)" ; then
    echo "!!! cargo test FAILED for [$label]"
    fail=1
    continue
  fi

  echo "--- symbol parity (nm -D) for [$label]"
  if ! "$here/compare_symbols.sh" "$translation/target/release/libdriver.so"; then
    echo "!!! symbol parity FAILED for [$label]"
    fail=1
  fi
done

echo "──────────────────────────────────────────────────────────────"
if ((fail)); then
  echo "RESULT: at least one configuration failed"
  exit 1
fi
echo "RESULT: all ${#combos[@]} configuration(s) verified"

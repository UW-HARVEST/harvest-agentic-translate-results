#!/usr/bin/env bash
# Build the C .so and the Rust .so, then run the full differential suite under
# EVERY feature combination declared in Cargo.toml.
#
# Usage:  ./run_all_features.sh
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a feature automatically widens the matrix.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

CARGO_FLAGS="${CARGO_FLAGS:---offline}"

echo "=== building C shared library ==="
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
c_so="$root/c_src/build/libdriver.so"
test -f "$c_so"
echo "  -> $c_so"

# ---------------------------------------------------------------- feature list
# Everything between the [features] header and the next [section] header, minus
# the implicit "default" key.
features=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' "$here/Cargo.toml")

combos=()
if [[ -z "$features" ]]; then
  echo "=== no [features] in Cargo.toml -> single configuration ==="
  combos+=("default")
  combos+=("no-default")
else
  echo "=== features found: $(echo "$features" | tr '\n' ' ')"
  # Full power set of the declared features, plus the default build.
  mapfile -t farr <<<"$features"
  n=${#farr[@]}
  combos+=("default")
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${farr[i]}")
    done
    combos+=("no-default:$(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

status=0
for combo in "${combos[@]}"; do
  case "$combo" in
  default) args=() ;;
  no-default) args=(--no-default-features) ;;
  no-default:*) args=(--no-default-features --features "${combo#no-default:}") ;;
  esac

  echo
  echo "############################################################"
  echo "# combination: $combo   (cargo ${args[*]:-<default>})"
  echo "############################################################"

  # The cdylib under test must be rebuilt for THIS combination before the
  # tests dlopen it.
  # shellcheck disable=SC2086
  cargo build $CARGO_FLAGS "${args[@]}"
  rust_so="$here/target/debug/libdriver.so"
  test -f "$rust_so"

  # shellcheck disable=SC2086
  if DRIVER_C_SO="$c_so" DRIVER_RUST_SO="$rust_so" \
    timeout 600 cargo test $CARGO_FLAGS "${args[@]}" -- --test-threads=4; then
    echo "RESULT[$combo]: PASS"
  else
    echo "RESULT[$combo]: FAIL"
    status=1
  fi
done

echo
echo "=== symbol diff (C vs Rust) ==="
diff <(nm -D --defined-only --format=posix "$c_so" | awk '{print $1}' | sort) \
  <(nm -D --defined-only --format=posix "$here/target/debug/libdriver.so" |
    awk '{print $1}' | grep -vE '^(_init|_fini|__bss_start|_edata|_end|__cxa_finalize|_ITM_|__gmon_start__|rust_eh_personality|_ZN|__rust)' |
    sort) &&
  echo "  (identical)" || echo "  ^^ extra Rust symbols are OK; MISSING C symbols are not"

exit "$status"

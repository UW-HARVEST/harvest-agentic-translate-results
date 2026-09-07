#!/usr/bin/env bash
# Build both libraries and run the full differential test suite under every
# feature combination declared in Cargo.toml.
#
# The tests redirect fd 1 process-wide, so they MUST run single-threaded.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"

echo "=== building C shared library ==="
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
ls -l "$root/c_src/build/libdriver.so"

# Enumerate declared features (empty if there is no [features] table).
mapfile -t features < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' \
    "$here/Cargo.toml"
)

combos=("default")
if ((${#features[@]} > 0)); then
  combos+=("no-default")
  for f in "${features[@]}"; do combos+=("$f"); done
  # all features at once
  combos+=("__all__")
fi

status=0
for combo in "${combos[@]}"; do
  case "$combo" in
    default)    args=() ;;
    no-default) args=(--no-default-features) ;;
    __all__)    args=(--all-features) ;;
    *)          args=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "=== feature combo: $combo  (cargo ${args[*]:-<default>}) ==="
  cargo build --release --offline "${args[@]}"
  echo "--- exported symbols (Rust .so) ---"
  nm -D --defined-only "$here/target/release/libdriver.so" | awk '{print $3}' | sort
  echo "--- symbol diff vs C .so (must be empty) ---"
  diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$here/target/release/libdriver.so" | awk '{print $3}' | sort) \
       | grep '^<' && { echo "MISSING SYMBOLS!"; status=1; } || echo "(empty)"

  # Both cargo profiles: `dev` and `release` differ in UB checks / panic
  # strategy, which is observable on the C library's null-pointer paths.
  for prof in "--release" ""; do
    echo "--- profile: ${prof:-dev} ---"
    cargo build --offline $prof "${args[@]}" -q
    RUST_TEST_THREADS=1 cargo test --offline $prof "${args[@]}" -- --test-threads=1 \
      || status=1
  done
done

echo
if ((status == 0)); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit $status

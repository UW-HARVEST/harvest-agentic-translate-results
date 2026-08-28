#!/usr/bin/env bash
# Full verification sweep: builds the C .so and the Rust cdylib, compares the
# exported symbol sets, then runs every differential test under every feature
# combination and both cargo profiles.
#
#   IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]`
#   artifact, so the .so must be built explicitly before each test run,
#   otherwise the tests would load a stale library.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cd "$root" || exit 1

fail=0

# ---------------------------------------------------------------- C library ---
echo "=== building the C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so=$(ls c_src/build/lib*.so | head -1)
echo "C  .so: $c_so"

# ------------------------------------------------------------- feature combos -
# Cargo.toml declares no [features], so the only combinations are the default
# feature set and --no-default-features (identical here, both are exercised).
combos=("default" "no-default")

for profile in release dev; do
  for combo in "${combos[@]}"; do
    case "$combo" in
      default)     fflags=() ;;
      no-default)  fflags=(--no-default-features) ;;
    esac
    case "$profile" in
      release) pflags=(--release); pdir=release ;;
      dev)     pflags=();          pdir=debug   ;;
    esac

    echo
    echo "=== profile=$profile features=$combo ==="

    echo "--- cargo build (cdylib) ---"
    ( cd translation && cargo build "${pflags[@]}" "${fflags[@]}" ) 2>&1 | tail -2 || fail=1
    r_so="translation/target/$pdir/libhm_geti_lib.so"
    if [ ! -f "$r_so" ]; then echo "MISSING $r_so"; fail=1; continue; fi

    echo "--- nm -D symbol diff ---"
    nm -D --defined-only "$c_so" | awk '{print $NF}' | sort > "$here/c_names.txt"
    nm -D --defined-only "$r_so" | awk '{print $NF}' | sort > "$here/r_names.txt"
    if diff "$here/c_names.txt" "$here/r_names.txt" > "$here/sym_diff.txt"; then
      echo "symbols: IDENTICAL ($(wc -l < "$here/c_names.txt") exported)"
    else
      echo "symbols: DIFFER"; cat "$here/sym_diff.txt"; fail=1
    fi

    echo "--- cargo test ---"
    RUST_SO="$PWD/$r_so" C_SO="$PWD/$c_so" \
      timeout 900 bash -c "cd translation && cargo test ${pflags[*]} ${fflags[*]} -- --test-threads=1" 2>&1 \
      | grep -E "^(running|test |test result|error|warning: unused)" \
      | grep -vE "^test .* \.\.\. ok$" || fail=1
    # a non-zero cargo exit is captured by PIPESTATUS
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then fail=1; fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL GREEN"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"

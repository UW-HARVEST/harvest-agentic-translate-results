#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run the differential suite across
# every profile and every feature combination.
#
# WHY THIS SCRIPT EXISTS: `cargo test` does NOT rebuild a `crate-type =
# ["cdylib"]` library target. Running `cargo test` alone therefore dlopens a
# STALE .so and the differential tests pass vacuously. The tests contain a
# freshness assertion as a backstop, but the correct workflow is this script.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

CARGO_OFFLINE="${CARGO_OFFLINE:---offline}"

echo "== building C shared library =="
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "   C  .so: $c_so"

# Feature combinations: derived from Cargo.toml's [features] table. If there is
# no [features] table the only combination is the default one.
mapfile -t features < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' \
    "$here/Cargo.toml"
)
echo "== feature list: ${features[*]:-<none: default only>} =="

run_combo() {
  local label="$1"; shift
  local profile="$1"; shift
  echo
  echo "======================================================================"
  echo "== combo: ${label}   profile: ${profile}"
  echo "======================================================================"
  # Build the cdylib FIRST so the .so under test matches the sources.
  cargo build $CARGO_OFFLINE ${profile:+--$profile} "$@"
  local so="$here/target/${profile:-debug}/librgb_to_hsv_lib.so"
  [ "$profile" = "release" ] && so="$here/target/release/librgb_to_hsv_lib.so"
  echo "   Rust .so: $so"
  nm -D --defined-only "$so" | grep ' T ' || true

  echo "   -- symbol parity --"
  diff <(nm -D --defined-only "$c_so" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$so"   | awk '$2=="T"{print $3}' | sort) \
    && echo "   symbol diff: EMPTY (all C symbols present in Rust)" \
    || { echo "   SYMBOL PARITY FAILURE"; return 1; }

  RUST_SO_PATH="$so" cargo test $CARGO_OFFLINE ${profile:+--$profile} "$@"
}

status=0
for profile in "" release; do
  if [ "${#features[@]}" -eq 0 ]; then
    run_combo "default (no [features] table)" "$profile" || status=1
  else
    run_combo "default" "$profile" || status=1
    run_combo "no-default-features" "$profile" --no-default-features || status=1
    for f in "${features[@]}"; do
      run_combo "no-default + $f" "$profile" --no-default-features --features "$f" \
        || status=1
    done
    run_combo "all-features" "$profile" --all-features || status=1
  fi
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$status"

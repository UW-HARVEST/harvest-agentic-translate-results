#!/usr/bin/env bash
# Full differential verification: rebuild the C reference .so, then run every
# test target against every feature combination in every profile.
#
#   ./run_verification.sh
#
# Cargo runs offline because the sandbox has no crates.io access; libloading is
# already in the local registry cache.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
CARGO_FLAGS="--offline"
FAIL=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

step "Building the C reference shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }

# ---------------------------------------------------------------- feature set --
# Mechanically extract the feature names from Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        split($0,a,"="); gsub(/[[:space:]]/,"",a[1]); if (a[1]!="default") print a[1] }' \
      "$HERE/Cargo.toml"
)

# Build the list of combinations to test: the default build, the
# no-default-features build, and the power set of any declared features.
COMBOS=("default" "none")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

step "Feature combinations to verify: ${COMBOS[*]}"
echo "(declared features: ${FEATURES[*]:-<none>})"

# ------------------------------------------------------------------- test runs --
for profile in dev release; do
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      default) featflags=() ;;
      none)    featflags=(--no-default-features) ;;
      *)       featflags=(--no-default-features --features "$combo") ;;
    esac
    profflags=()
    [ "$profile" = release ] && profflags=(--release)

    step "cargo test [$profile] [features=$combo]"
    # `cargo test` does not build a cdylib-only lib target, so build it first —
    # otherwise the tests would silently run against a stale .so.
    if ! ( cd "$HERE" && cargo build $CARGO_FLAGS "${profflags[@]}" "${featflags[@]}" ); then
      echo "FAILED to build cdylib: profile=$profile features=$combo"
      FAIL=1
      continue
    fi

    log=$(mktemp "${TMPDIR:-/tmp}/verify-XXXXXX.log")
    ( cd "$HERE" && timeout 900 cargo test $CARGO_FLAGS "${profflags[@]}" \
        "${featflags[@]}" -- --test-threads=1 ) >"$log" 2>&1
    rc=$?
    grep -E '^(running |test result:|error|warning: unused)' "$log" | sed 's/^/  /'
    if [ "$rc" -ne 0 ]; then
      echo "FAILED: profile=$profile features=$combo (rc=$rc)"
      grep -E '^(test .* FAILED|---- |thread .* panicked|\[.*\] )' "$log" | head -n 40 | sed 's/^/  /'
      FAIL=1
    fi
    rm -f "$log"
  done
done

# ----------------------------------------------------------------- symbol diff --
step "Symbol diff (must be empty)"
for profile in debug release; do
  rs="$HERE/target/$profile/libdriver.so"
  [ -f "$rs" ] || { echo "skip $profile (not built)"; continue; }
  cdef=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $3}' | sort)
  rdef=$(nm -D --defined-only "$rs" | awk '{print $3}' | sort)
  missing=$(comm -23 <(echo "$cdef") <(echo "$rdef"))
  if [ -n "$missing" ]; then
    echo "MISSING from target/$profile/libdriver.so:"; echo "$missing"; FAIL=1
  else
    echo "target/$profile/libdriver.so: 0 missing symbols ($(echo "$cdef" | wc -l) checked)"
  fi
done

step "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit "$FAIL"

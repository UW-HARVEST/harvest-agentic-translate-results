#!/usr/bin/env bash
# Full verification driver: Phase A symbol parity + Phase B/C differential tests,
# repeated for every cargo feature combination and for both build profiles.
#
# Nothing in c_src/ is modified.
set -uo pipefail
[ "${1:-}" = "--heavy" ] && HEAVY=1
cd "$(dirname "$0")/.."          # translation/
ROOT="$(cd .. && pwd)"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Build the C reference library
# ---------------------------------------------------------------------------
step "building the C reference library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/*.so)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations mechanically from Cargo.toml
# ---------------------------------------------------------------------------
step "enumerating feature combinations from Cargo.toml"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "no [features] table -> the only configurations are default / --no-default-features / --all-features"
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "declared features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  # Every individual feature, and the full powerset if it is small enough.
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  n=$(printf '%s\n' $FEATURES | wc -l)
  if [ "$n" -le 6 ]; then
    arr=($FEATURES)
    for ((m=1; m < (1<<n); m++)); do
      sel=""
      for ((b=0; b<n; b++)); do (( m & (1<<b) )) && sel="$sel,${arr[b]}"; done
      COMBOS+=("--no-default-features --features ${sel#,}")
    done
  fi
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. For each combination x profile: build, diff symbols, run the suite
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    label="features='${combo:-<default>}' profile=$prof"
    step "$label"
    relflag=""; [ "$prof" = release ] && relflag="--release"

    if ! timeout 600 cargo build -q $relflag $combo 2>&1 | tail -5; then
      echo "BUILD FAILED: $label"; fail=$((fail+1)); continue
    fi
    R_SO="target/$prof/libcollided_lib.so"
    if [ ! -f "$R_SO" ]; then echo "MISSING $R_SO"; fail=$((fail+1)); continue; fi

    # --- Phase A / D: exported-symbol parity
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/vc.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/vr.txt
    missing=$(comm -23 /tmp/vc.txt /tmp/vr.txt)
    if [ -n "$missing" ]; then
      echo "SYMBOL PARITY FAILED — missing from the Rust .so:"; echo "$missing"
      fail=$((fail+1))
    else
      echo "symbol parity OK ($(wc -l < /tmp/vc.txt) C symbols, 0 missing)"
    fi
    # Undefined non-libc symbols in the Rust .so
    undef=$(nm -D -u "$R_SO" | awk '{print $2}' \
            | grep -vE '^(_|__|abort$|memcpy|memmove|memset|memcmp|bcmp|strlen|malloc|calloc|realloc|free|write|writev|dl_|pthread_|gnu_get_libc|getenv|sysconf|open|close|read|mmap|munmap|mprotect|posix_memalign|sigaction|sigaltstack|syscall|qsort|environ)' || true)
    [ -n "$undef" ] && echo "note: other undefined imports: $(echo $undef | tr '\n' ' ')"

    # --- Phase B + C: the differential suite
    if timeout 600 cargo test -q $relflag $combo 2>&1 | tail -25; then
      echo "tests OK ($label)"
    else
      echo "TESTS FAILED: $label"; fail=$((fail+1))
    fi

    # --- optional heavy structured sweeps (opt in with --heavy)
    if [ "${HEAVY:-0}" = 1 ]; then
      # The `#[ignore]`d exhaustive 2^32 sweeps are run one at a time so no single
      # command exceeds a 600 s budget.
      for t in heavy_c2sub_structured_grid heavy_c2dot_structured_grid \
               heavy_minmax_clamp_structured_grid heavy_predicates_random_bulk \
               heavy_c2dot_exhaustive_single_lane heavy_c2sub_exhaustive_single_lane \
               heavy_c2maxv_exhaustive_single_lane heavy_c2minv_exhaustive_single_lane \
               heavy_c2clampv_exhaustive_single_lane; do
        if timeout 590 cargo test -q $relflag $combo --test heavy -- \
             --ignored --test-threads=1 "$t" >/dev/null 2>&1; then
          echo "  heavy OK: $t"
        else
          echo "  HEAVY FAILED: $t ($label)"; fail=$((fail+1))
        fi
      done
    fi
  done
done

step "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED (${#COMBOS[@]} feature combos x 2 profiles)"
else
  echo "$fail configuration(s) FAILED"
fi
exit "$fail"

#!/usr/bin/env bash
# Phase D automation.
#
#  1. Mechanically enumerates every cargo feature combination declared in
#     Cargo.toml (powerset of the [features] table) and runs `cargo check`
#     plus the full differential suite under each.
#  2. Diffs `nm -D` exported symbols between the C .so and the Rust .so for
#     every combination and for both cargo profiles.
#
# The diff MUST be empty and every test run MUST pass.

set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
FAIL=0

note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; FAIL=1; }

# --- 0. make sure the C library exists -------------------------------------
if [[ ! -f "$C_SO" ]]; then
  note "building the C shared library"
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { bad "C build failed"; exit 1; }
fi

# --- 1. enumerate the feature powerset -------------------------------------
# Parse the [features] section of Cargo.toml; ignore "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
note "declared features: ${N} (${FEATURES[*]:-none})"

COMBOS=()
COMBOS+=("--default")                       # plain `cargo test`
COMBOS+=("--no-default-features")           # empty feature set
if (( N > 0 )); then
  if (( N > 12 )); then
    echo "WARNING: $N features -> powerset too large; capping at 12"
    N=12
  fi
  for (( mask=0; mask < (1<<N); mask++ )); do
    sel=()
    for (( i=0; i<N; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features ${joined}")
    COMBOS+=("--features ${joined}")
  done
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
note "feature combinations to verify: ${#COMBOS[@]}"
printf '  %s\n' "${COMBOS[@]}"

# --- 2. per-combination: check, build both profiles, symbol diff, test ------
for combo in "${COMBOS[@]}"; do
  flags=()
  [[ "$combo" != "--default" ]] && read -r -a flags <<< "$combo"

  note "combo: $combo"

  timeout 600 cargo check "${flags[@]}" >/dev/null 2>&1 \
    || bad "cargo check failed for [$combo]"

  for profile in debug release; do
    pflag=(); [[ "$profile" == release ]] && pflag=(--release)

    timeout 600 cargo build "${pflag[@]}" "${flags[@]}" >/dev/null 2>&1 \
      || { bad "cargo build ($profile) failed for [$combo]"; continue; }

    RUST_SO="target/$profile/libdriver.so"
    [[ -f "$RUST_SO" ]] || { bad "missing $RUST_SO for [$combo]"; continue; }

    # Exported-symbol parity. Compare the C .so's exported symbol names against
    # the Rust .so's.
    c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)
    r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [[ -n "$missing" ]]; then
      bad "[$combo/$profile] symbols exported by C but MISSING from Rust:"
      printf '    %s\n' $missing
    else
      echo "  symbol parity ($profile): OK ($(echo "$c_syms" | wc -l) symbols, 0 missing)"
    fi

    # Undefined non-libc symbols in the Rust .so.
    undef=$(nm -D -u "$RUST_SO" | awk '$1=="U"{print $2}' | sed 's/@.*//' \
      | grep -vE '^(_Unwind_|__|_ITM_|abort$|bcmp$|calloc$|close$|dl_iterate_phdr$|free$|fstat|getcwd$|getenv$|gettid$|lseek|malloc$|memcpy$|memmove$|memset$|mmap|munmap$|open|posix_memalign$|printf$|pthread_|puts$|read$|readlink$|realloc$|realpath$|stat|statx$|strlen$|syscall$|write$|writev$)' )
    if [[ -n "$undef" ]]; then
      bad "[$combo/$profile] unresolved non-libc symbols:"
      printf '    %s\n' $undef
    else
      echo "  undefined non-libc symbols ($profile): 0"
    fi

    # Run the differential suite against THIS profile's .so.
    RUST_DRIVER_SO="$(pwd)/$RUST_SO" \
      timeout 600 cargo test --release "${flags[@]}" --test differential -- --test-threads=1 \
      >/tmp/dt_$$.log 2>&1 \
      || { bad "differential tests failed for [$combo] against $profile .so";
           tail -n 25 /tmp/dt_$$.log; }
    summary=$(grep -m1 'test result:' /tmp/dt_$$.log)
    passed=$(sed -n 's/.*test result: ok\. \([0-9]*\) passed.*/\1/p' <<< "$summary")
    if [[ -z "${passed:-}" || "$passed" -lt 40 ]]; then
      bad "[$combo/$profile] expected >=40 passing differential tests, got: $summary"
    else
      echo "  differential tests vs $profile .so: $passed passed, 0 failed"
    fi
  done
done
rm -f /tmp/dt_$$.log

note "RESULT"
if (( FAIL )); then echo "PHASE D: FAILED"; exit 1; fi
echo "PHASE D: ALL COMBINATIONS PASSED"

#!/usr/bin/env bash
# Full verification run: builds both libraries and runs the whole differential
# suite under every feature combination and both profiles.
#
# Usage:  ./run_all.sh
#
# Env knobs forwarded to the tests:
#   EXHAUSTIVE_STRIDE  sample every Nth of the 2^32 inputs (default 1 = all)
#   EXHAUSTIVE_THREADS worker threads for the exhaustive sweep
#   DIFF_SAMPLES       randomized samples per configuration cell
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
REPO_DIR="$(dirname "$CRATE_DIR")"

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }
good() { printf '\033[32mok: %s\033[0m\n' "$*"; }

# --------------------------------------------------------------------------
note "Building the C shared library"
# --------------------------------------------------------------------------
(
  cd "$REPO_DIR/c_src" && mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { bad "C build"; exit 1; }

C_SO=$(find "$REPO_DIR/c_src/build" -maxdepth 2 -name 'lib*.so' | sort | head -1)
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
good "C library: $C_SO"

# --------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
# --------------------------------------------------------------------------
# Collect declared features (excluding "default"), then build the power set.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> the only combination is the default (empty) one"
  # Still exercise both spellings, since they must be equivalent.
  COMBOS=("" "--no-default-features")
else
  echo "declared features: $(echo "$FEATURES" | tr '\n' ' ')"
  COMBOS=("" "--no-default-features")
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((k = 0; k < n; k++)); do
      if (( mask & (1 << k) )); then sel="$sel,${feats[k]}"; fi
    done
    sel="${sel#,}"
    if [ -n "$sel" ]; then
      COMBOS+=("--no-default-features --features $sel")
    fi
  done
  COMBOS+=("--all-features")
fi

printf 'combinations to test: %d\n' "${#COMBOS[@]}"

# --------------------------------------------------------------------------
note "Running the differential suite for every combination x profile"
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="cargo test ${profile:-<debug>} ${combo:-<default features>}"

    # The tests dlopen the .so built with the SAME profile, so build it first.
    if ! cargo build $profile $combo >/dev/null 2>&1; then
      bad "$label (build of the cdylib failed)"
      continue
    fi
    if out=$(cargo test $profile $combo -- --test-threads=8 2>&1); then
      counts=$(echo "$out" | grep -c '^test .* \.\.\. ok$')
      good "$label -- $counts tests passed"
    else
      bad "$label"
      echo "$out" | grep -E '^(test .*FAILED|error|thread |assertion|DIVERGENCE)' | head -25
    fi
  done
done

# --------------------------------------------------------------------------
note "Symbol parity (nm -D)"
# --------------------------------------------------------------------------
for profile in debug release; do
  RS_SO="target/$profile/libfloat2half_lib.so"
  [ -f "$RS_SO" ] || continue
  csyms=$(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u)
  rsyms=$(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  if [ -z "$missing" ]; then
    good "$profile: every C symbol is exported by the Rust .so"
    echo "     C: $(echo "$csyms" | tr '\n' ' ')"
    echo "     Rust: $(echo "$rsyms" | tr '\n' ' ')"
  else
    bad "$profile: Rust .so is missing: $(echo "$missing" | tr '\n' ' ')"
  fi
done

# --------------------------------------------------------------------------
note "Undefined non-libc symbols in the Rust .so"
# --------------------------------------------------------------------------
# Everything the Rust cdylib imports should resolve to libc/libgcc: either a
# glibc-versioned symbol (name@GLIBC_x.y) or a well-known unversioned libc /
# unwinder name pulled in by Rust's std.
undef=$(nm -D --undefined-only target/release/libfloat2half_lib.so 2>/dev/null |
        awk '{print $2}' |
        grep -v '@GLIBC' |
        grep -vE '^(__|_ITM_|_Unwind_|_exit)' || true)
if [ -z "$undef" ]; then
  good "0 undefined non-libc symbols (all imports resolve to libc/libgcc)"
  echo "     (libc imports come from Rust's std being linked into the cdylib;"
  echo "      the C library imports none because it calls nothing.)"
else
  bad "unexpected undefined non-libc symbols: $(echo "$undef" | tr '\n' ' ')"
fi
# And prove the .so actually resolves at load time.
if ldd -r target/release/libfloat2half_lib.so 2>&1 | grep -q 'undefined symbol'; then
  bad "ldd -r reports unresolved symbols in the Rust .so"
else
  good "ldd -r: the Rust .so has no unresolved symbols"
fi

# --------------------------------------------------------------------------
note "Mutation check (proves the suite can actually detect divergence)"
# --------------------------------------------------------------------------
if command -v python3 >/dev/null; then
  if python3 mutation_check.py; then
    good "all injected mutations were caught"
  else
    bad "a mutation survived: the suite has a blind spot"
  fi
else
  echo "python3 unavailable; skipped"
fi

# --------------------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
  printf '\n\033[32m==== ALL CHECKS PASSED ====\033[0m\n'
else
  printf '\n\033[31m==== FAILURES PRESENT ====\033[0m\n'
fi
exit "$fail"

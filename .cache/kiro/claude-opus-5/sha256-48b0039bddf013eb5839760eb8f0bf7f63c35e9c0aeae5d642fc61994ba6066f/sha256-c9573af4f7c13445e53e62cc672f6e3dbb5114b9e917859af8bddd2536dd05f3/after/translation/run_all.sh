#!/usr/bin/env bash
# Full verification sweep: builds the C .so, then for EVERY feature combination
# and both profiles builds the Rust cdylib, diffs `nm -D`, and runs the
# differential test suites (Phase B + Phase C).
#
# Usage: ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Build the C shared object (ground truth)
# ---------------------------------------------------------------------------
say "Building C shared library"
( mkdir -p "$ROOT/c_src/build" \
  && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/tmp/c_build.log 2>&1 || { bad "C build (see /tmp/c_build.log)"; tail -20 /tmp/c_build.log; exit 1; }

C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
# Everything between a [features] header and the next [section] header.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, "", $0); print $0 }
' Cargo.toml | grep -v '^default$' | sort -u)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> the only configuration is the default."
  COMBOS=("default")
else
  # Full power set of the declared features, plus the default build.
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  COMBOS=("default")
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("nodefault:${combo}")
  done
fi

echo "Configurations to verify: ${COMBOS[*]}"

# ---------------------------------------------------------------------------
# 3. For each configuration x profile: build, symbol-diff, test
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "default" ]; then
    FLAGS=()
  else
    feats="${combo#nodefault:}"
    FLAGS=(--no-default-features)
    [ -n "$feats" ] && FLAGS+=(--features "$feats")
  fi

  for profile in debug release; do
    PFLAGS=()
    [ "$profile" = release ] && PFLAGS=(--release)

    say "config=[$combo] profile=$profile : cargo check"
    timeout 600 cargo check "${FLAGS[@]}" "${PFLAGS[@]}" >/tmp/check.log 2>&1 \
      || { bad "cargo check [$combo/$profile]"; tail -20 /tmp/check.log; continue; }

    say "config=[$combo] profile=$profile : cargo build (emits the cdylib)"
    timeout 600 cargo build "${FLAGS[@]}" "${PFLAGS[@]}" >/tmp/build.log 2>&1 \
      || { bad "cargo build [$combo/$profile]"; tail -20 /tmp/build.log; continue; }

    R_SO="target/$profile/libsynth_pair_lib.so"
    [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

    say "config=[$combo] profile=$profile : nm -D symbol diff"
    nm -D --defined-only "$C_SO" | awk '{print $3}' | grep -v '^$' | sort -u >/tmp/c_syms.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^$' | sort -u >/tmp/r_syms.txt
    MISSING="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
    if [ -n "$MISSING" ]; then
      bad "symbols exported by C but MISSING from Rust [$combo/$profile]:"
      echo "$MISSING"
    else
      echo "OK: 0 missing symbols ($(wc -l </tmp/c_syms.txt) C symbols all present)"
    fi
    # Undefined symbols must all resolve to libc/glibc (versioned @GLIBC_*) or
    # the platform runtime; anything else would be an unresolved project symbol.
    UNDEF="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | grep -v '^$' \
             | grep -v '@GLIBC' | grep -vE '^(_ITM_|__cxa_|__gmon|__tls_get_addr|_Unwind_)' || true)"
    if [ -n "$UNDEF" ]; then
      bad "Rust .so has unresolved NON-libc undefined symbols:"; echo "$UNDEF"
    else
      echo "OK: all undefined symbols in the Rust .so resolve to libc/glibc"
    fi

    say "config=[$combo] profile=$profile : Phase B + C differential tests"
    timeout 600 cargo test "${FLAGS[@]}" "${PFLAGS[@]}" -- --test-threads=4 >/tmp/test.log 2>&1
    if [ $? -ne 0 ]; then
      bad "cargo test [$combo/$profile]"
      grep -nE 'DIVERGENCE|panicked|^test .* FAILED|failures:' /tmp/test.log | head -40
    else
      grep -E '^test result:' /tmp/test.log
    fi

    if [ "$profile" = release ]; then
      say "config=[$combo] profile=release : exhaustive sweeps (opt-in tests)"
      EXHAUSTIVE=1 timeout 600 cargo test "${FLAGS[@]}" --release --test exhaustive \
        -- --nocapture --test-threads=2 >/tmp/exh.log 2>&1
      if [ $? -ne 0 ]; then
        bad "exhaustive sweep [$combo]"
        grep -nE 'divergence|panicked|NOT equivalent' /tmp/exh.log | head -20
      else
        grep -E 'exhaustive' /tmp/exh.log
      fi
    fi
  done
done

say "config-independent: mutation sensitivity check"
FUZZ_ITERS=4000 timeout 600 ./mutation_check.sh >/tmp/mut.log 2>&1
if [ $? -ne 0 ]; then bad "mutation check (a mutation survived)"; fi
grep -E 'killed|SURVIVED|survived|RESULT' /tmp/mut.log

say "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"

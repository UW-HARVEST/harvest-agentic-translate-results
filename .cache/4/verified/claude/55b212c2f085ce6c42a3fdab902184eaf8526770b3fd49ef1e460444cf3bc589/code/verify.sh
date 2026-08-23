#!/bin/bash
# Full verification driver: builds the C and Rust shared objects, checks symbol
# parity and runs every differential test under *every* feature combination.
set -u
cd "$(dirname "$0")"
W="$PWD"
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- feature set
# Enumerate the [features] table of Cargo.toml and build the power set.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { inf = 1; next }
    /^\[/                { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }' Cargo.toml
)
NF=${#FEATURES[@]}
say "feature flags found: $NF (${FEATURES[*]:-none})"

COMBOS=()
if [ "$NF" -eq 0 ]; then
  COMBOS=("")
else
  for ((mask = 0; mask < (1 << NF); mask++)); do
    set=""
    for ((i = 0; i < NF; i++)); do
      if (((mask >> i) & 1)); then set="${set:+$set,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$set")
  done
fi
say "feature combinations to verify: ${#COMBOS[@]}"

# ------------------------------------------------------------------ C library
say "building the C shared library"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null) || { echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/libjansson.so

# ------------------------------------------------------- per-combination work
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FLAGS=(--no-default-features)
    LABEL="<no features>"
  else
    FLAGS=(--no-default-features --features "$combo")
    LABEL="$combo"
  fi

  say "cargo check  [$LABEL]"
  if ! timeout 600 cargo check "${FLAGS[@]}" --all-targets 2>&1 | tail -5; then
    echo "CHECK FAILED for $LABEL"
    FAIL=1
  fi

  say "cargo build --release  [$LABEL]"
  if ! timeout 600 cargo build --release "${FLAGS[@]}" 2>&1 | tail -5; then
    echo "BUILD FAILED for $LABEL"
    FAIL=1
    continue
  fi

  say "symbol parity  [$LABEL]"
  ./gen_symbols.sh
  MISSING=$(comm -23 \
    <(nm -D --defined-only c_src/build/libjansson.so | awk '$2 ~ /^[TDBRWVi]$/ {print $3}' | sort -u) \
    <(nm -D --defined-only target/release/libjansson.so | awk '$2 ~ /^[TDBRWVi]$/ {print $3}' | sort -u))
  if [ -n "$MISSING" ]; then
    echo "MISSING SYMBOLS for $LABEL:"; echo "$MISSING"
    FAIL=1
  else
    echo "0 missing symbols"
  fi

  say "cargo test  [$LABEL]"
  if ! timeout 600 cargo test "${FLAGS[@]}" 2>&1 | grep -E "^test |test result:|panicked|DIVERGENCE|SIGSEGV|SIGABRT|error"; then
    echo "no test output?"
    FAIL=1
  fi
  if timeout 600 cargo test "${FLAGS[@]}" 2>&1 | grep -qE "FAILED|failures:|error:"; then
    echo "TESTS FAILED for $LABEL"
    FAIL=1
  fi

  # Bonus pass: run the very same suite against the *debug* artefact.  It has
  # `debug_assertions` on (so the `debug_assert!`s that mirror the C `assert()`s
  # are live) and much larger stack frames, hence RUST_MIN_STACK for the
  # 2048-level-deep parser test.
  say "cargo test against the debug .so  [$LABEL]"
  timeout 600 cargo build "${FLAGS[@]}" >/dev/null 2>&1
  if JANSSON_RUST_SO="$W/target/debug/libjansson.so" RUST_MIN_STACK=134217728 \
     timeout 600 cargo test "${FLAGS[@]}" 2>&1 |
     grep -qE "FAILED|failures:|panicked|SIGSEGV|SIGABRT"; then
    echo "DEBUG-ARTEFACT TESTS FAILED for $LABEL"
    FAIL=1
  else
    echo "debug artefact: all green"
  fi
done

say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL GREEN"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"

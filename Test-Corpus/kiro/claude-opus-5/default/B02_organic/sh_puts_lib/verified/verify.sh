#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff dynamic symbols, enumerate every
# Cargo feature combination and run the full differential suite under each.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

step "Build C shared library"
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . ) \
  || { echo "C build FAILED"; exit 1; }
CSO="$(find "$CBUILD" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
echo "C  .so: $CSO"

step "Enumerate Cargo feature combinations"
# Extract feature names from [features] (excluding "default"); empty => single
# default configuration only.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]); if(a[1]!="default") print a[1]}' "$CRATE/Cargo.toml")
if [ -z "$FEATURES" ]; then
  echo "no [features] table in Cargo.toml -> exactly one build configuration"
  COMBOS=("__default__")
else
  echo "features: $FEATURES"
  # power set
  COMBOS=("__nodefault__")
  for f in $FEATURES; do
    new=()
    for c in "${COMBOS[@]}"; do new+=("$c,$f"); done
    COMBOS+=("${new[@]}")
  done
  COMBOS+=("__default__")
fi

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__)   ARGS=() ;              LABEL="default features" ;;
    __nodefault__) ARGS=(--no-default-features); LABEL="--no-default-features" ;;
    *)             ARGS=(--no-default-features --features "${combo#__nodefault__,}"); LABEL="$combo" ;;
  esac

  step "Configuration: $LABEL"

  ( cd "$CRATE" && cargo build --release "${ARGS[@]}" >/dev/null 2>&1 ) \
    || { echo "  cargo build FAILED"; fail=1; continue; }
  RSO="$CRATE/target/release/libsh_puts_lib.so"

  echo "-- symbol diff (C exports missing from Rust) --"
  missing=$(comm -23 \
    <(nm -D --defined-only "$CSO" | awk '{print $3}' | sort) \
    <(nm -D --defined-only "$RSO" | awk '{print $3}' | sort))
  if [ -n "$missing" ]; then
    echo "  MISSING: $missing"; fail=1
  else
    nc=$(nm -D --defined-only "$CSO" | wc -l)
    echo "  OK: all $nc C symbols exported by the Rust .so"
  fi

  echo "-- undefined non-libc symbols in the Rust .so --"
  und=$(nm -D -u "$RSO" | awk '{print $2}' | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^__cxa_\|^gettid\|^statx\|^_Unwind_' || true)
  if [ -n "$und" ]; then echo "  UNRESOLVED: $und"; fail=1; else echo "  OK: none"; fi

  for prof in "" "--release"; do
    echo "-- cargo test ${prof:-debug} --"
    if ( cd "$CRATE" && timeout 600 cargo test $prof "${ARGS[@]}" -- --test-threads=1 2>&1 \
         | grep -E '^(test result|error|warning: unused)' ); then :; fi
    # shellcheck disable=SC2181
    ( cd "$CRATE" && timeout 600 cargo test $prof "${ARGS[@]}" -- --test-threads=1 >/dev/null 2>&1 ) \
      || { echo "  TESTS FAILED (${prof:-debug})"; fail=1; }
  done
done

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"

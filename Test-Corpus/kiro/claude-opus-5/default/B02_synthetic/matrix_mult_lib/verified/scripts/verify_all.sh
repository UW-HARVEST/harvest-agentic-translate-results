#!/usr/bin/env bash
# Verifies the Rust translation against the C reference for every build
# configuration. The crate declares no [features], and CMakeLists.txt has no
# build options, so the only axis is the cargo profile (dev / release).
set -uo pipefail

cd "$(dirname "$0")/../.." || exit 1
ROOT="$PWD"
LOG=/tmp/verify_translation.log
: > "$LOG"

say() { printf '%s\n' "$*" | tee -a "$LOG"; }

say "== building C reference =="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
) >> "$LOG" 2>&1 || { say "FAIL: C build"; exit 1; }

cd "$ROOT/translation" || exit 1

# Enumerate feature combinations from Cargo.toml. With no [features] section the
# only valid combination is the empty one.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  say "== feature combinations: 1 (no [features] in Cargo.toml) =="
  COMBOS=("")
else
  say "== features found: $FEATURES =="
  COMBOS=("")
  for f in $FEATURES; do
    new=()
    for c in "${COMBOS[@]}"; do
      new+=("$c")
      if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
  done
fi

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  say "== cargo check --no-default-features --features '$label' =="
  if ! timeout 600 cargo check --no-default-features --features "$combo" >> "$LOG" 2>&1; then
    say "FAIL: cargo check ($label)"; rc=1; continue
  fi

  for profile in dev release; do
    outdir=debug; [ "$profile" = release ] && outdir=release
    say "== build+test profile=$profile features='$label' =="
    if ! timeout 600 cargo build --profile "$profile" --no-default-features --features "$combo" >> "$LOG" 2>&1; then
      say "FAIL: cargo build ($label, $profile)"; rc=1; continue
    fi
    # Point the harness at the cdylib built for this profile, so the release
    # build (optimised, panic=abort) is differentially tested too.
    if ! RUST_DRIVER_SO="$ROOT/translation/target/$outdir/libdriver.so" \
         timeout 600 cargo test --profile "$profile" --no-default-features --features "$combo" \
         -- --test-threads=1 >> "$LOG" 2>&1; then
      say "FAIL: cargo test ($label, $profile)"; rc=1; continue
    fi
    say "  ok (tested against target/$outdir/libdriver.so)"
  done
done

say "== symbol comparison =="
nm -D --defined-only "$ROOT/c_src/build/libdriver.so" \
  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > /tmp/c_syms.txt
for so in "$ROOT/translation/target/debug/libdriver.so" "$ROOT/translation/target/release/libdriver.so"; do
  [ -f "$so" ] || continue
  nm -D --defined-only "$so" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u > /tmp/r_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
  if [ -n "$missing" ]; then
    say "FAIL: $so missing: $missing"; rc=1
  else
    say "  ok: $(basename "$(dirname "$so")") exports all $(wc -l < /tmp/c_syms.txt) C symbols"
  fi
done

if [ "$rc" -eq 0 ]; then say "== ALL CONFIGURATIONS PASS =="; else say "== FAILURES PRESENT =="; fi
exit "$rc"

#!/usr/bin/env bash
# Full differential verification: C .so vs Rust .so, across every feature combo.
#
# Usage: ./verify.sh
#
# Why the explicit `cargo build` before every `cargo test`: no Rust target can
# depend on a `cdylib`, so `cargo test` does NOT rebuild libcharinbuf_lib.so.
# Without the build step the tests would silently load a STALE library and pass
# vacuously. tests/common/mod.rs also asserts freshness as a backstop.

set -uo pipefail
cd "$(dirname "$0")"

# /tmp may be read-only in sandboxes; keep scratch files somewhere writable.
WORK="$(mktemp -d "${TMPDIR:-/tmp}/charinbuf-verify.XXXXXX")" || { echo "cannot make temp dir"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

FAIL=0
say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
say "1. Enumerate build-time configurations"
# ---------------------------------------------------------------------------
# Every feature from Cargo.toml's [features] section.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&/^[a-zA-Z0-9_-]+ *=/{print $1}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml has no [features] section."
  # Cross-check that the C side has no compile-time switches either.
  if grep -rqE '#if(def|ndef)? |option\(|target_compile_definitions|add_definitions' \
        c_src/CMakeLists.txt c_src/src c_src/include 2>/dev/null; then
    bad "C source has conditional compilation but Cargo.toml has no features"
  else
    echo "C source has no #ifdef / option() / *_definitions either."
  fi
  echo "=> exactly ONE valid configuration: the empty feature set."
  COMBOS=("")
else
  echo "features found: $FEATURES"
  # Power set of all features.
  mapfile -t FEATURE_ARR <<<"$FEATURES"
  n=${#FEATURE_ARR[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEATURE_ARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  printf '=> %d combinations\n' "${#COMBOS[@]}"
fi

# ---------------------------------------------------------------------------
say "2. Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || bad "C build"
C_SO=c_src/build/libtranslated_rust.so
[ -f "$C_SO" ] || bad "missing $C_SO"
echo "built $C_SO"

# ---------------------------------------------------------------------------
say "3. cargo check every configuration"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  if [ -z "$combo" ]; then args=(--no-default-features); else args=(--no-default-features --features "$combo"); fi
  if timeout 600 cargo check --offline --all-targets "${args[@]}" >$WORK/chk 2>&1; then
    echo "  ok    check $label"
  else
    bad "check $label"; tail -20 $WORK/chk
  fi
done
# The default feature set is also a real configuration a consumer can select.
if timeout 600 cargo check --offline --all-targets >$WORK/chk 2>&1; then
  echo "  ok    check <default>"
else
  bad "check <default>"; tail -20 $WORK/chk
fi

# ---------------------------------------------------------------------------
say "4. Symbol parity (nm -D)"
# ---------------------------------------------------------------------------
timeout 600 cargo build --offline >/dev/null 2>&1 || bad "cargo build"
RUST_SO=target/debug/libcharinbuf_lib.so
nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort >$WORK/csym
nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort >$WORK/rsym
# Guard against a false pass from an empty/missing symbol dump.
[ -s "$WORK/csym" ] || bad "nm produced no symbols for the C .so"
[ -s "$WORK/rsym" ] || bad "nm produced no symbols for the Rust .so"
MISSING=$(comm -23 "$WORK/csym" "$WORK/rsym")
echo "C exports:    $(wc -l <$WORK/csym)"
echo "Rust exports: $(wc -l <$WORK/rsym) (incl. Rust-runtime symbols)"
if [ -n "$MISSING" ]; then
  bad "symbols missing from the Rust .so:"; echo "$MISSING"
else
  echo "  ok    0 missing symbols"
fi
# No undefined non-libc symbols.
UNDEF=$(nm -D -u "$RUST_SO" | awk '{print $2}' \
        | grep -vE '@|^_ITM_|^__gmon_start__$|^_Unwind|^__cxa|^_ZN|^rust_|^__rust' || true)
if [ -n "$UNDEF" ]; then
  bad "unresolved non-libc symbols:"; echo "$UNDEF"
else
  echo "  ok    no unresolved non-libc symbols"
fi

# ---------------------------------------------------------------------------
say "5. Phase B + C + D differential tests, every configuration"
# ---------------------------------------------------------------------------
run_tests() {
  local label="$1"; shift
  # MUST build first: cargo test does not rebuild a cdylib.
  if ! timeout 600 cargo build --offline "$@" >$WORK/bld 2>&1; then
    bad "build $label"; tail -20 $WORK/bld; return
  fi
  if timeout 600 cargo test --offline "$@" -- --test-threads=1 >$WORK/tst 2>&1; then
    echo "  ok    tests $label  ($(grep -c '^test .* ok$' $WORK/tst) passed)"
  else
    bad "tests $label"
    grep -E 'FAILED|panicked|mismatch|^test result' $WORK/tst | head -30
  fi
}

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  if [ -z "$combo" ]; then
    run_tests "$label" --no-default-features
  else
    run_tests "$label" --no-default-features --features "$combo"
  fi
done
run_tests "<default>"

# The release profile is a distinct configuration (Cargo.toml sets
# panic = "abort" there) and it optimizes the translated code differently, so
# the whole suite runs against the optimized .so too.
for combo in "${COMBOS[@]}"; do
  label="release ${combo:-<no features>}"
  if [ -z "$combo" ]; then
    run_tests "$label" --release --no-default-features
  else
    run_tests "$label" --release --no-default-features --features "$combo"
  fi
done
run_tests "release <default>" --release

# ---------------------------------------------------------------------------
say "6. Release-profile symbol parity"
# ---------------------------------------------------------------------------
REL_SO=target/release/libcharinbuf_lib.so
if [ -f "$REL_SO" ]; then
  nm -D --defined-only "$REL_SO" | awk '{print $3}' | sort >"$WORK/relsym"
  [ -s "$WORK/relsym" ] || bad "nm produced no symbols for the release .so"
  MISSING_REL=$(comm -23 "$WORK/csym" "$WORK/relsym")
  if [ -n "$MISSING_REL" ]; then
    bad "symbols missing from the release Rust .so:"; echo "$MISSING_REL"
  else
    echo "  ok    0 missing symbols (release)"
  fi
else
  bad "missing $REL_SO"
fi

# ---------------------------------------------------------------------------
if [ "$FAIL" -eq 0 ]; then
  printf '\n\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\n\033[31mVERIFICATION FAILED\033[0m\n'
fi
exit "$FAIL"

#!/usr/bin/env bash
# Phase D driver: builds both libraries, checks symbol parity, and runs the full
# differential suite under every feature combination and both Rust profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CSO="$ROOT/c_src/build/libdriver.so"
CARGO_FLAGS="--offline"

fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "$CSO" || { bad "C build"; exit 1; }

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# Every declared feature (none in this crate) -> powerset of flag sets.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' "$CRATE/Cargo.toml")
COMBOS=("")                        # default features
COMBOS+=("--no-default-features")  # nothing enabled
if [ -n "$FEATURES" ]; then
  COMBOS+=("--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
  echo "  declared features: $FEATURES"
else
  echo "  no [features] table -> only the default configuration exists"
fi
printf '  %d combination(s)\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "cargo check under every feature combination"
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  if ( cd "$CRATE" && timeout 600 cargo check $CARGO_FLAGS --all-targets $combo >/dev/null 2>&1 ); then
    ok "cargo check ${combo:-<default>}"
  else
    bad "cargo check ${combo:-<default>}"
  fi
done

# ---------------------------------------------------------------------------
step "Build the Rust shared library (both profiles, every feature combination)"
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    relflag=""; [ "$prof" = release ] && relflag="--release"
    # shellcheck disable=SC2086
    if ( cd "$CRATE" && timeout 600 cargo build $CARGO_FLAGS $relflag $combo >/dev/null 2>&1 ); then
      ok "cargo build $prof ${combo:-<default>}"
    else
      bad "cargo build $prof ${combo:-<default>}"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Symbol parity: nm -D on the C .so vs the Rust .so"
syms() { nm -D --defined-only "$1" | awk '{print $3}' | sort -u; }
for prof in release debug; do
  RSO="$CRATE/target/$prof/libdriver.so"
  [ -f "$RSO" ] || { bad "missing $RSO"; continue; }
  missing=$(comm -23 <(syms "$CSO") <(syms "$RSO"))
  extra=$(comm -13 <(syms "$CSO") <(syms "$RSO"))
  if [ -z "$missing" ]; then
    ok "$prof: 0 symbols missing from the Rust .so ($(syms "$CSO" | wc -l) exported by C)"
  else
    bad "$prof: symbols MISSING from the Rust .so:"; echo "$missing" | sed 's/^/       /'
  fi
  [ -n "$extra" ] && printf '  \033[33mNOTE\033[0m %s: extra Rust-only symbols:\n%s\n' "$prof" "$(echo "$extra" | sed 's/^/       /')"
done

step "Unresolved imports in the Rust .so (ldd -r: actual link closure)"
# Rust's std legitimately imports many more libc symbols than the C .so does;
# what matters is that NONE of them are unresolvable, i.e. there is no missing
# module that a stub would have papered over.
for prof in release debug; do
  RSO="$CRATE/target/$prof/libdriver.so"
  [ -f "$RSO" ] || continue
  und=$(ldd -r "$RSO" 2>&1 | grep -E 'undefined symbol|not found' || true)
  if [ -z "$und" ]; then
    n=$(nm -D --undefined-only "$RSO" | wc -l)
    ok "$prof: 0 unresolved symbols ($n imports, all satisfied by libc/libgcc)"
  else
    bad "$prof: UNRESOLVED symbols:"; echo "$und" | sed 's/^/       /'
  fi
done
# The C .so's own imports, for the record.
printf '  C .so imports: %s\n' "$(nm -D --undefined-only "$CSO" | awk '{print $NF}' | grep -vE '^_|GLIBC_2\.2\.5$' | paste -sd' ')"

# ---------------------------------------------------------------------------
step "Differential test suite (every feature combination x both profiles)"
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    RSO="$CRATE/target/$prof/libdriver.so"
    [ -f "$RSO" ] || continue
    # shellcheck disable=SC2086
    out=$( cd "$CRATE" && DRIVER_RUST_SO="$RSO" timeout 600 cargo test $CARGO_FLAGS $combo 2>&1 )
    if printf '%s' "$out" | grep -q 'FAILED\|error\[\|error:'; then
      bad "tests ${combo:-<default>} / $prof"
      printf '%s\n' "$out" | grep -E '^(test |error|---- |thread )' | head -40 | sed 's/^/       /'
    else
      n=$(printf '%s' "$out" | grep -oP '\d+(?= passed)' | awk '{s+=$1} END{print s+0}')
      ok "tests ${combo:-<default>} / $prof  ($n test fns passed)"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Result"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL PHASE D CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$fail"

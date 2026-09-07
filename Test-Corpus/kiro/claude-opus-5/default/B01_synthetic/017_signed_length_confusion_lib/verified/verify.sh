#!/usr/bin/env bash
# Phase D completion gate, fully automated.
#
#   1. rebuild the C .so and the Rust .so (debug + release)
#   2. enumerate the crate's feature combinations from Cargo.toml
#   3. cargo check + cargo test for every combination
#   4. run the suite against BOTH the debug and the release Rust .so
#   5. run the suite parallel AND single-threaded (twice, to catch flakiness)
#   6. diff `nm -D` between the two .so files; the diff must be empty
#
# Usage: cd translation && ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(pwd)"
C_BUILD="$ROOT/../c_src/build"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; FAIL=1; }

step "1. build the C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) \
  && ok "C .so built" || bad "C .so build"
C_SO="$C_BUILD/libdriver.so"

step "2. build the Rust cdylib (debug + release)"
timeout 600 cargo build          >/dev/null 2>&1 && ok "debug   cdylib" || bad "debug cdylib"
timeout 600 cargo build --release >/dev/null 2>&1 && ok "release cdylib" || bad "release cdylib"

step "3. enumerate feature combinations declared in Cargo.toml"
# Every feature name under a [features] table, if any exist at all.
FEATURES=$(awk '
  /^[[:space:]]*\[features\]/ { inf=1; next }
  /^[[:space:]]*\[/            { inf=0 }
  inf && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/,""); gsub(/[[:space:]]/,""); if ($0 != "") print
  }
' Cargo.toml | sort -u)
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> the only combination is the default (empty) set"
  COMBOS=("__default__" "__none__")
else
  echo "features found: $FEATURES"
  # Power set of the declared features, plus the default build.
  COMBOS=("__default__" "__none__")
  names=($FEATURES); n=${#names[@]}
  for ((m=1; m<(1<<n); m++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      (( m & (1<<i) )) && combo="${combo:+$combo,}${names[i]}"
    done
    COMBOS+=("$combo")
  done
fi

step "4. cargo check for every feature combination"
for c in "${COMBOS[@]}"; do
  case "$c" in
    __default__) args=() ;   label="(default features)" ;;
    __none__)    args=(--no-default-features); label="--no-default-features" ;;
    *)           args=(--no-default-features --features "$c"); label="--no-default-features --features $c" ;;
  esac
  if timeout 600 cargo check "${args[@]}" >/dev/null 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"
  fi
done

step "5. cargo test for every feature combination x {debug,release} .so x {parallel,serial}"
for c in "${COMBOS[@]}"; do
  case "$c" in
    __default__) args=() ;   label="(default)" ;;
    __none__)    args=(--no-default-features); label="--no-default-features" ;;
    *)           args=(--no-default-features --features "$c"); label="--features $c" ;;
  esac
  for so in debug release; do
    for threads in parallel serial; do
      if [ "$threads" = serial ]; then extra=(-- --test-threads=1); else extra=(); fi
      if RUST_SO="$ROOT/target/$so/libdriver.so" C_SO="$C_SO" \
         timeout 600 cargo test "${args[@]}" --test differential "${extra[@]}" >/tmp/vt.$$ 2>&1; then
        ok "cargo test $label  rust-so=$so  $threads  ($(grep -o '[0-9]* passed' /tmp/vt.$$ | head -1))"
      else
        bad "cargo test $label  rust-so=$so  $threads"
        tail -25 /tmp/vt.$$
      fi
      rm -f /tmp/vt.$$
    done
  done
done

step "6. nm -D symbol diff (must be empty)"
for so in debug release; do
  R="$ROOT/target/$so/libdriver.so"
  d=$(diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
           <(nm -D --defined-only "$R"    | awk '{print $3}' | sort))
  if [ -z "$d" ]; then
    ok "symbol sets identical (C vs rust-$so): $(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort | tr '\n' ' ')"
  else
    bad "symbol diff (C vs rust-$so):"; echo "$d"
  fi
  # No undefined NON-libc symbols in the Rust .so. Every glibc import carries an
  # @GLIBC_* version tag (or is a reserved `_`-prefixed ld/glibc name), so
  # anything left after stripping those would be a genuine unresolved dependency.
  u=$(nm -D --undefined-only "$R" | awk '{print $2}' \
      | grep -v -E '@GLIBC' | grep -v -E '^_' | grep -v -E '^$')
  if [ -z "$u" ]; then
    ok "no undefined non-libc symbols in rust-$so"
  else
    bad "undefined non-libc symbols in rust-$so:"; echo "$u"
  fi
done

step "7. binary/driver executable check"
if grep -q add_executable ../c_src/CMakeLists.txt 2>/dev/null; then
  bad "c_src declares an executable; its stdout must be compared too"
else
  ok "c_src builds no executable (add_library only) -> no binary stdout comparison applies"
fi
if [ -f src/main.rs ] || grep -q '^\[\[bin\]\]' Cargo.toml; then
  bad "the Rust crate declares a binary the C side does not have"
else
  ok "the Rust crate declares no binary either"
fi

printf '\n===========================\n'
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
printf '===========================\n'
exit "$FAIL"

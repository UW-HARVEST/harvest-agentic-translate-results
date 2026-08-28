#!/usr/bin/env bash
# Full verification driver.
#
#  1. cargo check for every feature combination declared in Cargo.toml
#  2. symbol-parity diff (nm -D) between the C and Rust .so
#  3. the whole differential suite for every feature combination
#  4. the whole differential suite against the C library rebuilt at
#     -O0/-O1/-O2/-O3/-Os (the C source relies on signed-overflow wraparound,
#     so the Rust must match gcc at every optimization level)
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE="$PWD"
CSRC="$(cd .. && pwd)/c_src"
OUT="${TMPDIR:-/tmp}/verify"
mkdir -p "$OUT"
rc=0
step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; rc=1; }

# --- 0. enumerate feature combinations -------------------------------------
step "feature combinations"
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "  Cargo.toml declares no [features]; combinations = { default, --no-default-features }"
  COMBOS=("" "--no-default-features")
else
  COMBOS=("" "--no-default-features")
  for f in $FEATS; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

# --- 1. cargo check per combination ----------------------------------------
step "cargo check per feature combination"
for c in "${COMBOS[@]}"; do
  label="${c:-<default>}"
  if timeout 600 cargo check --offline $c >"$OUT/check.log" 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"; tail -20 "$OUT/check.log"
  fi
done

# --- 2. symbol parity -------------------------------------------------------
step "symbol parity (nm -D)"
timeout 600 cargo build --offline --release --lib --target-dir "$CRATE/target/so_build" \
  >"$OUT/build.log" 2>&1 || { bad "cargo build --lib"; tail -20 "$OUT/build.log"; }
cmake -S "$CSRC" -B "$CRATE/target/c_build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  >"$OUT/cmake.log" 2>&1 && cmake --build "$CRATE/target/c_build" >>"$OUT/cmake.log" 2>&1 \
  || { bad "cmake build"; tail -20 "$OUT/cmake.log"; }

CSO=$(find "$CRATE/target/c_build" -maxdepth 1 -name '*.so' | head -1)
RSO="$CRATE/target/so_build/release/libdataentry_lib.so"
nm -D --defined-only "$CSO" | awk '{print $3}' | sed 's/@.*//' | sort -u >"$OUT/c.syms"
nm -D --defined-only "$RSO" | awk '{print $3}' | sed 's/@.*//' | sort -u >"$OUT/r.syms"
echo "  C   exports: $(wc -l <"$OUT/c.syms")  ($CSO)"
echo "  Rust exports: $(wc -l <"$OUT/r.syms")  ($RSO)"
MISSING=$(comm -23 "$OUT/c.syms" "$OUT/r.syms")
if [ -z "$MISSING" ]; then ok "0 C symbols missing from the Rust .so"
else bad "missing from Rust .so:"; echo "$MISSING" | sed 's/^/      /'; fi

# --- 3. full suite per feature combination ---------------------------------
step "differential suite per feature combination"
for c in "${COMBOS[@]}"; do
  label="${c:-<default>}"
  export CDYLIB_FEATURE_ARGS="$c"
  if timeout 600 cargo test --offline $c -- --test-threads=4 >"$OUT/test.log" 2>&1; then
    ok "cargo test $label  ($(grep -c '^test .* ok$' "$OUT/test.log") tests)"
  else
    bad "cargo test $label"; grep -E '^(test .* FAILED|failures:|thread)' "$OUT/test.log" | head -30
  fi
done
unset CDYLIB_FEATURE_ARGS

# --- 4. full suite vs C built at each optimization level -------------------
step "differential suite vs C at each optimization level"
for opt in -O0 -O1 -O2 -O3 -Os; do
  bdir="$CRATE/target/c_opt$opt"
  rm -rf "$bdir"
  cmake -S "$CSRC" -B "$bdir" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" >"$OUT/cmake$opt.log" 2>&1 \
    && cmake --build "$bdir" >>"$OUT/cmake$opt.log" 2>&1 \
    || { bad "cmake build $opt"; tail -20 "$OUT/cmake$opt.log"; continue; }
  so=$(find "$bdir" -maxdepth 1 -name '*.so' | head -1)

  # confirm the flag really reached the compiler
  if ! grep -q -- "$opt" "$bdir/CMakeFiles"/*/flags.make 2>/dev/null; then
    echo "      (note: could not confirm $opt in flags.make)"
  fi

  if C_SO_OVERRIDE="$so" timeout 600 cargo test --offline -- --test-threads=4 \
       >"$OUT/test$opt.log" 2>&1; then
    ok "C built with $opt  ($(grep -c '^test .* ok$' "$OUT/test$opt.log") tests)"
  else
    bad "C built with $opt"
    grep -E '^(test .* FAILED|failures:|thread|.*DIVERGENCE)' "$OUT/test$opt.log" | head -30
  fi
done

step "result"
[ $rc -eq 0 ] && printf '\033[32mALL CHECKS PASSED\033[0m\n' || printf '\033[31mFAILURES PRESENT\033[0m\n'
exit $rc

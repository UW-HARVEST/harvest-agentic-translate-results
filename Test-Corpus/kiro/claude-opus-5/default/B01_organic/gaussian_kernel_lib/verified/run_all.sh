#!/usr/bin/env bash
# Full verification driver: builds both libraries, enumerates every feature
# combination from Cargo.toml, and runs the whole differential suite against
# both the release and the debug Rust .so.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "build C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
ok "C .so = $C_SO"

# ---------------------------------------------------------------------------
step "enumerate feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,"");split($0,a,"=");if(a[1]!="default")print a[1]}' \
    "$CRATE/Cargo.toml"
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  ok "no [features] table -> only the default configuration exists"
  COMBOS=("<default>" "--no-default-features" "--all-features")
else
  ok "features: ${FEATURES[*]}"
  COMBOS=("<default>" "--no-default-features" "--all-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  # pairwise
  n=${#FEATURES[@]}
  for ((i=0;i<n;i++)); do for ((j=i+1;j<n;j++)); do
    COMBOS+=("--no-default-features --features ${FEATURES[i]},${FEATURES[j]}")
  done; done
fi
printf '  combos: %s\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  flags=""; [ "$combo" != "<default>" ] && flags="$combo"
  if ( cd "$CRATE" && timeout 300 cargo check --all-targets $flags >/dev/null 2>&1 ); then
    ok "cargo check $combo"
  else
    bad "cargo check $combo"
  fi
done

# ---------------------------------------------------------------------------
step "symbol parity (nm -D)"
for combo in "${COMBOS[@]}"; do
  flags=""; [ "$combo" != "<default>" ] && flags="$combo"
  ( cd "$CRATE" && timeout 300 cargo build --release $flags >/dev/null 2>&1 ) \
    || { bad "release build $combo"; continue; }
  R_SO="$CRATE/target/release/libgaussian_kernel_lib.so"
  cdef=$(nm -D --defined-only "$C_SO" | awk '$2!="w"{print $3}' | sed 's/@.*//' | sort -u)
  rdef=$(nm -D --defined-only "$R_SO" | awk '$2!="w"{print $3}' | sed 's/@.*//' | sort -u)
  missing=$(comm -23 <(echo "$cdef") <(echo "$rdef"))
  if [ -z "$missing" ]; then
    ok "symbol diff empty for $combo ($(echo "$cdef" | wc -l) C symbol(s))"
  else
    bad "symbols missing from Rust .so for $combo: $(echo $missing)"
  fi
done

# ---------------------------------------------------------------------------
step "differential suite: every feature combination x {release, debug} Rust .so"
for combo in "${COMBOS[@]}"; do
  flags=""; [ "$combo" != "<default>" ] && flags="$combo"

  # release .so
  ( cd "$CRATE" && timeout 300 cargo build --release $flags >/dev/null 2>&1 )
  if ( cd "$CRATE" && RUST_SO_PATH="$CRATE/target/release/libgaussian_kernel_lib.so" \
        C_SO_PATH="$C_SO" timeout 600 cargo test --release $flags >"$CRATE"/target/h_test.log 2>&1 ); then
    ok "tests $combo [release .so]  $(grep -c '^test .* ok$' "$CRATE"/target/h_test.log) test(s) passed"
  else
    bad "tests $combo [release .so]"; tail -30 "$CRATE"/target/h_test.log
  fi

  # debug .so (different codegen must still be bit-identical to the C)
  ( cd "$CRATE" && timeout 300 cargo build $flags >/dev/null 2>&1 )
  if ( cd "$CRATE" && RUST_SO_PATH="$CRATE/target/debug/libgaussian_kernel_lib.so" \
        C_SO_PATH="$C_SO" timeout 600 cargo test --release $flags >"$CRATE"/target/h_test_dbg.log 2>&1 ); then
    ok "tests $combo [debug .so]    $(grep -c '^test .* ok$' "$CRATE"/target/h_test_dbg.log) test(s) passed"
  else
    bad "tests $combo [debug .so]"; tail -30 "$CRATE"/target/h_test_dbg.log
  fi
done

# ---------------------------------------------------------------------------
step "robustness: C rebuilt at -O2 and -O3 must also match the Rust"
for opt in -O2 -O3; do
  d="$ROOT/c_src/build_opt"
  rm -rf "$d"; mkdir -p "$d"
  ( cd "$d" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DCMAKE_C_FLAGS="$opt" >/dev/null && cmake --build . >/dev/null ) \
    || { bad "C build $opt"; continue; }
  OPT_SO="$(find "$d" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
  if ( cd "$CRATE" && C_SO_PATH="$OPT_SO" \
        RUST_SO_PATH="$CRATE/target/release/libgaussian_kernel_lib.so" \
        timeout 600 cargo test --release >"$CRATE"/target/h_test_opt.log 2>&1 ); then
    ok "tests vs C built with $opt"
  else
    bad "tests vs C built with $opt"; tail -30 "$CRATE"/target/h_test_opt.log
  fi
  rm -rf "$d"
done

# ---------------------------------------------------------------------------
step "summary"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$FAIL"

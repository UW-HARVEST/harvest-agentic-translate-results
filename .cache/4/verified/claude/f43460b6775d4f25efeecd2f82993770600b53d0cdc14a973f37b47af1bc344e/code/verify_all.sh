#!/usr/bin/env bash
# One-shot reproduction of the full verification described in VERIFICATION.md:
#
#   1. build the C reference artifacts (executable via cmake, library via gcc)
#   2. every cargo feature combination: cargo check + build + test
#   3. the release profile (driven through DRIVER_RUST_SO / DRIVER_RUST_BIN,
#      because [profile.release] panic = "abort" rules out `cargo test --release`)
#   4. the C library rebuilt at every optimisation level and with clang, driven
#      through DRIVER_C_SO
#   5. the nm -D symbol diff
set -uo pipefail

ROOT=$(cd "$(dirname "$0")" && pwd)
cd "$ROOT"
SCRATCH=${TMPDIR:-/tmp}/driver-verify
mkdir -p "$SCRATCH"

FAILED=0
step() { echo; echo "################ $* ################"; }
note() { echo "  -> $*"; }
fail() { echo "  !! FAIL: $*"; FAILED=1; }

# ---------------------------------------------------------------- 1. C artifacts
step "1. C reference artifacts"
( mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) \
  && note "cmake: c_src/build/driver" || fail "cmake build"
./build_c_so.sh >/dev/null && note "gcc: c_build/libcdecisions.so" || fail "build_c_so.sh"

# ------------------------------------------------- 2. every feature combination
step "2. feature combinations"
if ./check_features.sh >"$SCRATCH/features.log" 2>&1; then
  note "$(grep -c '^test result: ok' "$SCRATCH/features.log") passing test binaries across all combos"
else
  fail "check_features.sh (see $SCRATCH/features.log)"
  tail -n 30 "$SCRATCH/features.log"
fi

# ------------------------------------------------------------- 3. release profile
step "3. release profile"
if timeout 600 cargo build --offline --release >/dev/null 2>&1; then
  if DRIVER_RUST_SO="$ROOT/target/release/libdriver.so" \
     DRIVER_RUST_BIN="$ROOT/target/release/driver" \
     timeout 600 cargo test --offline >"$SCRATCH/release.log" 2>&1; then
    note "release artifacts pass the full suite"
  else
    fail "release suite (see $SCRATCH/release.log)"
    grep -E "FAILED|panicked|mismatch" "$SCRATCH/release.log" | head -n 20
  fi
else
  fail "cargo build --release"
fi

# ------------------------------------------- 4. C at other optimisation levels
step "4. C compiler / optimisation sweep"
sweep() { # $1 = compiler, $2 = flag
  local cc=$1 opt=$2 so="$SCRATCH/${1}-${2}.so"
  command -v "$cc" >/dev/null 2>&1 || { note "$cc not installed, skipping"; return; }
  if ! "$cc" "-$opt" -shared -fPIC -o "$so" c_src/src/lib.c 2>/dev/null; then
    fail "$cc -$opt compile"
    return
  fi
  if DRIVER_C_SO="$so" timeout 600 cargo test --offline \
       --test differential --test error_paths --test symbols \
       >"$SCRATCH/${cc}-${opt}.log" 2>&1; then
    note "$cc -$opt: OK"
  else
    fail "$cc -$opt (see $SCRATCH/${cc}-${opt}.log)"
    grep -E "FAILED|mismatch" "$SCRATCH/${cc}-${opt}.log" | head -n 10
  fi
}
for opt in O0 O1 O2 O3 Os; do sweep gcc "$opt"; done
for opt in O0 O2; do sweep clang "$opt"; done

# ------------------------------------------------------------- 5. symbol parity
step "5. nm -D symbol parity"
for profile in debug release; do
  rso="target/$profile/libdriver.so"
  [ -f "$rso" ] || { note "$rso not built, skipping"; continue; }
  missing=$(comm -23 \
    <(nm -D --defined-only c_build/libcdecisions.so | awk '$2~/^[TDBRGS]$/{print $3}' | sort) \
    <(nm -D --defined-only "$rso"                   | awk '$2~/^[TDBRGS]$/{print $3}' | sort))
  if [ -z "$missing" ]; then
    note "$profile: 0 missing symbols"
  else
    fail "$profile is missing: $missing"
  fi
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "================ VERIFICATION PASSED ================"
else
  echo "================ VERIFICATION FAILED ================"
fi
exit "$FAILED"

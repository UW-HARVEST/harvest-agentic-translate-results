#!/usr/bin/env bash
# Phase D driver: rebuild everything from scratch, then run every phase across
# every feature combination and both cargo profiles, checking `nm -D` symbol
# parity for each.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library (CMake, exactly as shipped)"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "C .so built" || bad "C .so build"
C_SO=$(ls c_src/build/*.so)
echo "C .so: $C_SO"

step "Build the static-helper harness (harness/wrap.c, -fPIC only, matching CMake's C_FLAGS)"
gcc -fPIC -shared -I c_src/include -o harness/libcwrap.so harness/wrap.c \
  && ok "harness .so built" || bad "harness .so build"

# ---------------------------------------------------------------------------
# Feature combinations. The crate declares one optional feature
# (`test_internals`) and an empty default set, so the full power set is:
COMBOS=( "--no-default-features" "" "--features test_internals" \
         "--no-default-features --features test_internals" )

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"

  step "cargo check  [$label]"
  ( cd translation && timeout 600 cargo check $combo 2>&1 | tail -3 ) \
    && ok "check $label" || bad "check $label"

  for profile in "" "--release"; do
    pdir=$([ -n "$profile" ] && echo release || echo debug)

    # NOTE: `cargo test` does not emit the cdylib artifact, so the test harness
    # builds it itself into target/xverify for BOTH profiles (see
    # tests/common/mod.rs::rust_lib_paths). Nothing here depends on a stale .so.
    step "cargo test $profile  [$label]"
    ( cd translation && timeout 600 cargo test $profile $combo --no-fail-fast 2>&1 \
        | grep -E '^(test result|running|error|test .* FAILED)' ) \
      && ok "test $pdir $label" || bad "test $pdir $label"

    step "nm -D symbol parity, $pdir  [$label]"
    ( cd translation && timeout 600 cargo build $profile $combo >/dev/null 2>&1 )
    R_SO="translation/target/$pdir/libcall_predict_lib.so"
    if [ ! -f "$R_SO" ]; then bad "missing $R_SO"; continue; fi
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))
    if [ -z "$missing" ]; then
      ok "0 C symbols missing from Rust .so ($pdir, $label)"
    else
      bad "C symbols missing from Rust .so ($pdir, $label): $missing"
    fi
    # Undefined symbols in the Rust .so must all be libc / libgcc-unwind.
    stray=$(nm -D -u "$R_SO" | awk '{print $2}' \
      | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_' || true)
    if [ -z "$stray" ]; then
      ok "0 undefined non-libc symbols ($pdir, $label)"
    else
      bad "undefined non-libc symbols ($pdir, $label): $stray"
    fi
  done
done

# ---------------------------------------------------------------------------
step "Default-build exported-surface must be EXACTLY the C library's"
( cd translation && timeout 600 cargo build --release >/dev/null 2>&1 )
diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libcall_predict_lib.so \
        | awk '{print $3}' | sort) \
  && ok "exported symbol sets identical" || bad "exported symbol sets differ"

step "No binary executable to diff"
grep -q 'add_executable' c_src/CMakeLists.txt \
  && bad "c_src builds an executable -- stdout must be compared" \
  || ok "c_src builds only a SHARED library; no driver binary exists"
grep -q '\[\[bin\]\]' translation/Cargo.toml \
  && bad "translation declares a [[bin]]" \
  || ok "translation declares only a cdylib"

# ---------------------------------------------------------------------------
printf '\n=========================================\n'
if [ "$fail" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
printf '=========================================\n'
exit "$fail"

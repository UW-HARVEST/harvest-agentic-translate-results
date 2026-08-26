#!/usr/bin/env bash
# Full verification driver: Phase A artifacts -> Phase B/C differential tests ->
# Phase D symbol parity, for every build configuration.
#
# Usage: ./verify.sh            (everything)
#        ./verify.sh --quick    (skip the release profile and the -O2 C build)
set -uo pipefail
cd "$(dirname "$0")"

QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

CARGO="cargo --offline"
mkdir -p target/verify_logs
FAILURES=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------------------
# Phase A.3 — enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
step "Feature combinations declared in Cargo.toml"
mapfile -t FEATURES < <(awk '
  /^\[features\]/ { inf = 1; next }
  /^\[/           { inf = 0 }
  inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]); if (a[1] != "") print a[1] }
' Cargo.toml)
echo "   features found: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Power set of the feature list -> one combo string per line ("" = no features).
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  new=()
  for c in "${COMBOS[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS=("${new[@]}")
done
echo "   ${#COMBOS[@]} combination(s) to verify:"
for c in "${COMBOS[@]}"; do echo "     - '${c}'  (--no-default-features --features '${c}')"; done

# ---------------------------------------------------------------------------
# Phase A.2 — cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
for c in "${COMBOS[@]}"; do
  if $CARGO check --no-default-features --features "$c" --all-targets >target/verify_logs/check.log 2>&1; then
    ok "cargo check --no-default-features --features '${c}'"
  else
    bad "cargo check --no-default-features --features '${c}'"; tail -20 target/verify_logs/check.log
  fi
done
for extra in "" "--all-features"; do
  if $CARGO check $extra --all-targets >target/verify_logs/check.log 2>&1; then
    ok "cargo check ${extra:-(default features)}"
  else
    bad "cargo check ${extra:-(default features)}"; tail -20 target/verify_logs/check.log
  fi
done

# ---------------------------------------------------------------------------
# Build the C side (never touching c_src/)
# ---------------------------------------------------------------------------
step "Build the C shared object and executable"
mkdir -p target/c_build
CSRC=(c_src/src/sillymain.c c_src/src/main.c)
gcc -shared -fPIC -o target/c_build/libcdriver.so "${CSRC[@]}"  && ok "libcdriver.so (default flags)" || bad "libcdriver.so"
gcc -o target/c_build/driver_c "${CSRC[@]}"                     && ok "driver_c executable"          || bad "driver_c"
if [ "$QUICK" = 0 ]; then
  gcc -shared -fPIC -O2 -o target/c_build/libcdriver_O2.so "${CSRC[@]}" && ok "libcdriver_O2.so" || bad "libcdriver_O2.so"
  gcc -O2 -o target/c_build/driver_c_O2 "${CSRC[@]}"                    && ok "driver_c_O2"      || bad "driver_c_O2"
fi
# The sanctioned CMake build, out-of-source so c_src/ stays untouched.
if cmake -S c_src -B target/cmake_build -DCMAKE_POSITION_INDEPENDENT_CODE=ON >target/verify_logs/cmake.log 2>&1 &&
   cmake --build target/cmake_build >>target/verify_logs/cmake.log 2>&1; then
  ok "cmake build of c_src (executable target 'driver')"
else
  bad "cmake build of c_src"; tail -20 target/verify_logs/cmake.log
fi

# ---------------------------------------------------------------------------
# Build the Rust side (cargo test does not build cdylibs, so build explicitly)
# ---------------------------------------------------------------------------
step "Build the Rust cdylib and binary"
$CARGO build           >/dev/null 2>&1 && ok "cargo build (debug)"   || bad "cargo build (debug)"
if [ "$QUICK" = 0 ]; then
  $CARGO build --release >/dev/null 2>&1 && ok "cargo build (release)" || bad "cargo build (release)"
fi

# ---------------------------------------------------------------------------
# Phase D — symbol parity
# ---------------------------------------------------------------------------
step "Phase D: symbol parity (nm -D)"
nm -D --defined-only target/c_build/libcdriver.so | awk '{print $3}' | sort > target/c_syms.txt
check_syms() {
  local so="$1"
  nm -D --defined-only "$so" | awk '{print $3}' | sort > target/rust_syms.txt
  local missing
  missing=$(comm -23 target/c_syms.txt target/rust_syms.txt)
  if [ -z "$missing" ]; then
    ok "$so exports all $(wc -l < target/c_syms.txt) C symbols: $(paste -sd' ' target/c_syms.txt)"
  else
    bad "$so is missing C symbols:"; echo "$missing"
  fi
  local undef
  undef=$(nm -D -u "$so" | awk '{print $NF}' |
          grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^$' || true)
  if [ -z "$undef" ]; then
    ok "$so has no undefined non-libc symbols"
  else
    bad "$so has undefined non-libc symbols:"; echo "$undef"
  fi
}
check_syms target/debug/libdriver.so
[ "$QUICK" = 0 ] && check_syms target/release/libdriver.so

# ---------------------------------------------------------------------------
# Phases B + C — differential tests, for every configuration
# ---------------------------------------------------------------------------
run_suite() {
  local label="$1"; shift
  if env "$@" $CARGO test --no-default-features --features "$COMBO" \
        -- --test-threads=1 >target/verify_logs/test.log 2>&1; then
    ok "$label ($(grep -c '^test .* ok$' target/verify_logs/test.log) tests)"
  else
    bad "$label"; grep -E "^test .*(FAILED|panicked)|assertion|test result" target/verify_logs/test.log | head -30
  fi
}

step "Phases B + C: differential tests per configuration"
for COMBO in "${COMBOS[@]}"; do
  run_suite "features='${COMBO}' | rust=debug   | c=default" DUMMY=1
  if [ "$QUICK" = 0 ]; then
    run_suite "features='${COMBO}' | rust=release | c=default" \
      DRIVER_RUST_SO="$PWD/target/release/libdriver.so" \
      DRIVER_RUST_EXE="$PWD/target/release/driver"
    run_suite "features='${COMBO}' | rust=debug   | c=-O2" \
      DRIVER_C_SO="$PWD/target/c_build/libcdriver_O2.so" \
      DRIVER_C_EXE="$PWD/target/c_build/driver_c_O2"
    run_suite "features='${COMBO}' | rust=release | c=-O2" \
      DRIVER_RUST_SO="$PWD/target/release/libdriver.so" \
      DRIVER_C_SO="$PWD/target/c_build/libcdriver_O2.so" \
      DRIVER_C_EXE="$PWD/target/c_build/driver_c_O2"
  fi
done

# ---------------------------------------------------------------------------
# Harness self-check: the tests must actually be able to fail
# ---------------------------------------------------------------------------
if [ "$QUICK" = 0 ]; then
  step "Negative control (the suite must reject a knowingly wrong translation)"
  ./negative_control.sh && ok "negative control detected the divergence" \
                        || bad "negative control did NOT detect the divergence"
fi

# ---------------------------------------------------------------------------
step "Summary"
if [ "$FAILURES" = 0 ]; then
  printf '   \033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '   \033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILURES"
fi
exit "$FAILURES"

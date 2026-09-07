#!/usr/bin/env bash
# Phase D driver: symbol parity + the full differential suite under every
# build configuration (feature combination x build profile).
#
#   ./verify_all.sh            # everything
#   FAST=1 ./verify_all.sh     # skip the debug-profile .so pass
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE=$PWD
ROOT=$(cd .. && pwd)
TMP=${TMPDIR:-/tmp}
fail=0
step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# --------------------------------------------------------------- build the C
step "build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
ok "C .so = $C_SO"

# ------------------------------------------------ enumerate feature combos
# Every subset of the declared [features] plus the two "no features" spellings.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml
)
COMBOS=("default")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((b=0; b<n; b++)); do (( mask & (1<<b) )) && sel+=("${FEATURES[$b]}"); done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
else
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
fi
step "feature combinations to verify (${#COMBOS[@]})"
printf '  %s\n' "${COMBOS[@]}"

PROFILES=(release)
[ "${FAST:-0}" = "1" ] || PROFILES+=(debug)

for combo in "${COMBOS[@]}"; do
  [ "$combo" = "default" ] && flags=() || read -r -a flags <<< "$combo"

  step "cargo check   [$combo]"
  if cargo check --offline "${flags[@]}" >"$TMP/check.log" 2>&1; then
    ok "cargo check [$combo]"
  else
    bad "cargo check [$combo]"; tail -20 "$TMP/check.log"; continue
  fi

  for prof in "${PROFILES[@]}"; do
    pflag=(); [ "$prof" = release ] && pflag=(--release)

    step "cargo build --$prof   [$combo]"
    if cargo build --offline "${pflag[@]}" "${flags[@]}" >"$TMP/build.log" 2>&1; then
      ok "cargo build --$prof [$combo]"
    else
      bad "cargo build --$prof [$combo]"; tail -20 "$TMP/build.log"; continue
    fi
    R_SO="$CRATE/target/$prof/libspec_ray_lib.so"
    [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

    # ------------------------------------------------------ symbol parity
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u >"$TMP/c_syms.txt"
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u >"$TMP/r_syms.txt"
    missing=$(comm -23 "$TMP/c_syms.txt" "$TMP/r_syms.txt")
    if [ -z "$missing" ]; then
      ok "symbol parity [$combo/$prof]: $(wc -l <"$TMP/c_syms.txt") C symbols, 0 missing"
    else
      bad "symbol parity [$combo/$prof]: missing ->"; printf '      %s\n' $missing
    fi
    # No undefined non-libc symbols.  Rather than pattern-matching names, ask
    # the dynamic loader to resolve every relocation: anything left over is a
    # genuinely missing symbol.  (The Rust cdylib legitimately imports glibc,
    # libm and the libgcc unwinder; those all resolve.)
    undef=$(ldd -r "$R_SO" 2>&1 | grep -i 'undefined symbol' || true)
    if [ -z "$undef" ]; then
      nlibs=$(objdump -p "$R_SO" | awk '/NEEDED/{print $2}' | tr '\n' ' ')
      ok "no undefined symbols [$combo/$prof] (imports resolve from: $nlibs)"
    else
      bad "undefined symbols [$combo/$prof]:"; printf '      %s\n' "$undef"
    fi

    # -------------------------------------------------------- differential
    step "differential suite  [$combo/$prof .so]"
    if RUST_SO="$R_SO" timeout 600 cargo test --offline "${flags[@]}" \
         -- --test-threads=8 >"$TMP/test.log" 2>&1; then
      ok "tests [$combo/$prof]: $(grep -c '^test .* ok$' "$TMP/test.log") test fns passed"
    else
      bad "tests [$combo/$prof]"; grep -E 'DIVERGENCE|panicked|FAILED|^test result' -A4 "$TMP/test.log" | head -60
    fi
  done
done

# ------------------------------------------------------- driver executable?
step "binary/driver stdout comparison"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "c_src declares add_executable but no stdout comparison is implemented"
else
  ok "N/A: c_src/CMakeLists.txt builds only a SHARED library (no executable)"
fi

step "RESULT"
[ "$fail" = 0 ] && { echo "ALL CHECKS PASSED"; exit 0; } || { echo "SOME CHECKS FAILED"; exit 1; }

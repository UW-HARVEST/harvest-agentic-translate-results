#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + both codegen
# profiles. Nothing here is hand-repeated; the combination list is derived from
# Cargo.toml.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$PWD
C_SO=$(echo "$ROOT"/../c_src/build/*.so)
RC=0

# ---- enumerate feature combinations from Cargo.toml -----------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{
        sub(/[[:space:]]*=.*/,""); print}' Cargo.toml
)
echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

CONFIGS=("default:")
CONFIGS+=("no-default:--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  # power set of the declared features, built with --no-default-features
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((b=0; b<n; b++)); do
      if (( mask & (1<<b) )); then combo="$combo,${FEATURES[$b]}"; fi
    done
    combo=${combo#,}
    CONFIGS+=("$combo:--no-default-features --features $combo")
  done
  CONFIGS+=("all:--all-features")
fi

# ---- symbol parity -------------------------------------------------------
syms() { nm -D --defined-only "$1" | awk '{print $3}' | grep -v '^_' | sort -u; }
check_symbols() {
  local so=$1 label=$2
  local missing
  missing=$(comm -23 <(syms "$C_SO") <(syms "$so"))
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY [$label]: MISSING FROM RUST:"; echo "$missing" | sed 's/^/    /'
    return 1
  fi
  # no undefined non-libc references (i.e. nothing left un-translated)
  local undef
  undef=$(nm -D --undefined-only "$so" | awk '{print $2}' \
          | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^__cxa_\|^_Unwind_' || true)
  if [ -n "$undef" ]; then
    echo "  SYMBOL PARITY [$label]: UNDEFINED NON-LIBC SYMBOLS:"; echo "$undef" | sed 's/^/    /'
    return 1
  fi
  echo "  symbol parity [$label]: OK ($(syms "$C_SO" | wc -l) C symbol(s), 0 missing, 0 undefined non-libc)"
}

# ---- run every configuration --------------------------------------------
for entry in "${CONFIGS[@]}"; do
  label=${entry%%:*}
  flags=${entry#*:}
  echo "=== configuration: $label  (cargo $flags) ==="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $flags >/dev/null 2>&1; then
    echo "  cargo check FAILED"; RC=1; continue
  fi

  for profile in release debug; do
    pflag=""; [ "$profile" = release ] && pflag="--release"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $pflag $flags >/dev/null 2>&1; then
      echo "  cargo build ($profile) FAILED"; RC=1; continue
    fi
    so="$ROOT/target/$profile/libdequantize_granule_lib.so"
    check_symbols "$so" "$label/$profile" || RC=1
    # shellcheck disable=SC2086
    if RUST_SO="$so" timeout 600 cargo test --release $flags \
         --test phase_b_configs --test phase_c_errors >/tmp/pd_${label}_${profile}.log 2>&1; then
      echo "  differential suite [$label/$profile]: PASS"
    else
      echo "  differential suite [$label/$profile]: FAIL"
      grep -E "^test .*FAILED|panicked at|test result" /tmp/pd_${label}_${profile}.log | head -20 | sed 's/^/    /'
      RC=1
    fi
  done
done

echo
if [ $RC -eq 0 ]; then echo "PHASE D: ALL CONFIGURATIONS PASS"; else echo "PHASE D: FAILURES PRESENT"; fi
exit $RC

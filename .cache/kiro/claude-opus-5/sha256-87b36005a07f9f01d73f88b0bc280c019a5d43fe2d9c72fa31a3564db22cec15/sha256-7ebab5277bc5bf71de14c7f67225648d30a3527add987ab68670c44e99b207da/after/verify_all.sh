#!/usr/bin/env bash
# Verify the translation for every valid build-time configuration.
#
#  * enumerates the powerset of [features] from Cargo.toml (plus the default
#    build) - the crate currently declares none, so the sweep degenerates to
#    the default build and --no-default-features;
#  * cargo check / build / test for each combination;
#  * diffs the exported dynamic symbols of the C .so against the Rust .so.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
TIMEOUT=${TIMEOUT:-600}
fail=0

# --------------------------------------------------------------- C shared lib
if [ ! -d "$CSRC/build" ]; then
  ( cd "$CSRC" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(find "$CSRC/build" -maxdepth 1 -name '*.so' | head -1)"
[ -n "$C_SO" ] || { echo "no C .so found"; exit 1; }
echo "C library: $C_SO"

# ------------------------------------------------------- enumerate feature sets
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0,a,"="); gsub(/[ \t"]/,"",a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' "$CRATE/Cargo.toml"
)
n=${#FEATURES[@]}
echo "declared features: $n ${FEATURES[*]-}"

COMBOS=()
if [ "$n" -eq 0 ]; then
  COMBOS+=("")                       # default build (no features exist)
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then set="${set:+$set,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$set")
  done
fi

# ------------------------------------------------------------------ the sweep
declare -a EXPECT_SYMS
mapfile -t EXPECT_SYMS < <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort)
echo "C exports ${#EXPECT_SYMS[@]} symbols"

check_symbols() {
  local rs_so="$1" label="$2"
  local missing=()
  local rs_syms
  rs_syms="$(nm -D --defined-only "$rs_so" | awk '{print $3}' | sort -u)"
  for s in "${EXPECT_SYMS[@]}"; do
    grep -qx -- "$s" <<<"$rs_syms" || missing+=("$s")
  done
  if [ ${#missing[@]} -ne 0 ]; then
    echo "  [$label] MISSING EXPORTS: ${missing[*]}"
    return 1
  fi
  # object sizes must agree as well (callers may rely on sizeof)
  local diff
  diff="$(comm -3 \
    <(readelf --dyn-syms -W "$C_SO"  | awk '$5=="GLOBAL"&&$7!="UND"&&$4=="OBJECT"{print $8" "$3}' | sort) \
    <(readelf --dyn-syms -W "$rs_so" | awk '$5=="GLOBAL"&&$7!="UND"&&$4=="OBJECT"{print $8" "$3}' | sort))"
  if [ -n "$diff" ]; then
    echo "  [$label] OBJECT SIZE MISMATCH:"; echo "$diff"; return 1
  fi
  echo "  [$label] all ${#EXPECT_SYMS[@]} symbols present, object sizes match"
}

cd "$CRATE"
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="default"
    FLAGS=()
    NDF=(--no-default-features)
  else
    label="$combo"
    FLAGS=(--no-default-features --features "$combo")
    NDF=()
  fi

  for profile in debug release; do
    PROF=()
    [ "$profile" = release ] && PROF=(--release)
    tag="$label/$profile"

    echo "=== cargo check [$tag]"
    timeout "$TIMEOUT" cargo check "${PROF[@]}" "${FLAGS[@]}" --all-targets \
      >/tmp/check.$$.log 2>&1 || { echo "  CHECK FAILED"; tail -25 /tmp/check.$$.log; fail=1; continue; }

    echo "=== cargo build [$tag]"
    timeout "$TIMEOUT" cargo build "${PROF[@]}" "${FLAGS[@]}" \
      >/tmp/build.$$.log 2>&1 || { echo "  BUILD FAILED"; tail -25 /tmp/build.$$.log; fail=1; continue; }

    check_symbols "target/$profile/libconvert_pix_lib.so" "$tag" || fail=1

    echo "=== cargo test [$tag]"
    CONVERT_PIX_SO="$CRATE/target/$profile/libconvert_pix_lib.so" \
    CONVERT_PIX_FEATURES="$combo" \
    timeout "$TIMEOUT" cargo test "${PROF[@]}" "${FLAGS[@]}" \
      >/tmp/test.$$.log 2>&1 || { echo "  TEST FAILED"; grep -E "^test |panicked|differs|diff at" /tmp/test.$$.log | head -30; fail=1; continue; }
    grep -hE "test result:" /tmp/test.$$.log | sed 's/^/  /'
  done

  # also confirm the crate builds with default features explicitly disabled
  if [ ${#NDF[@]} -ne 0 ]; then
    echo "=== cargo check [--no-default-features]"
    timeout "$TIMEOUT" cargo check "${NDF[@]}" --all-targets >/tmp/ndf.$$.log 2>&1 \
      || { echo "  CHECK FAILED"; tail -25 /tmp/ndf.$$.log; fail=1; }
  fi
done

rm -f /tmp/check.$$.log /tmp/build.$$.log /tmp/test.$$.log /tmp/ndf.$$.log
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS OK"; else echo "FAILURES PRESENT"; fi
exit "$fail"

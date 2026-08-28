#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust cdylib, checks exported
# symbol parity for every build configuration, and runs the differential test
# suite for every feature combination in both the dev and release profiles.
#
# Usage:  ./verify.sh
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
work="$here/target/tmp"
mkdir -p "$work"
rc=0
fail() { echo "FAIL: $*"; rc=1; }

echo "=============================================================="
echo " 1. building the C shared object"
echo "=============================================================="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$work/cmake.log" 2>&1 \
  && cmake --build . >>"$work/cmake.log" 2>&1 ) \
  || { tail -30 "$work/cmake.log"; fail "the C build failed"; exit 1; }
c_so="$(ls "$root"/c_src/build/*.so | head -1)"
echo "C  .so: $c_so"

# ---------------------------------------------------------------------------
# Enumerate the feature combinations declared in Cargo.toml.  This crate has no
# [features] table, so the matrix is {default, --no-default-features}; the loop
# below picks up any features that are added later automatically.
# ---------------------------------------------------------------------------
mapfile -t features < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "default" && $0 != "") print}' \
    "$here/Cargo.toml"
)
combos=("default" "no-default")
for f in "${features[@]:-}"; do
  [ -n "$f" ] && combos+=("no-default+$f")
done

echo
echo "=============================================================="
echo " 2. feature combinations to verify: ${combos[*]}"
echo "=============================================================="

for profile in release dev; do
  if [ "$profile" = release ]; then
    pflag=(--release); pdir=release
  else
    pflag=(); pdir=debug
  fi

  for combo in "${combos[@]}"; do
    case "$combo" in
      default)     fflag=() ;;
      no-default)  fflag=(--no-default-features) ;;
      no-default+*) fflag=(--no-default-features --features "${combo#no-default+}") ;;
    esac

    echo
    echo "--------------------------------------------------------------"
    echo " profile=$profile  features=$combo"
    echo "--------------------------------------------------------------"

    cargo build --offline "${pflag[@]}" "${fflag[@]}" >"$work/build.log" 2>&1 \
      || { tail -30 "$work/build.log"; fail "cargo build ($profile/$combo)"; continue; }

    r_so="$here/target/$pdir/libgjk_cache_lib.so"
    [ -f "$r_so" ] || { fail "missing $r_so"; continue; }

    # --- symbol parity -----------------------------------------------------
    nm -D --defined-only "$c_so" | awk '{print $3}' | sort >"$work/c_syms.txt"
    nm -D --defined-only "$r_so" | awk '{print $3}' | sort >"$work/r_syms.txt"
    missing="$(comm -23 "$work/c_syms.txt" "$work/r_syms.txt")"
    extra="$(comm -13 "$work/c_syms.txt" "$work/r_syms.txt")"
    printf 'symbols: C=%s Rust=%s\n' \
      "$(wc -l <"$work/c_syms.txt")" "$(wc -l <"$work/r_syms.txt")"
    if [ -n "$missing" ]; then
      echo "MISSING FROM RUST:"; echo "$missing"; fail "symbol parity ($profile/$combo)"
    else
      echo "symbol parity: OK (0 missing)"
    fi
    [ -n "$extra" ] && { echo "extra in Rust (informational):"; echo "$extra"; }

    # non-libc undefined symbols in the Rust .so
    undef="$(nm -D --undefined-only "$r_so" | awk '{print $2}' \
             | grep -v '@GLIBC\|@GCC\|_ITM_\|__gmon_start__\|__cxa_thread_atexit_impl' || true)"
    if [ -n "$undef" ]; then
      echo "UNRESOLVED NON-LIBC SYMBOLS:"; echo "$undef"; fail "undefined symbols ($profile/$combo)"
    else
      echo "undefined non-libc symbols: none"
    fi

    # --- differential tests ------------------------------------------------
    if timeout 600 cargo test --offline "${pflag[@]}" "${fflag[@]}" \
         >"$work/test-$profile-$combo.log" 2>&1; then
      grep -h 'test result:' "$work/test-$profile-$combo.log" \
        | awk '{p+=$4; f+=$6} END {printf "tests: %d passed, %d failed\n", p, f}'
    else
      tail -40 "$work/test-$profile-$combo.log"
      fail "cargo test ($profile/$combo)"
    fi
  done
done

echo
echo "=============================================================="
if [ "$rc" -eq 0 ]; then
  echo " ALL CONFIGURATIONS VERIFIED"
else
  echo " VERIFICATION FAILED"
fi
echo "=============================================================="
exit "$rc"

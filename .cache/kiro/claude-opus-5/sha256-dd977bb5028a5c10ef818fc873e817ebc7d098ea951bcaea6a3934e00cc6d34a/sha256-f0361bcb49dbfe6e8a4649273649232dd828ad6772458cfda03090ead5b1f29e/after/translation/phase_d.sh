#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + both profiles.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
fail=0

echo "############ Phase D.1 — build both libraries ############"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so="$(ls "$root"/c_src/build/lib*.so)"
( cd "$here" && timeout 600 cargo build -q && timeout 600 cargo build -q --release ) \
  || { echo "Rust build FAILED"; exit 1; }
echo "C  : $c_so"
echo "RS : $here/target/{debug,release}/libstr_dups_lib.so"

echo
echo "############ Phase D.2 — symbol parity (nm -D) ############"
for prof in debug release; do
  rs_so="$here/target/$prof/libstr_dups_lib.so"
  echo "--- $prof ---"
  cdef=$(nm -D --defined-only "$c_so"  | awk '$2=="T"{print $3}' | sort)
  rdef=$(nm -D --defined-only "$rs_so" | awk '$2=="T"{print $3}' | sort)
  if diff <(echo "$cdef") <(echo "$rdef") >/dev/null; then
    echo "defined-symbol diff EMPTY ($(echo "$cdef" | wc -l) symbols)"
  else
    echo "DEFINED-SYMBOL DIFF NON-EMPTY:"; diff <(echo "$cdef") <(echo "$rdef"); fail=1
  fi
  # Undefined (imported) symbols in the Rust .so that are NOT libc / libgcc /
  # link-time weak stubs. Anything left would be a genuinely missing definition.
  undef=$(nm -D --undefined-only "$rs_so" | awk '{print $NF}' \
    | grep -v '@GLIBC' \
    | grep -vE '^_Unwind_[A-Za-z]+@GCC_[0-9.]+$' \
    | grep -vE '^(_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__gmon_start__)$' || true)
  if [ -z "$undef" ]; then
    echo "undefined non-libc symbols: NONE"
  else
    echo "UNEXPECTED UNDEFINED SYMBOLS:"; echo "$undef"; fail=1
  fi
done

echo
echo "############ Phase D.3 — feature combinations ############"
# Enumerate every feature declared in Cargo.toml.
features=$(sed -n '/^\[features\]/,/^\[/p' "$here/Cargo.toml" \
           | grep -E '^[a-zA-Z0-9_-]+[[:space:]]*=' | cut -d= -f1 | tr -d ' ' || true)
if [ -z "$features" ]; then
  echo "Cargo.toml declares NO [features] -> exactly one build configuration."
  combos=("<default>")
else
  echo "features: $features"
  combos=("<default>")
  for f in $features; do combos+=("$f"); done
fi

run_suite () {  # $1 = label, $2... = extra cargo args
  local label="$1"; shift
  echo "--- cargo test [$label] ---"
  if ( cd "$here" && timeout 600 cargo test -q "$@" -- --test-threads=1 ) >/tmp/pd_$$.log 2>&1; then
    grep -E "test result" /tmp/pd_$$.log | sed 's/^/    /'
  else
    echo "    FAILED [$label]"; tail -40 /tmp/pd_$$.log | sed 's/^/    /'; fail=1
  fi
  rm -f /tmp/pd_$$.log
}

run_suite "default features"
run_suite "no-default-features" --no-default-features
for f in $features; do
  ( cd "$here" && timeout 600 cargo check -q --no-default-features --features "$f" ) \
    || { echo "cargo check FAILED for feature $f"; fail=1; }
  run_suite "features=$f" --no-default-features --features "$f"
done

echo
echo "############ Phase D.4 — release-profile .so under test ############"
echo "--- cargo test against target/release/libstr_dups_lib.so ---"
if ( cd "$here" && RUST_SO="$here/target/release/libstr_dups_lib.so" \
     timeout 600 cargo test -q -- --test-threads=1 ) >/tmp/pdr_$$.log 2>&1; then
  grep -E "test result" /tmp/pdr_$$.log | sed 's/^/    /'
else
  echo "    FAILED (release .so)"; tail -40 /tmp/pdr_$$.log | sed 's/^/    /'; fail=1
fi
rm -f /tmp/pdr_$$.log

echo
echo "############ Phase D.5 — binary executable diff ############"
if grep -q '^\[\[bin\]\]' "$here/Cargo.toml" || [ -f "$here/src/main.rs" ]; then
  echo "Rust declares a binary target -> stdout diff required"; fail=1
else
  echo "no [[bin]] / src/main.rs in the crate"
fi
if grep -qE 'add_executable' "$root/c_src/CMakeLists.txt"; then
  echo "CMakeLists declares an executable -> stdout diff required"; fail=1
else
  echo "c_src/CMakeLists.txt builds only add_library(... SHARED) -> nothing to diff"
fi

echo
if [ "$fail" -eq 0 ]; then echo "########## PHASE D: ALL CHECKS PASSED ##########"; else
  echo "########## PHASE D: FAILURES PRESENT ##########"; fi
exit "$fail"

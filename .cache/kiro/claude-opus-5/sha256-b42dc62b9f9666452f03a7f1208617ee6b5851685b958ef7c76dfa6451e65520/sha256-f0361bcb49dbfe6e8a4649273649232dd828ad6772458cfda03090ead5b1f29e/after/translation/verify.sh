#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff their exported symbols, then run
# the full differential suite under EVERY Cargo feature combination.
#
#   ./verify.sh
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
fail=0

echo "=============================================================="
echo " 1. build the C shared library"
echo "=============================================================="
( cd "$root/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$here/target/c-build.log" 2>&1 \
  || { echo "C BUILD FAILED"; tail -20 "$here/target/c-build.log"; exit 1; }
c_so="$(find "$root/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
echo "C   .so: $c_so"

echo
echo "=============================================================="
echo " 2. enumerate Cargo feature combinations"
echo "=============================================================="
# Every declared feature (excluding the implicit `default`).
mapfile -t features < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$here/Cargo.toml"
)
n=${#features[@]}
echo "declared features: ${n} ${features[*]:-(none)}"

# combos[] holds the extra cargo flags for each configuration to test.
combos=()
labels=()
combos+=("");                                labels+=("default")
if [ "$n" -gt 0 ]; then
  combos+=("--no-default-features");          labels+=("no-default-features")
  # full power set of the declared features, with default features off
  total=$(( 1 << n ))
  for (( m=1; m<total; m++ )); do
    set=""
    for (( i=0; i<n; i++ )); do
      if (( (m >> i) & 1 )); then set="${set:+$set,}${features[$i]}"; fi
    done
    combos+=("--no-default-features --features $set"); labels+=("$set")
  done
  combos+=("--all-features");                 labels+=("all-features")
fi
echo "configurations to verify: ${#combos[@]}"

echo
echo "=============================================================="
echo " 3. per-configuration: build cdylib, diff symbols, run tests"
echo "=============================================================="
c_syms="$here/target/c.syms"
nm -D --defined-only "$c_so" | awk '$2=="T"||$2=="t"{print $3}' | sort -u >"$c_syms"
echo "C exports $(wc -l <"$c_syms") symbols"

for idx in "${!combos[@]}"; do
  flags="${combos[$idx]}"
  label="${labels[$idx]}"
  echo
  echo "--------------------------------------------------------------"
  echo "CONFIG [$label]  (cargo $flags)"
  echo "--------------------------------------------------------------"

  ( cd "$here" && timeout 600 cargo build --release $flags ) >"$here/target/rs-build.log" 2>&1
  if [ $? -ne 0 ]; then
    echo "  RUST BUILD FAILED"; tail -20 "$here/target/rs-build.log"; fail=1; continue
  fi
  ( cd "$here" && timeout 600 cargo build $flags ) >"$here/target/rs-build-dbg.log" 2>&1
  if [ $? -ne 0 ]; then
    echo "  RUST DEBUG BUILD FAILED"; tail -20 "$here/target/rs-build-dbg.log"; fail=1; continue
  fi
  r_so="$here/target/release/libcomplexmode_lib.so"
  r_syms="$here/target/rust.syms"
  nm -D --defined-only "$r_so" | awk '$2=="T"||$2=="t"{print $3}' | sort -u >"$r_syms"

  missing="$(comm -23 "$c_syms" "$r_syms")"
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY FAILED -- missing from Rust .so:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  symbol parity: OK ($(wc -l <"$r_syms") Rust exports, 0 missing)"
  fi

  undef="$(nm -D --undefined-only "$r_so" | awk '{print $2}' \
      | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
      | grep -vE '@GLIBC|@GCC' )"
  if [ -n "$undef" ]; then
    echo "  UNRESOLVED non-libc symbols:"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  undefined non-libc symbols: 0"
  fi

  # Run the suite against BOTH the release and the debug cdylib. `-C
  # debug-assertions` instruments raw-pointer derefs, which can turn a fault the
  # C takes into a Rust panic -- a real divergence an external consumer of the
  # debug .so would see.
  for prof in release debug; do
    ( cd "$here" && RUST_SO="$here/target/$prof/libcomplexmode_lib.so" \
        timeout 600 cargo test --release $flags -- --test-threads=1 ) \
        >"$here/target/test-$idx-$prof.log" 2>&1
    rc=$?
    echo "  tests [rust .so = $prof]: $(grep -h '^test result:' "$here/target/test-$idx-$prof.log" | sed 's/test result: //' | tr '\n' '|')"
    if [ $rc -ne 0 ]; then
      echo "  TESTS FAILED (see target/test-$idx-$prof.log)"
      grep -A6 '^failures:' "$here/target/test-$idx-$prof.log" | head -40
      fail=1
    fi
  done
done

echo
echo "=============================================================="
echo " 4. binary-executable stdout comparison"
echo "=============================================================="
if grep -qE '^\s*add_executable' "$root/c_src/CMakeLists.txt" \
   || [ -d "$here/src/bin" ] || [ -f "$here/src/main.rs" ]; then
  echo "  a binary target exists -- stdout comparison required"; fail=1
else
  echo "  N/A: the project builds no executable"
  echo "       (CMakeLists.txt has only add_library; Cargo.toml has only [lib])"
fi

echo
if [ $fail -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail

#!/usr/bin/env bash
# Phase D driver: build the C .so, then for every feature combination and every
# profile, build the Rust cdylib, diff `nm -D` against the C .so, and run the
# whole differential suite.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$root"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- C shared lib
note "building the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so="$(ls c_src/build/*.so | head -1)"
echo "C .so: $c_so"

# ------------------------------------------------------------ feature combos
cd translation
# Every feature name declared in Cargo.toml (the [features] table).
mapfile -t features < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml)

combos=()
if [ "${#features[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configurations are the"
  echo "default build and --no-default-features."
  combos=("--no-default-features" "")
else
  # power set of the declared features, plus the default build
  n=${#features[@]}
  combos=("")
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${features[$i]}")
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      combos+=("--no-default-features")
    else
      combos+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

nm_syms() { nm -D --defined-only "$1" | awk '{print $3}' | sort -u; }
nm_syms "../$c_so" > /tmp/pd_c_syms.txt

for profile in dev release; do
  relflag=""
  outdir="target/debug"
  if [ "$profile" = release ]; then relflag="--release"; outdir="target/release"; fi
  for combo in "${combos[@]}"; do
    label="profile=$profile features='${combo:-<default>}'"
    note "$label"

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $relflag $combo >/tmp/pd_build.log 2>&1; then
      echo "BUILD FAILED ($label)"; tail -20 /tmp/pd_build.log; fail=1; continue
    fi

    rust_so="$outdir/libstr_dups_lib.so"
    if [ ! -f "$rust_so" ]; then
      echo "MISSING $rust_so ($label)"; fail=1; continue
    fi

    nm_syms "$rust_so" > /tmp/pd_rs_syms.txt
    missing="$(comm -23 /tmp/pd_c_syms.txt /tmp/pd_rs_syms.txt)"
    extra="$(comm -13 /tmp/pd_c_syms.txt /tmp/pd_rs_syms.txt)"
    if [ -n "$missing" ]; then
      echo "SYMBOLS MISSING FROM RUST ($label):"; echo "$missing"; fail=1
    fi
    if [ -n "$extra" ]; then
      echo "EXTRA RUST SYMBOLS ($label):"; echo "$extra"; fail=1
    fi
    undef="$(nm -D -u "$rust_so" | awk '{print $2}' | grep -E '^stbds_|^str_dups$|^strkey$' || true)"
    if [ -n "$undef" ]; then
      echo "UNDEFINED stbds_* SYMBOLS ($label):"; echo "$undef"; fail=1
    fi
    echo "symbol diff: empty ($(wc -l < /tmp/pd_c_syms.txt) symbols)"

    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $relflag $combo -- --test-threads=1 >/tmp/pd_test.log 2>&1; then
      echo "TESTS FAILED ($label)"; grep -E "^(test |failures:|error)" /tmp/pd_test.log | tail -40; fail=1
    else
      grep -E "^test result:" /tmp/pd_test.log | sed 's/^/  /'
    fi
  done
done

note "summary"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$fail"

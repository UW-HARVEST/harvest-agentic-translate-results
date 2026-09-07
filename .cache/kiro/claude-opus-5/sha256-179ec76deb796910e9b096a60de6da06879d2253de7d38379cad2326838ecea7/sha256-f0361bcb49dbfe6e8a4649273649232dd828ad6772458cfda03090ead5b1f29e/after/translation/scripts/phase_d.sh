#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff exported symbols, and run the
# full differential suite under every cargo feature combination.
set -uo pipefail
# translation/scripts/ -> workspace root
cd "$(dirname "$0")/../.."
ROOT="$PWD"
C_BUILD="$ROOT/c_src/build"
RUST_SO="$ROOT/translation/target/release/libomni_collide_lib.so"
fail=0

echo "=== 1/4  Building the C shared library ==="
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$ROOT/translation/c_build.log" 2>&1 \
  || { echo "C build FAILED (see translation/c_build.log)"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | head -1)"
echo "C  .so: $C_SO"

echo
echo "=== 2/4  Building the Rust cdylib ==="
( cd "$ROOT/translation" && timeout 600 cargo build --release ) \
  >"$ROOT/translation/rust_build.log" 2>&1 \
  || { echo "Rust build FAILED (see translation/rust_build.log)"; exit 1; }
echo "Rust .so: $RUST_SO"

echo
echo "=== 3/4  Symbol parity (nm -D) ==="
nm -D --defined-only "$C_SO"   | awk '$2=="T"||$2=="W"{print $3}' | sort >/tmp/pd_c.txt
nm -D --defined-only "$RUST_SO"| awk '$2=="T"||$2=="W"{print $3}' | sort >/tmp/pd_r.txt
echo "C exports:    $(wc -l </tmp/pd_c.txt)"
echo "Rust exports: $(wc -l </tmp/pd_r.txt)"
missing="$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)"
extra="$(comm -13 /tmp/pd_c.txt /tmp/pd_r.txt)"
if [ -n "$missing" ]; then
  echo "MISSING from Rust:"; echo "$missing"; fail=1
else
  echo "MISSING from Rust: none"
fi
if [ -n "$extra" ]; then echo "Extra in Rust (informational):"; echo "$extra"; fi

# Undefined symbols in the Rust .so that are not libc / libgcc / Rust runtime.
undef_bad="$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' \
  | grep -v '@GLIBC' | grep -v '@GCC' \
  | grep -vE '^(_ITM_|__gmon_start__|_Unwind_|__cxa_|__tls_get_addr)' || true)"
if [ -n "$undef_bad" ]; then
  echo "Unresolved non-libc symbols in the Rust .so:"; echo "$undef_bad"; fail=1
else
  echo "Unresolved non-libc symbols: none"
fi

echo
echo "=== 4/4  Test suite across every feature combination ==="
cd "$ROOT/translation"
# Enumerate features declared in Cargo.toml (excluding the implicit "default").
feats="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' Cargo.toml \
        | grep -v '^default$' || true)"
combos=()
combos+=("DEFAULT|")                       # default features
combos+=("NO-DEFAULT|--no-default-features")
if [ -n "$feats" ]; then
  combos+=("ALL|--all-features")
  # Every non-empty subset of the declared features.
  list=($feats); n=${#list[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${list[$i]}"; fi
    done
    combos+=("$sel|--no-default-features --features $sel")
  done
else
  echo "(Cargo.toml declares no [features]; DEFAULT and NO-DEFAULT are the only"
  echo " configurations, and they are equivalent.)"
fi

for entry in "${combos[@]}"; do
  name="${entry%%|*}"; flags="${entry#*|}"
  echo
  echo "--- combination: $name  (cargo test --release $flags) ---"
  # shellcheck disable=SC2086
  if timeout 600 cargo test --release $flags -- --test-threads=4 >"/tmp/pd_test_$name.log" 2>&1; then
    grep -hE '^test result:' "/tmp/pd_test_$name.log" | sed 's/^/    /'
  else
    echo "    FAILED — see /tmp/pd_test_$name.log"
    tail -30 "/tmp/pd_test_$name.log" | sed 's/^/    /'
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "PHASE D: PASS — symbol parity exact, all suites green in every combination."
else
  echo "PHASE D: FAIL"
fi
exit "$fail"

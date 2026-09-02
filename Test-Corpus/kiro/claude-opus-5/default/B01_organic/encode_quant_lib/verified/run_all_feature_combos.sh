#!/usr/bin/env bash
# Phase D driver: run `cargo check` + the full differential suite under EVERY
# feature combination, and against BOTH the debug and the release cdylib.
#
# Feature names are extracted from Cargo.toml rather than hardcoded, so this
# keeps working if features are added later.
set -uo pipefail
cd "$(dirname "$0")"

# --- extract feature names from the [features] section of Cargo.toml ---------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the power set of feature names ------------------------------------
COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  # No features exist, so the only configurations are the flag variants below.
  COMBOS=("")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then sel="${sel:+$sel,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$sel")
  done
fi

fail=0
run() { # label, then command
  local label="$1"; shift
  printf '%-64s' "$label"
  if out=$(timeout 600 "$@" 2>&1); then
    echo "OK"
  else
    echo "FAIL"
    echo "$out" | tail -25 | sed 's/^/    | /'
    fail=1
  fi
}

echo
echo "=== cargo check across feature combinations ==="
run "check --no-default-features" cargo check --no-default-features
run "check (default)"             cargo check
run "check --all-features"        cargo check --all-features
for c in "${COMBOS[@]}"; do
  [ -z "$c" ] && continue
  run "check --no-default-features --features $c" \
      cargo check --no-default-features --features "$c"
done

echo
echo "=== build both profiles (so the loader has fresh artifacts) ==="
run "build --lib (debug)"   cargo build --lib
run "build --lib --release" cargo build --lib --release

echo
echo "=== differential suite across feature combinations ==="
run "test --no-default-features" cargo test --no-default-features
run "test (default)"             cargo test
run "test --all-features"        cargo test --all-features
for c in "${COMBOS[@]}"; do
  [ -z "$c" ] && continue
  run "test --no-default-features --features $c" \
      cargo test --no-default-features --features "$c"
done

echo
echo "=== differential suite against the RELEASE cdylib explicitly ==="
REL="$PWD/target/release/libencode_quant_lib.so"
if [ -f "$REL" ]; then
  printf '%-64s' "test vs release .so"
  if out=$(RUST_SO_PATH="$REL" timeout 600 cargo test 2>&1); then echo "OK"; else
    echo "FAIL"; echo "$out" | tail -25 | sed 's/^/    | /'; fail=1; fi
else
  echo "release .so missing at $REL"; fail=1
fi

echo
echo "=== symbol parity (nm -D) ==="
C_SO=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
R_SO="$PWD/target/release/libencode_quant_lib.so"
if [ -z "$C_SO" ]; then echo "C .so not built"; fail=1; else
  diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort) \
    && echo "symbol diff: EMPTY (parity reached)" \
    || { echo "symbol diff NON-EMPTY"; fail=1; }
  echo "non-libc undefined symbols in Rust .so:"
  nm -D -u "$R_SO" | awk '{print $NF}' \
    | grep -vE '@GLIBC|@GCC|^__gmon_start__$|^_ITM_|^__cxa_|^statx$|^gettid$' \
    | sed 's/^/    /' || true
  cnt=$(nm -D -u "$R_SO" | awk '{print $NF}' \
        | grep -vcE '@GLIBC|@GCC|^__gmon_start__$|^_ITM_|^__cxa_|^statx$|^gettid$')
  echo "    count = $cnt"
  [ "$cnt" -eq 0 ] || { echo "unexpected non-libc undefined symbols"; fail=1; }
fi

echo
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$fail"

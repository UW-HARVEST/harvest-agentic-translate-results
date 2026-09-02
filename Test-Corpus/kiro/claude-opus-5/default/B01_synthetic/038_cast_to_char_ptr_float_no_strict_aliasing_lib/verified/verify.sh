#!/usr/bin/env bash
# Phase D driver: symbol parity + Phase B/C differential suite across EVERY
# Cargo feature combination.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# the sweep stays correct if features are added later.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
FAIL=0

step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------------------
step "Build the C reference shared library"
# --------------------------------------------------------------------------
if [ ! -f "$C_SO" ]; then
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
fi
[ -f "$C_SO" ] && ok "$C_SO" || { bad "C .so missing"; exit 1; }

# --------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# --------------------------------------------------------------------------
# Every feature name declared under [features] (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("")                          # default features
COMBOS+=("--no-default-features")     # empty feature set
if [ -n "$FEATURES" ]; then
  # Full power set of the declared features, each also with --no-default-features.
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then sel="${sel:+$sel,}${FARR[$i]}"; fi
    done
    COMBOS+=("--features $sel")
    COMBOS+=("--no-default-features --features $sel")
  done
  ok "declared features: $(echo "$FEATURES" | tr '\n' ' ')"
else
  ok "no [features] table -> the only combinations are default and --no-default-features"
fi
printf '  %d combination(s):\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '    - cargo <cmd> %s\n' "${c:-(default)}"; done

# --------------------------------------------------------------------------
step "Per-combination: cargo check, symbol parity, differential suite"
# --------------------------------------------------------------------------
c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)

for combo in "${COMBOS[@]}"; do
  label="${combo:-(default)}"
  printf '\n--- combination: %s ---\n' "$label"

  # shellcheck disable=SC2086
  if timeout 600 cargo check $combo >/dev/null 2>&1; then
    ok "cargo check"
  else
    bad "cargo check $label"; continue
  fi

  tdir="target/sweep$(echo "$combo" | tr -c 'A-Za-z0-9' '_')"
  # shellcheck disable=SC2086
  if timeout 600 cargo build --release --target-dir "$tdir" $combo >/dev/null 2>&1; then
    ok "cargo build --release"
  else
    bad "cargo build $label"; continue
  fi

  rust_so="$tdir/release/libdriver.so"
  if [ ! -f "$rust_so" ]; then bad "missing $rust_so"; continue; fi

  # Symbol parity: every symbol the C .so exports must be exported by Rust.
  rust_syms=$(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -z "$missing" ]; then
    ok "symbol parity: 0 missing ($(echo "$c_syms" | wc -l) C export(s))"
  else
    bad "symbols missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  fi

  # Undefined symbols in the Rust .so must all be libc / unwinder imports.
  unknown=$(nm -D --undefined-only "$rust_so" | awk '{print $NF}' \
    | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location)' \
    | grep -vE '^(printf|putchar|memcpy|memmove|memset|bcmp|malloc|calloc|realloc|free|posix_memalign|abort|getenv|getcwd|open64|read|write|writev|close|lseek64|stat64|fstat64|statx|readlink|realpath|mmap64|munmap|dl_iterate_phdr|syscall|gettid|pthread_key_create|pthread_key_delete|pthread_setspecific|strlen)$')
  if [ -z "$unknown" ]; then
    ok "undefined symbols: libc/unwind only"
  else
    bad "non-libc undefined symbols: $(echo "$unknown" | tr '\n' ' ')"
  fi

  # Phases B + C against the .so built for THIS combination.
  # shellcheck disable=SC2086
  if RUST_DRIVER_SO="$(pwd)/$rust_so" C_DRIVER_SO="$C_SO" \
       timeout 600 cargo test $combo --test differential > "$tdir/test.log" 2>&1; then
    ok "differential suite: $(grep -c 'case .* \.\.\. ok$' "$tdir/test.log") case(s) passed"
  else
    bad "differential suite $label"
    grep -E 'FAILED|panicked|divergence' "$tdir/test.log" | head -20
  fi
done

printf '\n'
if [ "$FAIL" -eq 0 ]; then
  printf '\033[1mSWEEP RESULT: ok — all combinations passed.\033[0m\n'
else
  printf '\033[1mSWEEP RESULT: FAILED.\033[0m\n'
fi
exit "$FAIL"

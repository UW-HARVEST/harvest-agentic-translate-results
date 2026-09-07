#!/usr/bin/env bash
# Phase D driver: build both libraries, diff their exported symbols, and run the
# full Phase B + Phase C suite under EVERY feature combination and both cargo
# profiles. Exits non-zero on the first failure.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [OK]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
# ---------------------------------------------------------------------------
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && timeout 600 cmake --build . >/dev/null ) \
  && ok "c_src built" || { bad "c_src build"; exit 1; }

C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
ok "C  .so = $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
# Every feature declared in [features] (excluding "default"), then the power set.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$HERE/Cargo.toml"
)
printf '  declared features: %s\n' "${FEATURES[*]:-<none>}"

COMBOS=()                                  # cargo flag strings
COMBOS+=("")                               # default features
COMBOS+=("--no-default-features")          # nothing enabled
NF=${#FEATURES[@]}
if (( NF > 0 )); then
  for (( mask=1; mask < (1<<NF); mask++ )); do
    set=""
    for (( i=0; i<NF; i++ )); do
      if (( mask & (1<<i) )); then set="${set:+$set,}${FEATURES[i]}"; fi
    done
    COMBOS+=("--no-default-features --features $set")
    COMBOS+=("--features $set")
  done
fi
printf '  %d combination(s) to verify\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "cargo check under every combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  if timeout 600 cargo check --manifest-path "$HERE/Cargo.toml" $combo >/dev/null 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"
  fi
done

# ---------------------------------------------------------------------------
step "Symbol parity (nm -D) under every combination"
# ---------------------------------------------------------------------------
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/hv_c_syms.txt
printf '  C exports %d symbol(s): %s\n' \
  "$(wc -l < /tmp/hv_c_syms.txt)" "$(paste -sd' ' /tmp/hv_c_syms.txt)"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  timeout 600 cargo build --release --manifest-path "$HERE/Cargo.toml" $combo >/dev/null 2>&1 \
    || { bad "cargo build --release $label"; continue; }
  R_SO="$HERE/target/release/libtritanopia_lib.so"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/hv_r_syms.txt

  missing="$(comm -23 /tmp/hv_c_syms.txt /tmp/hv_r_syms.txt)"
  extra="$(comm -13 /tmp/hv_c_syms.txt /tmp/hv_r_syms.txt)"
  if [[ -z "$missing" && -z "$extra" ]]; then
    ok "symbol diff EMPTY for $label"
  else
    [[ -n "$missing" ]] && bad "missing from Rust .so ($label): $(paste -sd' ' <<<"$missing")"
    [[ -n "$extra"   ]] && bad "extra in Rust .so ($label): $(paste -sd' ' <<<"$extra")"
  fi

  # Undefined non-libc / non-unwind imports must be empty.
  stray="$(nm -D -u "$R_SO" | awk '$1=="U"{print $2}' | sed 's/@.*//' \
           | grep -v -E '^(_Unwind_|__cxa_|__gmon|__errno_location|__tls_get_addr|_ITM_)' \
           | grep -v -x -E 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pow|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev' \
           || true)"
  if [[ -z "$stray" ]]; then
    ok "0 missing/undefined non-libc symbols for $label"
  else
    bad "unresolved non-libc imports ($label): $(paste -sd' ' <<<"$stray")"
  fi
done

# ---------------------------------------------------------------------------
step "Binary / driver executable check"
# ---------------------------------------------------------------------------
if grep -qE '^[[:space:]]*add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "c_src declares add_executable — stdout comparison required but not implemented"
else
  ok "c_src builds only a SHARED library (no driver binary); stdout comparison N/A"
fi
if [[ -d "$HERE/src/bin" ]] || grep -qE '^\[\[bin\]\]' "$HERE/Cargo.toml"; then
  bad "Rust crate declares a binary the C does not have"
else
  ok "Rust crate declares no binary either"
fi

# ---------------------------------------------------------------------------
step "Phase B + Phase C differential suites under every combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="${combo:-<default>} ${profile:-<debug>}"
    # The cdylib must be rebuilt for the profile the tests will pick it up from.
    timeout 600 cargo build $profile --manifest-path "$HERE/Cargo.toml" $combo >/dev/null 2>&1 \
      || { bad "cargo build $label"; continue; }
    if timeout 600 cargo test $profile --manifest-path "$HERE/Cargo.toml" $combo \
         -- --test-threads=4 >/tmp/hv_test.log 2>&1; then
      passed="$(grep -c '^test .* ok$' /tmp/hv_test.log || true)"
      ok "cargo test $label — $passed test(s) passed"
    else
      bad "cargo test $label"
      tail -40 /tmp/hv_test.log | sed 's/^/      /'
    fi
  done
done

printf '\n=========================================\n'
if (( FAIL == 0 )); then
  printf 'ALL PHASE D CHECKS PASSED\n'
else
  printf 'FAILURES DETECTED\n'
fi
printf '=========================================\n'
exit "$FAIL"

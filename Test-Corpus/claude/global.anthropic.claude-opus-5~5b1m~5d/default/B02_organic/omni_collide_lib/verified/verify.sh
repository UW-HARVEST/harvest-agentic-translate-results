#!/usr/bin/env bash
#
# Full verification driver.
#
#   1. builds the C shared library with CMake
#   2. builds the Rust cdylib in BOTH profiles (the harness loads whichever
#      exist, so both get compared against the C)
#   3. diffs `nm -D` exports  C .so  vs  Rust .so  and fails if anything is
#      missing
#   4. enumerates every feature combination declared in Cargo.toml and runs
#      `cargo check` + the whole test suite for each
#
# Usage:  ./verify.sh
#
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
CARGO_FLAGS="--offline"
FAILURES=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; FAILURES=$((FAILURES + 1)); }
ok()   { printf '\033[32mok\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------------------
step "1. build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { fail "C build"; exit 1; }

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' -type f | sort | head -1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
ok "C .so: $C_SO"

# ---------------------------------------------------------------------------
step "2. build the Rust cdylib (debug + release)"
# ---------------------------------------------------------------------------
cd "$CRATE_DIR"
cargo build $CARGO_FLAGS -q          || fail "cargo build (debug)"
cargo build $CARGO_FLAGS --release -q || fail "cargo build (release)"
for p in debug release; do
  if [ -f "target/$p/libomni_collide_lib.so" ]; then
    ok "Rust .so: target/$p/libomni_collide_lib.so"
  else
    fail "missing target/$p/libomni_collide_lib.so"
  fi
done

# ---------------------------------------------------------------------------
step "3. symbol parity (nm -D)"
# ---------------------------------------------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/verify.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$TMP/c.txt"
for p in release debug; do
  RS="target/$p/libomni_collide_lib.so"
  [ -f "$RS" ] || continue
  nm -D --defined-only "$RS" | awk '{print $3}' | sort -u > "$TMP/r.txt"
  MISSING="$(comm -23 "$TMP/c.txt" "$TMP/r.txt")"
  if [ -n "$MISSING" ]; then
    fail "$p: Rust .so is missing $(echo "$MISSING" | wc -l) C symbol(s):"
    echo "$MISSING" | sed 's/^/       /'
  else
    ok "$p: all $(wc -l < "$TMP/c.txt") C symbols exported by the Rust .so"
  fi
done
# Undefined symbols in the Rust .so must all be libc/libgcc.
STRAY="$(nm -D --undefined-only target/release/libomni_collide_lib.so \
        | awk '{print $2}' | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
        | grep -vE '^[a-z_]+(64)?@GLIBC' | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|getcwd|getenv|malloc|memcpy|memmove|memset|posix_memalign|pthread_[a-z_]+|read|readlink|realloc|realpath|strlen|syscall|write|writev)$' || true)"
if [ -n "$STRAY" ]; then
  fail "Rust .so imports non-libc symbols:"; echo "$STRAY" | sed 's/^/       /'
else
  ok "all undefined symbols in the Rust .so are libc/libgcc"
fi

# ---------------------------------------------------------------------------
step "4. feature combinations"
# ---------------------------------------------------------------------------
# Enumerate the [features] table from Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features] table -> the only configuration is the"
  echo "default one. Verifying it under all three feature spellings anyway."
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "features: ${FEATURES[*]}"
  COMBOS=("" "--no-default-features" "--all-features")
  N=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << N); mask++)); do
    sel=()
    for ((b = 0; b < N; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[b]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  if ! cargo check $CARGO_FLAGS $combo -q 2>&1 | tail -5; then
    fail "cargo check $label"
    continue
  fi
  # The cdylib must be rebuilt for this combo before the tests dlopen it.
  cargo build $CARGO_FLAGS $combo -q          || { fail "build(debug) $label";   continue; }
  cargo build $CARGO_FLAGS $combo --release -q || { fail "build(release) $label"; continue; }
  out="$(cargo test $CARGO_FLAGS $combo 2>&1)"
  rc=$?
  passed="$(echo "$out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END{print s+0}')"
  failed="$(echo "$out" | grep -oE '[0-9]+ failed' | awk '{s+=$1} END{print s+0}')"
  if [ "$rc" -ne 0 ] || [ "$failed" -ne 0 ]; then
    fail "cargo test $label ($passed passed, $failed failed)"
    echo "$out" | grep -E 'FAILED|panicked|signal' | head -20 | sed 's/^/       /'
  else
    ok "cargo test $label: $passed passed, 0 failed"
  fi
done

# ---------------------------------------------------------------------------
step "summary"
# ---------------------------------------------------------------------------
if [ "$FAILURES" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
  exit 0
else
  printf '\033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILURES"
  exit 1
fi

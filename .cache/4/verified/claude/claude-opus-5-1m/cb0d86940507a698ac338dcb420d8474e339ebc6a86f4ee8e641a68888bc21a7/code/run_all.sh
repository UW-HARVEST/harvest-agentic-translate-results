#!/bin/bash
# Full differential verification driver.
#
# IMPORTANT: `cargo build` MUST run before `cargo test`. With a cdylib the
# integration tests do not force the cdylib to be re-linked/uplifted into
# target/<profile>/, so `cargo test` alone would silently reuse a STALE .so and
# every differential test would pass vacuously. tests/common/mod.rs also
# asserts the .so is newer than src/ as a second line of defence.
set -uo pipefail
cd "$(dirname "$0")"

# Use a sandbox-writable scratch dir; /tmp may be read-only.
WORK="${TMPDIR:-/tmp}/trit_verify.$$"
mkdir -p "$WORK" || { echo "cannot create scratch dir $WORK"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

RC=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ OK ]   %s\n' "$*"; }
bad()  { printf '  [FAIL]   %s\n' "$*"; RC=1; }

# --------------------------------------------------------------------------
# Feature combinations. Cargo.toml declares NO [features], so the complete
# set is the default (empty) one; both spellings are exercised anyway.
# --------------------------------------------------------------------------
FEATURE_SETS=("--no-default-features" "")

# Expected total number of passing tests per run (27 Phase B + 1 exhaustive
# + 13 Phase C). Guards against a vacuous "0 tests ran" success.
MIN_TESTS=41

step "Build C reference (default config, -O0)"
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "c_src/build/libtranslated_rust.so" \
  || { bad "C -O0 build"; exit 1; }

step "Build C reference (extra assurance, -O2)"
mkdir -p "$PWD/c_build_O2"
( cd c_build_O2 && cmake ../c_src -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_FLAGS_RELEASE=-O2 >/dev/null \
  && cmake --build . >/dev/null ) && ok "c_build_O2/libc_src.so" \
  || bad "C -O2 build"
C_O2=$(find "$PWD/c_build_O2" -maxdepth 1 -name '*.so' | head -1)

step "Symbol parity (nm -D)"
cargo build --release >/dev/null 2>&1
nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so \
  | awk '$2 ~ /^[TWDBRi]$/ {print $3}' | sort -u > "$WORK/c_syms"
nm -D --defined-only --extern-only target/release/libtritanopia_lib.so \
  | awk '$2 ~ /^[TWDBRi]$/ {print $3}' | sort -u > "$WORK/r_syms"
if [ ! -s "$WORK/c_syms" ]; then bad "could not read C symbols (scratch dir unwritable?)"; fi
if [ ! -s "$WORK/r_syms" ]; then bad "could not read Rust symbols"; fi
MISSING=$(comm -23 "$WORK/c_syms" "$WORK/r_syms")
EXTRA=$(comm -13 "$WORK/c_syms" "$WORK/r_syms")
if [ -z "$MISSING" ]; then ok "0 symbols missing from Rust .so"; else bad "missing: $MISSING"; fi
if [ -z "$EXTRA" ]; then ok "0 extra public symbols in Rust .so"; else bad "extra: $EXTRA"; fi
NONLIBC=$(nm -D --undefined-only target/release/libtritanopia_lib.so \
  | awk '{print $NF}' | sed 's/@.*//' \
  | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pow|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)' || true)
if [ -z "$NONLIBC" ]; then ok "0 undefined non-libc symbols"; else bad "undefined non-libc: $NONLIBC"; fi


# --------------------------------------------------------------------------
# cargo check for every feature combination
# --------------------------------------------------------------------------
for FS in "${FEATURE_SETS[@]}"; do
  step "cargo check ${FS:-<default features>}"
  if cargo check $FS --all-targets 2>&1 | tail -3 | grep -q '^error'; then
    bad "cargo check $FS"
  else
    ok "cargo check ${FS:-<default>} clean"
  fi
done

# --------------------------------------------------------------------------
# Phases B + C for every feature combination x profile x C build
# --------------------------------------------------------------------------
for FS in "${FEATURE_SETS[@]}"; do
  for PROFILE in debug release; do
    PF=""; [ "$PROFILE" = release ] && PF="--release"
    for CSO in "$PWD/c_src/build/libtranslated_rust.so" "$C_O2"; do
      [ -f "$CSO" ] || continue
      LABEL="features='${FS:-default}' profile=$PROFILE C=$(basename "$(dirname "$CSO")")"
      step "Phases B+C : $LABEL"
      # Build FIRST so the .so under test is fresh (see header comment).
      if ! cargo build $PF $FS >/dev/null 2>&1; then bad "build ($LABEL)"; continue; fi
      TRIT_C_SO="$CSO" cargo test $PF $FS > "$WORK/test.log" 2>&1
      # Fail CLOSED: require the log to exist, contain no FAILED result, and
      # report a plausible number of passing tests. A missing/empty log or a
      # zero test count is treated as a failure, never as success.
      if [ ! -s "$WORK/test.log" ]; then
        bad "$LABEL : test log missing/empty - cannot confirm success"
      elif grep -qE '^test result: FAILED' "$WORK/test.log"; then
        bad "$LABEL"
        grep -E '^test .* FAILED' "$WORK/test.log" | head -10 | sed 's/^/        /'
      else
        PASSED=$(grep -oE '^test result: ok\. [0-9]+ passed' "$WORK/test.log" \
                 | grep -oE '^test result: ok\. [0-9]+' | grep -oE '[0-9]+$' \
                 | awk '{s+=$1} END{print s+0}')
        if [ "$PASSED" -lt "$MIN_TESTS" ]; then
          bad "$LABEL : only $PASSED tests passed, expected >= $MIN_TESTS"
        else
          ok "$LABEL  ($PASSED tests passed)"
        fi
      fi
    done
  done
done

step "RESULT"
if [ $RC -eq 0 ]; then
  echo "  ALL CONFIGURATIONS PASSED"
else
  echo "  FAILURES PRESENT (see [FAIL] above)"
fi
exit $RC

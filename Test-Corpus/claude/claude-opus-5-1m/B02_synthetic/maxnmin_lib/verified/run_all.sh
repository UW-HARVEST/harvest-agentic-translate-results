#!/usr/bin/env bash
# Full differential verification: every build configuration x every test.
#
# Cargo.toml declares no [features] and c_src/CMakeLists.txt declares no
# option()/target_compile_definitions, and lib.c has no #if/#ifdef at all, so
# there is exactly ONE feature combination. It is still enumerated mechanically
# below rather than hard-coded, so new features would be picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/verify.XXXXXX")" || { echo "cannot create temp dir"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------- 1. C library
note "Building the C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
ls -l "$C_SO"

# ------------------------------------------------- 2. enumerate feature combos
# Every subset of the [features] table (empty table => just the empty set).
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
from itertools import combinations
combos = ['']
for r in range(1, len(names) + 1):
    combos += [','.join(c) for c in combinations(names, r)]
print('\n'.join(combos))
PY
)
note "Feature combinations to verify: ${#FEATURES[@]}"
for f in "${FEATURES[@]}"; do echo "  --no-default-features --features '${f}'"; done

# ------------------------------------------------------- 3. check every combo
for f in "${FEATURES[@]}"; do
  note "cargo check --no-default-features --features '${f}'"
  cargo check --offline --no-default-features --features "$f" --all-targets \
    || { echo "CHECK FAILED for '${f}'"; fail=1; }
done

# ------------------------------- 4. build + test every combo x every profile
for prof in dev release; do
  if [ "$prof" = release ]; then RFLAG=--release; PDIR=release; else RFLAG=; PDIR=debug; fi
  for f in "${FEATURES[@]}"; do
    note "profile=$prof features='${f}': build cdylib"
    # `cargo test` does NOT regenerate a cdylib-only lib target, so build it
    # explicitly first; tests/common/mod.rs also hard-fails on a stale artifact.
    cargo build --offline $RFLAG --no-default-features --features "$f" \
      || { echo "BUILD FAILED"; fail=1; continue; }

    note "profile=$prof features='${f}': symbol parity (nm -D)"
    R_SO="target/$PDIR/libmaxnmin_lib.so"
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$WORK/c_syms"
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$WORK/r_syms"
    ncs=$(wc -l < "$WORK/c_syms"); nrs=$(wc -l < "$WORK/r_syms")
    # Guard against a vacuous "no missing symbols" result from an empty listing.
    if [ "$ncs" -lt 7 ] || [ "$nrs" -lt 7 ]; then
      echo "SANITY FAILURE: nm listed $ncs C and $nrs Rust symbols (expected >= 7)"; fail=1
    fi
    missing=$(comm -23 "$WORK/c_syms" "$WORK/r_syms")
    if [ -n "$missing" ]; then
      echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
    else
      echo "OK: all $ncs C symbols are exported by the Rust .so ($nrs exported in total)"
    fi
    # No undefined symbol outside libc / libgcc-unwind / GNU weak hooks.
    bad=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' \
          | grep -vE '^(_ITM_|__gmon_start__|_Unwind_|__cxa_|__tls_get_addr|__errno_location)' \
          | grep -vE '^(malloc|calloc|realloc|free|posix_memalign|memcpy|memmove|memset|bcmp|strlen|strncpy|abort|getenv|getcwd|realpath|readlink|open64|close|read|write|writev|lseek64|stat64|fstat64|statx|mmap64|munmap|syscall|gettid|dl_iterate_phdr|pthread_key_create|pthread_key_delete|pthread_setspecific)$')
    if [ -n "$bad" ]; then echo "UNRESOLVED NON-LIBC SYMBOLS:"; echo "$bad"; fail=1; fi

    note "profile=$prof features='${f}': cargo test"
    cargo test --offline $RFLAG --no-default-features --features "$f" \
      || { echo "TESTS FAILED for profile=$prof features='${f}'"; fail=1; }
  done
done

note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"

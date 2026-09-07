#!/usr/bin/env bash
# Phase D driver: build the C, then for EVERY feature combination and EVERY
# cargo profile, rebuild the Rust cdylib, diff its exported symbols against the
# C .so, and run the whole differential test suite against that exact .so.
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
OFFLINE="${CARGO_OFFLINE:---offline}"

fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }
good() { printf '\033[32mok: %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------- 1. build C
step "building the C shared library"
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(ls "$CBUILD"/*.so | head -1)"
good "C .so = $C_SO"

# --------------------------------------------------- 2. enumerate feature sets
# Read [features] out of Cargo.toml.  This crate declares none, so the only
# combination is the default (empty) one -- but the loop is written generically
# so that adding features later is automatically covered.
mapfile -t FEATURES < <(
  python3 - "$CRATE/Cargo.toml" <<'PY'
import re, sys, itertools
src = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
if not names:
    print("__none__")
else:
    for k in range(len(names) + 1):
        for combo in itertools.combinations(names, k):
            print(",".join(combo) if combo else "__empty__")
PY
)
step "feature combinations: ${FEATURES[*]}"

# ------------------------------------------- 3. cross profiles x feature sets
for profile in debug release; do
  for feat in "${FEATURES[@]}"; do
    case "$feat" in
      __none__)  fargs=();                                    label="default (crate has no features)" ;;
      __empty__) fargs=(--no-default-features);               label="--no-default-features" ;;
      *)         fargs=(--no-default-features --features "$feat"); label="--features $feat" ;;
    esac
    pargs=(); [ "$profile" = release ] && pargs=(--release)

    step "profile=$profile  features=$label"

    if ! cargo build $OFFLINE "${pargs[@]}" "${fargs[@]}" >/dev/null 2>&1; then
      bad "cargo build (profile=$profile features=$label)"
      continue
    fi
    R_SO="$CRATE/target/$profile/libgjk_cache_lib.so"
    [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

    # ---- symbol parity ----
    nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > "$CRATE/target/.c.syms"
    nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort > "$CRATE/target/.r.syms"
    missing="$(comm -23 "$CRATE/target/.c.syms" "$CRATE/target/.r.syms")"
    extra="$(comm -13 "$CRATE/target/.c.syms" "$CRATE/target/.r.syms")"
    if [ -n "$missing" ]; then
      bad "symbols missing from the Rust .so:"; echo "$missing"
    else
      good "symbol parity ($(wc -l < "$CRATE/target/.c.syms") symbols, 0 missing)"
    fi
    [ -n "$extra" ] && printf 'note: extra Rust symbols:\n%s\n' "$extra"

    # ---- undefined non-libc symbols ----
    undef="$(nm -D -u "$R_SO" | awk '{print $NF}' \
      | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_|__tls_get_addr|__errno_location|statx|gettid)' \
      | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev)' \
      | sed '/^$/d')"
    if [ -n "$undef" ]; then
      bad "undefined non-libc symbols in the Rust .so:"; echo "$undef"
    else
      good "no undefined non-libc symbols"
    fi

    # ---- the differential suite, against THIS .so ----
    if RUST_SO="$R_SO" timeout 900 cargo test $OFFLINE "${pargs[@]}" "${fargs[@]}" \
         -- --test-threads=4 >"$CRATE/target/.test.$profile.log" 2>&1; then
      good "all differential tests passed"
      grep -hE "^test result:" "$CRATE/target/.test.$profile.log" | sed 's/^/    /'
    else
      bad "differential tests (profile=$profile features=$label)"
      tail -40 "$CRATE/target/.test.$profile.log"
    fi
  done
done

step "SUMMARY"
if [ "$fail" = 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$fail"

#!/usr/bin/env bash
# Full verification gate: build both libraries, diff their exported symbols,
# and run the whole differential test suite under EVERY feature combination.
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

# Scratch space: honour $TMPDIR (some environments have a read-only /tmp), and
# fail loudly rather than silently producing empty symbol diffs.
WORK="$(mktemp -d "${TMPDIR:-/tmp}/verify.XXXXXX")" || { echo "cannot create a temp dir"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "C build" || bad "C build"

C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)
[ -n "$C_SO" ] && ok "C .so: $(basename "$C_SO")" || bad "no C .so produced"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations
# ---------------------------------------------------------------------------
# The crate declares no [features] table, so the only configuration is the
# default (empty) feature set. Derive this mechanically rather than assuming.
FEATURES=$(python3 - "$CRATE/Cargo.toml" <<'PY'
import re, sys
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

COMBOS=()
COMBOS+=("--offline --release")                              # default features
COMBOS+=("--offline --release --no-default-features")        # empty feature set
if [ -n "${FEATURES// /}" ]; then
  # power set of the declared features
  read -ra FS <<< "$FEATURES"
  n=${#FS[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (( mask & (1 << i) )) && sel+=("${FS[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--offline --release --no-default-features --features $joined")
    COMBOS+=("--offline --release --features $joined")
  done
fi

step "Feature combinations to verify (${#COMBOS[@]})"
printf '  cargo test %s\n' "${COMBOS[@]}"
[ -z "${FEATURES// /}" ] && echo "  (Cargo.toml declares no [features], so these are the only two)"

# ---------------------------------------------------------------------------
# 3. Per-combination: build, symbol-diff, test
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  step "cargo build $combo"
  # shellcheck disable=SC2086
  ( cd "$CRATE" && cargo build $combo >/dev/null 2>&1 ) \
    && ok "build [$combo]" || { bad "build [$combo]"; continue; }

  R_SO="$CRATE/target/release/libarr_push_lib.so"
  [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

  step "Symbol parity [$combo]"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$WORK/c_syms" || bad "nm on $C_SO"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$WORK/r_syms" || bad "nm on $R_SO"
  n_c=$(wc -l < "$WORK/c_syms")
  n_r=$(wc -l < "$WORK/r_syms")
  [ "$n_c" -gt 0 ] || bad "nm -D listed 0 symbols for the C .so (symbol diff is meaningless)"
  [ "$n_r" -gt 0 ] || bad "nm -D listed 0 symbols for the Rust .so (symbol diff is meaningless)"
  echo "  C exports:    $n_c"
  echo "  Rust exports: $n_r"
  missing=$(comm -23 "$WORK/c_syms" "$WORK/r_syms")
  extra=$(comm -13 "$WORK/c_syms" "$WORK/r_syms")
  if [ -z "$missing" ]; then
    ok "all $n_c C symbols are exported by the Rust .so (symbol diff is EMPTY)"
  else
    bad "symbols missing from the Rust .so:"; echo "$missing" | sed 's/^/      /'
  fi
  if [ -n "$extra" ]; then
    echo "  note: Rust-only exported symbols (must be none for stb_ds):"
    echo "$extra" | sed 's/^/      /'
    bad "unexpected extra exports"
  fi
  # undefined symbols must all be libc / toolchain runtime
  nonlibc=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|memcmp|sprintf|strcmp|__assert_fail|__libc_)' \
    | sort -u)
  if [ -z "$nonlibc" ]; then
    ok "0 missing/undefined non-libc symbols in the Rust .so"
  else
    bad "undefined non-libc symbols:"; echo "$nonlibc" | sed 's/^/      /'
  fi

  step "cargo test $combo"
  # NB: `cargo test` does not rebuild a cdylib, so the `cargo build` above is
  # what refreshes the .so the tests dlopen. tests/common/mod.rs additionally
  # refuses to run against a .so older than src/, so a stale artifact can never
  # produce a false green.
  # shellcheck disable=SC2086
  if ( cd "$CRATE" && timeout 600 cargo test $combo 2>&1 | tail -25 ); then
    ok "tests [$combo]"
  else
    bad "tests [$combo]"
  fi
done

# ---------------------------------------------------------------------------
# 4. Binary / driver stdout comparison
# ---------------------------------------------------------------------------
step "Binary (driver) comparison"
if grep -qE '^\[\[bin\]\]' "$CRATE/Cargo.toml" || [ -f "$CRATE/src/main.rs" ]; then
  bad "a Rust binary exists but this script has no stdout comparison for it"
elif grep -qE 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "the C project builds an executable but no Rust binary exists"
else
  ok "neither project builds an executable (c_src/CMakeLists.txt is SHARED-only, \
Cargo.toml is crate-type=[\"cdylib\"]) -- no stdout to compare"
fi

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$FAIL"

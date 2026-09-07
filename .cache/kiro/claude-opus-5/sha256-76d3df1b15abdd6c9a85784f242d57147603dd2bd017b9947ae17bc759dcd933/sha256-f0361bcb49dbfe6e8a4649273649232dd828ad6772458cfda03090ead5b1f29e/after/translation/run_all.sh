#!/usr/bin/env bash
# Full verification run: Phases A-D.
#
#   ./run_all.sh
#
# 1. builds the C shared library
# 2. builds the Rust cdylib
# 3. Phase A/D: nm -D symbol parity diff (must be empty)
# 4. Phase B/C: differential test suites, for EVERY feature combination
# 5. Phase B: out-of-process stdout diff via tests/harness_main.c
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$CRATE_DIR/.." && pwd)"
C_DIR="$ROOT_DIR/c_src"
C_SO="$C_DIR/build/libdriver.so"
RUST_SO="$CRATE_DIR/target/ffi-so/release/libdriver.so"
WORK="$CRATE_DIR/target/verify"
mkdir -p "$WORK"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------- 1. build C
note "building C shared library"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || bad "C build failed"
[ -f "$C_SO" ] || bad "missing $C_SO"

# ------------------------------------------------------------- 2. build Rust
note "building Rust cdylib"
( cd "$CRATE_DIR" && timeout 600 cargo build --release --lib \
    --target-dir "$CRATE_DIR/target/ffi-so" >/dev/null 2>&1 ) \
  || bad "Rust cdylib build failed"
[ -f "$RUST_SO" ] || bad "missing $RUST_SO"

# ------------------------------------------------- 3. Phase A/D symbol parity
note "Phase D: nm -D symbol parity"
nm -D --defined-only "$C_SO"    | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort -u > "$WORK/c.syms"
nm -D --defined-only "$RUST_SO" | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort -u > "$WORK/rust.syms"
comm -23 "$WORK/c.syms" "$WORK/rust.syms" > "$WORK/missing.syms"
if [ -s "$WORK/missing.syms" ]; then
  bad "symbols exported by C but MISSING from Rust:"; cat "$WORK/missing.syms"
else
  echo "OK: 0 C symbols missing from the Rust .so"
  echo "C exports: $(wc -l < "$WORK/c.syms")   Rust exports: $(wc -l < "$WORK/rust.syms")"
fi

note "Phase A: undefined non-libc symbols in the Rust .so"
nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$WORK/rust.undef"
# Everything legitimately provided by libc / libgcc / the dynamic loader.
grep -Ev '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr$|__errno_location$|gettid$|statx$|syscall$|dl_iterate_phdr$|pthread_|abort$|bcmp$|calloc$|close$|free$|fstat|getcwd$|getenv$|lseek|malloc$|memcpy$|memmove$|memset$|mmap|munmap$|open|posix_memalign$|printf$|read$|readlink$|realloc$|realpath$|sscanf$|stat|strlen$|write$|writev$)' \
  "$WORK/rust.undef" > "$WORK/rust.undef.suspect" || true
if [ -s "$WORK/rust.undef.suspect" ]; then
  bad "non-libc undefined symbols in the Rust .so:"; cat "$WORK/rust.undef.suspect"
else
  echo "OK: 0 non-libc undefined symbols"
fi

# ------------------------------------------ 4. Phases B+C, all feature combos
# Cargo.toml declares no [features], so the complete combination set is the
# default build and the (identical) --no-default-features build. Discovered,
# not hard-coded:
FEATURES=$(cd "$CRATE_DIR" && cargo metadata --no-deps --format-version 1 2>/dev/null \
           | tr ',' '\n' | grep -o '"features":{[^}]*}' | head -1)
echo
echo "declared features: ${FEATURES:-none}"

COMBOS=("" "--no-default-features")
for combo in "${COMBOS[@]}"; do
  note "Phases B+C  cargo test --release ${combo:-（default)}"
  ( cd "$CRATE_DIR" && timeout 600 cargo test --release $combo -- --test-threads=1 ) \
    || bad "test suite failed for combo '${combo:-default}'"
done

# --------------------------------- 5. Phase B: out-of-process stdout byte diff
note "Phase B: out-of-process driver stdout diff"
CC=${CC:-cc}
"$CC" -O2 -o "$WORK/main_c"    "$CRATE_DIR/tests/harness_main.c" "$C_SO"    -Wl,-rpath,"$(dirname "$C_SO")"    2>/dev/null || bad "link main_c failed"
"$CC" -O2 -o "$WORK/main_rust" "$CRATE_DIR/tests/harness_main.c" "$RUST_SO" -Wl,-rpath,"$(dirname "$RUST_SO")" 2>/dev/null || bad "link main_rust failed"

if [ -x "$WORK/main_c" ] && [ -x "$WORK/main_rust" ]; then
  # A deterministic corpus: hand-picked edge cases plus generated ones.
  python3 - "$WORK/args" <<'PY'
import random, sys
random.seed(0xC0FFEE)
args = [
    "", " ", "\t\n\r", "0", "-0", "+0", "1", "-1", "2147483647", "-2147483648",
    "2147483648", "-2147483649", "99999999999", "007", "-00042", "0x10", "0X1f",
    "1e5", "1.5", ".5", "5.", "-", "+", "- 5", "--3", "abc", "1 2 x", "5,6",
    "1-2+3-4", "  \t 7 \n 8  ", "1\t2\n3\r4\v5\f6",
    " ".join(str(i) for i in range(1, 101)),
    " ".join(str(i) for i in range(1, 102)),
    " ".join(str(i) for i in range(1, 300)),
    " ".join(str(-i) for i in range(1, 150)),
    "9" * 200,
]
for _ in range(400):
    n = random.randint(0, 40)
    parts = []
    for _ in range(n):
        kind = random.randrange(6)
        if kind == 0:
            parts.append(str(random.randint(-2**31, 2**31 - 1)))
        elif kind == 1:
            parts.append(str(random.randint(-10, 10)))
        elif kind == 2:
            parts.append("+" + str(random.randint(0, 10**random.randint(1, 12))))
        elif kind == 3:
            parts.append("0" * random.randint(1, 5) + str(random.randint(0, 9999)))
        elif kind == 4:
            parts.append(random.choice(["x", "abc", ".", ",", "0x1f", "1e9", "-", "+"]))
        else:
            parts.append(str(random.randint(-2**40, 2**40)))
    sep = random.choice([" ", "\t", "\n", "  ", " \t "])
    args.append(sep.join(parts))
with open(sys.argv[1], "w") as f:
    for a in args:
        f.write(a.replace("\\", "\\\\").replace("\n", "\\n") + "\n")
PY
  # Feed the corpus in chunks so argv stays a sane size.
  : > "$WORK/out_c"; : > "$WORK/out_rust"
  mapfile -t CORPUS < "$WORK/args"
  chunk=()
  flushchunk() {
    [ ${#chunk[@]} -eq 0 ] && return
    "$WORK/main_c"    "${chunk[@]}" >> "$WORK/out_c"    2>&1
    "$WORK/main_rust" "${chunk[@]}" >> "$WORK/out_rust" 2>&1
    chunk=()
  }
  for line in "${CORPUS[@]}"; do
    chunk+=("$(printf '%b' "$line")")
    [ ${#chunk[@]} -ge 50 ] && flushchunk
  done
  flushchunk
  if cmp -s "$WORK/out_c" "$WORK/out_rust"; then
    echo "OK: stdout identical over $(wc -l < "$WORK/args") inputs ($(wc -c < "$WORK/out_c") bytes)"
  else
    bad "out-of-process stdout differs"; diff "$WORK/out_c" "$WORK/out_rust" | head -20
  fi
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASES PASSED"
else
  echo "VERIFICATION FAILED"
fi
exit "$fail"

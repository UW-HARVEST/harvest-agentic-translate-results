#!/usr/bin/env bash
# Phase A / Phase D: symbol parity between the C .so and the Rust .so, plus a
# value-by-value diff of the three lookup tables against the C source.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
RUST_SO="${1:-$ROOT/translation/target/release/libhalf2float_lib.so}"

[[ -f "$C_SO"    ]] || { echo "missing C .so (build c_src first)"    >&2; exit 1; }
[[ -f "$RUST_SO" ]] || { echo "missing Rust .so: $RUST_SO"           >&2; exit 1; }

echo "C    .so: $C_SO"
echo "Rust .so: $RUST_SO"
echo

# Weak/toolchain symbols that are not part of any library API.
IGNORE='^(_ITM_|__cxa_finalize|__gmon_start__|_init$|_fini$)'

exported() { nm -D --defined-only --format=posix "$1" | awk '{print $1}' | grep -Ev "$IGNORE" | sort -u; }

exported "$C_SO"    > /tmp/sym_c.txt
exported "$RUST_SO" > /tmp/sym_rust.txt

echo "=== C exported ($(wc -l < /tmp/sym_c.txt)) ===";    cat /tmp/sym_c.txt
echo "=== Rust exported ($(wc -l < /tmp/sym_rust.txt)) ==="; cat /tmp/sym_rust.txt
echo
echo "=== MISSING from Rust (C exported, Rust does not) ==="
MISSING="$(comm -23 /tmp/sym_c.txt /tmp/sym_rust.txt)"
if [[ -n "$MISSING" ]]; then echo "$MISSING"; else echo "(none)"; fi

echo
echo "=== Rust undefined, non-libc / non-unwinder ==="
SUSPICIOUS="$(nm -D --undefined-only --format=posix "$RUST_SO" | awk '{print $1}' \
  | grep -Ev '@GLIBC|@GCC|^_Unwind|^_ITM_|^__' \
  | grep -Ev '^(malloc|calloc|realloc|free|memcpy|memmove|memset|bcmp|strlen|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|syscall|gettid|posix_memalign|dl_iterate_phdr|pthread_key_create|pthread_key_delete|pthread_setspecific|pthread_getspecific)$' \
  || true)"
if [[ -n "$SUSPICIOUS" ]]; then echo "$SUSPICIOUS"; else echo "(none)"; fi

echo
echo "=== lookup-table value diff (C source vs Rust source) ==="
python3 - "$ROOT" <<'PY'
import re, sys
root = sys.argv[1]
c = open(f'{root}/c_src/src/lib.c').read()
r = open(f'{root}/translation/src/lib.rs').read()
hexes = lambda s: [int(x, 16) for x in re.findall(r'0x[0-9a-fA-F]+', s)]
def seg(t, start, end):
    i = t.index(start); j = t.index(end, i)
    return hexes(t[i + len(start):j])
tables = [
    ('m__mantissa', seg(c, 'm__mantissa[2048] = {', '};'), seg(r, 'M__MANTISSA: [u32; 2048] = [', '];'), 2048),
    ('m__offset',   seg(c, 'm__offset[64] = {',    '};'), seg(r, 'M__OFFSET: [u16; 64] = [',     '];'), 64),
    ('m__exponent', seg(c, 'm__exponent[64] = {',  '};'), seg(r, 'M__EXPONENT: [u32; 64] = [',   '];'), 64),
]
bad = 0
for name, a, b, n in tables:
    ok = (len(a) == len(b) == n) and a == b
    print(f'  {name:12s} C={len(a):5d} Rust={len(b):5d} expected={n:5d}  {"IDENTICAL" if ok else "MISMATCH"}')
    if not ok:
        bad = 1
        for k, (x, y) in enumerate(zip(a, b)):
            if x != y:
                print(f'    idx {k}: C=0x{x:08x} Rust=0x{y:08x}')
sys.exit(bad)
PY

echo
if [[ -z "$MISSING" && -z "$SUSPICIOUS" ]]; then
  echo "SYMBOL PARITY: OK (diff empty)"
else
  echo "SYMBOL PARITY: FAILED"; exit 1
fi

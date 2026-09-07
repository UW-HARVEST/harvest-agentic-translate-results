#!/usr/bin/env bash
# Harness-validation (mutation testing).
#
# A differential suite that passes tells you nothing unless it can also FAIL.
# This script builds deliberately-wrong `driver` implementations, points the
# test suite at each via RUST_DRIVER_SO, and checks that the suite rejects them.
#
# Usage: ./scripts/mutation_check.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(dirname "$HERE")"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
FAILED=0

export C_DRIVER_SO="$ROOT/c_src/build/libdriver.so"
export RUST_TEST_THREADS=1
[ -f "$C_DRIVER_SO" ] || { echo "build the C library first"; exit 1; }

# --- Mutant A: hand-rolled strcspn ------------------------------------------
# Agrees with libc on every well-formed input, but scans s1 in the outer loop,
# so it never dereferences s2 when s1 is empty. driver("", NULL) therefore
# prints 0 instead of faulting. This was the ACTUAL bug in the translation.
cat > "$WORK/a.rs" <<'EOF'
use std::ffi::{c_char, c_int};
extern "C" { fn printf(f: *const c_char, ...) -> c_int; }
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    let mut p = s1;
    loop {
        let c = *p; if c == 0 { break; }
        let mut q = s2;
        loop { let d = *q; if d == 0 { break; }
               if d == c { return p.offset_from(s1) as usize; } q = q.add(1); }
        p = p.add(1);
    }
    p.offset_from(s1) as usize
}
#[no_mangle] pub unsafe extern "C" fn driver(a: *const c_char, b: *const c_char) {
    printf(b"%zu\n\0".as_ptr() as *const c_char, strcspn(a, b));
}
EOF

# --- Mutant B: off-by-one that only shows up on large results ---------------
# Invisible to hand-picked short inputs; caught by the size-sweep rows.
cat > "$WORK/b.rs" <<'EOF'
use std::ffi::{c_char, c_int};
extern "C" { fn printf(f: *const c_char, ...) -> c_int;
             fn strcspn(a: *const c_char, b: *const c_char) -> usize; }
#[no_mangle] pub unsafe extern "C" fn driver(a: *const c_char, b: *const c_char) {
    let n = strcspn(a, b);
    let n = if n > 1000 { n - 1 } else { n };
    printf(b"%zu\n\0".as_ptr() as *const c_char, n);
}
EOF

# --- Mutant C: signed-char comparison (sign-extension) bug -----------------
# Only differs on bytes >= 0x80; caught by the high-byte rows.
cat > "$WORK/c.rs" <<'EOF'
use std::ffi::{c_char, c_int};
extern "C" { fn printf(f: *const c_char, ...) -> c_int; }
unsafe fn strcspn_signed(s1: *const c_char, s2: *const c_char) -> usize {
    let _ = *s2; // touch s2 first, like glibc, so mutant A's bug is absent
    let mut p = s1;
    loop {
        let c = *p as i32; if c == 0 { break; }   // sign-extends high bytes
        let mut q = s2;
        loop { let d = *q as i32; if d == 0 { break; }
               if (d & 0x7f) == (c & 0x7f) { return p.offset_from(s1) as usize; }
               q = q.add(1); }
        p = p.add(1);
    }
    p.offset_from(s1) as usize
}
#[no_mangle] pub unsafe extern "C" fn driver(a: *const c_char, b: *const c_char) {
    printf(b"%zu\n\0".as_ptr() as *const c_char, strcspn_signed(a, b));
}
EOF

# Locate the compiled test binaries (built by `cargo test --release --no-run`).
cd "$HERE" || exit 1
cargo test --offline --release --no-run >/dev/null 2>&1
find_bin() { ls -t "$HERE"/target/release/deps/$1-* 2>/dev/null | grep -v '\.d$' | head -1; }
PB="$(find_bin phase_b_valid)"
PC="$(find_bin phase_c_errors)"
[ -x "$PB" ] && [ -x "$PC" ] || { echo "could not locate test binaries"; exit 1; }

for m in a b c; do
  rustc --crate-type cdylib -O -o "$WORK/libmut_$m.so" "$WORK/$m.rs" 2>/dev/null \
    || { echo "could not build mutant $m"; FAILED=1; continue; }
  export RUST_DRIVER_SO="$WORK/libmut_$m.so"

  bout="$("$PB" 2>&1)"; brc=$?
  cout="$("$PC" 2>&1)"; crc=$?
  nb=$(printf '%s\n' "$bout" | grep -c '\.\.\. FAILED')
  nc=$(printf '%s\n' "$cout" | grep -c '\.\.\. FAILED')

  if [ "$brc" -eq 0 ] && [ "$crc" -eq 0 ]; then
    echo "MUTANT $m: NOT DETECTED  <-- the suite has a blind spot here"
    FAILED=1
  else
    echo "MUTANT $m: detected (phase B: $nb failing, phase C: $nc failing)"
  fi
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "All mutants detected: the differential suite has real discriminating power."
else
  echo "At least one mutant survived."
fi
exit "$FAILED"

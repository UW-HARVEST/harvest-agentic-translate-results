#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Negative control for the differential test suite ("do the tests bite?").
#
# Builds several deliberately WRONG Rust implementations of `custom_strdup`,
# points the test harness at each one via RUST_DRIVER_SO, and asserts the suite
# FAILS for every one of them. A suite that passes against a broken .so proves
# nothing, so this script is what makes the green run meaningful.
#
# It also asserts the suite PASSES against the real translation.
#
# The mutants live under translation/target/ so they are inside the tree the
# test process can read, and are never part of the shipped crate.
# ---------------------------------------------------------------------------
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MUT="$CRATE_DIR/target/mutants"
TESTS=(valid_paths error_paths malloc_failure)

rm -rf "$MUT"
mkdir -p "$MUT"

emit_crate() { # $1 = mutant name, stdin = src/lib.rs
  local d="$MUT/$1"
  mkdir -p "$d/src" "$d/.cargo"
  cat > "$d/Cargo.toml" <<'TOML'
[package]
name = "driver"
version = "0.1.0"
edition = "2021"

[lib]
name = "driver"
path = "src/lib.rs"
crate-type = ["cdylib"]

[workspace]
TOML
  printf '[net]\noffline = true\n' > "$d/.cargo/config.toml"
  cat > "$d/src/lib.rs"
}

# --- m1: returns the input pointer instead of a fresh copy (aliasing bug) ----
emit_crate m1 <<'EOF'
use std::ffi::c_char;
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    s as *mut c_char
}
EOF

# --- m2: uses the RUST global allocator (aborts on OOM, not NULL) ------------
emit_crate m2 <<'EOF'
use std::ffi::c_char;
extern "C" { fn strlen(s: *const c_char) -> usize; }
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() { return std::ptr::null_mut(); }
    let len = strlen(s) + 1;
    let mut v: Vec<u8> = Vec::with_capacity(len);
    std::ptr::copy_nonoverlapping(s as *const u8, v.as_mut_ptr(), len);
    v.set_len(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p as *mut c_char
}
EOF

# --- m3: off-by-one; drops the NUL terminator -------------------------------
emit_crate m3 <<'EOF'
use std::ffi::{c_char, c_void};
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() { return std::ptr::null_mut(); }
    let len = strlen(s);
    let n = malloc(len) as *mut c_char;
    if n.is_null() { return std::ptr::null_mut(); }
    memcpy(n as *mut c_void, s as *const c_void, len);
    n
}
EOF

# --- m4: invents a rejection for the empty string ---------------------------
emit_crate m4 <<'EOF'
use std::ffi::{c_char, c_void};
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() { return std::ptr::null_mut(); }
    let len = strlen(s);
    if len == 0 { return std::ptr::null_mut(); }
    let n = malloc(len + 1) as *mut c_char;
    if n.is_null() { return std::ptr::null_mut(); }
    memcpy(n as *mut c_void, s as *const c_void, len + 1);
    n
}
EOF

# --- m5: wrong sentinel on the NULL-input path ------------------------------
emit_crate m5 <<'EOF'
use std::ffi::{c_char, c_void};
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() { return 1usize as *mut c_char; }
    let len = strlen(s) + 1;
    let n = malloc(len) as *mut c_char;
    if n.is_null() { return std::ptr::null_mut(); }
    memcpy(n as *mut c_void, s as *const c_void, len);
    n
}
EOF

# --- m6: invents a maximum length cap ---------------------------------------
emit_crate m6 <<'EOF'
use std::ffi::{c_char, c_void};
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() { return std::ptr::null_mut(); }
    let len = strlen(s) + 1;
    if len > 65536 { return std::ptr::null_mut(); }
    let n = malloc(len) as *mut c_char;
    if n.is_null() { return std::ptr::null_mut(); }
    memcpy(n as *mut c_void, s as *const c_void, len);
    n
}
EOF

# --- m7: truncates high-bit bytes (signed-char / UTF-8 style corruption) ----
emit_crate m7 <<'EOF'
use std::ffi::{c_char, c_void};
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() { return std::ptr::null_mut(); }
    let len = strlen(s) + 1;
    let n = malloc(len) as *mut c_char;
    if n.is_null() { return std::ptr::null_mut(); }
    memcpy(n as *mut c_void, s as *const c_void, len);
    // mask off the high bit of every payload byte
    for i in 0..(len - 1) {
        let b = *(n.add(i)) as u8;
        *(n.add(i)) = (b & 0x7F) as c_char;
    }
    n
}
EOF

echo "=== building mutants ==="
MUTANTS=(m1 m2 m3 m4 m5 m6 m7)
for m in "${MUTANTS[@]}"; do
  ( cd "$MUT/$m" && cargo build --release -q ) \
    || { echo "FATAL: mutant $m failed to build"; exit 1; }
  so="$MUT/$m/target/release/libdriver.so"
  [ -f "$so" ] || { echo "FATAL: $so missing"; exit 1; }
  nm -D --defined-only "$so" | grep -q ' T custom_strdup' \
    || { echo "FATAL: mutant $m does not export custom_strdup"; exit 1; }
  echo "  built $m"
done

echo
echo "=== building test binaries ==="
( cd "$CRATE_DIR" && cargo test --no-run -q ) || exit 1

overall=0

echo
echo "=== control: the REAL translation must PASS ==="
for t in "${TESTS[@]}"; do
  if ( cd "$CRATE_DIR" && timeout 600 cargo test -q --test "$t" >/dev/null 2>&1 ); then
    echo "  PASS (expected)  real/$t"
  else
    echo "  *** FAIL (UNEXPECTED) real/$t — the real translation must pass!"
    overall=1
  fi
done

echo
echo "=== mutants: every one must be KILLED (suite must fail) ==="
for m in "${MUTANTS[@]}"; do
  so="$MUT/$m/target/release/libdriver.so"
  killed_by=()
  survived_in=()
  for t in "${TESTS[@]}"; do
    if ( cd "$CRATE_DIR" && RUST_DRIVER_SO="$so" timeout 600 cargo test -q --test "$t" >/dev/null 2>&1 ); then
      survived_in+=("$t")
    else
      killed_by+=("$t")
    fi
  done
  if [ ${#killed_by[@]} -gt 0 ]; then
    echo "  KILLED  $m  by: ${killed_by[*]}"
  else
    echo "  *** SURVIVED $m — the suite does not detect this bug!"
    overall=1
  fi
  [ ${#survived_in[@]} -gt 0 ] && echo "            (undetected by: ${survived_in[*]})"
done

echo
if [ $overall -eq 0 ]; then
  echo "MUTATION CHECK: OK — real translation passes, all mutants killed."
else
  echo "MUTATION CHECK: PROBLEM (see *** lines above)"
fi
exit $overall

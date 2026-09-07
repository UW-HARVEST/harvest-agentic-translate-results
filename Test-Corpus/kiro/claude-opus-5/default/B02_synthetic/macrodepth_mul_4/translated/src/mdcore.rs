// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `src/mdcore.c`.
//!
//! `mdmacros.h` declares no namespace/renaming macros, so every exported symbol
//! keeps its source-level spelling: `op_add`, `op_sub`, `op_mul`, `G_OP`,
//! `G_OP_NAME`, `helper_call`, `helper_ptr`, `use_generated`. `accum_<OP>`
//! is `static` in C (via `DEFINE_ACCUM`) and stays private here.

use core::ffi::{c_char, c_int};

use crate::mdmacros::{dispatch_rep, run_loop, INIT, OP_FN, OP_NAME_C};

// ---------------------------------------------------------------------------
// Define operations
// ---------------------------------------------------------------------------

/// `int op_add(int a, int b) { return a + b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

/// `int op_sub(int a, int b) { return a - b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// `int op_mul(int a, int b) { return a * b; }`
#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

// ---------------------------------------------------------------------------
// DEFINE_ACCUM(OP)  ->  static int accum_<OP>(int n)
// ---------------------------------------------------------------------------

/// `static int accum_<OP>(int n) { int acc = INIT_FOR(OP); DISPATCH_REP(OP, acc, n); return acc; }`
fn accum(n: c_int) -> c_int {
    let acc: c_int = INIT;
    dispatch_rep(acc, n)
}

// ---------------------------------------------------------------------------
// Global macro uses at file scope
// ---------------------------------------------------------------------------

/// Storage backing `G_OP_NAME`, standing in for the string literal that
/// `STR(OP)` expands to.
static OP_NAME_STORAGE: [u8; 4] = *OP_NAME_C;

/// `const char *` in a `static`: a bare raw pointer is not `Sync`, so it is
/// wrapped in a `repr(transparent)` newtype. The exported symbol is still a
/// single pointer-sized slot holding the address of the name, identical to what
/// the C compiler emits for `const char *G_OP_NAME = STR(OP);`.
#[repr(transparent)]
pub struct CStrPtr(pub *const c_char);

// SAFETY: the pointer targets `OP_NAME_STORAGE`, an immutable `static` that
// lives for the whole program and is never written, so sharing it across threads
// is sound. C exposes the same global with no synchronization at all.
unsafe impl Sync for CStrPtr {}

/// `int (*G_OP)(int,int) = OP_FN(OP);`
///
/// Mutable to match the non-`const` C global; nothing in the program writes it.
#[unsafe(no_mangle)]
pub static mut G_OP: extern "C" fn(c_int, c_int) -> c_int = OP_FN;

/// `const char *G_OP_NAME = STR(OP);`
#[unsafe(no_mangle)]
pub static G_OP_NAME: CStrPtr = CStrPtr(&OP_NAME_STORAGE as *const [u8; 4] as *const c_char);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// ```c
/// int helper_call(int a, int b) {
///     int r = (OP_FN(OP))(a, b);
///     int acc = INIT_FOR(OP);
///     RUN_LOOP(OP, acc, REPEAT);
///     printf("helper.call=%d helper.acc=%d\n", r, acc);
///     return r + acc;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = OP_FN(a, b);
    let acc = run_loop(INIT);
    crate::stdio::print_str(&format!("helper.call={} helper.acc={}\n", r, acc));
    r.wrapping_add(acc)
}

/// ```c
/// int helper_ptr(int a, int b) {
///     int (*fp)(int,int) = OP_FN(OP);
///     int r = fp(a, b);
///     printf("helper.ptr=%d\n", r);
///     return r;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = OP_FN;
    let r = fp(a, b);
    crate::stdio::print_str(&format!("helper.ptr={}\n", r));
    r
}

/// ```c
/// int use_generated(int n) {
///     int r = (ACCUM_FN(OP))(n);
///     printf("gen.acc=%d\n", r);
///     return r;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum(n);
    crate::stdio::print_str(&format!("gen.acc={}\n", r));
    r
}

/// Reads `G_OP` the way `mdmain.c` does (`int g = G_OP(a, b);`).
///
/// Exists so the driver can go through the global without an `unsafe` block of
/// its own; the `static mut` read is confined here.
#[inline]
pub fn g_op() -> extern "C" fn(c_int, c_int) -> c_int {
    // SAFETY: `G_OP` is initialized at load time and never mutated, so this read
    // cannot observe a torn or uninitialized value.
    unsafe { G_OP }
}

/// Borrows `G_OP_NAME` as bytes for `%s` formatting.
#[inline]
pub fn g_op_name_bytes() -> &'static [u8] {
    &OP_NAME_STORAGE[..OP_NAME_STORAGE.len() - 1]
}

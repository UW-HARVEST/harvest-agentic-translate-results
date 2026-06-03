// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/mdcore.c
//
// Build-time configurability is preserved via Cargo features:
//   OP    -> features `add`, `sub`, `mul`
//   REPEAT -> features `0`, `1`, `2`, `3`, `4`, `5`, `6`, `7`

use std::ffi::{c_char, c_int};

// -----------------------------------------------------------------------------
// Operation family (defined in md_core.c)
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

// -----------------------------------------------------------------------------
// REPEAT constant (selected by feature)
// -----------------------------------------------------------------------------

#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;
#[cfg(feature = "1")]
pub const REPEAT: c_int = 1;
#[cfg(feature = "2")]
pub const REPEAT: c_int = 2;
#[cfg(feature = "3")]
pub const REPEAT: c_int = 3;
#[cfg(feature = "4")]
pub const REPEAT: c_int = 4;
#[cfg(feature = "5")]
pub const REPEAT: c_int = 5;
#[cfg(feature = "6")]
pub const REPEAT: c_int = 6;
#[cfg(feature = "7")]
pub const REPEAT: c_int = 7;

// -----------------------------------------------------------------------------
// OP-specific items (selected by feature):
//   OP_FN_PTR : the chosen operation function pointer
//   INIT_FOR  : starting accumulator value
//   step_op   : applies STEP_<op>(acc, i)
//   OP_NAME_STR    : Rust &str for printing
//   G_OP_NAME_BYTES: NUL-terminated bytes for the C global
// -----------------------------------------------------------------------------

#[cfg(feature = "add")]
mod op_sel {
    use super::*;
    pub const OP_FN_PTR: extern "C" fn(c_int, c_int) -> c_int = op_add;
    pub const INIT_FOR: c_int = 0;
    pub const OP_NAME_STR: &str = "add";
    pub const G_OP_NAME_BYTES: &[u8] = b"add\0";
    #[inline]
    pub fn step_op(acc: &mut c_int, i: c_int) {
        *acc = acc.wrapping_add(i);
    }
}

#[cfg(feature = "sub")]
mod op_sel {
    use super::*;
    pub const OP_FN_PTR: extern "C" fn(c_int, c_int) -> c_int = op_sub;
    pub const INIT_FOR: c_int = 0;
    pub const OP_NAME_STR: &str = "sub";
    pub const G_OP_NAME_BYTES: &[u8] = b"sub\0";
    #[inline]
    pub fn step_op(acc: &mut c_int, i: c_int) {
        *acc = acc.wrapping_sub(i);
    }
}

#[cfg(feature = "mul")]
mod op_sel {
    use super::*;
    pub const OP_FN_PTR: extern "C" fn(c_int, c_int) -> c_int = op_mul;
    pub const INIT_FOR: c_int = 1;
    pub const OP_NAME_STR: &str = "mul";
    pub const G_OP_NAME_BYTES: &[u8] = b"mul\0";
    #[inline]
    pub fn step_op(acc: &mut c_int, i: c_int) {
        *acc = acc.wrapping_mul(i.wrapping_add(1));
    }
}

pub use op_sel::{INIT_FOR, OP_FN_PTR, OP_NAME_STR};

// -----------------------------------------------------------------------------
// RUN_LOOP equivalent: applies STEP_OP for i = 0..REPEAT-1
// (The C macro CHOOSE_REP(REPEAT) unrolls this; the result is identical.)
// -----------------------------------------------------------------------------

#[inline]
pub fn run_loop(acc: &mut c_int) {
    let mut i: c_int = 0;
    while i < REPEAT {
        op_sel::step_op(acc, i);
        i += 1;
    }
}

// -----------------------------------------------------------------------------
// Macro-generated accumulator: accum_<OP>(n)
//
// In C, DISPATCH_REP uses a `switch (n)` with explicit cases 0..6 and a
// `default` branch that does NOTHING.  This means accum_<OP>(7) returns the
// initial accumulator value untouched, even though REP7 exists in the macro
// table.  This quirk MUST be preserved.
// -----------------------------------------------------------------------------

fn accum_op(n: c_int) -> c_int {
    let mut acc: c_int = INIT_FOR;
    if (0..=6).contains(&n) {
        let mut i: c_int = 0;
        while i < n {
            op_sel::step_op(&mut acc, i);
            i += 1;
        }
    }
    // default branch in C `switch`: do nothing
    acc
}

// -----------------------------------------------------------------------------
// Globals: G_OP and G_OP_NAME (exported with C linkage)
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = OP_FN_PTR;

// Wrap raw pointer in a #[repr(transparent)] type so we can place it in a
// `static` (raw pointers don't implement Sync by default).
#[repr(transparent)]
pub struct CCharPtr(pub *const c_char);
unsafe impl Sync for CCharPtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: CCharPtr =
    CCharPtr(op_sel::G_OP_NAME_BYTES.as_ptr() as *const c_char);

// -----------------------------------------------------------------------------
// Public helpers (extern "C")
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = OP_FN_PTR(a, b);
    let mut acc: c_int = INIT_FOR;
    run_loop(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = OP_FN_PTR;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum_op(n);
    println!("gen.acc={}", r);
    r
}

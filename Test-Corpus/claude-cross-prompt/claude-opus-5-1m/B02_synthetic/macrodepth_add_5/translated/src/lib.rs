// Rust translation of c_src/src/mdcore.c (and the macros from mdmacros.h).
//
// CMake cache variables -> Cargo features (lowercase, exact name):
//   OP    -> features "add", "sub", "mul"      (default: "add")
//   REPEAT-> features "0".."7"                 (default: "5")
//
// Only one OP feature and one REPEAT feature are expected at a time. If
// multiple are enabled the priority below picks one deterministically so the
// crate still compiles for any combination.
//
// Priority for OP (highest first):  sub > mul > add (default)
// Priority for REPEAT (highest first): 7 > 6 > 5 > 4 > 3 > 2 > 1 > 0 ; default 5

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int};

// =========================================================================
//                         OP selection (compile-time)
// =========================================================================

// --------- accumulator init ---------
#[cfg(all(feature = "mul", not(feature = "sub")))]
const OP_INIT: c_int = 1;
#[cfg(not(all(feature = "mul", not(feature = "sub"))))]
const OP_INIT: c_int = 0;

// --------- per-step macro body ---------
//   STEP_add(acc, i): acc += i
//   STEP_sub(acc, i): acc -= i
//   STEP_mul(acc, i): acc *= (i + 1)
#[inline(always)]
fn step_op(acc: c_int, i: c_int) -> c_int {
    #[cfg(feature = "sub")]
    {
        acc.wrapping_sub(i)
    }
    #[cfg(all(feature = "mul", not(feature = "sub")))]
    {
        acc.wrapping_mul(i.wrapping_add(1))
    }
    #[cfg(all(not(feature = "sub"), not(feature = "mul")))]
    {
        acc.wrapping_add(i)
    }
}

// --------- selected OP function pointer + textual name ---------
#[cfg(feature = "sub")]
const SELECTED_OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_sub;
#[cfg(all(feature = "mul", not(feature = "sub")))]
const SELECTED_OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_mul;
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
const SELECTED_OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_add;

#[cfg(feature = "sub")]
pub const SELECTED_OP_NAME: &str = "sub";
#[cfg(all(feature = "mul", not(feature = "sub")))]
pub const SELECTED_OP_NAME: &str = "mul";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
pub const SELECTED_OP_NAME: &str = "add";

// NUL-terminated C string for G_OP_NAME (matches `const char *` in C).
#[cfg(feature = "sub")]
const SELECTED_OP_NAME_CSTR: &[u8] = b"sub\0";
#[cfg(all(feature = "mul", not(feature = "sub")))]
const SELECTED_OP_NAME_CSTR: &[u8] = b"mul\0";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
const SELECTED_OP_NAME_CSTR: &[u8] = b"add\0";

// =========================================================================
//                       REPEAT selection (compile-time)
// =========================================================================

#[cfg(feature = "7")]
pub const REPEAT: c_int = 7;
#[cfg(all(feature = "6", not(feature = "7")))]
pub const REPEAT: c_int = 6;
#[cfg(all(feature = "5", not(feature = "7"), not(feature = "6")))]
pub const REPEAT: c_int = 5;
#[cfg(all(
    feature = "4",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5")
))]
pub const REPEAT: c_int = 4;
#[cfg(all(
    feature = "3",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4")
))]
pub const REPEAT: c_int = 3;
#[cfg(all(
    feature = "2",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3")
))]
pub const REPEAT: c_int = 2;
#[cfg(all(
    feature = "1",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2")
))]
pub const REPEAT: c_int = 1;
#[cfg(all(
    feature = "0",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    not(feature = "1")
))]
pub const REPEAT: c_int = 0;
// Fallback: no REPEAT feature enabled -> default to 5 (mirrors mdmacros.h)
#[cfg(all(
    not(feature = "0"),
    not(feature = "1"),
    not(feature = "2"),
    not(feature = "3"),
    not(feature = "4"),
    not(feature = "5"),
    not(feature = "6"),
    not(feature = "7")
))]
pub const REPEAT: c_int = 5;

// =========================================================================
//                            Operation primitives
// =========================================================================

// op_add / op_sub / op_mul are exported with C ABI by the cdylib.
// (mdcore.c declares them as `int op_add(int,int)` etc.)

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

// =========================================================================
//                  RUN_LOOP — REPEAT-many unrolled steps
// =========================================================================

/// Equivalent of `RUN_LOOP(OP, acc, REPEAT)` in mdmacros.h.
/// The C macro manually unrolls to STEP_OP(acc, 0); STEP_OP(acc, 1); ... ;
/// STEP_OP(acc, REPEAT-1).  A plain loop produces identical numeric results.
#[inline]
pub fn run_loop(acc: &mut c_int) {
    let mut i: c_int = 0;
    while i < REPEAT {
        *acc = step_op(*acc, i);
        i += 1;
    }
}

// =========================================================================
//   accum_<OP>(n)  — generated via DEFINE_ACCUM(OP) in mdcore.c
// =========================================================================
//
// DISPATCH_REP switches on the runtime value n: cases 0..=6 each unroll that
// many STEP_OP invocations, default does nothing. (Note: `case 7` has NO
// case label — it falls into `default` and leaves acc at INIT.)
fn accum_op(n: c_int) -> c_int {
    let mut acc: c_int = OP_INIT;
    match n {
        0 => { /* REP0: nothing */ }
        1 => {
            acc = step_op(acc, 0);
        }
        2 => {
            acc = step_op(acc, 0);
            acc = step_op(acc, 1);
        }
        3 => {
            acc = step_op(acc, 0);
            acc = step_op(acc, 1);
            acc = step_op(acc, 2);
        }
        4 => {
            acc = step_op(acc, 0);
            acc = step_op(acc, 1);
            acc = step_op(acc, 2);
            acc = step_op(acc, 3);
        }
        5 => {
            acc = step_op(acc, 0);
            acc = step_op(acc, 1);
            acc = step_op(acc, 2);
            acc = step_op(acc, 3);
            acc = step_op(acc, 4);
        }
        6 => {
            acc = step_op(acc, 0);
            acc = step_op(acc, 1);
            acc = step_op(acc, 2);
            acc = step_op(acc, 3);
            acc = step_op(acc, 4);
            acc = step_op(acc, 5);
        }
        _ => { /* default: no-op */ }
    }
    acc
}

// =========================================================================
//                              Globals
// =========================================================================
// `int (*G_OP)(int,int) = OP_FN(OP);`  and  `const char *G_OP_NAME = STR(OP);`

#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = SELECTED_OP_FN;

// Wrapper to make `*const c_char` Sync so it can live in a `static`.
#[repr(transparent)]
pub struct SyncCharPtr(pub *const c_char);
unsafe impl Sync for SyncCharPtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: SyncCharPtr =
    SyncCharPtr(SELECTED_OP_NAME_CSTR.as_ptr() as *const c_char);

// =========================================================================
//                           Helper functions
// =========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = SELECTED_OP_FN(a, b);
    let mut acc: c_int = OP_INIT;
    run_loop(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = SELECTED_OP_FN;
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

// Small helpers used by the binary entry point.
#[doc(hidden)]
pub fn selected_op_fn() -> extern "C" fn(c_int, c_int) -> c_int {
    SELECTED_OP_FN
}

#[doc(hidden)]
pub fn selected_op_init() -> c_int {
    OP_INIT
}

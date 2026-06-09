// Copyright 2025 MIT Lincoln Laboratory
// Translation of mdcore.c / mdmacros.h to Rust.

use std::ffi::{c_char, c_int};

// Compile-time validation: exactly one OP feature must be selected.
#[cfg(not(any(feature = "op_add", feature = "op_sub", feature = "op_mul")))]
compile_error!("exactly one of features op_add, op_sub, op_mul must be enabled");

// Compile-time validation: exactly one REPEAT feature must be selected.
#[cfg(not(any(
    feature = "repeat_0",
    feature = "repeat_1",
    feature = "repeat_2",
    feature = "repeat_3",
    feature = "repeat_4",
    feature = "repeat_5",
    feature = "repeat_6",
    feature = "repeat_7",
)))]
compile_error!("exactly one of features repeat_0..repeat_7 must be enabled");

// REPEAT cache variable
#[cfg(feature = "repeat_0")]
pub const REPEAT: c_int = 0;
#[cfg(feature = "repeat_1")]
pub const REPEAT: c_int = 1;
#[cfg(feature = "repeat_2")]
pub const REPEAT: c_int = 2;
#[cfg(feature = "repeat_3")]
pub const REPEAT: c_int = 3;
#[cfg(feature = "repeat_4")]
pub const REPEAT: c_int = 4;
#[cfg(feature = "repeat_5")]
pub const REPEAT: c_int = 5;
#[cfg(feature = "repeat_6")]
pub const REPEAT: c_int = 6;
#[cfg(feature = "repeat_7")]
pub const REPEAT: c_int = 7;

// OP_NAME cache variable (the stringified OP)
#[cfg(feature = "op_add")]
pub const OP_NAME: &str = "add";
#[cfg(feature = "op_sub")]
pub const OP_NAME: &str = "sub";
#[cfg(feature = "op_mul")]
pub const OP_NAME: &str = "mul";

// ----- Operation primitives (op_add / op_sub / op_mul) -----

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

// ----- The selected operation (OP_FN(OP)) -----

#[inline]
pub fn op_selected(a: c_int, b: c_int) -> c_int {
    #[cfg(feature = "op_add")]
    {
        op_add(a, b)
    }
    #[cfg(feature = "op_sub")]
    {
        op_sub(a, b)
    }
    #[cfg(feature = "op_mul")]
    {
        op_mul(a, b)
    }
}

/// The accumulator initial value for the selected OP (INIT_FOR(OP)).
#[inline]
pub fn init_for_selected() -> c_int {
    #[cfg(feature = "op_add")]
    {
        0
    }
    #[cfg(feature = "op_sub")]
    {
        0
    }
    #[cfg(feature = "op_mul")]
    {
        1
    }
}

/// Single step for the selected OP (STEP_OP(OP, acc, i)).
#[inline]
pub fn step_selected(acc: &mut c_int, i: c_int) {
    #[cfg(feature = "op_add")]
    {
        *acc = (*acc).wrapping_add(i);
    }
    #[cfg(feature = "op_sub")]
    {
        *acc = (*acc).wrapping_sub(i);
    }
    #[cfg(feature = "op_mul")]
    {
        *acc = (*acc).wrapping_mul(i.wrapping_add(1));
    }
}

/// RUN_LOOP(OP, acc, n): manual unrolling chosen statically by REPEAT (CHOOSE_REP(REPEAT)).
#[inline]
pub fn run_loop_selected(acc: &mut c_int) {
    // The unroll length is fixed at compile time by REPEAT.
    let n: c_int = REPEAT;
    let mut i: c_int = 0;
    while i < n {
        step_selected(acc, i);
        i += 1;
    }
}

/// DISPATCH_REP(OP, acc, n): switch on n and execute REP<n>.
#[inline]
fn dispatch_rep_selected(acc: &mut c_int, n: c_int) {
    match n {
        0 => { /* nothing */ }
        1 => {
            step_selected(acc, 0);
        }
        2 => {
            step_selected(acc, 0);
            step_selected(acc, 1);
        }
        3 => {
            step_selected(acc, 0);
            step_selected(acc, 1);
            step_selected(acc, 2);
        }
        4 => {
            step_selected(acc, 0);
            step_selected(acc, 1);
            step_selected(acc, 2);
            step_selected(acc, 3);
        }
        5 => {
            step_selected(acc, 0);
            step_selected(acc, 1);
            step_selected(acc, 2);
            step_selected(acc, 3);
            step_selected(acc, 4);
        }
        6 => {
            step_selected(acc, 0);
            step_selected(acc, 1);
            step_selected(acc, 2);
            step_selected(acc, 3);
            step_selected(acc, 4);
            step_selected(acc, 5);
        }
        // Note: the C `DISPATCH_REP` switch only covers cases 0..=6.
        // For any other value of n (including 7), the default branch is
        // taken and the accumulator is left unchanged — so we mirror that.
        _ => { /* nothing */ }
    }
}

/// Macro-generated accumulator function: accum_<OP>(n).
fn accum_selected(n: c_int) -> c_int {
    let mut acc = init_for_selected();
    dispatch_rep_selected(&mut acc, n);
    acc
}

// ----- Globals (G_OP, G_OP_NAME) -----

// G_OP: function pointer to the selected op.
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = {
    #[cfg(feature = "op_add")]
    {
        op_add
    }
    #[cfg(feature = "op_sub")]
    {
        op_sub
    }
    #[cfg(feature = "op_mul")]
    {
        op_mul
    }
};

// G_OP_NAME: NUL-terminated C string holding the stringified OP.
#[cfg(feature = "op_add")]
const G_OP_NAME_CSTR: &[u8] = b"add\0";
#[cfg(feature = "op_sub")]
const G_OP_NAME_CSTR: &[u8] = b"sub\0";
#[cfg(feature = "op_mul")]
const G_OP_NAME_CSTR: &[u8] = b"mul\0";

#[repr(transparent)]
pub struct OpNamePtr(pub *const c_char);
unsafe impl Sync for OpNamePtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: OpNamePtr = OpNamePtr(G_OP_NAME_CSTR.as_ptr() as *const c_char);

// ----- Helpers (printing functions) -----

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = op_selected(a, b);
    let mut acc = init_for_selected();
    run_loop_selected(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: extern "C" fn(c_int, c_int) -> c_int = {
        #[cfg(feature = "op_add")]
        {
            op_add
        }
        #[cfg(feature = "op_sub")]
        {
            op_sub
        }
        #[cfg(feature = "op_mul")]
        {
            op_mul
        }
    };
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum_selected(n);
    println!("gen.acc={}", r);
    r
}

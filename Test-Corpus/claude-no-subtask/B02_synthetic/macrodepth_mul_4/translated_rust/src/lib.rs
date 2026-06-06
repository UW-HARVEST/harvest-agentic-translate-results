// Translation of c_src/src/mdcore.c and mdmacros.h to Rust.
// Preserves the C linker symbols and observable behavior.

use std::ffi::c_char;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Build-time configurability via Cargo features (mirroring CMake cache vars).
// OP feature: "add" (default), "sub", "mul".
// REPEAT feature: "0".."7" (default "5").
// ---------------------------------------------------------------------------

// OP selection. Use precedence so any combination of features compiles.
#[cfg(feature = "mul")]
pub const OP_NAME: &str = "mul";
#[cfg(all(feature = "sub", not(feature = "mul")))]
pub const OP_NAME: &str = "sub";
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
pub const OP_NAME: &str = "add";

// REPEAT selection (precedence: highest set wins; default 5 if none).
#[cfg(feature = "7")]
pub const REPEAT: i32 = 7;
#[cfg(all(feature = "6", not(feature = "7")))]
pub const REPEAT: i32 = 6;
#[cfg(all(
    feature = "5",
    not(feature = "7"),
    not(feature = "6")
))]
pub const REPEAT: i32 = 5;
#[cfg(all(
    feature = "4",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5")
))]
pub const REPEAT: i32 = 4;
#[cfg(all(
    feature = "3",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4")
))]
pub const REPEAT: i32 = 3;
#[cfg(all(
    feature = "2",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3")
))]
pub const REPEAT: i32 = 2;
#[cfg(all(
    feature = "1",
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2")
))]
pub const REPEAT: i32 = 1;
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
pub const REPEAT: i32 = 0;
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
pub const REPEAT: i32 = 5;

// ---------------------------------------------------------------------------
// Operation primitives (mirror op_add / op_sub / op_mul in C).
// All use wrapping arithmetic to match C's signed int wrap-on-overflow
// behavior in practice for typical inputs.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// STEP / RUN_LOOP / INIT_FOR equivalents (compile-time selection of OP).
// ---------------------------------------------------------------------------

/// Initial accumulator value for the configured OP.
#[inline]
pub fn init_for_op() -> i32 {
    #[cfg(feature = "mul")]
    {
        return 1;
    }
    #[cfg(all(feature = "sub", not(feature = "mul")))]
    {
        return 0;
    }
    #[cfg(all(not(feature = "mul"), not(feature = "sub")))]
    {
        return 0;
    }
}

/// Apply one STEP_<OP> using index `i` to the accumulator (in place).
#[inline]
pub fn step_op(acc: &mut i32, i: i32) {
    #[cfg(feature = "mul")]
    {
        *acc = acc.wrapping_mul(i.wrapping_add(1));
        return;
    }
    #[cfg(all(feature = "sub", not(feature = "mul")))]
    {
        *acc = acc.wrapping_sub(i);
        return;
    }
    #[cfg(all(not(feature = "mul"), not(feature = "sub")))]
    {
        *acc = acc.wrapping_add(i);
    }
}

/// Equivalent of `RUN_LOOP(OP, acc, REPEAT)` — the unrolled CHOOSE_REP(n).
#[inline]
pub fn run_loop(acc: &mut i32) {
    // REP<n> performs steps for i = 0..n-1.
    for i in 0..REPEAT {
        step_op(acc, i);
    }
}

/// Equivalent of `accum_<OP>(n)` produced by DEFINE_ACCUM(OP).
/// In C this is a static function with a switch over n in 0..=6;
/// any other n falls through to the default branch (no steps).
pub fn accum_op(n: i32) -> i32 {
    let mut acc = init_for_op();
    if (0..=6).contains(&n) {
        for i in 0..n {
            step_op(&mut acc, i);
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// Selected OP function pointer (G_OP) and its name (G_OP_NAME).
// ---------------------------------------------------------------------------

#[cfg(feature = "mul")]
const SELECTED_OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_mul;
#[cfg(all(feature = "sub", not(feature = "mul")))]
const SELECTED_OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_sub;
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
const SELECTED_OP_FN: extern "C" fn(c_int, c_int) -> c_int = op_add;

/// The C global `int (*G_OP)(int,int) = OP_FN(OP);`
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = SELECTED_OP_FN;

// G_OP_NAME — a NUL-terminated C string with the OP name.
#[cfg(feature = "mul")]
static G_OP_NAME_STORAGE: [u8; 4] = *b"mul\0";
#[cfg(all(feature = "sub", not(feature = "mul")))]
static G_OP_NAME_STORAGE: [u8; 4] = *b"sub\0";
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
static G_OP_NAME_STORAGE: [u8; 4] = *b"add\0";

/// Pointer wrapper that's safe to share across threads. The pointee is an
/// immutable, statically-allocated NUL-terminated C string, so this is sound.
#[repr(transparent)]
pub struct CStrPtr(pub *const c_char);
unsafe impl Sync for CStrPtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: CStrPtr = CStrPtr(G_OP_NAME_STORAGE.as_ptr() as *const c_char);

// ---------------------------------------------------------------------------
// Helpers: helper_call, helper_ptr, use_generated.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = SELECTED_OP_FN(a, b);
    let mut acc = init_for_op();
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

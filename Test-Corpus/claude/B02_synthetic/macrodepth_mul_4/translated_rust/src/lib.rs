// Translated from C source by deterministic translation.
// Preserves byte-identical stdout for the same inputs as the original C binary.

use std::ffi::c_int;

// ---- Compile-time selection mirroring CMake cache variables ----

// REPEAT: pick exactly one of the numeric features.
#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;
#[cfg(all(feature = "1", not(feature = "0")))]
pub const REPEAT: c_int = 1;
#[cfg(all(feature = "2", not(any(feature = "0", feature = "1"))))]
pub const REPEAT: c_int = 2;
#[cfg(all(feature = "3", not(any(feature = "0", feature = "1", feature = "2"))))]
pub const REPEAT: c_int = 3;
#[cfg(all(feature = "4", not(any(feature = "0", feature = "1", feature = "2", feature = "3"))))]
pub const REPEAT: c_int = 4;
#[cfg(all(
    feature = "5",
    not(any(feature = "0", feature = "1", feature = "2", feature = "3", feature = "4"))
))]
pub const REPEAT: c_int = 5;
#[cfg(all(
    feature = "6",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4",
        feature = "5"
    ))
))]
pub const REPEAT: c_int = 6;
#[cfg(all(
    feature = "7",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4",
        feature = "5",
        feature = "6"
    ))
))]
pub const REPEAT: c_int = 7;

// Fallback (when no REPEAT feature is enabled at all): default to 5
// (matches the CMake default of REPEAT="5").
#[cfg(not(any(
    feature = "0",
    feature = "1",
    feature = "2",
    feature = "3",
    feature = "4",
    feature = "5",
    feature = "6",
    feature = "7"
)))]
pub const REPEAT: c_int = 5;

// OP: pick exactly one of {add, sub, mul} (default add when none enabled).
#[cfg(feature = "sub")]
pub const OP_NAME: &str = "sub";
#[cfg(all(feature = "mul", not(feature = "sub")))]
pub const OP_NAME: &str = "mul";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
pub const OP_NAME: &str = "add";

// ---- Operations ----

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

// Return the operation function pointer for the currently-selected OP.
#[inline]
fn selected_op() -> extern "C" fn(c_int, c_int) -> c_int {
    #[cfg(feature = "sub")]
    {
        op_sub
    }
    #[cfg(all(feature = "mul", not(feature = "sub")))]
    {
        op_mul
    }
    #[cfg(all(not(feature = "sub"), not(feature = "mul")))]
    {
        op_add
    }
}

// STEP_add(acc, i): acc += i
// STEP_sub(acc, i): acc -= i
// STEP_mul(acc, i): acc *= (i + 1)
#[inline]
fn step(acc: c_int, i: c_int) -> c_int {
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

// INIT_add 0, INIT_sub 0, INIT_mul 1
#[inline]
fn init_for() -> c_int {
    #[cfg(feature = "sub")]
    {
        0
    }
    #[cfg(all(feature = "mul", not(feature = "sub")))]
    {
        1
    }
    #[cfg(all(not(feature = "sub"), not(feature = "mul")))]
    {
        0
    }
}

// Apply n steps with i = 0..n-1.
#[inline]
fn apply_steps(acc: &mut c_int, n: c_int) {
    let mut i: c_int = 0;
    while i < n {
        *acc = step(*acc, i);
        i += 1;
    }
}

// Compile-time-unrolled RUN_LOOP using REPEAT.
#[inline]
fn run_loop(acc: &mut c_int) {
    apply_steps(acc, REPEAT);
}

// `accum_<OP>` — file-scope static in C, switching on n at runtime.
// We emit it under the same final symbol name `accum_<OP>` for parity.
#[inline]
fn accum_impl(n: c_int) -> c_int {
    let mut acc = init_for();
    // Mirrors DISPATCH_REP: switch over n in 0..=6, default no-op.
    match n {
        0 => {}
        1 => apply_steps(&mut acc, 1),
        2 => apply_steps(&mut acc, 2),
        3 => apply_steps(&mut acc, 3),
        4 => apply_steps(&mut acc, 4),
        5 => apply_steps(&mut acc, 5),
        6 => apply_steps(&mut acc, 6),
        _ => {}
    }
    acc
}

// ---- Helpers (extern "C" so they can be called from C as well) ----

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let f = selected_op();
    let r = f(a, b);
    let mut acc = init_for();
    run_loop(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp = selected_op();
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum_impl(n);
    println!("gen.acc={}", r);
    r
}

// ---- Global variables matching C `G_OP` and `G_OP_NAME` ----

// `G_OP_NAME` — NUL-terminated C string holding the OP name.
// In C this is `const char *G_OP_NAME = "add";` — a single pointer-sized
// global. We use `&'static [u8; N]` which is ABI-compatible with `const char *`.
#[cfg(feature = "sub")]
#[unsafe(no_mangle)]
pub static G_OP_NAME: &[u8; 4] = b"sub\0";
#[cfg(all(feature = "mul", not(feature = "sub")))]
#[unsafe(no_mangle)]
pub static G_OP_NAME: &[u8; 4] = b"mul\0";
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
#[unsafe(no_mangle)]
pub static G_OP_NAME: &[u8; 4] = b"add\0";

// `G_OP` — function pointer to the selected operation.
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = {
    #[cfg(feature = "sub")]
    {
        op_sub
    }
    #[cfg(all(feature = "mul", not(feature = "sub")))]
    {
        op_mul
    }
    #[cfg(all(not(feature = "sub"), not(feature = "mul")))]
    {
        op_add
    }
};

// ---- Public re-exports for use by the binary driver ----

#[inline]
pub fn driver_op(a: c_int, b: c_int) -> c_int {
    let f = selected_op();
    f(a, b)
}

#[inline]
pub fn driver_run_loop(acc: &mut c_int) {
    run_loop(acc);
}

#[inline]
pub fn driver_init_for() -> c_int {
    init_for()
}

#[inline]
pub fn driver_op_name() -> &'static str {
    OP_NAME
}

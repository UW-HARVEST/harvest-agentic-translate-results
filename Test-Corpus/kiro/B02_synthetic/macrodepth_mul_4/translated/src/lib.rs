#![allow(dead_code)]
use std::ffi::c_int;
use std::io::Write;

// --- Operations ---
#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int { a + b }
#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int { a - b }
#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int { a * b }

// --- Feature-gated OP selection ---
#[cfg(feature = "op_add")]
pub fn op_fn(a: c_int, b: c_int) -> c_int { op_add(a, b) }
#[cfg(feature = "op_sub")]
pub fn op_fn(a: c_int, b: c_int) -> c_int { op_sub(a, b) }
#[cfg(feature = "op_mul")]
pub fn op_fn(a: c_int, b: c_int) -> c_int { op_mul(a, b) }

#[cfg(feature = "op_add")]
pub fn g_op_name() -> &'static str { "add" }
#[cfg(feature = "op_sub")]
pub fn g_op_name() -> &'static str { "sub" }
#[cfg(feature = "op_mul")]
pub fn g_op_name() -> &'static str { "mul" }

pub fn g_op(a: c_int, b: c_int) -> c_int { op_fn(a, b) }

// --- Exported globals matching C's G_OP and G_OP_NAME ---
#[cfg(feature = "op_add")]
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = op_add;
#[cfg(feature = "op_sub")]
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = op_sub;
#[cfg(feature = "op_mul")]
#[unsafe(no_mangle)]
pub static G_OP: extern "C" fn(c_int, c_int) -> c_int = op_mul;

#[cfg(feature = "op_add")]
#[unsafe(no_mangle)]
pub static G_OP_NAME: &[u8; 4] = b"add\0";
#[cfg(feature = "op_sub")]
#[unsafe(no_mangle)]
pub static G_OP_NAME: &[u8; 4] = b"sub\0";
#[cfg(feature = "op_mul")]
#[unsafe(no_mangle)]
pub static G_OP_NAME: &[u8; 4] = b"mul\0";

// --- INIT values ---
#[cfg(feature = "op_add")]
pub fn init_for() -> c_int { 0 }
#[cfg(feature = "op_sub")]
pub fn init_for() -> c_int { 0 }
#[cfg(feature = "op_mul")]
pub fn init_for() -> c_int { 1 }

// --- STEP logic ---
#[cfg(feature = "op_add")]
fn step(acc: &mut c_int, i: c_int) { *acc += i; }
#[cfg(feature = "op_sub")]
fn step(acc: &mut c_int, i: c_int) { *acc -= i; }
#[cfg(feature = "op_mul")]
fn step(acc: &mut c_int, i: c_int) { *acc *= i + 1; }

// --- REPn: unrolled steps ---
fn rep0(_acc: &mut c_int) {}
fn rep1(acc: &mut c_int) { step(acc, 0); }
fn rep2(acc: &mut c_int) { rep1(acc); step(acc, 1); }
fn rep3(acc: &mut c_int) { rep2(acc); step(acc, 2); }
fn rep4(acc: &mut c_int) { rep3(acc); step(acc, 3); }
fn rep5(acc: &mut c_int) { rep4(acc); step(acc, 4); }
fn rep6(acc: &mut c_int) { rep5(acc); step(acc, 5); }
fn rep7(acc: &mut c_int) { rep6(acc); step(acc, 6); }

// --- RUN_LOOP: compile-time REPEAT selection ---
pub fn run_loop() -> c_int {
    let mut acc = init_for();
    #[cfg(feature = "repeat_0")] rep0(&mut acc);
    #[cfg(feature = "repeat_1")] rep1(&mut acc);
    #[cfg(feature = "repeat_2")] rep2(&mut acc);
    #[cfg(feature = "repeat_3")] rep3(&mut acc);
    #[cfg(feature = "repeat_4")] rep4(&mut acc);
    #[cfg(feature = "repeat_5")] rep5(&mut acc);
    #[cfg(feature = "repeat_6")] rep6(&mut acc);
    #[cfg(feature = "repeat_7")] rep7(&mut acc);
    acc
}

// --- DISPATCH_REP: runtime switch (cases 0-6 only, 7+ returns init) ---
pub fn accum_fn(n: c_int) -> c_int {
    let mut acc = init_for();
    match n {
        0 => rep0(&mut acc),
        1 => rep1(&mut acc),
        2 => rep2(&mut acc),
        3 => rep3(&mut acc),
        4 => rep4(&mut acc),
        5 => rep5(&mut acc),
        6 => rep6(&mut acc),
        _ => {}
    }
    acc
}

// --- REPEAT constant for use_generated ---
#[cfg(feature = "repeat_0")] pub const REPEAT: c_int = 0;
#[cfg(feature = "repeat_1")] pub const REPEAT: c_int = 1;
#[cfg(feature = "repeat_2")] pub const REPEAT: c_int = 2;
#[cfg(feature = "repeat_3")] pub const REPEAT: c_int = 3;
#[cfg(feature = "repeat_4")] pub const REPEAT: c_int = 4;
#[cfg(feature = "repeat_5")] pub const REPEAT: c_int = 5;
#[cfg(feature = "repeat_6")] pub const REPEAT: c_int = 6;
#[cfg(feature = "repeat_7")] pub const REPEAT: c_int = 7;

// --- Exported helpers ---
#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = op_fn(a, b);
    let acc = run_loop();
    let _ = write!(std::io::stdout(), "helper.call={} helper.acc={}\n", r, acc);
    r + acc
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let r = op_fn(a, b);
    let _ = write!(std::io::stdout(), "helper.ptr={}\n", r);
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum_fn(n);
    let _ = write!(std::io::stdout(), "gen.acc={}\n", r);
    r
}

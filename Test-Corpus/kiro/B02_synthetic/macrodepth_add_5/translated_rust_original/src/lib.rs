use std::ffi::c_int;
use std::os::raw::c_char;

// ── Operation functions ──────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int { a + b }

#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int { a - b }

#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int { a * b }

// ── Feature-selected OP function, name, init, step ──────────────────

#[cfg(feature = "add")]
pub fn selected_op(a: c_int, b: c_int) -> c_int { op_add(a, b) }
#[cfg(feature = "sub")]
pub fn selected_op(a: c_int, b: c_int) -> c_int { op_sub(a, b) }
#[cfg(feature = "mul")]
pub fn selected_op(a: c_int, b: c_int) -> c_int { op_mul(a, b) }

#[cfg(feature = "add")]
pub const G_OP_NAME_STR: &[u8] = b"add\0";
#[cfg(feature = "sub")]
pub const G_OP_NAME_STR: &[u8] = b"sub\0";
#[cfg(feature = "mul")]
pub const G_OP_NAME_STR: &[u8] = b"mul\0";

#[cfg(feature = "add")]
const INIT: c_int = 0;
#[cfg(feature = "sub")]
const INIT: c_int = 0;
#[cfg(feature = "mul")]
const INIT: c_int = 1;

#[cfg(feature = "add")]
fn step(acc: &mut c_int, i: c_int) { *acc += i; }
#[cfg(feature = "sub")]
fn step(acc: &mut c_int, i: c_int) { *acc -= i; }
#[cfg(feature = "mul")]
fn step(acc: &mut c_int, i: c_int) { *acc *= i + 1; }

// ── REPEAT constant ─────────────────────────────────────────────────

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

// ── REPn: unrolled step sequences ───────────────────────────────────

fn rep0(_acc: &mut c_int) {}
fn rep1(acc: &mut c_int) { step(acc, 0); }
fn rep2(acc: &mut c_int) { rep1(acc); step(acc, 1); }
fn rep3(acc: &mut c_int) { rep2(acc); step(acc, 2); }
fn rep4(acc: &mut c_int) { rep3(acc); step(acc, 3); }
fn rep5(acc: &mut c_int) { rep4(acc); step(acc, 4); }
fn rep6(acc: &mut c_int) { rep5(acc); step(acc, 5); }
fn rep7(acc: &mut c_int) { rep6(acc); step(acc, 6); }

// ── RUN_LOOP: compile-time selected REPn ────────────────────────────

#[cfg(feature = "repeat_0")]
pub fn run_loop(acc: &mut c_int) { rep0(acc); }
#[cfg(feature = "repeat_1")]
pub fn run_loop(acc: &mut c_int) { rep1(acc); }
#[cfg(feature = "repeat_2")]
pub fn run_loop(acc: &mut c_int) { rep2(acc); }
#[cfg(feature = "repeat_3")]
pub fn run_loop(acc: &mut c_int) { rep3(acc); }
#[cfg(feature = "repeat_4")]
pub fn run_loop(acc: &mut c_int) { rep4(acc); }
#[cfg(feature = "repeat_5")]
pub fn run_loop(acc: &mut c_int) { rep5(acc); }
#[cfg(feature = "repeat_6")]
pub fn run_loop(acc: &mut c_int) { rep6(acc); }
#[cfg(feature = "repeat_7")]
pub fn run_loop(acc: &mut c_int) { rep7(acc); }

// ── DISPATCH_REP: runtime switch on n ───────────────────────────────

fn dispatch_rep(acc: &mut c_int, n: c_int) {
    match n {
        0 => rep0(acc),
        1 => rep1(acc),
        2 => rep2(acc),
        3 => rep3(acc),
        4 => rep4(acc),
        5 => rep5(acc),
        6 => rep6(acc),
        _ => {}
    }
}

// ── accum_OP(n): macro-generated accumulator ────────────────────────

fn accum(n: c_int) -> c_int {
    let mut acc = INIT;
    dispatch_rep(&mut acc, n);
    acc
}

// ── Globals ─────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub static mut G_OP: Option<extern "C" fn(c_int, c_int) -> c_int> = None;

#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = std::ptr::null();

pub fn init_globals() {
    unsafe {
        G_OP = Some(selected_op_extern);
        G_OP_NAME = G_OP_NAME_STR.as_ptr() as *const c_char;
    }
}

extern "C" fn selected_op_extern(a: c_int, b: c_int) -> c_int {
    selected_op(a, b)
}

// ── Helpers (from mdcore.c) ─────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = selected_op(a, b);
    let mut acc = INIT;
    run_loop(&mut acc);
    unsafe { libc::printf(b"helper.call=%d helper.acc=%d\n\0".as_ptr() as *const c_char, r, acc) };
    r + acc
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: fn(c_int, c_int) -> c_int = selected_op;
    let r = fp(a, b);
    unsafe { libc::printf(b"helper.ptr=%d\n\0".as_ptr() as *const c_char, r) };
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum(n);
    unsafe { libc::printf(b"gen.acc=%d\n\0".as_ptr() as *const c_char, r) };
    r
}

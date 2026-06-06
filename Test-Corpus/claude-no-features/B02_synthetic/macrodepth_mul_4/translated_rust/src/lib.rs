// Translation of mdcore.c to Rust.
// The C build uses CMake defaults: OP=add, REPEAT=5

use std::ffi::c_char;
use std::os::raw::c_int;

// Default OP is "add" and REPEAT is 5 per CMakeLists.txt defaults.
pub const OP_NAME_BYTES: &[u8] = b"add\0";
pub const REPEAT: c_int = 5;

// ---------- Operation family ----------
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

// ---------- STEP / INIT operations for OP=add ----------
#[inline]
fn step_add(acc: &mut c_int, i: c_int) {
    *acc = acc.wrapping_add(i);
}

#[inline]
pub fn init_for_add() -> c_int {
    0
}

// Manual unrolling for OP=add (REPEAT=5) — equivalent to REP5(add, acc)
#[inline]
pub fn run_loop_add_5(acc: &mut c_int) {
    step_add(acc, 0);
    step_add(acc, 1);
    step_add(acc, 2);
    step_add(acc, 3);
    step_add(acc, 4);
}

// Macro-generated accumulator function: accum_add(n)
// In C this is `static`, so it is not exported with extern linkage.
fn accum_add(n: c_int) -> c_int {
    let mut acc = init_for_add();
    match n {
        0 => {}
        1 => {
            step_add(&mut acc, 0);
        }
        2 => {
            step_add(&mut acc, 0);
            step_add(&mut acc, 1);
        }
        3 => {
            step_add(&mut acc, 0);
            step_add(&mut acc, 1);
            step_add(&mut acc, 2);
        }
        4 => {
            step_add(&mut acc, 0);
            step_add(&mut acc, 1);
            step_add(&mut acc, 2);
            step_add(&mut acc, 3);
        }
        5 => {
            step_add(&mut acc, 0);
            step_add(&mut acc, 1);
            step_add(&mut acc, 2);
            step_add(&mut acc, 3);
            step_add(&mut acc, 4);
        }
        6 => {
            step_add(&mut acc, 0);
            step_add(&mut acc, 1);
            step_add(&mut acc, 2);
            step_add(&mut acc, 3);
            step_add(&mut acc, 4);
            step_add(&mut acc, 5);
        }
        _ => {}
    }
    acc
}

// ---------- Globals ----------
// G_OP is a function pointer initialized at file scope to op_add
pub type OpFn = extern "C" fn(c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub static G_OP: OpFn = op_add;

// G_OP_NAME is `const char *` in C — a raw pointer. Wrap it for Sync.
#[repr(transparent)]
pub struct CharPtr(pub *const c_char);
// SAFETY: Underlying pointer targets static read-only memory; sharing it
// across threads via &'static is safe because nothing mutates it.
unsafe impl Sync for CharPtr {}

#[unsafe(no_mangle)]
pub static G_OP_NAME: CharPtr = CharPtr(OP_NAME_BYTES.as_ptr() as *const c_char);

// ---------- Helpers ----------
#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = op_add(a, b);
    let mut acc = init_for_add();
    run_loop_add_5(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp: OpFn = op_add;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = accum_add(n);
    println!("gen.acc={}", r);
    r
}

// Public name accessor used by the binary driver.
pub fn op_name_str() -> &'static str {
    "add"
}

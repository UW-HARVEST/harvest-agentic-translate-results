// Translation of the `static int counter` state and the four operations that
// mutate it.
//
// The C original is a plain `static int`, mutated without synchronisation. An
// `AtomicI32` with relaxed ordering is used here so the translation needs no
// `static mut`; for the single-threaded use the C code assumed, the observable
// sequence of values is identical.
//
// All arithmetic is `wrapping_*`: signed overflow is undefined behaviour in C,
// but the compiled C library wraps two's-complement, and `multiply_counter` in
// particular is reachable with values that overflow. Reproducing the wrap keeps
// us bug-for-bug compatible instead of panicking in a debug build.

use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, Ordering};

static COUNTER: AtomicI32 = AtomicI32::new(0);

/// Reads the shared counter (`counter` in the C source).
pub fn get() -> c_int {
    COUNTER.load(Ordering::Relaxed)
}

/// Writes the shared counter.
pub fn set(value: c_int) {
    COUNTER.store(value, Ordering::Relaxed);
}

/// `int increment_counter(int value)` -- `counter += value; return counter;`
#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int) -> c_int {
    let previous = COUNTER.fetch_add(value, Ordering::Relaxed);
    previous.wrapping_add(value)
}

/// `int decrement_counter(int value)` -- `counter -= value; return counter;`
#[unsafe(no_mangle)]
pub extern "C" fn decrement_counter(value: c_int) -> c_int {
    let previous = COUNTER.fetch_sub(value, Ordering::Relaxed);
    previous.wrapping_sub(value)
}

/// `int multiply_counter(int value)` -- `counter *= value; return counter;`
#[unsafe(no_mangle)]
pub extern "C" fn multiply_counter(value: c_int) -> c_int {
    let updated = COUNTER.load(Ordering::Relaxed).wrapping_mul(value);
    COUNTER.store(updated, Ordering::Relaxed);
    updated
}

/// `int reset_counter(int value)` -- `counter = value; return counter;`
#[unsafe(no_mangle)]
pub extern "C" fn reset_counter(value: c_int) -> c_int {
    COUNTER.store(value, Ordering::Relaxed);
    value
}

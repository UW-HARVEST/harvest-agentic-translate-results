//! The file-scope `static int counter;` from `src/lib.c` and the four exported
//! functions that mutate it.
//!
//! The C original is not thread safe (a plain non-atomic `static int` touched
//! from exported entry points), and signed overflow in the arithmetic is
//! technically undefined behaviour that GCC/Clang compile into plain wrapping
//! `add`/`sub`/`imul`. Both traits are preserved here: the state lives in an
//! `UnsafeCell` with no synchronisation, and the arithmetic uses the explicit
//! `wrapping_*` operators so the release build cannot diverge and the debug
//! build cannot panic.

use core::cell::UnsafeCell;
use core::ffi::c_int;

/// `static int counter = 0;`
struct Counter(UnsafeCell<c_int>);

// SAFETY: This is a faithful model of a C file-scope `static int`, which comes
// with no synchronisation of its own. Every access happens through the exported
// entry points below, exactly as in the C original.
unsafe impl Sync for Counter {}

static COUNTER: Counter = Counter(UnsafeCell::new(0));

/// Read the file-scope counter.
#[inline]
pub(crate) fn get() -> c_int {
    // SAFETY: no concurrent access is guarded against, matching the C original.
    unsafe { *COUNTER.0.get() }
}

/// Overwrite the file-scope counter.
#[inline]
pub(crate) fn set(value: c_int) {
    // SAFETY: see `get`.
    unsafe { *COUNTER.0.get() = value }
}

/// ```c
/// int increment_counter(int value) {
///     counter += value;
///     return counter;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int) -> c_int {
    let updated = get().wrapping_add(value);
    set(updated);
    updated
}

/// ```c
/// int decrement_counter(int value) {
///     counter -= value;
///     return counter;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn decrement_counter(value: c_int) -> c_int {
    let updated = get().wrapping_sub(value);
    set(updated);
    updated
}

/// ```c
/// int multiply_counter(int value) {
///     counter *= value;
///     return counter;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn multiply_counter(value: c_int) -> c_int {
    let updated = get().wrapping_mul(value);
    set(updated);
    updated
}

/// ```c
/// int reset_counter(int value) {
///     counter = value;
///     return counter;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn reset_counter(value: c_int) -> c_int {
    set(value);
    value
}

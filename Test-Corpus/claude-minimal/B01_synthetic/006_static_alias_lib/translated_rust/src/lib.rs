// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from the original C source.
//
// The translation preserves the semantics of the original C code, including
// the use of a function-local `static` variable in `static_alias` and the
// pointer-aliasing pattern used by `driver`.

use std::os::raw::c_int;
use std::sync::Mutex;

/// Storage for the function-local `static int inner = 1;` from the C code.
///
/// In C, `static_alias` returns a pointer to this storage, so callers can
/// observe and (through the returned pointer) implicitly read the value.
/// To preserve that behavior we expose the value through a raw pointer that
/// points at a process-wide static cell.
static INNER: Mutex<c_int> = Mutex::new(1);

// Backing storage that the returned raw pointer refers to. We synchronize
// updates to it via `INNER` (the Mutex) so that, on the single-threaded
// path the C code assumes, the contents of `INNER_CELL` and the value
// stored in `INNER` are kept in lockstep.
static mut INNER_CELL: c_int = 1;

/// Rust translation of:
///
/// ```c
/// int *static_alias(int *outer) {
///   static int inner = 1;
///   if (*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
///
/// # Safety
///
/// `outer` must be a valid, properly aligned pointer to a `c_int` that is
/// not aliased by another mutable reference for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let mut guard = INNER.lock().unwrap();
    let inner_val = *guard;
    let outer_val = *outer;

    if outer_val >= inner_val {
        let new_inner = inner_val + outer_val;
        *guard = new_inner;
        // Keep the backing cell in sync with the mutex-protected value so
        // the returned raw pointer observes the new value.
        INNER_CELL = new_inner;
        // Release the lock before returning the raw pointer to avoid
        // holding it across the caller's use of the pointer.
        drop(guard);
        let p: *mut c_int = &raw mut INNER_CELL;
        p
    } else {
        *outer = outer_val + inner_val;
        drop(guard);
        outer
    }
}

/// Rust translation of:
///
/// ```c
/// void driver(int initial_value, int iterations) {
///   int *running_sum = &initial_value;
///   for (int i = 0; i < iterations; i++) {
///     running_sum = static_alias(running_sum);
///     printf("%d\n", *running_sum);
///   }
/// }
/// ```
#[no_mangle]
pub extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value as *mut c_int;
    let mut i: c_int = 0;
    while i < iterations {
        unsafe {
            running_sum = static_alias(running_sum);
            println!("{}", *running_sum);
        }
        i += 1;
    }
}

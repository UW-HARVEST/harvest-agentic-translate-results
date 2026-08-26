#![allow(non_snake_case)]

use std::ffi::c_int;
use std::io::Write;
use std::sync::atomic::{AtomicI32, Ordering};

// Static "inner" variable equivalent to C's `static int inner = 1;`
// Using AtomicI32 to allow safe mutation; the C code is single-threaded
// in spirit but this preserves the equivalent behavior.
static INNER: AtomicI32 = AtomicI32::new(1);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    // Reproduce exact behavior of the C code, including aliasing semantics.
    let outer_val = unsafe { *outer };
    let inner_val = INNER.load(Ordering::SeqCst);
    if outer_val >= inner_val {
        let new_inner = inner_val.wrapping_add(outer_val);
        INNER.store(new_inner, Ordering::SeqCst);
        // Return a pointer to the static INNER. Since AtomicI32 has the same
        // memory layout as i32, this pointer is valid for reading the value.
        INNER.as_ptr()
    } else {
        unsafe {
            *outer = outer_val.wrapping_add(inner_val);
        }
        outer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // Mirror the C code: `int *running_sum = &initial_value;`
    // initial_value is a stack-local (parameter) in C, and the pointer
    // is taken to it. We replicate that here.
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value as *mut c_int;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let mut i: c_int = 0;
    while i < iterations {
        running_sum = unsafe { static_alias(running_sum) };
        let value = unsafe { *running_sum };
        // printf("%d\n", value) - byte-identical output
        let _ = writeln!(handle, "{}", value);
        i += 1;
    }
    let _ = handle.flush();
}

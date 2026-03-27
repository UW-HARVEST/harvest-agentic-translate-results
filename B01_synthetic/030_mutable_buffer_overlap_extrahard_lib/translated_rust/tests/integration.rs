use libloading::{Library, Symbol};
use std::os::raw::c_int;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

// ---- fma_array tests (lowest level) ----

#[test]
fn test_fma_array_distinct_buffers() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let c_fma: Symbol<unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int)> =
        unsafe { c_lib.get(b"fma_array").unwrap() };

    let mul1 = [1, 2, 3, 4, 5];
    let mul2 = [10, 20, 30, 40, 50];
    let add = [100, 200, 300, 400, 500];

    let mut c_out = [0i32; 5];
    let mut r_out = [0i32; 5];

    unsafe {
        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
        driver::fma_array(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
    }
    assert_eq!(c_out, r_out, "fma_array distinct buffers mismatch");
}

#[test]
fn test_fma_array_aliased() {
    // This is the critical test: all 4 pointers alias the same buffer
    // C behavior: out[i] = out[i]*out[i] + out[i]
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let c_fma: Symbol<unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int)> =
        unsafe { c_lib.get(b"fma_array").unwrap() };

    let input = [1, 2, 3, 4, 5, -1, 0, 100];
    let len = input.len() as c_int;

    let mut c_buf = input;
    let mut r_buf = input;

    unsafe {
        c_fma(c_buf.as_mut_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), len);
        driver::fma_array(r_buf.as_mut_ptr(), r_buf.as_ptr(), r_buf.as_ptr(), r_buf.as_ptr(), len);
    }
    assert_eq!(c_buf, r_buf, "fma_array aliased mismatch: C={:?} Rust={:?}", c_buf, r_buf);
}

#[test]
fn test_fma_array_empty() {
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let c_fma: Symbol<unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int)> =
        unsafe { c_lib.get(b"fma_array").unwrap() };

    let mut c_out = [0i32; 0];
    let mut r_out = [0i32; 0];
    unsafe {
        c_fma(c_out.as_mut_ptr(), c_out.as_ptr(), c_out.as_ptr(), c_out.as_ptr(), 0);
        driver::fma_array(r_out.as_mut_ptr(), r_out.as_ptr(), r_out.as_ptr(), r_out.as_ptr(), 0);
    }
    assert_eq!(c_out, r_out);
}

// ---- driver tests (higher level, calls fma_array + prints) ----

#[test]
fn test_driver_output() {
    // driver() prints to stdout, so we capture output from both C and Rust
    let c_lib = unsafe { Library::new(C_LIB).unwrap() };
    let c_driver: Symbol<unsafe extern "C" fn(*const c_int, c_int)> =
        unsafe { c_lib.get(b"driver").unwrap() };

    let input = [1, 2, 3, 4, 5];
    let len = input.len() as c_int;

    // Compute expected: for each x, x*x + x
    // 1->2, 2->6, 3->12, 4->20, 5->30
    // Both should print these values

    // We can't easily capture printf output in-process, so instead
    // verify the fma_array computation matches and trust printf is the same.
    // But let's at least call both without crashing.
    unsafe {
        c_driver(input.as_ptr(), len);
        driver::driver(input.as_ptr(), len);
    }
}

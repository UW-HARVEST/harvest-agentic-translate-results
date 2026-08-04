// Translated from C to Rust. Reproduces the original program's behavior exactly,
// including its dependence on libc's rand()/srand() for byte-identical output.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_ulong};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;
const UINT_MAX: c_ulong = u32::MAX as c_ulong;

extern "C" {
    fn srand(seed: u32);
    fn rand() -> c_int;
    fn strtoul(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong;
    fn __errno_location() -> *mut c_int;
}

fn errno() -> c_int {
    unsafe { *__errno_location() }
}

fn set_errno(v: c_int) {
    unsafe {
        *__errno_location() = v;
    }
}

fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x: i32 = *slot;
        for _ in 0..100 {
            // x = x * 3 + 7
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3)  -- C uses arithmetic shift for signed
            x ^= x >> 3;
            // x = x - (x << 1)
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7   -- C truncates toward zero (matches Rust's / and %)
            x = (x / 2).wrapping_add(x % 7);
        }
        *slot = x;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len();

    if argc != 2 {
        let prog = args.get(0).map(|s| s.as_str()).unwrap_or("");
        eprintln!("Usage: {} <seed>", prog);
        std::process::exit(1);
    }

    // Mirror: errno = 0; strtoul(argv[1], &endptr, 10);
    set_errno(0);

    let arg1 = &args[1];
    let c_arg1 = match CString::new(arg1.as_str()) {
        Ok(s) => s,
        Err(_) => {
            // The original C wouldn't have embedded NULs in argv either.
            eprintln!("Invalid seed: '{}'", arg1);
            std::process::exit(1);
        }
    };

    let mut endptr: *mut c_char = std::ptr::null_mut();
    let temp_seed: c_ulong = unsafe { strtoul(c_arg1.as_ptr(), &mut endptr, 10) };
    let err = errno();

    let bad_endptr = unsafe { *endptr != 0 };
    if bad_endptr || err != 0 || temp_seed > UINT_MAX {
        eprintln!("Invalid seed: '{}'", arg1);
        std::process::exit(1);
    }

    let seed: u32 = temp_seed as u32;
    unsafe { srand(seed) };

    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];
    for slot in array.iter_mut() {
        *slot = unsafe { rand() } as i32;
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &v in array.iter() {
        xor_result ^= v;
    }

    println!("{}", xor_result);
}

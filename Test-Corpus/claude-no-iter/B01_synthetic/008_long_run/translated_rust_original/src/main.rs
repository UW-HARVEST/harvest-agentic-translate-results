// Translation of c_src/src/main.c to Rust.
// Calls libc's srand/rand/strtoul via FFI to preserve byte-identical output.

use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint, c_ulong};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::ptr;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn strtoul(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong;
    fn __errno_location() -> *mut c_int;
}

// Perform expensive arithmetic on each element. Mirrors the C logic exactly,
// using wrapping arithmetic to match C's int (i32) overflow semantics.
fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x: i32 = *slot;
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);
            // C arithmetic shift on signed int >> 3 -- Rust i32 >> is arithmetic.
            x ^= x >> 3;
            // x = x - (x << 1);
            // Use wrapping_shl(1) to avoid UB for any input; matches typical C semantics.
            let shifted = (x as u32).wrapping_shl(1) as i32;
            x = x.wrapping_sub(shifted);
            // x = x / 2 + x % 7;
            // C / and % truncate toward zero; Rust i32 / and % do the same.
            x = (x / 2).wrapping_add(x % 7);
        }
        *slot = x;
    }
}

fn run() -> ExitCode {
    let args_os: Vec<std::ffi::OsString> = env::args_os().collect();
    let argc = args_os.len();

    // argv[0] string (used in usage message)
    let prog_display: String = args_os
        .get(0)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    if argc != 2 {
        eprintln!("Usage: {} <seed>", prog_display);
        return ExitCode::from(1);
    }

    let arg_os = &args_os[1];
    let arg_bytes = arg_os.as_bytes();
    let arg_display = arg_os.to_string_lossy();

    // Convert to C string. argv strings are guaranteed null-terminated by the OS,
    // so embedded nulls are impossible in practice.
    let cstr = match CString::new(arg_bytes) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Invalid seed: '{}'", arg_display);
            return ExitCode::from(1);
        }
    };

    let mut endptr: *mut c_char = ptr::null_mut();
    let (temp_seed, errno_val, end_char): (c_ulong, c_int, c_char) = unsafe {
        *__errno_location() = 0;
        let v = strtoul(cstr.as_ptr(), &mut endptr as *mut *mut c_char, 10);
        let e = *__errno_location();
        let c = *endptr;
        (v, e, c)
    };

    if end_char != 0 || errno_val != 0 || temp_seed > c_uint::MAX as c_ulong {
        eprintln!("Invalid seed: '{}'", arg_display);
        return ExitCode::from(1);
    }

    let seed = temp_seed as c_uint;
    unsafe {
        srand(seed);
    }

    // Global array in C is zero-initialized; we overwrite every entry with rand()
    // before reading, so initial value doesn't matter.
    let mut array: Vec<i32> = vec![0i32; ARRAY_SIZE];

    for slot in array.iter_mut() {
        let r = unsafe { rand() };
        *slot = r as i32;
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &v in array.iter() {
        xor_result ^= v;
    }

    println!("{}", xor_result);
    ExitCode::from(0)
}

fn main() -> ExitCode {
    run()
}

use std::env;
use std::ffi::{c_char, c_int, c_uint, c_ulong, CString, OsString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn rand() -> c_int;
    fn srand(seed: c_uint);
    fn strtoul(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong;
}

fn write_stderr(parts: &[&[u8]]) {
    let mut stderr = io::stderr().lock();
    for part in parts {
        let _ = stderr.write_all(part);
    }
}

fn parse_seed(value: &OsString) -> Option<c_uint> {
    let value = CString::new(value.as_os_str().as_bytes()).expect("argv contains a NUL byte");
    let mut endptr = std::ptr::null_mut();

    // SAFETY: value is NUL-terminated, endptr is valid, and errno is thread-local.
    let seed = unsafe {
        *__errno_location() = 0;
        strtoul(value.as_ptr(), &mut endptr, 10)
    };

    // SAFETY: strtoul sets endptr to an address within the live value allocation.
    let has_trailing_input = unsafe { *endptr != 0 };
    if has_trailing_input {
        return None;
    }
    // SAFETY: errno is thread-local and no intervening operation changes it.
    if unsafe { *__errno_location() } != 0 {
        return None;
    }
    if seed > c_uint::MAX as c_ulong {
        return None;
    }

    Some(seed as c_uint)
}

#[inline]
fn perform_expensive_operations(array: &mut [i32]) {
    for value in array {
        let mut x = *value;
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        *value = x;
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() != 2 {
        write_stderr(&[b"Usage: ", args[0].as_os_str().as_bytes(), b" <seed>\n"]);
        return ExitCode::from(1);
    }

    let Some(seed) = parse_seed(&args[1]) else {
        write_stderr(&[
            b"Invalid seed: '",
            args[1].as_os_str().as_bytes(),
            b"'\n",
        ]);
        return ExitCode::from(1);
    };

    // SAFETY: srand and rand have no pointer preconditions and are used single-threaded.
    unsafe { srand(seed) };
    let mut array = vec![0_i32; ARRAY_SIZE];
    for value in &mut array {
        *value = unsafe { rand() };
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let xor_result = array.into_iter().fold(0, |result, value| result ^ value);
    println!("{xor_result}");
    ExitCode::SUCCESS
}

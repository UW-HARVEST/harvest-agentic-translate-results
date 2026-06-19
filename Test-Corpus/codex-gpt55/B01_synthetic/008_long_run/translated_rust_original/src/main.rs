use std::ffi::CString;
use std::io::{self, Write};
use std::os::raw::{c_char, c_int, c_ulong};
use std::os::unix::ffi::OsStrExt;
use std::ffi::OsStr;
use std::ptr;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn strtoul(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong;
    fn __errno_location() -> *mut c_int;
}

#[allow(non_camel_case_types)]
type c_uint = u32;

fn perform_expensive_operations(array: &mut [i32]) {
    for value in array.iter_mut() {
        let mut x = *value;
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = (x / 2).wrapping_add(x % 7);
        }
        *value = x;
    }
}

fn write_usage(program_name: &OsStr) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Usage: ");
    let _ = stderr.write_all(program_name.as_bytes());
    let _ = stderr.write_all(b" <seed>\n");
}

fn write_invalid_seed(seed: &OsStr) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Invalid seed: '");
    let _ = stderr.write_all(seed.as_bytes());
    let _ = stderr.write_all(b"'\n");
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();

    if args.len() != 2 {
        write_usage(args.get(0).map(|arg| arg.as_os_str()).unwrap_or_else(|| OsStr::new("")));
        std::process::exit(1);
    }

    let seed_arg = CString::new(args[1].as_bytes()).expect("argv cannot contain NUL bytes");

    let mut endptr: *mut c_char = ptr::null_mut();
    let temp_seed = unsafe {
        *__errno_location() = 0;
        strtoul(seed_arg.as_ptr(), &mut endptr, 10)
    };

    let invalid = unsafe {
        *endptr != 0 || *__errno_location() != 0 || temp_seed > u32::MAX as c_ulong
    };

    if invalid {
        write_invalid_seed(args[1].as_os_str());
        std::process::exit(1);
    }

    unsafe {
        srand(temp_seed as c_uint);
    }

    let mut array = vec![0i32; ARRAY_SIZE];
    for value in array.iter_mut() {
        *value = unsafe { rand() as i32 };
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result = 0i32;
    for value in array {
        xor_result ^= value;
    }

    println!("{}", xor_result);
}

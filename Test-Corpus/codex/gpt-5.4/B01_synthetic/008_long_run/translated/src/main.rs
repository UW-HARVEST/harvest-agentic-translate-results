use std::env;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::io::{self, Write};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

fn perform_expensive_operations(array: &mut [i32; ARRAY_SIZE]) {
    for value in array.iter_mut() {
        let mut x = *value;
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x.wrapping_div(2).wrapping_add(x.wrapping_rem(7));
        }
        *value = x;
    }
}

fn write_usage_and_exit(program_name: &[u8]) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Usage: ");
    let _ = stderr.write_all(program_name);
    let _ = stderr.write_all(b" <seed>\n");
    std::process::exit(1);
}

fn write_invalid_seed_and_exit(seed: &[u8]) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Invalid seed: '");
    let _ = stderr.write_all(seed);
    let _ = stderr.write_all(b"'\n");
    std::process::exit(1);
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 2 {
        write_usage_and_exit(args[0].as_os_str().as_bytes());
    }

    let arg1_bytes = args[1].as_os_str().as_bytes();
    let arg1 = match CString::new(arg1_bytes) {
        Ok(value) => value,
        Err(_) => write_invalid_seed_and_exit(arg1_bytes),
    };

    let mut endptr = std::ptr::null_mut();
    // SAFETY: `arg1` is a valid NUL-terminated C string and `endptr` points to writable storage.
    let temp_seed = unsafe {
        *libc::__errno_location() = 0;
        libc::strtoul(arg1.as_ptr(), &mut endptr, 10)
    };

    // SAFETY: `strtoul` sets `endptr` to point into `arg1`, which remains alive here.
    let invalid = unsafe {
        *endptr != 0 || *libc::__errno_location() != 0 || temp_seed > u32::MAX as libc::c_ulong
    };
    if invalid {
        write_invalid_seed_and_exit(arg1_bytes);
    }

    let seed = temp_seed as u32;
    let mut array = Box::new([0_i32; ARRAY_SIZE]);

    // SAFETY: `srand` and `rand` are called with valid arguments and use libc global PRNG state.
    unsafe {
        libc::srand(seed);
        for value in array.iter_mut() {
            *value = libc::rand();
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result = 0_i32;
    for value in array.iter() {
        xor_result ^= *value;
    }

    println!("{xor_result}");
}

use std::env;
use std::ffi::{c_char, c_int, c_ulong, CString, OsString};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::ptr;
use std::thread;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn strtoul(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong;
    fn srand(seed: u32);
    fn rand() -> c_int;
}

fn write_usage(program: &OsString) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Usage: ");
    let _ = stderr.write_all(program.as_os_str().as_bytes());
    let _ = stderr.write_all(b" <seed>\n");
}

fn write_invalid_seed(argument: &OsString) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Invalid seed: '");
    let _ = stderr.write_all(argument.as_os_str().as_bytes());
    let _ = stderr.write_all(b"'\n");
}

fn parse_seed(argument: &OsString) -> Option<u32> {
    let argument = CString::new(argument.as_os_str().as_bytes())
        .expect("process arguments cannot contain NUL bytes");
    let mut endptr = ptr::null_mut();

    let temp_seed = unsafe {
        *__errno_location() = 0;
        strtoul(argument.as_ptr(), &mut endptr, 10)
    };

    let has_trailing_characters = unsafe { *endptr != 0 };
    if has_trailing_characters {
        return None;
    }

    let errno_is_set = unsafe { *__errno_location() != 0 };
    if errno_is_set {
        return None;
    }

    if temp_seed > u32::MAX as c_ulong {
        return None;
    }

    Some(temp_seed as u32)
}

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

fn perform_all_iterations(array: &mut [i32]) {
    let thread_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(array.len());
    let chunk_size = array.len().div_ceil(thread_count);

    thread::scope(|scope| {
        for chunk in array.chunks_mut(chunk_size) {
            scope.spawn(move || {
                for _ in 0..ITERATIONS {
                    perform_expensive_operations(chunk);
                }
            });
        }
    });
}

fn main() {
    let arguments: Vec<OsString> = env::args_os().collect();
    if arguments.len() != 2 {
        write_usage(&arguments[0]);
        std::process::exit(1);
    }

    let Some(seed) = parse_seed(&arguments[1]) else {
        write_invalid_seed(&arguments[1]);
        std::process::exit(1);
    };

    unsafe {
        srand(seed);
    }

    let mut array = vec![0_i32; ARRAY_SIZE];
    for value in &mut array {
        *value = unsafe { rand() };
    }

    perform_all_iterations(&mut array);

    let mut xor_result = 0_i32;
    for value in array {
        xor_result ^= value;
    }

    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{xor_result}");
}

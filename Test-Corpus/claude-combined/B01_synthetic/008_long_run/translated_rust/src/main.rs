use std::env;
use std::ffi::CString;
use std::io::Write;
use std::process::ExitCode;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;
const UINT_MAX: u64 = u32::MAX as u64;

fn perform_expensive_operations(array: &mut [i32]) {
    for i in 0..ARRAY_SIZE {
        let mut x: i32 = array[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);
            x ^= x >> 3;
            // x = x - (x << 1);
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;
            x = (x / 2).wrapping_add(x % 7);
        }
        array[i] = x;
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 2 {
        let prog = args.get(0).map(|s| s.as_str()).unwrap_or("");
        let stderr = std::io::stderr();
        let mut h = stderr.lock();
        let _ = writeln!(h, "Usage: {} <seed>", prog);
        return ExitCode::from(1);
    }

    // Use libc's strtoul to match exact behavior
    let arg1 = &args[1];
    let c_arg = match CString::new(arg1.as_bytes()) {
        Ok(c) => c,
        Err(_) => {
            // contains a null byte; treat as invalid seed
            eprintln!("Invalid seed: '{}'", arg1);
            return ExitCode::from(1);
        }
    };

    let temp_seed: u64;
    let mut endptr: *mut libc::c_char = std::ptr::null_mut();
    unsafe {
        // Reset errno
        *libc::__errno_location() = 0;
        temp_seed = libc::strtoul(c_arg.as_ptr(), &mut endptr as *mut *mut libc::c_char, 10) as u64;
        let errno_val = *libc::__errno_location();
        let endbyte = *endptr;
        if endbyte != 0 || errno_val != 0 || temp_seed > UINT_MAX {
            eprintln!("Invalid seed: '{}'", arg1);
            return ExitCode::from(1);
        }
    }

    let seed: u32 = temp_seed as u32;
    unsafe {
        libc::srand(seed);
    }

    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];

    for i in 0..ARRAY_SIZE {
        array[i] = unsafe { libc::rand() };
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    println!("{}", xor_result);
    ExitCode::from(0)
}

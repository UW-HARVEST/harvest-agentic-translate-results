// Rust translation of c_src/src/main.c
// Reproduces C's behavior using libc's srand/rand and strtoul for byte-identical output.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: i32 = 2000;

// Global array (matches C's `int array[ARRAY_SIZE];` zero-initialized BSS).
static mut ARRAY: [i32; ARRAY_SIZE] = [0i32; ARRAY_SIZE];

// Perform expensive arithmetic on each element
fn perform_expensive_operations() {
    // SAFETY: single-threaded access to the global array, mirroring the C code.
    let arr: &mut [i32; ARRAY_SIZE] = unsafe {
        // Reborrow the static mut as a &mut reference.
        &mut *core::ptr::addr_of_mut!(ARRAY)
    };
    for i in 0..ARRAY_SIZE {
        let mut x: i32 = arr[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);  -- arithmetic right shift on signed int (matches C on all common targets)
            x ^= x >> 3;
            // x = x - (x << 1);
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;  -- C truncated division; same in Rust for i32.
            x = (x / 2).wrapping_add(x % 7);
        }
        arr[i] = x;
    }
}

fn main() -> ExitCode {
    // Collect args as raw bytes (preserve any non-UTF-8 inputs the way C would).
    let args_os: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args_os.len();

    // argv[0] for the usage message (lossy is acceptable; C would print raw bytes).
    let prog_name = args_os
        .get(0)
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    if argc != 2 {
        eprintln!("Usage: {} <seed>", prog_name);
        return ExitCode::from(1);
    }

    // Build a C string from argv[1] bytes for strtoul. NUL byte inside would be an
    // invalid C argument anyway; in that case we mimic an "Invalid seed" failure.
    let arg1_os = &args_os[1];
    let arg1_bytes = arg1_os.as_bytes();

    let arg1_cstr = match CString::new(arg1_bytes) {
        Ok(c) => c,
        Err(_) => {
            // Contains a NUL — couldn't be a real C argv element anyway.
            eprintln!("Invalid seed: '{}'", arg1_os.to_string_lossy());
            return ExitCode::from(1);
        }
    };

    // Replicate: errno = 0; strtoul(argv[1], &endptr, 10);
    // Then check `*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX`.
    let temp_seed: libc::c_ulong;
    let endptr_offset: usize;
    let errno_after: i32;
    unsafe {
        // Reset errno before the call.
        *libc::__errno_location() = 0;
        let mut endptr: *mut libc::c_char = core::ptr::null_mut();
        let start_ptr = arg1_cstr.as_ptr();
        temp_seed = libc::strtoul(start_ptr, &mut endptr, 10);
        errno_after = *libc::__errno_location();
        endptr_offset = (endptr as usize).wrapping_sub(start_ptr as usize);
    }

    // *endptr != '\0' means the consumed length didn't reach the end of the C string.
    let consumed_to_end = endptr_offset == arg1_bytes.len();

    // UINT_MAX in C is u32::MAX (unsigned int is 32 bits on all relevant platforms).
    let temp_seed_u64 = temp_seed as u64;
    let uint_max: u64 = u32::MAX as u64;

    if !consumed_to_end || errno_after != 0 || temp_seed_u64 > uint_max {
        eprintln!("Invalid seed: '{}'", arg1_os.to_string_lossy());
        return ExitCode::from(1);
    }

    let seed: libc::c_uint = temp_seed as libc::c_uint;
    unsafe {
        libc::srand(seed);
    }

    // Fill the array with rand() values.
    {
        let arr: &mut [i32; ARRAY_SIZE] =
            unsafe { &mut *core::ptr::addr_of_mut!(ARRAY) };
        for i in 0..ARRAY_SIZE {
            let r = unsafe { libc::rand() };
            arr[i] = r as i32;
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: i32 = 0;
    {
        let arr: &[i32; ARRAY_SIZE] =
            unsafe { &*core::ptr::addr_of!(ARRAY) };
        for i in 0..ARRAY_SIZE {
            xor_result ^= arr[i];
        }
    }

    println!("{}", xor_result);
    ExitCode::from(0)
}

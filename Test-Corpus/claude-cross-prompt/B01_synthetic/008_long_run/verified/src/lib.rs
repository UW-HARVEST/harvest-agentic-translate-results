use std::ffi::c_char;
use std::ffi::c_int;
use std::sync::Mutex;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: c_int = 2000;

// Global array — equivalent to C's `int array[ARRAY_SIZE]`.
// In C this lives in BSS (zero-initialized). We lazily allocate on first use.
static ARRAY: Mutex<Vec<i32>> = Mutex::new(Vec::new());

fn ensure_array_initialized(arr: &mut Vec<i32>) {
    if arr.is_empty() {
        arr.resize(ARRAY_SIZE, 0);
    }
}

fn perform_expensive_operations_inner(arr: &mut [i32]) {
    for i in 0..ARRAY_SIZE {
        let mut x: i32 = arr[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);  (arithmetic shift on signed int)
            x ^= x >> 3;
            // x = x - (x << 1);
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;  (C99+ truncates toward zero, matching Rust)
            x = (x / 2).wrapping_add(x % 7);
        }
        arr[i] = x;
    }
}

// Glibc exposes `stderr` as an extern symbol (FILE *).
extern "C" {
    static stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn perform_expensive_operations() {
    let mut arr = ARRAY.lock().unwrap();
    ensure_array_initialized(&mut arr);
    perform_expensive_operations_inner(&mut arr);
}

#[unsafe(no_mangle)]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    unsafe {
        if argc != 2 {
            let prog = if !argv.is_null() && !(*argv).is_null() {
                *argv
            } else {
                b"\0".as_ptr() as *const c_char
            };
            let fmt = b"Usage: %s <seed>\n\0".as_ptr() as *const c_char;
            libc::fprintf(stderr, fmt, prog);
            return 1;
        }

        // errno = 0;
        *libc::__errno_location() = 0;

        let arg1 = *argv.add(1);
        let mut endptr: *mut c_char = std::ptr::null_mut();
        let temp_seed: libc::c_ulong =
            libc::strtoul(arg1, &mut endptr as *mut *mut c_char, 10);

        let errno_val = *libc::__errno_location();
        if (!endptr.is_null() && *endptr != 0)
            || errno_val != 0
            || temp_seed > libc::c_uint::MAX as libc::c_ulong
        {
            let fmt = b"Invalid seed: '%s'\n\0".as_ptr() as *const c_char;
            libc::fprintf(stderr, fmt, arg1);
            return 1;
        }

        let seed: libc::c_uint = temp_seed as libc::c_uint;
        libc::srand(seed);

        {
            let mut arr = ARRAY.lock().unwrap();
            ensure_array_initialized(&mut arr);
            for i in 0..ARRAY_SIZE {
                arr[i] = libc::rand() as i32;
            }
        }

        for _ in 0..ITERATIONS {
            perform_expensive_operations();
        }

        let xor_result: i32 = {
            let arr = ARRAY.lock().unwrap();
            let mut acc: i32 = 0;
            for i in 0..ARRAY_SIZE {
                acc ^= arr[i];
            }
            acc
        };

        let fmt = b"%d\n\0".as_ptr() as *const c_char;
        libc::printf(fmt, xor_result);
        0
    }
}

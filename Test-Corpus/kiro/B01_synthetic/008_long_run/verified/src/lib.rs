use std::ffi::CString;
use std::ptr;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

#[no_mangle]
pub static mut array: [i32; ARRAY_SIZE] = [0i32; ARRAY_SIZE];

#[no_mangle]
pub extern "C" fn perform_expensive_operations() {
    unsafe {
        for i in 0..ARRAY_SIZE {
            let mut x: i32 = array[i];
            for _ in 0..100 {
                x = x.wrapping_mul(3).wrapping_add(7);
                x ^= x >> 3;
                x = x.wrapping_sub(x.wrapping_shl(1));
                x = x / 2 + x % 7;
            }
            array[i] = x;
        }
    }
}

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const libc::c_char) -> i32 {
    if argc != 2 {
        unsafe {
            let prog = if argv.is_null() || (*argv).is_null() {
                CString::new("driver").unwrap()
            } else {
                CString::new(std::ffi::CStr::from_ptr(*argv).to_bytes()).unwrap()
            };
            libc::fprintf(
                libc_stderr(),
                b"Usage: %s <seed>\n\0".as_ptr() as *const libc::c_char,
                prog.as_ptr(),
            );
        }
        return 1;
    }

    unsafe {
        *libc::__errno_location() = 0;
        let mut endptr: *mut libc::c_char = ptr::null_mut();
        let arg1 = *argv.add(1);
        let temp_seed = libc::strtoul(arg1, &mut endptr, 10);
        if *endptr != 0
            || *libc::__errno_location() != 0
            || temp_seed > u32::MAX as libc::c_ulong
        {
            libc::fprintf(
                libc_stderr(),
                b"Invalid seed: '%s'\n\0".as_ptr() as *const libc::c_char,
                arg1,
            );
            return 1;
        }

        let seed = temp_seed as libc::c_uint;
        libc::srand(seed);
        for i in 0..ARRAY_SIZE {
            array[i] = libc::rand();
        }
        for _ in 0..ITERATIONS {
            perform_expensive_operations();
        }
        let mut xor_result: i32 = 0;
        for i in 0..ARRAY_SIZE {
            xor_result ^= array[i];
        }
        libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, xor_result);
    }
    0
}

unsafe fn libc_stderr() -> *mut libc::FILE {
    // stderr is typically a macro in C; in libc crate it's accessed differently
    libc::fdopen(2, b"w\0".as_ptr() as *const libc::c_char)
}

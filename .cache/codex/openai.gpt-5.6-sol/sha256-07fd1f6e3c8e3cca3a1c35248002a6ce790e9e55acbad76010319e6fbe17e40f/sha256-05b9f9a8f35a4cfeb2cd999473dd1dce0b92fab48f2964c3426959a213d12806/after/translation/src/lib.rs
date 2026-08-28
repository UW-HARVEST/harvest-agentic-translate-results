use std::ffi::{c_char, c_int, c_void};
use std::ptr;

type File = c_void;

unsafe extern "C" {
    static mut stderr: *mut File;

    fn fclose(stream: *mut File) -> c_int;
    fn ferror(stream: *mut File) -> c_int;
    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut File) -> *mut c_char;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut File;
    fn fprintf(stream: *mut File, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

const READ_MODE: &[u8] = b"r\0";
const PROCESSING_FORMAT: &[u8] = b"Processing: %d\n\0";
const NEGATIVE_INPUT_ERROR: &[u8] = b"Error: negative input\n\0";
const STRING_FORMAT: &[u8] = b"%s\0";
const FILE_ERROR_FORMAT: &[u8] = b"Error: opening or processing file %s\n\0";
const GOTO_OUTPUT_FORMAT: &[u8] = b"Goto output: %d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        // SAFETY: `stderr` and the format string are supplied by the C runtime.
        unsafe {
            fprintf(stderr, NEGATIVE_INPUT_ERROR.as_ptr().cast::<c_char>());
        }
        return -1;
    }

    // SAFETY: the format string matches the promoted C integer argument.
    unsafe {
        printf(PROCESSING_FORMAT.as_ptr().cast::<c_char>(), x);
    }
    x.wrapping_mul(2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut File {
    // SAFETY: the caller supplies `filename`, matching the original C API contract.
    let file = unsafe { fopen(filename, READ_MODE.as_ptr().cast::<c_char>()) };
    if file.is_null() {
        // SAFETY: the format string matches the caller-provided C string argument.
        unsafe {
            fprintf(
                stderr,
                FILE_ERROR_FORMAT.as_ptr().cast::<c_char>(),
                filename,
            );
        }
        return ptr::null_mut();
    }

    let mut buffer = [0_u8; 100];
    // SAFETY: `buffer` is writable for 100 bytes and `file` is an open C stream.
    while !unsafe { fgets(buffer.as_mut_ptr().cast::<c_char>(), 100, file) }.is_null() {
        // SAFETY: `fgets` wrote a null-terminated C string into `buffer`.
        unsafe {
            printf(
                STRING_FORMAT.as_ptr().cast::<c_char>(),
                buffer.as_ptr().cast::<c_char>(),
            );
        }
    }

    // SAFETY: `file` remains an open C stream.
    if unsafe { ferror(file) } != 0 {
        // SAFETY: arguments match the format, and `file` is closed exactly on this path.
        unsafe {
            fprintf(
                stderr,
                FILE_ERROR_FORMAT.as_ptr().cast::<c_char>(),
                filename,
            );
            fclose(file);
        }
        return ptr::null_mut();
    }

    file
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    // SAFETY: this forwards the integer unchanged to the exported implementation.
    let result = unsafe { forward_goto_example(num) };
    if result == -1 {
        return -1;
    }

    // SAFETY: the format string matches the promoted C integer argument.
    unsafe {
        printf(GOTO_OUTPUT_FORMAT.as_ptr().cast::<c_char>(), result);
    }

    // SAFETY: this forwards the caller's filename under the original C API contract.
    let output = unsafe { open_with_cleanup(filename) };
    if output.is_null() {
        return -2;
    }

    // SAFETY: `output` is an open C stream returned by `open_with_cleanup`.
    unsafe {
        fclose(output);
    }
    0
}

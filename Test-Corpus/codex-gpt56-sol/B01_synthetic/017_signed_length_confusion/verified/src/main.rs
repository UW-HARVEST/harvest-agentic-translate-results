use std::ffi::{c_char, c_int};

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

extern "C" {
    static mut stdin: *mut CFile;

    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut CFile) -> *mut c_char;
    fn atoi(value: *const c_char) -> c_int;
    fn strncpy(dest: *mut c_char, source: *const c_char, count: usize) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // libc output preserves printf's buffering behavior, including on a later crash.
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

fn main() {
    let mut data: c_int = -1;

    {
        let mut input_buffer = [0 as c_char; 14];
        let result = unsafe { fgets(input_buffer.as_mut_ptr(), 14, stdin) };
        if !result.is_null() {
            data = unsafe { atoi(input_buffer.as_ptr()) };
        } else {
            print_line(c"fgets() failed.".as_ptr());
        }
    }

    {
        let mut source = [0 as c_char; 100];
        let mut dest = [0 as c_char; 100];
        source[..99].fill(b'A' as c_char);

        if data < 100 {
            // Keep the C conversion to size_t and indexing behavior for negative data.
            unsafe {
                strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
                *dest.as_mut_ptr().offset(data as isize) = 0;
            }
        }
        print_line(dest.as_ptr());
    }
}

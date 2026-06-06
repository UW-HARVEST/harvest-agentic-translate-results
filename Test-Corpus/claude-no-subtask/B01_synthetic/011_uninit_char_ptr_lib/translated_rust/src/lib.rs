use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;
use std::mem::MaybeUninit;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // Compute length until null terminator (mimics C's printf("%s\n", line))
        let mut len = 0usize;
        unsafe {
            while *line.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(line as *const u8, len);
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            let _ = handle.write_all(slice);
            let _ = handle.write_all(b"\n");
            let _ = handle.flush();
        }
    }
}

fn bad() {
    // Mirror the C bug: `char *data;` is uninitialized, then passed to printLine.
    #[allow(invalid_value)]
    let data: *const c_char = unsafe { MaybeUninit::<*const c_char>::uninit().assume_init() };
    print_line(data);
}

fn good() {
    // `data = "string";` — pass a static C string with terminating null byte.
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    print_line(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

use std::ffi::{c_char, c_int, c_void};

type File = c_void;

unsafe extern "C" {
    static mut stdin: *mut File;

    fn atoi(nptr: *const c_char) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut File) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

const INTEGER_FORMAT: &[u8] = b"%d\n\0";
const FGETS_FAILED: &[u8] = b"fgets() failed.\0";
const NEGATIVE_INDEX: &[u8] = b"ERROR: Array index is negative.\0";
const OUT_OF_BOUNDS: &[u8] = b"ERROR: Array index is out-of-bounds\0";
#[cfg(not(test))]
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
#[cfg(not(test))]
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
#[cfg(not(test))]
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
#[cfg(not(test))]
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        puts(line);
    }
}

#[no_mangle]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    printf(INTEGER_FORMAT.as_ptr().cast(), int_number);
}

unsafe fn print_buffer(buffer: &[c_int; 10]) {
    for value in buffer {
        printIntLine(*value);
    }
}

unsafe fn read_data() -> c_int {
    let mut data = -1;
    let mut input_buffer = [0 as c_char; 14];

    if !fgets(input_buffer.as_mut_ptr(), 14, stdin).is_null() {
        data = atoi(input_buffer.as_ptr());
    } else {
        printLine(FGETS_FAILED.as_ptr().cast());
    }

    data
}

#[no_mangle]
pub unsafe extern "C" fn bad() {
    let data = read_data();
    let mut buffer = [0 as c_int; 10];

    if data >= 0 {
        if let Some(element) = buffer.get_mut(data as usize) {
            *element = 1;
        }
        print_buffer(&buffer);
    } else {
        printLine(NEGATIVE_INDEX.as_ptr().cast());
    }
}

unsafe fn good_g2b() {
    let data = 7;
    let mut buffer = [0 as c_int; 10];

    if data >= 0 {
        buffer[data as usize] = 1;
        print_buffer(&buffer);
    } else {
        printLine(NEGATIVE_INDEX.as_ptr().cast());
    }
}

unsafe fn good_b2g() {
    let data = read_data();
    let mut buffer = [0 as c_int; 10];

    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        print_buffer(&buffer);
    } else {
        printLine(OUT_OF_BOUNDS.as_ptr().cast());
    }
}

#[no_mangle]
pub unsafe extern "C" fn good() {
    good_g2b();
    good_b2g();
}

#[cfg(not(test))]
#[export_name = "main"]
pub unsafe extern "C" fn exported_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    printLine(CALLING_GOOD.as_ptr().cast());
    good();
    printLine(FINISHED_GOOD.as_ptr().cast());
    printLine(CALLING_BAD.as_ptr().cast());
    bad();
    printLine(FINISHED_BAD.as_ptr().cast());
    0
}

use libc::{c_char, c_int, FILE};

unsafe extern "C" {
    static mut stdin: *mut FILE;
    fn atoi(nptr: *const c_char) -> c_int;
    fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

const PRINT_LINE_FMT: &[u8] = b"%s\n\0";
const PRINT_INT_FMT: &[u8] = b"%d\n\0";

const FGETS_FAILED: &[u8] = b"fgets() failed.\0";
const NEGATIVE_INDEX: &[u8] = b"ERROR: Array index is negative.\0";
const OOB_INDEX: &[u8] = b"ERROR: Array index is out-of-bounds\0";
const CALLING_GOOD: &[u8] = b"Calling good()...\0";
const FINISHED_GOOD: &[u8] = b"Finished good()\0";
const CALLING_BAD: &[u8] = b"Calling bad()...\0";
const FINISHED_BAD: &[u8] = b"Finished bad()\0";

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(PRINT_LINE_FMT.as_ptr().cast(), line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(PRINT_INT_FMT.as_ptr().cast(), int_number);
    }
}

fn bad() {
    let mut data: c_int = -1;
    {
        let mut input_buffer = [0 as c_char; 14];
        let result = unsafe { fgets(input_buffer.as_mut_ptr(), 14, stdin) };
        if !result.is_null() {
            data = unsafe { atoi(input_buffer.as_ptr()) };
        } else {
            print_line(FGETS_FAILED.as_ptr().cast());
        }
    }
    {
        let mut buffer: [c_int; 10] = [0; 10];
        if data >= 0 {
            unsafe {
                buffer.as_mut_ptr().wrapping_add(data as usize).write(1);
            }
            for value in buffer {
                print_int_line(value);
            }
        } else {
            print_line(NEGATIVE_INDEX.as_ptr().cast());
        }
    }
}

#[allow(non_snake_case)]
#[allow(unused_assignments)]
fn goodG2B() {
    let mut data: c_int = -1;
    data = 7;
    {
        let mut buffer: [c_int; 10] = [0; 10];
        if data >= 0 {
            unsafe {
                buffer.as_mut_ptr().wrapping_add(data as usize).write(1);
            }
            for value in buffer {
                print_int_line(value);
            }
        } else {
            print_line(NEGATIVE_INDEX.as_ptr().cast());
        }
    }
}

#[allow(non_snake_case)]
fn goodB2G() {
    let mut data: c_int = -1;
    {
        let mut input_buffer = [0 as c_char; 14];
        let result = unsafe { fgets(input_buffer.as_mut_ptr(), 14, stdin) };
        if !result.is_null() {
            data = unsafe { atoi(input_buffer.as_ptr()) };
        } else {
            print_line(FGETS_FAILED.as_ptr().cast());
        }
    }
    {
        let mut buffer: [c_int; 10] = [0; 10];
        if data >= 0 && data < 10 {
            unsafe {
                buffer.as_mut_ptr().add(data as usize).write(1);
            }
            for value in buffer {
                print_int_line(value);
            }
        } else {
            print_line(OOB_INDEX.as_ptr().cast());
        }
    }
}

fn good() {
    goodG2B();
    goodB2G();
}

fn main() {
    print_line(CALLING_GOOD.as_ptr().cast());
    good();
    print_line(FINISHED_GOOD.as_ptr().cast());
    print_line(CALLING_BAD.as_ptr().cast());
    bad();
    print_line(FINISHED_BAD.as_ptr().cast());
}

use std::ffi::{c_char, c_double, c_int};

const CHAR_ARRAY_SIZE: usize = 20;

#[repr(C)]
struct File {
    _private: [u8; 0],
}

unsafe extern "C" {
    #[link_name = "stdin"]
    static mut C_STDIN: *mut File;

    fn atof(input: *const c_char) -> c_double;
    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut File) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(c"%s\n".as_ptr(), line);
    }
}

#[no_mangle]
pub unsafe extern "C" fn printIntLine(number: c_int) {
    printf(c"%d\n".as_ptr(), number);
}

fn c_double_to_int(value: f64) -> c_int {
    if value.is_nan() || !(-2147483648.0..2147483648.0).contains(&value) {
        c_int::MIN
    } else {
        value.trunc() as c_int
    }
}

unsafe fn read_data() -> f32 {
    let mut data = 0.0_f32;
    let mut input_buffer = [0 as c_char; CHAR_ARRAY_SIZE];

    if !fgets(input_buffer.as_mut_ptr(), CHAR_ARRAY_SIZE as c_int, C_STDIN).is_null() {
        data = atof(input_buffer.as_ptr()) as f32;
    } else {
        printLine(c"fgets() failed.".as_ptr());
    }

    data
}

unsafe fn divide_and_print(data: f32) {
    let result = c_double_to_int(100.0_f64 / f64::from(data));
    printIntLine(result);
}

#[no_mangle]
pub unsafe extern "C" fn bad() {
    divide_and_print(read_data());
}

unsafe fn good_g2b() {
    divide_and_print(2.0_f32);
}

unsafe fn good_b2g() {
    let data = read_data();

    if f64::from(data).abs() > 0.000001_f64 {
        divide_and_print(data);
    } else {
        printLine(c"This would result in a divide by zero".as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn good() {
    good_g2b();
    good_b2g();
}

#[export_name = "main"]
pub unsafe extern "C" fn driver_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    printLine(c"Calling good()...".as_ptr());
    good();
    printLine(c"Finished good()".as_ptr());
    printLine(c"Calling bad()...".as_ptr());
    bad();
    printLine(c"Finished bad()".as_ptr());
    0
}

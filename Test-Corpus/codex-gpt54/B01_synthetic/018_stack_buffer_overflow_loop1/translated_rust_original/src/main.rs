use std::ffi::{c_char, c_int};
unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(c"%s\n".as_ptr(), line);
        }
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), int_number);
    }
}

fn bad() {
    let mut data = [0 as c_int; 10];
    let source = [0 as c_int; 10];

    for i in 0..10 {
        data[i] = source[i];
    }

    print_int_line(data[0]);
}

fn good() {
    let mut data = [0 as c_int; 10];
    let data_ptr = data.as_mut_ptr();

    let source = [0 as c_int; 10];
    for i in 0..10 {
        unsafe {
            *data_ptr.add(i) = source[i];
        }
    }

    unsafe {
        print_int_line(*data_ptr);
    }
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = print_line as fn(*const c_char);
}

use std::ffi::c_int;

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

fn bad() {
    // alloca(10) — only 10 bytes, not enough for 10 ints (reproducing the bug)
    let mut buf = [0u8; 10];
    let data: *mut c_int = buf.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    unsafe {
        for i in 0..10 {
            *data.add(i) = source[i];
        }
        print_int_line(*data);
    }
}

fn good() {
    // alloca(10 * sizeof(int)) — correct allocation
    let mut buf = [0u8; 10 * std::mem::size_of::<c_int>()];
    let data: *mut c_int = buf.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    unsafe {
        for i in 0..10 {
            *data.add(i) = source[i];
        }
        print_int_line(*data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

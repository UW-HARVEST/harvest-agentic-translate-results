use std::ffi::{c_char, c_int, CStr};
use std::os::raw::c_void;

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

fn bad() {
    let data: *mut c_int = unsafe {
        std::alloc::alloc(std::alloc::Layout::from_size_align(10, std::mem::align_of::<c_int>()).unwrap()) as *mut c_int
    };
    if data.is_null() {
        return;
    }
    {
        let source: [c_int; 10] = [0; 10];
        for i in 0..10 {
            unsafe {
                *data.add(i) = source[i];
            }
        }
        unsafe {
            print_int_line(*data);
        }
    }
    unsafe {
        std::alloc::dealloc(data as *mut u8, std::alloc::Layout::from_size_align(10, std::mem::align_of::<c_int>()).unwrap());
    }
}

fn good() {
    let data: *mut c_int = unsafe {
        std::alloc::alloc(std::alloc::Layout::from_size_align(10 * std::mem::size_of::<c_int>(), std::mem::align_of::<c_int>()).unwrap()) as *mut c_int
    };
    if data.is_null() {
        return;
    }
    {
        let source: [c_int; 10] = [0; 10];
        for i in 0..10 {
            unsafe {
                *data.add(i) = source[i];
            }
        }
        unsafe {
            print_int_line(*data);
        }
    }
    unsafe {
        std::alloc::dealloc(data as *mut u8, std::alloc::Layout::from_size_align(10 * std::mem::size_of::<c_int>(), std::mem::align_of::<c_int>()).unwrap());
    }
}
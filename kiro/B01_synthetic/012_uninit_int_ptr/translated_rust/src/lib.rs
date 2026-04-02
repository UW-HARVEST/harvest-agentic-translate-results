use std::io::{self, Read};
use std::mem::MaybeUninit;

extern "C" {
    fn printf(fmt: *const i8, ...) -> i32;
}

#[no_mangle]
pub extern "C" fn printIntPtrLine(int_number: *const i32) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const i8, *int_number);
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
    let ptr = unsafe { data.assume_init() };
    printIntPtrLine(ptr);
}

#[no_mangle]
pub extern "C" fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    printIntPtrLine(data_addr);
}

pub fn run_main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}

#[cfg(feature = "export_main")]
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    run_main();
    0
}

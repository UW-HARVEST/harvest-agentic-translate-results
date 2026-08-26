use std::ffi::{c_char, c_int};

#[cfg(not(test))]
const SCAN_FORMAT: &[u8] = b"%f\0";
const BYTE_FORMAT: &[u8] = b"%02x\0";
const NEWLINE_FORMAT: &[u8] = b"\n\0";

unsafe extern "C" {
    #[cfg(not(test))]
    #[cfg_attr(target_env = "gnu", link_name = "__isoc99_scanf")]
    #[cfg_attr(not(target_env = "gnu"), link_name = "scanf")]
    fn c_scanf(format: *const c_char, ...) -> c_int;

    #[link_name = "printf"]
    fn c_printf(format: *const c_char, ...) -> c_int;
}

#[cfg(not(test))]
fn scan_float(value: &mut f32) {
    unsafe {
        c_scanf(SCAN_FORMAT.as_ptr().cast::<c_char>(), value as *mut f32);
    }
}

fn print_hex(value: f32) {
    for byte in value.to_ne_bytes() {
        unsafe {
            c_printf(BYTE_FORMAT.as_ptr().cast::<c_char>(), c_int::from(byte));
        }
    }

    unsafe {
        c_printf(NEWLINE_FORMAT.as_ptr().cast::<c_char>());
    }
}

#[no_mangle]
pub extern "C" fn driver(value: f32) {
    print_hex(value);
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main() -> c_int {
    let mut value = 0.0_f32;
    scan_float(&mut value);
    driver(value);
    0
}

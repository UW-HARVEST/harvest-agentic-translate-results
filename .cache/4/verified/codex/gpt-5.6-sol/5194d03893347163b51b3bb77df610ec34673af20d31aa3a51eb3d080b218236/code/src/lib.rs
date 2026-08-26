use std::ffi::{c_char, c_int, c_void};

type File = c_void;

const HEX_FORMAT: &[u8] = b"%02x\n\0";
const CHAR_FORMAT: &[u8] = b"%c\0";

extern "C" {
    static mut stdin: *mut File;

    fn printf(format: *const c_char, ...) -> c_int;

    #[link_name = "__isoc99_fscanf"]
    fn fscanf(stream: *mut File, format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn printHexCharLine(char_hex: c_char) {
    printf(HEX_FORMAT.as_ptr().cast(), c_int::from(char_hex));
}

#[export_name = "main"]
pub unsafe extern "C" fn c_main() -> c_int {
    let mut data: c_char = b' ' as c_char;
    fscanf(
        stdin,
        CHAR_FORMAT.as_ptr().cast(),
        std::ptr::addr_of_mut!(data),
    );

    let result = data.wrapping_add(1);
    printHexCharLine(result);
    0
}

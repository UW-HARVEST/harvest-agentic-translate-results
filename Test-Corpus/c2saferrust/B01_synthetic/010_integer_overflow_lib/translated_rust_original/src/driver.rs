

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn printHexCharLine(char_hex: ::core::ffi::c_char) {
    println!("{:02x}", char_hex as u8);
}

#[no_mangle]
pub fn driver(data: i8) {
    let result = data.wrapping_add(1);
    printHexCharLine(result);
}


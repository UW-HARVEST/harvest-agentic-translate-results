use std::ffi::c_char;

/// Matches C: printf("%02x\n", charHex)
/// char is promoted to int (sign-extended), then printed as unsigned hex.
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    let promoted = char_hex as i32;
    let as_unsigned = promoted as u32;
    if as_unsigned <= 0xff {
        println!("{:02x}", as_unsigned);
    } else {
        println!("{:x}", as_unsigned);
    }
}

/// Matches C: char result = data + 1; (wrapping signed addition)
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result = (data as i8).wrapping_add(1) as c_char;
    printHexCharLine(result);
}

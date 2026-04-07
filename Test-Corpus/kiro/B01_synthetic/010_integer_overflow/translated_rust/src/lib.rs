use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    let promoted = char_hex as i32 as u32;
    print!("{:02x}\n", promoted);
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main() -> std::os::raw::c_int {
    let mut data: c_char = b' ' as c_char;
    let mut buf = [0u8; 1];
    use std::io::Read;
    if std::io::stdin().read(&mut buf).unwrap_or(0) > 0 {
        data = buf[0] as c_char;
    }
    let result: c_char = data.wrapping_add(1);
    printHexCharLine(result);
    0
}

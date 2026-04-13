use std::ffi::c_char;

fn print_hex_char_line(char_hex: c_char) {
    println!("{:02x}", char_hex as u8);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}
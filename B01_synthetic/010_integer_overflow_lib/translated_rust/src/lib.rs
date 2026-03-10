use std::os::raw::c_char;

fn print_hex_char_line(char_hex: c_char) {
    // C printf("%02x\n", charHex): char promotes to int (sign-extends), printed as unsigned hex
    let promoted = char_hex as i32;
    let as_unsigned = promoted as u32;
    println!("{:02x}", as_unsigned);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}

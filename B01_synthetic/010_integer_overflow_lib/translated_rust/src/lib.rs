use std::os::raw::c_char;

fn print_hex_char_line(char_hex: c_char) {
    // C printf("%02x\n", charHex) promotes char to int (sign-extends),
    // then prints as unsigned hex. Reproduce by casting i8 -> i32 -> u32.
    let as_int = char_hex as i32;
    println!("{:02x}", as_int as u32);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    // C: char result = data + 1;  (integer promotion, then truncation back to char)
    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}

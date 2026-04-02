use std::io::Read;

fn print_hex_char_line(char_hex: i8) {
    // C promotes signed char to int, then %02x prints as unsigned hex of that int.
    // e.g. (char)-1 -> (int)-1 -> printed as "ffffffff"
    let as_i32 = char_hex as i32;
    let as_u32 = as_i32 as u32;
    println!("{:02x}", as_u32);
}

#[no_mangle]
pub extern "C" fn printHexCharLine(char_hex: i8) {
    print_hex_char_line(char_hex);
}

fn main() {
    let mut data: i8 = b' ' as i8;
    // fscanf(stdin, "%c", &data) reads exactly one byte
    let mut buf = [0u8; 1];
    if std::io::stdin().read(&mut buf).unwrap_or(0) > 0 {
        data = buf[0] as i8;
    }
    // char arithmetic with wrapping (C signed overflow is UB but wraps on x86)
    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);
}

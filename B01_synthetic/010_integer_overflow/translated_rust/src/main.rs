use std::io::Read;

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex) — char promotes to int (sign-extends),
    // then %x prints the unsigned representation of that int.
    let as_i32 = char_hex as i32;
    let as_u32 = as_i32 as u32;
    println!("{:02x}", as_u32);
}

fn main() {
    let mut data: i8 = b' ' as i8;
    // fscanf(stdin, "%c", &data) — reads one byte; on failure data stays ' '
    let mut buf = [0u8; 1];
    if std::io::stdin().read(&mut buf).unwrap_or(0) == 1 {
        data = buf[0] as i8;
    }
    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);
}

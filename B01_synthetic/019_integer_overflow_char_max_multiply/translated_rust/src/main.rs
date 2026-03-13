use std::io::{self, Read};

fn print_line(line: &str) {
    println!("{}", line);
}

/// Matches C's printf("%02x\n", charHex) where charHex is a signed char.
/// C promotes char to int, so negative values print as 32-bit two's complement.
fn print_hex_char_line(char_hex: i8) {
    let promoted = char_hex as i32;
    if promoted >= 0 {
        println!("{:02x}", promoted);
    } else {
        // C prints negative char as unsigned 32-bit hex via int promotion
        println!("{:02x}", promoted as u32);
    }
}

fn bad() {
    let data: i8 = i8::MAX; // CHAR_MAX = 127
    if data > 0 {
        let result: i8 = data.wrapping_mul(2); // 127 * 2 overflows to -2
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let data: i8 = i8::MAX; // CHAR_MAX
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            print_line("data value is too large to perform arithmetic safely.");
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    // Read all stdin, then parse first integer (matches scanf("%d", &x) behavior)
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: i32 = input.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}

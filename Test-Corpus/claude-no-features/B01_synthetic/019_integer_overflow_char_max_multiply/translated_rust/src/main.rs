use std::io::{self, Read};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex)
    // In C, the char is promoted to int when passed as a variadic arg.
    // %x then interprets the int as unsigned int.
    let promoted: i32 = char_hex as i32;
    let unsigned: u32 = promoted as u32;
    println!("{:02x}", unsigned);
}

fn bad() {
    let data: i8 = i8::MAX;
    if data > 0 {
        // C: char result = data * 2;
        // data is promoted to int, multiplied (127*2=254), then narrowed to signed char.
        // On typical x86 GCC this truncates to -2 (0xFE).
        let result: i8 = data.wrapping_mul(2);
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
    let mut data: i8;
    #[allow(unused_assignments)]
    {
        data = b' ' as i8;
    }
    data = i8::MAX;
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

/// Mimic scanf("%d", &x) behavior: skip leading whitespace, then parse
/// the longest prefix of an optional sign followed by decimal digits.
/// If no valid integer can be parsed, x remains unchanged.
fn scanf_d(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // Skip leading whitespace (matches C isspace for typical scanf %d).
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }
    let start = i;
    if i < input.len() && (input[i] == b'-' || input[i] == b'+') {
        i += 1;
    }
    let digits_start = i;
    while i < input.len() && input[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    let s = std::str::from_utf8(&input[start..i]).ok()?;
    // C scanf with %d on overflow has undefined behavior; we just parse normally.
    s.parse::<i32>().ok()
}

fn main() {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok();

    let mut x: i32 = 0;
    if let Some(parsed) = scanf_d(&buf) {
        x = parsed;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}

// Translation of c_src/src/main.c to Rust.
// Produces byte-identical output to the original C program.

use std::io::{self, Read, Write};

const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: &str) {
    // C: if(line != NULL) printf("%s\n", line);
    // We always pass a real string, so just print it.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.write_all(b"\n");
}

fn print_hex_char_line(char_hex: i8) {
    // C: printf("%02x\n", charHex);
    // In C, the (signed) char is promoted to int when passed as a variadic
    // argument. The %x conversion then interprets it as unsigned int.
    // A negative char becomes a 32-bit two's-complement value (e.g. -2 -> 0xfffffffe).
    // %02x has minimum width 2, so values < 0x10 print as two hex digits with
    // a leading zero, but values wider than 2 hex digits print without truncation.
    let promoted: i32 = char_hex as i32;
    let as_unsigned: u32 = promoted as u32;
    let s = if as_unsigned <= 0xff {
        format!("{:02x}", as_unsigned)
    } else {
        // Print full unsigned hex value (no leading zero needed when width is exceeded).
        format!("{:x}", as_unsigned)
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(s.as_bytes());
    let _ = out.write_all(b"\n");
}

fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        // char result = data * 2; — C promotes to int, then assigns back to char,
        // which truncates / wraps modulo 256.
        let product: i32 = (data as i32) * 2;
        let result: i8 = (product as i64 as i32 & 0xff) as i8; // emulate truncation to signed char
        // Equivalent: take low 8 bits and reinterpret as signed.
        let _ = result;
        // Use wrapping conversion explicitly:
        let result_wrapped: i8 = (product as u8) as i8;
        print_hex_char_line(result_wrapped);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let product: i32 = (data as i32) * 2;
        let result: i8 = (product as u8) as i8;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    #[allow(unused_assignments)]
    let mut data: i8 = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let product: i32 = (data as i32) * 2;
            let result: i8 = (product as u8) as i8;
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

/// Emulate scanf("%d", &x). Skips leading whitespace and reads an optional
/// sign followed by decimal digits. If no integer is matched, x is left as 0.
fn scanf_int(input: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace: space, \t, \n, \v, \f, \r).
    while i < input.len() {
        let c = input[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    if i >= input.len() {
        return 0;
    }
    let mut sign: i64 = 1;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        sign = -1;
        i += 1;
    }
    let mut have_digit = false;
    let mut value: i64 = 0;
    while i < input.len() {
        let c = input[i];
        if c.is_ascii_digit() {
            have_digit = true;
            value = value
                .wrapping_mul(10)
                .wrapping_add((c - b'0') as i64);
            i += 1;
        } else {
            break;
        }
    }
    if !have_digit {
        return 0;
    }
    let result = sign.wrapping_mul(value);
    // Truncate to int (32-bit).
    result as i32
}

fn main() {
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);
    let x = scanf_int(&buf);

    if x != 0 {
        good();
    } else {
        bad();
    }
}

use std::env;
use std::os::unix::ffi::OsStrExt;

const LONG_MAX: i128 = i64::MAX as i128;
const LONG_MIN_ABS: i128 = (i64::MAX as i128) + 1;

fn c_isspace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn parse_strtol_base10_to_int(bytes: &[u8]) -> Option<i32> {
    let mut index = 0;
    while index < bytes.len() && c_isspace(bytes[index]) {
        index += 1;
    }

    let mut negative = false;
    if index < bytes.len() {
        if bytes[index] == b'-' {
            negative = true;
            index += 1;
        } else if bytes[index] == b'+' {
            index += 1;
        }
    }

    if index >= bytes.len() || !bytes[index].is_ascii_digit() {
        return None;
    }

    let limit = if negative { LONG_MIN_ABS } else { LONG_MAX };
    let mut magnitude = 0_i128;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        let digit = (bytes[index] - b'0') as i128;
        if magnitude <= limit {
            magnitude = magnitude * 10 + digit;
            if magnitude > limit {
                magnitude = limit;
            }
        }
        index += 1;
    }

    let long_value = if negative { -magnitude } else { magnitude };
    Some(long_value as i64 as i32)
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 2 {
        print!("Error: should only be a single (integer) argument!\n");
        std::process::exit(1);
    }

    let mut val = match parse_strtol_base10_to_int(args[1].as_os_str().as_bytes()) {
        Some(value) => value,
        None => {
            print!("Error: first argument must be an integer!\n");
            std::process::exit(1);
        }
    };

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val = val.wrapping_add(1);
    }
}

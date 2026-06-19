use std::env;
use std::os::unix::ffi::OsStringExt;

fn is_c_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn strtol_to_int_base10(bytes: &[u8]) -> Option<i32> {
    let mut index = 0;
    while index < bytes.len() && is_c_space(bytes[index]) {
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

    let mut value: u64 = 0;
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };

    while index < bytes.len() && bytes[index].is_ascii_digit() {
        let digit = (bytes[index] - b'0') as u64;
        if value <= (limit - digit) / 10 {
            value = value * 10 + digit;
        } else {
            value = limit;
        }
        index += 1;
    }

    let long_value = if negative {
        if value == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };

    Some(long_value as i32)
}

fn static_sum(sum: &mut i32, update: i32) -> i32 {
    *sum = sum.wrapping_add(update);
    *sum
}

fn main() {
    let args: Vec<Vec<u8>> = env::args_os().map(OsStringExt::into_vec).collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        std::process::exit(1);
    }

    let stride = match strtol_to_int_base10(&args[1]) {
        Some(value) => value,
        None => {
            println!("Error: first argument must be an integer!");
            std::process::exit(1);
        }
    };

    let mut sum = 0;
    for i in 0..10_i32 {
        println!("{}", static_sum(&mut sum, i.wrapping_mul(stride)));
    }
}

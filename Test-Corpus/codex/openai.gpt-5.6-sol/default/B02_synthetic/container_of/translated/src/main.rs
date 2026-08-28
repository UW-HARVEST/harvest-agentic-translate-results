use std::env;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

fn atoi(value: &OsStr) -> i32 {
    let bytes = value.as_bytes();
    let mut index = 0;

    while index < bytes.len()
        && matches!(bytes[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = match bytes.get(index) {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0_u64;

    while let Some(&byte) = bytes.get(index) {
        if !byte.is_ascii_digit() {
            break;
        }

        magnitude = magnitude
            .checked_mul(10)
            .and_then(|number| number.checked_add((byte - b'0') as u64))
            .unwrap_or(limit)
            .min(limit);
        index += 1;
    }

    let value = if negative {
        if magnitude == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude as i64
    };

    value as i32
}

fn main() {
    let mut arguments = env::args_os();
    arguments.next();

    let first = arguments.next().unwrap_or_else(|| std::process::abort());
    let a = atoi(&first);
    let second = arguments.next().unwrap_or_else(|| std::process::abort());
    let b = atoi(&second);

    println!("{}", a.wrapping_add(b));
}

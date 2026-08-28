use std::env;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

fn static_sum(update: i32) -> i32 {
    SUM.fetch_add(update, Ordering::Relaxed)
        .wrapping_add(update)
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn strtol_base_10(argument: &OsStr) -> Option<i64> {
    let bytes = argument.as_bytes();
    let mut index = 0;

    while index < bytes.len() && is_c_whitespace(bytes[index]) {
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

    let digit_start = index;
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = 0_u64;

    while let Some(digit) = bytes.get(index).and_then(|byte| byte.checked_sub(b'0')) {
        if digit > 9 {
            break;
        }

        value = value
            .checked_mul(10)
            .and_then(|current| current.checked_add(u64::from(digit)))
            .unwrap_or(u64::MAX)
            .min(limit);
        index += 1;
    }

    if index == digit_start {
        return None;
    }

    if negative {
        if value == (i64::MAX as u64) + 1 {
            Some(i64::MIN)
        } else {
            Some(-(value as i64))
        }
    } else {
        Some(value as i64)
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().collect();

    if arguments.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::FAILURE;
    }

    let Some(parsed_stride) = strtol_base_10(&arguments[1]) else {
        println!("Error: first argument must be an integer!");
        return ExitCode::FAILURE;
    };
    let stride = parsed_stride as i32;

    for i in 0_i32..10 {
        println!("{}", static_sum(i.wrapping_mul(stride)));
    }

    ExitCode::SUCCESS
}

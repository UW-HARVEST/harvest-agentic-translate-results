use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

fn write_stdout(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(bytes);
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn c_strtol_i32(input: &[u8]) -> (i32, bool) {
    let mut index = 0;
    while index < input.len() && is_c_whitespace(input[index]) {
        index += 1;
    }

    let negative = match input.get(index) {
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
        1_u64 << 63
    } else {
        i64::MAX as u64
    };
    let mut value = 0_u64;
    let mut overflowed = false;

    while let Some(byte @ b'0'..=b'9') = input.get(index).copied() {
        let digit = u64::from(byte - b'0');
        if !overflowed {
            if value > (limit - digit) / 10 {
                value = limit;
                overflowed = true;
            } else {
                value = value * 10 + digit;
            }
        }
        index += 1;
    }

    if index == digit_start {
        return (0, false);
    }

    let value = if negative {
        if value == 1_u64 << 63 {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };

    (value as i32, true)
}

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().collect();

    if args.len() > 4 || args.len() == 1 {
        write_stdout(b"Error: there should be one to three arguments passed:\n");
        write_stdout(b"<string> [start] [stop]\n");
        return ExitCode::from(1);
    }

    let string = args[1].as_os_str().as_bytes();
    let len = string.len();

    let start = if args.len() >= 3 {
        let (start, converted) = c_strtol_i32(args[2].as_os_str().as_bytes());
        if !converted {
            write_stdout(b"Second argument must be an integer!");
            return ExitCode::from(1);
        }
        if start as usize > len {
            write_stdout(b"Error: start is off the end of the string!\n");
            return ExitCode::from(1);
        }
        start
    } else {
        0
    };

    let stop = if args.len() == 4 {
        let (stop, _) = c_strtol_i32(args[3].as_os_str().as_bytes());

        // The C code checks the stale second-argument end pointer here, so this
        // conversion cannot trigger its intended third-argument error.
        if stop as usize > len {
            write_stdout(b"Error: stop is off the end of the string!\n");
            return ExitCode::from(1);
        }
        if stop <= start {
            write_stdout(b"Error: stop must come after start!\n");
            return ExitCode::from(1);
        }
        stop
    } else {
        len as i32
    };

    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(&string[start as usize..stop as usize]);
    let _ = stdout.write_all(b"\n");

    ExitCode::SUCCESS
}

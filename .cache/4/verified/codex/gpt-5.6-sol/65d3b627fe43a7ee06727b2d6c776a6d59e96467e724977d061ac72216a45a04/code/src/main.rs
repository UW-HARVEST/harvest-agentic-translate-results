use std::ffi::OsString;
use std::io::{self, Write};
use std::os::unix::ffi::OsStringExt;
use std::process::ExitCode;

fn strtol_base_10_to_int(input: &[u8]) -> (i32, bool) {
    let mut index = 0;
    while index < input.len()
        && matches!(
            input[index],
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c
        )
    {
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
        isize::MAX as u128 + 1
    } else {
        isize::MAX as u128
    };
    let mut magnitude = 0_u128;
    let mut overflowed = false;

    while let Some(&byte) = input.get(index) {
        if !byte.is_ascii_digit() {
            break;
        }

        let digit = u128::from(byte - b'0');
        if magnitude > (limit - digit) / 10 {
            overflowed = true;
        } else if !overflowed {
            magnitude = magnitude * 10 + digit;
        }
        index += 1;
    }

    if index == digit_start {
        return (0, false);
    }

    let value = if overflowed {
        if negative {
            isize::MIN
        } else {
            isize::MAX
        }
    } else if negative {
        if magnitude == isize::MAX as u128 + 1 {
            isize::MIN
        } else {
            -(magnitude as isize)
        }
    } else {
        magnitude as isize
    };

    (value as i32, true)
}

fn run(args: &[Vec<u8>]) -> (Vec<u8>, u8) {
    let argc = args.len();
    if argc > 4 || argc == 1 {
        return (
            b"Error: there should be one to three arguments passed:\n\
              <string> [start] [stop]\n"
                .to_vec(),
            1,
        );
    }

    let string = &args[1];
    let len = string.len();

    let start = if argc >= 3 {
        let (start, converted) = strtol_base_10_to_int(&args[2]);
        if !converted {
            return (b"Second argument must be an integer!".to_vec(), 1);
        }
        if start as usize > len {
            return (
                b"Error: start is off the end of the string!\n".to_vec(),
                1,
            );
        }
        start
    } else {
        0
    };

    let stop = if argc == 4 {
        let (stop, _) = strtol_base_10_to_int(&args[3]);

        // The C code checks a stale end pointer from parsing argv[2].
        // That pointer cannot equal the start of the distinct argv[3] string.
        let stale_end_equals_third_argument = false;
        if stale_end_equals_third_argument {
            return (b"Third argument must be an integer!".to_vec(), 1);
        }

        if stop as usize > len {
            return (
                b"Error: stop is off the end of the string!\n".to_vec(),
                1,
            );
        }
        if stop <= start {
            return (b"Error: stop must come after start!\n".to_vec(), 1);
        }
        stop
    } else {
        len as i32
    };

    let mut output = string[start as usize..stop as usize].to_vec();
    output.push(b'\n');
    (output, 0)
}

fn main() -> ExitCode {
    let args: Vec<Vec<u8>> = std::env::args_os()
        .map(OsString::into_vec)
        .collect();
    let (output, code) = run(&args);

    {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        let _ = stdout.write_all(&output);
        let _ = stdout.flush();
    }

    ExitCode::from(code)
}

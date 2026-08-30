// Translation of c_src/src/main.c
//
// Takes two arguments, a base and an exponent, and prints base^exponent.
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

mod bignum;
mod strtod;

use std::io::Write;

/// C `errno` values used by this program (Linux/glibc).
const EDOM: i32 = 33;
const ERANGE: i32 = 34;

fn main() {
    let code = run();
    std::process::exit(code);
}

fn run() -> i32 {
    let args = argv();

    // if (argc != 3)
    if args.len() != 3 {
        let mut msg = b"Usage: ".to_vec();
        match args.first() {
            Some(a) => msg.extend_from_slice(a),
            None => msg.extend_from_slice(b"(null)"),
        }
        msg.extend_from_slice(b" base exponent\n");
        write_stderr(&msg);
        return 1;
    }

    // Convert base
    let conv1 = strtod::strtod(&args[1]);
    let base = conv1.value;
    if conv1.erange {
        let mut msg = b"Range error while converting base '".to_vec();
        msg.extend_from_slice(&args[1]);
        msg.extend_from_slice(b"'\n");
        write_stderr(&msg);
        return 1;
    } else if conv1.consumed != args[1].len() {
        let mut msg = b"Invalid numeric input for base: '".to_vec();
        msg.extend_from_slice(&args[1]);
        msg.extend_from_slice(b"'\n");
        write_stderr(&msg);
        return 1;
    }

    // Convert exponent
    let conv2 = strtod::strtod(&args[2]);
    let exponent = conv2.value;
    if conv2.erange {
        let mut msg = b"Range error while converting exponent '".to_vec();
        msg.extend_from_slice(&args[2]);
        msg.extend_from_slice(b"'\n");
        write_stderr(&msg);
        return 1;
    } else if conv2.consumed != args[2].len() {
        let mut msg = b"Invalid numeric input for exponent: '".to_vec();
        msg.extend_from_slice(&args[2]);
        msg.extend_from_slice(b"'\n");
        write_stderr(&msg);
        return 1;
    }

    // Calculate power
    let (result, err) = pow_with_errno(base, exponent);
    if err == EDOM {
        let msg = format!(
            "Domain error: pow({}, {}) is undefined in the real number domain.\n",
            fmt2(base),
            fmt2(exponent)
        );
        write_stderr(msg.as_bytes());
        return 1;
    } else if err == ERANGE {
        let msg = format!(
            "Range error: pow({}, {}) caused overflow or underflow.\n",
            fmt2(base),
            fmt2(exponent)
        );
        write_stderr(msg.as_bytes());
        return 1;
    }

    let out = format!("Result: {}\n", fmt2(result));
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
    0
}

/// Raw (byte exact) command line arguments, as C's `argv`.
fn argv() -> Vec<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::env::args_os()
            .map(|a| a.as_bytes().to_vec())
            .collect()
    }
    #[cfg(not(unix))]
    {
        std::env::args_os()
            .map(|a| a.to_string_lossy().into_owned().into_bytes())
            .collect()
    }
}

fn write_stderr(bytes: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}

/// `printf("%.2f", x)` as glibc renders it.
fn fmt2(x: f64) -> String {
    if x.is_nan() {
        return if x.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.2}", x)
}

/// `pow` together with the `errno` value glibc would leave behind.
fn pow_with_errno(x: f64, y: f64) -> (f64, i32) {
    let r = x.powf(y);

    // Special operands are all handled exactly by pow; no exception is raised.
    if x.is_nan() || y.is_nan() {
        return (r, 0);
    }
    if r.is_nan() {
        // Invalid operation: negative base with a non-integer exponent.
        return (r, EDOM);
    }
    if x.is_infinite() || y.is_infinite() {
        return (r, 0);
    }
    if r.is_infinite() {
        // Overflow, or a pole error for pow(+-0, negative).
        return (r, ERANGE);
    }
    if r == 0.0 && x != 0.0 {
        // Underflow all the way to zero.
        return (r, ERANGE);
    }
    (r, 0)
}

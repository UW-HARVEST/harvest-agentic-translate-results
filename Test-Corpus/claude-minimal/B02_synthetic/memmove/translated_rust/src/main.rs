//! Translation of c_src/src/main.c to Rust.

use std::io::{self, Read, Write};
use std::process::ExitCode;

use driver::process_buffer;

fn read_all_stdin() -> io::Result<String> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;
    Ok(s)
}

fn main() -> ExitCode {
    let input = match read_all_stdin() {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(io::stderr(), "Error reading input");
            return ExitCode::from(1);
        }
    };

    let mut tokens = input.split_ascii_whitespace();

    // Read flags (u32)
    let flags: u32 = match tokens.next().and_then(|t| t.parse::<u32>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading flags");
            return ExitCode::from(1);
        }
    };

    // Read param1 (i32)
    let param1: i32 = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading param1");
            return ExitCode::from(1);
        }
    };

    // Read param2 (i32)
    let param2: i32 = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading param2");
            return ExitCode::from(1);
        }
    };

    // Read length (usize)
    let length: usize = match tokens.next().and_then(|t| t.parse::<usize>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading length");
            return ExitCode::from(1);
        }
    };

    if length > 256 {
        let _ = writeln!(
            io::stderr(),
            "Error: length {} exceeds maximum 256",
            length
        );
        return ExitCode::from(1);
    }

    let mut buffer = [0u8; 256];

    // Read buffer data
    for i in 0..length {
        let byte: u32 = match tokens.next().and_then(|t| t.parse::<u32>().ok()) {
            Some(v) => v,
            None => {
                let _ = writeln!(io::stderr(), "Error reading byte {}", i);
                return ExitCode::from(1);
            }
        };
        buffer[i] = byte as u8;
    }

    // Process the buffer
    let new_length = process_buffer(&mut buffer[..], length, flags, param1, param2);

    // Output new length and buffer contents
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}", new_length);
    for i in 0..new_length {
        let _ = write!(out, " {}", buffer[i]);
    }
    let _ = writeln!(out);

    ExitCode::from(0)
}

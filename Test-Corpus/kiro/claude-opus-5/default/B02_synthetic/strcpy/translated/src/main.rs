//! Translation of `c_src/src/main.c`.

mod scan;
mod strcpy_fun;

use std::io::Write;
use std::process::ExitCode;

use scan::Scanner;
use strcpy_fun::{process_strings, BUF_SIZE};

const MAX_BUFFER_SIZE: usize = 1024;

fn main() -> ExitCode {
    let stdin = std::io::stdin();
    let mut sc = Scanner::new(stdin.lock());

    // The C code leaves these stack buffers uninitialised; see the memory model
    // note in `strcpy_fun.rs`.
    let mut input_buffer = [0u8; BUF_SIZE];
    let mut ref_buffer = [0u8; BUF_SIZE];

    /* Read operation */
    let operation = match sc.scan_int() {
        Some(v) => v,
        None => {
            eprint!("Error reading operation\n");
            return ExitCode::from(1);
        }
    };

    /* Read flags */
    let flags = match sc.scan_uint() {
        Some(v) => v,
        None => {
            eprint!("Error reading flags\n");
            return ExitCode::from(1);
        }
    };

    /* Read input length */
    let input_len = match sc.scan_size() {
        Some(v) => v,
        None => {
            eprint!("Error reading input length\n");
            return ExitCode::from(1);
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        );
        return ExitCode::from(1);
    }

    /* Read input buffer data */
    for i in 0..input_len {
        let byte = match sc.scan_uint() {
            Some(v) => v,
            None => {
                eprint!("Error reading input byte {}\n", i);
                return ExitCode::from(1);
            }
        };
        input_buffer[i] = byte as u8;
    }

    /* Read reference length */
    let ref_len = match sc.scan_size() {
        Some(v) => v,
        None => {
            eprint!("Error reading reference length\n");
            return ExitCode::from(1);
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        );
        return ExitCode::from(1);
    }

    /* Read reference buffer data */
    for i in 0..ref_len {
        let byte = match sc.scan_uint() {
            Some(v) => v,
            None => {
                eprint!("Error reading reference byte {}\n", i);
                return ExitCode::from(1);
            }
        };
        ref_buffer[i] = byte as u8;
    }

    /* Call the library function */
    let result = process_strings(
        &input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    /* Print result to stdout */
    print!("{}\n", result);
    let _ = std::io::stdout().flush();

    ExitCode::from(0)
}

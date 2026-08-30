//! Translation of `c_src/src/main.c`.

mod frame;
mod scan;
mod strcpy_fun;
mod uninit;

use std::io::Write;
use std::process::ExitCode;

use frame::Frame;
use scan::Scanner;
use strcpy_fun::process_strings;

const MAX_BUFFER_SIZE: usize = 1024;

// The frame model covers both `MAX_BUFFER_SIZE` buffers.
const _: () = assert!(frame::BUF_SIZE == MAX_BUFFER_SIZE);

fn main() -> ExitCode {
    let stdin = std::io::stdin();
    let mut sc = Scanner::new(stdin.lock());

    // `char input_buffer[MAX_BUFFER_SIZE]; char ref_buffer[MAX_BUFFER_SIZE];`
    // are uninitialised locals; `Frame` models them together with the rest of
    // `main`'s frame, because `process_strings` reads past both of them.
    let mut fr = Frame::new();

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
        fr.store_input(i, byte as u8);
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
        fr.store_ref(i, byte as u8);
    }

    fr.commit_locals(input_len, ref_len, operation, flags);

    /* Call the library function */
    let result = process_strings(&fr, input_len, ref_len, operation, flags);

    /* Print result to stdout */
    print!("{}\n", result);
    let _ = std::io::stdout().flush();

    ExitCode::from(0)
}

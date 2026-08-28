//! Translation of `c_src/src/main.c`.
//!
//! The C `main()` keeps two *uninitialised* 1024 byte buffers on the stack and
//! fills only the first `input_len` / `ref_len` bytes of them from stdin.  The
//! library it calls happily runs `strlen`/`strcmp` over those buffers, so it
//! reads the leftover stack contents whenever the data it was given is not NUL
//! terminated (which is exactly the bug the original code advertises in its
//! comments).
//!
//! Since Rust has no uninitialised memory, the buffers are pre-filled with a
//! deterministic imitation of the residue that the dynamic loader and the C
//! start-up code leave on the stack of a freshly started x86-64 process:
//! a run of 8-byte slots holding leftover 48-bit pointers, i.e. six non-zero
//! bytes followed by two zero bytes.  This keeps the translation deterministic
//! while behaving like the original program (strings are *not* implicitly
//! terminated at the end of the data that was read, but a NUL is found a few
//! bytes further on).

mod cstr;
mod scanf;
mod strcpy_fun;

use std::io::{self, Write};

const MAX_BUFFER_SIZE: usize = 1024;

/// Extra modelled bytes behind each buffer, standing in for the stack memory
/// that follows it; used by the C code when a buffer is completely filled
/// without a NUL terminator.
const OVERRUN_PAD: usize = 16;

/// Build one of the two stack buffers, filled with modelled stack residue.
///
/// Every eight bytes hold a leftover "pointer": six pseudo-random non-zero
/// bytes followed by the two zero high-order bytes of a 48-bit address.
fn stack_residue(slot_seed: u64) -> Vec<u8> {
    let mut buffer = vec![0u8; MAX_BUFFER_SIZE + OVERRUN_PAD];
    for (offset, byte) in buffer.iter_mut().enumerate() {
        if offset % 8 < 6 {
            let mixed = offset as u64 * 31 + slot_seed * 97 + 7;
            *byte = (mixed % 251 + 1) as u8;
        }
    }
    buffer
}

fn main() {
    let code = run();
    let _ = io::stdout().flush();
    std::process::exit(code);
}

fn run() -> i32 {
    let stdin = io::stdin();
    let mut sc = scanf::Scanner::new(stdin.lock());

    let mut input_buffer = stack_residue(1);
    let mut ref_buffer = stack_residue(2);

    /* Read operation */
    let operation: i32 = match sc.read_int() {
        Some(v) => v,
        None => {
            eprintln!("Error reading operation");
            return 1;
        }
    };

    /* Read flags */
    let flags: u32 = match sc.read_uint() {
        Some(v) => v,
        None => {
            eprintln!("Error reading flags");
            return 1;
        }
    };

    /* Read input length */
    let input_len: usize = match sc.read_usize() {
        Some(v) => v,
        None => {
            eprintln!("Error reading input length");
            return 1;
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        eprintln!(
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    /* Read input buffer data */
    for i in 0..input_len {
        let byte: u32 = match sc.read_uint() {
            Some(v) => v,
            None => {
                eprintln!("Error reading input byte {}", i);
                return 1;
            }
        };
        input_buffer[i] = byte as u8;
    }

    /* Read reference length */
    let ref_len: usize = match sc.read_usize() {
        Some(v) => v,
        None => {
            eprintln!("Error reading reference length");
            return 1;
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        eprintln!(
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    /* Read reference buffer data */
    for i in 0..ref_len {
        let byte: u32 = match sc.read_uint() {
            Some(v) => v,
            None => {
                eprintln!("Error reading reference byte {}", i);
                return 1;
            }
        };
        ref_buffer[i] = byte as u8;
    }

    /* Call the library function */
    let result = strcpy_fun::process_strings(
        Some(&input_buffer),
        input_len,
        Some(&ref_buffer),
        ref_len,
        operation,
        flags,
    );

    /* Print result to stdout */
    println!("{}", result);

    0
}

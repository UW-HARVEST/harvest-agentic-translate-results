//! Translation of `c_src/src/main.c`.
//!
//! The C `main()` keeps two *uninitialised* 1024 byte buffers on its stack and
//! fills only the first `input_len` / `ref_len` bytes of them from stdin.  The
//! library it calls happily runs `strlen`/`strcmp` over those buffers, so it
//! reads the leftover stack contents whenever the data it was given is not NUL
//! terminated (which is exactly the bug the original code advertises in its
//! comments).
//!
//! To reproduce that observable behaviour the whole relevant part of the C
//! stack frame is modelled as one contiguous byte array, laid out exactly the
//! way `gcc` lays out `main()` on x86-64:
//!
//! ```text
//!   offset      C object                 address in the C program
//!   ---------------------------------------------------------------
//!   0    ..1024 ref_buffer[1024]         %rbp-0x830 .. %rbp-0x431
//!   1024 ..2048 input_buffer[1024]       %rbp-0x430 .. %rbp-0x031
//!   2048 ..2056 ref_len                  %rbp-0x30
//!   2056 ..2064 input_len                %rbp-0x28
//!   2064 ..2068 (padding, uninitialised) %rbp-0x20
//!   2068 ..2072 flags                    %rbp-0x1c
//!   2072 ..2076 operation                %rbp-0x18
//!   2076 ..2080 result (uninitialised)   %rbp-0x14
//!   2080 ..2088 i of the reference loop  %rbp-0x10
//!   2088 ..2096 i of the input loop      %rbp-0x08
//!   2096 ..     saved %rbp, return address, ...
//! ```
//!
//! Two consequences of that layout are directly observable and are therefore
//! reproduced here:
//!
//! * `ref_buffer` is immediately followed by `input_buffer`, so a `strcmp()`
//!   walking off the end of the reference data continues inside the input data;
//! * `input_buffer` is immediately followed by `main()`'s locals, so a
//!   `strcmp()` walking off the end of the input data reads the little-endian
//!   bytes of `ref_len` (which is why a completely filled input buffer behaves
//!   like a string whose length depends on the reference length).
//!
//! The bytes of the buffers that the C program never writes hold the stack
//! residue left behind by the dynamic loader; it was captured from the real
//! program and lives in [`residue`].

mod cstr;
mod residue;
mod scanf;
mod strcpy_fun;

use std::io::{self, Write};

const MAX_BUFFER_SIZE: usize = 1024;

/// Offset of `ref_buffer` inside the modelled frame (`%rbp-0x830`).
const REF_OFF: usize = 0;
/// Offset of `input_buffer` inside the modelled frame (`%rbp-0x430`).
const INPUT_OFF: usize = 1024;
/// Offset of `main()`'s locals inside the modelled frame (`%rbp-0x30`).
const LOCALS_OFF: usize = 2048;

/// The modelled `main()` stack frame, pre-filled with the captured residue.
fn new_frame() -> Vec<u8> {
    let mut frame = Vec::with_capacity(LOCALS_OFF + residue::FRAME_TAIL_RESIDUE.len());
    frame.extend_from_slice(&residue::REF_RESIDUE);
    frame.extend_from_slice(&residue::INPUT_RESIDUE);
    frame.extend_from_slice(&residue::FRAME_TAIL_RESIDUE);
    frame
}

/// Store a `size_t` local into the modelled frame (little endian).
fn store_u64(frame: &mut [u8], off: usize, value: u64) {
    frame[off..off + 8].copy_from_slice(&value.to_le_bytes());
}

/// Store a 32 bit local into the modelled frame (little endian).
fn store_u32(frame: &mut [u8], off: usize, value: u32) {
    frame[off..off + 4].copy_from_slice(&value.to_le_bytes());
}

fn main() {
    let code = run();
    let _ = io::stdout().flush();
    std::process::exit(code);
}

fn run() -> i32 {
    let stdin = io::stdin();
    let mut sc = scanf::Scanner::new(stdin.lock());

    let mut frame = new_frame();

    /* Read operation */
    let operation: i32 = match sc.read_int() {
        Some(v) => v,
        None => {
            eprint!("Error reading operation\n");
            return 1;
        }
    };

    /* Read flags */
    let flags: u32 = match sc.read_uint() {
        Some(v) => v,
        None => {
            eprint!("Error reading flags\n");
            return 1;
        }
    };

    /* Read input length */
    let input_len: usize = match sc.read_usize() {
        Some(v) => v,
        None => {
            eprint!("Error reading input length\n");
            return 1;
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    /* Read input buffer data */
    for i in 0..input_len {
        let byte: u32 = match sc.read_uint() {
            Some(v) => v,
            None => {
                eprint!("Error reading input byte {}\n", i);
                return 1;
            }
        };
        frame[INPUT_OFF + i] = byte as u8;
    }

    /* Read reference length */
    let ref_len: usize = match sc.read_usize() {
        Some(v) => v,
        None => {
            eprint!("Error reading reference length\n");
            return 1;
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    /* Read reference buffer data */
    for i in 0..ref_len {
        let byte: u32 = match sc.read_uint() {
            Some(v) => v,
            None => {
                eprint!("Error reading reference byte {}\n", i);
                return 1;
            }
        };
        frame[REF_OFF + i] = byte as u8;
    }

    /* Publish the values of main()'s locals: they sit right behind
     * `input_buffer` and are read by the library whenever a `strcmp()` runs off
     * the end of a completely unterminated input buffer. */
    store_u64(&mut frame, LOCALS_OFF, ref_len as u64);
    store_u64(&mut frame, LOCALS_OFF + 0x08, input_len as u64);
    store_u32(&mut frame, LOCALS_OFF + 0x14, flags);
    store_u32(&mut frame, LOCALS_OFF + 0x18, operation as u32);
    store_u64(&mut frame, LOCALS_OFF + 0x20, ref_len as u64);
    store_u64(&mut frame, LOCALS_OFF + 0x28, input_len as u64);

    /* Call the library function */
    let result = strcpy_fun::process_strings(
        Some(&frame[INPUT_OFF..]),
        input_len,
        Some(&frame[REF_OFF..]),
        ref_len,
        operation,
        flags,
    );

    /* Print result to stdout */
    print!("{}\n", result);

    0
}

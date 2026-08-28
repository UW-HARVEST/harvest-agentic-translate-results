/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Faithful Rust translation of `c_src/src/main.c`.

use std::io::{Read, Write};
use std::process::ExitCode;

use driver::{process_buffer_tracked, Buffer};

mod scan;

use scan::Scanner;

/*
 * `main.c` declares `uint8_t buffer[256]`, but `compact_runs()` can grow the
 * logical length up to `2 * 256 == 512`, so the original program writes past
 * the end of that array.  A larger backing region is used here so the very same
 * sequence of byte moves can be replayed without tripping Rust's bounds checks;
 * the stray writes are then reinterpreted through the stack-frame model below,
 * which is what makes them observable in exactly the same way as in C.
 */
const BUFFER_CAPACITY: usize = 1024;

/*
 * Layout of `main`'s stack frame as emitted by the CMake build (gcc -O0,
 * x86-64).  `buffer` lives at `rbp-0x130`, and the remaining locals sit at
 * *higher* addresses, i.e. immediately after the array:
 *
 *   buffer + 256 .. 264   rbp-0x30   size_t length     (dead after the call)
 *   buffer + 268 .. 272   rbp-0x24   int    param2     (dead after the call)
 *   buffer + 272 .. 276   rbp-0x20   int    param1     (dead after the call)
 *   buffer + 276 .. 280   rbp-0x1c   uint32 flags      (dead after the call)
 *   buffer + 280 .. 288   rbp-0x18   size_t new_length (WRITTEN after the call)
 *   buffer + 288 .. 296   rbp-0x10   size_t i          (print-loop counter)
 *   buffer + 296 .. 304   rbp-0x08   size_t i          (read-loop, dead)
 *   buffer + 304 .. 312   rbp+0x00   saved rbp         (harmless: `leave` only)
 *   buffer + 312 .. 320   rbp+0x08   return address    (corruption => SIGSEGV)
 *
 * Consequences reproduced here:
 *   * bytes 280..288 read back as the little-endian image of `new_length`,
 *     because `main` stores the return value there after `process_buffer()`;
 *   * bytes 288..296 read back as the little-endian image of the print loop's
 *     own counter, which is being incremented while those bytes are printed;
 *   * a write that reaches index 312 or beyond clobbers the return address, so
 *     `main` faults on `ret`.  stdout is a fully buffered pipe holding at most
 *     ~2 KiB here, so nothing has been flushed and both streams stay empty.
 */
const NEW_LENGTH_OFFSET: usize = 280;
const PRINT_INDEX_OFFSET: usize = 288;
const RETURN_ADDRESS_OFFSET: usize = 312;

const SIGSEGV: i32 = 11;
const SIG_DFL: usize = 0;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
}

/// Reproduce `main` faulting on a clobbered return address: die by SIGSEGV
/// without having flushed anything to stdout.
fn die_by_segv() -> ! {
    unsafe {
        /* Rust's runtime installs a SIGSEGV handler for stack-overflow
         * reporting; restore the default disposition so the signal is fatal
         * and the wait status is "killed by SIGSEGV", as it is for the C. */
        signal(SIGSEGV, SIG_DFL);
        raise(SIGSEGV);
    }
    /* `raise` above does not return; keep the function diverging regardless. */
    std::process::abort();
}

fn main() -> ExitCode {
    let mut stdin_data = Vec::new();
    /* Reading the whole stream up-front matches `scanf`'s buffered, whitespace
     * skipping behaviour: conversions may span newlines freely. */
    let _ = std::io::stdin().read_to_end(&mut stdin_data);
    let mut sc = Scanner::new(stdin_data);

    let flags: u32;
    let param1: i32;
    let param2: i32;
    let length: usize;
    let mut backing = vec![0u8; BUFFER_CAPACITY];

    /* Read flags */
    match sc.scan_u32() {
        Some(v) => flags = v,
        None => {
            eprint!("Error reading flags\n");
            return ExitCode::from(1);
        }
    }

    /* Read param1 */
    match sc.scan_i32() {
        Some(v) => param1 = v,
        None => {
            eprint!("Error reading param1\n");
            return ExitCode::from(1);
        }
    }

    /* Read param2 */
    match sc.scan_i32() {
        Some(v) => param2 = v,
        None => {
            eprint!("Error reading param2\n");
            return ExitCode::from(1);
        }
    }

    /* Read buffer length */
    match sc.scan_usize() {
        Some(v) => length = v,
        None => {
            eprint!("Error reading length\n");
            return ExitCode::from(1);
        }
    }

    if length > 256 {
        eprint!("Error: length {} exceeds maximum 256\n", length);
        return ExitCode::from(1);
    }

    /* Read buffer data */
    {
        let mut buffer = Buffer::new(&mut backing);
        for i in 0..length {
            match sc.scan_u32() {
                Some(byte) => buffer.set(i, byte as u8),
                None => {
                    eprint!("Error reading byte {}\n", i);
                    return ExitCode::from(1);
                }
            }
        }
    }

    /* Process the buffer */
    let mut buffer = Buffer::new(&mut backing);
    let new_length = process_buffer_tracked(&mut buffer, length, flags, param1, param2);

    /* Any write at or past the return address slot makes `main` fault on
     * `ret`, after the (still buffered, therefore lost) output was produced. */
    if let Some(max_written) = buffer.max_written() {
        if max_written >= RETURN_ADDRESS_OFFSET {
            die_by_segv();
        }
    }

    let new_length_bytes = (new_length as u64).to_le_bytes();

    /* Output new length */
    let mut out = String::new();
    out.push_str(&new_length.to_string());

    /* Output buffer contents */
    for i in 0..new_length {
        let byte = if (NEW_LENGTH_OFFSET..NEW_LENGTH_OFFSET + 8).contains(&i) {
            /* Aliases `new_length`, stored by `main` before printing. */
            new_length_bytes[i - NEW_LENGTH_OFFSET]
        } else if (PRINT_INDEX_OFFSET..PRINT_INDEX_OFFSET + 8).contains(&i) {
            /* Aliases the print loop counter, whose value right now is `i`. */
            (i as u64).to_le_bytes()[i - PRINT_INDEX_OFFSET]
        } else {
            buffer.get(i)
        };
        out.push(' ');
        out.push_str(&byte.to_string());
    }
    out.push('\n');

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();

    ExitCode::from(0)
}

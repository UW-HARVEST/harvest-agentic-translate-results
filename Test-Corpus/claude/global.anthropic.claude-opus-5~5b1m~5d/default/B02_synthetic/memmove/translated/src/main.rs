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

use driver::process_buffer;

mod scan;

use scan::Scanner;

/*
 * The C program declares `uint8_t buffer[256]`, but `compact_runs()` can grow
 * the logical length past 256 (a latent overflow in the original code).  A
 * larger backing region is used here so that the very same sequence of byte
 * moves can be replayed without stepping outside of the allocation, while the
 * observable behaviour for all in-range inputs stays identical.
 */
const BUFFER_CAPACITY: usize = 4096;

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
    let mut buffer = vec![0u8; BUFFER_CAPACITY];

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
    for i in 0..length {
        match sc.scan_u32() {
            Some(byte) => buffer[i] = byte as u8,
            None => {
                eprint!("Error reading byte {}\n", i);
                return ExitCode::from(1);
            }
        }
    }

    /* Process the buffer */
    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    /* Output new length */
    let mut out = String::new();
    out.push_str(&new_length.to_string());

    /* Output buffer contents */
    for i in 0..new_length {
        out.push(' ');
        out.push_str(&buffer[i].to_string());
    }
    out.push('\n');

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();

    ExitCode::from(0)
}

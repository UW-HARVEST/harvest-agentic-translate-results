// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

mod libc_compat;

use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

use libc_compat::{strtoul_base10, GlibcRand};

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

const UINT_MAX: u64 = u32::MAX as u64;

/// Perform expensive arithmetic on each element.
///
/// The C original relies on signed integer overflow (which GCC implements as
/// two's-complement wrap-around), so every arithmetic operation here is an
/// explicit wrapping one.
fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x: i32 = *slot;
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(((x as u32) << 1) as i32);
            x = (x / 2).wrapping_add(x % 7);
        }
        *slot = x;
    }
}

fn run() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args.len();

    if argc != 2 {
        let program = args
            .first()
            .map(|a| a.as_bytes().to_vec())
            .unwrap_or_default();
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        let _ = stderr.write_all(b"Usage: ");
        let _ = stderr.write_all(&program);
        let _ = stderr.write_all(b" <seed>\n");
        let _ = stderr.flush();
        return ExitCode::from(1);
    }

    let arg = args[1].as_bytes();

    // errno = 0; strtoul(argv[1], &endptr, 10);
    let parsed = strtoul_base10(arg);
    let temp_seed = parsed.value;
    // `*endptr != '\0'` is only false when the whole string was consumed.
    if parsed.end_index != arg.len() || parsed.range_error || temp_seed > UINT_MAX {
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        let _ = stderr.write_all(b"Invalid seed: '");
        let _ = stderr.write_all(arg);
        let _ = stderr.write_all(b"'\n");
        let _ = stderr.flush();
        return ExitCode::from(1);
    }

    let seed = temp_seed as u32;
    let mut rng = GlibcRand::new(seed);

    // Global array, zero initialised just like the C one living in .bss.
    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];

    for slot in array.iter_mut() {
        *slot = rng.next();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &value in array.iter() {
        xor_result ^= value;
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let _ = write!(stdout, "{}\n", xor_result);
    let _ = stdout.flush();

    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    run()
}

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

use std::io::{Read, Write};

/// Size of the C `char in[1000]` buffer.
const BUF_LEN: usize = 1000;

/// Mirrors the C:
///
/// ```c
/// int foo(const char *in, char c) {
///     int res = 0;
///     for (const char *s = in; s = strchr(s, c); s++) { res++; }
///     return res;
/// }
/// ```
///
/// `strchr` scans the NUL-terminated string, so `in` is the bytes of the
/// buffer up to (not including) the first NUL byte. Counting is done with a
/// wrapping add so overflow behaves like the C `int` increment (unreachable in
/// practice: the buffer holds at most 1000 bytes).
fn foo(in_str: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    let mut idx = 0usize;
    while idx <= in_str.len() {
        // strchr(s, c): search from `s` through the terminating NUL.
        // A NUL search byte would match the terminator itself; `c` is never
        // NUL here, so only the real bytes can match.
        match in_str[idx..].iter().position(|&b| b == c) {
            Some(off) => {
                res = res.wrapping_add(1);
                // s++ past the found character.
                idx = idx + off + 1;
            }
            None => break,
        }
    }
    res
}

/// Mirrors the C `driver`, including printf formatting.
fn driver<W: Write>(out: &mut W, in_str: &[u8]) {
    let _ = write!(out, "A: {}\n", foo(in_str, b'A'));
    let _ = write!(out, "x: {}\n", foo(in_str, b'x'));
}

/// Restore the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a failed
/// write to a pipe whose reader is gone merely returns `EPIPE` and the process
/// exits 0. A C program inherits the default disposition and is *killed* by
/// `SIGPIPE` instead, which shows up as a different exit status. Restoring
/// `SIG_DFL` makes this program die exactly the way `c_src/build/driver` does.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal` is the POSIX libc function; SIG_DFL is a valid handler
    // value and SIGPIPE is a valid signal number.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    // char in[1000] = "";  -> fully zero-initialized
    let mut buf = [0u8; BUF_LEN];

    // fread(in, 1, sizeof(in), stdin): read up to 1000 bytes, looping until
    // the buffer is full or EOF/error. Newlines are not delimiters.
    let mut stdin = std::io::stdin();
    let mut filled = 0usize;
    while filled < BUF_LEN {
        match stdin.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // The C code then treats `in` as a NUL-terminated string. If fewer than
    // 1000 bytes were read the remaining zero fill terminates it; if the input
    // itself contains a NUL byte the string ends there.
    //
    // If all 1000 bytes were filled and none of them is NUL, the C `strchr`
    // walks off the end of the buffer into uninitialized stack padding and the
    // saved frame pointer. That is undefined behaviour whose result the input
    // does not determine (see ERRORS.md); this program produces the in-bounds
    // answer, which is what the C program prints on the overwhelming majority
    // of runs.
    let end = buf[..filled]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(filled);
    let in_str = &buf[..end];

    // C's stdout is fully buffered when it is not a terminal, so both lines
    // leave the process together rather than one write per line.
    let mut out = Vec::with_capacity(32);
    driver(&mut out, in_str);

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(&out);
    let _ = lock.flush();
}

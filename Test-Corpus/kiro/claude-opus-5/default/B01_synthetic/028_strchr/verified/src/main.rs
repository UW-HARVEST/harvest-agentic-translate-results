// Rust translation of c_src/src/main.c
//
// Original C:
//   int foo(const char *in, char c) {
//       int res = 0;
//       for (const char *s = in; s = strchr(s, c); s++) { res++; }
//       return res;
//   }
//   void driver(const char *in) {
//       printf("A: %d\n", foo(in, 'A'));
//       printf("x: %d\n", foo(in, 'x'));
//   }
//   int main() {
//       char in[1000] = "";
//       fread(in, 1, sizeof(in), stdin);
//       driver(in);
//       return 0;
//   }
//
// Behavior preserved exactly, including:
//   * `fread` reads raw bytes (newlines are not special) up to 1000 bytes.
//   * The buffer is zero-initialized, so the effective C string ends at the
//     first NUL byte, which means any NUL byte present in the input truncates
//     what `foo` sees.
//   * `foo` counts occurrences of the target byte in that C string.

use std::io::{self, Read, Write};

const BUF_LEN: usize = 1000;

/// The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs,
/// which a C program does not. Without this, writing to a closed pipe makes the
/// C program die from `SIGPIPE` (exit status "killed by signal 13") while the
/// Rust program would quietly ignore the write error and exit 0. Restore the
/// default disposition so the process-level behavior matches the C.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // `signal(2)`; SIGPIPE is 13 and SIG_DFL is 0 on every Unix target.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// Equivalent of the C `foo`: counts occurrences of `c` in the NUL-terminated
/// string `in_`. `in_` is the buffer contents; the logical string stops at the
/// first NUL byte (mirroring `strchr` walking a C string).
fn foo(in_: &[u8], c: u8) -> i32 {
    let mut res: i32 = 0;
    for &b in in_ {
        if b == 0 {
            break;
        }
        if b == c {
            res += 1;
        }
    }
    res
}

fn driver<W: Write>(out: &mut W, in_: &[u8]) {
    let _ = write!(out, "A: {}\n", foo(in_, b'A'));
    let _ = write!(out, "x: {}\n", foo(in_, b'x'));
}

/// Emulates `fread(in, 1, sizeof(in), stdin)` into a zero-initialized buffer:
/// keeps reading until the buffer is full or the stream ends.
fn fread_stdin(buf: &mut [u8]) {
    let mut stdin = io::stdin();
    let mut filled = 0usize;
    while filled < buf.len() {
        match stdin.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

fn main() {
    restore_default_sigpipe();

    // char in[1000] = "";  -> zero-initialized
    let mut in_ = [0u8; BUF_LEN];
    fread_stdin(&mut in_);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, &in_);
    let _ = out.flush();
}

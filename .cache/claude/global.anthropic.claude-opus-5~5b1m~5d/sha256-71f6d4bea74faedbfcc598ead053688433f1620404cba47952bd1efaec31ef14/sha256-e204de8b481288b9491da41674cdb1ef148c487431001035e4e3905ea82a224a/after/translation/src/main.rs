// Translation of c_src/src/main.c
//
// Copyright 2025 MIT Lincoln Laboratory  (see c_src for the full license text)

mod engine;
mod impl_a;
mod impl_b;
mod lib_target;
mod util;

use std::ffi::OsStr;
use std::io::{Read, Write};

use engine::run_engine;
use util::{vm_print, IntVec, Vm};

#[cfg(unix)]
fn os_bytes(s: &OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes().to_vec()
}

#[cfg(not(unix))]
fn os_bytes(s: &OsStr) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, but a C
/// program starts with the default disposition.  main.c therefore dies from
/// `SIGPIPE` when its stdout is a pipe whose reader has gone away (e.g.
/// `driver 0 1 7 300000 3 | head -c 4`), while an unpatched Rust build would
/// swallow the `EPIPE` and exit 0.  Restore the C behaviour so the exit status
/// matches.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal` with SIG_DFL for SIGPIPE is async-signal-safe and is
    // exactly the disposition the C program runs with.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// `isspace` for the "C" locale, as used by `strtol` when skipping leading
/// whitespace.
fn is_c_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r'
}

/// Emulates `long t = strtol(s, &e, 10);` followed by the `e && *e=='\0'`
/// test used everywhere in main.c, and the subsequent `(int)t` narrowing.
///
/// Returns `Some((int)t)` when `strtol` consumed the whole NUL-terminated
/// string, `None` otherwise.  An empty string yields `Some(0)` because
/// `strtol` leaves `e == s` which already points at the terminator.
fn strtol10_full(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }
    if i == digits_start {
        // No conversion performed: strtol sets e = s.
        return if s.is_empty() { Some(0) } else { None };
    }
    if i != s.len() {
        // *e != '\0'
        return None;
    }
    let t: i64 = if overflow {
        // strtol saturates at LONG_MAX / LONG_MIN (long is 64-bit here).
        if neg {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if neg {
        acc.wrapping_neg()
    } else {
        acc
    };
    // (int)t
    Some(t as i32)
}

/// `static void usage(const char *p)`
fn usage(p: &[u8]) {
    let stderr = std::io::stderr();
    let mut h = stderr.lock();
    let _ = h.write_all(b"Usage: ");
    let _ = h.write_all(p);
    let _ = h.write_all(
        b" [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
    );
    let _ = h.flush();
}

const FGETS_BUF: usize = 4096;

/// `static size_t read_stdin(IntVec *v)`
///
/// `fgets(buf, 4096, stdin)` stops after a newline or after 4095 bytes, so a
/// very long line is handed to the tokenizer in pieces (which can split a
/// number in half).  The tokenizer treats `buf` as a C string, so anything at
/// or after an embedded NUL byte in a chunk is ignored.
fn read_stdin(v: &mut IntVec) -> usize {
    let mut data = Vec::new();
    {
        let stdin = std::io::stdin();
        let mut h = stdin.lock();
        let _ = h.read_to_end(&mut data);
    }

    let mut count = 0usize;
    let mut pos = 0usize;
    while pos < data.len() {
        // One `fgets` call worth of input.
        let hard_end = std::cmp::min(pos + (FGETS_BUF - 1), data.len());
        let end = match data[pos..hard_end].iter().position(|&c| c == b'\n') {
            Some(k) => pos + k + 1,
            None => hard_end,
        };
        let mut chunk = &data[pos..end];
        pos = end;

        // The chunk is used as a NUL-terminated string.
        if let Some(nul) = chunk.iter().position(|&c| c == 0) {
            chunk = &chunk[..nul];
        }

        let mut i = 0usize;
        while i < chunk.len() {
            let mut j = i;
            while j < chunk.len()
                && chunk[j] != b' '
                && chunk[j] != b'\t'
                && chunk[j] != b'\n'
                && chunk[j] != b'\r'
            {
                j += 1;
            }
            if j > i {
                if let Some(t) = strtol10_full(&chunk[i..j]) {
                    v.push(t);
                    count += 1;
                }
            }
            // p = (*q ? q+1 : q)
            i = j + 1;
        }
    }
    count
}

fn main() {
    restore_default_sigpipe();

    let argv: Vec<Vec<u8>> = std::env::args_os().map(|a| os_bytes(&a)).collect();
    let arg0: &[u8] = match argv.first() {
        Some(a) => a,
        None => b"",
    };

    let mut use_stdin = false;
    let mut code = IntVec::new();

    for i in 1..argv.len() {
        let a: &[u8] = &argv[i];
        if a == b"--help" {
            usage(arg0);
            code.free();
            std::process::exit(0);
        } else if a == b"--stdin" {
            use_stdin = true;
        } else {
            match strtol10_full(a) {
                Some(t) => {
                    code.push(t);
                }
                None => {
                    let stderr = std::io::stderr();
                    let mut h = stderr.lock();
                    let _ = h.write_all(b"skip '");
                    let _ = h.write_all(a);
                    let _ = h.write_all(b"'\n");
                    let _ = h.flush();
                }
            }
        }
    }
    if use_stdin {
        read_stdin(&mut code);
    }
    if code.len() == 0 {
        let _ = std::io::stderr().write_all(b"no program\n");
        code.free();
        std::process::exit(2);
    }

    let mut vm_a = Vm::new();
    let mut vm_b = Vm::new();
    let mut vm_e = Vm::new();

    let rc_a = run_engine(0, &code.data, &mut vm_a);
    let rc_b = run_engine(1, &code.data, &mut vm_b);
    let rc_e = run_engine(2, &code.data, &mut vm_e);

    {
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        let _ = write!(out, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e);
        vm_print(&mut out, "A:", &vm_a);
        vm_print(&mut out, "B:", &vm_b);
        vm_print(&mut out, "EXT:", &vm_e);
        let _ = out.flush();
    }

    vm_a.free();
    vm_b.free();
    vm_e.free();
    code.free();
    std::process::exit(0);
}

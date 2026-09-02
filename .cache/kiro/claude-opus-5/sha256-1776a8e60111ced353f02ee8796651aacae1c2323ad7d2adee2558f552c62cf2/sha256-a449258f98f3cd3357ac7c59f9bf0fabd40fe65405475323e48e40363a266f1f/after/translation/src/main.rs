// Translation of c_src/src/main.c

mod a;
mod b;
mod engine;
mod libtarget;
mod util;

use std::io::{BufReader, Read, Write};
use std::os::unix::ffi::OsStrExt;

use engine::run_engine;
use util::{vm_print, Vm};

/// Emulates `strtol(nptr, &endptr, 10)`.
///
/// Returns `(value, end_offset)` where `end_offset` is the index that C's
/// `endptr` would point at. Per the C standard, when no conversion is performed
/// `endptr` is set back to `nptr`, i.e. offset 0. Callers test `*endptr == '\0'`,
/// which corresponds to `end_offset == s.len()` here -- note that for an empty
/// input this is trivially true, so `""` parses "successfully" as 0.
fn strtol10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    // C locale isspace(): space, \t, \n, \v, \f, \r
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
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
        // No conversion performed: endptr == nptr.
        return (0, 0);
    }
    let val = if overflow {
        // ERANGE: clamped to LONG_MAX / LONG_MIN.
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
    (val, i)
}

/// C: `fprintf(stderr, "Usage: %s [--stdin] [bytecodes...]\n"
///                     "Bytecodes are integers forming a small VM program.\n", p);`
fn usage(prog: &[u8]) {
    let err = std::io::stderr();
    let mut err = err.lock();
    let _ = err.write_all(b"Usage: ");
    let _ = err.write_all(prog);
    let _ = err.write_all(b" [--stdin] [bytecodes...]\n");
    let _ = err.write_all(b"Bytecodes are integers forming a small VM program.\n");
    let _ = err.flush();
}

/// Emulates `fgets(buf, cap, stdin)`.
///
/// Reads at most `cap - 1` bytes, stopping after a newline (which is retained).
/// Returns `None` at EOF with nothing read. Unlike a line-oriented read, an
/// over-long line is split into multiple chunks -- which can split a number in
/// half. That behavior is preserved.
fn fgets<R: Read>(r: &mut R, cap: usize) -> Option<Vec<u8>> {
    let max = cap - 1;
    let mut out: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while out.len() < max {
        match r.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// C: `static size_t read_stdin(IntVec *v)`
///
/// The returned count is discarded by the caller, so it is not modeled.
fn read_stdin(v: &mut Vec<i32>) {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    // C: `char buf[4096]; while (fgets(buf, sizeof buf, stdin)) { ... }`
    while let Some(chunk) = fgets(&mut reader, 4096) {
        // The C code walks `buf` as a NUL-terminated string, so any embedded
        // NUL byte ends processing of this chunk early.
        let s: &[u8] = match chunk.iter().position(|&c| c == 0) {
            Some(idx) => &chunk[..idx],
            None => &chunk[..],
        };

        let mut p = 0usize;
        while p < s.len() {
            let mut q = p;
            while q < s.len() && !matches!(s[q], b' ' | b'\t' | b'\n' | b'\r') {
                q += 1;
            }
            // C temporarily writes '\0' at q; an empty token (two adjacent
            // delimiters) fails the `if (*p)` test and is skipped.
            let tok = &s[p..q];
            if !tok.is_empty() {
                let (t, end) = strtol10(tok);
                if end == tok.len() {
                    v.push(t as i32);
                }
            }
            p = if q < s.len() { q + 1 } else { q };
        }
    }
}

/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a write to
/// a closed pipe returns `EPIPE` (which every write site here ignores) and the
/// process still exits 0. The C program inherits the default disposition and is
/// killed by `SIGPIPE` instead, exiting 141 under a shell. Restore `SIG_DFL` so
/// the observable exit status matches.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|arg| arg.as_os_str().as_bytes().to_vec())
        .collect();
    let argv0: &[u8] = match argv.first() {
        Some(a) => a,
        None => b"",
    };

    let mut use_stdin = false;
    // C: `IntVec code; iv_init(&code);`
    let mut code: Vec<i32> = Vec::new();

    for arg in argv.iter().skip(1) {
        if arg.as_slice() == b"--help" {
            usage(argv0);
            std::process::exit(0);
        } else if arg.as_slice() == b"--stdin" {
            use_stdin = true;
        } else {
            let (t, end) = strtol10(arg);
            if end == arg.len() {
                code.push(t as i32);
            } else {
                let err = std::io::stderr();
                let mut err = err.lock();
                let _ = err.write_all(b"skip '");
                let _ = err.write_all(arg);
                let _ = err.write_all(b"'\n");
                let _ = err.flush();
            }
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.is_empty() {
        let err = std::io::stderr();
        let mut err = err.lock();
        let _ = err.write_all(b"no program\n");
        let _ = err.flush();
        std::process::exit(2);
    }

    let mut vm_a = Vm::new();
    let mut vm_b = Vm::new();
    let mut vm_e = Vm::new();

    let n = code.len();
    let rc_a = run_engine(0, &code, n, &mut vm_a);
    let rc_b = run_engine(1, &code, n, &mut vm_b);
    let rc_e = run_engine(2, &code, n, &mut vm_e);

    let out = std::io::stdout();
    let mut out = out.lock();
    let _ = write!(out, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e);
    vm_print(&mut out, "A:", &vm_a);
    vm_print(&mut out, "B:", &vm_b);
    vm_print(&mut out, "EXT:", &vm_e);
    let _ = out.flush();

    std::process::exit(0);
}

//! Port of `c_src/src/main.c`.

mod a;
mod b;
mod cstd;
mod engine;
mod lib_target;
mod util;

use std::io::{self, BufWriter, Write};
use std::os::unix::ffi::OsStrExt;

use cstd::{c_fgets, c_strtol};
use engine::run_engine;
use util::{vm_print, Vm};

/// stderr is unbuffered in C, so write straight through.
fn err_write(bytes: &[u8]) {
    let mut e = io::stderr();
    let _ = e.write_all(bytes);
    let _ = e.flush();
}

/// `usage`
fn usage(prog: &[u8]) {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"Usage: ");
    out.extend_from_slice(prog);
    out.extend_from_slice(b" [--stdin] [bytecodes...]\n");
    out.extend_from_slice(b"Bytecodes are integers forming a small VM program.\n");
    err_write(&out);
}

/// `read_stdin`
///
/// Reads with `fgets` into a 4096-byte buffer, so a token straddling a buffer
/// boundary is split in two -- that behaviour is preserved. The tokenizer walks
/// the C string, so an embedded NUL byte silently discards the rest of the
/// buffered chunk.
fn read_stdin(v: &mut Vec<i32>) -> usize {
    let stdin = io::stdin();
    let mut r = stdin.lock();
    let mut count: usize = 0;

    while let Some(raw) = c_fgets(&mut r, 4096) {
        let end = raw.iter().position(|&x| x == 0).unwrap_or(raw.len());
        let buf = &raw[..end];

        let mut p = 0usize;
        while p < buf.len() {
            let mut q = p;
            while q < buf.len()
                && buf[q] != b' '
                && buf[q] != b'\t'
                && buf[q] != b'\n'
                && buf[q] != b'\r'
            {
                q += 1;
            }
            // The token is buf[p..q], NUL-terminated in the C original.
            if q > p {
                let tok = &buf[p..q];
                let (t, e) = c_strtol(tok);
                if e == tok.len() {
                    v.push(t as i32);
                    count += 1;
                }
            }
            p = if q < buf.len() { q + 1 } else { q };
        }
    }
    count
}

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    let argv0: &[u8] = if argv.is_empty() { b"driver" } else { &argv[0] };

    let mut use_stdin = false;
    let mut code: Vec<i32> = Vec::new();

    for i in 1..argv.len() {
        let arg = &argv[i];
        if arg.as_slice() == b"--help" {
            usage(argv0);
            std::process::exit(0);
        } else if arg.as_slice() == b"--stdin" {
            use_stdin = true;
        } else {
            // An empty argument parses as 0 here: strtol performs no
            // conversion, so endptr == nptr, which already points at the NUL.
            let (t, e) = c_strtol(arg);
            if e == arg.len() {
                code.push(t as i32);
            } else {
                let mut msg: Vec<u8> = Vec::new();
                msg.extend_from_slice(b"skip '");
                msg.extend_from_slice(arg);
                msg.extend_from_slice(b"'\n");
                err_write(&msg);
            }
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.is_empty() {
        err_write(b"no program\n");
        std::process::exit(2);
    }

    let mut vm_a = Vm::new();
    let mut vm_b = Vm::new();
    let mut vm_e = Vm::new();

    // Order matters: a.c and b.c keep static state across these three runs.
    let rc_a = run_engine(0, &code, &mut vm_a);
    let rc_b = run_engine(1, &code, &mut vm_b);
    let rc_e = run_engine(2, &code, &mut vm_e);

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let _ = write!(out, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e);
    vm_print(&mut out, "A:", &vm_a);
    vm_print(&mut out, "B:", &vm_b);
    vm_print(&mut out, "EXT:", &vm_e);
    let _ = out.flush();
}

// Translation of main.c
// Top-level driver. Mirrors the C program's stdin/argv reading and printing.

mod a;
mod b;
mod engine;
mod lib_target;
mod util;

use std::io::{self, Read, Write};

use crate::engine::run_engine;
use crate::util::{vm_print, IntVec, VM};

fn usage<W: Write>(fp: &mut W, prog: &str) {
    let _ = writeln!(
        fp,
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.",
        prog
    );
}

/// Parse a single token using strtol(token, &e, 10) semantics — accept the
/// token only if the entire string converts. Returns None if invalid.
fn strtol_full(s: &str) -> Option<i32> {
    if s.is_empty() {
        return None;
    }
    // strtol parses a long; cast to int truncates. We replicate that.
    // Accept optional leading '+' or '-' and digits only. strtol also accepts
    // leading whitespace, but tokens here have already been stripped of it.
    let bytes = s.as_bytes();
    let mut i = 0;
    let _neg = match bytes[0] {
        b'+' => {
            i = 1;
            false
        }
        b'-' => {
            i = 1;
            true
        }
        _ => false,
    };
    if i == bytes.len() {
        // Just "+" or "-" is not a valid number; strtol leaves *endptr = s.
        return None;
    }
    for &c in &bytes[i..] {
        if !c.is_ascii_digit() {
            return None;
        }
    }
    // Parse as i64 (mirroring 'long' on a 64-bit system) and truncate to i32.
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => {
            // Out-of-range. C strtol clamps to LONG_MAX/LONG_MIN; cast to int
            // gives -1 / 0. For typical inputs this branch is unused.
            if s.starts_with('-') {
                Some(i64::MIN as i32)
            } else {
                Some(i64::MAX as i32)
            }
        }
    }
}

fn read_stdin(v: &mut IntVec) -> usize {
    // C reads via fgets in a 4096-byte buffer and tokenizes each chunk by
    // ' '/'\t'/'\n'/'\r'. For typical inputs (no token longer than the
    // buffer) this is equivalent to splitting all of stdin on those chars.
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return 0;
    }
    let mut count = 0usize;
    for token in buf.split(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r') {
        if token.is_empty() {
            continue;
        }
        if let Some(n) = strtol_full(token) {
            v.push(n);
            count += 1;
        }
    }
    count
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(|s| s.as_str()).unwrap_or("driver");

    let mut use_stdin = false;
    let mut code = IntVec::new();

    for arg in args.iter().skip(1) {
        if arg == "--help" {
            usage(&mut io::stderr(), prog);
            std::process::exit(0);
        } else if arg == "--stdin" {
            use_stdin = true;
        } else {
            match strtol_full(arg) {
                Some(n) => {
                    code.push(n);
                }
                None => {
                    let _ = writeln!(io::stderr(), "skip '{}'", arg);
                }
            }
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.len() == 0 {
        let _ = writeln!(io::stderr(), "no program");
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, &code.data, code.data.len(), &mut vm_a);
    let rc_b = run_engine(1, &code.data, code.data.len(), &mut vm_b);
    let rc_e = run_engine(2, &code.data, code.data.len(), &mut vm_e);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e);
    vm_print(&mut out, "A:", &vm_a);
    vm_print(&mut out, "B:", &vm_b);
    vm_print(&mut out, "EXT:", &vm_e);
}

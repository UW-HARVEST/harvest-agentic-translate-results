// Translated from c_src/src/main.c

mod a;
mod b;
mod engine;
mod lib_target;
mod util;

use std::env;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use crate::engine::run_engine;
use crate::util::{vm_print, Vm};

fn usage<W: Write>(fp: &mut W, prog: &str) {
    let _ = write!(
        fp,
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
        prog
    );
}

/// Parses a token using C's `strtol` semantics with base 10. Returns Some(int)
/// only when the entire token is a valid integer (matching the C check
/// `e && *e=='\0'`). Empty tokens return None.
fn parse_int_token(s: &str) -> Option<i32> {
    if s.is_empty() {
        return None;
    }
    // strtol skips leading whitespace, accepts optional sign, parses digits.
    // Our tokens come pre-trimmed of whitespace, but strtol still allows leading
    // whitespace within an arbitrary single-arg string. We mirror its acceptance.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == 0x0B || bytes[i] == 0x0C) {
        i += 1;
    }
    let sign_start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if digits_start == i {
        return None; // no digits
    }
    if i != bytes.len() {
        return None; // trailing chars: e doesn't point to '\0'
    }
    // Parse using i64 to allow C-style overflow saturation for typical inputs;
    // beyond i64 range, strtol returns LONG_MAX/LONG_MIN. For simplicity (and
    // because callers cast to int), we attempt i64 then truncate to i32 like
    // `(int)strtol(...)`.
    let s_trim = &s[sign_start..];
    match s_trim.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => {
            // Try saturating: too large for i64 → C strtol saturates and sets errno.
            // The C code doesn't check errno, so it would still proceed with the
            // saturated value. We replicate by saturating at i64 bounds, then
            // truncating.
            if s_trim.starts_with('-') {
                Some(i64::MIN as i32)
            } else {
                Some(i64::MAX as i32)
            }
        }
    }
}

fn read_stdin_tokens(v: &mut Vec<i32>) -> usize {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return 0;
    }
    let mut count = 0usize;
    // C reads via fgets and tokenizes on ' ', '\t', '\n', '\r'.
    for tok in buf.split(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r') {
        if tok.is_empty() {
            continue;
        }
        // C uses strtol with base 10 and only accepts the token if e points to '\0'.
        if let Some(val) = parse_int_token(tok) {
            v.push(val);
            count += 1;
        }
    }
    count
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let prog = args.first().cloned().unwrap_or_else(|| "driver".to_string());

    let mut use_stdin = false;
    let mut code: Vec<i32> = Vec::new();

    let stderr = io::stderr();
    let mut err = stderr.lock();

    for arg in args.iter().skip(1) {
        if arg == "--help" {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            // C uses fprintf(stderr, ...) for usage in --help case.
            usage(&mut err, &prog);
            let _ = out.flush();
            return ExitCode::from(0);
        } else if arg == "--stdin" {
            use_stdin = true;
        } else {
            match parse_int_token(arg) {
                Some(v) => code.push(v),
                None => {
                    let _ = writeln!(err, "skip '{}'", arg);
                }
            }
        }
    }

    if use_stdin {
        read_stdin_tokens(&mut code);
    }

    if code.is_empty() {
        let _ = writeln!(err, "no program");
        return ExitCode::from(2);
    }

    let mut vm_a = Vm::new();
    let mut vm_b = Vm::new();
    let mut vm_e = Vm::new();

    let rc_a = run_engine(0, &code, &mut vm_a);
    let rc_b = run_engine(1, &code, &mut vm_b);
    let rc_e = run_engine(2, &code, &mut vm_e);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    vm_print(&mut out, "A:", &vm_a);
    vm_print(&mut out, "B:", &vm_b);
    vm_print(&mut out, "EXT:", &vm_e);
    let _ = out.flush();

    ExitCode::from(0)
}

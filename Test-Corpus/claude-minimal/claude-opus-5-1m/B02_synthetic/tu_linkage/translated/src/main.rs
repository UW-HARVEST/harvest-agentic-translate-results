// Translated from c_src/src/main.c

mod a;
mod b;
mod engine;
mod lib_target;
mod util;

use std::env;
use std::io::{self, BufRead, Write};

use engine::run_engine;
use util::{IntVec, VM};

fn usage(p: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(
        handle,
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.",
        p
    )
    .ok();
}

fn read_stdin(v: &mut IntVec) -> usize {
    let stdin = io::stdin();
    let mut count: usize = 0;
    let handle = stdin.lock();
    for line_result in handle.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        // Tokenize on space, tab, \r (lines() already strips \n)
        for tok in line.split(|c: char| c == ' ' || c == '\t' || c == '\r') {
            if tok.is_empty() {
                continue;
            }
            if let Ok(t) = tok.parse::<i64>() {
                v.push(t as i32);
                count += 1;
            }
        }
    }
    count
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog_name = args.first().cloned().unwrap_or_else(|| "driver".to_string());

    let mut use_stdin = false;
    let mut code = IntVec::new();

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--help" {
            usage(&prog_name);
            std::process::exit(0);
        } else if arg == "--stdin" {
            use_stdin = true;
        } else {
            match arg.parse::<i64>() {
                Ok(t) => {
                    code.push(t as i32);
                }
                Err(_) => {
                    let stderr = io::stderr();
                    let mut handle = stderr.lock();
                    writeln!(handle, "skip '{}'", arg).ok();
                }
            }
        }
        i += 1;
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.len() == 0 {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(handle, "no program").ok();
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, &code.data, &mut vm_a);
    let rc_b = run_engine(1, &code.data, &mut vm_b);
    let rc_e = run_engine(2, &code.data, &mut vm_e);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "RC:A={} B={} EXT={}", rc_a, rc_b, rc_e).ok();
    vm_a.print(&mut out, "A:");
    vm_b.print(&mut out, "B:");
    vm_e.print(&mut out, "EXT:");
}

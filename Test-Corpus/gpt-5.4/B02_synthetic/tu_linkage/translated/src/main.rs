mod a;
mod api;
mod b;
mod engine;
mod util;

use std::env;
use std::io::{self, BufRead, Write};
use util::{iv_free, iv_init, iv_push, vm_free, vm_init, vm_print, IntVec, VM};

static USAGE: &str = "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n";

fn usage(p: &str) {
    let _ = write!(io::stderr(), "{}", USAGE.replacen("{}", p, 1));
}

fn read_stdin(v: &mut IntVec) -> usize {
    let stdin = io::stdin();
    let mut count = 0usize;
    for line in stdin.lock().lines() {
        if let Ok(buf) = line {
            for part in buf.split_whitespace() {
                if let Ok(t) = part.parse::<i32>() {
                    let _ = iv_push(v, t);
                    count += 1;
                }
            }
        }
    }
    count
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_stdin = false;
    let mut code = IntVec::default();
    iv_init(&mut code);

    for arg in args.iter().skip(1) {
        if arg == "--help" {
            usage(&args[0]);
            iv_free(&mut code);
            return;
        } else if arg == "--stdin" {
            use_stdin = true;
        } else if let Ok(t) = arg.parse::<i32>() {
            let _ = iv_push(&mut code, t);
        } else {
            let _ = writeln!(io::stderr(), "skip '{}'", arg);
        }
    }

    if use_stdin {
        let _ = read_stdin(&mut code);
    }
    if code.len == 0 {
        let _ = writeln!(io::stderr(), "no program");
        iv_free(&mut code);
        std::process::exit(2);
    }

    let mut vm_a = VM::default();
    let mut vm_b = VM::default();
    let mut vm_e = VM::default();
    vm_init(&mut vm_a);
    vm_init(&mut vm_b);
    vm_init(&mut vm_e);

    let rc_a = engine::run_engine(0, &code.data, &mut vm_a);
    let rc_b = engine::run_engine(1, &code.data, &mut vm_b);
    let rc_e = engine::run_engine(2, &code.data, &mut vm_e);

    println!("RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    vm_print(&mut io::stdout(), "A:", &vm_a);
    vm_print(&mut io::stdout(), "B:", &vm_b);
    vm_print(&mut io::stdout(), "EXT:", &vm_e);

    vm_free(&mut vm_a);
    vm_free(&mut vm_b);
    vm_free(&mut vm_e);
    iv_free(&mut code);
}

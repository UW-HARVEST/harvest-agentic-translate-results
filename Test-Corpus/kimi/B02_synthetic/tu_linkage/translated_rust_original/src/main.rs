use std::env;
use std::io::{self, BufRead};

mod util;
mod engine;
mod a;
mod b;
mod lib;

use util::{IntVec, VM, vm_init, vm_free, vm_print, iv_init, iv_free, iv_push};
use engine::run_engine;

fn usage(p: &str) {
    eprintln!("Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.", p);
}

fn read_stdin(v: &mut IntVec) -> usize {
    let stdin = io::stdin();
    let mut count = 0;
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            for token in line.split_whitespace() {
                if let Ok(t) = token.parse::<i64>() {
                    iv_push(v, t as i32);
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
    let mut code = IntVec::new();
    iv_init(&mut code);

    for i in 1..args.len() {
        if args[i] == "--help" {
            usage(&args[0]);
            iv_free(&mut code);
            return;
        } else if args[i] == "--stdin" {
            use_stdin = true;
        } else {
            if let Ok(t) = args[i].parse::<i64>() {
                iv_push(&mut code, t as i32);
            } else {
                eprintln!("skip '{}'", args[i]);
            }
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.len == 0 {
        eprintln!("no program");
        iv_free(&mut code);
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();
    vm_init(&mut vm_a);
    vm_init(&mut vm_b);
    vm_init(&mut vm_e);

    let rc_a = run_engine(0, &code.data[..code.len], &mut vm_a);
    let rc_b = run_engine(1, &code.data[..code.len], &mut vm_b);
    let rc_e = run_engine(2, &code.data[..code.len], &mut vm_e);

    println!("RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    vm_print(&mut std::io::stdout(), "A:", &vm_a);
    vm_print(&mut std::io::stdout(), "B:", &vm_b);
    vm_print(&mut std::io::stdout(), "EXT:", &vm_e);

    vm_free(&mut vm_a);
    vm_free(&mut vm_b);
    vm_free(&mut vm_e);
    iv_free(&mut code);
}

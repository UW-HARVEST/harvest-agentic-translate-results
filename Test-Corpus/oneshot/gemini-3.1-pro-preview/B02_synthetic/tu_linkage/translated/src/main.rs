mod api;
mod util;
mod engine;
mod a;
mod b;

use std::env;
use std::io::BufRead;
use util::VM;
use engine::run_engine;

fn usage(p: &str) {
    eprintln!("Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.", p);
}

fn read_stdin(v: &mut Vec<i32>) -> usize {
    let stdin = std::io::stdin();
    let mut count = 0;
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            for token in line.split_whitespace() {
                if let Ok(num) = token.parse::<i32>() {
                    v.push(num);
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
    let mut code = Vec::new();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--help" {
            usage(&args[0]);
            return;
        } else if args[i] == "--stdin" {
            use_stdin = true;
        } else {
            if let Ok(num) = args[i].parse::<i32>() {
                code.push(num);
            } else {
                eprintln!("skip '{}'", args[i]);
            }
        }
        i += 1;
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.is_empty() {
        eprintln!("no program");
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, &code, &mut vm_a);
    let rc_b = run_engine(1, &code, &mut vm_b);
    let rc_e = run_engine(2, &code, &mut vm_e);

    println!("RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    vm_a.print("A:");
    vm_b.print("B:");
    vm_e.print("EXT:");
}

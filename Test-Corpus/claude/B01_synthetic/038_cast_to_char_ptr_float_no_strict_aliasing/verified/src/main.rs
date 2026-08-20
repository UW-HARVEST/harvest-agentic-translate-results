// Rust translation of c_src/src/main.c -- executable entry point.
//
// See src/scan.rs for the implementation and a description of the exact glibc
// scanf("%f") behavior that is reproduced.

mod scan;

use std::io::{self, Write};

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let rc = scan::run(&mut input, &mut out);
    let _ = out.flush();
    std::process::exit(rc);
}

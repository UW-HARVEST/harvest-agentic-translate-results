//! Entry point of the translation of c_src/src/main.c.
//!
//! The program logic lives in `driver.rs` so that both this binary and the
//! shared-library `main` export drive exactly the same code.

mod analyzer;
mod cio;
mod driver;
mod tokenizer;

use analyzer::Analyzer;
use cio::{In, Out};
use tokenizer::Tokenizer;

/// The Rust runtime ignores `SIGPIPE`, while a C program starts with the
/// default disposition; restore it so that writing to a closed `stdout` kills
/// this process exactly like it kills the C build.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let mut out = Out::new();
    let mut stdin = In::new();

    // Get tokenizer operations (function pointers)
    let mut tok = Tokenizer::new();

    // Initialize analyzer with function pointers
    let mut an = Analyzer::new();

    driver::run(&mut out, &mut stdin, &mut tok, &mut an);
}

//! Entry point.
//!
//! The original C source is a shared library — it exposes only the
//! `ldexp_q2` symbol and has no `main`. The "executable" we produce
//! therefore does nothing and writes no output, exiting with status 0.

#[allow(unused_imports)]
use translated_rust::ldexp_q2;

fn main() {}

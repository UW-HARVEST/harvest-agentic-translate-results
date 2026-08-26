//! `driver` executable -- translation of `c_src/src/main.c`.
//!
//! `main.c`'s `int main(int argc, char **argv)` is translated in `src/lib.rs`
//! and exported there with C linkage (the shared library built from the C
//! sources exports `main` too, so the Rust shared library has to as well).
//!
//! This binary therefore declares `#![no_main]` and lets the C runtime call
//! that very same `main` symbol, exactly like the C program does -- rather than
//! adding a second Rust-side entry point that could drift from it.

#![no_main]

extern crate driver;

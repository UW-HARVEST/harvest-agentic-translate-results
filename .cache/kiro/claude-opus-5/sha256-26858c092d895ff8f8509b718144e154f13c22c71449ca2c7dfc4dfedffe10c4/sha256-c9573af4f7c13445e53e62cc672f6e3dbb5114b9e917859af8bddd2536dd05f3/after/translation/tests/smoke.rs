//! Harness smoke test: proves both `.so`s load and that every symbol required by
//! `SYMBOLS.md` is resolvable through `dlsym` in BOTH libraries.
mod harness;
use harness::*;

#[test]
fn both_libraries_export_every_symbol() {
    let (c, r) = both();
    println!("C   : {:?}", c.path);
    println!("Rust: {:?}", r.path);
    // Loading already `dlsym`ed all 10 symbols and would have panicked on a
    // missing one; make one trivial call through each library to prove the
    // pointers are live and the struct-return ABI agrees.
    let a = (c.c2V)(1.5, -2.5);
    let b = (r.c2V)(1.5, -2.5);
    assert!(same_v(a, b), "c2V: C {} != Rust {}", fmt_v(a), fmt_v(b));
}

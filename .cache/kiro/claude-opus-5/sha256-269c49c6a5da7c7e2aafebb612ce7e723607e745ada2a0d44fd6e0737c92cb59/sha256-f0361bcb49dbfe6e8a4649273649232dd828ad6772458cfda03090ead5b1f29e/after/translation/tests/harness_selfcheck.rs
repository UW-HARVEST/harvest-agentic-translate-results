//! Harness self-check: prove both `.so` files are really being loaded and that
//! `diff_eq!` can actually observe a divergence (guards against a
//! silently-passing test suite).

mod common;
use common::*;

#[test]
fn harness_loads_two_distinct_libraries() {
    let l = libs();
    // Both handles must resolve to *different* code addresses.
    let c_addr = l.c.c2Dot as usize;
    let r_addr = l.r.c2Dot as usize;
    assert_ne!(
        c_addr, r_addr,
        "C and Rust c2Dot resolved to the same address — only one library loaded"
    );
    let c_gen = l.c.gen_ray as usize;
    let r_gen = l.r.gen_ray as usize;
    assert_ne!(c_gen, r_gen, "gen_ray resolved to the same address");

    // and both must actually compute something non-trivial
    let a = C2v { x: 3.0, y: 4.0 };
    assert_eq!((l.c.c2Len)(a), 5.0);
    assert_eq!((l.r.c2Len)(a), 5.0);
}

#[test]
fn harness_detects_divergence() {
    // `diff_eq!` must panic when the two values differ.
    let caught = std::panic::catch_unwind(|| {
        diff_eq!("self-check", 1u32, 2u32);
    });
    assert!(caught.is_err(), "diff_eq! failed to detect a divergence");
}

#[test]
fn print_library_paths() {
    let l = libs();
    // Resolve a symbol from each so the loader is definitely exercised.
    eprintln!(
        "C impl `{}` c2V @ {:p} | Rust impl `{}` c2V @ {:p}",
        l.c.name, l.c.c2V as *const (), l.r.name, l.r.c2V as *const ()
    );
}

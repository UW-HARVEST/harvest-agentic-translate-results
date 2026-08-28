//! Exploratory probe (not part of the verification suite): characterise the
//! *environmental* nondeterminism of `compare_allocations`.
//!
//! `compare_allocations()` compares the addresses returned by two consecutive
//! `malloc(sizeof(int))` calls, so its result depends on the state of the
//! process-wide glibc allocator (tcache is LIFO, so freeing `ptr1` then `ptr2`
//! makes the *next* pair of allocations come back in the opposite order). The
//! observable consequence is that the value has period 2 in the number of calls
//! made so far, for the C library just as much as for the Rust one.
//!
//! To prove that this is a property of the C code and not of the translation,
//! the probe loads the SAME C `.so` twice from two different paths (dlopen keeps
//! two independent instances) and interleaves them: the two C instances diverge
//! from each other exactly like C-vs-Rust does. Hence the differential tests
//! compare *pairs* of back-to-back calls, which is invariant under this parity.
//!
//! Run with: cargo test --test probe_alloc -- --nocapture

mod common;

use libloading::{Library, Symbol};
use std::ffi::c_int;

type Cmp = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[test]
fn probe_compare_allocations_sequence() {
    let (c, r) = common::both();

    let mut cs = [0; 8];
    let mut rs = [0; 8];
    for i in 0..8 {
        cs[i] = unsafe { (c.compare_allocations)(1, 2) };
        rs[i] = unsafe { (r.compare_allocations)(1, 2) };
    }
    println!("C/Rust interleaved   C: {cs:?}");
    println!("C/Rust interleaved   R: {rs:?}");

    let mut cs2 = [0; 8];
    for i in 0..8 {
        cs2[i] = unsafe { (c.compare_allocations)(1, 2) };
    }
    let mut rs2 = [0; 8];
    for i in 0..8 {
        rs2[i] = unsafe { (r.compare_allocations)(1, 2) };
    }
    println!("blocked              C: {cs2:?}");
    println!("blocked              R: {rs2:?}");

    // ---- C against an independent copy of itself -------------------------
    let tmp = std::env::temp_dir().join("c_lib_copy_probe.so");
    let src = {
        let build = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/build");
        std::fs::read_dir(build)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
            .unwrap()
    };
    std::fs::copy(&src, &tmp).unwrap();
    let lib2 = unsafe { Library::new(&tmp).unwrap() };
    let c2: Cmp = unsafe {
        let s: Symbol<Cmp> = lib2.get(b"compare_allocations").unwrap();
        *s
    };

    let mut a = [0; 8];
    let mut b = [0; 8];
    for i in 0..8 {
        a[i] = unsafe { (c.compare_allocations)(1, 2) };
        b[i] = unsafe { c2(1, 2) };
    }
    println!("C/C-copy interleaved C1: {a:?}");
    println!("C/C-copy interleaved C2: {b:?}");
    // Informational, deliberately not an assertion: on glibc the two C instances
    // disagree here (which is the whole point — the value depends on the shared
    // allocator, not on the implementation), but an allocator without a LIFO
    // per-thread cache could legitimately make them agree. The suite does not
    // depend on this; it depends on `normalize_allocator`, asserted below.
    if a == b {
        println!("note: this allocator does not exhibit the parity effect");
    } else {
        println!("note: two instances of the *same* C library disagree -> the value is a function of allocator state, not of the implementation");
    }

    // ---- canonicalising the allocator makes the value deterministic ------
    // This is the mechanism the real test suites use (see common::normalize_allocator).
    use common::AllocOrder;
    for order in AllocOrder::both() {
        let mut cs = [0; 8];
        let mut c2s = [0; 8];
        let mut rs = [0; 8];
        for i in 0..8 {
            common::normalize_allocator(order);
            cs[i] = unsafe { (c.compare_allocations)(1, 2) };
            common::normalize_allocator(order);
            c2s[i] = unsafe { c2(1, 2) };
            common::normalize_allocator(order);
            rs[i] = unsafe { (r.compare_allocations)(1, 2) };
        }
        println!("normalized {order:?}: C={cs:?} C-copy={c2s:?} Rust={rs:?}");
        let expect = order.expected_branch() + 10; // val1 = 1 > 0 -> +10
        assert_eq!(cs, [expect; 8], "C not deterministic under {order:?}");
        assert_eq!(c2s, [expect; 8], "C-copy not deterministic under {order:?}");
        assert_eq!(rs, [expect; 8], "Rust not deterministic under {order:?}");
    }
    println!("both address orderings are reachable deterministically");
}

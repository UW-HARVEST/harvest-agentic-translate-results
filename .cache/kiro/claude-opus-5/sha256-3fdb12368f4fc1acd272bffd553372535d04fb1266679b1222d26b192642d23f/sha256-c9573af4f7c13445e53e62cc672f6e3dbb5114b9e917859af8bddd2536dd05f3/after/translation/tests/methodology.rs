//! Methodology control tests.
//!
//! `compare_allocations` compares the addresses of two live `malloc(4)` blocks,
//! so its return value is a function of the process-wide allocator state, not
//! only of its arguments. glibc's tcache is LIFO, so the address ordering flips
//! from call to call. The consequence: a naive differential harness that calls C
//! once and then Rust once compares two DIFFERENT allocator states and reports a
//! difference that has nothing to do with the translation.
//!
//! These tests prove that claim by using the C library as its OWN reference
//! implementation. Two copies of the same C `.so` are loaded (dlopen dedupes by
//! path, so a copy is needed to get two independent instances):
//!
//!  * unseeded 1:1 interleaving makes the C library "disagree with itself";
//!  * seeding the tcache before each call makes it agree with itself.
//!
//! That is exactly the technique `valid_paths.rs` and `errors.rs` use, so those
//! suites measure the library rather than the allocator.

mod common;

use common::{Heap, Impl, HEAP_STATES};
use std::path::PathBuf;

fn two_c_instances() -> (Impl, Impl, PathBuf) {
    let src = common::c_so_path();
    let dir = std::env::temp_dir().join(format!("harvest_ctl_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let a = dir.join("libc_control_a.so");
    let b = dir.join("libc_control_b.so");
    std::fs::copy(&src, &a).expect("copy a");
    std::fs::copy(&src, &b).expect("copy b");
    (
        Impl::load("C-a", a),
        Impl::load("C-b", b),
        dir,
    )
}

#[test]
fn control_unseeded_interleaving_makes_c_disagree_with_itself() {
    let (a, b, dir) = two_c_instances();
    let mut disagreements = 0;
    for _ in 0..10 {
        let ra = unsafe { (a.compare_allocations)(5, 7) };
        let rb = unsafe { (b.compare_allocations)(5, 7) };
        if ra != rb {
            disagreements += 1;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        disagreements, 10,
        "expected the C library to 'disagree with itself' on every unseeded \
         1:1 interleaved call — if this ever stops holding, the allocator \
         behavior changed and the seeding rationale must be re-derived"
    );
}

#[test]
fn control_seeded_comparison_makes_c_agree_with_itself() {
    let (a, b, dir) = two_c_instances();
    for order in HEAP_STATES {
        for _ in 0..50 {
            common::seed_heap(order);
            let ra = unsafe { (a.compare_allocations)(5, 7) };
            common::seed_heap(order);
            let rb = unsafe { (b.compare_allocations)(5, 7) };
            assert_eq!(
                ra, rb,
                "seeded comparison must make the C library agree with itself"
            );
            assert_eq!(
                ra,
                match order {
                    Heap::Ascending => 11,
                    Heap::Descending => 12,
                },
                "the seeded ordering was not observed"
            );
        }
    }
    // Same for the full pipeline.
    for order in HEAP_STATES {
        for i in -5..5i32 {
            common::seed_heap(order);
            let ra = unsafe { (a.arity4)(i, i + 1, i + 2, i + 3) };
            common::seed_heap(order);
            let rb = unsafe { (b.arity4)(i, i + 1, i + 2, i + 3) };
            assert_eq!(ra, rb, "seeded arity4 must agree C-vs-C");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The seeding must work no matter what the tcache bin held beforehand. glibc
/// only puts a freed chunk in the tcache while the bin is below capacity
/// (7); beyond that it goes to the fastbin and a naive "free hi, free lo" seed
/// silently stops controlling which chunk `malloc` hands back next. This test
/// pre-loads the bin with every count from 0 to 12 and checks the forced
/// ordering is still observed — it guards the invariant every other seeded
/// assertion in this repo depends on.
#[test]
fn control_seeding_survives_any_prefilled_tcache() {
    use std::ffi::c_void;
    unsafe extern "C" {
        fn malloc(n: usize) -> *mut c_void;
        fn free(p: *mut c_void);
    }

    let (a, _b, dir) = two_c_instances();
    let rust = common::load_rust();

    for prefill in 0..=12usize {
        // Put `prefill` chunks into the 32-byte bin, in descending address order
        // so the head is deliberately hostile to the Ascending seed.
        let mut held: Vec<*mut c_void> = (0..prefill).map(|_| unsafe { malloc(4) }).collect();
        held.sort_unstable_by_key(|p| *p as usize);
        for p in held.into_iter() {
            unsafe { free(p) };
        }

        for order in HEAP_STATES {
            let expected = match order {
                Heap::Ascending => 11,
                Heap::Descending => 12,
            };
            common::seed_heap(order);
            let rc = unsafe { (a.compare_allocations)(1, 2) };
            common::seed_heap(order);
            let rr = unsafe { (rust.compare_allocations)(1, 2) };
            assert_eq!(
                rc, expected,
                "seeding failed for C with a bin pre-filled with {prefill} chunks \
                 [heap={order:?}]"
            );
            assert_eq!(
                rr, expected,
                "seeding failed for Rust with a bin pre-filled with {prefill} chunks \
                 [heap={order:?}]"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The Rust `.so` must behave the same way toward the C `.so` as the C `.so`
/// behaves toward a second copy of itself: same disagreement without seeding,
/// same agreement with seeding. If the Rust side had a different allocation
/// pattern (an extra `malloc`, a different size, a different free order), the
/// seeded comparison would fail even though the C-vs-C control passes.
#[test]
fn control_rust_matches_c_exactly_as_c_matches_itself() {
    let (a, _b, dir) = two_c_instances();
    let rust = common::load_rust();
    for order in HEAP_STATES {
        for i in -20..20i32 {
            common::seed_heap(order);
            let rc = unsafe { (a.arity4)(i, -i, i % 3, i + 7) };
            common::seed_heap(order);
            let rr = unsafe { (rust.arity4)(i, -i, i % 3, i + 7) };
            assert_eq!(rc, rr, "arity4({i}, ..) [heap={order:?}]");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

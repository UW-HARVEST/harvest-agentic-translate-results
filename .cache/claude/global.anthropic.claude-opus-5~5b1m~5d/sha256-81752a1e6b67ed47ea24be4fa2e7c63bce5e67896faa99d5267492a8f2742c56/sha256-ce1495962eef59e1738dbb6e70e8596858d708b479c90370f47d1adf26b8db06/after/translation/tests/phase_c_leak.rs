//! Allocator-accounting differential test.
//!
//! `free_array` has no return value, so a translation that forgot one of its two
//! `free()` calls is invisible to every return-value comparison — and
//! `mutation_check.sh` confirmed that mutant SURVIVED the rest of the suite.
//!
//! Both `.so`s and this test binary share glibc's allocator (see `SYMBOLS.md`:
//! the Rust cdylib deliberately imports glibc `malloc`/`realloc`/`free` rather
//! than using Rust's allocator), so `mallinfo2()` can measure each library's net
//! allocator footprint across an identical churn loop and compare them.

mod common;

use common::load;

/// glibc `struct mallinfo2` — 10 `size_t` fields (glibc >= 2.33).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Mallinfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    /// total allocated space — what a leak grows
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

extern "C" {
    fn mallinfo2() -> Mallinfo2;
}

fn in_use() -> usize {
    unsafe { mallinfo2().uordblks }
}

/// Number of init/free cycles. Chosen so that either missing `free()` leaks far
/// more than `TOLERANCE`:
///   * missing `free(arr->data)` -> N * CAP * 4 == 800 MB
///   * missing `free(arr)`       -> N * 24     == 4.8 MB
const N: usize = 200_000;
const CAP: usize = 1000; // 4000 bytes: below the 128 KiB mmap threshold, so it
                         // lands in the arena and is counted by `uordblks`
const TOLERANCE: usize = 1 << 20; // 1 MiB of allocator slack

fn churn(imp: &common::Impl) -> usize {
    // warm the arena so the measurement is not dominated by first-touch growth
    unsafe {
        for _ in 0..1000 {
            let a = imp.init_array(CAP);
            imp.free_array(a);
        }
    }
    let before = in_use();
    unsafe {
        for _ in 0..N {
            let a = imp.init_array(CAP);
            assert!(!a.is_null(), "{}: init_array({CAP}) failed mid-churn", imp.name);
            imp.add_element(a, 1);
            imp.add_element(a, 2);
            imp.free_array(a);
        }
    }
    in_use().saturating_sub(before)
}

#[test]
fn l1_free_array_reclaims_everything_in_both() {
    let p = load();

    let c_delta = churn(&p.c);
    let rs_delta = churn(&p.rs);

    let leak_if_data_missed = N * CAP * 4;
    let leak_if_struct_missed = N * std::mem::size_of::<common::DynamicArray>();

    println!(
        "C net allocator growth   = {c_delta} bytes\n\
         Rust net allocator growth = {rs_delta} bytes\n\
         (a missing free(arr->data) would be ~{leak_if_data_missed}, \
          a missing free(arr) ~{leak_if_struct_missed})"
    );

    assert!(
        c_delta < TOLERANCE,
        "the C library itself grew by {c_delta} bytes over {N} init/free cycles — \
         the measurement is unsound, not a translation bug"
    );
    assert!(
        rs_delta < TOLERANCE,
        "Rust `free_array` did not reclaim everything: net allocator growth of \
         {rs_delta} bytes over {N} init/free cycles (C: {c_delta}). A missing \
         free(arr->data) would be ~{leak_if_data_missed} bytes, a missing \
         free(arr) ~{leak_if_struct_missed} bytes."
    );
    let diff = c_delta.abs_diff(rs_delta);
    assert!(
        diff < TOLERANCE,
        "C and Rust allocator footprints diverged by {diff} bytes \
         (C={c_delta}, Rust={rs_delta})"
    );
}

/// The same check for the `matrixsum` one-shot wrapper, which does its own
/// internal `init_array` / `add_element` x4 / `free_array` cycle.
#[test]
fn l2_matrixsum_does_not_leak_in_either() {
    let p = load();
    let iters = 500_000;

    let measure = |f: &dyn Fn(i32, i32, i32, i32) -> i32| -> usize {
        for i in 0..1000i32 {
            f(i, i + 1, i + 2, i + 3);
        }
        let before = in_use();
        for i in 0..iters as i32 {
            f(i, i.wrapping_mul(3), i ^ 0x5A5A, -i);
        }
        in_use().saturating_sub(before)
    };

    let c_delta = measure(&|a, b, c, d| p.c.matrixsum(a, b, c, d));
    let rs_delta = measure(&|a, b, c, d| p.rs.matrixsum(a, b, c, d));

    println!("matrixsum churn: C={c_delta} bytes, Rust={rs_delta} bytes over {iters} calls");
    assert!(
        c_delta < TOLERANCE,
        "the C `matrixsum` grew the heap by {c_delta} bytes — measurement unsound"
    );
    assert!(
        rs_delta < TOLERANCE,
        "Rust `matrixsum` leaks: net allocator growth {rs_delta} bytes over {iters} \
         calls (C: {c_delta})"
    );
    assert!(
        c_delta.abs_diff(rs_delta) < TOLERANCE,
        "matrixsum allocator footprints diverged (C={c_delta}, Rust={rs_delta})"
    );
}

/// `expand_array` must hand the old block back to the allocator (via `realloc`)
/// exactly as the C does — a translation that allocated a fresh block and forgot
/// the old one would leak here.
#[test]
fn l3_expand_array_does_not_leak_in_either() {
    let p = load();
    let cycles = 20_000;

    let measure = |imp: &common::Impl| -> usize {
        unsafe {
            for _ in 0..200 {
                let a = imp.init_array(4);
                for _ in 0..8 {
                    imp.expand_array(a);
                }
                imp.free_array(a);
            }
            let before = in_use();
            for _ in 0..cycles {
                let a = imp.init_array(4);
                for _ in 0..8 {
                    assert_eq!(imp.expand_array(a), 1, "{}: expand failed", imp.name);
                }
                imp.free_array(a);
            }
            in_use().saturating_sub(before)
        }
    };

    let c_delta = measure(&p.c);
    let rs_delta = measure(&p.rs);
    println!("expand churn: C={c_delta} bytes, Rust={rs_delta} bytes over {cycles} cycles");
    assert!(c_delta < TOLERANCE, "C grew by {c_delta} — measurement unsound");
    assert!(
        rs_delta < TOLERANCE,
        "Rust `expand_array`/`free_array` leaks: {rs_delta} bytes over {cycles} cycles \
         (C: {c_delta})"
    );
    assert!(
        c_delta.abs_diff(rs_delta) < TOLERANCE,
        "expand_array allocator footprints diverged (C={c_delta}, Rust={rs_delta})"
    );
}

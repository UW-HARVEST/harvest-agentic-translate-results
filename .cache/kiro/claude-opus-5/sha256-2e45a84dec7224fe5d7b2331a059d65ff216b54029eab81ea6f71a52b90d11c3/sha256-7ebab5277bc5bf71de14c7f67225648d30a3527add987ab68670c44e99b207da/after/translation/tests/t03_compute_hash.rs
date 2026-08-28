//! Level 3: `compute_hash`.
//!
//! The function's result depends only on the relative ordering of the two
//! `MemoryBlock` header pointers and of their `data` pointers. To cover every
//! branch deterministically the tests build the headers themselves and fill
//! `data` with synthetic addresses -- `compute_hash` never dereferences them.

mod common;

use common::{ComputeHashFn, MemoryBlock, pair};
use std::ffi::c_int;

fn synth(addr: usize) -> *mut c_int {
    addr as *mut c_int
}

/// All 9 combinations of (data ordering) x (header ordering).
#[test]
fn compute_hash_covers_every_branch() {
    let p = pair();
    let c_fn: ComputeHashFn = *p.c.compute_hash();
    let rs_fn: ComputeHashFn = *p.rs.compute_hash();

    // Two headers with a known ordering, plus a third alias for the equal case.
    let mut a = MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    };
    let mut b = MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    };

    let data_pairs: [(usize, usize); 5] = [
        (0x1000, 0x2000), // d1 < d2
        (0x2000, 0x1000), // d1 > d2
        (0x1000, 0x1000), // d1 == d2
        (0, 0),           // both NULL -> equal
        (0, 0x1000),      // NULL < non-NULL
    ];

    for (d1, d2) in data_pairs {
        // Distinct headers, both orderings.
        for swap in [false, true] {
            let (m1, m2): (*mut MemoryBlock, *mut MemoryBlock) = if swap {
                (&mut b, &mut a)
            } else {
                (&mut a, &mut b)
            };
            unsafe {
                (*m1).data = synth(d1);
                (*m2).data = synth(d2);
                let cv = c_fn(m1, m2);
                let rv = rs_fn(m1, m2);
                assert_eq!(
                    cv, rv,
                    "compute_hash mismatch: data=({d1:#x},{d2:#x}) swap={swap}"
                );

                // Cross-check against the C's own rules.
                let mut expect = 0;
                if d1 < d2 {
                    expect += 100;
                } else if d1 > d2 {
                    expect += 200;
                }
                if (m1 as usize) < (m2 as usize) {
                    expect += 10;
                } else if (m1 as usize) > (m2 as usize) {
                    expect += 20;
                }
                assert_eq!(cv, expect, "unexpected C value for ({d1:#x},{d2:#x})");
            }
        }

        // Same header passed twice -> header comparison is "equal".
        unsafe {
            a.data = synth(d1);
            let m: *mut MemoryBlock = &mut a;
            let cv = c_fn(m, m);
            let rv = rs_fn(m, m);
            assert_eq!(cv, rv, "compute_hash aliased mismatch for {d1:#x}");
            assert_eq!(cv, 0, "aliased headers with equal data must score 0");
        }
    }
}

/// Also exercise the function against real blocks produced by each library's
/// own `allocate_block`, which is how `betagamma` uses it.
#[test]
fn compute_hash_on_real_allocations_matches() {
    let p = pair();
    let c_hash: ComputeHashFn = *p.c.compute_hash();
    let rs_hash: ComputeHashFn = *p.rs.compute_hash();
    let c_alloc = p.c.allocate_block();
    let c_free = p.c.free_block();
    let rs_alloc = p.rs.allocate_block();
    let rs_free = p.rs.free_block();

    for count in [1usize, 5, 8, 14, 64] {
        unsafe {
            let c1 = c_alloc(count, 1);
            let c2 = c_alloc(count, 2);
            let r1 = rs_alloc(count, 1);
            let r2 = rs_alloc(count, 2);
            assert!(!c1.is_null() && !c2.is_null() && !r1.is_null() && !r2.is_null());

            // Both libraries must agree when handed *the same* blocks.
            assert_eq!(c_hash(c1, c2), rs_hash(c1, c2), "count={count}");
            assert_eq!(c_hash(r1, r2), rs_hash(r1, r2), "count={count}");
            assert_eq!(c_hash(c2, c1), rs_hash(c2, c1), "count={count}");

            c_free(c1);
            c_free(c2);
            rs_free(r1);
            rs_free(r2);
        }
    }
}

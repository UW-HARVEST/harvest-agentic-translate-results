//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every rejection is compared by its *exact* result (the `NULL` sentinel, the
//! `-1` return, or the terminating signal) — never merely "both failed".

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
}

// ===========================================================================
// E1 — allocate_block: malloc(sizeof(MemoryBlock)) failure
// ===========================================================================

/// `if (!mb) return NULL;` (lib.c:50). The request is a fixed 16 bytes and is
/// independent of every argument, so no input can drive it to fail on a 64-bit
/// host. What IS observable — and what the row asserts — is that the guard
/// never fires for any reachable `count`, in both implementations alike.
#[test]
fn err_e1_allocate_block_malloc_failure_guard_never_fires() {
    let pair = load_pair();
    unsafe {
        for count in [0usize, 1, 2, 5, 14, 1000] {
            let mc = (pair.c.allocate_block)(count, 0);
            let mr = (pair.rs.allocate_block)(count, 0);
            assert!(!mc.is_null(), "E1: C returned NULL for count={count}");
            assert!(!mr.is_null(), "E1: Rust returned NULL for count={count}");
            (pair.c.free_block)(mc);
            (pair.rs.free_block)(mr);
        }
    }
}

// ===========================================================================
// E2 — allocate_block: calloc(count, 4) unsatisfiable  =>  NULL
// ===========================================================================

#[test]
fn err_e2_allocate_block_calloc_overflow_returns_null() {
    let pair = load_pair();
    // Only counts whose `count * 4` is either an integer overflow or an
    // impossible mapping, so `calloc` is guaranteed to fail rather than
    // succeeding lazily via overcommit.
    let counts: &[usize] = &[
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4,
        1usize << 62,
        1usize << 63,
        0x4000_0000_0000_0000,
        0xFFFF_FFFF_FFFF_FFF0,
    ];
    unsafe {
        for &count in counts {
            let mc = (pair.c.allocate_block)(count, 7);
            let mr = (pair.rs.allocate_block)(count, 7);
            assert!(
                mc.is_null(),
                "E2: C unexpectedly succeeded for count={count:#x}"
            );
            assert_eq!(
                mc.is_null(),
                mr.is_null(),
                "E2: allocate_block({count:#x}) sentinel diverged: \
                 C null={}, Rust null={}",
                mc.is_null(),
                mr.is_null()
            );
        }
    }
}

// ===========================================================================
// E3 — allocate_block(0, _) is NOT an error (boundary below the valid range)
// ===========================================================================

#[test]
fn err_e3_allocate_block_zero_count_is_not_an_error() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE3);
    unsafe {
        for _ in 0..200 {
            let init = rng.interesting_i32();
            let mc = (pair.c.allocate_block)(0, init);
            let mr = (pair.rs.allocate_block)(0, init);
            assert!(!mc.is_null(), "E3: C returned NULL for count=0");
            assert_eq!(mc.is_null(), mr.is_null(), "E3: sentinel diverged");
            assert_eq!((*mc).size, 0, "E3: C size");
            assert_eq!((*mr).size, 0, "E3: Rust size");
            // glibc calloc(0, 4) yields a unique NON-NULL pointer; the C relies
            // on that in betagamma's `mem1->data > NULL` guard.
            assert!(!(*mc).data.is_null(), "E3: C data was NULL for count=0");
            assert_eq!(
                (*mc).data.is_null(),
                (*mr).data.is_null(),
                "E3: data NULL-ness diverged for count=0"
            );
            (pair.c.free_block)(mc);
            (pair.rs.free_block)(mr);
        }
    }
}

// ===========================================================================
// E4 — free_block(NULL) is a silent no-op
// ===========================================================================

#[test]
fn err_e4_free_block_null_is_noop() {
    let pair = load_pair();
    let sc = child_status(|| unsafe {
        for _ in 0..1000 {
            (pair.c.free_block)(std::ptr::null_mut());
        }
    });
    let sr = child_status(|| unsafe {
        for _ in 0..1000 {
            (pair.rs.free_block)(std::ptr::null_mut());
        }
    });
    assert_eq!(
        outcome(sc),
        Outcome::Exited(0),
        "E4: C crashed on free_block(NULL)"
    );
    assert_eq!(
        outcome(sc),
        outcome(sr),
        "E4: free_block(NULL) outcome diverged"
    );
}

// ===========================================================================
// E5 — free_block with mb != NULL but mb->data == NULL
// ===========================================================================

#[test]
fn err_e5_free_block_null_data_skips_inner_free() {
    let pair = load_pair();
    // The struct must be heap-allocated because free_block free()s it.
    let make = || unsafe {
        let p = malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
        (*p).data = std::ptr::null_mut();
        (*p).size = 12345; // size is not consulted by free_block
        p
    };
    let sc = child_status(|| unsafe {
        for _ in 0..200 {
            (pair.c.free_block)(make());
        }
    });
    let sr = child_status(|| unsafe {
        for _ in 0..200 {
            (pair.rs.free_block)(make());
        }
    });
    assert_eq!(
        outcome(sc),
        Outcome::Exited(0),
        "E5: C crashed freeing a block with NULL data"
    );
    assert_eq!(
        outcome(sc),
        outcome(sr),
        "E5: free_block(mb with NULL data) outcome diverged"
    );
}

// ===========================================================================
// E6 — betagamma: (param1 % 10) + 5 < 0  =>  -1
// ===========================================================================

/// Every `param1` whose C remainder puts `block_size` below zero. Derived from
/// the source, not guessed: `param1 % 10` in C truncates toward zero, so a
/// negative `param1` yields a negative remainder, and `-6 .. -9` are the four
/// residues that push `+5` below zero.
fn negative_block_size_params(rng: &mut Rng) -> Vec<i32> {
    let mut v = Vec::new();
    for r in 6i32..=9 {
        for mult in [0i32, 1, 2, 3, 10, 100, 1000, 214_748_36] {
            v.push(-(mult.wrapping_mul(10).wrapping_add(r)));
        }
    }
    for _ in 0..2000 {
        let x = rng.next_i32();
        let p = if x > 0 { -x } else { x };
        if p != i32::MIN && matches!((p % 10).abs(), 6 | 7 | 8 | 9) {
            v.push(p);
        }
    }
    v
}

#[test]
fn err_e6_betagamma_negative_block_size_returns_minus_one() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE6);
    let params = negative_block_size_params(&mut rng);
    assert!(params.len() > 100, "test bug: too few E6 cases");

    let n = params.len();
    let batch = params.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        for (i, &p1) in batch.iter().enumerate() {
            out[i] = (imp.betagamma)(p1, 1, 2, 3);
        }
    });
    for i in 0..n {
        assert_eq!(
            c[i], -1,
            "E6: C did not return -1 for param1={} (block_size={})",
            params[i],
            params[i] % 10 + 5
        );
        assert_eq!(
            c[i], rs[i],
            "E6: betagamma({}, 1, 2, 3) diverged: C={} Rust={}",
            params[i], c[i], rs[i]
        );
    }
}

// ===========================================================================
// E7 — betagamma(INT_MIN, ..)  =>  -1  (extreme boundary of E6)
// ===========================================================================

#[test]
fn err_e7_betagamma_int_min_param1() {
    let pair = load_pair();
    // INT_MIN % 10 == -8 in C  =>  block_size == -3  =>  the -1 path.
    let others = [0i32, 1, -1, i32::MIN, i32::MAX];
    let mut cases = Vec::new();
    for &b in &others {
        for &c in &others {
            for &d in &others {
                cases.push([i32::MIN, b, c, d]);
            }
        }
    }
    let n = cases.len();
    let batch = cases.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        for (i, p) in batch.iter().enumerate() {
            out[i] = (imp.betagamma)(p[0], p[1], p[2], p[3]);
        }
    });
    for i in 0..n {
        assert_eq!(c[i], -1, "E7: C did not return -1 for {:?}", cases[i]);
        assert_eq!(c[i], rs[i], "E7: diverged for {:?}", cases[i]);
    }
}

// ===========================================================================
// E8 — betagamma with block_size == 0 is NOT an error
// ===========================================================================

#[test]
fn err_e8_betagamma_zero_block_size_is_not_an_error() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE8);
    // param1 % 10 == -5  =>  block_size == 0
    let mut cases: Vec<[i32; 4]> = Vec::new();
    for mult in [0i32, 1, 2, 3, 10, 100, 1000, 21_474_836] {
        let p1 = -(mult.wrapping_mul(10) + 5);
        cases.push([p1, 0, 0, 0]);
        cases.push([p1, i32::MAX, i32::MIN, -1]);
        for _ in 0..50 {
            cases.push([
                p1,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ]);
        }
    }
    let n = cases.len();
    let batch = cases.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        for (i, p) in batch.iter().enumerate() {
            out[i] = (imp.betagamma)(p[0], p[1], p[2], p[3]);
        }
    });
    for i in 0..n {
        assert_eq!(
            c[i], rs[i],
            "E8: betagamma{:?} diverged: C={} Rust={}",
            cases[i], c[i], rs[i]
        );
    }
    // With block_size == 0 both sum loops are empty and `data` is still
    // non-NULL, so the result is +99 +255 + hash — never the -1 sentinel.
    let sentinels = (0..n).filter(|&i| c[i] == -1).count();
    assert_eq!(
        sentinels, 0,
        "E8: C returned the -1 error sentinel for a block_size==0 case"
    );
}

// ===========================================================================
// E9 — compute_hash with aliased operands
// ===========================================================================

#[test]
fn err_e9_compute_hash_aliased_pointers() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE9);
    unsafe {
        // synthetic aliased struct: both 3-way branches take the "equal" arm
        for d in [0usize, 1, 0x1000, usize::MAX] {
            let mut mb = MemoryBlock {
                data: d as *mut c_int,
                size: 3,
            };
            let p: *mut MemoryBlock = &mut mb;
            let hc = (pair.c.compute_hash)(p, p);
            let hr = (pair.rs.compute_hash)(p, p);
            assert_eq!(hc, 0, "E9: C aliased hash should be 0, got {hc} (data={d:#x})");
            assert_eq!(hc, hr, "E9: aliased compute_hash diverged (data={d:#x})");
        }
        // real allocation, aliased
        for _ in 0..200 {
            let count = rng.below(20) as usize;
            let m = (pair.c.allocate_block)(count, 5);
            let hc = (pair.c.compute_hash)(m, m);
            let hr = (pair.rs.compute_hash)(m, m);
            assert_eq!(hc, 0, "E9: C aliased hash on real block should be 0");
            assert_eq!(hc, hr, "E9: aliased compute_hash on real block diverged");
            (pair.c.free_block)(m);
        }
    }
}

// ===========================================================================
// E10 — compute_hash has NO null guard: it must FAULT, not reject
// ===========================================================================

#[test]
fn err_e10_compute_hash_has_no_null_guard() {
    let pair = load_pair();
    let mut good = MemoryBlock {
        data: 0x1000 as *mut c_int,
        size: 0,
    };
    let gp: *mut MemoryBlock = &mut good;
    let nul: *mut MemoryBlock = std::ptr::null_mut();

    for (label, a, b) in [
        ("NULL,NULL", nul, nul),
        ("NULL,valid", nul, gp),
        ("valid,NULL", gp, nul),
    ] {
        let sc = child_status(|| unsafe {
            let _ = (pair.c.compute_hash)(a, b);
        });
        let sr = child_status(|| unsafe {
            let _ = (pair.rs.compute_hash)(a, b);
        });
        // The C dereferences mb1->data / mb2->data unconditionally, so at least
        // one of these must be a fault; whatever the C does, the Rust must match.
        assert_eq!(
            outcome(sc),
            outcome(sr),
            "E10: compute_hash({label}) outcome diverged: C={:?} Rust={:?}",
            outcome(sc),
            outcome(sr)
        );
        assert!(
            matches!(outcome(sc), Outcome::Signalled(_)),
            "E10: expected the unguarded C deref to fault for ({label}), got {:?}",
            outcome(sc)
        );
    }
}

// ===========================================================================
// E11 — create_block has NO null guard on `name`: it must FAULT, not reject
// ===========================================================================

#[test]
fn err_e11_create_block_has_no_null_guard() {
    let pair = load_pair();
    let sc = child_status(|| unsafe {
        let _ = (pair.c.create_block)(1, std::ptr::null(), 0xAA);
    });
    let sr = child_status(|| unsafe {
        let _ = (pair.rs.create_block)(1, std::ptr::null(), 0xAA);
    });
    assert_eq!(
        outcome(sc),
        outcome(sr),
        "E11: create_block(_, NULL, _) outcome diverged: C={:?} Rust={:?}",
        outcome(sc),
        outcome(sr)
    );
    assert!(
        matches!(outcome(sc), Outcome::Signalled(_)),
        "E11: expected strcpy(dest, NULL) to fault in C, got {:?}",
        outcome(sc)
    );
}

// ===========================================================================
// E12 — create_block name-length boundary (31 chars + NUL exactly fills [32])
// ===========================================================================

#[test]
fn err_e12_create_block_max_length_name_boundary() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE12);
    unsafe {
        for len in 0usize..=31 {
            for _ in 0..40 {
                let mut buf: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
                buf.push(0);
                let flags = rng.next_u32() as u8;
                let id = rng.interesting_i32();
                let bc = (pair.c.create_block)(id, buf.as_ptr() as *const c_char, flags);
                let br = (pair.rs.create_block)(id, buf.as_ptr() as *const c_char, flags);
                assert_eq!(
                    defined(&bc),
                    defined(&br),
                    "E12: create_block diverged at name length {len}"
                );
                // At the exact boundary the NUL lands on the last byte of name[].
                if len == 31 {
                    let d = defined(&bc);
                    assert_eq!(d.name.len(), 32, "E12: 31-char name must fill name[32]");
                    assert_eq!(d.name[31], 0, "E12: NUL must be at index 31");
                }
            }
        }
    }
    // Lengths > 31 overflow the struct in C (undefined behaviour in both
    // implementations) and are deliberately NOT exercised.
}

// ===========================================================================
// E13 — allocate_block: init_value + i overflows int, wraps, never errors
// ===========================================================================

#[test]
fn err_e13_allocate_block_init_value_overflow_wraps() {
    let pair = load_pair();
    unsafe {
        for (count, init) in [
            (14usize, i32::MAX),
            (14, i32::MAX - 1),
            (14, i32::MAX - 13),
            (14, i32::MIN),
            (1000, i32::MAX - 500),
            (5, i32::MAX - 2),
        ] {
            let mc = (pair.c.allocate_block)(count, init);
            let mr = (pair.rs.allocate_block)(count, init);
            assert!(!mc.is_null() && !mr.is_null(), "E13: overflow must not error");
            let a = std::slice::from_raw_parts((*mc).data, count);
            let b = std::slice::from_raw_parts((*mr).data, count);
            assert_eq!(a, b, "E13: wrapped contents diverged for ({count}, {init})");
            for (i, &v) in a.iter().enumerate() {
                let expect = (init as isize as usize).wrapping_add(i) as u32 as i32;
                assert_eq!(
                    v, expect,
                    "E13: element {i} of ({count}, {init}) is not the C's wrapped value"
                );
            }
            (pair.c.free_block)(mc);
            (pair.rs.free_block)(mr);
        }
    }
}

// ===========================================================================
// Generic FFI-boundary sweep (per the ERRORS.md closing section)
// ===========================================================================

/// This library declares no `enum`, so there is no invalid-variant class. The
/// equivalent "any bit pattern is a valid input" axis is `uint8_t flags` (swept
/// exhaustively over all 256 values, including the ones with no meaning to any
/// of the four mask tests) plus the unconstrained `int` parameters.
#[test]
fn err_generic_all_256_flag_values_and_int_extremes() {
    let pair = load_pair();
    unsafe {
        let name = b"x\0";
        for flags in 0u16..=255 {
            for id in [0i32, -1, 1, i32::MIN, i32::MAX] {
                let bc = (pair.c.create_block)(id, name.as_ptr() as *const c_char, flags as u8);
                let br = (pair.rs.create_block)(id, name.as_ptr() as *const c_char, flags as u8);
                assert_eq!(
                    defined(&bc),
                    defined(&br),
                    "generic: create_block(id={id}, flags={flags}) diverged"
                );
            }
        }
    }

    // Oversized / zero / one-past-range lengths for allocate_block in one sweep.
    unsafe {
        for count in [
            0usize,
            1,
            2,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX / 4,
            1 << 62,
        ] {
            let mc = (pair.c.allocate_block)(count, -3);
            let mr = (pair.rs.allocate_block)(count, -3);
            assert_eq!(
                mc.is_null(),
                mr.is_null(),
                "generic: allocate_block({count:#x}) sentinel diverged"
            );
            if !mc.is_null() {
                assert_eq!((*mc).size, (*mr).size, "generic: size diverged");
                (pair.c.free_block)(mc);
                (pair.rs.free_block)(mr);
            }
        }
    }

    // betagamma one step either side of the block_size validity frontier.
    let frontier: Vec<i32> = (-24i32..=24).collect();
    let n = frontier.len();
    let batch = frontier.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        for (i, &p1) in batch.iter().enumerate() {
            out[i] = (imp.betagamma)(p1, -p1, p1 / 2, -3);
        }
    });
    for i in 0..n {
        assert_eq!(
            c[i], rs[i],
            "generic: betagamma frontier param1={} diverged: C={} Rust={}",
            frontier[i], c[i], rs[i]
        );
    }
}

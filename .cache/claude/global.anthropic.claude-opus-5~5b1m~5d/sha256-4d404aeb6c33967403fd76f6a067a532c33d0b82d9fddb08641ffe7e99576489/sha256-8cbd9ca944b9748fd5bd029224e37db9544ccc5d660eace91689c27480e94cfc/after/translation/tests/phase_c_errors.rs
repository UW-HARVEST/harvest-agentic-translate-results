//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//! Each test constructs the exact invalid input/condition and asserts BOTH
//! libraries return the SAME error code / sentinel, not merely "both failed".

mod common;
use common::oracle::{self, Op};
use common::*;
use std::ffi::c_int;

const SEED: u64 = 0xBADC0DE_9999;

// ---------------------------------------------------------------------------
// E1 — malloc(sizeof(MemoryBlock)) failure.
//
// A 16-byte malloc cannot be made to fail from outside the library, but it
// takes the *same* `return NULL` exit as the calloc failure below, which E2/E3
// drive.  Documented here for traceability.
// ---------------------------------------------------------------------------

#[test]
fn err_e1_documented_via_e2_e3() {
    // Sanity: the NULL sentinel really is a null pointer in both libs (E2).
    let l = libs();
    unsafe {
        assert!((l.c.allocate_block)(usize::MAX, 0).is_null());
        assert!((l.rs.allocate_block)(usize::MAX, 0).is_null());
    }
}

// ---------------------------------------------------------------------------
// E2 — calloc(count, 4) where count*4 OVERFLOWS size_t  => NULL
// ---------------------------------------------------------------------------

#[test]
fn err_e2_allocate_block_calloc_overflow() {
    let l = libs();
    let overflowing: [usize; 8] = [
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 2 + 1,
        usize::MAX / 4 + 1,
        usize::MAX / 4 + 2,
        1usize << 63,
        (1usize << 63) | 1,
    ];
    for &count in &overflowing {
        for init in [0i32, -1, i32::MIN, i32::MAX] {
            unsafe {
                let c = (l.c.allocate_block)(count, init);
                let r = (l.rs.allocate_block)(count, init);
                assert!(
                    c.is_null(),
                    "C allocate_block({}, {}) should be NULL, got {:p}",
                    count,
                    init,
                    c
                );
                assert!(
                    r.is_null(),
                    "Rust allocate_block({}, {}) should be NULL, got {:p}",
                    count,
                    init,
                    r
                );
                assert_eq!(
                    c.is_null(),
                    r.is_null(),
                    "NULL sentinel mismatch for count={}",
                    count
                );
                // Also exercise free_block on the returned NULL (E5).
                (l.c.free_block)(c);
                (l.rs.free_block)(r);
            }
        }
    }
    // Out of process too (same sentinel, pristine heap).
    let ops: Vec<Op> = overflowing
        .iter()
        .map(|&count| Op::AllocFail { count, init: 3 })
        .collect();
    oracle::assert_same("e2", &ops);
}

// ---------------------------------------------------------------------------
// E3 — calloc(count, 4) huge but NON-overflowing => NULL
// ---------------------------------------------------------------------------

#[test]
fn err_e3_allocate_block_calloc_too_large() {
    let l = libs();
    // Every one of these asks for >= 2^61 bytes, far past the 47-bit user
    // address space, so mmap must fail.  (Deliberately avoiding sizes that an
    // overcommitting kernel might grant, which would then loop for hours.)
    let huge: [usize; 5] = [
        usize::MAX / 4, // *4 == SIZE_MAX-3: no overflow, unallocatable
        1usize << 62,
        1usize << 61,
        (1usize << 62) - 1,
        (usize::MAX / 4) - 12345,
    ];
    for &count in &huge {
        unsafe {
            let c = (l.c.allocate_block)(count, 1);
            let r = (l.rs.allocate_block)(count, 1);
            assert!(c.is_null(), "C allocate_block({}) should be NULL", count);
            assert!(r.is_null(), "Rust allocate_block({}) should be NULL", count);
            (l.c.free_block)(c);
            (l.rs.free_block)(r);
        }
    }
    let ops: Vec<Op> = huge
        .iter()
        .map(|&count| Op::AllocFail { count, init: -7 })
        .collect();
    oracle::assert_same("e3", &ops);
}

// ---------------------------------------------------------------------------
// E4 — count == 0 is NOT an error: calloc(0, 4) returns a unique non-NULL ptr
// ---------------------------------------------------------------------------

#[test]
fn err_e4_allocate_block_zero_count() {
    let l = libs();
    for init in [0i32, 1, -1, i32::MIN, i32::MAX] {
        unsafe {
            let c = (l.c.allocate_block)(0, init);
            let r = (l.rs.allocate_block)(0, init);
            assert!(!c.is_null(), "C allocate_block(0, {}) must be non-NULL", init);
            assert!(
                !r.is_null(),
                "Rust allocate_block(0, {}) must be non-NULL",
                init
            );
            assert_eq!((*c).size, 0, "C size");
            assert_eq!((*r).size, 0, "Rust size");
            assert!(!(*c).data.is_null(), "C data must be non-NULL for count 0");
            assert!(
                !(*r).data.is_null(),
                "Rust data must be non-NULL for count 0"
            );
            (l.c.free_block)(c);
            (l.rs.free_block)(r);
        }
    }
    oracle::assert_same(
        "e4",
        &[
            Op::Allocate { count: 0, init: 0 },
            Op::Allocate { count: 0, init: -1 },
            Op::Allocate {
                count: 0,
                init: i32::MIN,
            },
        ],
    );
}

// ---------------------------------------------------------------------------
// E5 — free_block(NULL) is a no-op
// ---------------------------------------------------------------------------

#[test]
fn err_e5_free_block_null() {
    let l = libs();
    unsafe {
        for _ in 0..1000 {
            (l.c.free_block)(std::ptr::null_mut());
            (l.rs.free_block)(std::ptr::null_mut());
        }
    }
    // Reaching here without a crash in either library is the assertion.
}

// ---------------------------------------------------------------------------
// E6 — free_block on a non-NULL block whose `data` is NULL
// ---------------------------------------------------------------------------

#[test]
fn err_e6_free_block_null_data() {
    let l = libs();
    // The struct must come from malloc so `free(mb)` is legal; build it by
    // hijacking a real allocate_block result and NULLing its data (after
    // releasing the data buffer ourselves is not possible, so allocate a
    // 0-count block and free its data through the library first is also not
    // possible) -- instead allocate the struct via the library and overwrite
    // `data` with NULL *after* freeing the buffer via a second full free.
    unsafe {
        for _ in 0..200 {
            // C side
            let c = (l.c.allocate_block)(4, 1);
            assert!(!c.is_null());
            let cdata = (*c).data;
            (*c).data = std::ptr::null_mut();
            (l.c.free_block)(c); // must free only `mb`, skipping the NULL data
            // release the orphaned buffer through a struct that owns it
            let c2 = (l.c.allocate_block)(0, 0);
            assert!(!c2.is_null());
            let c2data = (*c2).data;
            (*c2).data = cdata;
            (l.c.free_block)(c2);
            (l.c.free_block)({
                let t = (l.c.allocate_block)(0, 0);
                (*t).data = c2data;
                t
            });

            // Rust side, identical sequence
            let r = (l.rs.allocate_block)(4, 1);
            assert!(!r.is_null());
            let rdata = (*r).data;
            (*r).data = std::ptr::null_mut();
            (l.rs.free_block)(r);
            let r2 = (l.rs.allocate_block)(0, 0);
            assert!(!r2.is_null());
            let r2data = (*r2).data;
            (*r2).data = rdata;
            (l.rs.free_block)(r2);
            (l.rs.free_block)({
                let t = (l.rs.allocate_block)(0, 0);
                (*t).data = r2data;
                t
            });
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — free_block on a valid block with size == 0 and non-NULL data
// ---------------------------------------------------------------------------

#[test]
fn err_e7_free_block_zero_size() {
    let l = libs();
    unsafe {
        for _ in 0..500 {
            let c = (l.c.allocate_block)(0, 42);
            let r = (l.rs.allocate_block)(0, 42);
            assert_eq!((*c).size, 0);
            assert_eq!((*r).size, 0);
            (l.c.free_block)(c);
            (l.rs.free_block)(r);
        }
    }
}

// ---------------------------------------------------------------------------
// E8 — betagamma with param1 % 10 in {-9..-6} => block_size < 0 => -1
// ---------------------------------------------------------------------------

#[test]
fn err_e8_betagamma_negative_block_size() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);
    let mut checked = 0;
    for r in -9i32..=-6 {
        for k in 0..40 {
            let p1 = r - 10 * k;
            assert_eq!(p1 % 10, r);
            let (p2, p3, p4) = (rng.i32(), rng.i32(), rng.i32());
            unsafe {
                let cv = (l.c.betagamma)(p1, p2, p3, p4);
                let rv = (l.rs.betagamma)(p1, p2, p3, p4);
                assert_eq!(cv, -1, "C betagamma({}, ..) must be -1", p1);
                assert_eq!(rv, -1, "Rust betagamma({}, ..) must be -1", p1);
                assert_eq!(cv, rv);
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 160);
}

// ---------------------------------------------------------------------------
// E9 / E9b — the near-miss residues that must NOT error
// ---------------------------------------------------------------------------

#[test]
fn err_e9_betagamma_block_size_zero_is_valid() {
    let l = libs();
    // param1 % 10 == -5 => block_size == 0 => calloc(0,4) succeeds.
    // param1 % 10 in {-4..-1} => block_size 1..4.
    for r in -5i32..=-1 {
        for k in 0..20 {
            let p1 = r - 10 * k;
            assert_eq!(p1 % 10, r);
            unsafe {
                let cv = (l.c.betagamma)(p1, 0, 0, 0);
                let rv = (l.rs.betagamma)(p1, 0, 0, 0);
                assert_ne!(
                    cv, -1,
                    "C betagamma({}, 0,0,0) must NOT take the error path (block_size={})",
                    p1,
                    r + 5
                );
                assert_ne!(rv, -1, "Rust betagamma({}, 0,0,0) must NOT error", p1);
            }
        }
    }
    // And the two libraries agree on the actual values, out of process.
    let mut ops = Vec::new();
    for r in -5i32..=-1 {
        for k in 0..20 {
            ops.push(Op::Betagamma(r - 10 * k, 3, -4, 5));
        }
    }
    oracle::assert_same("e9", &ops);
}

// ---------------------------------------------------------------------------
// E10 — INT_MIN: INT_MIN % 10 == -8 => block_size == -3 => -1
// ---------------------------------------------------------------------------

#[test]
fn err_e10_betagamma_int_min() {
    let l = libs();
    assert_eq!(i32::MIN % 10, -8, "C semantics: truncation toward zero");
    for &p1 in &[i32::MIN, i32::MIN + 10, i32::MIN + 20] {
        for &p2 in &[0i32, i32::MIN, i32::MAX] {
            unsafe {
                let cv = (l.c.betagamma)(p1, p2, p2, p2);
                let rv = (l.rs.betagamma)(p1, p2, p2, p2);
                assert_eq!(cv, -1, "C betagamma({}, ..)", p1);
                assert_eq!(rv, -1, "Rust betagamma({}, ..)", p1);
            }
        }
    }
    // One step away from INT_MIN in each direction changes the residue and
    // therefore the outcome; both libraries must agree on that too.
    let ops: Vec<Op> = (0..12)
        .map(|k| Op::Betagamma(i32::MIN + k, 1, 2, 3))
        .collect();
    oracle::assert_same("e10", &ops);
}

// ---------------------------------------------------------------------------
// E11 — INT_MAX / INT_MIN extremes on the non-erroring path (signed wrap)
// ---------------------------------------------------------------------------

#[test]
fn err_e11_betagamma_int_extremes() {
    assert_eq!(i32::MAX % 10, 7, "block_size == 12, valid");
    let ex = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0, -1, 1];
    let mut ops = Vec::new();
    for &a in &ex {
        for &b in &ex {
            for &c in &ex {
                for &d in &ex {
                    ops.push(Op::Betagamma(a, b, c, d));
                }
            }
        }
    }
    oracle::assert_same("e11", &ops);
}

// ---------------------------------------------------------------------------
// E12 — compute_hash(mb, mb): both comparisons equal => 0
// ---------------------------------------------------------------------------

#[test]
fn err_e12_compute_hash_identical() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    let mut ints: Vec<c_int> = vec![0; 8];
    let mut mb = MemoryBlock {
        data: ints.as_mut_ptr(),
        size: 8,
    };
    let p = &mut mb as *mut MemoryBlock;
    for _ in 0..500 {
        unsafe {
            (*p).size = rng.range_usize(0, 1000);
            diff_compute_hash("E12/stack", p, p, Some(0));
        }
    }
    // Also with library-owned blocks.
    unsafe {
        for _ in 0..100 {
            let c = (l.c.allocate_block)(rng.range_usize(0, 16), rng.i32());
            let r = (l.rs.allocate_block)(rng.range_usize(0, 16), rng.i32());
            assert_eq!((l.c.compute_hash)(c, c), 0);
            assert_eq!((l.rs.compute_hash)(c, c), 0);
            assert_eq!((l.c.compute_hash)(r, r), 0);
            assert_eq!((l.rs.compute_hash)(r, r), 0);
            (l.c.free_block)(c);
            (l.rs.free_block)(r);
        }
    }
}

// ---------------------------------------------------------------------------
// E13 — compute_hash with NULL `data` in one or both operands
// ---------------------------------------------------------------------------

#[test]
fn err_e13_compute_hash_null_data() {
    let mut arena: Vec<MemoryBlock> = vec![
        MemoryBlock {
            data: std::ptr::null_mut(),
            size: 0
        };
        2
    ];
    let lo = arena.as_mut_ptr();
    let hi = unsafe { arena.as_mut_ptr().add(1) };
    assert!(lo < hi);
    let mut ints: Vec<c_int> = vec![0; 2];
    let real = ints.as_mut_ptr();
    let null = std::ptr::null_mut::<c_int>();
    assert!(null < real, "NULL sorts below any real heap address");

    // (data1, data2, expected data term)
    let cases: [(*mut c_int, *mut c_int, c_int); 4] = [
        (null, null, 0),
        (null, real, 100),
        (real, null, 200),
        (real, real, 0),
    ];
    for (d1, d2, dterm) in cases {
        unsafe {
            // mb1 = lo, mb2 = hi  => struct term 10
            (*lo).data = d1;
            (*hi).data = d2;
            diff_compute_hash("E13 lo,hi", lo, hi, Some(dterm + 10));
            // reversed => struct term 20 and the data term flips
            let flipped = match dterm {
                100 => 200,
                200 => 100,
                x => x,
            };
            diff_compute_hash("E13 hi,lo", hi, lo, Some(flipped + 20));
        }
    }
}

// ---------------------------------------------------------------------------
// E14 — reversed argument order
// ---------------------------------------------------------------------------

#[test]
fn err_e14_compute_hash_reversed() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut arena: Vec<MemoryBlock> = vec![
        MemoryBlock {
            data: std::ptr::null_mut(),
            size: 0
        };
        8
    ];
    let mut ints: Vec<c_int> = vec![0; 8];
    let base = arena.as_mut_ptr();
    let ibase = ints.as_mut_ptr();
    for _ in 0..1000 {
        let (ia, ib) = (rng.range_usize(0, 7), rng.range_usize(0, 7));
        let (da, db) = (rng.range_usize(0, 7), rng.range_usize(0, 7));
        unsafe {
            let a = base.add(ia);
            let b = base.add(ib);
            (*a).data = ibase.add(da);
            (*b).data = ibase.add(db);
            let fwd = diff_compute_hash("E14 fwd", a, b, None);
            let d1 = (*a).data;
            let d2 = (*b).data;
            (*a).data = d1;
            (*b).data = d2;
            let rev = diff_compute_hash("E14 rev", b, a, None);
            // Reversal maps 100<->200 and 10<->20, 0 stays 0.
            let expect_rev = {
                let d = fwd / 100 * 100;
                let s = fwd % 100;
                let d2 = match d {
                    100 => 200,
                    200 => 100,
                    x => x,
                };
                let s2 = match s {
                    10 => 20,
                    20 => 10,
                    x => x,
                };
                d2 + s2
            };
            assert_eq!(rev, expect_rev, "E14 reversal invariant (fwd={})", fwd);
        }
    }
}

// ---------------------------------------------------------------------------
// E15 — create_block name length boundaries (0/1/30/31; 31 is the last length
// that fits with its NUL, so it is the step just before overflow)
// ---------------------------------------------------------------------------

#[test]
fn err_e15_create_block_exact_31_and_boundary() {
    for len in [0usize, 1, 2, 29, 30, 31] {
        let name: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        for flags in [0u8, 1, 0x80, 0xFF] {
            for id in [0i32, -1, i32::MIN, i32::MAX] {
                diff_create_block(&format!("E15 len={}", len), id, &name, flags);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E16 — empty name
// ---------------------------------------------------------------------------

#[test]
fn err_e16_create_block_empty_name() {
    let l = libs();
    unsafe {
        for id in [0i32, 1, -1, i32::MIN, i32::MAX] {
            for flags in [0u8, 0x55, 0xAA, 0xFF] {
                let c = (l.c.create_block)(id, b"\0".as_ptr(), flags);
                let r = (l.rs.create_block)(id, b"\0".as_ptr(), flags);
                assert_datablock_eq("E16", &c, &r, 1);
                assert_eq!(c.name[0], 0, "C name[0]");
                assert_eq!(r.name[0], 0, "Rust name[0]");
                assert_eq!(c.id, id);
                assert_eq!(r.id, id);
                assert_eq!(c.flags, flags);
                assert_eq!(r.flags, flags);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E17 — the entire `flags` domain, including values that mean nothing to the
// code.  This is the closest analogue of an out-of-range enum in this API:
// `uint8_t` accepts any int, and every one of the 256 values is exercised.
// ---------------------------------------------------------------------------

#[test]
fn err_e17_create_block_flag_range() {
    let l = libs();
    unsafe {
        for f in 0u16..=255 {
            let flags = f as u8;
            let c = (l.c.create_block)(7, b"f\0".as_ptr(), flags);
            let r = (l.rs.create_block)(7, b"f\0".as_ptr(), flags);
            assert_datablock_eq(&format!("E17 flags={}", flags), &c, &r, 2);
            assert_eq!(c.flags, flags, "C stores flags verbatim");
            assert_eq!(r.flags, flags, "Rust stores flags verbatim");
        }
    }
    // Out-of-range ints truncated into the uint8_t parameter, as a C caller
    // passing an out-of-range enum constant would produce.
    for v in [256i32, 257, -1, -128, 1000, i32::MIN, i32::MAX] {
        let flags = v as u8;
        diff_create_block(&format!("E17 trunc {}", v), 1, b"t", flags);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep: zero / oversized lengths and one-past-range
// values for every entry point that takes a size.
// ---------------------------------------------------------------------------

#[test]
fn err_generic_boundary_sweep() {
    let l = libs();
    // Just below / at / just above the largest allocatable-looking counts.
    let counts: [usize; 12] = [
        0,
        1,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 4,
        usize::MAX / 4 + 1,
        usize::MAX / 4 - 1,
        1usize << 61,
        1usize << 62,
        1usize << 63,
        (1usize << 63) + 4,
        usize::MAX / 2,
    ];
    for &count in &counts {
        unsafe {
            let c = (l.c.allocate_block)(count, 1);
            let r = (l.rs.allocate_block)(count, 1);
            assert_eq!(
                c.is_null(),
                r.is_null(),
                "allocate_block({}) NULL-ness: C={} Rust={}",
                count,
                c.is_null(),
                r.is_null()
            );
            if !c.is_null() {
                assert_eq!((*c).size, count);
                assert_eq!((*r).size, count);
            }
            (l.c.free_block)(c);
            (l.rs.free_block)(r);
        }
    }

    // betagamma over every residue plus the int extremes: identical sentinel or
    // identical value, decided out of process.
    let ops: Vec<Op> = (-25i32..=25)
        .flat_map(|p1| {
            [
                Op::Betagamma(p1, 0, 0, 0),
                Op::Betagamma(p1, i32::MAX, i32::MIN, 0),
            ]
        })
        .collect();
    oracle::assert_same("generic", &ops);
}

/// Child process entry point used by `common::oracle`.
#[test]
fn zz_oracle_child() {
    common::oracle::maybe_run_as_child();
}

//! Phase C -- error-path differential tests, one per row of `ERRORS.md`.
//!
//! Rows 12-14 and 16 (the unchecked-pointer faults) cannot be asserted inside a
//! test process, so they live in `tests/driver.rs`, which forks a child per `.so`
//! and compares the *termination signal* of the two libraries. Row 1 (a failing
//! `malloc(16)`) is not reachable from a harness on this platform and is
//! documented rather than tested.

mod common;

use common::*;
use std::ffi::c_char;

fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

/// Both libraries must agree on whether `allocate_block` rejects `count`.
fn assert_alloc_rejects(c: &Api, r: &Api, count: usize) {
    unsafe {
        let cm = (c.allocate_block)(count, 7);
        let rm = (r.allocate_block)(count, 7);
        let cnull = cm.is_null();
        let rnull = rm.is_null();
        if !cnull {
            (c.free_block)(cm);
        }
        if !rnull {
            (r.free_block)(rm);
        }
        assert!(
            cnull,
            "C allocate_block({count:#x}, 7) unexpectedly succeeded; the test's \
             assumption about calloc failing is wrong"
        );
        assert_eq!(
            cnull, rnull,
            "allocate_block({count:#x}): C returned {}, Rust returned {}",
            if cnull { "NULL" } else { "non-NULL" },
            if rnull { "NULL" } else { "non-NULL" }
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 -- calloc fails because count is SIZE_MAX
// ---------------------------------------------------------------------------

#[test]
fn err02_allocate_block_count_size_max() {
    let (c, r) = load_both();
    assert_alloc_rejects(&c, &r, usize::MAX);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 -- count * sizeof(int) overflows size_t
// ---------------------------------------------------------------------------

#[test]
fn err03_allocate_block_count_times_four_overflows() {
    let (c, r) = load_both();
    // 2^62 * 4 == 2^64 -> exact wrap to 0 if unchecked.
    assert_alloc_rejects(&c, &r, usize::MAX / 4 + 1);
    assert_alloc_rejects(&c, &r, usize::MAX / 2);
    assert_alloc_rejects(&c, &r, usize::MAX - 1);
    // One step past the largest count whose byte size still fits in size_t.
    assert_alloc_rejects(&c, &r, (usize::MAX / 4) + 2);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 -- request merely too large to satisfy (no overflow)
// ---------------------------------------------------------------------------

#[test]
fn err04_allocate_block_count_too_large_to_satisfy() {
    let (c, r) = load_both();
    assert_alloc_rejects(&c, &r, 1usize << 62);
    assert_alloc_rejects(&c, &r, usize::MAX / 8);
    assert_alloc_rejects(&c, &r, 1usize << 50);
    assert_alloc_rejects(&c, &r, 1usize << 46);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 -- count == 0 is NOT an error
// ---------------------------------------------------------------------------

#[test]
fn err05_allocate_block_zero_count_is_not_an_error() {
    let (c, r) = load_both();
    unsafe {
        for init in [0i32, -1, i32::MAX, i32::MIN] {
            let cm = (c.allocate_block)(0, init);
            let rm = (r.allocate_block)(0, init);
            assert!(!cm.is_null(), "C: calloc(0,4) must return non-NULL");
            assert!(!rm.is_null(), "Rust: calloc(0,4) must return non-NULL");
            assert_eq!((*cm).size, 0);
            assert_eq!((*rm).size, 0);
            assert!(!(*cm).data.is_null(), "C: data must be non-NULL for count 0");
            assert!(
                !(*rm).data.is_null(),
                "Rust: data must be non-NULL for count 0"
            );
            (c.free_block)(cm);
            (r.free_block)(rm);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 -- free_block(NULL) is a guarded no-op
// ---------------------------------------------------------------------------

#[test]
fn err06_free_block_null_is_noop() {
    let (c, r) = load_both();
    unsafe {
        for _ in 0..64 {
            (c.free_block)(std::ptr::null_mut());
            (r.free_block)(std::ptr::null_mut());
        }
    }
    // Surviving to here with no fault in either library is the assertion.
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 -- free_block with mb->data == NULL skips the inner free
// ---------------------------------------------------------------------------

#[test]
fn err07_free_block_null_data_skips_inner_free() {
    let (c, r) = load_both();
    unsafe {
        for (api, tag) in [(&c, "C"), (&r, "Rust")] {
            for _ in 0..32 {
                // Reuse allocate_block(0, _) as an owner struct, hand its inner
                // pointer to a throwaway free_block, then clear it.
                let mb = (api.allocate_block)(0, 0);
                assert!(!mb.is_null(), "{tag}");
                let carrier = (api.allocate_block)(0, 0);
                assert!(!carrier.is_null(), "{tag}");
                (*carrier).data = (*mb).data;
                (api.free_block)(carrier); // frees mb's array + carrier
                (*mb).data = std::ptr::null_mut();
                (api.free_block)(mb); // must take the `if (mb->data)`-false path
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 8 & 9 -- betagamma returns -1 for negative block_size
// ---------------------------------------------------------------------------

/// C `%` truncates toward zero, so `param1 % 10` is negative for negative
/// `param1`; `(param1 % 10) + 5 < 0` iff the residue is -6..-9.
fn expects_minus_one(param1: i32) -> bool {
    param1 % 10 + 5 < 0
}

#[test]
fn err08_betagamma_negative_block_size_returns_minus_one() {
    let (c, r) = load_both();
    let mut checked = 0usize;
    unsafe {
        for p1 in -400i32..=0 {
            if !expects_minus_one(p1) {
                continue;
            }
            for &(p2, p3, p4) in &[(0, 0, 0), (1, 2, 3), (-9, 8, -7), (i32::MAX, i32::MIN, 1)] {
                let cv = (c.betagamma)(p1, p2, p3, p4);
                let rv = (r.betagamma)(p1, p2, p3, p4);
                assert_eq!(cv, -1, "C betagamma({p1},{p2},{p3},{p4}) must be -1");
                assert_eq!(
                    rv, cv,
                    "Rust betagamma({p1},{p2},{p3},{p4}) = {rv}, C = {cv}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 4 * 4 * 40, "only {checked} cases checked");
}

#[test]
fn err09_betagamma_int_min_returns_minus_one() {
    let (c, r) = load_both();
    unsafe {
        // INT_MIN % 10 == -8 -> block_size == (size_t)(-3)
        for p1 in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3] {
            let expect_err = expects_minus_one(p1);
            let cv = (c.betagamma)(p1, 5, 6, 7);
            let rv = (r.betagamma)(p1, 5, 6, 7);
            if expect_err {
                assert_eq!(cv, -1, "C betagamma({p1},5,6,7)");
                assert_eq!(rv, -1, "Rust betagamma({p1},5,6,7)");
            } else {
                assert_ne!(cv, -1, "C betagamma({p1},5,6,7) must not error");
                assert_ne!(rv, -1, "Rust betagamma({p1},5,6,7) must not error");
            }
        }
        // INT_MIN itself: residue -8, so 5 + (-8) == -3 -> error.
        assert_eq!((c.betagamma)(i32::MIN, 0, 0, 0), -1);
        assert_eq!((r.betagamma)(i32::MIN, 0, 0, 0), -1);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 10 & 11 -- boundaries one step off the erroring residues
// ---------------------------------------------------------------------------

#[test]
fn err10_11_betagamma_boundary_residues_do_not_error() {
    let (c, r) = load_both();
    unsafe {
        // residue -5 -> block_size 0; residues -4..-1 -> block_size 1..4.
        for p1 in [-5i32, -4, -3, -2, -1, -15, -14, -13, -12, -11, -1005, -1001] {
            assert!(!expects_minus_one(p1), "p1={p1} misclassified by the test");
            for &(p2, p3, p4) in &[(0, 0, 0), (1, 2, 3), (-9, 8, -7)] {
                let cv = (c.betagamma)(p1, p2, p3, p4);
                let rv = (r.betagamma)(p1, p2, p3, p4);
                assert_ne!(cv, -1, "C betagamma({p1},{p2},{p3},{p4}) must not error");
                assert_ne!(rv, -1, "Rust betagamma({p1},{p2},{p3},{p4}) must not error");
            }
        }
        // Exhaustively: agreement on *whether* -1 is returned, over a wide sweep.
        for p1 in -2000i32..=2000 {
            let cv = (c.betagamma)(p1, 1, 1, 1);
            let rv = (r.betagamma)(p1, 1, 1, 1);
            assert_eq!(
                cv == -1,
                rv == -1,
                "error-path disagreement at param1={p1}: C={cv} Rust={rv}"
            );
            assert_eq!(
                cv == -1,
                expects_minus_one(p1),
                "C disagrees with the ERRORS.md rule at param1={p1}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 15 -- compute_hash aliasing falls through both chains
// ---------------------------------------------------------------------------

#[test]
fn err15_compute_hash_aliased_returns_zero() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x15);
    unsafe {
        for _ in 0..500 {
            let mut only = MemoryBlock {
                data: rng.next_u64() as usize as *mut i32,
                size: rng.next_u64() as usize,
            };
            let p: *mut MemoryBlock = &mut only;
            let cv = (c.compute_hash)(p, p);
            let rv = (r.compute_hash)(p, p);
            assert_eq!(cv, 0, "C compute_hash(p, p) must be 0");
            assert_eq!(rv, 0, "Rust compute_hash(p, p) must be 0");
        }
        // data == NULL in both (aliased and non-aliased).
        let mut a = MemoryBlock {
            data: std::ptr::null_mut(),
            size: 0,
        };
        let mut b = MemoryBlock {
            data: std::ptr::null_mut(),
            size: 0,
        };
        let pa: *mut MemoryBlock = &mut a;
        let pb: *mut MemoryBlock = &mut b;
        assert_eq!((c.compute_hash)(pa, pb), (r.compute_hash)(pa, pb));
        assert_eq!((c.compute_hash)(pb, pa), (r.compute_hash)(pb, pa));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 17 -- name length boundary for create_block's strcpy
// ---------------------------------------------------------------------------

#[test]
fn err17_create_block_name_length_boundary() {
    let (c, r) = load_both();
    unsafe {
        // 0..=31 are in bounds (31 chars + NUL exactly fills char[32]).
        for len in 0..=31usize {
            let name: Vec<u8> = (0..len).map(|i| b'A' + (i as u8 % 26)).collect();
            let s = cstr(&name);
            let cb = (c.create_block)(0x5A5A5A5A, s.as_ptr(), 0x3C);
            let rb = (r.create_block)(0x5A5A5A5A, s.as_ptr(), 0x3C);
            assert_eq!(
                defined_bytes(&cb, len),
                defined_bytes(&rb, len),
                "create_block with name length {len}"
            );
            // Verify the NUL landed where C's strcpy would put it.
            assert_eq!(cb.name[len], 0);
            assert_eq!(rb.name[len], 0);
        }
        // len == 32 would write name[32], i.e. into `flags`/padding -> UB.
        // Not asserted; see ERRORS.md row 17.
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 18 -- there is no other rejection; the "invalid value" surface
// is just arbitrary bit patterns, which are all legal inputs here.
// ---------------------------------------------------------------------------

#[test]
fn err18_arbitrary_bit_patterns_are_never_rejected_except_by_row_8() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x18);
    unsafe {
        for _ in 0..4000 {
            let p1 = rng.interesting_i32();
            let p2 = rng.interesting_i32();
            let p3 = rng.interesting_i32();
            let p4 = rng.interesting_i32();
            let cv = (c.betagamma)(p1, p2, p3, p4);
            let rv = (r.betagamma)(p1, p2, p3, p4);
            assert_eq!(
                cv == -1,
                expects_minus_one(p1),
                "C rejected/accepted unexpectedly: betagamma({p1},{p2},{p3},{p4}) = {cv}"
            );
            assert_eq!(
                rv == -1,
                cv == -1,
                "rejection disagreement: betagamma({p1},{p2},{p3},{p4}) C={cv} Rust={rv}"
            );
        }
        // create_block's `flags` is a uint8_t with no valid-variant set: every
        // one of the 256 values is legal and must behave identically. This is the
        // out-of-range-"enum" analogue for this API.
        let s = cstr(b"probe");
        for flags in 0u8..=255 {
            let cb = (c.create_block)(-1, s.as_ptr(), flags);
            let rb = (r.create_block)(-1, s.as_ptr(), flags);
            assert_eq!(cb.flags, flags);
            assert_eq!(rb.flags, flags);
            assert_eq!(defined_bytes(&cb, 5), defined_bytes(&rb, 5));
        }
        // allocate_block's `count` is a size_t. Sweep the exponent range so every
        // "one step past" boundary is hit. The two libraries are driven
        // *sequentially* (allocate -> inspect -> free) because `allocate_block`
        // writes every element, so holding both libraries' blocks live at once
        // would be a multi-gigabyte test-harness artifact rather than a property
        // of the code.
        //
        // Bits 25..=54 are skipped deliberately: in that band whether `calloc`
        // succeeds depends on the host's free memory and overcommit policy, not on
        // anything either library does differently, so an assertion there would
        // test the machine. The size_t-boundary behaviour itself is covered
        // exactly by err02/err03/err04.
        let sweep = (0u32..=24).chain(55u32..64);
        for bit in sweep {
            let count = 1usize << bit;
            let probe = |api: &Api| -> (bool, Option<(usize, Vec<(usize, i32)>)>) {
                let m = (api.allocate_block)(count, 3);
                if m.is_null() {
                    return (true, None);
                }
                let s = read_block_sampled(m, 2048);
                (api.free_block)(m);
                (false, s)
            };
            let (cnull, cvals) = probe(&c);
            let (rnull, rvals) = probe(&r);
            assert_eq!(
                cnull, rnull,
                "allocate_block(1<<{bit}) null-ness disagreement (C null={cnull}, Rust null={rnull})"
            );
            if !cnull {
                let cv = cvals.unwrap();
                let rv = rvals.unwrap();
                assert_eq!(cv.0, rv.0, "size differs for count 1<<{bit}");
                assert_eq!(cv.1, rv.1, "contents differ for count 1<<{bit}");
            }
        }
    }
}

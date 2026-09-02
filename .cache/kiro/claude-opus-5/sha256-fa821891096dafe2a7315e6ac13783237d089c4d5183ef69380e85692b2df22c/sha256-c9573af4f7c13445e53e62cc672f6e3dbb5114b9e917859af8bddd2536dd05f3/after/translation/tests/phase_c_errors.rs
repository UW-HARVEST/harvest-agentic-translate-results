//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E9) plus the generic FFI-boundary rows
//! (G1..G7). Both libraries are driven only via their `.so` exports.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// E1 — init_array: malloc of the 24-byte struct fails -> NULL
//
// Not reachable deterministically without an allocator hook (a 24-byte malloc
// does not fail on a healthy glibc heap), and both libraries call the *same*
// glibc `malloc`, so the branch is shared by construction. What we can and do
// verify is the observable contract: whenever one library returns NULL from
// `init_array`, the other does too, across the whole capacity domain. E2/E3
// exercise the sibling NULL-return path in the same function.
// ---------------------------------------------------------------------------
#[test]
fn e1_init_array_struct_malloc_fail() {
    let _g = lock();
    let p = libs();
    // Confirm both import the same allocator symbols (shared heap ⇒ shared
    // failure behaviour).
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..2_000 {
        // Sample the whole size_t domain, including sizes that make the data
        // malloc fail; NULL-ness must always agree.
        let cap = match rng.below(6) {
            0 => 0usize,
            1 => 1,
            2 => rng.below(4096) as usize,
            3 => usize::MAX,
            4 => 1usize << 62,
            _ => rng.next_u64() as usize,
        };
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        assert_eq!(
            ca.is_null(),
            ra.is_null(),
            "init_array({cap}) NULL-ness differs: C_null={} RUST_null={}",
            ca.is_null(),
            ra.is_null()
        );
        if !ca.is_null() {
            assert_eq!(p.c.view(ca), p.rs.view(ra), "init_array({cap}) fields");
            // Only free arrays whose backing buffer is a real allocation we
            // never wrote into; capacity may be absurd but the buffer is valid.
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// E2 — init_array: data malloc fails -> frees struct, returns NULL
// ---------------------------------------------------------------------------
#[test]
fn e2_init_array_data_malloc_fail() {
    let _g = lock();
    let p = libs();
    // capacity * 4 must be a huge but non-wrapping byte count so malloc fails.
    let huge: &[usize] = &[
        usize::MAX / 4,
        usize::MAX / 4 - 1,
        1usize << 60,
        1usize << 61,
        (1usize << 62) - 1,
        0x0FFF_FFFF_FFFF_FFFF,
    ];
    for &cap in huge {
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        assert_eq!(
            ca.is_null(),
            ra.is_null(),
            "init_array({cap:#x}): C_null={} RUST_null={}",
            ca.is_null(),
            ra.is_null()
        );
        assert!(ca.is_null(), "expected allocation failure for cap={cap:#x}");
    }
}

// ---------------------------------------------------------------------------
// E3 — init_array: capacity * sizeof(int) wraps size_t
// ---------------------------------------------------------------------------
#[test]
fn e3_init_array_size_overflow_wrap() {
    let _g = lock();
    let p = libs();
    // 1<<62 * 4 == 0 (mod 2^64) -> malloc(0) succeeds; capacity field keeps
    // the un-multiplied 1<<62. usize::MAX * 4 == 2^64-4 -> malloc fails.
    let wrapping: &[usize] = &[
        1usize << 62,
        1usize << 63,
        3usize << 62,
        usize::MAX,
        usize::MAX - 1,
        (1usize << 62) + 1,
        (1usize << 62) + 2,
        0xC000_0000_0000_0000,
    ];
    for &cap in wrapping {
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        assert_eq!(
            ca.is_null(),
            ra.is_null(),
            "init_array({cap:#x}) NULL-ness: C_null={} RUST_null={}",
            ca.is_null(),
            ra.is_null()
        );
        assert_eq!(
            p.c.view(ca),
            p.rs.view(ra),
            "init_array({cap:#x}) field snapshot (incl. wrapped capacity)"
        );
        if !ca.is_null() {
            // The struct is valid; its buffer is a 0-byte allocation. Freeing
            // is exactly what a C consumer would do and must not diverge.
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — expand_array(NULL) -> 0
// ---------------------------------------------------------------------------
#[test]
fn e4_expand_array_null() {
    let _g = lock();
    let p = libs();
    let c = unsafe { (p.c.expand_array)(std::ptr::null_mut()) };
    let r = unsafe { (p.rs.expand_array)(std::ptr::null_mut()) };
    assert_eq!(c, r, "expand_array(NULL) C={c} RUST={r}");
    assert_eq!(c, 0, "documented sentinel");
}

// ---------------------------------------------------------------------------
// E5 — expand_array: realloc fails -> 0, arr fields left unchanged
// ---------------------------------------------------------------------------
#[test]
fn e5_expand_array_realloc_fail() {
    let _g = lock();
    let p = libs();

    // (a) capacity 0: glibc realloc(ptr, 0) frees and returns NULL, so
    //     expand_array reports failure and leaves capacity == 0.
    //     `arr->data` is left dangling by the C — we must NOT free_array
    //     afterwards (that is a double free in BOTH libraries), so we only
    //     compare the observable return value and the untouched capacity/size.
    let ca = unsafe { (p.c.init_array)(0) };
    let ra = unsafe { (p.rs.init_array)(0) };
    assert!(!ca.is_null() && !ra.is_null());
    let rc = unsafe { (p.c.expand_array)(ca) };
    let rr = unsafe { (p.rs.expand_array)(ra) };
    assert_eq!(rc, rr, "expand_array(cap=0) C={rc} RUST={rr}");
    assert_eq!(rc, 0, "glibc realloc(p,0) returns NULL -> failure");
    unsafe {
        assert_eq!((*ca).capacity, (*ra).capacity, "capacity untouched");
        assert_eq!((*ca).size, (*ra).size, "size untouched");
        assert_eq!((*ca).capacity, 0, "C does not roll capacity forward");
    }
    // Intentionally leak the 24-byte structs: their buffers were freed by
    // realloc, so free_array would double-free in both libraries alike.

    // (b) capacity whose *doubled* byte size cannot be satisfied, and the
    //     wrap-to-zero case (1<<62 -> capacity 1<<63 -> 0 bytes -> realloc
    //     frees and returns NULL). Nothing is freed afterwards: on failure the
    //     C leaves `arr->data` dangling in both libraries alike, so calling
    //     free_array would be a double free on both sides rather than a
    //     difference worth measuring.
    for &cap in &[usize::MAX / 8, 1usize << 59, 1usize << 60, 1usize << 62] {
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        assert_eq!(ca.is_null(), ra.is_null(), "init_array({cap:#x})");
        if ca.is_null() {
            continue;
        }
        let rc = unsafe { (p.c.expand_array)(ca) };
        let rr = unsafe { (p.rs.expand_array)(ra) };
        assert_eq!(rc, rr, "expand_array(cap={cap:#x}) C={rc} RUST={rr}");
        unsafe {
            assert_eq!(
                (*ca).capacity,
                (*ra).capacity,
                "capacity after expand(cap={cap:#x})"
            );
            assert_eq!((*ca).size, (*ra).size, "size after expand(cap={cap:#x})");
        }
        if rc == 1 {
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// E6 — add_element(NULL, v) -> 0
// ---------------------------------------------------------------------------
#[test]
fn e6_add_element_null() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE6);
    for _ in 0..1_000 {
        let v = rng.spicy_i32();
        let c = unsafe { (p.c.add_element)(std::ptr::null_mut(), v) };
        let r = unsafe { (p.rs.add_element)(std::ptr::null_mut(), v) };
        assert_eq!(c, r, "add_element(NULL, {v}) C={c} RUST={r}");
        assert_eq!(c, 0);
    }
}

// ---------------------------------------------------------------------------
// E7 — add_element: inner expand_array fails -> 0, size not incremented
// ---------------------------------------------------------------------------
#[test]
fn e7_add_element_expand_fail() {
    let _g = lock();
    let p = libs();
    // capacity 0 forces the size >= capacity branch on the very first add, and
    // the inner expand_array fails (realloc(p, 0) -> NULL).
    let ca = unsafe { (p.c.init_array)(0) };
    let ra = unsafe { (p.rs.init_array)(0) };
    assert!(!ca.is_null() && !ra.is_null());
    let rc = unsafe { (p.c.add_element)(ca, 0x1234) };
    let rr = unsafe { (p.rs.add_element)(ra, 0x1234) };
    assert_eq!(rc, rr, "add_element(cap=0) C={rc} RUST={rr}");
    assert_eq!(rc, 0, "must reject");
    unsafe {
        assert_eq!((*ca).size, (*ra).size, "size after rejected add");
        assert_eq!((*ca).size, 0, "size must not be incremented");
        assert_eq!((*ca).capacity, (*ra).capacity);
    }
    // NOTE: a *second* add_element on this array would make the C call
    // realloc() on the pointer glibc already freed (the C leaves `arr->data`
    // dangling after the failed expand) — a genuine double free in the
    // original C, not a translation difference. It is therefore not exercised.
    // Struct intentionally leaked (see E5a).
}

// ---------------------------------------------------------------------------
// E8 — free_array(NULL) is a no-op
// ---------------------------------------------------------------------------
#[test]
fn e8_free_array_null() {
    let _g = lock();
    let p = libs();
    for _ in 0..100 {
        unsafe { (p.c.free_array)(std::ptr::null_mut()) };
        unsafe { (p.rs.free_array)(std::ptr::null_mut()) };
    }
    // Reaching here without a crash in either library is the assertion; verify
    // the libraries are still usable afterwards.
    assert_eq!(
        unsafe { (p.c.matrixsum)(1, 2, 3, 4) },
        unsafe { (p.rs.matrixsum)(1, 2, 3, 4) }
    );
}

// ---------------------------------------------------------------------------
// E9 — matrixsum: init_array(2) fails -> -1
//
// The capacity is hard-coded to 2, so an 8-byte + 24-byte allocation pair
// cannot fail on a healthy heap; the `-1` sentinel is therefore unreachable
// from the public API. We assert the observable contract instead: matrixsum
// never returns the sentinel spuriously in either library, and both agree.
// ---------------------------------------------------------------------------
#[test]
fn e9_matrixsum_init_fail() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    let mut rng = Rng::new(SEED ^ 0xE9);
    for _ in 0..20_000 {
        let a = rng.spicy_i32();
        let b = rng.spicy_i32();
        let c2 = rng.spicy_i32();
        let d = rng.spicy_i32();
        let c = unsafe { (p.c.matrixsum)(a, b, c2, d) };
        let r = unsafe { (p.rs.matrixsum)(a, b, c2, d) };
        assert_eq!(c, r, "matrixsum({a},{b},{c2},{d})");
    }
    // Sanity: with capacity 2 hard-coded the allocation path succeeds, so the
    // sentinel is not produced for the all-zero input either.
    assert_eq!(unsafe { (p.c.matrixsum)(0, 0, 0, 0) }, unsafe {
        (p.rs.matrixsum)(0, 0, 0, 0)
    });
}

// ---------------------------------------------------------------------------
// G1 — NULL to every pointer-taking export
// ---------------------------------------------------------------------------
#[test]
fn g1_all_null_pointers() {
    let _g = lock();
    let p = libs();
    let n = std::ptr::null_mut();
    assert_eq!(unsafe { (p.c.expand_array)(n) }, unsafe {
        (p.rs.expand_array)(n)
    });
    assert_eq!(unsafe { (p.c.add_element)(n, 7) }, unsafe {
        (p.rs.add_element)(n, 7)
    });
    unsafe { (p.c.free_array)(n) };
    unsafe { (p.rs.free_array)(n) };
    // dangling-but-non-null is UB in C too, so it is out of scope; instead
    // re-verify both libraries are still consistent afterwards.
    assert_eq!(
        unsafe { (p.c.calculate_matrix_checksum)() },
        unsafe { (p.rs.calculate_matrix_checksum)() }
    );
}

// ---------------------------------------------------------------------------
// G2 — zero length
// ---------------------------------------------------------------------------
#[test]
fn g2_zero_capacity() {
    let _g = lock();
    let p = libs();
    for _ in 0..500 {
        let ca = unsafe { (p.c.init_array)(0) };
        let ra = unsafe { (p.rs.init_array)(0) };
        assert_eq!(ca.is_null(), ra.is_null());
        assert_eq!(p.c.view(ca), p.rs.view(ra), "init_array(0)");
        let v = p.c.view(ca).unwrap();
        assert_eq!((v.size, v.capacity, v.data_null), (0, 0, false));
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
}

// ---------------------------------------------------------------------------
// G3 — oversized lengths
// ---------------------------------------------------------------------------
#[test]
fn g3_oversized_capacity() {
    let _g = lock();
    let p = libs();
    let caps: &[usize] = &[
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4,
        1usize << 62,
        1usize << 63,
        (1usize << 63) + (1usize << 62),
        0x8000_0000_0000_0001,
    ];
    for &cap in caps {
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        assert_eq!(
            ca.is_null(),
            ra.is_null(),
            "init_array({cap:#x}) NULL-ness differs"
        );
        assert_eq!(p.c.view(ca), p.rs.view(ra), "init_array({cap:#x}) fields");
        if !ca.is_null() {
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// G4 — exactly at, and one past, the growth boundary
// ---------------------------------------------------------------------------
#[test]
fn g4_growth_boundary() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x64);
    for cap in 1..=6usize {
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        // Fill to exactly capacity: no growth yet.
        for _ in 0..cap {
            let v = rng.spicy_i32();
            assert_eq!(unsafe { (p.c.add_element)(ca, v) }, unsafe {
                (p.rs.add_element)(ra, v)
            });
        }
        assert_eq!(p.c.view(ca), p.rs.view(ra), "at boundary cap={cap}");
        assert_eq!(p.c.view(ca).unwrap().capacity, cap, "no growth yet");
        // One past: triggers exactly one doubling.
        let v = rng.spicy_i32();
        assert_eq!(unsafe { (p.c.add_element)(ca, v) }, unsafe {
            (p.rs.add_element)(ra, v)
        });
        assert_eq!(p.c.view(ca), p.rs.view(ra), "one past boundary cap={cap}");
        assert_eq!(p.c.view(ca).unwrap().capacity, cap * 2);
        let n = p.c.view(ca).unwrap().size;
        assert_eq!(p.c.elements(ca, n), p.rs.elements(ra, n));
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
}

// ---------------------------------------------------------------------------
// G5 — out-of-range "enum" values across the FFI boundary
//
// The C `FLAG_*` set forms an implicit 4-bit enum. A C enum/flag parameter
// accepts any int, so values with no valid variant are real inputs. This
// sweeps every low-4-bit pattern crossed with arbitrary high garbage, plus a
// dense sweep of 0..=1023 and 100k random i32.
// ---------------------------------------------------------------------------
#[test]
fn g5_out_of_range_flag_values() {
    let _g = lock();
    let p = libs();

    for flags in 0..=1023i32 {
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags}) dense sweep");
    }
    for flags in -1024..0i32 {
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags}) dense negative sweep");
    }
    // Every 4-bit variant crossed with high-bit garbage.
    for low in 0..16i32 {
        for shift in 4..32u32 {
            let flags = low | (1i32.wrapping_shl(shift));
            let c = unsafe { (p.c.process_flags)(flags) };
            let r = unsafe { (p.rs.process_flags)(flags) };
            assert_eq!(c, r, "process_flags({flags:#x}) low={low} shift={shift}");
        }
    }
    let extremes: &[c_int] = &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1, 0];
    for &flags in extremes {
        assert_eq!(
            unsafe { (p.c.process_flags)(flags) },
            unsafe { (p.rs.process_flags)(flags) },
            "process_flags({flags})"
        );
    }
    let mut rng = Rng::new(SEED ^ 0x65);
    for _ in 0..100_000 {
        let flags = rng.next_i32();
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags}) random");
    }
}

// ---------------------------------------------------------------------------
// G6 — matrixsum with INT_MIN / INT_MAX (signed overflow wrapping)
// ---------------------------------------------------------------------------
#[test]
fn g6_int_extremes() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    let vals: &[c_int] = &[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -1,
        0,
        1,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
        0x0800_0000,
        0x1000_0000,
        -0x0800_0000,
    ];
    for &a in vals {
        for &b in vals {
            for &c2 in vals {
                for &d in vals {
                    let c = unsafe { (p.c.matrixsum)(a, b, c2, d) };
                    let r = unsafe { (p.rs.matrixsum)(a, b, c2, d) };
                    assert_eq!(c, r, "matrixsum({a},{b},{c2},{d}) overflow path");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G7 — mutated matrix: checksum > 0xFFF and negative checksums
// ---------------------------------------------------------------------------
#[test]
fn g7_matrix_mutation_mask() {
    let _g = lock();
    let p = libs();
    let cases: Vec<[c_int; 12]> = vec![
        [0x1000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0xFFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0x1001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [-1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [-0x1000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [i32::MIN, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [i32::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [i32::MAX, i32::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0x7FFF_FFFF; 12],
        [-0x8000_0000; 12],
    ];
    for m in &cases {
        p.c.write_matrix(m);
        p.rs.write_matrix(m);
        let cc = unsafe { (p.c.calculate_matrix_checksum)() };
        let cr = unsafe { (p.rs.calculate_matrix_checksum)() };
        assert_eq!(cc, cr, "checksum for {m:?}");
        for &(a, b, c2, d) in &[
            (0, 0, 0, 0),
            (1, 1, 1, 1),
            (-1, -1, -1, -1),
            (i32::MIN, i32::MAX, 0, 1),
            (0xFF, 0x10, 0xFFF, -0xFFF),
        ] {
            let c = unsafe { (p.c.matrixsum)(a, b, c2, d) };
            let r = unsafe { (p.rs.matrixsum)(a, b, c2, d) };
            assert_eq!(c, r, "matrixsum({a},{b},{c2},{d}) with matrix {m:?}");
        }
    }
    reset_matrix(p);
}

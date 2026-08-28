//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input/condition, calls BOTH `.so`s
//! and asserts the SAME error code / sentinel is produced (not merely "both
//! failed somehow").

mod common;

use common::{load, make_array, DynamicArray, SEED};
use std::ffi::c_int;
use std::ptr;

// ---------------------------------------------------------------------------
// E1 — init_array: struct malloc fails -> NULL
// ---------------------------------------------------------------------------
// `malloc(sizeof(DynamicArray))` is a 24-byte request; it cannot be made to fail
// through the public ABI without an allocator interposer. What IS verifiable is
// that the C and the Rust take the identical branch structure and share the very
// same allocator (`nm -D` shows both import glibc `malloc`/`realloc`/`free`, see
// SYMBOLS.md), so the failure branch is reached under exactly the same
// conditions. Its *observable effect* — `NULL` out of `init_array` — is what E2
// exercises for real, through the sibling `malloc` on the same code path.
#[test]
fn e1_init_array_struct_alloc_failure() {
    let p = load();
    // Sanity: the 24-byte struct allocation never fails here, so both return
    // non-NULL for a normal capacity, i.e. neither takes the E1 branch.
    unsafe {
        for cap in [1usize, 2, 3, 8] {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            assert!(!a.is_null(), "C took the E1 branch unexpectedly");
            assert!(!b.is_null(), "Rust took the E1 branch unexpectedly");
            assert_eq!(a.is_null(), b.is_null());
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
    // And both agree on the reachable form of the same sentinel (see E2).
    let huge = usize::MAX;
    unsafe {
        assert!(p.c.init_array(huge).is_null());
        assert!(p.rs.init_array(huge).is_null());
    }
}

// ---------------------------------------------------------------------------
// E2 — init_array: data malloc fails -> free(arr); return NULL
// ---------------------------------------------------------------------------
#[test]
fn e2_init_array_data_alloc_failure() {
    let p = load();
    // Capacities whose byte product (size_t, wrapping) is an un-servicable size.
    let caps: [usize; 10] = [
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4,          // *4 == 0xFFFF_FFFF_FFFF_FFFC
        usize::MAX / 4 - 1,
        (1usize << 62) - 1,      // *4 == 0xFFFF_FFFF_FFFF_FFFC
        (1usize << 61) + 1,
        1usize << 60,
        1usize << 55,
        1usize << 50,
    ];
    for cap in caps {
        unsafe {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "init_array({cap:#x}): C null={} Rust null={}",
                a.is_null(),
                b.is_null()
            );
            assert!(a.is_null(), "expected NULL from init_array({cap:#x})");
            if !a.is_null() {
                p.c.free_array(a);
                p.rs.free_array(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E3 — init_array: capacity*sizeof(int) WRAPS to a servicable byte count.
// The C performs no range check, so this must succeed in both.
// ---------------------------------------------------------------------------
#[test]
fn e3_init_array_capacity_wraps_to_zero_bytes() {
    let p = load();
    // 2^62 * 4 == 2^64 == 0 (mod 2^64)  -> malloc(0)  -> succeeds
    // 2^63 * 4 == 2^65 == 0 (mod 2^64)  -> malloc(0)  -> succeeds
    // (2^62+1) * 4 == 4                 -> malloc(4)  -> succeeds
    for cap in [1usize << 62, 1usize << 63, (1usize << 62) + 1, (1usize << 63) + 1] {
        unsafe {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "init_array({cap:#x}) NULL-ness diverged (C null={}, Rust null={})",
                a.is_null(),
                b.is_null()
            );
            assert!(
                !a.is_null(),
                "expected the wrapped byte count to be allocatable for {cap:#x}"
            );
            let ha = p.c.header(a);
            let hb = p.rs.header(b);
            assert_eq!(ha.size, hb.size);
            assert_eq!(ha.capacity, hb.capacity);
            assert_eq!(ha.capacity, cap, "capacity must be echoed unchecked");
            assert_eq!(ha.size, 0);
            assert_eq!(ha.data.is_null(), hb.data.is_null());
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — expand_array(NULL) -> 0
// ---------------------------------------------------------------------------
#[test]
fn e4_expand_array_null() {
    let p = load();
    unsafe {
        let c = p.c.expand_array(ptr::null_mut());
        let r = p.rs.expand_array(ptr::null_mut());
        assert_eq!(c, r, "expand_array(NULL) diverged");
        assert_eq!(c, 0, "expand_array(NULL) must return 0");
        // repeated calls stay stable
        for _ in 0..16 {
            assert_eq!(p.c.expand_array(ptr::null_mut()), p.rs.expand_array(ptr::null_mut()));
        }
    }
}

// ---------------------------------------------------------------------------
// E5 — expand_array: realloc fails -> 0, arr untouched
// ---------------------------------------------------------------------------
#[test]
fn e5_expand_array_realloc_failure() {
    let p = load();
    // capacity*2*4 must be un-servicable but non-zero.
    let caps: [usize; 5] = [
        usize::MAX / 8,      // *2*4 == 0xFFFF_FFFF_FFFF_FFF8
        (1usize << 60) + 1,
        1usize << 59,
        1usize << 55,
        1usize << 50,
    ];
    for cap in caps {
        unsafe {
            // A real 4-int buffer, but a lying (huge) capacity — exactly what the
            // C accepts without validation.
            let a = make_array(4, 0, &[11, 22, 33, 44]);
            let b = make_array(4, 0, &[11, 22, 33, 44]);
            (*a).capacity = cap;
            (*b).capacity = cap;
            let before_a = ptr::read(a);
            let before_b = ptr::read(b);

            let rc = p.c.expand_array(a);
            let rr = p.rs.expand_array(b);
            assert_eq!(rc, rr, "expand_array(cap={cap:#x}) return code diverged");
            assert_eq!(rc, 0, "expected failure for cap={cap:#x}");

            let after_a = ptr::read(a);
            let after_b = ptr::read(b);
            // On failure the C leaves data/capacity alone.
            assert_eq!(after_a.data, before_a.data, "C mutated data on failure");
            assert_eq!(after_b.data, before_b.data, "Rust mutated data on failure");
            assert_eq!(after_a.capacity, cap);
            assert_eq!(after_b.capacity, cap);
            assert_eq!(after_a.size, after_b.size);
            assert_eq!(after_a.capacity, after_b.capacity);
            // data survived: contents intact and identical
            assert_eq!(p.c.elements(a, 4), p.rs.elements(b, 4));
            assert_eq!(p.c.elements(a, 4), vec![11, 22, 33, 44]);

            common::libc_free(after_a.data as *mut std::ffi::c_void);
            common::libc_free(a as *mut std::ffi::c_void);
            common::libc_free(after_b.data as *mut std::ffi::c_void);
            common::libc_free(b as *mut std::ffi::c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// E6 — expand_array with capacity == 0 -> realloc(data, 0) -> 0
// ---------------------------------------------------------------------------
#[test]
fn e6_expand_array_zero_capacity() {
    let p = load();
    unsafe {
        // (a) capacity 0 straight out of init_array(0)
        let a = p.c.init_array(0);
        let b = p.rs.init_array(0);
        assert_eq!(a.is_null(), b.is_null());
        assert!(!a.is_null(), "init_array(0) must succeed (malloc(0) != NULL)");
        assert_eq!(p.c.header(a).capacity, 0);
        assert_eq!(p.rs.header(b).capacity, 0);

        let rc = p.c.expand_array(a);
        let rr = p.rs.expand_array(b);
        assert_eq!(rc, rr, "expand_array(capacity=0) return code diverged");
        assert_eq!(rc, 0, "glibc realloc(p, 0) returns NULL -> expand_array == 0");
        // capacity must be unchanged (the C only assigns it on success)
        let ha = ptr::read(a);
        let hb = ptr::read(b);
        assert_eq!(ha.capacity, hb.capacity);
        assert_eq!(ha.capacity, 0);
        assert_eq!(ha.size, hb.size);
        // arr->data is now dangling in BOTH libraries (realloc freed it and the
        // C never nulls it). Deliberately leak the 24-byte headers rather than
        // provoke the identical double-free in both.

        // (b) capacity forced to 0 on a caller-built struct that also has size 0
        let c1 = make_array(0, 0, &[]);
        let c2 = make_array(0, 0, &[]);
        let r1 = p.c.expand_array(c1);
        let r2 = p.rs.expand_array(c2);
        assert_eq!(r1, r2, "expand_array(caller struct, capacity=0) diverged");
        assert_eq!(r1, 0);
        assert_eq!(ptr::read(c1).capacity, ptr::read(c2).capacity);

        // (c) capacity whose doubling wraps to 0: 2^63 * 2 == 0 (mod 2^64)
        for cap in [1usize << 63, (1usize << 63) + 1] {
            let d1 = make_array(4, 0, &[1, 2, 3, 4]);
            let d2 = make_array(4, 0, &[1, 2, 3, 4]);
            (*d1).capacity = cap;
            (*d2).capacity = cap;
            let x = p.c.expand_array(d1);
            let y = p.rs.expand_array(d2);
            assert_eq!(x, y, "expand_array(cap={cap:#x}) wrap-to-zero diverged");
            assert_eq!(
                ptr::read(d1).capacity,
                ptr::read(d2).capacity,
                "capacity after wrap-to-zero expand diverged"
            );
            assert_eq!(ptr::read(d1).size, ptr::read(d2).size);
            // data may or may not have been freed by realloc(p, 0): leak.
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — add_element(NULL, v) -> 0
// ---------------------------------------------------------------------------
#[test]
fn e7_add_element_null() {
    let p = load();
    let vals: [c_int; 9] = [0, 1, -1, 7, c_int::MIN, c_int::MAX, c_int::MIN + 1, c_int::MAX - 1, 0x5A5A_5A5A];
    unsafe {
        for v in vals {
            let c = p.c.add_element(ptr::null_mut(), v);
            let r = p.rs.add_element(ptr::null_mut(), v);
            assert_eq!(c, r, "add_element(NULL, {v}) diverged");
            assert_eq!(c, 0, "add_element(NULL, {v}) must return 0");
        }
        let mut rng = common::Rng::new(SEED ^ 0xE7);
        for _ in 0..1024 {
            let v = rng.next_i32();
            assert_eq!(
                p.c.add_element(ptr::null_mut(), v),
                p.rs.add_element(ptr::null_mut(), v),
                "add_element(NULL, {v}) diverged"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E8 — add_element: size >= capacity and expand_array fails -> 0, size unchanged
// ---------------------------------------------------------------------------
#[test]
fn e8_add_element_expand_failure() {
    let p = load();
    unsafe {
        // (a) capacity 0 -> expand fails -> add_element returns 0
        let a = p.c.init_array(0);
        let b = p.rs.init_array(0);
        let rc = p.c.add_element(a, 42);
        let rr = p.rs.add_element(b, 42);
        assert_eq!(rc, rr, "add_element on capacity-0 array diverged");
        assert_eq!(rc, 0, "expected 0 (expand_array failed)");
        assert_eq!(ptr::read(a).size, ptr::read(b).size);
        assert_eq!(ptr::read(a).size, 0, "size must not be incremented");
        assert_eq!(ptr::read(a).capacity, ptr::read(b).capacity);
        // data is dangling in both after realloc(p, 0): leak the headers.

        // (b) huge capacity, size == capacity -> expand's realloc fails -> 0
        for cap in [usize::MAX / 8, 1usize << 59, 1usize << 50] {
            let c1 = make_array(4, 0, &[9, 9, 9, 9]);
            let c2 = make_array(4, 0, &[9, 9, 9, 9]);
            (*c1).capacity = cap;
            (*c1).size = cap;
            (*c2).capacity = cap;
            (*c2).size = cap;
            let x = p.c.add_element(c1, -7);
            let y = p.rs.add_element(c2, -7);
            assert_eq!(x, y, "add_element(cap=size={cap:#x}) diverged");
            assert_eq!(x, 0, "expected 0 for cap=size={cap:#x}");
            assert_eq!(ptr::read(c1).size, ptr::read(c2).size);
            assert_eq!(ptr::read(c1).size, cap, "size must not be incremented");
            assert_eq!(ptr::read(c1).capacity, ptr::read(c2).capacity);
            // buffers untouched and identical
            assert_eq!(p.c.elements(c1, 4), p.rs.elements(c2, 4));
            let h1 = ptr::read(c1);
            let h2 = ptr::read(c2);
            common::libc_free(h1.data as *mut std::ffi::c_void);
            common::libc_free(c1 as *mut std::ffi::c_void);
            common::libc_free(h2.data as *mut std::ffi::c_void);
            common::libc_free(c2 as *mut std::ffi::c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// E9 — free_array(NULL) -> no-op, no crash
// ---------------------------------------------------------------------------
#[test]
fn e9_free_array_null() {
    let p = load();
    unsafe {
        for _ in 0..1000 {
            p.c.free_array(ptr::null_mut());
            p.rs.free_array(ptr::null_mut());
        }
    }
    // Both survived, and the library is still functional afterwards.
    assert_eq!(p.c.matrixsum(1, 2, 3, 4), p.rs.matrixsum(1, 2, 3, 4));
}

// ---------------------------------------------------------------------------
// E10 — matrixsum: init_array(2) fails -> -1
// ---------------------------------------------------------------------------
// `init_array(2)` needs 24 + 8 bytes; it cannot be made to fail through the
// public ABI. What IS asserted differentially is that neither implementation
// ever produces the `-1` sentinel for a huge randomized input set, and that
// both agree on every result — i.e. both take the same (non-error) branch.
#[test]
fn e10_matrixsum_alloc_failure() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xE10);
    for _ in 0..4096 {
        let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        let rc = p.c.matrixsum(a, b, c, d);
        let rr = p.rs.matrixsum(a, b, c, d);
        assert_eq!(rc, rr, "matrixsum({a},{b},{c},{d}) diverged");
        // The only way to observe -1 legitimately would be the alloc failure
        // branch; assert that both stay out of it in lockstep.
        assert_eq!(
            rc == -1,
            rr == -1,
            "matrixsum({a},{b},{c},{d}): error-sentinel disagreement"
        );
    }
}

// ---------------------------------------------------------------------------
// G2 — zero length / zero capacity across the whole API
// ---------------------------------------------------------------------------
#[test]
fn g2_zero_capacity_lifecycle() {
    let p = load();
    unsafe {
        // init_array(0): malloc(0) is non-NULL on glibc -> array exists
        let a = p.c.init_array(0);
        let b = p.rs.init_array(0);
        assert_eq!(a.is_null(), b.is_null(), "init_array(0) NULL-ness diverged");
        assert!(!a.is_null());
        let ha = ptr::read(a);
        let hb = ptr::read(b);
        assert_eq!(ha.size, hb.size);
        assert_eq!(ha.capacity, hb.capacity);
        assert_eq!((ha.size, ha.capacity), (0, 0));
        assert_eq!(ha.data.is_null(), hb.data.is_null());
        // free_array on a pristine capacity-0 array is safe in both
        p.c.free_array(a);
        p.rs.free_array(b);

        // and reading 0 elements out of a fresh array is a no-op in both
        let a = p.c.init_array(0);
        let b = p.rs.init_array(0);
        assert_eq!(p.c.elements(a, 0), p.rs.elements(b, 0));
        p.c.free_array(a);
        p.rs.free_array(b);
    }
}

// ---------------------------------------------------------------------------
// G3 — oversized / boundary capacities: full power-of-two sweep
// ---------------------------------------------------------------------------
#[test]
fn g3_capacity_sweep() {
    let p = load();
    let mut caps: Vec<usize> = (0..64).map(|k| 1usize << k).collect();
    caps.extend([
        0,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4,
        usize::MAX / 4 - 1,
        usize::MAX / 4 + 1,
        (1usize << 62) - 1,
        (1usize << 62) + 1,
        3,
        5,
        7,
        1023,
        1025,
    ]);
    // NOTE: allocations are probed SEQUENTIALLY (init -> snapshot -> free) and
    // never held live in both libraries at once. `ulimit -d` (RLIMIT_DATA, 6 GiB
    // here) is a *per-process* budget shared by both `.so`s, so overlapping two
    // multi-gigabyte requests would make the second one fail for a reason that
    // has nothing to do with the translation.
    for cap in caps {
        let a = probe_init(&p.c, cap);
        let b = probe_init(&p.rs, cap);
        assert_eq!(
            a, b,
            "init_array({cap:#x}) diverged: C={a:?} Rust={b:?} (bytes={:#x})",
            cap.wrapping_mul(4)
        );
        if let Some(snap) = a {
            assert_eq!(snap.0, 0, "init_array({cap:#x}) size must be 0");
            assert_eq!(snap.1, cap, "init_array({cap:#x}) capacity must be echoed");
        }
    }
}

/// `init_array(cap)` -> observable `(size, capacity, data_is_null)` -> free.
/// `None` means the C sentinel `NULL` was returned.
fn probe_init(imp: &common::Impl, cap: usize) -> Option<(usize, usize, bool)> {
    unsafe {
        let a = imp.init_array(cap);
        if a.is_null() {
            return None;
        }
        let h = ptr::read(a);
        let snap = (h.size, h.capacity, h.data.is_null());
        imp.free_array(a);
        Some(snap)
    }
}

// ---------------------------------------------------------------------------
// G4 — add_element right at the size/capacity boundary
// ---------------------------------------------------------------------------
#[test]
fn g4_add_element_boundary() {
    let p = load();
    for cap in [1usize, 2, 3, 4, 8] {
        for size in [0usize, 1, 2, 3, 4, 8] {
            if size > cap {
                continue; // would be an out-of-bounds write in the C too
            }
            unsafe {
                let fill: Vec<c_int> = (0..cap as c_int).collect();
                let a = make_array(cap, size, &fill);
                let b = make_array(cap, size, &fill);
                let x = p.c.add_element(a, 0x7EED);
                let y = p.rs.add_element(b, 0x7EED);
                assert_eq!(x, y, "cap={cap} size={size}: return code");
                let ha = ptr::read(a);
                let hb = ptr::read(b);
                assert_eq!(ha.size, hb.size, "cap={cap} size={size}: size");
                assert_eq!(ha.capacity, hb.capacity, "cap={cap} size={size}: capacity");
                assert_eq!(
                    p.c.elements(a, ha.size),
                    p.rs.elements(b, hb.size),
                    "cap={cap} size={size}: buffer"
                );
                p.c.free_array(a);
                p.rs.free_array(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G5 — out-of-range / no-valid-variant "enum" words across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn g5_out_of_range_flag_words() {
    let p = load();
    // The C has no `enum`; the mode is an `int` flag word, so *every* int is a
    // reachable input, including words with no valid flag bit and words with
    // every reserved bit set.
    let mut cases: Vec<c_int> = vec![
        0,
        !0,            // -1: all bits, valid + reserved
        c_int::MIN,    // sign bit only
        c_int::MAX,    // all but sign
        c_int::MIN + 1,
        c_int::MAX - 1,
        0x10,          // first reserved bit only, no valid flag
        0x20,
        0xF0,          // reserved nibble only
        !0xF,          // every reserved bit, no valid flag
        0x7FFF_FFF0,
        -16,
        1 << 30,
        1 << 4,
        0x0BAD_F00D,
        0x1234_5678,
        -0x1234_5678,
    ];
    // one step past each valid single-flag value
    for k in 0..4 {
        cases.push(1 << k);
        cases.push((1 << k) + 1);
        cases.push((1 << k) - 1);
    }
    for f in cases {
        assert_eq!(
            p.c.process_flags(f),
            p.rs.process_flags(f),
            "process_flags({f:#x}) diverged for an out-of-range flag word"
        );
    }
}

#[test]
fn g5_matrixsum_extremes() {
    let p = load();
    // Signed-overflow paths: sum, sum*0x10.
    let ext: [c_int; 10] = [
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX,
        c_int::MAX - 1,
        -1,
        0,
        1,
        0x4000_0000,
        -0x4000_0000,
        0x1000_0000,
    ];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                for &d in &ext {
                    assert_eq!(
                        p.c.matrixsum(a, b, c, d),
                        p.rs.matrixsum(a, b, c, d),
                        "matrixsum({a},{b},{c},{d}) diverged"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G6 — caller-built (unvalidated) DynamicArray
// ---------------------------------------------------------------------------
#[test]
fn g6_caller_built_struct() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC6);
    for _ in 0..256 {
        let cap = 1 + rng.below(8) as usize;
        let size = rng.below(cap as u64 + 1) as usize; // 0..=cap
        let fill: Vec<c_int> = (0..cap).map(|_| rng.spicy_i32()).collect();
        unsafe {
            let a = make_array(cap, size, &fill);
            let b = make_array(cap, size, &fill);
            let v = rng.spicy_i32();
            // random operation on a struct the library never created
            let op = rng.below(3);
            let (x, y) = match op {
                0 => (p.c.add_element(a, v), p.rs.add_element(b, v)),
                1 => (p.c.expand_array(a), p.rs.expand_array(b)),
                _ => {
                    let x = p.c.add_element(a, v);
                    let y = p.rs.add_element(b, v);
                    assert_eq!(x, y, "add rc");
                    (p.c.expand_array(a), p.rs.expand_array(b))
                }
            };
            assert_eq!(x, y, "op={op} cap={cap} size={size}: return code");
            let ha = ptr::read(a);
            let hb = ptr::read(b);
            assert_eq!(ha.size, hb.size, "op={op} cap={cap} size={size}: size");
            assert_eq!(ha.capacity, hb.capacity, "op={op} cap={cap} size={size}: capacity");
            assert_eq!(
                p.c.elements(a, ha.size),
                p.rs.elements(b, hb.size),
                "op={op} cap={cap} size={size}: buffer"
            );
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
}

// ---------------------------------------------------------------------------
// G7 — free_array on an array whose data is NULL (free(NULL) is a no-op)
// ---------------------------------------------------------------------------
#[test]
fn g7_free_array_null_data() {
    let p = load();
    unsafe {
        for _ in 0..64 {
            let a = common::libc_malloc(std::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
            let b = common::libc_malloc(std::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
            ptr::write(
                a,
                DynamicArray { data: ptr::null_mut(), size: 0, capacity: 0 },
            );
            ptr::write(
                b,
                DynamicArray { data: ptr::null_mut(), size: 0, capacity: 0 },
            );
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
    // still functional
    assert_eq!(p.c.matrixsum(1, 1, 1, 1), p.rs.matrixsum(1, 1, 1, 1));
}

// Phase C -- error-path differential tests.
//
// One test per row of ERRORS.md, plus the generic FFI boundary cases (G1-G6).
// Each test constructs the exact invalid input/condition, calls BOTH shared
// objects, and asserts they return the SAME sentinel (not merely "both failed").

mod common;
use common::*;
use std::ffi::c_int;

/// Overwrite a handle's raw fields, to reach states only a caller that pokes the
/// struct directly can produce (the C guards are `if (!arr)` / `size >= capacity`,
/// both reachable this way).
unsafe fn set_fields(h: *mut DynamicArray, data: *mut c_int, size: usize, capacity: usize) {
    (*h).data = data;
    (*h).size = size;
    (*h).capacity = capacity;
}

/// `realloc(p, 0)` frees `p` on glibc, which leaves `arr->data` dangling because
/// the C deliberately does not update it on failure. Neutralise `data` before
/// freeing so the test process does not double-free.
unsafe fn free_after_dangling(api: &Api, h: *mut DynamicArray) {
    (*h).data = std::ptr::null_mut();
    (*h).size = 0;
    (*h).capacity = 0;
    (api.free_array)(h);
}

// Capacities whose `* sizeof(int)` product cannot be satisfied.
const UNSATISFIABLE_CAPS: [usize; 6] = [
    usize::MAX,
    usize::MAX / 2,
    usize::MAX / 4,
    usize::MAX / 8,
    (1usize << 62) - 1,
    1usize << 61,
];

// Capacities whose `* sizeof(int)` product wraps to exactly 0 -> malloc(0).
const WRAP_TO_ZERO_CAPS: [usize; 3] = [1usize << 62, 1usize << 63, 3usize << 62];

// ===========================================================================
// Row 1: init_array -- struct allocation failure (structurally unreachable)
// ===========================================================================

#[test]
fn err01_init_array_struct_alloc_failure_documented() {
    let p = load_pair();
    // `malloc(sizeof(DynamicArray))` is a fixed 24-byte request that cannot be
    // made to fail by any argument, so line 47's `return NULL` is unreachable
    // from the public API. Assert it is never spuriously taken: a small
    // capacity must always yield a non-NULL handle from BOTH libraries.
    for cap in [0usize, 1, 2, 3, 8, 64] {
        unsafe {
            let ch = (p.c.init_array)(cap);
            let rh = (p.r.init_array)(cap);
            assert_same!(
                format!("init_array({cap}) null-ness"),
                ch.is_null(),
                rh.is_null()
            );
            assert!(!ch.is_null(), "C init_array({cap}) unexpectedly failed");
            assert!(!rh.is_null(), "Rust init_array({cap}) unexpectedly failed");
            (p.c.free_array)(ch);
            (p.r.free_array)(rh);
        }
    }
}

// ===========================================================================
// Row 2: init_array -- data allocation failure -> NULL
// ===========================================================================

#[test]
fn err02_init_array_huge_capacity_returns_null() {
    let p = load_pair();
    for &cap in &UNSATISFIABLE_CAPS {
        unsafe {
            let ch = (p.c.init_array)(cap);
            let rh = (p.r.init_array)(cap);
            assert_same!(
                format!("init_array({cap:#x}) null-ness (expect NULL sentinel)"),
                ch.is_null(),
                rh.is_null()
            );
            assert!(
                ch.is_null(),
                "C init_array({cap:#x}) should return the NULL sentinel"
            );
            assert!(
                rh.is_null(),
                "Rust init_array({cap:#x}) should return the NULL sentinel"
            );
        }
    }
}

// ===========================================================================
// Row 3: capacity * sizeof(int) wraps to 0 -> malloc(0) -> NOT an error
// ===========================================================================

#[test]
fn err03_init_array_capacity_product_wraps_to_zero() {
    let p = load_pair();
    for &cap in &WRAP_TO_ZERO_CAPS {
        unsafe {
            let ch = (p.c.init_array)(cap);
            let rh = (p.r.init_array)(cap);
            assert_same!(
                format!("init_array({cap:#x}) null-ness (size_t product wraps to 0)"),
                ch.is_null(),
                rh.is_null()
            );
            if ch.is_null() {
                continue; // whatever malloc(0) does, both agree
            }
            let cs = p.c.read_handle(ch);
            let rs = p.r.read_handle(rh);
            assert_same!(
                format!("init_array({cap:#x}) size"),
                cs.size,
                rs.size
            );
            assert_same!(
                format!("init_array({cap:#x}) capacity"),
                cs.capacity,
                rs.capacity
            );
            assert_eq!(cs.size, 0);
            assert_eq!(
                cs.capacity, cap,
                "capacity must be stored verbatim despite the 0-byte allocation"
            );
            (p.c.free_array)(ch);
            (p.r.free_array)(rh);
        }
    }
}

// ===========================================================================
// Row 4: init_array(0) -> not an error
// ===========================================================================

#[test]
fn err04_init_array_zero_capacity_is_not_an_error() {
    let p = load_pair();
    unsafe {
        let ch = (p.c.init_array)(0);
        let rh = (p.r.init_array)(0);
        assert_same!("init_array(0) null-ness", ch.is_null(), rh.is_null());
        assert!(!ch.is_null(), "glibc malloc(0) returns non-NULL");
        assert_same!("init_array(0) state", p.c.snapshot(ch), p.r.snapshot(rh));
        let cs = p.c.read_handle(ch);
        assert_eq!((cs.size, cs.capacity), (0, 0));
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

// ===========================================================================
// Row 5: expand_array(NULL) -> 0
// ===========================================================================

#[test]
fn err05_expand_array_null_returns_zero() {
    let p = load_pair();
    let cv = unsafe { (p.c.expand_array)(std::ptr::null_mut()) };
    let rv = unsafe { (p.r.expand_array)(std::ptr::null_mut()) };
    assert_same!("expand_array(NULL)", cv, rv);
    assert_eq!(cv, 0, "C expand_array(NULL) must return the 0 sentinel");
}

// ===========================================================================
// Row 6: expand_array -- realloc failure -> 0, struct untouched
// ===========================================================================

#[test]
fn err06_expand_array_realloc_failure_returns_zero() {
    let p = load_pair();
    // capacity/2 values whose doubled byte product is unsatisfiable.
    for &cap in &[usize::MAX / 8, usize::MAX / 4, usize::MAX / 2, 1usize << 60] {
        unsafe {
            let ch = (p.c.init_array)(4);
            let rh = (p.r.init_array)(4);
            assert!(!ch.is_null() && !rh.is_null());
            (p.c.add_element)(ch, 0x1234);
            (p.r.add_element)(rh, 0x1234);

            let c_data = (*ch).data;
            let r_data = (*rh).data;
            (*ch).capacity = cap;
            (*rh).capacity = cap;

            let cv = (p.c.expand_array)(ch);
            let rv = (p.r.expand_array)(rh);
            assert_same!(format!("expand_array rc with capacity {cap:#x}"), cv, rv);
            assert_eq!(cv, 0, "realloc failure must return the 0 sentinel");

            // C updates neither `data` nor `capacity` on failure.
            assert_eq!((*ch).data, c_data, "C data pointer must be unchanged");
            assert_eq!((*rh).data, r_data, "Rust data pointer must be unchanged");
            assert_same!(
                format!("capacity after failed expand ({cap:#x})"),
                (*ch).capacity,
                (*rh).capacity
            );
            assert_eq!((*ch).capacity, cap, "capacity must be unchanged");
            assert_same!("size after failed expand", (*ch).size, (*rh).size);
            assert_eq!(p.c.read_elems(ch), vec![0x1234], "contents intact");
            assert_eq!(p.r.read_elems(rh), vec![0x1234], "contents intact");

            (*ch).capacity = 4;
            (*rh).capacity = 4;
            (p.c.free_array)(ch);
            (p.r.free_array)(rh);
        }
    }
}

// ===========================================================================
// Row 7: expand_array on capacity 0 -> realloc(data, 0) -> 0
// ===========================================================================

#[test]
fn err07_expand_array_zero_capacity_realloc_to_zero() {
    let p = load_pair();
    unsafe {
        let ch = (p.c.init_array)(0);
        let rh = (p.r.init_array)(0);
        assert!(!ch.is_null() && !rh.is_null());
        assert_eq!((*ch).capacity, 0);

        let cv = (p.c.expand_array)(ch);
        let rv = (p.r.expand_array)(rh);
        assert_same!("expand_array on capacity 0", cv, rv);
        assert_eq!(
            cv, 0,
            "glibc realloc(p,0) returns NULL, so the C treats it as failure"
        );
        // capacity must NOT have been updated (still 0).
        assert_same!(
            "capacity after realloc-to-0",
            (*ch).capacity,
            (*rh).capacity
        );
        assert_eq!((*ch).capacity, 0);
        // `data` is now dangling in BOTH implementations -- identical (preserved) bug.
        free_after_dangling(&p.c, ch);
        free_after_dangling(&p.r, rh);
    }
}

// ===========================================================================
// Row 8: expand_array -- capacity doubling wraps size_t
// ===========================================================================

#[test]
fn err08_expand_array_capacity_doubling_wraps() {
    let p = load_pair();
    // 1<<63 doubles to 0; the others double to small values, so realloc SUCCEEDS
    // and the (absurd) capacity is stored verbatim. Both must agree either way.
    for &cap in &[1usize << 63, (1usize << 63) + 5, (1usize << 63) + 1, usize::MAX] {
        unsafe {
            let ch = (p.c.init_array)(4);
            let rh = (p.r.init_array)(4);
            assert!(!ch.is_null() && !rh.is_null());
            (p.c.add_element)(ch, 99);
            (p.r.add_element)(rh, 99);
            (*ch).capacity = cap;
            (*rh).capacity = cap;

            let cv = (p.c.expand_array)(ch);
            let rv = (p.r.expand_array)(rh);
            assert_same!(format!("expand_array rc with wrapping capacity {cap:#x}"), cv, rv);
            assert_same!(
                format!("capacity after wrapping expand {cap:#x}"),
                (*ch).capacity,
                (*rh).capacity
            );
            assert_same!(
                format!("data null-ness after wrapping expand {cap:#x}"),
                (*ch).data.is_null(),
                (*rh).data.is_null()
            );
            if cv == 1 {
                assert_eq!(
                    (*ch).capacity,
                    cap.wrapping_mul(2),
                    "on success capacity becomes the WRAPPED double"
                );
            } else {
                // On failure the C returns before touching `capacity`.
                assert_eq!(
                    (*ch).capacity, cap,
                    "on failure capacity must be left unchanged"
                );
            }

            if cv == 0 {
                // realloc(p, 0) freed the buffer: avoid a double free.
                free_after_dangling(&p.c, ch);
                free_after_dangling(&p.r, rh);
            } else {
                (*ch).size = 1;
                (*rh).size = 1;
                (p.c.free_array)(ch);
                (p.r.free_array)(rh);
            }
        }
    }
}

// ===========================================================================
// Row 9: add_element(NULL, v) -> 0
// ===========================================================================

#[test]
fn err09_add_element_null_returns_zero() {
    let p = load_pair();
    for v in [0, 1, -1, c_int::MAX, c_int::MIN] {
        let cv = unsafe { (p.c.add_element)(std::ptr::null_mut(), v) };
        let rv = unsafe { (p.r.add_element)(std::ptr::null_mut(), v) };
        assert_same!(format!("add_element(NULL, {v})"), cv, rv);
        assert_eq!(cv, 0, "C add_element(NULL) must return the 0 sentinel");
    }
}

// ===========================================================================
// Row 10: add_element -- expansion failure propagates
// ===========================================================================

#[test]
fn err10_add_element_expand_failure_propagates() {
    let p = load_pair();
    let huge = usize::MAX / 8;
    unsafe {
        let ch = (p.c.init_array)(4);
        let rh = (p.r.init_array)(4);
        assert!(!ch.is_null() && !rh.is_null());
        (p.c.add_element)(ch, 7);
        (p.r.add_element)(rh, 7);

        // size >= capacity, and expanding is impossible.
        (*ch).size = huge;
        (*ch).capacity = huge;
        (*rh).size = huge;
        (*rh).capacity = huge;

        let cv = (p.c.add_element)(ch, 12345);
        let rv = (p.r.add_element)(rh, 12345);
        assert_same!("add_element with unsatisfiable expansion", cv, rv);
        assert_eq!(cv, 0, "failure must propagate as the 0 sentinel");
        // The element was NOT stored and size was NOT incremented.
        assert_same!("size after failed add", (*ch).size, (*rh).size);
        assert_eq!((*ch).size, huge, "size must not be incremented");
        assert_same!("capacity after failed add", (*ch).capacity, (*rh).capacity);

        (*ch).size = 1;
        (*ch).capacity = 4;
        (*rh).size = 1;
        (*rh).capacity = 4;
        assert_eq!(p.c.read_elems(ch), vec![7], "no element appended");
        assert_eq!(p.r.read_elems(rh), vec![7], "no element appended");
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

// ===========================================================================
// Row 11: add_element on a zero-capacity array
// ===========================================================================

#[test]
fn err11_add_element_on_zero_capacity_array() {
    let p = load_pair();
    unsafe {
        let ch = (p.c.init_array)(0);
        let rh = (p.r.init_array)(0);
        assert!(!ch.is_null() && !rh.is_null());

        let cv = (p.c.add_element)(ch, 42);
        let rv = (p.r.add_element)(rh, 42);
        assert_same!("add_element on capacity-0 array", cv, rv);
        assert_eq!(cv, 0, "expand_array's realloc(p,0) failure propagates");
        assert_same!("size after failed add", (*ch).size, (*rh).size);
        assert_eq!((*ch).size, 0, "size must stay 0");
        assert_same!("capacity after failed add", (*ch).capacity, (*rh).capacity);
        assert_eq!((*ch).capacity, 0);

        free_after_dangling(&p.c, ch);
        free_after_dangling(&p.r, rh);
    }
}

// ===========================================================================
// Row 12: add_element with size STRICTLY greater than capacity
// ===========================================================================

#[test]
fn err12_add_element_size_greater_than_capacity() {
    let p = load_pair();
    // capacity 8 fully initialised, then size forced to 10 (> capacity). The
    // `>=` check routes through expand_array, which doubles 8 -> 16, so the
    // write at index 10 stays inside the new 16-element buffer.
    unsafe {
        let ch = (p.c.init_array)(8);
        let rh = (p.r.init_array)(8);
        assert!(!ch.is_null() && !rh.is_null());
        for i in 0..8 {
            (p.c.add_element)(ch, 100 + i);
            (p.r.add_element)(rh, 100 + i);
        }
        (*ch).size = 10;
        (*rh).size = 10;

        let cv = (p.c.add_element)(ch, 777);
        let rv = (p.r.add_element)(rh, 777);
        assert_same!("add_element with size(10) > capacity(8)", cv, rv);
        assert_eq!(cv, 1, "expansion succeeds, so the element is stored");
        assert_same!("size after add", (*ch).size, (*rh).size);
        assert_eq!((*ch).size, 11, "size incremented past the forced value");
        assert_same!("capacity after add", (*ch).capacity, (*rh).capacity);
        assert_eq!((*ch).capacity, 16, "8 doubled to 16");

        // Compare only DEFINED elements: 0..8 (initialised before the poke) and
        // index 10 (just written). Indices 8, 9 and 11.. are indeterminate
        // realloc padding in both implementations.
        let cd = std::slice::from_raw_parts((*ch).data, 11);
        let rd = std::slice::from_raw_parts((*rh).data, 11);
        assert_same!("preserved prefix", cd[0..8].to_vec(), rd[0..8].to_vec());
        assert_eq!(cd[0..8], [100, 101, 102, 103, 104, 105, 106, 107]);
        assert_same!("value written at index 10", cd[10], rd[10]);
        assert_eq!(cd[10], 777, "value lands at data[size] == data[10]");

        (*ch).size = 8;
        (*rh).size = 8;
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

// ===========================================================================
// Row 13: free_array(NULL) -> silent no-op
// ===========================================================================

#[test]
fn err13_free_array_null_is_noop() {
    let p = load_pair();
    unsafe {
        // Must not crash in either implementation, and must be repeatable.
        for _ in 0..100 {
            (p.c.free_array)(std::ptr::null_mut());
            (p.r.free_array)(std::ptr::null_mut());
        }
        // A NULL `data` field is also fine (free(NULL) is a no-op in C).
        let ch = (p.c.init_array)(2);
        let rh = (p.r.init_array)(2);
        set_fields(ch, std::ptr::null_mut(), 0, 0);
        set_fields(rh, std::ptr::null_mut(), 0, 0);
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

// ===========================================================================
// Row 14: matrixsum's -1 sentinel is unreachable
// ===========================================================================

#[test]
fn err14_matrixsum_never_returns_error_sentinel() {
    let p = load_pair();
    with_matrix_lock(&p, || {
        // `matrixsum` allocates a fixed capacity of 2 (8 bytes), which never
        // fails, so the `return -1` at line 155 is unreachable. Prove the
        // allocation always succeeds, then confirm C/Rust agree regardless.
        for _ in 0..2000 {
            unsafe {
                let ch = (p.c.init_array)(2);
                let rh = (p.r.init_array)(2);
                assert!(!ch.is_null(), "init_array(2) must never fail in C");
                assert!(!rh.is_null(), "init_array(2) must never fail in Rust");
                (p.c.free_array)(ch);
                (p.r.free_array)(rh);
            }
        }
        // -1 is nonetheless a legitimate ordinary result for some inputs, so we
        // only assert C/Rust agreement here, including on inputs that produce it.
        let mut rng = Rng::new(SEED ^ 0xE14);
        for _ in 0..3000 {
            let (a, b, c, d) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            assert_same!(
                format!("matrixsum({a},{b},{c},{d})"),
                unsafe { (p.c.matrixsum)(a, b, c, d) },
                unsafe { (p.r.matrixsum)(a, b, c, d) }
            );
        }
    });
}

// ===========================================================================
// Row 15: capacity * sizeof(int) wraps to a SMALL non-zero product
// ===========================================================================

#[test]
fn err15_init_array_capacity_product_wraps_to_small() {
    let p = load_pair();
    // (1<<62)+1 -> byte product 4: malloc SUCCEEDS with a 4-byte buffer while
    // `capacity` is 2^62+1. A checked multiply in Rust would wrongly reject.
    for &cap in &[(1usize << 62) + 1, (1usize << 62) + 2, (1usize << 63) + 3] {
        unsafe {
            let ch = (p.c.init_array)(cap);
            let rh = (p.r.init_array)(cap);
            assert_same!(
                format!("init_array({cap:#x}) null-ness (product wraps small)"),
                ch.is_null(),
                rh.is_null()
            );
            assert!(
                !ch.is_null(),
                "C init_array({cap:#x}) should succeed: byte product is {}",
                cap.wrapping_mul(4)
            );
            let cs = p.c.read_handle(ch);
            let rs = p.r.read_handle(rh);
            assert_same!(format!("init_array({cap:#x}) size"), cs.size, rs.size);
            assert_same!(
                format!("init_array({cap:#x}) capacity"),
                cs.capacity,
                rs.capacity
            );
            assert_eq!((cs.size, cs.capacity), (0, cap));
            (p.c.free_array)(ch);
            (p.r.free_array)(rh);
        }
    }
}

// ===========================================================================
// G4: out-of-range / reserved-bit "enum" values across the FFI boundary
// ===========================================================================

#[test]
fn err_g4_process_flags_out_of_range_and_reserved_bits() {
    let p = load_pair();
    // This library has no C enum; `process_flags` takes an `int` bitmask, so the
    // analogous "no valid variant" inputs are values whose bits fall entirely
    // outside the four documented flags. The C masks each flag and `!!`s it, so
    // every unknown bit MUST be ignored.
    let mut cases: Vec<c_int> = vec![
        0,          // no variant at all
        0x10,       // first reserved bit only
        0x20,
        0x40,
        0x80,
        0x100,
        !0x0F,      // every reserved bit, no valid flag
        c_int::MIN, // sign bit only
        c_int::MAX,
        -1, // all bits
        0x7FFF_FFF0,
        -16,
        16,
        0xFFFF & !0xF,
    ];
    let mut rng = Rng::new(SEED ^ 0x64);
    for _ in 0..3000 {
        // Reserved bits only -- guaranteed to have no valid flag bit set.
        cases.push((rng.next_i32() as u32 & 0xFFFF_FFF0) as c_int);
    }
    for flags in cases {
        let cv = unsafe { (p.c.process_flags)(flags) };
        let rv = unsafe { (p.r.process_flags)(flags) };
        assert_same!(format!("process_flags({flags:#010x})"), cv, rv);
        if flags & 0x0F == 0 {
            assert_eq!(
                cv, 0,
                "reserved bits must not be counted: process_flags({flags:#010x})"
            );
        }
        assert!(
            (0..=4).contains(&cv),
            "count must stay in 0..=4, got {cv} for {flags:#010x}"
        );
    }
}

// ===========================================================================
// G5: signed-overflow extremes through matrixsum
// ===========================================================================

#[test]
fn err_g5_matrixsum_signed_overflow_extremes() {
    let p = load_pair();
    with_matrix_lock(&p, || {
        // sum*0x10 and the subsequent adds overflow `int`; gcc wraps, so the
        // Rust must use wrapping arithmetic (a debug-mode `+` would panic).
        const E: [c_int; 6] = [
            c_int::MAX,
            c_int::MIN,
            c_int::MAX - 1,
            c_int::MIN + 1,
            0x7FFF_FFFF,
            -0x8000_0000,
        ];
        for &a in &E {
            for &b in &E {
                for &c in &E {
                    for &d in &E {
                        assert_same!(
                            format!("matrixsum({a},{b},{c},{d}) [overflow]"),
                            unsafe { (p.c.matrixsum)(a, b, c, d) },
                            unsafe { (p.r.matrixsum)(a, b, c, d) }
                        );
                    }
                }
            }
        }
    });
}

// ===========================================================================
// G6: mutated matrix producing a negative checksum (masked with & 0xFFF)
// ===========================================================================

#[test]
fn err_g6_matrix_mutation_negative_checksum() {
    let p = load_pair();
    with_matrix_lock(&p, || {
        // `matrix_sum & 0xFFF` on a NEGATIVE sum: C's `&` on a negative int uses
        // its two's-complement bit pattern, yielding a non-negative result.
        let cases: [[c_int; MATRIX_LEN]; 6] = [
            [-1; MATRIX_LEN],
            [c_int::MIN; MATRIX_LEN],
            [c_int::MAX; MATRIX_LEN],
            [-4096; MATRIX_LEN],
            [-1, -2, -3, -4, -5, -6, -7, -8, -9, -10, -11, -12],
            [
                c_int::MIN,
                c_int::MIN,
                1,
                2,
                3,
                4,
                -5,
                -6,
                c_int::MAX,
                c_int::MAX,
                -1,
                0,
            ],
        ];
        for m in cases {
            p.set_matrices(&m);
            let cs = unsafe { (p.c.calculate_matrix_checksum)() };
            let rs = unsafe { (p.r.calculate_matrix_checksum)() };
            assert_same!(format!("checksum for {m:?}"), cs, rs);
            for (a, b, c, d) in [(0, 0, 0, 0), (1, 1, 1, 1), (c_int::MIN, -1, 0, 1)] {
                assert_same!(
                    format!("matrixsum({a},{b},{c},{d}) with negative checksum {cs}"),
                    unsafe { (p.c.matrixsum)(a, b, c, d) },
                    unsafe { (p.r.matrixsum)(a, b, c, d) }
                );
            }
        }
    });
}

// ===========================================================================
// G1/G2 aggregate: NULL and zero-length across every pointer entry point
// ===========================================================================

#[test]
fn err_g1_g2_null_and_zero_length_across_all_entry_points() {
    let p = load_pair();
    let null: *mut DynamicArray = std::ptr::null_mut();
    unsafe {
        assert_same!("expand_array(NULL)", (p.c.expand_array)(null), (p.r.expand_array)(null));
        assert_same!("add_element(NULL,0)", (p.c.add_element)(null, 0), (p.r.add_element)(null, 0));
        // free_array(NULL) returns void; assert it does not crash either side.
        (p.c.free_array)(null);
        (p.r.free_array)(null);

        // Zero-length handles through the whole lifecycle.
        let ch = (p.c.init_array)(0);
        let rh = (p.r.init_array)(0);
        assert_same!("init_array(0) null-ness", ch.is_null(), rh.is_null());
        assert_same!("init_array(0) state", p.c.snapshot(ch), p.r.snapshot(rh));
        assert_same!(
            "add_element on zero-length",
            (p.c.add_element)(ch, 1),
            (p.r.add_element)(rh, 1)
        );
        assert_same!("state after add on zero-length", p.c.snapshot(ch), p.r.snapshot(rh));
        free_after_dangling(&p.c, ch);
        free_after_dangling(&p.r, rh);
    }
}

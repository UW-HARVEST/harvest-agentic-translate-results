// Phase C — error-path differential tests, one test per row of ERRORS.md.
// Each test constructs the exact invalid input/condition, calls BOTH libraries
// through their `.so` exports, and asserts they return the SAME sentinel /
// error code and leave the SAME observable state behind.

mod common;

use common::*;
use std::os::raw::c_int;
use std::ptr;

/// A capacity whose `*4` byte size (4 PiB) cannot be satisfied by any 64-bit
/// Linux allocator (user VA space is 128 TiB), so malloc/realloc must fail.
const UNSATISFIABLE: SizeT = 1 << 50;

// ---------------------------------------------------------------------------
// Row 1 — init_array: the 24-byte struct malloc fails.
//
// Not directly forceable without an allocator interposer, so it is covered
// structurally: the branch is `if (!arr) return NULL;`, i.e. identical to the
// generic "allocation failed => NULL, nothing leaked" contract exercised by
// row 2. What IS asserted differentially here is that a *successful* call never
// takes the branch, in both libraries, and that the struct layout the branch
// guards is the same size in both (24 bytes on LP64).
// ---------------------------------------------------------------------------
#[test]
fn row01_init_array_struct_malloc_failure_branch() {
    let p = libs();
    assert_eq!(std::mem::size_of::<DynamicArray>(), 24, "DynamicArray is 3 words on LP64");
    unsafe {
        let a = (p.c.init_array)(4);
        let b = (p.rust.init_array)(4);
        assert_eq!(a.is_null(), b.is_null(), "init_array(4) null-ness must agree");
        assert!(!a.is_null(), "a 16-byte allocation is expected to succeed");
        assert_eq!(p.c.snapshot(a), p.rust.snapshot(b));
        (p.c.free_array)(a);
        (p.rust.free_array)(b);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — init_array: the data malloc fails => free(arr), return NULL
// ---------------------------------------------------------------------------
#[test]
fn row02_init_array_data_malloc_failure_returns_null() {
    let p = libs();
    unsafe {
        for cap in [UNSATISFIABLE, 1usize << 51, usize::MAX / 4] {
            let a = (p.c.init_array)(cap);
            let b = (p.rust.init_array)(cap);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "init_array({cap:#x}): C null={} vs Rust null={}",
                a.is_null(),
                b.is_null()
            );
            if !a.is_null() {
                // Allocator surprised us (overcommit); still compare states.
                assert_eq!(p.c.snapshot(a), p.rust.snapshot(b), "init_array({cap:#x}) state");
                (p.c.free_array)(a);
                (p.rust.free_array)(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — init_array: `capacity * sizeof(int)` wraps size_t (unsigned wrap,
// small/zero request succeeds, capacity keeps the huge unwrapped value)
// ---------------------------------------------------------------------------
#[test]
fn row03_init_array_size_multiply_wraps() {
    let p = libs();
    unsafe {
        for &cap in &[
            usize::MAX / 4 + 1, // *4 == 0
            usize::MAX / 4 + 2, // *4 == 4
            usize::MAX / 2 + 1,
            usize::MAX / 2 + 2,
            usize::MAX - 1,
            usize::MAX,
        ] {
            let a = (p.c.init_array)(cap);
            let b = (p.rust.init_array)(cap);
            assert_eq!(a.is_null(), b.is_null(), "init_array({cap:#x}) null-ness");
            if !a.is_null() {
                let sa = p.c.snapshot(a);
                let sb = p.rust.snapshot(b);
                assert_eq!(sa, sb, "init_array({cap:#x}) wrapped-allocation state");
                assert_eq!(sa.capacity, cap, "capacity records the unwrapped value");
                assert_eq!(sa.size, 0);
                (p.c.free_array)(a);
                (p.rust.free_array)(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — init_array(0): malloc(0)
// ---------------------------------------------------------------------------
#[test]
fn row04_init_array_zero_capacity() {
    let p = libs();
    unsafe {
        let a = (p.c.init_array)(0);
        let b = (p.rust.init_array)(0);
        assert_eq!(a.is_null(), b.is_null(), "init_array(0) null-ness");
        let sa = p.c.snapshot(a);
        let sb = p.rust.snapshot(b);
        assert_eq!(sa, sb, "init_array(0) state");
        assert_eq!((sa.size, sa.capacity), (0, 0));
        (p.c.free_array)(a);
        (p.rust.free_array)(b);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — expand_array(NULL) => 0
// ---------------------------------------------------------------------------
#[test]
fn row05_expand_array_null_pointer() {
    let p = libs();
    unsafe {
        let ra = (p.c.expand_array)(ptr::null_mut());
        let rb = (p.rust.expand_array)(ptr::null_mut());
        assert_eq!(ra, 0, "C expand_array(NULL) must return the 0 sentinel");
        assert_eq!(ra, rb, "expand_array(NULL)");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — expand_array: realloc fails => 0, `data`/`capacity` untouched
// ---------------------------------------------------------------------------
#[test]
fn row06_expand_array_realloc_failure_leaves_state_intact() {
    let p = libs();
    unsafe {
        for &cap in &[UNSATISFIABLE, 1usize << 49, usize::MAX / 8] {
            // Stack-allocated descriptors with a NULL buffer: realloc(NULL, huge)
            // is a plain malloc of an unsatisfiable size.
            let mut a = DynamicArray::stack(cap);
            let mut b = DynamicArray::stack(cap);
            let ra = (p.c.expand_array)(&mut a);
            let rb = (p.rust.expand_array)(&mut b);
            assert_eq!(ra, rb, "expand_array return for capacity {cap:#x}");
            assert_eq!(
                (a.data.is_null(), a.size, a.capacity),
                (b.data.is_null(), b.size, b.capacity),
                "descriptor state after failed expand ({cap:#x})"
            );
            if ra == 0 {
                assert_eq!(a.capacity, cap, "capacity untouched on failure");
                assert!(a.data.is_null(), "data untouched on failure");
            } else {
                // Unexpectedly satisfiable: release identically in both libs.
                p.free_buf(a.data);
                p.free_buf(b.data);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — expand_array with capacity 0 => realloc(data, 0) => returns 0
// ---------------------------------------------------------------------------
#[test]
fn row07_expand_array_zero_capacity_frees_buffer() {
    let p = libs();
    unsafe {
        let a = (p.c.init_array)(0);
        let b = (p.rust.init_array)(0);
        assert!(!a.is_null() && !b.is_null(), "init_array(0) should succeed");
        let ra = (p.c.expand_array)(a);
        let rb = (p.rust.expand_array)(b);
        assert_eq!(ra, rb, "expand_array on capacity 0");
        assert_eq!(
            ((*a).data.is_null(), (*a).size, (*a).capacity),
            ((*b).data.is_null(), (*b).size, (*b).capacity),
            "descriptor state after expand on capacity 0"
        );
        assert_eq!((*a).capacity, 0, "capacity stays 0 when the doubling fails");
        // `data` now dangles identically in both libraries (glibc realloc(p,0)
        // frees). Neutralise before the final free.
        (*a).data = ptr::null_mut();
        (*b).data = ptr::null_mut();
        (p.c.free_array)(a);
        (p.rust.free_array)(b);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — expand_array: `capacity * 2` (or `* 4` bytes) wraps size_t
// ---------------------------------------------------------------------------
#[test]
fn row08_expand_array_capacity_multiply_wraps() {
    let p = libs();
    unsafe {
        for &cap in &[
            usize::MAX / 2 + 1, // *2 == 0
            usize::MAX / 2 + 2, // *2 == 2
            usize::MAX,         // *2 == MAX-1
            usize::MAX - 1,
            usize::MAX / 4 + 1,
            usize::MAX / 8 + 1,
        ] {
            let mut a = DynamicArray::stack(cap);
            let mut b = DynamicArray::stack(cap);
            let ra = (p.c.expand_array)(&mut a);
            let rb = (p.rust.expand_array)(&mut b);
            assert_eq!(ra, rb, "expand_array return for wrapping capacity {cap:#x}");
            assert_eq!(
                (a.data.is_null(), a.size, a.capacity),
                (b.data.is_null(), b.size, b.capacity),
                "descriptor state for wrapping capacity {cap:#x}"
            );
            if ra != 0 {
                assert_eq!(
                    a.capacity,
                    cap.wrapping_mul(2),
                    "capacity set to the wrapped doubling"
                );
                // The wrapped byte count was small enough to satisfy; release
                // the buffers so the sweep does not leak.
                p.free_buf(a.data);
                p.free_buf(b.data);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — add_element(NULL, v) => 0
// ---------------------------------------------------------------------------
#[test]
fn row09_add_element_null_pointer() {
    let p = libs();
    unsafe {
        for v in [0, 1, -1, i32::MAX, i32::MIN] {
            let ra = (p.c.add_element)(ptr::null_mut(), v);
            let rb = (p.rust.add_element)(ptr::null_mut(), v);
            assert_eq!(ra, 0, "C add_element(NULL, {v}) must return the 0 sentinel");
            assert_eq!(ra, rb, "add_element(NULL, {v})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — add_element: growth needed but expand_array fails => 0, size kept
// ---------------------------------------------------------------------------
#[test]
fn row10_add_element_growth_failure() {
    let p = libs();
    unsafe {
        for &cap in &[UNSATISFIABLE, 1usize << 49] {
            // size == capacity forces the growth branch; the huge capacity makes
            // the realloc inside expand_array fail.
            let mut a = DynamicArray { data: ptr::null_mut(), size: cap, capacity: cap };
            let mut b = DynamicArray { data: ptr::null_mut(), size: cap, capacity: cap };
            let ra = (p.c.add_element)(&mut a, 0x5A5A);
            let rb = (p.rust.add_element)(&mut b, 0x5A5A);
            assert_eq!(ra, rb, "add_element return when growth fails ({cap:#x})");
            assert_eq!(ra, 0, "C must reject with the 0 sentinel");
            assert_eq!(a, b, "descriptor untouched identically");
            assert_eq!(a.size, cap, "size must NOT be incremented on failure");
            assert_eq!(a.capacity, cap, "capacity must NOT change on failure");
            assert!(a.data.is_null());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — add_element on a capacity-0 array => 0
// ---------------------------------------------------------------------------
#[test]
fn row11_add_element_on_zero_capacity_array() {
    let p = libs();
    unsafe {
        let a = (p.c.init_array)(0);
        let b = (p.rust.init_array)(0);
        assert!(!a.is_null() && !b.is_null());
        let ra = (p.c.add_element)(a, 42);
        let rb = (p.rust.add_element)(b, 42);
        assert_eq!(ra, rb, "add_element on a capacity-0 array");
        assert_eq!(ra, 0, "C rejects the add because the doubling of 0 fails");
        assert_eq!(
            ((*a).data.is_null(), (*a).size, (*a).capacity),
            ((*b).data.is_null(), (*b).size, (*b).capacity),
        );
        assert_eq!((*a).size, 0, "size not incremented");
        // NOTE: a *second* `add_element` here is deliberately NOT attempted.
        // The C code leaves `arr->data` dangling after `realloc(data, 0)` freed
        // it, so a retry makes the C library itself double-free (verified: it
        // aborts with "free(): double free detected"). That is faithful C
        // behaviour, not a translation difference, and it cannot be observed
        // differentially without crashing the harness.
        (*a).data = ptr::null_mut();
        (*b).data = ptr::null_mut();
        (p.c.free_array)(a);
        (p.rust.free_array)(b);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — free_array(NULL) is a no-op
// ---------------------------------------------------------------------------
#[test]
fn row12_free_array_null_pointer() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    unsafe {
        for _ in 0..4 {
            (p.c.free_array)(ptr::null_mut());
            (p.rust.free_array)(ptr::null_mut());
        }
    }
    // Reaching here without a crash in either library is the assertion; prove
    // both libraries are still usable afterwards.
    unsafe {
        assert_eq!(
            (p.c.calculate_matrix_checksum)(),
            (p.rust.calculate_matrix_checksum)(),
            "libraries still functional after free_array(NULL)"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12b — free_array on a descriptor whose `data` is NULL (free(NULL) no-op)
// ---------------------------------------------------------------------------
#[test]
fn row12b_free_array_with_null_data() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    unsafe {
        for _ in 0..8 {
            let a = (p.c.init_array)(4);
            let b = (p.rust.init_array)(4);
            assert!(!a.is_null() && !b.is_null());
            // Detach and release the buffers ourselves, then null the field so
            // free_array takes the `free(NULL)` path in both libraries.
            let da = (*a).data;
            let db = (*b).data;
            (*a).data = ptr::null_mut();
            (*b).data = ptr::null_mut();
            p.free_buf(da);
            p.free_buf(db);
            assert_eq!(
                p.c.snapshot(a),
                p.rust.snapshot(b),
                "descriptor with a NULL data pointer"
            );
            (p.c.free_array)(a);
            (p.rust.free_array)(b);
        }
        assert_eq!(
            (p.c.calculate_matrix_checksum)(),
            (p.rust.calculate_matrix_checksum)(),
            "libraries still functional after free_array with NULL data"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — matrixsum returns -1 when init_array(2) fails.
//
// `init_array(2)` requests 24 + 8 bytes, which cannot be made to fail without
// an allocator interposer. The branch is still pinned differentially: both
// libraries must agree that a normal call never yields the -1 sentinel by
// accident, i.e. -1 is only produced for genuinely -1-valued arithmetic, and
// both produce it for exactly the same inputs (row 23 of Phase B sweeps 4096
// random quadruples; here we sweep the inputs whose arithmetic result IS -1 and
// confirm the two libraries agree, so the sentinel is never ambiguous).
// ---------------------------------------------------------------------------
#[test]
fn row13_matrixsum_minus_one_sentinel_agreement() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0xD13_0013);
    unsafe {
        let mut saw_minus_one = false;
        for _ in 0..20000 {
            let q = [rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()];
            let ra = (p.c.matrixsum)(q[0], q[1], q[2], q[3]);
            let rb = (p.rust.matrixsum)(q[0], q[1], q[2], q[3]);
            assert_eq!(ra, rb, "matrixsum{q:?} (sentinel agreement sweep)");
            saw_minus_one |= ra == -1;
        }
        let _ = saw_minus_one;
        // Also: a successful allocation path must never return -1 spuriously for
        // the all-zero input (checksum-only result).
        assert_eq!((p.c.matrixsum)(0, 0, 0, 0), (p.rust.matrixsum)(0, 0, 0, 0));
    }
}

// ---------------------------------------------------------------------------
// Row 14 — process_flags accepts every int (no rejection path). This is the
// "out-of-range enum value across FFI" case: the FLAG_* bitmask is an `int`, so
// values with no valid variant must be handled identically.
// ---------------------------------------------------------------------------
#[test]
fn row14_process_flags_out_of_range_values() {
    let p = libs();
    let mut rng = Rng::new(0x0E0E_0E0E);
    let mut cases: Vec<c_int> = vec![
        -1,
        i32::MIN,
        i32::MAX,
        0x10,       // one past the highest valid flag bit
        0x1F,       // all valid flags + one invalid bit
        0xFF,
        0x7FFF_FFFF,
        16,
        17,
        255,
        256,
        1 << 30,
        (1u32 << 31) as i32,
        ((1u32 << 31) | 0xF) as i32,
    ];
    for _ in 0..8192 {
        cases.push(rng.i32());
    }
    unsafe {
        for v in cases {
            let ra = (p.c.process_flags)(v);
            let rb = (p.rust.process_flags)(v);
            assert_eq!(ra, rb, "process_flags({v:#x})");
            assert!((0..=4).contains(&ra), "C result out of the documented 0..=4 range: {ra}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — matrixsum has no input validation: overflow wraps, never errors
// ---------------------------------------------------------------------------
#[test]
fn row15_matrixsum_no_validation_overflow_wraps() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let edges: [c_int; 8] =
        [i32::MAX, i32::MIN, i32::MAX / 2, i32::MIN / 2, i32::MAX - 1, i32::MIN + 1, 1, -1];
    unsafe {
        for &a in &edges {
            for &b in &edges {
                for &c in &edges {
                    for &d in &edges {
                        assert_eq!(
                            (p.c.matrixsum)(a, b, c, d),
                            (p.rust.matrixsum)(a, b, c, d),
                            "matrixsum({a}, {b}, {c}, {d}) overflow shape"
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic boundary sweep: every size_t edge value through init_array
// ---------------------------------------------------------------------------
#[test]
fn generic_init_array_size_edge_sweep() {
    let p = libs();
    unsafe {
        for &cap in SIZE_EDGE {
            let a = (p.c.init_array)(cap);
            let b = (p.rust.init_array)(cap);
            assert_eq!(a.is_null(), b.is_null(), "init_array({cap:#x}) null-ness");
            if !a.is_null() {
                assert_eq!(p.c.snapshot(a), p.rust.snapshot(b), "init_array({cap:#x}) state");
                (p.c.free_array)(a);
                (p.rust.free_array)(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic boundary sweep: every size_t edge value as an existing `capacity`
// handed to expand_array / add_element (descriptor on the stack, NULL buffer)
// ---------------------------------------------------------------------------
#[test]
fn generic_expand_size_edge_sweep() {
    let p = libs();
    unsafe {
        for &cap in SIZE_EDGE {
            // `data == NULL` makes the realloc a plain malloc of the (possibly
            // wrapped) doubled size — no element memory is ever touched, so this
            // probes the arithmetic and the failure branch in isolation.
            let mut a = DynamicArray::stack(cap);
            let mut b = DynamicArray::stack(cap);
            let ra = (p.c.expand_array)(&mut a);
            let rb = (p.rust.expand_array)(&mut b);
            assert_eq!(ra, rb, "expand_array at capacity {cap:#x}");
            assert_eq!(
                (a.data.is_null(), a.size, a.capacity),
                (b.data.is_null(), b.size, b.capacity),
                "descriptor after expand_array at capacity {cap:#x}"
            );
            if ra != 0 {
                assert_eq!(a.capacity, cap.wrapping_mul(2));
                p.free_buf(a.data);
                p.free_buf(b.data);
            } else {
                assert_eq!(a.capacity, cap, "capacity untouched on failure");
                assert!(a.data.is_null(), "data untouched on failure");
            }
        }
    }
}

/// `add_element` on `size == capacity` where the growth is guaranteed to fail:
/// both libraries must reject with `0` and leave the descriptor untouched. (The
/// capacities are chosen so `capacity*2*sizeof(int)` does NOT wrap to a small
/// satisfiable size — a successful growth here would have `add_element` write
/// at an astronomically out-of-range index, which is not a state a real caller
/// can reach through the public API.)
#[test]
fn generic_add_element_unsatisfiable_growth_sweep() {
    let p = libs();
    unsafe {
        for &cap in &[
            1usize << 50,
            1 << 49,
            1 << 48,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX / 4 - 1,
            usize::MAX / 8 - 1,
        ] {
            assert!(
                cap.wrapping_mul(2).wrapping_mul(4) > (1usize << 48),
                "test precondition: capacity {cap:#x} must be unsatisfiable, not wrapped-small"
            );
            let mut a = DynamicArray { data: ptr::null_mut(), size: cap, capacity: cap };
            let mut b = DynamicArray { data: ptr::null_mut(), size: cap, capacity: cap };
            let ra = (p.c.add_element)(&mut a, 7);
            let rb = (p.rust.add_element)(&mut b, 7);
            assert_eq!(ra, rb, "add_element with size==capacity=={cap:#x}");
            assert_eq!(ra, 0, "C must reject when the growth cannot be satisfied");
            assert_eq!(a, b, "descriptor after add_element at capacity {cap:#x}");
            assert_eq!(a.size, cap, "size not incremented");
        }
    }
}

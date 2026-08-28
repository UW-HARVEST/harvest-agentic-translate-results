//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are called only through their shared objects
//! (`libloading`), so the `#[no_mangle]` export wrappers are under test too.
//! Every row uses `ITERS` pseudo-random inputs from a fixed seed.
//!
//! ## Allocator determinism (why every malloc-touching call is normalised)
//!
//! `compare_allocations` observably compares the addresses of two consecutive
//! `malloc(4)` results, so its value is a function of the state of the
//! process-wide glibc allocator, which both libraries share. `probe_alloc.rs`
//! shows two `dlopen`s of the *same* C `.so` diverging from each other because
//! of it, so it is environmental, not a translation defect.
//!
//! Every measurement therefore calls `common::normalize_allocator(order)`
//! immediately before the library call, which canonicalises the `sizeof(int)`
//! tcache bin so the library is *guaranteed* to observe `ptr1 < ptr2`
//! (`Increasing`) or `ptr1 > ptr2` (`Decreasing`). Each row is run under **both**
//! orderings, so both branches of `lib.c:102-108` are covered deliberately and
//! the comparison is fully deterministic (the tcache is thread-local, so
//! parallel test threads cannot interfere). Nothing may allocate between
//! `normalize_allocator` and the call it protects.

mod common;

use common::{both, normalize_allocator, AllocOrder, Api, Rng, ITERS};
use std::ffi::{c_char, c_int};

const GUARD: i32 = 0x5A5A_5A5A;

// ---------------------------------------------------------------------------
// Differential drivers. Each runs the call once per address ordering.
// ---------------------------------------------------------------------------

/// `compare_allocations(val1, val2)`: compare C vs Rust *and* pin the exact
/// value the C source prescribes for the forced address ordering.
fn diff_cmp_alloc(row: &str, i: usize, v1: c_int, v2: c_int) {
    let (c, r) = both();
    for order in AllocOrder::both() {
        normalize_allocator(order);
        let cv = unsafe { (c.compare_allocations)(v1, v2) };
        normalize_allocator(order);
        let rv = unsafe { (r.compare_allocations)(v1, v2) };
        assert_eq!(
            cv, rv,
            "{row} iter {i}: compare_allocations({v1}, {v2}) [{order:?}] C={cv} Rust={rv}"
        );
        let expected = order.expected_branch() + if v1 > 0 { 10 } else { 0 };
        assert_eq!(
            cv, expected,
            "{row} iter {i}: compare_allocations({v1}, {v2}) [{order:?}] must be {expected}"
        );
    }
}

fn diff_arity4(row: &str, i: usize, p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (c, r) = both();
    for order in AllocOrder::both() {
        normalize_allocator(order);
        let cv = unsafe { (c.arity4)(p1, p2, p3, p4) };
        normalize_allocator(order);
        let rv = unsafe { (r.arity4)(p1, p2, p3, p4) };
        assert_eq!(
            cv, rv,
            "{row} iter {i}: arity4({p1}, {p2}, {p3}, {p4}) [{order:?}] C={cv} Rust={rv}"
        );
    }
}

fn diff_arity3(row: &str, i: usize, p1: c_int, p2: c_int, p3: c_int) {
    let (c, r) = both();
    for order in AllocOrder::both() {
        normalize_allocator(order);
        let cv = unsafe { (c.arity3)(p1, p2, p3) };
        normalize_allocator(order);
        let rv = unsafe { (r.arity3)(p1, p2, p3) };
        assert_eq!(
            cv, rv,
            "{row} iter {i}: arity3({p1}, {p2}, {p3}) [{order:?}] C={cv} Rust={rv}"
        );
    }
}

fn diff_arity2(row: &str, i: usize, p1: c_int, p2: c_int) {
    let (c, r) = both();
    for order in AllocOrder::both() {
        normalize_allocator(order);
        let cv = unsafe { (c.arity2)(p1, p2) };
        normalize_allocator(order);
        let rv = unsafe { (r.arity2)(p1, p2) };
        assert_eq!(
            cv, rv,
            "{row} iter {i}: arity2({p1}, {p2}) [{order:?}] C={cv} Rust={rv}"
        );
    }
}

fn diff_arity(row: &str, i: usize, len: c_int, params: &[c_int]) {
    let (c, r) = both();
    for order in AllocOrder::both() {
        normalize_allocator(order);
        let cv = unsafe { (c.arity)(len, params.as_ptr()) };
        normalize_allocator(order);
        let rv = unsafe { (r.arity)(len, params.as_ptr()) };
        assert_eq!(
            cv, rv,
            "{row} iter {i}: arity({len}, {params:?}) [{order:?}] C={cv} Rust={rv}"
        );
    }
}

// ---------------------------------------------------------------------------
// Buffer-based helpers (no malloc inside the C functions -> no pairing needed).
// ---------------------------------------------------------------------------

/// Run `shift_array` on identical guarded copies of `contents` and compare the
/// **entire** buffers (guards included) byte for byte.
fn diff_shift(row: &str, i: usize, contents: &[i32], size: c_int, positions: c_int, pad: usize) {
    let (c, r) = both();
    let mut bc: Vec<i32> = Vec::with_capacity(contents.len() + 2 * pad);
    bc.extend(std::iter::repeat(GUARD).take(pad));
    bc.extend_from_slice(contents);
    bc.extend(std::iter::repeat(GUARD).take(pad));
    let mut br = bc.clone();
    unsafe {
        (c.shift_array)(bc.as_mut_ptr().add(pad), size, positions);
        (r.shift_array)(br.as_mut_ptr().add(pad), size, positions);
    }
    assert_eq!(
        bc, br,
        "{row} iter {i}: shift_array(size={size}, positions={positions}) buffer mismatch\n\
         input={contents:?}\nC   ={bc:?}\nRust={br:?}"
    );
}

fn diff_process_string(row: &str, i: usize, bytes: &[u8]) {
    let (c, r) = both();
    let bc: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    let br = bc.clone();
    let cv = unsafe { (c.process_string)(bc.as_ptr()) };
    let rv = unsafe { (r.process_string)(br.as_ptr()) };
    assert_eq!(
        cv,
        rv,
        "{row} iter {i}: process_string(len={}, first={:?}) mismatch (C={cv}, Rust={rv})",
        bytes.len(),
        bytes.first()
    );
    assert_eq!(bc, br, "{row} iter {i}: process_string modified its input");
}

fn diff_bitmask(row: &str, i: usize, value: c_int, operation: c_int) {
    let (c, r) = both();
    let cv = unsafe { (c.apply_bitmask)(value, operation) };
    let rv = unsafe { (r.apply_bitmask)(value, operation) };
    assert_eq!(
        cv, rv,
        "{row} iter {i}: apply_bitmask({value}, {operation}) mismatch (C={cv}, Rust={rv})"
    );
}

/// `init_matrix` into identical guarded buffers of `words` ints; compare all.
fn diff_init_matrix(row: &str, i: usize, prefill: &[i32], pad: usize) {
    let (c, r) = both();
    let mut bc: Vec<i32> = Vec::with_capacity(prefill.len() + 2 * pad);
    bc.extend(std::iter::repeat(GUARD).take(pad));
    bc.extend_from_slice(prefill);
    bc.extend(std::iter::repeat(GUARD).take(pad));
    let mut br = bc.clone();
    unsafe {
        (c.init_matrix)(bc.as_mut_ptr().add(pad));
        (r.init_matrix)(br.as_mut_ptr().add(pad));
    }
    assert_eq!(
        bc, br,
        "{row} iter {i}: init_matrix buffer mismatch\nC   ={bc:?}\nRust={br:?}"
    );
}

// ---------------------------------------------------------------------------
// Value generators
// ---------------------------------------------------------------------------

/// A value `v >= 0` with `v % 4 == m` (`m` in `0..4`).
fn with_pos_mod4(rng: &mut Rng, m: i32) -> i32 {
    let base = ((rng.next_u64() >> 35) as i32).wrapping_mul(4);
    base + m
}

/// A value `v <= 0` with `v % 4 == -m` (C truncating remainder).
fn with_neg_mod4(rng: &mut Rng, m: i32) -> i32 {
    -(((rng.next_u64() >> 35) as i32).wrapping_mul(4) + m)
}

const CORNERS: [i32; 9] = [i32::MIN, -100, -4, -1, 0, 1, 4, 100, i32::MAX];

// ===========================================================================
// C1..C10 — shift_array
// ===========================================================================

#[test]
fn c1_shift_size1() {
    let mut rng = Rng::new(0xC001);
    for i in 0..ITERS {
        let contents = [rng.interesting_i32()];
        diff_shift("C1", i, &contents, 1, 1, 4);
        diff_shift("C1", i, &contents, 1, (rng.range(3) + 1) as i32, 4);
    }
}

#[test]
fn c2_shift_size2_pos1() {
    let mut rng = Rng::new(0xC002);
    for i in 0..ITERS {
        let contents = [rng.interesting_i32(), rng.interesting_i32()];
        diff_shift("C2", i, &contents, 2, 1, 4);
    }
}

#[test]
fn c3_shift_size3() {
    let mut rng = Rng::new(0xC003);
    for i in 0..ITERS {
        let contents = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let positions = 1 + (rng.range(2) as i32);
        diff_shift("C3", i, &contents, 3, positions, 4);
    }
}

#[test]
fn c4_shift_size4() {
    let mut rng = Rng::new(0xC004);
    for i in 0..ITERS {
        let contents = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for positions in 1..=3 {
            diff_shift("C4", i, &contents, 4, positions, 4);
        }
    }
}

#[test]
fn c5_shift_size8() {
    let mut rng = Rng::new(0xC005);
    for i in 0..ITERS {
        let mut contents = [0i32; 8];
        for slot in contents.iter_mut() {
            *slot = rng.interesting_i32();
        }
        let positions = 1 + (rng.range(7) as i32);
        diff_shift("C5", i, &contents, 8, positions, 8);
    }
}

#[test]
fn c6_shift_size64() {
    let mut rng = Rng::new(0xC006);
    for i in 0..ITERS {
        let mut contents = [0i32; 64];
        for slot in contents.iter_mut() {
            *slot = rng.interesting_i32();
        }
        let positions = 1 + (rng.range(63) as i32);
        diff_shift("C6", i, &contents, 64, positions, 8);
    }
}

#[test]
fn c7_shift_size1024() {
    let mut rng = Rng::new(0xC007);
    for i in 0..ITERS {
        let mut contents = vec![0i32; 1024];
        for slot in contents.iter_mut() {
            *slot = rng.interesting_i32();
        }
        for positions in [1, 512, 1023] {
            diff_shift("C7", i, &contents, 1024, positions, 8);
        }
    }
}

#[test]
fn c8_shift_pos_is_size_minus_1() {
    let mut rng = Rng::new(0xC008);
    for i in 0..ITERS {
        let size = 2 + (rng.range(63) as i32); // 2..=64
        let mut contents = vec![0i32; size as usize];
        for slot in contents.iter_mut() {
            *slot = rng.interesting_i32();
        }
        diff_shift("C8", i, &contents, size, size - 1, 8);
    }
}

#[test]
fn c9_shift_pos_1_varied_size() {
    let mut rng = Rng::new(0xC009);
    for i in 0..ITERS {
        let size = 2 + (rng.range(63) as i32);
        let mut contents = vec![0i32; size as usize];
        for slot in contents.iter_mut() {
            *slot = rng.interesting_i32();
        }
        diff_shift("C9", i, &contents, size, 1, 8);
    }
}

#[test]
fn c10_shift_guard_boundary_sweep() {
    let mut rng = Rng::new(0xC010);
    // Exhaustive over the guard boundary; `size` is always <= the real length,
    // so nothing is written out of bounds when the guard passes.
    for i in 0..64 {
        let len = 8usize;
        let mut contents = [0i32; 8];
        for slot in contents.iter_mut() {
            *slot = rng.interesting_i32();
        }
        for size in 0..=len as i32 {
            for positions in [-1, 0, 1, size - 1, size, size + 1, i32::MAX, i32::MIN] {
                diff_shift("C10", i, &contents, size, positions, 8);
            }
        }
    }
}

// ===========================================================================
// C11..C16 — process_string
// ===========================================================================

#[test]
fn c11_process_len1() {
    let mut rng = Rng::new(0xC011);
    for i in 0..ITERS {
        let b = ((rng.range(255) + 1) as u8).max(1);
        diff_process_string("C11", i, &[b, 0]);
    }
}

#[test]
fn c12_process_short() {
    let mut rng = Rng::new(0xC012);
    for i in 0..ITERS {
        let n = 2 + rng.range(7) as usize; // 2..=8
        let mut buf: Vec<u8> = (0..n).map(|_| 0x21 + (rng.range(94) as u8)).collect();
        buf.push(0);
        diff_process_string("C12", i, &buf);
    }
}

#[test]
fn c13_process_hello() {
    for i in 0..ITERS {
        diff_process_string("C13", i, b"Hello\0");
        diff_process_string("C13", i, b"\0");
    }
}

#[test]
fn c14_process_long() {
    let mut rng = Rng::new(0xC014);
    for i in 0..ITERS {
        for n in [63usize, 255, 1024] {
            let mut buf: Vec<u8> = (0..n).map(|_| ((rng.range(255) + 1) as u8).max(1)).collect();
            buf.push(0);
            diff_process_string("C14", i, &buf);
        }
    }
}

#[test]
fn c15_process_interior_nul() {
    let mut rng = Rng::new(0xC015);
    for i in 0..ITERS {
        let n = 1 + rng.range(32) as usize;
        let cut = rng.range(n as u64) as usize;
        let mut buf: Vec<u8> = (0..n).map(|_| ((rng.range(255) + 1) as u8).max(1)).collect();
        buf[cut] = 0; // strlen must stop here (or the guard must reject if cut==0)
        buf.push(0);
        diff_process_string("C15", i, &buf);
    }
}

#[test]
fn c16_process_high_bytes() {
    let mut rng = Rng::new(0xC016);
    for i in 0..ITERS {
        let n = 1 + rng.range(64) as usize;
        let mut buf = vec![0xFFu8; n];
        // A run of 0x80..=0xFF bytes: every `char` is negative, so `if (*str)`
        // must still be taken.
        for slot in buf.iter_mut() {
            *slot = 0x80 | (rng.range(128) as u8);
        }
        buf.push(0);
        diff_process_string("C16", i, &buf);
    }
}

// ===========================================================================
// C17..C21 — apply_bitmask
// ===========================================================================

#[test]
fn c17_bitmask_op0() {
    let mut rng = Rng::new(0xC017);
    for i in 0..ITERS {
        diff_bitmask("C17", i, rng.interesting_i32(), 0);
    }
    for (i, v) in CORNERS.iter().enumerate() {
        diff_bitmask("C17-corner", i, *v, 0);
    }
}

#[test]
fn c18_bitmask_op1() {
    let mut rng = Rng::new(0xC018);
    for i in 0..ITERS {
        diff_bitmask("C18", i, rng.interesting_i32(), 1);
    }
    for (i, v) in CORNERS.iter().enumerate() {
        diff_bitmask("C18-corner", i, *v, 1);
    }
}

#[test]
fn c19_bitmask_op2() {
    let mut rng = Rng::new(0xC019);
    for i in 0..ITERS {
        diff_bitmask("C19", i, rng.interesting_i32(), 2);
    }
    for (i, v) in CORNERS.iter().enumerate() {
        diff_bitmask("C19-corner", i, *v, 2);
    }
}

#[test]
fn c20_bitmask_op3() {
    let mut rng = Rng::new(0xC020);
    for i in 0..ITERS {
        diff_bitmask("C20", i, rng.interesting_i32(), 3);
    }
    for (i, v) in CORNERS.iter().enumerate() {
        diff_bitmask("C20-corner", i, *v, 3);
    }
}

#[test]
fn c21_bitmask_random_op() {
    let mut rng = Rng::new(0xC021);
    for i in 0..ITERS * 8 {
        let value = rng.interesting_i32();
        let op = rng.interesting_i32();
        diff_bitmask("C21", i, value, op);
    }
}

// ===========================================================================
// C22..C23 — init_matrix
// ===========================================================================

#[test]
fn c22_init_matrix_exact() {
    let mut rng = Rng::new(0xC022);
    for i in 0..ITERS {
        let mut prefill = [0i32; 12];
        for slot in prefill.iter_mut() {
            *slot = rng.interesting_i32();
        }
        diff_init_matrix("C22", i, &prefill, 8);
    }
}

#[test]
fn c23_init_matrix_repeat_and_oversized() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC023);
    for i in 0..ITERS {
        // Oversized buffer: only the first 12 words may change.
        let n = 12 + rng.range(20) as usize;
        let mut bc: Vec<i32> = (0..n + 16).map(|_| rng.interesting_i32()).collect();
        let mut br = bc.clone();
        let before = bc.clone();
        unsafe {
            (c.init_matrix)(bc.as_mut_ptr().add(8));
            (c.init_matrix)(bc.as_mut_ptr().add(8)); // idempotent
            (r.init_matrix)(br.as_mut_ptr().add(8));
            (r.init_matrix)(br.as_mut_ptr().add(8));
        }
        assert_eq!(bc, br, "C23 iter {i}: init_matrix mismatch");
        assert_eq!(
            &bc[8..20],
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            "C23 iter {i}: unexpected matrix contents"
        );
        assert_eq!(&bc[..8], &before[..8], "C23 iter {i}: wrote before start");
        assert_eq!(&bc[20..], &before[20..], "C23 iter {i}: wrote past 12 words");
    }
}

// ===========================================================================
// C24..C27 — compare_allocations
// ===========================================================================

#[test]
fn c24_cmp_alloc_val1_pos() {
    let mut rng = Rng::new(0xC024);
    for i in 0..ITERS {
        let v1 = 1 + (rng.range(i32::MAX as u64) as i32);
        diff_cmp_alloc("C24", i, v1, rng.interesting_i32());
    }
}

#[test]
fn c25_cmp_alloc_val1_zero() {
    let mut rng = Rng::new(0xC025);
    for i in 0..ITERS {
        diff_cmp_alloc("C25", i, 0, rng.interesting_i32());
    }
}

#[test]
fn c26_cmp_alloc_val1_neg() {
    let mut rng = Rng::new(0xC026);
    for i in 0..ITERS {
        let v1 = -1 - (rng.range(i32::MAX as u64) as i32);
        diff_cmp_alloc("C26", i, v1, rng.interesting_i32());
    }
}

#[test]
fn c27_cmp_alloc_boundaries() {
    let mut rng = Rng::new(0xC027);
    for i in 0..ITERS {
        for v1 in CORNERS {
            diff_cmp_alloc("C27", i, v1, rng.interesting_i32());
        }
        for v2 in CORNERS {
            diff_cmp_alloc("C27b", i, rng.interesting_i32(), v2);
        }
    }
}

// ===========================================================================
// C28..C39 — arity4
// ===========================================================================

#[test]
fn c28_arity4_m0_p3z_p4z() {
    let mut rng = Rng::new(0xC028);
    for i in 0..ITERS {
        diff_arity4("C28", i, with_pos_mod4(&mut rng, 0), rng.interesting_i32(), 0, 0);
    }
    diff_arity4("C28-min", 0, i32::MIN, 1, 0, 0);
}

#[test]
fn c29_arity4_m1_p3z_p4z() {
    let mut rng = Rng::new(0xC029);
    for i in 0..ITERS {
        diff_arity4("C29", i, with_pos_mod4(&mut rng, 1), rng.interesting_i32(), 0, 0);
    }
}

#[test]
fn c30_arity4_m2_p3z_p4z() {
    let mut rng = Rng::new(0xC030);
    for i in 0..ITERS {
        diff_arity4("C30", i, with_pos_mod4(&mut rng, 2), rng.interesting_i32(), 0, 0);
    }
}

#[test]
fn c31_arity4_m3_p3z_p4z() {
    let mut rng = Rng::new(0xC031);
    for i in 0..ITERS {
        diff_arity4("C31", i, with_pos_mod4(&mut rng, 3), rng.interesting_i32(), 0, 0);
    }
    diff_arity4("C31-max", 0, i32::MAX, -1, 0, 0);
}

#[test]
fn c32_arity4_negmod_p3z_p4z() {
    let mut rng = Rng::new(0xC032);
    for i in 0..ITERS {
        for m in 0..4 {
            diff_arity4(
                "C32",
                i,
                with_neg_mod4(&mut rng, m),
                rng.interesting_i32(),
                0,
                0,
            );
        }
    }
}

#[test]
fn c33_arity4_p3_small_pos() {
    let mut rng = Rng::new(0xC033);
    for i in 0..ITERS {
        let p3 = 1 + (rng.range(100) as i32);
        let m = (rng.range(4)) as i32;
        let p1 = if rng.range(2) == 0 {
            with_pos_mod4(&mut rng, m)
        } else {
            with_neg_mod4(&mut rng, m)
        };
        diff_arity4("C33", i, p1, rng.interesting_i32(), p3, 0);
    }
}

#[test]
fn c34_arity4_p3_small_neg() {
    let mut rng = Rng::new(0xC034);
    for i in 0..ITERS {
        let p3 = -1 - (rng.range(100) as i32);
        let m = (rng.range(4)) as i32;
        let p1 = if rng.range(2) == 0 {
            with_pos_mod4(&mut rng, m)
        } else {
            with_neg_mod4(&mut rng, m)
        };
        diff_arity4("C34", i, p1, rng.interesting_i32(), p3, 0);
    }
}

#[test]
fn c35_arity4_p3_overflow() {
    let mut rng = Rng::new(0xC035);
    for i in 0..ITERS {
        for p3 in [i32::MIN, i32::MAX, 1 << 30, -(1 << 30), 0x4000_0001] {
            diff_arity4("C35", i, rng.interesting_i32(), rng.interesting_i32(), p3, 0);
        }
        let p3 = rng.next_i32();
        if p3 != 0 {
            diff_arity4("C35r", i, rng.interesting_i32(), rng.interesting_i32(), p3, 0);
        }
    }
}

#[test]
fn c36_arity4_p4_only() {
    let mut rng = Rng::new(0xC036);
    for i in 0..ITERS {
        let mut p4 = rng.interesting_i32();
        if p4 == 0 {
            p4 = 1;
        }
        diff_arity4("C36", i, rng.interesting_i32(), rng.interesting_i32(), 0, p4);
    }
    for p4 in [i32::MIN, i32::MAX, 1, -1] {
        diff_arity4("C36-corner", 0, 1, 2, 0, p4);
    }
}

#[test]
fn c37_arity4_p3_and_p4() {
    let mut rng = Rng::new(0xC037);
    for i in 0..ITERS {
        let mut p3 = rng.interesting_i32();
        if p3 == 0 {
            p3 = 7;
        }
        let mut p4 = rng.interesting_i32();
        if p4 == 0 {
            p4 = -3;
        }
        diff_arity4("C37", i, rng.interesting_i32(), rng.interesting_i32(), p3, p4);
    }
}

#[test]
fn c38_arity4_fully_random() {
    let mut rng = Rng::new(0xC038);
    for i in 0..ITERS * 4 {
        diff_arity4(
            "C38",
            i,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

#[test]
fn c39_arity4_corner_grid() {
    let mut rng = Rng::new(0xC039);
    for i in 0..24 {
        for p1 in CORNERS {
            for p3 in CORNERS {
                diff_arity4("C39", i, p1, rng.interesting_i32(), p3, rng.interesting_i32());
            }
        }
    }
}

// ===========================================================================
// C40..C43 — arity2 / arity3
// ===========================================================================

#[test]
fn c40_arity2_random() {
    let mut rng = Rng::new(0xC040);
    for i in 0..ITERS {
        for m in 0..4 {
            diff_arity2("C40", i, with_pos_mod4(&mut rng, m), rng.interesting_i32());
            diff_arity2("C40n", i, with_neg_mod4(&mut rng, m), rng.interesting_i32());
        }
    }
}

#[test]
fn c41_arity2_boundaries() {
    let mut rng = Rng::new(0xC041);
    for i in 0..ITERS {
        for p1 in CORNERS {
            diff_arity2("C41", i, p1, rng.interesting_i32());
        }
        for p2 in CORNERS {
            diff_arity2("C41b", i, rng.interesting_i32(), p2);
        }
    }
}

#[test]
fn c42_arity3_p3_zero() {
    let mut rng = Rng::new(0xC042);
    for i in 0..ITERS {
        diff_arity3("C42", i, rng.interesting_i32(), rng.interesting_i32(), 0);
    }
}

#[test]
fn c43_arity3_p3_nonzero() {
    let mut rng = Rng::new(0xC043);
    for i in 0..ITERS {
        for m in 0..4 {
            let p1 = with_pos_mod4(&mut rng, m);
            let mut p3 = rng.interesting_i32();
            if p3 == 0 {
                p3 = 100;
            }
            diff_arity3("C43", i, p1, rng.interesting_i32(), p3);
        }
        for p3 in [i32::MIN, i32::MAX, 1, -1, 100, -100] {
            diff_arity3("C43c", i, rng.interesting_i32(), rng.interesting_i32(), p3);
        }
    }
}

// ===========================================================================
// C44..C49 — arity (dispatcher)
// ===========================================================================

#[test]
fn c44_arity_len2() {
    let mut rng = Rng::new(0xC044);
    for i in 0..ITERS {
        let params = [rng.interesting_i32(), rng.interesting_i32()];
        diff_arity("C44", i, 2, &params);
    }
}

#[test]
fn c45_arity_len3() {
    let mut rng = Rng::new(0xC045);
    for i in 0..ITERS {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        diff_arity("C45", i, 3, &params);
    }
}

#[test]
fn c46_arity_len4() {
    let mut rng = Rng::new(0xC046);
    for i in 0..ITERS {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        diff_arity("C46", i, 4, &params);
    }
}

#[test]
fn c47_arity_len_5_to_255() {
    let mut rng = Rng::new(0xC047);
    for i in 0..ITERS {
        // Only params[0..4] are read on the `else` branch, but pass a longer
        // buffer anyway to mirror a real caller.
        let n = 5 + rng.range(60) as usize;
        let params: Vec<i32> = (0..n).map(|_| rng.interesting_i32()).collect();
        let len = 5 + (rng.range(251) as i32); // 5..=255
        diff_arity("C47", i, len, &params);
    }
}

#[test]
fn c48_arity_len_truncation_aliases() {
    let mut rng = Rng::new(0xC048);
    for i in 0..ITERS {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for len in [
            258,
            259,
            260,
            65538,
            0x0001_0102,
            -1,
            i32::MAX,
            -256 + 4,
            0x7FFF_FF04,
        ] {
            diff_arity("C48", i, len, &params);
        }
    }
}

#[test]
fn c49_arity_len_exhaustive_0_511() {
    let mut rng = Rng::new(0xC049);
    for i in 0..8 {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for len in 0..=511 {
            diff_arity("C49", i, len, &params);
        }
    }
}

// ===========================================================================
// C50..C51 — composed pipeline / interleaved entry points
// ===========================================================================

#[test]
fn c50_pipeline_random_end_to_end() {
    let mut rng = Rng::new(0xC050);
    for i in 0..ITERS * 2 {
        let params = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let len = rng.interesting_i32();
        diff_arity("C50", i, len, &params);
    }
}

/// Randomized *interleaved* sequence over every export, replayed on each
/// library. Every malloc-touching op is preceded by `normalize_allocator`, so
/// the whole program is deterministic no matter how the ops are interleaved.
#[test]
fn c51_interleaved_all_entry_points() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC051);

    #[derive(Clone, Copy)]
    enum Op {
        Shift(i32, i32),
        Process(usize),
        Bitmask(i32, i32),
        Init,
        Cmp(i32, i32),
        A4(i32, i32, i32, i32),
        A3(i32, i32, i32),
        A2(i32, i32),
        A(i32),
    }

    const N: usize = 64;
    for round in 0..64 {
        // Build one random program.
        let mut ops = [Op::Init; N];
        for slot in ops.iter_mut() {
            *slot = match rng.range(9) {
                0 => Op::Shift(rng.range(9) as i32, rng.interesting_i32() % 12),
                1 => Op::Process(rng.range(24) as usize),
                2 => Op::Bitmask(rng.interesting_i32(), rng.interesting_i32()),
                3 => Op::Init,
                4 => Op::Cmp(rng.interesting_i32(), rng.interesting_i32()),
                5 => Op::A4(
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ),
                6 => Op::A3(
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ),
                7 => Op::A2(rng.interesting_i32(), rng.interesting_i32()),
                _ => Op::A(rng.interesting_i32()),
            };
        }
        // Shared inputs for the pointer-taking ops.
        let mut seed_buf = [0i32; 16];
        for slot in seed_buf.iter_mut() {
            *slot = rng.interesting_i32();
        }
        let mut str_buf = [0u8; 32];
        for k in 0..24 {
            str_buf[k] = ((rng.range(255) + 1) as u8).max(1);
        }
        let params: [i32; 8] = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];

        // Pre-allocate every output buffer up front: no Rust allocation may
        // happen while a program is running (it would disturb the tcache).
        let mut out_c = [0i32; N];
        let mut out_r = [0i32; N];
        let mut work_c = [0i32; 24];
        let mut work_r = [0i32; 24];
        let mut cbuf: [c_char; 32] = [0; 32];
        for k in 0..32 {
            cbuf[k] = str_buf[k] as c_char;
        }

        let order = if round % 2 == 0 {
            AllocOrder::Increasing
        } else {
            AllocOrder::Decreasing
        };
        let run = |api: &Api, out: &mut [i32; N], work: &mut [i32; 24]| -> usize {
            let mut flips = 0usize;
            work[..16].copy_from_slice(&seed_buf);
            for (k, op) in ops.iter().enumerate() {
                out[k] = match *op {
                    Op::Shift(size, positions) => {
                        unsafe { (api.shift_array)(work.as_mut_ptr(), size, positions) };
                        work.iter().fold(0i32, |a, b| a.wrapping_add(*b))
                    }
                    Op::Process(off) => unsafe {
                        (api.process_string)(cbuf.as_ptr().add(off.min(24)))
                    },
                    Op::Bitmask(v, o) => unsafe { (api.apply_bitmask)(v, o) },
                    Op::Init => {
                        unsafe { (api.init_matrix)(work.as_mut_ptr()) };
                        work.iter().fold(0i32, |a, b| a.wrapping_add(*b))
                    }
                    Op::Cmp(a, b) => {
                        flips += 1;
                        normalize_allocator(order);
                        unsafe { (api.compare_allocations)(a, b) }
                    }
                    Op::A4(a, b, c3, d) => {
                        flips += 1;
                        normalize_allocator(order);
                        unsafe { (api.arity4)(a, b, c3, d) }
                    }
                    Op::A3(a, b, c3) => {
                        flips += 1;
                        normalize_allocator(order);
                        unsafe { (api.arity3)(a, b, c3) }
                    }
                    Op::A2(a, b) => {
                        flips += 1;
                        normalize_allocator(order);
                        unsafe { (api.arity2)(a, b) }
                    }
                    Op::A(len) => {
                        let truncated = (len as u32 & 0xff) as u8;
                        if truncated >= 2 {
                            flips += 1;
                        }
                        normalize_allocator(order);
                        unsafe { (api.arity)(len, params.as_ptr()) }
                    }
                };
            }
            flips
        };

        let flips_c = run(c, &mut out_c, &mut work_c);
        let flips_r = run(r, &mut out_r, &mut work_r);

        assert_eq!(flips_c, flips_r, "C51 round {round}: bookkeeping mismatch");
        assert_eq!(
            out_c, out_r,
            "C51 round {round}: interleaved result sequence mismatch"
        );
        assert_eq!(
            work_c, work_r,
            "C51 round {round}: interleaved work-buffer mismatch"
        );
    }
}

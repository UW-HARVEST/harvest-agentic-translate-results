// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row drives BOTH the C `.so` and the
// Rust `.so` through their exported symbols with MANY randomized inputs
// (fixed seed => reproducible) and asserts byte-identical results.
//
// Ordering note: functions that reach `compare_allocations` are
// allocator-state dependent (see ERRORS.md Note B), so they are compared with
// parity-neutral 2-call batches via `assert_alloc_eq`.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ===========================================================================
// C1-C3: shift_array  (lowest-level entry point, out-parameter)
// ===========================================================================

/// Run `shift_array` on both libraries with identical inputs and compare the
/// ENTIRE buffer (contents + guard regions).
fn diff_shift(api_c: &Api, api_r: &Api, contents: &[c_int], size: c_int, positions: c_int) {
    let mut gc = Guarded::new(contents);
    let mut gr = Guarded::new(contents);
    unsafe {
        (api_c.shift_array)(gc.ptr(), size, positions);
        (api_r.shift_array)(gr.ptr(), size, positions);
    }
    assert_eq_diff(
        &format!("shift_array(len={}, size={size}, positions={positions})", contents.len()),
        gc.all(),
        gr.all(),
    );
    assert!(
        gc.guards_intact() && gr.guards_intact(),
        "shift_array wrote outside its buffer (size={size}, positions={positions})"
    );
}

#[test]
fn row_c01_shift_array_canonical_shape() {
    // size=4, positions=1: exactly the shape arity4() uses internally.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED);
    for _ in 0..2000 {
        let contents = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        diff_shift(&c, &r, &contents, 4, 1);
    }
    // plus explicit extreme contents
    for contents in [
        [i32::MIN, i32::MAX, 0, -1],
        [0, 0, 0, 0],
        [i32::MAX, i32::MAX, i32::MIN, i32::MIN],
    ] {
        diff_shift(&c, &r, &contents, 4, 1);
    }
}

#[test]
fn row_c02_shift_array_full_size_position_sweep() {
    // All sizes 1..=16 x all positions -2..=size+1 (covers the no-op guards
    // E1/E2/E3 and every in-range shift).
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC02);
    for size in 1..=16i32 {
        for positions in -2..=(size + 1) {
            for _ in 0..8 {
                let contents: Vec<c_int> = (0..size).map(|_| rng.next_i32()).collect();
                diff_shift(&c, &r, &contents, size, positions);
            }
        }
    }
    // size = 0 and negative sizes (guard can never pass)
    for size in [0i32, -1, -4, i32::MIN] {
        for positions in [-1i32, 0, 1, 7, i32::MAX] {
            let contents: Vec<c_int> = (0..4).map(|_| rng.next_i32()).collect();
            diff_shift(&c, &r, &contents, size, positions);
        }
    }
}

#[test]
fn row_c03_shift_array_large_shape() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC03);
    let size = 1024i32;
    for positions in [1i32, 2, 512, 1022, 1023, 1024] {
        for _ in 0..4 {
            let contents: Vec<c_int> = (0..size).map(|_| rng.next_i32()).collect();
            diff_shift(&c, &r, &contents, size, positions);
        }
    }
}

// ===========================================================================
// C4-C6: process_string
// ===========================================================================

fn diff_process_string(api_c: &Api, api_r: &Api, bytes: &[u8], label: &str) {
    // bytes must be NUL-terminated by the caller
    let buf: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    let rc = unsafe { (api_c.process_string)(buf.as_ptr()) };
    let rr = unsafe { (api_r.process_string)(buf.as_ptr()) };
    assert_eq_diff(label, rc, rr);
}

#[test]
fn row_c04_process_string_random_ascii() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC04);
    for _ in 0..1000 {
        let len = 1 + rng.below(256) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| 0x21 + (rng.below(0x5E) as u8)).collect();
        bytes.push(0);
        diff_process_string(&c, &r, &bytes, "process_string(random ascii)");
    }
}

#[test]
fn row_c05_process_string_high_bit_and_embedded_nul() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC05);
    // High-bit bytes => NEGATIVE `char` on x86-64 Linux (`char` is signed).
    for _ in 0..1000 {
        let len = 1 + rng.below(64) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| 0x80 | (rng.below(0x80) as u8)).collect();
        bytes.push(0);
        diff_process_string(&c, &r, &bytes, "process_string(high-bit)");
    }
    // Exactly 0x80 and 0xFF as the first byte (most negative / -1 char).
    for first in [0x80u8, 0xFF, 0x7F, 0x01] {
        diff_process_string(&c, &r, &[first, b'a', b'b', 0], "process_string(first byte)");
    }
    // Embedded NUL: strlen must stop at the first NUL.
    for _ in 0..500 {
        let pre = 1 + rng.below(16) as usize;
        let post = rng.below(16) as usize;
        let mut bytes: Vec<u8> = (0..pre).map(|_| 1 + (rng.below(255) as u8)).collect();
        bytes.push(0);
        bytes.extend((0..post).map(|_| 1 + (rng.below(255) as u8)));
        bytes.push(0);
        diff_process_string(&c, &r, &bytes, "process_string(embedded NUL)");
    }
}

#[test]
fn row_c06_process_string_long_buffer() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC06);
    for len in [1usize, 2, 5, 255, 256, 257, 4096] {
        let mut bytes: Vec<u8> = (0..len).map(|_| 1 + (rng.below(255) as u8)).collect();
        bytes.push(0);
        diff_process_string(&c, &r, &bytes, "process_string(long)");
    }
}

// ===========================================================================
// C7: apply_bitmask  (switch over `operation`)
// ===========================================================================

#[test]
fn row_c07_apply_bitmask_all_branches() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC07);
    // Each handled case 0..=3 with boundary and random values.
    for operation in 0..=3i32 {
        for &value in BOUNDARY.iter() {
            let rc = unsafe { (c.apply_bitmask)(value, operation) };
            let rr = unsafe { (r.apply_bitmask)(value, operation) };
            assert_eq_diff(&format!("apply_bitmask({value}, {operation})"), rc, rr);
        }
        for _ in 0..2000 {
            let value = rng.next_i32();
            let rc = unsafe { (c.apply_bitmask)(value, operation) };
            let rr = unsafe { (r.apply_bitmask)(value, operation) };
            assert_eq_diff(&format!("apply_bitmask({value}, {operation})"), rc, rr);
        }
    }
    // Fully random (value, operation) pairs: mostly the `default` branch,
    // including out-of-range "enum" ints crossing the FFI boundary.
    for _ in 0..2000 {
        let value = rng.next_i32();
        let operation = rng.next_i32();
        let rc = unsafe { (c.apply_bitmask)(value, operation) };
        let rr = unsafe { (r.apply_bitmask)(value, operation) };
        assert_eq_diff(&format!("apply_bitmask({value}, {operation})"), rc, rr);
    }
}

// ===========================================================================
// C8-C9: compare_allocations  (allocator-state dependent)
// ===========================================================================

#[test]
fn row_c08_compare_allocations_positive_val1() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC08);
    for _ in 0..500 {
        let val1 = 1 + (rng.below(i32::MAX as u64) as i32); // > 0 => +10
        let val2 = rng.next_i32();
        assert_alloc_eq(
            &format!("compare_allocations({val1}, {val2})"),
            || unsafe { (c.compare_allocations)(val1, val2) },
            || unsafe { (r.compare_allocations)(val1, val2) },
        );
    }
    for val1 in [1i32, 2, 100, i32::MAX - 1, i32::MAX] {
        for &val2 in BOUNDARY.iter() {
            assert_alloc_eq(
                &format!("compare_allocations({val1}, {val2})"),
                || unsafe { (c.compare_allocations)(val1, val2) },
                || unsafe { (r.compare_allocations)(val1, val2) },
            );
        }
    }
}

#[test]
fn row_c09_compare_allocations_nonpositive_val1() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC09);
    for _ in 0..500 {
        let val1 = -(rng.below(i32::MAX as u64) as i32); // <= 0 => +0
        let val2 = rng.next_i32();
        assert_alloc_eq(
            &format!("compare_allocations({val1}, {val2})"),
            || unsafe { (c.compare_allocations)(val1, val2) },
            || unsafe { (r.compare_allocations)(val1, val2) },
        );
    }
    for val1 in [0i32, -1, -100, i32::MIN + 1, i32::MIN] {
        for &val2 in BOUNDARY.iter() {
            assert_alloc_eq(
                &format!("compare_allocations({val1}, {val2})"),
                || unsafe { (c.compare_allocations)(val1, val2) },
                || unsafe { (r.compare_allocations)(val1, val2) },
            );
        }
    }
}

// ===========================================================================
// C10: init_matrix
// ===========================================================================

#[test]
fn row_c10_init_matrix() {
    let (c, r) = load_both();
    // Pre-fill with junk so we can see exactly which cells get written.
    let mut rng = Rng::new(SEED ^ 0xC10);
    for _ in 0..200 {
        let junk: Vec<c_int> = (0..12).map(|_| rng.next_i32()).collect();
        let mut gc = Guarded::new(&junk);
        let mut gr = Guarded::new(&junk);
        unsafe {
            (c.init_matrix)(gc.ptr());
            (r.init_matrix)(gr.ptr());
        }
        assert_eq_diff("init_matrix (full buffer incl. guards)", gc.all(), gr.all());
        assert!(
            gc.guards_intact() && gr.guards_intact(),
            "init_matrix wrote outside the 3x4 matrix"
        );
        // Sanity: it really is the documented matrix.
        assert_eq!(gc.data(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }
}

// ===========================================================================
// C11-C19: arity4  (mid-level entry point; the whole internal pipeline)
// ===========================================================================

fn diff_arity4(api_c: &Api, api_r: &Api, p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    assert_alloc_eq(
        &format!("arity4({p1}, {p2}, {p3}, {p4})"),
        || unsafe { (api_c.arity4)(p1, p2, p3, p4) },
        || unsafe { (api_r.arity4)(p1, p2, p3, p4) },
    );
}

#[test]
fn row_c11_arity4_mod4_zero() {
    // param1 % 4 == 0 => apply_bitmask case 0 (value & 0xF0)
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC11);
    for _ in 0..400 {
        let p1 = rng.i32_with_mod4(0);
        diff_arity4(&c, &r, p1, rng.next_i32(), 0, 0);
    }
    for p1 in [0i32, 4, -4, 100, -100, 2147483644] {
        diff_arity4(&c, &r, p1, 7, 0, 0);
    }
}

#[test]
fn row_c12_arity4_mod4_positive_cases() {
    // param1 % 4 in {1,2,3} => apply_bitmask cases 1, 2, 3
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC12);
    for target in 1..=3i32 {
        for _ in 0..400 {
            let p1 = rng.i32_with_mod4(target);
            diff_arity4(&c, &r, p1, rng.next_i32(), 0, 0);
        }
    }
    for p1 in [1i32, 2, 3, 5, 6, 7, i32::MAX, i32::MAX - 1, i32::MAX - 2] {
        diff_arity4(&c, &r, p1, 7, 0, 0);
    }
}

#[test]
fn row_c13_arity4_mod4_negative_cases() {
    // Negative param1: C's `%` truncates toward zero, so param1 % 4 is
    // NEGATIVE (-1/-2/-3) and apply_bitmask takes the `default` branch.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC13);
    for target in [-1i32, -2, -3] {
        for _ in 0..400 {
            let p1 = rng.i32_with_mod4(target);
            diff_arity4(&c, &r, p1, rng.next_i32(), 0, 0);
        }
    }
    for p1 in [-1i32, -2, -3, -5, -6, -7, i32::MIN, i32::MIN + 1, i32::MIN + 2] {
        diff_arity4(&c, &r, p1, 7, 0, 0);
    }
}

#[test]
fn row_c14_arity4_param3_only() {
    // param3 != 0, param4 == 0 => result = result * param3 / 100
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC14);
    for _ in 0..1000 {
        diff_arity4(&c, &r, rng.next_i32(), rng.next_i32(), rng.nonzero_i32(), 0);
    }
    for p3 in [1i32, -1, 99, 100, 101, -100, i32::MAX, i32::MIN] {
        for p1 in [-3i32, 0, 1, 2, 3, 1000] {
            diff_arity4(&c, &r, p1, 5, p3, 0);
        }
    }
}

#[test]
fn row_c15_arity4_param4_only() {
    // param3 == 0, param4 != 0 => result += param4
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC15);
    for _ in 0..1000 {
        diff_arity4(&c, &r, rng.next_i32(), rng.next_i32(), 0, rng.nonzero_i32());
    }
    for p4 in [1i32, -1, i32::MAX, i32::MIN, 100, -100] {
        for p1 in [-3i32, 0, 1, 2, 3] {
            diff_arity4(&c, &r, p1, 5, 0, p4);
        }
    }
}

#[test]
fn row_c16_arity4_param3_and_param4() {
    // Both post-adjustments composed.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC16);
    for _ in 0..1500 {
        diff_arity4(
            &c,
            &r,
            rng.next_i32(),
            rng.next_i32(),
            rng.nonzero_i32(),
            rng.nonzero_i32(),
        );
    }
}

#[test]
fn row_c17_arity4_boundary_cross_product() {
    // Full 10^4 cross-product of interesting boundary values: exercises the
    // wrapping `result * param3`, the `/100` truncation toward zero, and the
    // wrapping `result + param4`.
    let (c, r) = load_both();
    for &p1 in BOUNDARY.iter() {
        for &p2 in BOUNDARY.iter() {
            for &p3 in BOUNDARY.iter() {
                for &p4 in BOUNDARY.iter() {
                    diff_arity4(&c, &r, p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn row_c18_arity4_random_tuples() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC18);
    for _ in 0..5000 {
        diff_arity4(
            &c,
            &r,
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

#[test]
fn row_c19_arity4_sign_of_param1_x_p3_p4_matrix() {
    // param1 > 0 vs <= 0 drives compare_allocations' `+10`; cross that with
    // the param3/param4 on-off matrix.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC19);
    for positive in [true, false] {
        for &(p3_on, p4_on) in [(false, false), (true, false), (false, true), (true, true)].iter() {
            for _ in 0..250 {
                let mag = rng.below(i32::MAX as u64) as i32;
                let p1 = if positive { 1 + mag / 2 } else { -(mag / 2) };
                let p3 = if p3_on { rng.nonzero_i32() } else { 0 };
                let p4 = if p4_on { rng.nonzero_i32() } else { 0 };
                diff_arity4(&c, &r, p1, rng.next_i32(), p3, p4);
            }
        }
    }
}

// ===========================================================================
// C20-C23: arity2 / arity3 wrappers
// ===========================================================================

#[test]
fn row_c20_arity2_random() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC20);
    for _ in 0..2000 {
        let (p1, p2) = (rng.next_i32(), rng.next_i32());
        assert_alloc_eq(
            &format!("arity2({p1}, {p2})"),
            || unsafe { (c.arity2)(p1, p2) },
            || unsafe { (r.arity2)(p1, p2) },
        );
        // The wrapper must equal the underlying call on EACH library
        // (compared at matching allocator parity).
        assert_alloc_eq(
            &format!("C: arity2({p1},{p2}) vs arity4({p1},{p2},0,0)"),
            || unsafe { (c.arity2)(p1, p2) },
            || unsafe { (c.arity4)(p1, p2, 0, 0) },
        );
        assert_alloc_eq(
            &format!("Rust: arity2({p1},{p2}) vs arity4({p1},{p2},0,0)"),
            || unsafe { (r.arity2)(p1, p2) },
            || unsafe { (r.arity4)(p1, p2, 0, 0) },
        );
    }
}

#[test]
fn row_c21_arity2_boundary_cross_product() {
    let (c, r) = load_both();
    for &p1 in BOUNDARY.iter() {
        for &p2 in BOUNDARY.iter() {
            assert_alloc_eq(
                &format!("arity2({p1}, {p2})"),
                || unsafe { (c.arity2)(p1, p2) },
                || unsafe { (r.arity2)(p1, p2) },
            );
        }
    }
}

#[test]
fn row_c22_arity3_random() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC22);
    for i in 0..2000 {
        let (p1, p2) = (rng.next_i32(), rng.next_i32());
        // alternate p3 == 0 and p3 != 0
        let p3 = if i % 2 == 0 { 0 } else { rng.nonzero_i32() };
        assert_alloc_eq(
            &format!("arity3({p1}, {p2}, {p3})"),
            || unsafe { (c.arity3)(p1, p2, p3) },
            || unsafe { (r.arity3)(p1, p2, p3) },
        );
        assert_alloc_eq(
            &format!("C: arity3({p1},{p2},{p3}) vs arity4(..,0)"),
            || unsafe { (c.arity3)(p1, p2, p3) },
            || unsafe { (c.arity4)(p1, p2, p3, 0) },
        );
        assert_alloc_eq(
            &format!("Rust: arity3({p1},{p2},{p3}) vs arity4(..,0)"),
            || unsafe { (r.arity3)(p1, p2, p3) },
            || unsafe { (r.arity4)(p1, p2, p3, 0) },
        );
    }
}

#[test]
fn row_c23_arity3_boundary_cross_product() {
    let (c, r) = load_both();
    for &p1 in BOUNDARY.iter() {
        for &p2 in BOUNDARY.iter() {
            for &p3 in BOUNDARY.iter() {
                assert_alloc_eq(
                    &format!("arity3({p1}, {p2}, {p3})"),
                    || unsafe { (c.arity3)(p1, p2, p3) },
                    || unsafe { (r.arity3)(p1, p2, p3) },
                );
            }
        }
    }
}

// ===========================================================================
// C24-C31: arity  (public dispatcher; 8-bit truncation of `len`)
// ===========================================================================

/// Compare `arity(len, params)` on both libraries; `params` is a guarded copy
/// per library so we can also prove neither writes to it.
fn diff_arity(api_c: &Api, api_r: &Api, len: c_int, params: &[c_int]) {
    let mut gc = Guarded::new(params);
    let mut gr = Guarded::new(params);
    let pc = gc.ptr();
    let pr = gr.ptr();
    assert_alloc_eq(
        &format!("arity({len}, {params:?})"),
        || unsafe { (api_c.arity)(len, pc) },
        || unsafe { (api_r.arity)(len, pr) },
    );
    assert_eq_diff(
        &format!("arity({len}) params buffer must be unmodified"),
        gc.all(),
        gr.all(),
    );
    assert_eq!(gc.data(), params, "C arity modified its params buffer");
    assert_eq!(gr.data(), params, "Rust arity modified its params buffer");
}

#[test]
fn row_c24_arity_len2_dispatch() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC24);
    for _ in 0..1000 {
        // 4 elements supplied but only the first 2 may be read.
        let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        diff_arity(&c, &r, 2, &params);
    }
    for &p0 in BOUNDARY.iter() {
        for &p1 in BOUNDARY.iter() {
            diff_arity(&c, &r, 2, &[p0, p1, 12345, -999]);
        }
    }
}

#[test]
fn row_c25_arity_len3_dispatch() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC25);
    for _ in 0..1000 {
        let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        diff_arity(&c, &r, 3, &params);
    }
    for &p2 in BOUNDARY.iter() {
        diff_arity(&c, &r, 3, &[7, -7, p2, 424242]);
    }
}

#[test]
fn row_c26_arity_len4_dispatch() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC26);
    for _ in 0..1000 {
        let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        diff_arity(&c, &r, 4, &params);
    }
    for &p3 in BOUNDARY.iter() {
        diff_arity(&c, &r, 4, &[3, -3, 200, p3]);
    }
}

#[test]
fn row_c27_arity_len_gt4_reads_only_four() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC27);
    for len in [5i32, 6, 10, 100, 254, 255] {
        for _ in 0..40 {
            // Provide `len` elements; only params[0..3] may influence output.
            let params: Vec<c_int> = (0..len).map(|_| rng.next_i32()).collect();
            diff_arity(&c, &r, len, &params);
        }
    }
}

#[test]
fn row_c28_arity_len_truncates_to_valid_dispatch() {
    // High bits are dropped by the `unsigned char` parameter.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC28);
    for len in [258i32, 259, 260, 261, 65538, 65539, 65540, 0x7FFF_FF04] {
        for _ in 0..40 {
            let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
            diff_arity(&c, &r, len, &params);
        }
    }
}

#[test]
fn row_c29_arity_negative_len_truncates_to_ge4() {
    // -1 -> 0xFF = 255, -2 -> 254, -4 -> 252: all >= 4 => arity4 path.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC29);
    for len in [-1i32, -2, -3, -4, -5, -100, i32::MIN + 1, -253] {
        for _ in 0..40 {
            let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
            diff_arity(&c, &r, len, &params);
        }
    }
}

#[test]
fn row_c30_arity_all_256_low_byte_values() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC30);
    for low in 0..=255i32 {
        // The same low byte reached through several different `int` values.
        for high in [0i32, 0x100, 0x1_0000, -0x1_0000] {
            let len = high + low;
            let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
            diff_arity(&c, &r, len, &params);
        }
    }
    // Fully random `int` lengths: `len` is the most ABI-subtle parameter of the
    // library (declared `int` in the header, defined `unsigned char`), so drive
    // it with arbitrary 32-bit values as well.
    for _ in 0..2000 {
        let len = rng.next_i32();
        let params = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        diff_arity(&c, &r, len, &params);
    }
}

#[test]
fn row_c31_arity_params_offset_into_larger_buffer() {
    // params need not point at the start of an allocation.
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0xC31);
    for _ in 0..300 {
        let big: Vec<c_int> = (0..32).map(|_| rng.next_i32()).collect();
        let offset = rng.below(28) as usize;
        let len = 2 + (rng.below(3) as c_int); // 2, 3 or 4
        let slice = &big[offset..offset + 4];
        diff_arity(&c, &r, len, slice);
    }
}


// ===========================================================================
// C32: composed pipeline -- long call sequence, one library per fresh process
// ===========================================================================
//
// A long sequence CANNOT be compared inside one process: `compare_allocations`
// reads the glibc tcache, so the second sequence in a process starts from an
// allocator state that the first sequence itself changed (and unrelated
// same-size-class allocations shift which chunks are on top -- neither counting
// calls nor probing the state in-process is sufficient). Each library therefore
// runs its sequence in its OWN freshly spawned process, where the allocator is
// pristine and evolves identically for both. This is also how a real consumer
// experiences the library.

const SEQ_TEST: &str = "row_c32_long_composed_sequence";
const SEQ_N: usize = 1000;

/// Build the (deterministic) input sequence used by both children.
fn seq_inputs() -> Vec<(c_int, [c_int; 4])> {
    let mut rng = Rng::new(SEED ^ 0xC32);
    let mut inputs = Vec::with_capacity(SEQ_N);
    for _ in 0..SEQ_N {
        let len = match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            _ => rng.next_i32(),
        };
        inputs.push((
            len,
            [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()],
        ));
    }
    inputs
}

#[test]
fn row_c32_long_composed_sequence() {
    if let Some(which) = seq_child_lib() {
        // ---- child: drive ONE library through the whole sequence ----------
        let (c, r) = load_both();
        let api = if which == "c" { &c } else { &r };
        let inputs = seq_inputs();
        let mut out: Vec<c_int> = Vec::with_capacity(2 * SEQ_N);
        let mut params: Vec<c_int> = vec![0; 4];
        // Interleave the dispatcher and the low-level entry point so the
        // allocator state, mask selection and post-adjustments all compose.
        for &(len, p) in inputs.iter() {
            params.copy_from_slice(&p);
            out.push(unsafe { (api.arity)(len, params.as_mut_ptr()) });
            out.push(unsafe { (api.arity4)(p[0], p[1], p[2], p[3]) });
        }
        // Print only after every call is done, so formatting cannot disturb
        // the allocator mid-sequence.
        let mut line = String::from("SEQ:");
        for v in out {
            line.push(' ');
            line.push_str(&v.to_string());
        }
        println!("{line}");
        return;
    }

    // ---- parent: run both children and compare their sequences ------------
    let c_seq = run_seq_child(SEQ_TEST, "c");
    let r_seq = run_seq_child(SEQ_TEST, "r");

    let cv: Vec<&str> = c_seq.split_whitespace().skip(1).collect();
    let rv: Vec<&str> = r_seq.split_whitespace().skip(1).collect();
    assert_eq!(
        cv.len(),
        2 * SEQ_N,
        "child produced {} values, expected {}",
        cv.len(),
        2 * SEQ_N
    );
    assert_eq!(cv.len(), rv.len(), "sequence lengths differ");

    let inputs = seq_inputs();
    for i in 0..cv.len() {
        if cv[i] != rv[i] {
            let (len, p) = inputs[i / 2];
            let entry = if i % 2 == 0 { "arity" } else { "arity4" };
            panic!(
                "DIVERGENCE at sequence index {i} ({entry}): len={len} params={p:?}\n  \
                 C   -> {}\n  Rust-> {}",
                cv[i], rv[i]
            );
        }
    }
    println!("composed sequence: {} calls identical", cv.len());
}

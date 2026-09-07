//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both libraries are loaded through
//! `libloading` and every call crosses the FFI boundary via the dynamic
//! symbol table.

mod common;
use common::*;
use std::ffi::c_int;

// ===========================================================================
// apply_bitmask — rows 1..6
// ===========================================================================

fn bitmask_row(row: &str, op: c_int, n: usize) {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ op as u64);
    for _ in 0..n {
        let v = rng.spicy_i32();
        unsafe { eq_i32(row, (v, op), (c.apply_bitmask)(v, op), (r.apply_bitmask)(v, op)) };
    }
    // plus the fixed boundary values
    for &v in &[0i32, 1, -1, 0xFF, 0xF0, 0x0F, 0xAA, 0x55, i32::MIN, i32::MAX] {
        unsafe { eq_i32(row, (v, op), (c.apply_bitmask)(v, op), (r.apply_bitmask)(v, op)) };
    }
}

#[test]
fn cfg_01_apply_bitmask_op0() {
    bitmask_row("cfg-01", 0, 2000);
}
#[test]
fn cfg_02_apply_bitmask_op1() {
    bitmask_row("cfg-02", 1, 2000);
}
#[test]
fn cfg_03_apply_bitmask_op2() {
    bitmask_row("cfg-03", 2, 2000);
}
#[test]
fn cfg_04_apply_bitmask_op3() {
    bitmask_row("cfg-04", 3, 2000);
}

#[test]
fn cfg_05_apply_bitmask_op_out_of_range() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    // Random operations over the whole i32 range: nearly all hit `default:`.
    for _ in 0..4000 {
        let op = rng.spicy_i32();
        let v = rng.spicy_i32();
        unsafe { eq_i32("cfg-05", (v, op), (c.apply_bitmask)(v, op), (r.apply_bitmask)(v, op)) };
    }
    // Exhaustive small neighbourhood around the valid case labels, plus the
    // extreme "enum value with no variant" inputs.
    let mut ops: Vec<c_int> = (-8..=8).collect();
    ops.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 255, 256, -255]);
    for &op in &ops {
        for _ in 0..64 {
            let v = rng.spicy_i32();
            unsafe { eq_i32("cfg-05", (v, op), (c.apply_bitmask)(v, op), (r.apply_bitmask)(v, op)) };
        }
    }
}

#[test]
fn cfg_06_apply_bitmask_boundary_cross_product() {
    let (c, r) = both();
    let values = [
        0i32, 1, -1, 2, -2, 0xFF, 0xF0, 0x0F, 0xAA, 0x55, 0xFFFF, -0xFF,
        i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
    ];
    // Every value x every operation in -4..=8: full cross-product.
    for &v in &values {
        for op in -4i32..=8 {
            unsafe { eq_i32("cfg-06", (v, op), (c.apply_bitmask)(v, op), (r.apply_bitmask)(v, op)) };
        }
    }
}

// ===========================================================================
// process_string — rows 7..11
// ===========================================================================

#[test]
fn cfg_07_process_string_single_byte_all_values() {
    let (c, r) = both();
    // Every possible non-NUL single byte, incl. 0x80..0xFF where `char` is
    // signed on x86-64 Linux (so `if (*str)` sees a negative value).
    for b in 1u16..=255 {
        let buf = cbuf(&[b as u8]);
        unsafe {
            eq_i32("cfg-07", b, (c.process_string)(buf.as_ptr()), (r.process_string)(buf.as_ptr()))
        };
    }
}

#[test]
fn cfg_08_process_string_random_lengths() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..2000 {
        let len = rng.range_usize(2, 64);
        let bytes: Vec<u8> = (0..len).map(|_| rng.u8_nonzero()).collect();
        let buf = cbuf(&bytes);
        unsafe {
            eq_i32(
                "cfg-08",
                len,
                (c.process_string)(buf.as_ptr()),
                (r.process_string)(buf.as_ptr()),
            )
        };
    }
}

#[test]
fn cfg_09_process_string_long() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for &len in &[128usize, 255, 256, 257, 1023, 1024, 4096, 8191] {
        let bytes: Vec<u8> = (0..len).map(|_| rng.u8_nonzero()).collect();
        let buf = cbuf(&bytes);
        unsafe {
            eq_i32(
                "cfg-09",
                len,
                (c.process_string)(buf.as_ptr()),
                (r.process_string)(buf.as_ptr()),
            )
        };
    }
}

#[test]
fn cfg_10_process_string_interior_nul() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    // strlen must stop at the interior NUL even though the buffer continues.
    for _ in 0..1000 {
        let total = rng.range_usize(2, 64);
        let nul_at = rng.range_usize(1, total - 1); // >=1 so *str != 0
        let mut bytes: Vec<u8> = (0..total).map(|_| rng.u8_nonzero()).collect();
        bytes[nul_at] = 0;
        let buf = cbuf(&bytes);
        unsafe {
            eq_i32(
                "cfg-10",
                (total, nul_at),
                (c.process_string)(buf.as_ptr()),
                (r.process_string)(buf.as_ptr()),
            )
        };
    }
}

#[test]
fn cfg_11_process_string_high_bit_bytes() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    // Only 0x80..=0xFF: on this target `char` is signed, so `*str` is < 0 but
    // still truthy. Both implementations must agree.
    for _ in 0..1000 {
        let len = rng.range_usize(1, 32);
        let bytes: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() % 0x80) as u8).collect();
        let buf = cbuf(&bytes);
        unsafe {
            eq_i32(
                "cfg-11",
                len,
                (c.process_string)(buf.as_ptr()),
                (r.process_string)(buf.as_ptr()),
            )
        };
    }
}

// ===========================================================================
// init_matrix — rows 12..13
// ===========================================================================

/// Runs `init_matrix` on a 12-slot window inside a larger buffer that is
/// fenced with guard cells, so an over-write past the 3x4 region is detected.
fn init_matrix_once(f: FnInitMatrix, fill: c_int) -> Vec<c_int> {
    const GUARD: usize = 8;
    let mut buf = vec![fill; GUARD + 12 + GUARD];
    unsafe { f(buf.as_mut_ptr().add(GUARD)) };
    buf
}

#[test]
fn cfg_12_init_matrix_exact_writes() {
    let (c, r) = both();
    let cb = init_matrix_once(c.init_matrix, -9999);
    let rb = init_matrix_once(r.init_matrix, -9999);
    eq_slice("cfg-12", "full buffer incl. guards", &cb, &rb);
    // And confirm the ground truth: 1..12 row-major, guards untouched.
    assert_eq!(&cb[8..20], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    assert!(cb[..8].iter().all(|&x| x == -9999), "C wrote before the matrix");
    assert!(cb[20..].iter().all(|&x| x == -9999), "C wrote past the matrix");
}

#[test]
fn cfg_13_init_matrix_overwrites_dirty_buffer() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..500 {
        let fill = rng.i32();
        let cb = init_matrix_once(c.init_matrix, fill);
        let rb = init_matrix_once(r.init_matrix, fill);
        eq_slice("cfg-13", fill, &cb, &rb);
    }
    // Called twice on the same buffer: must be idempotent and identical.
    const N: usize = 12;
    let mut cb = vec![0 as c_int; N];
    let mut rb = vec![0 as c_int; N];
    for _ in 0..3 {
        unsafe {
            (c.init_matrix)(cb.as_mut_ptr());
            (r.init_matrix)(rb.as_mut_ptr());
        }
        eq_slice("cfg-13/idempotent", "repeat", &cb, &rb);
    }
}

// ===========================================================================
// shift_array — rows 14..19
// ===========================================================================

/// Applies `shift_array` to identical copies of `data` in both libraries and
/// returns the two resulting buffers. Guard cells on both sides catch strays.
fn shift_pair(c: &Lib, r: &Lib, data: &[c_int], size: c_int, positions: c_int) -> (Vec<c_int>, Vec<c_int>) {
    const GUARD: usize = 4;
    const G: c_int = 0x7BAD_BAD1u32 as c_int;
    let mk = || {
        let mut v = vec![G; GUARD];
        v.extend_from_slice(data);
        v.extend(std::iter::repeat(G).take(GUARD));
        v
    };
    let mut cb = mk();
    let mut rb = mk();
    unsafe {
        (c.shift_array)(cb.as_mut_ptr().add(GUARD), size, positions);
        (r.shift_array)(rb.as_mut_ptr().add(GUARD), size, positions);
    }
    (cb, rb)
}

#[test]
fn cfg_14_shift_array_size4_pos1() {
    // The exact shape arity4() uses internally.
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..2000 {
        let data: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
        let (cb, rb) = shift_pair(&c, &r, &data, 4, 1);
        eq_slice("cfg-14", &data, &cb, &rb);
        // ground truth: {0, d0, d1, d2}
        assert_eq!(&cb[4..8], &[0, data[0], data[1], data[2]]);
    }
}

#[test]
fn cfg_15_shift_array_random_size_and_positions() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..3000 {
        let size = rng.range_i32(1, 64);
        // positions anywhere in 1..size keeps the guard true; also allow the
        // out-of-guard values so the no-op path is mixed in.
        let positions = rng.range_i32(-2, size + 2);
        let data: Vec<c_int> = (0..size as usize).map(|_| rng.spicy_i32()).collect();
        let (cb, rb) = shift_pair(&c, &r, &data, size, positions);
        eq_slice("cfg-15", (size, positions), &cb, &rb);
    }
}

#[test]
fn cfg_16_shift_array_positions_eq_size_minus_1() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for size in 2i32..=64 {
        let data: Vec<c_int> = (0..size as usize).map(|_| rng.spicy_i32()).collect();
        let (cb, rb) = shift_pair(&c, &r, &data, size, size - 1);
        eq_slice("cfg-16", size, &cb, &rb);
        // memmove length is exactly one element
        assert_eq!(cb[4 + size as usize - 1], data[0]);
    }
}

#[test]
fn cfg_17_shift_array_overlapping_middle() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for &size in &[8i32, 16, 32, 64] {
        for positions in 1..size {
            let data: Vec<c_int> = (0..size as usize).map(|_| rng.spicy_i32()).collect();
            let (cb, rb) = shift_pair(&c, &r, &data, size, positions);
            eq_slice("cfg-17", (size, positions), &cb, &rb);
        }
    }
}

#[test]
fn cfg_18_shift_array_size1_always_noop() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..200 {
        let data = vec![rng.spicy_i32()];
        for positions in -3i32..=3 {
            let (cb, rb) = shift_pair(&c, &r, &data, 1, positions);
            eq_slice("cfg-18", positions, &cb, &rb);
            assert_eq!(cb[4], data[0], "size=1 must never be modified");
        }
    }
}

#[test]
fn cfg_19_shift_array_size2_pos1() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..1000 {
        let data: Vec<c_int> = (0..2).map(|_| rng.spicy_i32()).collect();
        let (cb, rb) = shift_pair(&c, &r, &data, 2, 1);
        eq_slice("cfg-19", &data, &cb, &rb);
        assert_eq!(&cb[4..6], &[0, data[0]]);
    }
}

// ===========================================================================
// compare_allocations — rows 20..22   (heap-normalized, both orders)
// ===========================================================================

#[test]
fn cfg_20_compare_allocations_positive_val1() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..1000 {
            let v1 = rng.range_i32(1, i32::MAX);
            let v2 = rng.i32();
            let cv = diff_norm("cfg-20", (asc, v1, v2), asc,
                &mut || unsafe { (c.compare_allocations)(v1, v2) },
                &mut || unsafe { (r.compare_allocations)(v1, v2) });
            // ground truth: val1 > 0 -> +10 arm
            assert_eq!(cv, if asc { 11 } else { 12 }, "asc={asc} v1={v1}");
        }
    }
}

#[test]
fn cfg_21_compare_allocations_nonpositive_val1() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..1000 {
            let v1 = rng.range_i32(i32::MIN, 0);
            let v2 = rng.i32();
            let cv = diff_norm("cfg-21", (asc, v1, v2), asc,
                &mut || unsafe { (c.compare_allocations)(v1, v2) },
                &mut || unsafe { (r.compare_allocations)(v1, v2) });
            // ground truth: val1 <= 0 -> +0 arm
            assert_eq!(cv, if asc { 1 } else { 2 }, "asc={asc} v1={v1}");
        }
    }
}

#[test]
fn cfg_22_compare_allocations_long_unnormalized_run() {
    // Without normalization the value depends on process-global tcache
    // history. Replaying the *same* call sequence against each library from a
    // freshly normalized start must yield the identical SEQUENCE.
    let (c, r) = both();
    let n = 512;
    let mut rng = Rng::new(SEED);
    let inputs: Vec<(i32, i32)> = (0..n).map(|_| (rng.spicy_i32(), rng.spicy_i32())).collect();

    diff_norm_seq(
        "cfg-22",
        true,
        &mut || {
            let mut cs: Vec<i32> = Vec::with_capacity(n);
            for &(a, b) in &inputs {
                cs.push(unsafe { (c.compare_allocations)(a, b) });
            }
            cs
        },
        &mut || {
            let mut rs: Vec<i32> = Vec::with_capacity(n);
            for &(a, b) in &inputs {
                rs.push(unsafe { (r.compare_allocations)(a, b) });
            }
            rs
        },
    );
}

// ===========================================================================
// arity4 — rows 23..34   (heap-normalized, both orders)
// ===========================================================================

fn a4(c: &Lib, r: &Lib, row: &str, asc: bool, p: (i32, i32, i32, i32)) {
    diff_norm(row, (asc, p), asc,
        &mut || unsafe { (c.arity4)(p.0, p.1, p.2, p.3) },
        &mut || unsafe { (r.arity4)(p.0, p.1, p.2, p.3) });
}

/// rows 23..28: each residue class of `param1 % 4`, both signs.
fn arity4_residue_row(row: &str, want_rem: i32, negative: bool) {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED ^ want_rem as u64);
        let mut n = 0;
        while n < 800 {
            let p1 = if negative {
                rng.range_i32(i32::MIN + 4, 0)
            } else {
                rng.range_i32(0, i32::MAX - 4)
            };
            if p1 % 4 != want_rem {
                continue;
            }
            n += 1;
            a4(&c, &r, row, asc, (p1, rng.i32(), 0, 0));
        }
    }
}

#[test]
fn cfg_23_arity4_param1_rem0() {
    arity4_residue_row("cfg-23", 0, false);
}
#[test]
fn cfg_24_arity4_param1_rem1() {
    arity4_residue_row("cfg-24", 1, false);
}
#[test]
fn cfg_25_arity4_param1_rem2() {
    arity4_residue_row("cfg-25", 2, false);
}
#[test]
fn cfg_26_arity4_param1_rem3() {
    arity4_residue_row("cfg-26", 3, false);
}

#[test]
fn cfg_27_arity4_param1_negative_rem() {
    // C's `%` is truncating, so param1<0 gives rem in {-1,-2,-3}, which falls
    // through apply_bitmask's `default:` (identity). A Rust translation using
    // rem_euclid would silently pick case 1/2/3 instead.
    for rem in [-1, -2, -3] {
        arity4_residue_row("cfg-27", rem, true);
    }
}

#[test]
fn cfg_28_arity4_param1_negative_rem0() {
    arity4_residue_row("cfg-28", 0, true);
}

#[test]
fn cfg_29_arity4_param3_positive() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for &p3 in &[1i32, 2, 3, 50, 99, 100, 101, 1000, i32::MAX] {
            for _ in 0..200 {
                a4(&c, &r, "cfg-29", asc, (rng.spicy_i32(), rng.spicy_i32(), p3, 0));
            }
        }
        for _ in 0..1000 {
            let p3 = rng.range_i32(1, i32::MAX);
            a4(&c, &r, "cfg-29", asc, (rng.spicy_i32(), rng.spicy_i32(), p3, 0));
        }
    }
}

#[test]
fn cfg_30_arity4_param3_negative() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for &p3 in &[-1i32, -2, -50, -99, -100, -101, -1000, i32::MIN, i32::MIN + 1] {
            for _ in 0..200 {
                a4(&c, &r, "cfg-30", asc, (rng.spicy_i32(), rng.spicy_i32(), p3, 0));
            }
        }
        for _ in 0..1000 {
            let p3 = rng.range_i32(i32::MIN, -1);
            a4(&c, &r, "cfg-30", asc, (rng.spicy_i32(), rng.spicy_i32(), p3, 0));
        }
    }
}

#[test]
fn cfg_31_arity4_param3_zero_param4_nonzero() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..1500 {
            let mut p4 = rng.spicy_i32();
            if p4 == 0 {
                p4 = 1;
            }
            a4(&c, &r, "cfg-31", asc, (rng.spicy_i32(), rng.spicy_i32(), 0, p4));
        }
    }
}

#[test]
fn cfg_32_arity4_param3_and_param4_nonzero() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..2000 {
            let mut p3 = rng.spicy_i32();
            if p3 == 0 {
                p3 = 7;
            }
            let mut p4 = rng.spicy_i32();
            if p4 == 0 {
                p4 = -7;
            }
            a4(&c, &r, "cfg-32", asc, (rng.spicy_i32(), rng.spicy_i32(), p3, p4));
        }
    }
}

#[test]
fn cfg_33_arity4_full_range_random() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..2000 {
            a4(
                &c,
                &r,
                "cfg-33",
                asc,
                (rng.i32(), rng.i32(), rng.i32(), rng.i32()),
            );
        }
    }
}

#[test]
fn cfg_34_arity4_boundary_cross_product() {
    // 9^4 = 6561 combinations, exhaustively, for each heap order.
    let (c, r) = both();
    const B: [i32; 9] = [i32::MIN, -101, -100, -1, 0, 1, 100, 101, i32::MAX];
    for asc in BOTH_ORDERS {
        for &p1 in &B {
            for &p2 in &B {
                for &p3 in &B {
                    for &p4 in &B {
                        a4(&c, &r, "cfg-34", asc, (p1, p2, p3, p4));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// arity2 / arity3 — rows 35..36
// ===========================================================================

#[test]
fn cfg_35_arity2() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..2000 {
            let (p1, p2) = (rng.spicy_i32(), rng.spicy_i32());
            diff_norm("cfg-35", (asc, p1, p2), asc,
                &mut || unsafe { (c.arity2)(p1, p2) },
                &mut || unsafe { (r.arity2)(p1, p2) });
            // ground truth: arity2 != arity4(_,_,0,0) — compared within C itself
            diff_norm("cfg-35: arity2 != arity4(_,_,0,0)", (asc, p1, p2), asc,
                &mut || unsafe { (c.arity2)(p1, p2) },
                &mut || unsafe { (c.arity4)(p1, p2, 0, 0) });
        }
    }
}

#[test]
fn cfg_36_arity3() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..2000 {
            let (p1, p2, p3) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            diff_norm("cfg-36", (asc, p1, p2, p3), asc,
                &mut || unsafe { (c.arity3)(p1, p2, p3) },
                &mut || unsafe { (r.arity3)(p1, p2, p3) });
            // ground truth: arity3 != arity4(_,_,_,0) — compared within C itself
            diff_norm("cfg-36: arity3 != arity4(_,_,_,0)", (asc, p1, p2, p3), asc,
                &mut || unsafe { (c.arity3)(p1, p2, p3) },
                &mut || unsafe { (c.arity4)(p1, p2, p3, 0) });
        }
    }
}

// ===========================================================================
// arity — rows 37..41
// ===========================================================================

fn arity_pair(c: &Lib, r: &Lib, row: &str, asc: bool, len: c_int, params: &[c_int]) {
    diff_norm(row, (asc, len, params), asc,
        &mut || unsafe { (c.arity)(len, params.as_ptr()) },
        &mut || unsafe { (r.arity)(len, params.as_ptr()) });
}

#[test]
fn cfg_37_arity_len2() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..1500 {
            // Provide 4 slots so a wrong dispatch reads defined memory rather
            // than tripping ASAN-style noise; only [0..2] may be used.
            let p: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
            arity_pair(&c, &r, "cfg-37", asc, 2, &p);
        }
    }
}

#[test]
fn cfg_38_arity_len3() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..1500 {
            let p: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
            arity_pair(&c, &r, "cfg-38", asc, 3, &p);
        }
    }
}

#[test]
fn cfg_39_arity_len4() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for _ in 0..1500 {
            let p: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
            arity_pair(&c, &r, "cfg-39", asc, 4, &p);
        }
    }
}

#[test]
fn cfg_40_arity_len_5_to_255() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        for len in 5i32..=255 {
            // 8 slots: proves arity() never reads past params[3] regardless of
            // what `len` claims.
            let p: Vec<c_int> = (0..8).map(|_| rng.spicy_i32()).collect();
            arity_pair(&c, &r, "cfg-40", asc, len, &p);
            // ground truth: identical to arity4 on the first four elements
            diff_norm("cfg-40: len must ignore elements past [3]", (asc, len), asc,
                &mut || unsafe { (c.arity4)(p[0], p[1], p[2], p[3]) },
                &mut || unsafe { (c.arity)(len, p.as_ptr()) });
        }
    }
}

#[test]
fn cfg_41_arity_len_truncation() {
    let (c, r) = both();
    for asc in BOTH_ORDERS {
        let mut rng = Rng::new(SEED);
        let mut lens: Vec<c_int> = (256..=520).collect();
        lens.extend_from_slice(&[
            1000, 1024, 65535, 65536, 65538, 0x7FFF_FF00, i32::MAX, i32::MAX - 1,
            -1, -2, -3, -4, -255, -256, -257, -512, i32::MIN, i32::MIN + 1,
        ]);
        for &len in &lens {
            let p: Vec<c_int> = (0..8).map(|_| rng.spicy_i32()).collect();
            arity_pair(&c, &r, "cfg-41", asc, len, &p);
        }
    }
}

// ===========================================================================
// row 42 — mixed sequence across every entry point
// ===========================================================================

#[test]
fn cfg_42_mixed_entry_point_sequence() {
    // Drives all nine exports in one randomized, allocator-perturbing sequence
    // (NO normalization inside the run), replayed identically against each
    // library from the same normalized starting state. Catches composed
    // pipeline / allocator-interaction divergence that per-call tests miss.
    #[derive(Debug, Clone, Copy)]
    enum Op {
        Arity(i32, [i32; 8]),
        Arity2(i32, i32),
        Arity3(i32, i32, i32),
        Arity4(i32, i32, i32, i32),
        Cmp(i32, i32),
        Bitmask(i32, i32),
        Str(usize),
        Shift(i32, i32, [i32; 8]),
        Matrix,
    }

    let mut rng = Rng::new(SEED);
    let mut ops = Vec::with_capacity(1000);
    for _ in 0..1000 {
        ops.push(match rng.next_u64() % 9 {
            0 => {
                let mut p = [0i32; 8];
                for x in p.iter_mut() {
                    *x = rng.spicy_i32();
                }
                Op::Arity(rng.range_i32(-8, 600), p)
            }
            1 => Op::Arity2(rng.spicy_i32(), rng.spicy_i32()),
            2 => Op::Arity3(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()),
            3 => Op::Arity4(rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()),
            4 => Op::Cmp(rng.spicy_i32(), rng.spicy_i32()),
            5 => Op::Bitmask(rng.spicy_i32(), rng.range_i32(-6, 10)),
            6 => Op::Str(rng.range_usize(1, 40)),
            7 => {
                let mut p = [0i32; 8];
                for x in p.iter_mut() {
                    *x = rng.spicy_i32();
                }
                Op::Shift(rng.range_i32(0, 8), rng.range_i32(-2, 9), p)
            }
            _ => Op::Matrix,
        });
    }

    // Replay the sequence against one library, recording everything observable.
    let run = |l: &Lib| -> Vec<i32> {
        let mut out = Vec::with_capacity(16000);
        for op in &ops {
            match *op {
                Op::Arity(len, p) => out.push(unsafe { (l.arity)(len, p.as_ptr()) }),
                Op::Arity2(a, b) => out.push(unsafe { (l.arity2)(a, b) }),
                Op::Arity3(a, b, c) => out.push(unsafe { (l.arity3)(a, b, c) }),
                Op::Arity4(a, b, c, d) => out.push(unsafe { (l.arity4)(a, b, c, d) }),
                Op::Cmp(a, b) => out.push(unsafe { (l.compare_allocations)(a, b) }),
                Op::Bitmask(v, o) => out.push(unsafe { (l.apply_bitmask)(v, o) }),
                Op::Str(n) => {
                    let buf: Vec<std::ffi::c_char> =
                        cbuf(&vec![b'x'; n]);
                    out.push(unsafe { (l.process_string)(buf.as_ptr()) });
                }
                Op::Shift(size, pos, p) => {
                    let mut b = p;
                    unsafe { (l.shift_array)(b.as_mut_ptr(), size, pos) };
                    out.extend_from_slice(&b);
                }
                Op::Matrix => {
                    let mut m = [0i32; 12];
                    unsafe { (l.init_matrix)(m.as_mut_ptr()) };
                    out.extend_from_slice(&m);
                }
            }
        }
        out
    };

    let (cl, rl) = both();
    diff_norm_seq("cfg-42", true, &mut || run(&cl), &mut || run(&rl));
}

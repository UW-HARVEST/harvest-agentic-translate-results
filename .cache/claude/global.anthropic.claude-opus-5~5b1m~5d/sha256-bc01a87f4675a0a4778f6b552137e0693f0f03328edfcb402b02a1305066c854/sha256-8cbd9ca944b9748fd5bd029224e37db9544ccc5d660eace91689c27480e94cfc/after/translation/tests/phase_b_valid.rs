//! Phase B — valid-path differential tests.
//! One test per row of CONFIGS.md, driven through the `.so` exports of BOTH
//! libraries.  Every row uses many seeded-random inputs, not a single value.

mod common;
use common::*;

use std::ffi::CString;
use std::os::raw::{c_int, c_uint};

const N: usize = 400;

// ===========================================================================
// Row 1 / 2 — multiply_with_static
// ===========================================================================

#[test]
fn cfg_01_multiply_random() {
    let mut rng = Rng::new(0x0101_0101);
    for i in 0..N {
        let (a, b) = (rng.i32_biased(), rng.i32_biased());
        let ctx = format!("row1[{i}] multiply_with_static({a},{b})");
        diff(&ctx, |l| l.multiply_with_static(a, b));
    }
}

#[test]
fn cfg_02_multiply_boundary_grid() {
    let extra: [c_int; 4] = [i32::MAX / 3, i32::MIN / 3, i32::MAX / 3 + 1, i32::MIN / 3 - 1];
    let vals: Vec<c_int> = BOUNDARY.iter().copied().chain(extra).collect();
    for &a in &vals {
        for &b in &vals {
            let ctx = format!("row2 multiply_with_static({a},{b})");
            diff(&ctx, |l| l.multiply_with_static(a, b));
        }
    }
}

// ===========================================================================
// Row 3 / 4 — add_with_static
// ===========================================================================

#[test]
fn cfg_03_add_random() {
    let mut rng = Rng::new(0x0303_0303);
    for i in 0..N {
        let (a, b) = (rng.i32_biased(), rng.i32_biased());
        let ctx = format!("row3[{i}] add_with_static({a},{b})");
        diff(&ctx, |l| l.add_with_static(a, b));
    }
}

#[test]
fn cfg_04_add_boundary_grid() {
    let extra: [c_int; 6] = [
        i32::MAX - 99,
        i32::MAX - 100,
        i32::MAX - 101,
        i32::MIN + 99,
        i32::MIN + 100,
        i32::MIN + 101,
    ];
    let vals: Vec<c_int> = BOUNDARY.iter().copied().chain(extra).collect();
    for &a in &vals {
        for &b in &vals {
            let ctx = format!("row4 add_with_static({a},{b})");
            diff(&ctx, |l| l.add_with_static(a, b));
        }
    }
}

// ===========================================================================
// Row 5 — xor_operation
// ===========================================================================

#[test]
fn cfg_05_xor_random_and_edges() {
    let mut rng = Rng::new(0x0505_0505);
    for i in 0..N {
        let (a, b) = (rng.i32_biased(), rng.i32_biased());
        let ctx = format!("row5[{i}] xor_operation({a},{b})");
        diff(&ctx, |l| l.xor_operation(a, b));
    }
    let vals: Vec<c_int> = BOUNDARY
        .iter()
        .copied()
        .chain([0xABCD, !0xABCD, 0x7FFF_ABCD])
        .collect();
    for &a in &vals {
        for &b in &vals {
            let ctx = format!("row5 xor_operation({a},{b})");
            diff(&ctx, |l| l.xor_operation(a, b));
        }
        // a == b case
        let ctx = format!("row5 xor_operation({a},{a})");
        diff(&ctx, |l| l.xor_operation(a, a));
    }
}

// ===========================================================================
// Row 6 / 7 — shift_with_static:  (a << 2) | (b >> 2)
// ===========================================================================

#[test]
fn cfg_06_shift_random() {
    let mut rng = Rng::new(0x0606_0606);
    for i in 0..N * 3 {
        let (a, b) = (rng.i32_biased(), rng.i32_biased());
        let ctx = format!("row6[{i}] shift_with_static({a},{b})");
        diff(&ctx, |l| l.shift_with_static(a, b));
    }
}

#[test]
fn cfg_07_shift_boundary_grid() {
    let a_vals: Vec<c_int> = BOUNDARY
        .iter()
        .copied()
        .chain([0x6000_0000, 0x1FFF_FFFF, -0x4000_0000])
        .collect();
    let b_vals: Vec<c_int> = BOUNDARY
        .iter()
        .copied()
        .chain([-5, -6, -7, -8, 7, 8, 0x7FFF_FFFF])
        .collect();
    for &a in &a_vals {
        for &b in &b_vals {
            let ctx = format!("row7 shift_with_static({a},{b})");
            diff(&ctx, |l| l.shift_with_static(a, b));
        }
    }
}

// ===========================================================================
// Rows 8..11 — get_operation dispatch: returned pointer must behave like the
// corresponding exported function *of the same library*.
// ===========================================================================

fn dispatch_row(opcode: c_int, direct: fn(&'static LibHandle, c_int, c_int) -> c_int, seed: u64) {
    // 1. non-NULL in both libs
    let (c, r) = both();
    assert!(
        !c.get_operation_raw(opcode).is_null(),
        "C get_operation({opcode}) returned NULL"
    );
    assert!(
        !r.get_operation_raw(opcode).is_null(),
        "Rust get_operation({opcode}) returned NULL"
    );

    // 2. the returned pointer equals the library's own exported symbol address
    let names: [&[u8]; 4] = [
        b"multiply_with_static\0",
        b"add_with_static\0",
        b"xor_operation\0",
        b"shift_with_static\0",
    ];
    for lib in [c, r] {
        assert_eq!(
            lib.get_operation_raw(opcode),
            lib.op_addr(names[opcode as usize]),
            "{} get_operation({opcode}) != &{}",
            lib.which,
            String::from_utf8_lossy(names[opcode as usize])
        );
    }

    // 3. calling through the returned pointer matches across libs, and matches
    //    the direct call within each lib.
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let (a, b) = (rng.i32_biased(), rng.i32_biased());
        let ctx = format!("get_operation({opcode})[{i}]({a},{b})");
        diff(&ctx, |l| {
            let f = l.get_operation(opcode).unwrap();
            unsafe { f(a, b) }
        });
        let ctx2 = format!("direct-vs-dispatch({opcode})[{i}]({a},{b})");
        diff(&ctx2, |l| {
            let f = l.get_operation(opcode).unwrap();
            let via_ptr = unsafe { f(a, b) };
            let direct_v = direct(l, a, b);
            assert_eq!(via_ptr, direct_v, "{ctx2}: {} internal mismatch", l.which);
            via_ptr
        });
    }
}

#[test]
fn cfg_08_get_operation_0_multiply() {
    dispatch_row(0, |l, a, b| l.multiply_with_static(a, b), 0x0808);
}

#[test]
fn cfg_09_get_operation_1_add() {
    dispatch_row(1, |l, a, b| l.add_with_static(a, b), 0x0909);
}

#[test]
fn cfg_10_get_operation_2_xor() {
    dispatch_row(2, |l, a, b| l.xor_operation(a, b), 0x0a0a);
}

#[test]
fn cfg_11_get_operation_3_shift() {
    dispatch_row(3, |l, a, b| l.shift_with_static(a, b), 0x0b0b);
}

// ===========================================================================
// Row 12 — lazy `static ops[]` init: repeated calls are stable
// ===========================================================================

#[test]
fn cfg_12_get_operation_repeated_stable() {
    for lib in [clib(), rlib()] {
        for opcode in 0..4 {
            let first = lib.get_operation_raw(opcode);
            for _ in 0..50 {
                assert_eq!(
                    lib.get_operation_raw(opcode),
                    first,
                    "{} get_operation({opcode}) not stable across calls",
                    lib.which
                );
            }
        }
    }
    // interleaved order (the C table is filled on the first call regardless of
    // which opcode triggered it)
    for lib in [clib(), rlib()] {
        let seq = [3, 0, 2, 1, 3, 1, 0, 2];
        let mut seen: [*mut std::ffi::c_void; 4] = [std::ptr::null_mut(); 4];
        for &op in &seq {
            let p = lib.get_operation_raw(op);
            assert!(!p.is_null());
            if seen[op as usize].is_null() {
                seen[op as usize] = p;
            } else {
                assert_eq!(seen[op as usize], p, "{} unstable", lib.which);
            }
        }
    }
}

// ===========================================================================
// Row 13 / 14 — execute_operation (prints 3 lines)
// ===========================================================================

#[test]
fn cfg_13_execute_operation_all_opcodes() {
    let name = CString::new("XOR").unwrap();
    let mut rng = Rng::new(0x0d0d_0d0d);
    for opcode in 0..4 {
        for i in 0..N / 2 {
            let (a, b) = (rng.i32_biased(), rng.i32_biased());
            let ctx = format!("row13 execute_operation(op{opcode})[{i}]({a},{b})");
            diff(&ctx, |l| {
                let f = l.get_operation(opcode);
                l.execute_operation(f, a, b, name.as_ptr())
            });
        }
    }
}

#[test]
fn cfg_14_execute_operation_name_shapes() {
    let names = [
        CString::new("").unwrap(),
        CString::new("%d%s").unwrap(),
        CString::new("%%%%").unwrap(),
        CString::new("SHIFT").unwrap(),
        CString::new("a\tb").unwrap(),
        CString::new("x".repeat(200)).unwrap(),
    ];
    let mut rng = Rng::new(0x0e0e_0e0e);
    for (ni, name) in names.iter().enumerate() {
        for opcode in 0..4 {
            let (a, b) = (rng.i32_biased(), rng.i32_biased());
            let ctx = format!("row14 execute_operation(name{ni},op{opcode},{a},{b})");
            diff(&ctx, |l| {
                let f = l.get_operation(opcode);
                l.execute_operation(f, a, b, name.as_ptr())
            });
        }
    }
}

// ===========================================================================
// Row 15 — cross-library callback: a func pointer from lib X handed to lib Y
// ===========================================================================

#[test]
fn cfg_15_cross_library_callback() {
    let name = CString::new("CROSS").unwrap();
    let (c, r) = both();
    let mut rng = Rng::new(0x0f0f_0f0f);

    for opcode in 0..4 {
        for i in 0..64 {
            let (a, b) = (rng.i32_biased(), rng.i32_biased());

            // Rust callback executed by the C execute_operation, and C callback
            // executed by the Rust execute_operation.  Both transcripts must be
            // byte-identical.
            let _g = capture_lock();
            let (v1, out1) = capture_locked(|| {
                c.execute_operation(r.get_operation(opcode), a, b, name.as_ptr())
            });
            let (v2, out2) = capture_locked(|| {
                r.execute_operation(c.get_operation(opcode), a, b, name.as_ptr())
            });
            drop(_g);
            assert_eq!(
                v1, v2,
                "row15[{i}] op{opcode}({a},{b}): C-calls-Rust={v1} Rust-calls-C={v2}"
            );
            assert_eq!(
                String::from_utf8_lossy(&out1),
                String::from_utf8_lossy(&out2),
                "row15[{i}] op{opcode}({a},{b}) stdout mismatch"
            );

            // and both must agree with the homogeneous calls
            let ctx = format!("row15-homog[{i}] op{opcode}({a},{b})");
            let v = diff(&ctx, |l| {
                l.execute_operation(l.get_operation(opcode), a, b, name.as_ptr())
            });
            assert_eq!(v, v1, "{ctx}: cross-call value differs from homogeneous");
        }
    }
}

// ===========================================================================
// Rows 16..21 — compute_checksum
// ===========================================================================

fn checksum_row(count: c_int, arr_len: usize, seed: u64, iters: usize, tag: &str) {
    let mut rng = Rng::new(seed);
    for i in 0..iters {
        let mut vals: Vec<c_int> = (0..arr_len).map(|_| rng.i32_biased()).collect();
        let ctx = format!("{tag}[{i}] count={count} vals={vals:?}");
        let vals_snapshot = vals.clone();
        diff(&ctx, |l| {
            vals.copy_from_slice(&vals_snapshot);
            l.compute_checksum(vals.as_mut_ptr(), count)
        });
        // the C code never writes through `values`
        assert_eq!(vals, vals_snapshot, "{ctx}: input array mutated");
    }
}

#[test]
fn cfg_16_checksum_count_1() {
    checksum_row(1, 4, 0x1010, 300, "row16");
}

#[test]
fn cfg_17_checksum_count_2() {
    checksum_row(2, 4, 0x1111, 300, "row17");
}

#[test]
fn cfg_18_checksum_count_3() {
    checksum_row(3, 4, 0x1212, 300, "row18");
}

#[test]
fn cfg_19_checksum_count_4() {
    checksum_row(4, 4, 0x1313, 400, "row19");
}

#[test]
fn cfg_20_checksum_count_clamped_above_4() {
    let mut rng = Rng::new(0x1414);
    for i in 0..200 {
        let mut vals: Vec<c_int> = (0..4).map(|_| rng.i32_biased()).collect();
        let snap = vals.clone();
        let base = diff(&format!("row20[{i}] base count=4"), |l| {
            vals.copy_from_slice(&snap);
            l.compute_checksum(vals.as_mut_ptr(), 4)
        });
        for &count in &[5, 6, 100, 1024, i32::MAX] {
            // NOTE: only 4 elements are readable; the C code clamps copy_count
            // to 4 so this is exactly the input shape the clamp exists for.
            let got = diff(&format!("row20[{i}] count={count}"), |l| {
                vals.copy_from_slice(&snap);
                l.compute_checksum(vals.as_mut_ptr(), count)
            });
            assert_eq!(got, base, "row20[{i}]: count={count} not clamped to 4");
        }
    }
}

#[test]
fn cfg_21_checksum_byte_patterns() {
    let patterns: Vec<Vec<c_int>> = vec![
        vec![0, 0, 0, 0],
        vec![-1, -1, -1, -1],
        vec![i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        vec![i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        vec![0x0100_0000, 0x0001_0000, 0x0000_0100, 0x0000_0001],
        vec![0x00FF_0000, 0x0000_FF00, 0x00_00_00_FF, 0xFF00_0000u32 as i32],
        vec![1, 0, 0, 0],
        vec![0, 0, 0, 1],
        vec![0x8000_0000u32 as i32, 0x7FFF_FFFF, -2, 2],
        vec![0xDEAD_BEEFu32 as i32, 0xBEEF_DEADu32 as i32, 0, -1],
        vec![0x0000_FFFF, 0xFFFF_0000u32 as i32, 0x0000_FFFF, 0xFFFF_0000u32 as i32],
        // engineered so the pre-mask 32-bit checksums differ in high bits only
        vec![0x1000_0000, 0, 0, 0],
        vec![0x2000_0000, 0, 0, 0],
    ];
    for (pi, pat) in patterns.iter().enumerate() {
        for count in 1..=4 {
            let mut vals = pat.clone();
            let snap = vals.clone();
            let got = diff(&format!("row21 pat{pi} count={count} {pat:?}"), |l| {
                vals.copy_from_slice(&snap);
                l.compute_checksum(vals.as_mut_ptr(), count)
            });
            assert_eq!(got & !0xFFFFu32, 0, "row21: result not masked to 16 bits");
        }
    }
    // larger arrays where only the first 4 elements may be read
    let mut rng = Rng::new(0x1515);
    for i in 0..100 {
        let mut vals: Vec<c_int> = (0..16).map(|_| rng.i32_biased()).collect();
        let snap = vals.clone();
        for count in 1..=4 {
            diff(&format!("row21-big[{i}] count={count}"), |l| {
                vals.copy_from_slice(&snap);
                l.compute_checksum(vals.as_mut_ptr(), count)
            });
        }
    }
}

// ===========================================================================
// Rows 22 / 23 — init_state
// ===========================================================================

#[test]
fn cfg_22_init_state_values() {
    let mut rng = Rng::new(0x1616);
    let mut vals: Vec<c_int> = BOUNDARY.to_vec();
    for _ in 0..200 {
        vals.push(rng.i32_biased());
    }
    for (i, &v) in vals.iter().enumerate() {
        // Pre-poison the struct so that "fields not written" would be visible.
        let poison = ComputeState {
            accumulator: 0x5A5A_5A5A,
            operation_count: 0x3C3C_3C3C,
            checksum: 0xA5A5_A5A5,
        };
        let mut st = poison;
        let ctx = format!("row22[{i}] init_state(_, {v})");
        let got = diff(&ctx, |l| {
            st = poison;
            l.init_state(&mut st as *mut ComputeState, v);
            st
        });
        assert_eq!(
            got,
            ComputeState {
                accumulator: v,
                operation_count: 0,
                checksum: 0
            },
            "{ctx}: struct contents"
        );
    }
}

#[test]
fn cfg_23_init_state_reinit() {
    let mut rng = Rng::new(0x1717);
    for i in 0..100 {
        let (v1, v2) = (rng.i32_biased(), rng.i32_biased());
        let mut st = ComputeState::default();
        let ctx = format!("row23[{i}] reinit({v1} then {v2})");
        let got = diff(&ctx, |l| {
            let p = &mut st as *mut ComputeState;
            l.init_state(p, v1);
            // dirty the state as a real consumer would
            st.operation_count = 7;
            st.checksum = 0xFFFF;
            l.init_state(p, v2);
            st
        });
        assert_eq!(
            got,
            ComputeState {
                accumulator: v2,
                operation_count: 0,
                checksum: 0
            }
        );
    }
}

// ===========================================================================
// Rows 24..26 — apply_operation
// ===========================================================================

#[test]
fn cfg_24_apply_operation_single() {
    let mut rng = Rng::new(0x1818);
    for opcode in 0..4 {
        for i in 0..N / 2 {
            let (acc, value) = (rng.i32_biased(), rng.i32_biased());
            let mut st = ComputeState::default();
            let ctx = format!("row24 apply(op{opcode})[{i}] acc={acc} val={value}");
            diff(&ctx, |l| {
                st = ComputeState {
                    accumulator: acc,
                    operation_count: 0,
                    checksum: 0xDEAD,
                };
                l.apply_operation(&mut st as *mut ComputeState, value, l.get_operation(opcode));
                st
            });
            assert_eq!(st.operation_count, 1, "{ctx}: operation_count");
            assert_eq!(st.checksum, 0xDEAD, "{ctx}: checksum must be untouched");
        }
    }
}

#[test]
fn cfg_25_apply_operation_chain() {
    let mut rng = Rng::new(0x1919);
    for i in 0..120 {
        let initial = rng.i32_biased();
        let steps: Vec<(c_int, c_int)> = (0..16)
            .map(|_| (rng.below(4) as c_int, rng.i32_biased()))
            .collect();
        let mut st = ComputeState::default();
        let ctx = format!("row25[{i}] chain initial={initial} steps={steps:?}");
        let got = diff(&ctx, |l| {
            let p = &mut st as *mut ComputeState;
            l.init_state(p, initial);
            for &(opcode, value) in &steps {
                l.apply_operation(p, value, l.get_operation(opcode));
            }
            st
        });
        assert_eq!(got.operation_count, 16, "{ctx}: operation_count");
    }
}

#[test]
fn cfg_26_apply_operation_count_base() {
    let bases: [c_int; 6] = [0, 1, 41, i32::MAX - 1, i32::MAX, -1];
    let mut rng = Rng::new(0x1a1a);
    for &base in &bases {
        for opcode in 0..4 {
            let (acc, value) = (rng.i32_biased(), rng.i32_biased());
            let mut st = ComputeState::default();
            let ctx = format!("row26 base={base} op{opcode} acc={acc} val={value}");
            diff(&ctx, |l| {
                st = ComputeState {
                    accumulator: acc,
                    operation_count: base,
                    checksum: 0,
                };
                l.apply_operation(&mut st as *mut ComputeState, value, l.get_operation(opcode));
                st
            });
        }
    }
}

// ===========================================================================
// Row 27 — caller-composed pipeline (the same sequence checkshift performs)
// ===========================================================================

#[test]
fn cfg_27_manual_pipeline() {
    let xor_name = CString::new("XOR").unwrap();
    let shift_name = CString::new("SHIFT").unwrap();
    let mut rng = Rng::new(0x1b1b);

    for i in 0..200 {
        let (p1, p2, p3, p4) = (
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
        );
        let ctx = format!("row27[{i}] pipeline({p1},{p2},{p3},{p4})");
        diff(&ctx, |l| {
            let mut st = ComputeState::default();
            let p = &mut st as *mut ComputeState;
            l.init_state(p, p1);
            l.apply_operation(p, p2, l.get_operation(0));
            l.apply_operation(p, p3, l.get_operation(1));
            let xor_result =
                l.execute_operation(l.get_operation(2), st.accumulator, p4, xor_name.as_ptr());
            let shift_result =
                l.execute_operation(l.get_operation(3), xor_result, p2, shift_name.as_ptr());
            let mut params: [c_int; 4] = [p1, p2, p3, p4];
            st.checksum = l.compute_checksum(params.as_mut_ptr(), 4);
            let final_result =
                ((st.accumulator.wrapping_add(shift_result) as c_uint) ^ st.checksum) as c_int;
            (st, xor_result, shift_result, final_result)
        });
    }
}

// ===========================================================================
// Rows 28..32 — checkshift end to end (return value + full transcript)
// ===========================================================================

#[test]
fn cfg_28_checkshift_random() {
    let mut rng = Rng::new(0x1c1c_1c1c);
    for i in 0..600 {
        let (p1, p2, p3, p4) = (
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
        );
        let ctx = format!("row28[{i}] checkshift({p1},{p2},{p3},{p4})");
        diff(&ctx, |l| l.checkshift(p1, p2, p3, p4));
    }
}

#[test]
fn cfg_29_checkshift_all_zero() {
    diff("row29 checkshift(0,0,0,0)", |l| l.checkshift(0, 0, 0, 0));
}

#[test]
fn cfg_30_checkshift_boundary_positions() {
    let edges: [c_int; 8] = [
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        0x4000_0000u32 as i32,
        -0x4000_0000,
        0x0000_ABCD,
    ];
    // one edge value in each of the four positions, others fixed
    for pos in 0..4 {
        for &e in &edges {
            for &filler in &[0, 1, -1, i32::MAX, i32::MIN] {
                let mut p = [filler; 4];
                p[pos] = e;
                let ctx = format!("row30 checkshift({},{},{},{})", p[0], p[1], p[2], p[3]);
                diff(&ctx, |l| l.checkshift(p[0], p[1], p[2], p[3]));
            }
        }
    }
    // and the full edge^4 corner set (a subset, to stay fast)
    for &a in &edges {
        for &b in &edges {
            let ctx = format!("row30-corner checkshift({a},{b},{a},{b})");
            diff(&ctx, |l| l.checkshift(a, b, a, b));
        }
    }
}

#[test]
fn cfg_31_checkshift_doc_style() {
    for p in [
        [1, 2, 3, 4],
        [10, 20, 30, 40],
        [5, 5, 5, 5],
        [100, -100, 200, -200],
        [7, 11, 13, 17],
    ] {
        let ctx = format!("row31 checkshift({},{},{},{})", p[0], p[1], p[2], p[3]);
        diff(&ctx, |l| l.checkshift(p[0], p[1], p[2], p[3]));
    }
}

#[test]
fn cfg_32_checkshift_repeated_invocation() {
    let mut rng = Rng::new(0x1f1f);
    for i in 0..60 {
        let (p1, p2, p3, p4) = (
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
            rng.i32_biased(),
        );
        // Call each library three times in a row: no cross-call state may leak.
        let _g = capture_lock();
        let mut c_runs = Vec::new();
        let mut r_runs = Vec::new();
        for _ in 0..3 {
            c_runs.push(capture_locked(|| clib().checkshift(p1, p2, p3, p4)));
            r_runs.push(capture_locked(|| rlib().checkshift(p1, p2, p3, p4)));
        }
        drop(_g);
        for k in 0..3 {
            assert_eq!(
                c_runs[k].0, r_runs[k].0,
                "row32[{i}] run{k}: value mismatch for ({p1},{p2},{p3},{p4})"
            );
            assert_eq!(
                String::from_utf8_lossy(&c_runs[k].1),
                String::from_utf8_lossy(&r_runs[k].1),
                "row32[{i}] run{k}: stdout mismatch for ({p1},{p2},{p3},{p4})"
            );
            assert_eq!(c_runs[k].1, c_runs[0].1, "row32[{i}]: C not idempotent");
            assert_eq!(r_runs[k].1, r_runs[0].1, "row32[{i}]: Rust not idempotent");
        }
    }
}

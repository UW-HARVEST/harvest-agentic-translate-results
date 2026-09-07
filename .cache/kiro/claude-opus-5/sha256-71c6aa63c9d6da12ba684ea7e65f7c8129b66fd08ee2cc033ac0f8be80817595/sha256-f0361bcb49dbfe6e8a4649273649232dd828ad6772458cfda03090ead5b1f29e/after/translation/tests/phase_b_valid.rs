//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Every call goes through a `dlsym`'d export of the C `.so` and of the Rust
//! `.so`; return values AND captured stdout bytes are compared.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};

// ===========================================================================
// helpers
// ===========================================================================

/// Run a pure (non-printing) binary op over `inputs` on both libs and compare.
fn diff_leaf(row: &str, op_index: usize, inputs: &[(c_int, c_int)]) {
    let p = pair();
    let cf = p.c.op_by_index(op_index);
    let rf = p.rs.op_by_index(op_index);
    for &(a, b) in inputs {
        let cv = unsafe { cf(a, b) };
        let rv = unsafe { rf(a, b) };
        assert_eq!(
            cv,
            rv,
            "[{row}] {}({a}, {b}): C={cv} Rust={rv}",
            Impl::op_name(op_index)
        );
    }
    // Leaf ops must print nothing at all.
    let (_, c_out) = capture_stdout(|| {
        for &(a, b) in inputs.iter().take(64) {
            unsafe { cf(a, b) };
        }
    });
    let (_, r_out) = capture_stdout(|| {
        for &(a, b) in inputs.iter().take(64) {
            unsafe { rf(a, b) };
        }
    });
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(c_out.is_empty(), "[{row}] leaf op unexpectedly printed");
}

fn rand_pairs(seed_bump: u64, n: usize) -> Vec<(c_int, c_int)> {
    let mut rng = Rng::new(SEED ^ seed_bump);
    (0..n).map(|_| (rng.spicy_i32(), rng.spicy_i32())).collect()
}

fn edge_grid() -> Vec<(c_int, c_int)> {
    let mut v = Vec::new();
    for &a in EDGES.iter() {
        for &b in EDGES.iter() {
            v.push((a, b));
        }
    }
    v
}

// ===========================================================================
// Rows 1-3: multiply_with_static
// ===========================================================================

#[test]
fn row01_multiply_random_full_range() {
    diff_leaf("row01 multiply random", 0, &rand_pairs(1, 20_000));
}

#[test]
fn row02_multiply_edge_grid() {
    let mut v = edge_grid();
    // the *3 overflow boundary specifically
    for &x in &[
        i32::MAX / 3,
        i32::MAX / 3 + 1,
        i32::MIN / 3,
        i32::MIN / 3 - 1,
        715_827_883,
    ] {
        v.push((x, 1));
        v.push((1, x));
        v.push((x, -1));
    }
    diff_leaf("row02 multiply edges", 0, &v);
}

#[test]
fn row03_multiply_small_magnitude() {
    let mut rng = Rng::new(SEED ^ 3);
    let v: Vec<_> = (0..20_000)
        .map(|_| (rng.range_i32(-1000, 1000), rng.range_i32(-1000, 1000)))
        .collect();
    diff_leaf("row03 multiply small", 0, &v);
}

// ===========================================================================
// Rows 4-5: add_with_static
// ===========================================================================

#[test]
fn row04_add_random_full_range() {
    diff_leaf("row04 add random", 1, &rand_pairs(4, 20_000));
}

#[test]
fn row05_add_edge_grid() {
    let mut v = edge_grid();
    for &x in &[
        i32::MAX,
        i32::MAX - 99,
        i32::MAX - 100,
        i32::MAX - 101,
        i32::MIN,
        i32::MIN + 99,
        i32::MIN + 100,
        -100,
        -101,
        -99,
    ] {
        v.push((x, 0));
        v.push((0, x));
        v.push((x, x));
        v.push((x, -100));
    }
    diff_leaf("row05 add edges", 1, &v);
}

// ===========================================================================
// Rows 6-7: xor_operation
// ===========================================================================

#[test]
fn row06_xor_random_full_range() {
    diff_leaf("row06 xor random", 2, &rand_pairs(6, 20_000));
}

#[test]
fn row07_xor_edge_grid() {
    let mut v = edge_grid();
    for &x in &[0, -1, 0xABCD, !0xABCD, 0xABCDi32 << 16, i32::MIN, i32::MAX] {
        v.push((x, 0));
        v.push((0, x));
        v.push((x, 0xABCD));
        v.push((0xABCD, x));
    }
    diff_leaf("row07 xor edges", 2, &v);
}

// ===========================================================================
// Rows 8-9: shift_with_static
// ===========================================================================

#[test]
fn row08_shift_random_full_range() {
    diff_leaf("row08 shift random", 3, &rand_pairs(8, 20_000));
}

#[test]
fn row09_shift_edge_grid() {
    let mut v = edge_grid();
    let a_edges = [
        i32::MIN,
        i32::MAX,
        0x4000_0000,
        0x6000_0000u32 as i32,
        0x2000_0000,
        -1,
        -2,
        1,
        0,
        0x8000_0001u32 as i32,
        0xC000_0000u32 as i32,
    ];
    let b_edges = [-4, -3, -2, -1, 0, 1, 2, 3, 4, i32::MIN, i32::MAX, -5, 5];
    for &a in a_edges.iter() {
        for &b in b_edges.iter() {
            v.push((a, b));
        }
    }
    // every single-bit `a` (checks bits shifted out of the top of a<<2)
    for bit in 0..32u32 {
        let a = (1i32).wrapping_shl(bit);
        v.push((a, -1));
        v.push((a, a));
        v.push((-1, a));
    }
    diff_leaf("row09 shift edges", 3, &v);
}

// ===========================================================================
// Rows 10-11: get_operation
// ===========================================================================

fn as_addr(f: OpFnRaw) -> Option<usize> {
    f.map(|g| g as usize)
}

#[test]
fn row10_get_operation_valid_opcodes_identity_and_behaviour() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..4usize {
        let cg = unsafe { (p.c.get_operation)(i as c_int) };
        let rg = unsafe { (p.rs.get_operation)(i as c_int) };
        assert!(cg.is_some(), "[row10] C get_operation({i}) returned NULL");
        assert!(rg.is_some(), "[row10] Rust get_operation({i}) returned NULL");

        // Pointer identity: must be the same library's exported op.
        assert_eq!(
            as_addr(cg),
            Some(p.c.op_by_index(i) as usize),
            "[row10] C get_operation({i}) != &{}",
            Impl::op_name(i)
        );
        assert_eq!(
            as_addr(rg),
            Some(p.rs.op_by_index(i) as usize),
            "[row10] Rust get_operation({i}) != &{}",
            Impl::op_name(i)
        );

        // And the returned pointers must behave identically when called.
        for _ in 0..5_000 {
            let (a, b) = (rng.spicy_i32(), rng.spicy_i32());
            let cv = unsafe { (cg.unwrap())(a, b) };
            let rv = unsafe { (rg.unwrap())(a, b) };
            assert_eq!(cv, rv, "[row10] op {i} ({a},{b}): C={cv} Rust={rv}");
        }
    }
    // get_operation must print nothing.
    let (_, c_out) = capture_stdout(|| {
        for i in -2..6 {
            unsafe { (p.c.get_operation)(i) };
        }
    });
    let (_, r_out) = capture_stdout(|| {
        for i in -2..6 {
            unsafe { (p.rs.get_operation)(i) };
        }
    });
    assert_stdout_eq("row10 get_operation stdout", &c_out, &r_out);
    assert!(c_out.is_empty());
}

#[test]
fn row11_get_operation_lazy_init_branch() {
    // The C version fills a function-static table on first call
    // (`if (ops[0] == NULL)`); exercise first-call, all-opcode, and
    // reverse-order re-call sequences.
    let p = pair();
    let seq: Vec<c_int> = vec![0, 0, 1, 2, 3, 3, 2, 1, 0, -1, 4, 0, 1, 2, 3];
    let c_res: Vec<Option<usize>> =
        seq.iter().map(|&i| as_addr(unsafe { (p.c.get_operation)(i) })).collect();
    let r_res: Vec<Option<usize>> =
        seq.iter().map(|&i| as_addr(unsafe { (p.rs.get_operation)(i) })).collect();

    // Compare the *shape* (which entries are NULL) and self-consistency
    // (repeated calls return the same pointer within a library).
    let c_null: Vec<bool> = c_res.iter().map(|x| x.is_none()).collect();
    let r_null: Vec<bool> = r_res.iter().map(|x| x.is_none()).collect();
    assert_eq!(c_null, r_null, "[row11] NULL pattern differs");

    for (k, &i) in seq.iter().enumerate() {
        if (0..4).contains(&i) {
            assert_eq!(
                c_res[k],
                Some(p.c.op_by_index(i as usize) as usize),
                "[row11] C call #{k} opcode {i} unstable"
            );
            assert_eq!(
                r_res[k],
                Some(p.rs.op_by_index(i as usize) as usize),
                "[row11] Rust call #{k} opcode {i} unstable"
            );
        }
    }
}

// ===========================================================================
// Rows 12-17: execute_operation
// ===========================================================================

fn diff_execute(row: &str, opcode: c_int, name: &str, inputs: &[(c_int, c_int)]) {
    let p = pair();
    let cname = CString::new(name).unwrap();
    let (c_res, c_out) = capture_stdout(|| {
        let f = unsafe { (p.c.get_operation)(opcode) };
        inputs
            .iter()
            .map(|&(a, b)| unsafe { (p.c.execute_operation)(f, a, b, cname.as_ptr()) })
            .collect::<Vec<c_int>>()
    });
    let (r_res, r_out) = capture_stdout(|| {
        let f = unsafe { (p.rs.get_operation)(opcode) };
        inputs
            .iter()
            .map(|&(a, b)| unsafe { (p.rs.execute_operation)(f, a, b, cname.as_ptr()) })
            .collect::<Vec<c_int>>()
    });
    assert_eq!(c_res, r_res, "[{row}] execute_operation return values differ");
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(!c_out.is_empty(), "[{row}] expected output, got none");
}

#[test]
fn row12_execute_multiply() {
    diff_execute("row12 execute MULT", 0, "MULT", &rand_pairs(12, 4_000));
}

#[test]
fn row13_execute_add() {
    diff_execute("row13 execute ADD", 1, "ADD", &rand_pairs(13, 4_000));
}

#[test]
fn row14_execute_xor() {
    diff_execute("row14 execute XOR", 2, "XOR", &rand_pairs(14, 4_000));
}

#[test]
fn row15_execute_shift() {
    diff_execute("row15 execute SHIFT", 3, "SHIFT", &rand_pairs(15, 4_000));
}

#[test]
fn row16_execute_op_name_shapes() {
    let long = "N".repeat(200);
    let names = [
        "",
        "X",
        "with space",
        "%d %s %n percent",
        "%%",
        long.as_str(),
        "tab\tand\nnewline",
        "ünïcödé",
    ];
    for (k, n) in names.iter().enumerate() {
        for opcode in 0..4 {
            diff_execute(
                &format!("row16 name[{k}] opcode {opcode}"),
                opcode,
                n,
                &rand_pairs(160 + k as u64, 40),
            );
        }
    }
}

#[test]
fn row17_execute_cross_library_function_pointer() {
    // A C op pointer handed to Rust's execute_operation and vice versa.
    let p = pair();
    let cname = CString::new("CROSS").unwrap();
    let inputs = rand_pairs(17, 2_000);

    for i in 0..4usize {
        let c_op: OpFnRaw = Some(p.c.op_by_index(i));
        let r_op: OpFnRaw = Some(p.rs.op_by_index(i));

        // C's op through C's entry point vs C's op through Rust's entry point.
        let (a_res, a_out) = capture_stdout(|| {
            inputs
                .iter()
                .map(|&(a, b)| unsafe { (p.c.execute_operation)(c_op, a, b, cname.as_ptr()) })
                .collect::<Vec<c_int>>()
        });
        let (b_res, b_out) = capture_stdout(|| {
            inputs
                .iter()
                .map(|&(a, b)| unsafe { (p.rs.execute_operation)(c_op, a, b, cname.as_ptr()) })
                .collect::<Vec<c_int>>()
        });
        assert_eq!(a_res, b_res, "[row17] C-op: return values differ (op {i})");
        assert_stdout_eq(&format!("row17 C-op into both, op {i}"), &a_out, &b_out);

        // Rust's op through both entry points.
        let (c_res, c_out) = capture_stdout(|| {
            inputs
                .iter()
                .map(|&(a, b)| unsafe { (p.c.execute_operation)(r_op, a, b, cname.as_ptr()) })
                .collect::<Vec<c_int>>()
        });
        let (d_res, d_out) = capture_stdout(|| {
            inputs
                .iter()
                .map(|&(a, b)| unsafe { (p.rs.execute_operation)(r_op, a, b, cname.as_ptr()) })
                .collect::<Vec<c_int>>()
        });
        assert_eq!(c_res, d_res, "[row17] Rust-op: return values differ (op {i})");
        assert_stdout_eq(&format!("row17 Rust-op into both, op {i}"), &c_out, &d_out);

        // And the C op must equal the Rust op numerically.
        assert_eq!(a_res, c_res, "[row17] C op != Rust op (op {i})");
    }
}

// ===========================================================================
// Rows 18-24: compute_checksum
// ===========================================================================

fn diff_checksum(row: &str, arrays: &[(Vec<c_int>, c_int)]) {
    let p = pair();
    let mut c_res = Vec::new();
    let mut r_res = Vec::new();
    for (vals, count) in arrays {
        let mut a = vals.clone();
        let mut b = vals.clone();
        c_res.push(unsafe { (p.c.compute_checksum)(a.as_mut_ptr(), *count) });
        r_res.push(unsafe { (p.rs.compute_checksum)(b.as_mut_ptr(), *count) });
        assert_eq!(a, *vals, "[{row}] C mutated the input array");
        assert_eq!(b, *vals, "[{row}] Rust mutated the input array");
    }
    for (k, (cv, rv)) in c_res.iter().zip(r_res.iter()).enumerate() {
        assert_eq!(
            cv, rv,
            "[{row}] case {k} values={:?} count={}: C=0x{cv:08X} Rust=0x{rv:08X}",
            arrays[k].0, arrays[k].1
        );
    }
    // compute_checksum must print nothing.
    let (_, c_out) = capture_stdout(|| {
        for (vals, count) in arrays.iter().take(32) {
            let mut a = vals.clone();
            unsafe { (p.c.compute_checksum)(a.as_mut_ptr(), *count) };
        }
    });
    let (_, r_out) = capture_stdout(|| {
        for (vals, count) in arrays.iter().take(32) {
            let mut a = vals.clone();
            unsafe { (p.rs.compute_checksum)(a.as_mut_ptr(), *count) };
        }
    });
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(c_out.is_empty());
}

fn random_checksum_cases(seed_bump: u64, count: c_int, n: usize) -> Vec<(Vec<c_int>, c_int)> {
    let mut rng = Rng::new(SEED ^ seed_bump);
    (0..n)
        .map(|_| {
            // Always allocate 4 elements so the pointer is valid regardless of
            // count; only `count` of them are read by the C code.
            let v: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
            (v, count)
        })
        .collect()
}

#[test]
fn row18_checksum_count1() {
    diff_checksum("row18 count=1", &random_checksum_cases(18, 1, 5_000));
}

#[test]
fn row19_checksum_count2() {
    diff_checksum("row19 count=2", &random_checksum_cases(19, 2, 5_000));
}

#[test]
fn row20_checksum_count3() {
    diff_checksum("row20 count=3", &random_checksum_cases(20, 3, 5_000));
}

#[test]
fn row21_checksum_count4() {
    diff_checksum("row21 count=4", &random_checksum_cases(21, 4, 5_000));
}

#[test]
fn row22_checksum_count5_clamp() {
    let mut rng = Rng::new(SEED ^ 22);
    let cases: Vec<(Vec<c_int>, c_int)> = (0..5_000)
        .map(|_| ((0..8).map(|_| rng.spicy_i32()).collect::<Vec<c_int>>(), 5))
        .collect();
    diff_checksum("row22 count=5 clamp", &cases);

    // The clamp must make count=5..=8 identical to count=4 on the same data.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x22);
    for _ in 0..500 {
        let v: Vec<c_int> = (0..8).map(|_| rng.spicy_i32()).collect();
        let mut a = v.clone();
        let base_c = unsafe { (p.c.compute_checksum)(a.as_mut_ptr(), 4) };
        let mut b = v.clone();
        let base_r = unsafe { (p.rs.compute_checksum)(b.as_mut_ptr(), 4) };
        assert_eq!(base_c, base_r);
        for n in 5..=8 {
            let mut a = v.clone();
            let mut b = v.clone();
            let cv = unsafe { (p.c.compute_checksum)(a.as_mut_ptr(), n) };
            let rv = unsafe { (p.rs.compute_checksum)(b.as_mut_ptr(), n) };
            assert_eq!(cv, rv, "[row22] count={n}");
            assert_eq!(cv, base_c, "[row22] count={n} should clamp to 4");
        }
    }
}

#[test]
fn row23_checksum_oversized_count() {
    // count = 1000 / INT_MAX with only a 4-element array: the clamp must
    // prevent any over-read, and both must agree.
    let mut rng = Rng::new(SEED ^ 23);
    let mut cases = Vec::new();
    for _ in 0..2_000 {
        let v: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
        cases.push((v.clone(), 1000));
        cases.push((v.clone(), i32::MAX));
        cases.push((v.clone(), 5));
        cases.push((v, 0x4000_0000));
    }
    diff_checksum("row23 oversized count", &cases);
}

#[test]
fn row24_checksum_byte_patterns() {
    let patterns: Vec<Vec<c_int>> = vec![
        vec![0, 0, 0, 0],
        vec![-1, -1, -1, -1],
        vec![i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        vec![i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        vec![0x8080_8080u32 as i32; 4],
        vec![0x0000_0080, 0x0000_8000, 0x0080_0000, 0x8000_0000u32 as i32],
        vec![0x7F7F_7F7F, 0x8181_8181u32 as i32, 0x00FF_00FF, 0xFF00_FF00u32 as i32],
        vec![1, 2, 3, 4],
        vec![0x0102_0304, 0x0506_0708, 0x090A_0B0C, 0x0D0E_0F10],
        vec![-1, 0, -1, 0],
        vec![0x5555_5555, 0xAAAA_AAAAu32 as i32, 0x3333_3333, 0xCCCC_CCCCu32 as i32],
    ];
    let mut cases = Vec::new();
    for pat in &patterns {
        for count in 1..=4 {
            cases.push((pat.clone(), count));
        }
        for count in [5, 6, 100] {
            cases.push((pat.clone(), count));
        }
    }
    diff_checksum("row24 byte patterns", &cases);
}

// ===========================================================================
// Rows 25-26: init_state
// ===========================================================================

#[test]
fn row25_init_state_values() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 25);
    let mut vals: Vec<c_int> = EDGES.to_vec();
    vals.extend((0..2_000).map(|_| rng.spicy_i32()));

    let (c_states, c_out) = capture_stdout(|| {
        vals.iter()
            .map(|&v| {
                let mut s = ComputeState::default();
                unsafe { (p.c.init_state)(&mut s, v) };
                s.bytes()
            })
            .collect::<Vec<_>>()
    });
    let (r_states, r_out) = capture_stdout(|| {
        vals.iter()
            .map(|&v| {
                let mut s = ComputeState::default();
                unsafe { (p.rs.init_state)(&mut s, v) };
                s.bytes()
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(c_states, r_states, "[row25] ComputeState bytes differ");
    assert_stdout_eq("row25 init_state", &c_out, &r_out);
}

#[test]
fn row26_init_state_overwrites_dirty_buffer() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    let dirty: Vec<ComputeState> = (0..1_000)
        .map(|_| ComputeState {
            accumulator: rng.spicy_i32(),
            operation_count: rng.spicy_i32(),
            checksum: rng.next_i32() as c_uint,
        })
        .collect();
    let vals: Vec<c_int> = (0..1_000).map(|_| rng.spicy_i32()).collect();

    let (c_states, c_out) = capture_stdout(|| {
        dirty
            .iter()
            .zip(vals.iter())
            .map(|(d, &v)| {
                let mut s = *d;
                unsafe { (p.c.init_state)(&mut s, v) };
                s.bytes()
            })
            .collect::<Vec<_>>()
    });
    let (r_states, r_out) = capture_stdout(|| {
        dirty
            .iter()
            .zip(vals.iter())
            .map(|(d, &v)| {
                let mut s = *d;
                unsafe { (p.rs.init_state)(&mut s, v) };
                s.bytes()
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(c_states, r_states, "[row26] dirty-buffer init differs");
    assert_stdout_eq("row26 init_state dirty", &c_out, &r_out);
    // Sanity: all three fields must have been overwritten.
    for (k, b) in c_states.iter().enumerate() {
        assert_eq!(&b[4..12], &[0u8; 8], "[row26] case {k} tail not zeroed");
    }
}

// ===========================================================================
// Rows 27-30: apply_operation
// ===========================================================================

#[test]
fn row27_apply_operation_each_op() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 27);
    let cases: Vec<(c_int, c_int, c_int)> = (0..4_000)
        .map(|_| (rng.spicy_i32(), rng.spicy_i32(), rng.range_i32(0, 3)))
        .collect();

    let run = |imp: &Impl| {
        cases
            .iter()
            .map(|&(init, value, opcode)| {
                let mut s = ComputeState::default();
                unsafe {
                    (imp.init_state)(&mut s, init);
                    let f = (imp.get_operation)(opcode);
                    (imp.apply_operation)(&mut s, value, f);
                }
                s.bytes()
            })
            .collect::<Vec<_>>()
    };
    let (c_states, c_out) = capture_stdout(|| run(&p.c));
    let (r_states, r_out) = capture_stdout(|| run(&p.rs));
    assert_eq!(c_states, r_states, "[row27] state bytes differ");
    assert_stdout_eq("row27 apply_operation", &c_out, &r_out);
}

#[test]
fn row28_apply_operation_repeated() {
    let p = pair();
    for n_calls in [1usize, 2, 10] {
        let mut rng = Rng::new(SEED ^ 28 ^ n_calls as u64);
        let seqs: Vec<(c_int, Vec<(c_int, c_int)>)> = (0..600)
            .map(|_| {
                (
                    rng.spicy_i32(),
                    (0..n_calls).map(|_| (rng.range_i32(0, 3), rng.spicy_i32())).collect(),
                )
            })
            .collect();

        let run = |imp: &Impl| {
            seqs.iter()
                .map(|(init, steps)| {
                    let mut s = ComputeState::default();
                    unsafe { (imp.init_state)(&mut s, *init) };
                    let mut trace = Vec::new();
                    for &(opcode, value) in steps {
                        unsafe {
                            let f = (imp.get_operation)(opcode);
                            (imp.apply_operation)(&mut s, value, f);
                        }
                        trace.extend_from_slice(&s.bytes());
                    }
                    trace
                })
                .collect::<Vec<_>>()
        };
        let (c_t, c_out) = capture_stdout(|| run(&p.c));
        let (r_t, r_out) = capture_stdout(|| run(&p.rs));
        assert_eq!(c_t, r_t, "[row28 n={n_calls}] state traces differ");
        assert_stdout_eq(&format!("row28 n={n_calls}"), &c_out, &r_out);
    }
}

#[test]
fn row29_apply_operation_preserves_checksum_field() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 29);
    let cases: Vec<(ComputeState, c_int, c_int)> = (0..2_000)
        .map(|_| {
            (
                ComputeState {
                    accumulator: rng.spicy_i32(),
                    operation_count: rng.range_i32(-5, 1000),
                    checksum: rng.next_i32() as c_uint,
                },
                rng.spicy_i32(),
                rng.range_i32(0, 3),
            )
        })
        .collect();

    let run = |imp: &Impl| {
        cases
            .iter()
            .map(|&(st, value, opcode)| {
                let mut s = st;
                unsafe {
                    let f = (imp.get_operation)(opcode);
                    (imp.apply_operation)(&mut s, value, f);
                }
                s.bytes()
            })
            .collect::<Vec<_>>()
    };
    let (c_states, c_out) = capture_stdout(|| run(&p.c));
    let (r_states, r_out) = capture_stdout(|| run(&p.rs));
    assert_eq!(c_states, r_states, "[row29] state bytes differ");
    assert_stdout_eq("row29 preset checksum", &c_out, &r_out);
    for (k, (st, _, _)) in cases.iter().enumerate() {
        assert_eq!(
            &c_states[k][8..12],
            &st.checksum.to_ne_bytes(),
            "[row29] case {k}: checksum field was modified"
        );
    }
}

#[test]
fn row30_apply_operation_cross_library_pointer() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 30);
    let cases: Vec<(c_int, c_int, usize)> = (0..2_000)
        .map(|_| (rng.spicy_i32(), rng.spicy_i32(), (rng.next_u64() % 4) as usize))
        .collect();

    // Feed the C library's op pointer to BOTH apply_operation entry points,
    // then the Rust library's op pointer to both.
    for (label, pick) in [("C-op", 0usize), ("Rust-op", 1usize)] {
        let run = |imp: &Impl| {
            cases
                .iter()
                .map(|&(init, value, i)| {
                    let f: OpFnRaw = Some(if pick == 0 {
                        p.c.op_by_index(i)
                    } else {
                        p.rs.op_by_index(i)
                    });
                    let mut s = ComputeState::default();
                    unsafe {
                        (imp.init_state)(&mut s, init);
                        (imp.apply_operation)(&mut s, value, f);
                    }
                    s.bytes()
                })
                .collect::<Vec<_>>()
        };
        let (c_states, c_out) = capture_stdout(|| run(&p.c));
        let (r_states, r_out) = capture_stdout(|| run(&p.rs));
        assert_eq!(c_states, r_states, "[row30 {label}] state bytes differ");
        assert_stdout_eq(&format!("row30 {label}"), &c_out, &r_out);
    }
}

// ===========================================================================
// Row 31: hand-composed pipeline out of the low-level exports
// ===========================================================================

#[test]
fn row31_manual_pipeline_from_low_level_exports() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 31);
    let params: Vec<[c_int; 4]> = (0..1_500)
        .map(|_| [rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()])
        .collect();
    let xor_name = CString::new("XOR").unwrap();
    let shift_name = CString::new("SHIFT").unwrap();

    let run = |imp: &Impl| {
        params
            .iter()
            .map(|pp| {
                let mut trace: Vec<u8> = Vec::new();
                unsafe {
                    let mut s = ComputeState::default();
                    (imp.init_state)(&mut s, pp[0]);
                    trace.extend_from_slice(&s.bytes());

                    let mult = (imp.get_operation)(0);
                    let add = (imp.get_operation)(1);
                    let xorf = (imp.get_operation)(2);
                    let shiftf = (imp.get_operation)(3);

                    (imp.apply_operation)(&mut s, pp[1], mult);
                    trace.extend_from_slice(&s.bytes());
                    (imp.apply_operation)(&mut s, pp[2], add);
                    trace.extend_from_slice(&s.bytes());

                    let xr = (imp.execute_operation)(xorf, s.accumulator, pp[3], xor_name.as_ptr());
                    trace.extend_from_slice(&xr.to_ne_bytes());
                    let sr = (imp.execute_operation)(shiftf, xr, pp[1], shift_name.as_ptr());
                    trace.extend_from_slice(&sr.to_ne_bytes());

                    let mut arr = *pp;
                    s.checksum = (imp.compute_checksum)(arr.as_mut_ptr(), 4);
                    trace.extend_from_slice(&s.bytes());

                    let final_result =
                        ((s.accumulator.wrapping_add(sr)) as c_uint ^ s.checksum) as c_int;
                    trace.extend_from_slice(&final_result.to_ne_bytes());
                }
                trace
            })
            .collect::<Vec<_>>()
    };
    let (c_t, c_out) = capture_stdout(|| run(&p.c));
    let (r_t, r_out) = capture_stdout(|| run(&p.rs));
    assert_eq!(c_t, r_t, "[row31] composed pipeline traces differ");
    assert_stdout_eq("row31 manual pipeline", &c_out, &r_out);

    // And the hand-composed result must equal the wrapper's result.
    for pp in params.iter().take(200) {
        let (cv, _) = capture_stdout(|| unsafe { (p.c.checkshift)(pp[0], pp[1], pp[2], pp[3]) });
        let manual = {
            let t = &c_t[params.iter().position(|q| q == pp).unwrap()];
            c_int::from_ne_bytes(t[t.len() - 4..].try_into().unwrap())
        };
        assert_eq!(cv, manual, "[row31] checkshift != manual composition for {pp:?}");
    }
}

// ===========================================================================
// Rows 32-36: checkshift (the public header's only entry point)
// ===========================================================================

fn diff_checkshift(row: &str, params: &[[c_int; 4]]) {
    let p = pair();
    let (c_res, c_out) = capture_stdout(|| {
        params
            .iter()
            .map(|q| unsafe { (p.c.checkshift)(q[0], q[1], q[2], q[3]) })
            .collect::<Vec<c_int>>()
    });
    let (r_res, r_out) = capture_stdout(|| {
        params
            .iter()
            .map(|q| unsafe { (p.rs.checkshift)(q[0], q[1], q[2], q[3]) })
            .collect::<Vec<c_int>>()
    });
    for (k, (cv, rv)) in c_res.iter().zip(r_res.iter()).enumerate() {
        assert_eq!(cv, rv, "[{row}] case {k} params={:?}: C={cv} Rust={rv}", params[k]);
    }
    assert_eq!(c_res.len(), r_res.len());
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(!c_out.is_empty());
}

#[test]
fn row32_checkshift_random() {
    let mut rng = Rng::new(SEED ^ 32);
    let params: Vec<[c_int; 4]> = (0..4_000)
        .map(|_| [rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()])
        .collect();
    diff_checkshift("row32 checkshift random", &params);
}

#[test]
fn row33_checkshift_all_zero() {
    diff_checkshift("row33 checkshift zeros", &[[0, 0, 0, 0]]);
}

#[test]
fn row34_checkshift_edge_cross_product() {
    let e = [0i32, 1, -1, i32::MAX, i32::MIN];
    let mut params = Vec::new();
    for &a in &e {
        for &b in &e {
            for &c in &e {
                for &d in &e {
                    params.push([a, b, c, d]);
                }
            }
        }
    }
    assert_eq!(params.len(), 625);
    diff_checkshift("row34 checkshift edges", &params);
}

#[test]
fn row35_checkshift_small_magnitude() {
    let mut rng = Rng::new(SEED ^ 35);
    let params: Vec<[c_int; 4]> = (0..3_000)
        .map(|_| {
            [
                rng.range_i32(-16, 16),
                rng.range_i32(-16, 16),
                rng.range_i32(-16, 16),
                rng.range_i32(-16, 16),
            ]
        })
        .collect();
    diff_checkshift("row35 checkshift small", &params);
}

#[test]
fn row36_checkshift_checksum_hex_width() {
    // Search for params whose checksum needs zero padding in "0x%04X"
    // (< 0x1000) and params whose checksum uses all four digits.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 36);
    let mut narrow: Vec<[c_int; 4]> = Vec::new();
    let mut wide: Vec<[c_int; 4]> = Vec::new();
    let mut tries = 0;
    while (narrow.len() < 40 || wide.len() < 40) && tries < 400_000 {
        tries += 1;
        let mut q = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
        if tries % 3 == 0 {
            q = [
                rng.range_i32(-4096, 4096),
                rng.range_i32(-4096, 4096),
                rng.range_i32(-4096, 4096),
                rng.range_i32(-4096, 4096),
            ];
        }
        let mut arr = q;
        let cs = unsafe { (p.c.compute_checksum)(arr.as_mut_ptr(), 4) };
        if cs < 0x1000 && narrow.len() < 40 {
            narrow.push(q);
        } else if cs >= 0x1000 && wide.len() < 40 {
            wide.push(q);
        }
    }
    assert!(!narrow.is_empty(), "[row36] found no narrow checksum in {tries} tries");
    assert!(!wide.is_empty(), "[row36] found no wide checksum");
    let mut all = narrow.clone();
    all.extend(wide.iter().copied());
    diff_checkshift("row36 checksum hex width", &all);

    // Confirm the padding actually appears in the captured output.
    let (_, out) = capture_stdout(|| unsafe {
        (p.c.checkshift)(narrow[0][0], narrow[0][1], narrow[0][2], narrow[0][3])
    });
    let text = String::from_utf8_lossy(&out);
    let line = text
        .lines()
        .find(|l| l.starts_with("Computed checksum:"))
        .expect("[row36] no checksum line");
    assert_eq!(line.len(), "Computed checksum: 0xFFFF".len(), "[row36] not 4 hex digits: {line}");
}

// keep c_char import used on all platforms
const _: Option<*const c_char> = None;

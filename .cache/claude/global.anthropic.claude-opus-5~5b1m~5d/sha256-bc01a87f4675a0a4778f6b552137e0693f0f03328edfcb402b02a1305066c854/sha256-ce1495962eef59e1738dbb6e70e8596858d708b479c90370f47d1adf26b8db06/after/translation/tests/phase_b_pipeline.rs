// Phase B — CONFIGS.md rows C27..C31
//
// End-to-end: the `checkshift` one-shot wrapper, a hand-composed pipeline built
// out of the LOW-LEVEL entry points (which catches divergence that per-wrapper
// tests cannot see), and an interleaved fuzz driver over every entry point.

mod common;
use common::*;

use std::ffi::CString;

/// Fixed seed for reproducibility.
const SEED: u64 = 0x91FE_1111_2222_3333;

/// Run `checkshift` on both libraries and compare return value + full transcript.
#[track_caller]
fn diff_checkshift(ctx: &str, p: [i32; 4]) -> i32 {
    let (c, r) = libs();
    let (cv, co) = capture(|| unsafe { (c.checkshift)(p[0], p[1], p[2], p[3]) });
    let (rv, ro) = capture(|| unsafe { (r.checkshift)(p[0], p[1], p[2], p[3]) });
    assert_eq!(
        cv, rv,
        "{ctx}: checkshift{p:?} return value (C={cv}, Rust={rv})"
    );
    assert_stdout_eq(&format!("{ctx}: checkshift{p:?}"), &co, &ro);
    assert!(!co.is_empty(), "{ctx}: expected a transcript");
    cv
}

// ---------------------------------------------------------------------------
// C27 — 2000 random 4-tuples over the full i32 range
// ---------------------------------------------------------------------------

#[test]
fn c27_checkshift_random_tuples() {
    let mut rng = Rng::new(SEED ^ 0xC27);
    for i in 0..2000 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        diff_checkshift(&format!("C27 iter {i}"), p);
    }
}

#[test]
fn c27b_checkshift_uniform_random_tuples() {
    // Pure uniform i32 (no boundary bias), to catch value-dependent bugs.
    let mut rng = Rng::new(SEED ^ 0xC27B);
    for i in 0..1000 {
        let p = [rng.i32(), rng.i32(), rng.i32(), rng.i32()];
        diff_checkshift(&format!("C27b iter {i}"), p);
    }
}

// ---------------------------------------------------------------------------
// C28 — boundary 4-tuples: all 5^4 = 625 combinations
// ---------------------------------------------------------------------------

#[test]
fn c28_checkshift_boundary_tuples() {
    let vals = [0i32, 1, -1, i32::MAX, i32::MIN];
    for &a in &vals {
        for &b in &vals {
            for &cc in &vals {
                for &d in &vals {
                    diff_checkshift("C28", [a, b, cc, d]);
                }
            }
        }
    }
}

#[test]
fn c28b_checkshift_wider_boundary_tuples() {
    // A wider boundary set on the two params that feed the shift/multiply
    // stages, with the others held at interesting values.
    let vals = [
        0i32,
        1,
        -1,
        2,
        -2,
        3,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x4000_0000,
        0xC000_0000u32 as i32,
        0xABCD,
        100,
        -100,
    ];
    for &a in &vals {
        for &b in &vals {
            diff_checkshift("C28b p3=0x0102_0304 p4=-7", [a, b, 0x0102_0304, -7]);
        }
    }
}

// ---------------------------------------------------------------------------
// C29 — exercise the `0x%04X` checksum formatting across all widths
// ---------------------------------------------------------------------------

#[test]
fn c29_checkshift_checksum_format_widths() {
    let c = c_lib();

    // Find, by scanning param4, one tuple per checksum magnitude bucket so the
    // zero-padding of `printf("0x%04X")` is exercised for 1-, 2-, 3- and
    // 4-digit values (and for exactly zero).
    let mut found: Vec<(u32, [i32; 4])> = Vec::new();
    let buckets: [(u32, u32); 5] = [
        (0, 0),           // prints 0x0000
        (0x1, 0xF),       // prints 0x000X
        (0x10, 0xFF),     // prints 0x00XX
        (0x100, 0xFFF),   // prints 0x0XXX
        (0x1000, 0xFFFF), // prints 0xXXXX
    ];
    let mut hit = [false; 5];

    // All FOUR params must vary: `checksum & 0xFFFF` bits 11..15 are determined
    // by the earlier params' bytes, so scanning param4 alone can never leave the
    // top bucket (param4's bytes only reach bits 0..10).
    let mut rng = Rng::new(SEED ^ 0xC29);
    for _ in 0..4_000_000u32 {
        let p = [rng.i32(), rng.i32(), rng.i32(), rng.i32()];
        let mut vals = p.to_vec();
        let sum = unsafe { (c.compute_checksum)(vals.as_mut_ptr(), 4) };
        for (bi, &(lo, hi)) in buckets.iter().enumerate() {
            if !hit[bi] && sum >= lo && sum <= hi {
                hit[bi] = true;
                found.push((sum, p));
            }
        }
        if hit.iter().all(|&h| h) {
            break;
        }
    }

    assert!(
        hit.iter().all(|&h| h),
        "C29: expected to find all 5 checksum magnitude buckets, hit = {hit:?}"
    );

    for (sum, p) in &found {
        let ctx = format!("C29 checksum=0x{sum:04X}");
        diff_checkshift(&ctx, *p);
        // Confirm the transcript really contains the padded form.
        let (_, co) = capture(|| unsafe { (c.checkshift)(p[0], p[1], p[2], p[3]) });
        let text = String::from_utf8_lossy(&co);
        assert!(
            text.contains(&format!("Computed checksum: 0x{sum:04X}")),
            "C29: transcript missing padded checksum 0x{sum:04X}: {text:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// C30 — hand-composed pipeline out of the LOW-LEVEL entry points
//
// Replicates `checkshift`'s body using init_state / get_operation /
// apply_operation / execute_operation / compute_checksum called directly across
// the FFI, comparing the state struct after EVERY step. Divergence in the
// composed pipeline is invisible to per-wrapper tests.
// ---------------------------------------------------------------------------

#[test]
fn c30_composed_pipeline_matches_step_by_step() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0xC30);
    let xor_name = CString::new("XOR").unwrap();
    let shift_name = CString::new("SHIFT").unwrap();

    for i in 0..500 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let ctx = |step: &str| format!("C30 iter {i} {step} params={p:?}");

        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();

        // --- init_state(state, param1)
        let (_, co) = capture(|| unsafe { (c.init_state)(cb.as_ptr(), p[0]) });
        let (_, ro) = capture(|| unsafe { (r.init_state)(rb.as_ptr(), p[0]) });
        assert_stdout_eq(&ctx("init_state"), &co, &ro);
        assert_eq!(cb.state(), rb.state(), "{}", ctx("after init_state"));
        assert_eq!(cb.bytes(), rb.bytes(), "{}", ctx("bytes after init_state"));

        // --- get_operation(0..3)
        let cops: Vec<OptOperationFunc> =
            (0..4).map(|k| unsafe { (c.get_operation)(k) }).collect();
        let rops: Vec<OptOperationFunc> =
            (0..4).map(|k| unsafe { (r.get_operation)(k) }).collect();
        for k in 0..4 {
            assert_eq!(
                cops[k].is_some(),
                rops[k].is_some(),
                "{}",
                ctx(&format!("get_operation({k}) NULL-ness"))
            );
        }

        // --- apply_operation(state, param2, multiply)
        let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), p[1], cops[0]) });
        let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), p[1], rops[0]) });
        assert_stdout_eq(&ctx("apply multiply"), &co, &ro);
        assert_eq!(cb.state(), rb.state(), "{}", ctx("after apply multiply"));

        // --- apply_operation(state, param3, add)
        let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), p[2], cops[1]) });
        let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), p[2], rops[1]) });
        assert_stdout_eq(&ctx("apply add"), &co, &ro);
        assert_eq!(cb.state(), rb.state(), "{}", ctx("after apply add"));

        // --- execute_operation(xor, accumulator, param4, "XOR")
        let acc_c = cb.state().accumulator;
        let acc_r = rb.state().accumulator;
        let (cx, co) =
            capture(|| unsafe { (c.execute_operation)(cops[2], acc_c, p[3], xor_name.as_ptr()) });
        let (rx, ro) =
            capture(|| unsafe { (r.execute_operation)(rops[2], acc_r, p[3], xor_name.as_ptr()) });
        assert_eq!(cx, rx, "{}", ctx("xor_result"));
        assert_stdout_eq(&ctx("execute xor"), &co, &ro);

        // --- execute_operation(shift, xor_result, param2, "SHIFT")
        let (cs, co) =
            capture(|| unsafe { (c.execute_operation)(cops[3], cx, p[1], shift_name.as_ptr()) });
        let (rs, ro) =
            capture(|| unsafe { (r.execute_operation)(rops[3], rx, p[1], shift_name.as_ptr()) });
        assert_eq!(cs, rs, "{}", ctx("shift_result"));
        assert_stdout_eq(&ctx("execute shift"), &co, &ro);

        // --- compute_checksum(params, 4)
        let mut cparams = p.to_vec();
        let mut rparams = p.to_vec();
        let csum = unsafe { (c.compute_checksum)(cparams.as_mut_ptr(), 4) };
        let rsum = unsafe { (r.compute_checksum)(rparams.as_mut_ptr(), 4) };
        assert_eq!(csum, rsum, "{}", ctx("checksum"));

        // --- final_result = (accumulator + shift_result) ^ checksum
        let mut cst = cb.state();
        let mut rst = rb.state();
        cst.checksum = csum;
        rst.checksum = rsum;
        cb.set_state(cst);
        rb.set_state(rst);
        let c_final = (cst.accumulator.wrapping_add(cs) as u32 ^ cst.checksum) as i32;
        let r_final = (rst.accumulator.wrapping_add(rs) as u32 ^ rst.checksum) as i32;
        assert_eq!(c_final, r_final, "{}", ctx("final_result"));
        assert_eq!(cb.bytes(), rb.bytes(), "{}", ctx("final state bytes"));

        // The composed pipeline must reproduce exactly what the one-shot
        // `checkshift` wrapper returns -- in BOTH libraries.
        let (cw, _) = capture(|| unsafe { (c.checkshift)(p[0], p[1], p[2], p[3]) });
        let (rw, _) = capture(|| unsafe { (r.checkshift)(p[0], p[1], p[2], p[3]) });
        assert_eq!(
            cw,
            c_final,
            "{}",
            ctx("C: composed pipeline vs C checkshift wrapper")
        );
        assert_eq!(
            rw,
            r_final,
            "{}",
            ctx("Rust: composed pipeline vs Rust checkshift wrapper")
        );
        assert_eq!(cw, rw, "{}", ctx("checkshift wrappers"));
        assert_eq!(cb.state().operation_count, 2, "{}", ctx("operation_count"));
    }
}

// ---------------------------------------------------------------------------
// C31 — interleaved randomized fuzz over EVERY entry point, sharing state
// ---------------------------------------------------------------------------

#[test]
fn c31_interleaved_fuzz_all_entry_points() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0xC31);
    let names: Vec<CString> = ["XOR", "SHIFT", "", "OP"]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();

    let mut cb = StateBuf::new();
    let mut rb = StateBuf::new();
    let _ = capture(|| unsafe { (c.init_state)(cb.as_ptr(), 0) });
    let _ = capture(|| unsafe { (r.init_state)(rb.as_ptr(), 0) });

    for step in 0..3000 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let opcode = (rng.next_u32() % 7) as i32 - 1; // -1..5, includes invalid
        let name = &names[(rng.next_u32() as usize) % names.len()];
        let which = rng.next_u32() % 10;
        let ctx = format!("C31 step {step} which={which} a={a} b={b} opcode={opcode}");

        match which {
            0 => {
                let cv = unsafe { (c.multiply_with_static)(a, b) };
                let rv = unsafe { (r.multiply_with_static)(a, b) };
                assert_eq!(cv, rv, "{ctx} multiply_with_static");
            }
            1 => {
                let cv = unsafe { (c.add_with_static)(a, b) };
                let rv = unsafe { (r.add_with_static)(a, b) };
                assert_eq!(cv, rv, "{ctx} add_with_static");
            }
            2 => {
                let cv = unsafe { (c.xor_operation)(a, b) };
                let rv = unsafe { (r.xor_operation)(a, b) };
                assert_eq!(cv, rv, "{ctx} xor_operation");
            }
            3 => {
                let cv = unsafe { (c.shift_with_static)(a, b) };
                let rv = unsafe { (r.shift_with_static)(a, b) };
                assert_eq!(cv, rv, "{ctx} shift_with_static");
            }
            4 => {
                let cv = unsafe { (c.get_operation)(opcode) };
                let rv = unsafe { (r.get_operation)(opcode) };
                assert_eq!(cv.is_some(), rv.is_some(), "{ctx} get_operation");
            }
            5 => {
                let cf = unsafe { (c.get_operation)(opcode) };
                let rf = unsafe { (r.get_operation)(opcode) };
                let (cv, co) =
                    capture(|| unsafe { (c.execute_operation)(cf, a, b, name.as_ptr()) });
                let (rv, ro) =
                    capture(|| unsafe { (r.execute_operation)(rf, a, b, name.as_ptr()) });
                assert_eq!(cv, rv, "{ctx} execute_operation");
                assert_stdout_eq(&format!("{ctx} execute_operation"), &co, &ro);
            }
            6 => {
                let count = (rng.next_u32() % 8) as i32 - 1; // -1..6
                let values: Vec<i32> = (0..8).map(|_| rng.interesting_i32()).collect();
                let mut cvv = values.clone();
                let mut rvv = values.clone();
                let cv = unsafe { (c.compute_checksum)(cvv.as_mut_ptr(), count) };
                let rv = unsafe { (r.compute_checksum)(rvv.as_mut_ptr(), count) };
                assert_eq!(cv, rv, "{ctx} compute_checksum count={count}");
            }
            7 => {
                let (_, co) = capture(|| unsafe { (c.init_state)(cb.as_ptr(), a) });
                let (_, ro) = capture(|| unsafe { (r.init_state)(rb.as_ptr(), a) });
                assert_stdout_eq(&format!("{ctx} init_state"), &co, &ro);
                assert_eq!(cb.bytes(), rb.bytes(), "{ctx} init_state bytes");
            }
            8 => {
                let cf = unsafe { (c.get_operation)(opcode) };
                let rf = unsafe { (r.get_operation)(opcode) };
                let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), a, cf) });
                let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), a, rf) });
                assert_stdout_eq(&format!("{ctx} apply_operation"), &co, &ro);
                assert_eq!(cb.bytes(), rb.bytes(), "{ctx} apply_operation bytes");
            }
            _ => {
                let d = rng.interesting_i32();
                let e = rng.interesting_i32();
                let (cv, co) = capture(|| unsafe { (c.checkshift)(a, b, d, e) });
                let (rv, ro) = capture(|| unsafe { (r.checkshift)(a, b, d, e) });
                assert_eq!(cv, rv, "{ctx} checkshift({a},{b},{d},{e})");
                assert_stdout_eq(&format!("{ctx} checkshift({a},{b},{d},{e})"), &co, &ro);
            }
        }

        // State buffers must stay in lockstep for the whole run.
        assert_eq!(cb.bytes(), rb.bytes(), "{ctx}: state drift");
        assert!(cb.guard_intact() && rb.guard_intact(), "{ctx}: guard clobbered");
    }
}

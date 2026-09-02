//! Phase B/D — differential tests for `buffapp` and the composed low-level
//! pipeline, plus the cross-library ABI checks.
//!
//! `harness = false`: this binary owns `main()` and runs every scenario
//! sequentially on one thread. That is required, not stylistic — `buffapp`
//! writes its log with `printf`, so comparing that output means redirecting
//! fd 1, which is process-global. Under libtest's default thread pool,
//! libtest's own progress lines get written into the capture window and show up
//! as bogus divergences.

mod support;

use std::ffi::{c_char, c_int};
use support::*;

macro_rules! diff {
    ($row:literal, $inputs:expr, $c:expr, $rs:expr) => {
        assert_eq!(
            $c, $rs,
            "CONFIGS.md row {} diverged for inputs {:?}\n  C   = {:?}\n  Rust= {:?}",
            $row, $inputs, $c, $rs
        )
    };
}

/// Runs `buffapp` in both libraries, capturing stdout for each, and returns
/// `((c_ret, c_stdout), (rs_ret, rs_stdout))`.
fn buffapp_both(a: c_int, b: c_int, c: c_int, d: c_int) -> ((c_int, Vec<u8>), (c_int, Vec<u8>)) {
    let p = pair();
    let (cr, cout) = capture_stdout(|| unsafe { (p.c.buffapp)(a, b, c, d) });
    let (rr, rout) = capture_stdout(|| unsafe { (p.rs.buffapp)(a, b, c, d) });
    ((cr, cout), (rr, rout))
}

/// `buffapp` return value only (no stdout capture) — cheaper for bulk rows.
fn buffapp_ret_both(a: c_int, b: c_int, c: c_int, d: c_int) -> (c_int, c_int) {
    let p = pair();
    let (cr, _) = capture_stdout(|| unsafe { (p.c.buffapp)(a, b, c, d) });
    let (rr, _) = capture_stdout(|| unsafe { (p.rs.buffapp)(a, b, c, d) });
    (cr, rr)
}

/// Builds an `i32` whose C `% 4` residue is exactly `r` (`r` in `-3..=3`).
fn with_residue(rng: &mut Rng, r: i32) -> i32 {
    const KMAX: u32 = 536_870_910; // (i32::MAX / 4) - 1
    let k = rng.range(0, KMAX) as i32;
    if r >= 0 {
        k * 4 + r
    } else {
        -(k * 4 + (-r))
    }
}

const RESIDUES: [i32; 7] = [0, 1, 2, 3, -1, -2, -3];

// ===========================================================================
// Row 25 — all 7x7 residue combinations of param1 % 4 x param3 % 4.
// ===========================================================================
fn row25_buffapp_all_residue_combinations() {
    let mut rng = Rng::new(0x2525_2525);
    for r1 in RESIDUES {
        for r3 in RESIDUES {
            for _ in 0..20 {
                let p1 = with_residue(&mut rng, r1);
                let p3 = with_residue(&mut rng, r3);
                let p2 = rng.interesting_i32();
                let p4 = rng.interesting_i32();
                let (c, r) = buffapp_both(p1, p2, p3, p4);
                assert_eq!(
                    c.0, r.0,
                    "row 25 return diverged: residues ({r1},{r3}) params ({p1},{p2},{p3},{p4}): C={} Rust={}",
                    c.0, r.0
                );
                assert_eq!(
                    c.1,
                    r.1,
                    "row 25 stdout diverged: residues ({r1},{r3}) params ({p1},{p2},{p3},{p4})\n  C   = {:?}\n  Rust= {:?}",
                    String::from_utf8_lossy(&c.1),
                    String::from_utf8_lossy(&r.1)
                );
                // Sanity: the log really was produced.
                assert!(c.1.starts_with(b"Computation Log:\n"));
            }
        }
    }
}

// ===========================================================================
// Row 26 — intermediate3 == 0 (the `else` branch: sum of all four params).
// ===========================================================================
fn row26_buffapp_intermediate3_zero_sum_path() {
    let mut rng = Rng::new(0x2626_2626);
    let mut cases = 0;
    // residue -1/-2/-3 selects "unknown" => intermediate == 0 => product == 0.
    for r1 in [-1, -2, -3] {
        for r3 in RESIDUES {
            for _ in 0..15 {
                let p1 = with_residue(&mut rng, r1);
                let p3 = with_residue(&mut rng, r3);
                let p2 = rng.interesting_i32();
                let p4 = rng.interesting_i32();
                let (c, r) = buffapp_both(p1, p2, p3, p4);
                diff!(26, (p1, p2, p3, p4), c, r);
                let want = p1.wrapping_add(p2).wrapping_add(p3).wrapping_add(p4);
                assert_eq!(c.0, want, "sum path expected for ({p1},{p2},{p3},{p4})");
                cases += 1;
            }
        }
    }
    // Also reach it with a genuine zero intermediate: op1 = "multiply" (res 2)
    // with param2 == 0.
    for _ in 0..40 {
        let p1 = with_residue(&mut rng, 2);
        let p3 = with_residue(&mut rng, 0);
        let (c, r) = buffapp_both(p1, 0, p3, rng.interesting_i32());
        diff!(26, (p1, 0, p3), c, r);
        cases += 1;
    }
    assert!(cases > 300, "row 26 covered {cases} cases");
}

// ===========================================================================
// Row 27 — intermediate3 != 0 (the divide branch).
// ===========================================================================
fn row27_buffapp_intermediate3_nonzero_divide_path() {
    let mut rng = Rng::new(0x2727_2727);
    let mut cases = 0;
    // op1 = "add" (residue 0) and op2 = "add" (residue 0): pick params so both
    // intermediates are non-zero.
    for _ in 0..200 {
        let p1 = with_residue(&mut rng, 0);
        let p3 = with_residue(&mut rng, 0);
        let p2 = (rng.range(1, 1000)) as i32;
        let p4 = (rng.range(1, 1000)) as i32;
        let i1 = p1.wrapping_add(p2);
        let i2 = p3.wrapping_add(p4);
        if i1 == 0 || i2 == 0 || i1.wrapping_mul(i2) == 0 {
            continue;
        }
        let (c, r) = buffapp_both(p1, p2, p3, p4);
        diff!(27, (p1, p2, p3, p4), c, r);
        let want = i1.wrapping_add(i2) / i1.wrapping_mul(i2);
        assert_eq!(c.0, want, "divide path expected for ({p1},{p2},{p3},{p4})");
        cases += 1;
    }
    // Mixed operations, residues 1/2/3 on both sides.
    for r1 in [1, 2, 3] {
        for r3 in [1, 2, 3] {
            for _ in 0..15 {
                let p1 = with_residue(&mut rng, r1);
                let p3 = with_residue(&mut rng, r3);
                let p2 = rng.range(1, 10_000) as i32;
                let p4 = rng.range(1, 10_000) as i32;
                let (c, r) = buffapp_both(p1, p2, p3, p4);
                diff!(27, (p1, p2, p3, p4), c, r);
                cases += 1;
            }
        }
    }
    assert!(cases > 200, "row 27 covered {cases} cases");
}

// ===========================================================================
// Row 28 — randomized full-i32 params through the whole pipeline.
// ===========================================================================
fn row28_buffapp_randomized_full_range() {
    let mut rng = Rng::new(0x2828_2828);
    for _ in 0..1500 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        let p4 = rng.interesting_i32();
        let (c, r) = buffapp_ret_both(p1, p2, p3, p4);
        diff!(28, (p1, p2, p3, p4), c, r);
    }
}

fn row28b_buffapp_randomized_full_range_stdout() {
    let mut rng = Rng::new(0x28B0_28B0);
    for _ in 0..400 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        let p4 = rng.interesting_i32();
        let (c, r) = buffapp_both(p1, p2, p3, p4);
        diff!(28, (p1, p2, p3, p4), c, r);
    }
}

// ===========================================================================
// Row 29 — corner tuples.
// ===========================================================================
fn row29_buffapp_corner_tuples() {
    let corners: [i32; 9] = [0, 1, -1, 2, -2, 3, -3, i32::MIN, i32::MAX];
    for &a in &corners {
        for &b in &corners {
            for &c in &corners {
                for &d in &corners {
                    let (cc, rr) = buffapp_ret_both(a, b, c, d);
                    diff!(29, (a, b, c, d), cc, rr);
                }
            }
        }
    }
    // Full stdout comparison for the all-equal tuples.
    for &v in &corners {
        let (c, r) = buffapp_both(v, v, v, v);
        diff!(30, (v, v, v, v), c, r);
    }
}

// ===========================================================================
// Row 30 — stdout byte comparison is asserted inside rows 25/26/27/28b/29.
// This row additionally pins the exact expected log text for a fixed tuple so
// a silent change in either library's formatting is caught.
// ===========================================================================
fn row30_buffapp_stdout_exact_text() {
    let (c, r) = buffapp_both(4, 5, 9, 2);
    diff!(30, (4, 5, 9, 2), c, r);
    let expected = concat!(
        "Computation Log:\n",
        "Starting computation with 4 parameters\n",
        "Operation 1: add(4, 5)\n",
        "Operation 2: subtract(9, 2)\n",
        "Operation 3: multiply(9, 7)\n",
        "Final result: 0\n",
        "\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&c.1),
        expected,
        "C log text changed unexpectedly"
    );
    assert_eq!(c.0, (9 + 7) / (9 * 7));
}

// ===========================================================================
// Row 31 — hand-composed low-level pipeline mirroring buffapp's internals.
// ===========================================================================
fn row31_low_level_pipeline_composition() {
    let p = pair();
    let mut rng = Rng::new(0x3131_3131);
    unsafe {
        for _ in 0..200 {
            let p1 = rng.interesting_i32();
            let p2 = rng.interesting_i32();
            let p3 = rng.interesting_i32();
            let p4 = rng.interesting_i32();

            let cb = (p.c.create_buffer)(32);
            let rb = (p.rs.create_buffer)(32);
            assert!(!cb.is_null() && !rb.is_null());
            diff!(31, "post-create", snapshot(cb), snapshot(rb));

            // buffapp's line 116.
            (*cb).length = 0;
            (*rb).length = 0;

            let push = |line: String, tag: &str| {
                let mut bytes = line.into_bytes();
                bytes.push(0);
                let ptr = bytes.as_ptr() as *const c_char;
                let cr = (p.c.append_to_buffer)(cb, ptr);
                let rr = (p.rs.append_to_buffer)(rb, ptr);
                assert_eq!(
                    (cr, snapshot(cb)),
                    (rr, snapshot(rb)),
                    "CONFIGS.md row 31 diverged at {tag} for ({p1},{p2},{p3},{p4})"
                );
            };

            push(
                "Starting computation with 4 parameters\n".to_string(),
                "line1",
            );

            let c_op1 = (p.c.get_operation_name)(p1.wrapping_rem(4));
            let r_op1 = (p.rs.get_operation_name)(p1.wrapping_rem(4));
            diff!(31, "op1", cstr_bytes(c_op1), cstr_bytes(r_op1));
            let name1 = String::from_utf8(cstr_bytes(c_op1).unwrap()).unwrap();
            push(format!("Operation 1: {name1}({p1}, {p2})\n"), "line2");

            let i1c = (p.c.perform_operation)(p1, p2, c_op1);
            let i1r = (p.rs.perform_operation)(p1, p2, r_op1);
            diff!(31, "intermediate1", i1c, i1r);

            let c_op2 = (p.c.get_operation_name)(p3.wrapping_rem(4));
            let r_op2 = (p.rs.get_operation_name)(p3.wrapping_rem(4));
            diff!(31, "op2", cstr_bytes(c_op2), cstr_bytes(r_op2));
            let name2 = String::from_utf8(cstr_bytes(c_op2).unwrap()).unwrap();
            push(format!("Operation 2: {name2}({p3}, {p4})\n"), "line3");

            let i2c = (p.c.perform_operation)(p3, p4, c_op2);
            let i2r = (p.rs.perform_operation)(p3, p4, r_op2);
            diff!(31, "intermediate2", i2c, i2r);

            push(format!("Operation 3: multiply({i1c}, {i2c})\n"), "line4");

            let mul = b"multiply\0";
            let i3c = (p.c.perform_operation)(i1c, i2c, mul.as_ptr() as *const c_char);
            let i3r = (p.rs.perform_operation)(i1r, i2r, mul.as_ptr() as *const c_char);
            diff!(31, "intermediate3", i3c, i3r);

            let result = if i3c != 0 {
                // Proven unreachable: i1*i2 == -1 forces i1+i2 == 0.
                assert!(
                    !(i1c.wrapping_add(i2c) == i32::MIN && i3c == -1),
                    "unexpected INT_MIN / -1"
                );
                i1c.wrapping_add(i2c) / i3c
            } else {
                p1.wrapping_add(p2).wrapping_add(p3).wrapping_add(p4)
            };
            push(format!("Final result: {result}\n"), "line5");

            // The composed pipeline must agree with the one-shot wrapper.
            let (oneshot_c, oneshot_r) = buffapp_ret_both(p1, p2, p3, p4);
            diff!(31, (p1, p2, p3, p4), oneshot_c, oneshot_r);
            assert_eq!(
                oneshot_c, result,
                "row 31: composed pipeline disagrees with buffapp for ({p1},{p2},{p3},{p4})"
            );

            (p.c.destroy_buffer)(cb);
            (p.rs.destroy_buffer)(rb);
        }
    }
}

// ===========================================================================
// Row 32 — cross-library ABI interchange.
// ===========================================================================
fn row32_cross_library_abi_interchange() {
    let p = pair();
    let mut rng = Rng::new(0x3232_3232);
    unsafe {
        // struct layout must be identical in both directions
        for _ in 0..200 {
            let cap = rng.range(0, 64) as c_int;
            let s1 = rng.cstring_len(0, 50);
            let s2 = rng.cstring_len(0, 50);

            // C creates, Rust appends, C appends, Rust destroys.
            let b1 = (p.c.create_buffer)(cap);
            let a = (p.rs.append_to_buffer)(b1, s1.as_ptr() as *const c_char);
            let b = (p.c.append_to_buffer)(b1, s2.as_ptr() as *const c_char);
            let mixed1 = (a, b, snapshot(b1));
            (p.rs.destroy_buffer)(b1);

            // Rust creates, C appends, Rust appends, C destroys.
            let b2 = (p.rs.create_buffer)(cap);
            let a = (p.c.append_to_buffer)(b2, s1.as_ptr() as *const c_char);
            let b = (p.rs.append_to_buffer)(b2, s2.as_ptr() as *const c_char);
            let mixed2 = (a, b, snapshot(b2));
            (p.c.destroy_buffer)(b2);

            diff!(32, (cap, s1.len(), s2.len()), mixed1, mixed2);

            // Both must equal the homogeneous C-only result.
            let b3 = (p.c.create_buffer)(cap);
            let a = (p.c.append_to_buffer)(b3, s1.as_ptr() as *const c_char);
            let b = (p.c.append_to_buffer)(b3, s2.as_ptr() as *const c_char);
            let pure_c = (a, b, snapshot(b3));
            (p.c.destroy_buffer)(b3);
            diff!(32, (cap, "pure-C vs mixed"), pure_c, mixed1);
        }
        // Struct size / field offsets, as observed through the exported API.
        assert_eq!(std::mem::size_of::<StringBuffer>(), 16);
    }
}

// ===========================================================================
// Additional observable properties of the C that the rows above do not pin.
// ===========================================================================

/// The C returns string-literal addresses from `get_operation_name`, so the
/// pointer is stable across calls, equal for every `default` code, and distinct
/// per matched code. A Rust translation that built a fresh buffer per call, or
/// that let each match arm promote its own static, would break these.
fn extra_get_operation_name_pointer_identity() {
    let p = pair();
    unsafe {
        for imp in [&p.c, &p.rs] {
            // Stable across repeated calls.
            for code in [0, 1, 2, 3, 4, -1, i32::MIN, i32::MAX] {
                let a = (imp.get_operation_name)(code);
                let b = (imp.get_operation_name)(code);
                assert_eq!(a, b, "{}: pointer not stable for code {code}", imp.name);
            }
            // Every out-of-range code shares one "unknown" address.
            let base = (imp.get_operation_name)(4);
            for code in [-1, -2, -3, 5, 99, i32::MIN, i32::MAX] {
                assert_eq!(
                    (imp.get_operation_name)(code),
                    base,
                    "{}: code {code} should share the \"unknown\" literal",
                    imp.name
                );
            }
            // The four matched codes are pairwise distinct addresses.
            let ptrs: Vec<_> = (0..4).map(|c| (imp.get_operation_name)(c)).collect();
            for i in 0..4 {
                for j in (i + 1)..4 {
                    assert_ne!(ptrs[i], ptrs[j], "{}: codes {i}/{j} aliased", imp.name);
                }
            }
            assert!(!ptrs.contains(&base), "{}: matched code aliased unknown", imp.name);
        }
    }
}

/// Neither library may carry state between calls: `buffapp` allocates and frees
/// its own buffer each time, so repeated identical calls must be identical.
fn extra_no_cross_call_state() {
    let p = pair();
    let mut rng = Rng::new(0xC0FF_EE00);
    for _ in 0..20 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let c = rng.interesting_i32();
        let d = rng.interesting_i32();
        let mut first: Option<(c_int, Vec<u8>)> = None;
        for iter in 0..25 {
            let (cr, cout) = capture_stdout(|| unsafe { (p.c.buffapp)(a, b, c, d) });
            let (rr, rout) = capture_stdout(|| unsafe { (p.rs.buffapp)(a, b, c, d) });
            assert_eq!(
                (cr, &cout),
                (rr, &rout),
                "iteration {iter} diverged for ({a},{b},{c},{d})"
            );
            match &first {
                None => first = Some((cr, cout)),
                Some(f) => assert_eq!(
                    (f.0, &f.1),
                    (cr, &cout),
                    "iteration {iter} differs from the first call for ({a},{b},{c},{d})"
                ),
            }
        }
    }
}

/// A buffer must survive an arbitrary number of appends interleaved between the
/// two libraries and still be freeable by either one.
fn extra_long_interleaved_append_run() {
    let p = pair();
    let mut rng = Rng::new(0xBEEF_0001);
    unsafe {
        let cb = (p.c.create_buffer)(1);
        let rb = (p.rs.create_buffer)(1);
        for step in 0..2000 {
            let s = rng.cstring_len(0, 8);
            let ptr = s.as_ptr() as *const c_char;
            // Alternate which library performs the append on each buffer, but
            // keep the two buffers' histories identical.
            let (crc, rrc) = if step % 2 == 0 {
                ((p.c.append_to_buffer)(cb, ptr), (p.rs.append_to_buffer)(rb, ptr))
            } else {
                ((p.rs.append_to_buffer)(cb, ptr), (p.c.append_to_buffer)(rb, ptr))
            };
            assert_eq!(
                (crc, snapshot(cb)),
                (rrc, snapshot(rb)),
                "interleaved append diverged at step {step}"
            );
        }
        (p.rs.destroy_buffer)(cb);
        (p.c.destroy_buffer)(rb);
    }
}

// ---------------------------------------------------------------------------
// Row 18 — buffapp's intermediate3 == 0 branch.
// ---------------------------------------------------------------------------
fn row18_buffapp_zero_product_takes_sum_branch() {
    let p = pair();
    unsafe {
        // param1 % 4 == -1 => "unknown" => intermediate1 == 0 => product == 0.
        for (a, b, c, d) in [
            (-1, 5, 8, 9),
            (-5, 7, 4, 2),
            (-9, -9, 1, 1),
            (2, 0, 4, 5),   // multiply by 0
            (0, 0, 0, 0),   // add(0,0) == 0
            (i32::MIN, 0, 0, 0),
        ] {
            let (cr, _) = capture_stdout(|| (p.c.buffapp)(a, b, c, d));
            let (rr, _) = capture_stdout(|| (p.rs.buffapp)(a, b, c, d));
            diff!(18, (a, b, c, d), cr, rr);
            let want = a.wrapping_add(b).wrapping_add(c).wrapping_add(d);
            assert_eq!(cr, want, "sum branch expected for ({a},{b},{c},{d})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — INT_MIN / -1 inside buffapp is unreachable. Proven by exhaustive
// reasoning (i1*i2 == -1 forces {i1,i2} == {1,-1} hence i1+i2 == 0) and
// searched for with randomized inputs: if it were reachable the process would
// die with SIGFPE and this test would fail.
// ---------------------------------------------------------------------------
fn row19_buffapp_never_divides_int_min_by_minus_one() {
    let p = pair();
    let mut rng = Rng::new(0x0019_0019);
    unsafe {
        for _ in 0..3000 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let c = rng.interesting_i32();
            let d = rng.interesting_i32();
            let (cr, _) = capture_stdout(|| (p.c.buffapp)(a, b, c, d));
            let (rr, _) = capture_stdout(|| (p.rs.buffapp)(a, b, c, d));
            diff!(19, (a, b, c, d), cr, rr);
        }
    }
}

fn main() {
    println!("running {} differential scenarios (sequential, harness-free)", 14);
    let started = std::time::Instant::now();
    print!("  {:<58} ", "row25_buffapp_all_residue_combinations");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    row25_buffapp_all_residue_combinations();
    println!("ok");
    print!("  {:<58} ", "row26_buffapp_intermediate3_zero_sum_path");
    std::io::stdout().flush().unwrap();
    row26_buffapp_intermediate3_zero_sum_path();
    println!("ok");
    print!("  {:<58} ", "row27_buffapp_intermediate3_nonzero_divide_path");
    std::io::stdout().flush().unwrap();
    row27_buffapp_intermediate3_nonzero_divide_path();
    println!("ok");
    print!("  {:<58} ", "row28_buffapp_randomized_full_range");
    std::io::stdout().flush().unwrap();
    row28_buffapp_randomized_full_range();
    println!("ok");
    print!("  {:<58} ", "row28b_buffapp_randomized_full_range_stdout");
    std::io::stdout().flush().unwrap();
    row28b_buffapp_randomized_full_range_stdout();
    println!("ok");
    print!("  {:<58} ", "row29_buffapp_corner_tuples");
    std::io::stdout().flush().unwrap();
    row29_buffapp_corner_tuples();
    println!("ok");
    print!("  {:<58} ", "row30_buffapp_stdout_exact_text");
    std::io::stdout().flush().unwrap();
    row30_buffapp_stdout_exact_text();
    println!("ok");
    print!("  {:<58} ", "row31_low_level_pipeline_composition");
    std::io::stdout().flush().unwrap();
    row31_low_level_pipeline_composition();
    println!("ok");
    print!("  {:<58} ", "row32_cross_library_abi_interchange");
    std::io::stdout().flush().unwrap();
    row32_cross_library_abi_interchange();
    println!("ok");
    print!("  {:<58} ", "extra_get_operation_name_pointer_identity");
    std::io::stdout().flush().unwrap();
    extra_get_operation_name_pointer_identity();
    println!("ok");
    print!("  {:<58} ", "extra_no_cross_call_state");
    std::io::stdout().flush().unwrap();
    extra_no_cross_call_state();
    println!("ok");
    print!("  {:<58} ", "extra_long_interleaved_append_run");
    std::io::stdout().flush().unwrap();
    extra_long_interleaved_append_run();
    println!("ok");
    print!("  {:<58} ", "row18_buffapp_zero_product_takes_sum_branch");
    std::io::stdout().flush().unwrap();
    row18_buffapp_zero_product_takes_sum_branch();
    println!("ok");
    print!("  {:<58} ", "row19_buffapp_never_divides_int_min_by_minus_one");
    std::io::stdout().flush().unwrap();
    row19_buffapp_never_divides_int_min_by_minus_one();
    println!("ok");
    println!("\nstdout_diff result: ok. {} scenarios passed in {:.2?}", 14, started.elapsed());
}

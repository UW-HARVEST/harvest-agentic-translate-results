//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`, named `eNN_*`. Every test constructs the
//! exact invalid input or boundary condition and asserts that BOTH `.so` files
//! reject/handle it identically — the same output bits, or (for the NULL-pointer
//! rows) the same terminating signal.

mod common;

use common::*;
use std::sync::OnceLock;

fn cached_regimes() -> &'static std::collections::HashMap<SqdRegime, Vec<[f32; 3]>> {
    static M: OnceLock<std::collections::HashMap<SqdRegime, Vec<[f32; 3]>>> = OnceLock::new();
    M.get_or_init(|| find_sqd_regimes(300, 0xC0DE_C0DE))
}

fn regime(r: SqdRegime) -> Vec<[f32; 3]> {
    cached_regimes().get(&r).cloned().unwrap_or_default()
}

/// Diff every triple in `ts` individually and as one batch.
fn diff_each(ctx: &str, ts: &[[f32; 3]]) {
    let p = pair();
    assert!(!ts.is_empty(), "{ctx}: no inputs");
    let flat: Vec<f32> = ts.iter().flat_map(|t| t.iter().copied()).collect();
    diff(ctx, &p, &flat, ts.len() as i32);
    for (i, t) in ts.iter().enumerate() {
        diff(&format!("{ctx} #{i}"), &p, t, 1);
    }
}

// ===========================================================================
// E1–E5 — the loop guard: `count <= 0` is the library's only input validation
// ===========================================================================

/// Assert that `count` produces a complete no-op: `dest` keeps every canary, and
/// both impls agree.
fn assert_noop(row: &str, count: i32, iters: usize) {
    let p = pair();
    let mut rng = Rng::new(0xE000_0000u64.wrapping_add(count as u32 as u64));
    for it in 0..iters {
        let dest_len = 16usize;
        let src: Vec<f32> = (0..24).map(|_| rng.any_bits_f32()).collect();

        let mut dc = canary_buf(dest_len);
        let mut dr = canary_buf(dest_len);
        unsafe {
            (p.c.tfm)(dc.as_mut_ptr(), src.as_ptr(), count);
            (p.rs.tfm)(dr.as_mut_ptr(), src.as_ptr(), count);
        }
        let ctx = format!("{row} count={count} it={it}");
        assert_bits_eq(&ctx, &dc, &dr);
        // The strong claim: not one float was stored.
        for i in 0..dest_len {
            assert_eq!(
                dc[i].to_bits(),
                canary_bits(i),
                "{ctx}: C WROTE dest[{i}] for count={count} (expected a no-op)"
            );
            assert_eq!(
                dr[i].to_bits(),
                canary_bits(i),
                "{ctx}: Rust WROTE dest[{i}] for count={count} (expected a no-op)"
            );
        }
    }
}

#[test]
fn e01_count_zero_is_a_noop() {
    assert_noop("E1", 0, 512);
}

#[test]
fn e02_count_minus_one_is_a_noop() {
    assert_noop("E2", -1, 512);
}

#[test]
fn e03_count_int_min_is_a_noop() {
    assert_noop("E3", i32::MIN, 512);
}

#[test]
fn e04_other_negative_counts_are_noops() {
    for c in [-2i32, -3, -1000, -65536, i32::MIN + 1, -0x7FFF_FFFF] {
        assert_noop("E4", c, 64);
    }
}

#[test]
fn e05_null_pointers_with_nonpositive_count_do_not_fault() {
    // The loop body never runs, so neither pointer is ever dereferenced. This
    // MUST NOT crash for either impl — a Rust translation that eagerly built a
    // slice from the raw pointers (e.g. `slice::from_raw_parts`) would be UB
    // here even though the C is fine.
    let p = pair();
    for count in [0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        unsafe {
            (p.c.tfm)(std::ptr::null_mut(), std::ptr::null(), count);
            (p.rs.tfm)(std::ptr::null_mut(), std::ptr::null(), count);
        }
        // Also: one pointer null, the other valid.
        let src = [1.0f32, 2.0, 3.0];
        let mut dest = [0.0f32; 2];
        unsafe {
            (p.c.tfm)(std::ptr::null_mut(), src.as_ptr(), count);
            (p.rs.tfm)(std::ptr::null_mut(), src.as_ptr(), count);
            (p.c.tfm)(dest.as_mut_ptr(), std::ptr::null(), count);
            (p.rs.tfm)(dest.as_mut_ptr(), std::ptr::null(), count);
        }
        assert_eq!(
            bits(&dest),
            vec![0u32, 0u32],
            "count={count}: dest must be untouched"
        );
    }
    // Dangling-but-nonnull pointers are equally untouched.
    let bogus = 0x1usize as *mut f32;
    let bogus_c = 0x1usize as *const f32;
    unsafe {
        (p.c.tfm)(bogus, bogus_c, 0);
        (p.rs.tfm)(bogus, bogus_c, 0);
        (p.c.tfm)(bogus, bogus_c, -5);
        (p.rs.tfm)(bogus, bogus_c, -5);
    }
}

// ===========================================================================
// E6 / E7 — NULL pointers with count > 0: both must fault the SAME way.
// Executed in a forked child (re-exec of this test binary) so the signal can be
// observed without killing the test run.
// ===========================================================================

const CRASH_IMPL: &str = "TFM_CRASH_IMPL";
const CRASH_WHICH: &str = "TFM_CRASH_WHICH";

/// The child half of E6/E7. Ignored by default; the parent re-execs this binary
/// with `--ignored --exact zz_null_pointer_crash_child`.
#[test]
#[ignore]
fn zz_null_pointer_crash_child() {
    let which = std::env::var(CRASH_WHICH).expect("TFM_CRASH_WHICH");
    let imp = std::env::var(CRASH_IMPL).expect("TFM_CRASH_IMPL");
    let p = pair();
    let f = if imp == "c" { p.c.tfm } else { p.rs.tfm };

    let src = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    let mut dest = [0.0f32; 4];
    unsafe {
        match which.as_str() {
            "dest" => f(std::ptr::null_mut(), src.as_ptr(), 2),
            "src" => f(dest.as_mut_ptr(), std::ptr::null(), 2),
            "both" => f(std::ptr::null_mut(), std::ptr::null(), 2),
            other => panic!("bad TFM_CRASH_WHICH={other}"),
        }
    }
    // If we get here the call did NOT fault; report that distinctly.
    eprintln!("NO_FAULT");
    std::process::exit(77);
}

/// `(signal, exit_code)` observed when the child runs `which` against `imp`.
fn run_crash_child(imp: &str, which: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--ignored", "--exact", "--test-threads=1", "zz_null_pointer_crash_child"])
        .env(CRASH_IMPL, imp)
        .env(CRASH_WHICH, which)
        // Propagate the .so overrides so the child tests the same objects.
        .envs(
            ["TFM_C_SO", "TFM_RUST_SO"]
                .iter()
                .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v))),
        )
        .output()
        .expect("spawn crash child");
    (out.status.signal(), out.status.code())
}

#[test]
fn e06_null_dest_with_positive_count_faults_identically() {
    let (cs, cc) = run_crash_child("c", "dest");
    let (rs, rc) = run_crash_child("rust", "dest");
    eprintln!("E6 dest=NULL: C signal={cs:?} code={cc:?} | Rust signal={rs:?} code={rc:?}");
    assert_eq!(
        cs, rs,
        "E6: C and Rust died from DIFFERENT signals (C={cs:?} code={cc:?}, Rust={rs:?} code={rc:?})"
    );
    assert_eq!(
        cs,
        Some(11),
        "E6: expected SIGSEGV(11) storing through NULL, got signal={cs:?} code={cc:?}"
    );
}

#[test]
fn e07_null_src_with_positive_count_faults_identically() {
    let (cs, cc) = run_crash_child("c", "src");
    let (rs, rc) = run_crash_child("rust", "src");
    eprintln!("E7 src=NULL: C signal={cs:?} code={cc:?} | Rust signal={rs:?} code={rc:?}");
    assert_eq!(
        cs, rs,
        "E7: C and Rust died from DIFFERENT signals (C={cs:?} code={cc:?}, Rust={rs:?} code={rc:?})"
    );
    assert_eq!(
        cs,
        Some(11),
        "E7: expected SIGSEGV(11) loading through NULL, got signal={cs:?} code={cc:?}"
    );

    // …and both pointers null.
    let (cs2, cc2) = run_crash_child("c", "both");
    let (rs2, rc2) = run_crash_child("rust", "both");
    eprintln!("E7 both=NULL: C signal={cs2:?} code={cc2:?} | Rust signal={rs2:?} code={rc2:?}");
    assert_eq!(cs2, rs2, "E7(both): signals differ (C={cs2:?}/{cc2:?}, Rust={rs2:?}/{rc2:?})");
}

// ===========================================================================
// E8–E12 — the relational test `src[0] < src[1]` rejecting into the `else` branch
// ===========================================================================

/// Assert both impls agree AND that the branch actually observed matches
/// `want_if` (deduced from the verbatim `dxy` copy).
fn assert_branch(ctx: &str, ts: &[[f32; 3]], want_if: bool) {
    let p = pair();
    assert!(!ts.is_empty(), "{ctx}: no inputs");
    let mut decided = 0usize;
    for (i, &t) in ts.iter().enumerate() {
        let dc = run_one(p.c.tfm, t);
        let dr = run_one(p.rs.tfm, t);
        assert_bits_eq(
            &format!("{ctx} #{i} src={}", fmt_slice(&t)),
            &dc,
            &dr,
        );
        if let Some(took_if) = observed_branch(t, dc) {
            decided += 1;
            assert_eq!(
                took_if,
                want_if,
                "{ctx} #{i}: C took the {} branch but the row requires the {} branch \
                 (src={}, dest=[{}, {}])",
                if took_if { "if" } else { "else" },
                if want_if { "if" } else { "else" },
                fmt_slice(&t),
                fmt_f32(dc[0]),
                fmt_f32(dc[1]),
            );
            let took_if_rs = observed_branch(t, dr).unwrap_or(took_if);
            assert_eq!(took_if_rs, took_if, "{ctx} #{i}: Rust took the other branch");
        }
    }
    assert!(
        decided > 0,
        "{ctx}: branch was indistinguishable for every input (test proves nothing)"
    );
    eprintln!("[{ctx}] branch confirmed for {}/{} inputs", decided, ts.len());
    diff_each(&format!("{ctx} (batched)"), ts);
}

#[test]
fn e08_equal_operands_take_else_branch() {
    let mut rng = Rng::new(0xE008);
    let mut ts = Vec::new();
    for _ in 0..3_000 {
        let a = rng.wild_normal();
        ts.push([a, a, rng.wild_normal()]);
    }
    // Exact-equality boundary cases, incl. the +0.0 == -0.0 pair.
    for &(a, b) in &[
        (0.0f32, -0.0f32),
        (-0.0f32, 0.0f32),
        (0.0f32, 0.0f32),
        (-0.0f32, -0.0f32),
        (1.0f32, 1.0f32),
        (-1.0f32, -1.0f32),
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY),
        (f32::MAX, f32::MAX),
        (f32::MIN_POSITIVE, f32::MIN_POSITIVE),
    ] {
        for &d in &[1.0f32, -1.0, 0.0, -0.0, 3.5, f32::INFINITY] {
            ts.push([a, b, d]);
        }
    }
    assert_branch("E8 src[0]==src[1]", &ts, false);
}

#[test]
fn e09_greater_operand_takes_else_branch() {
    let mut rng = Rng::new(0xE009);
    let mut ts = Vec::new();
    for _ in 0..3_000 {
        let a = rng.wild_normal();
        let b = rng.wild_normal();
        if a == b {
            continue;
        }
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        ts.push([hi, lo, rng.wild_normal()]);
    }
    // Adjacent-float boundary: hi is exactly one ULP above lo.
    for base in [0x3F80_0000u32, 0x0000_0000, 0x7F7F_FFFE, 0x0080_0000] {
        let lo = f32::from_bits(base);
        let hi = f32::from_bits(base + 1);
        ts.push([hi, lo, 1.0]);
    }
    assert_branch("E9 src[0]>src[1]", &ts, false);
}

#[test]
fn e10_nan_in_lane0_takes_else_branch() {
    let mut rng = Rng::new(0xE010);
    let mut ts = Vec::new();
    for _ in 0..3_000 {
        ts.push([rng.any_nan(), rng.wild_normal(), rng.wild_normal()]);
    }
    for &n in &[0x7FC0_0000u32, 0xFFC0_0000, 0x7FA0_0000, 0xFFA0_0000, 0x7F80_0001, 0xFFBF_FFFF] {
        for &b in &[1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
            ts.push([f32::from_bits(n), b, 2.5]);
        }
    }
    assert_branch("E10 src[0] is NaN", &ts, false);
}

#[test]
fn e11_nan_in_lane1_takes_else_branch() {
    let mut rng = Rng::new(0xE011);
    let mut ts = Vec::new();
    for _ in 0..3_000 {
        ts.push([rng.wild_normal(), rng.any_nan(), rng.wild_normal()]);
    }
    for &n in &[0x7FC0_0000u32, 0xFFC0_0000, 0x7FA0_0000, 0xFFA0_0000, 0x7F80_0001, 0xFFBF_FFFF] {
        for &a in &[1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
            ts.push([a, f32::from_bits(n), 2.5]);
        }
    }
    assert_branch("E11 src[1] is NaN", &ts, false);
}

#[test]
fn e12_nan_in_both_compare_lanes_takes_else_branch() {
    let mut rng = Rng::new(0xE012);
    let mut ts = Vec::new();
    for _ in 0..3_000 {
        ts.push([rng.any_nan(), rng.any_nan(), rng.wild_normal()]);
    }
    let nans = [0x7FC0_0000u32, 0xFFC0_0000, 0x7FA0_0000, 0xFFA0_0000, 0x7F80_0001];
    for &x in &nans {
        for &y in &nans {
            for &d in &[1.0f32, -0.0, f32::INFINITY] {
                ts.push([f32::from_bits(x), f32::from_bits(y), d]);
            }
        }
    }
    assert_branch("E12 both compare lanes NaN", &ts, false);
}

// ===========================================================================
// E13–E19 — the clamp and the IEEE invalid-operation paths
// ===========================================================================

#[test]
fn e13_negative_sqd_is_clamped_to_zero() {
    let ts = regime(SqdRegime::Negative);
    assert!(
        ts.len() >= 50,
        "E13: only {} negative-sqd inputs found; the clamp branch would be \
         under-tested",
        ts.len()
    );
    eprintln!("E13: {} triples with sqd < 0 (clamp taken)", ts.len());
    // Both branches of the `if` must be represented.
    let n_if = ts.iter().filter(|t| roles(**t).3).count();
    assert!(n_if > 0 && n_if < ts.len(), "E13: only one C branch represented");
    diff_each("E13 sqd<0 clamped", &ts);

    // The clamp means `sqrtf` gets exactly +0.0f, so lambda == 0.5*(dy2+dx2).
    // Verify no NaN sneaks out (the un-clamped `sqrtf(negative)` would give one).
    let p = pair();
    for &t in &ts {
        let d = run_one(p.c.tfm, t);
        let (dx2, dy2, _, _) = roles(t);
        if dx2.is_finite() && dy2.is_finite() {
            assert!(
                !d[0].is_nan() || !d[1].is_nan(),
                "E13: clamp failed, C produced all-NaN for finite input {}",
                fmt_slice(&t)
            );
        }
    }
}

#[test]
fn e14_negative_zero_and_the_clamp_boundary() {
    // (a) The `-0.0f` *discriminant* is unreachable: `sqd = dxy_term + acc`,
    //     `dxy_term = (4*dxy)*dxy` is a square (never -0.0), and `acc` can only
    //     be -0.0 if `dx2*dx2` were, which is likewise impossible. IEEE
    //     round-to-nearest only yields -0.0 when both addends are -0.0.
    //     Confirm the search agrees, so ERRORS.md E14 is verified not assumed.
    let neg_zero = regime(SqdRegime::NegZero);
    eprintln!("E14: sqd == -0.0 reachable? {} hits", neg_zero.len());
    if !neg_zero.is_empty() {
        diff_each("E14 sqd==-0.0", &neg_zero);
    }

    // (b) The reachable neighbours of the clamp boundary must still agree: the
    //     largest negative sqd (clamped) and exactly +0.0 (not clamped).
    let pos_zero = regime(SqdRegime::PosZero);
    assert!(!pos_zero.is_empty(), "E14: no sqd == +0.0 inputs found");
    diff_each("E14 sqd==+0.0 (clamp NOT taken)", &pos_zero);

    // (c) -0.0 fed directly through every lane, so signed-zero propagation
    //     through `dy2 + dx2`, `dx2 - lambda` and the verbatim `dxy` copy is
    //     pinned. All 3^3 combinations of {-0.0, +0.0, 1.0}.
    let vals = [-0.0f32, 0.0f32, 1.0f32, -1.0f32];
    let mut ts = Vec::new();
    for &x in &vals {
        for &y in &vals {
            for &z in &vals {
                ts.push([x, y, z]);
            }
        }
    }
    diff_each("E14 signed zeros through every lane", &ts);
}

#[test]
fn e15_nan_sqd_propagates_through_sqrt() {
    let ts = regime(SqdRegime::Nan);
    assert!(ts.len() >= 50, "E15: only {} NaN-sqd inputs found", ts.len());
    eprintln!("E15: {} triples with sqd == NaN", ts.len());
    diff_each("E15 sqd==NaN", &ts);

    // The clamp `0 > NaN` is FALSE, so the NaN reaches sqrtf and must come back
    // out. Confirm at least one *finite-input* case where the output is NaN —
    // that is only possible if the NaN survived the clamp and the sqrt.
    let p = pair();
    let finite_nan: Vec<[f32; 3]> = ts
        .iter()
        .copied()
        .filter(|t| t.iter().all(|x| x.is_finite()))
        .collect();
    assert!(
        !finite_nan.is_empty(),
        "E15: no finite-input NaN-sqd case (invalid-operation path untested)"
    );
    for &t in &finite_nan {
        let d = run_one(p.c.tfm, t);
        let dr = run_one(p.rs.tfm, t);
        assert!(
            d[0].is_nan() || d[1].is_nan(),
            "E15: NaN sqd did not reach the output for finite input {}",
            fmt_slice(&t)
        );
        assert_eq!(bits(&d), bits(&dr), "E15: divergence on {}", fmt_slice(&t));
    }
    eprintln!("E15: {} finite-input invalid-operation cases", finite_nan.len());
}

#[test]
fn e16_infinite_sqd() {
    let ts = regime(SqdRegime::PosInf);
    assert!(ts.len() >= 20, "E16: only {} +inf-sqd inputs found", ts.len());
    eprintln!("E16: {} triples with sqd == +inf", ts.len());
    diff_each("E16 sqd==+inf", &ts);
}

#[test]
fn e17_inf_minus_inf_invalid_operation() {
    // dy2*dy2 -> +inf AND 2*dx2*dy2 -> +inf, with no NaN in the input.
    let mut rng = Rng::new(0xE017);
    let mut ts = Vec::new();
    for _ in 0..40_000 {
        let a = rng.huge();
        let b = rng.huge();
        for t in [[a, b, rng.signed_unit()], [b, a, rng.signed_unit()]] {
            if t.iter().all(|x| x.is_finite()) && sqd_for_triple(t).is_nan() {
                ts.push(t);
            }
        }
        if ts.len() > 2_000 {
            break;
        }
    }
    for &a in &[f32::MAX, -f32::MAX, 1e30f32, -1e30f32, 1e20f32, -1e20f32] {
        for &b in &[f32::MAX, -f32::MAX, 1e30f32, -1e30f32, 1e20f32, -1e20f32] {
            for &z in &[0.0f32, -0.0, 1.0, -1.0, 1e-30] {
                let t = [a, b, z];
                if sqd_for_triple(t).is_nan() {
                    ts.push(t);
                }
            }
        }
    }
    assert!(!ts.is_empty(), "E17: could not construct inf-inf");
    eprintln!("E17: {} inf-inf triples (all-finite inputs)", ts.len());
    diff_each("E17 inf-inf", &ts);
}

#[test]
fn e18_zero_times_inf_invalid_operation() {
    // 2.0f*dx2*dy2 with one factor ±0 and the other ±inf.
    let mut ts = Vec::new();
    for &z in &[0.0f32, -0.0f32] {
        for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
            for &d in &[
                0.0f32,
                -0.0f32,
                1.0,
                -1.0,
                f32::MIN_POSITIVE,
                f32::from_bits(0x0000_0001),
                1e30,
                f32::MAX,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ] {
                ts.push([z, inf, d]);
                ts.push([inf, z, d]);
            }
        }
    }
    let invalid: Vec<[f32; 3]> = ts
        .iter()
        .copied()
        .filter(|t| t.iter().all(|x| !x.is_nan()) && sqd_for_triple(*t).is_nan())
        .collect();
    assert!(!invalid.is_empty(), "E18: could not construct 0*inf");
    eprintln!("E18: {} of {} triples hit 0*inf -> NaN", invalid.len(), ts.len());
    diff_each("E18 0*inf family", &ts);
}

#[test]
fn e19_inf_plus_neg_inf_in_lambda_sum() {
    // `dy2 + dx2` with dy2 = +inf, dx2 = -inf (and the mirror), so the lambda
    // sum itself is an invalid operation.
    let mut ts = Vec::new();
    for &d in &[
        0.0f32,
        -0.0f32,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FA0_0000),
    ] {
        // src=[-inf, +inf] -> if branch: dx2=-inf, dy2=+inf
        ts.push([f32::NEG_INFINITY, f32::INFINITY, d]);
        // src=[+inf, -inf] -> else branch: dy2=+inf, dx2=-inf
        ts.push([f32::INFINITY, f32::NEG_INFINITY, d]);
    }
    // Confirm the intended condition really arises for the finite-dxy cases.
    let hit = ts
        .iter()
        .filter(|t| {
            let (dx2, dy2, _, _) = roles(**t);
            (dy2 + dx2).is_nan() && !dx2.is_nan() && !dy2.is_nan()
        })
        .count();
    assert!(hit > 0, "E19: inf + (-inf) never arose");
    eprintln!("E19: {hit}/{} triples make `dy2 + dx2` an invalid op", ts.len());
    diff_each("E19 inf+(-inf) in lambda", &ts);
}

// ===========================================================================
// E20–E22 — subnormals and NaN payload rules
// ===========================================================================

#[test]
fn e20_subnormals_are_not_flushed_to_zero() {
    let p = pair();
    let mut rng = Rng::new(0xE020);

    // Exhaustive-ish sweep of the extreme subnormal magnitudes, both signs.
    let mut mags: Vec<u32> = (0u32..256).collect();
    mags.extend((0x007F_FF00u32..=0x007F_FFFF).collect::<Vec<_>>());
    mags.push(0x0080_0000); // FLT_MIN (smallest normal)
    mags.push(0x0080_0001);
    let vals: Vec<f32> = mags
        .iter()
        .flat_map(|&m| [f32::from_bits(m), f32::from_bits(0x8000_0000 | m)])
        .collect();

    let mut ts = Vec::new();
    for (i, &a) in vals.iter().enumerate() {
        for &b in vals.iter().skip(i % 7).step_by(23) {
            ts.push([a, b, vals[(i * 13) % vals.len()]]);
        }
    }
    eprintln!("E20: {} subnormal triples", ts.len());
    let flat: Vec<f32> = ts.iter().flat_map(|t| t.iter().copied()).collect();
    diff("E20 subnormal sweep", &p, &flat, ts.len() as i32);

    // Randomized subnormals, mixed with normals so the underflow happens inside
    // the arithmetic rather than only at the inputs.
    for it in 0..200 {
        let n = 2048usize;
        let src: Vec<f32> = (0..3 * n)
            .map(|i| {
                if i % 3 == 2 {
                    rng.subnormal()
                } else if i % 5 == 0 {
                    rng.wild_normal()
                } else {
                    rng.subnormal()
                }
            })
            .collect();
        diff(&format!("E20 subnormal mix it={it}"), &p, &src, n as i32);
    }

    // And an explicit assertion that a subnormal RESULT is preserved, i.e. FTZ
    // is off in both objects.
    let mut any_subnormal_out = false;
    for it in 0..20_000 {
        let a = rng.subnormal();
        let t = [a, a, f32::from_bits(0x0000_0001)];
        let d = run_one(p.c.tfm, t);
        let dr = run_one(p.rs.tfm, t);
        assert_eq!(bits(&d), bits(&dr), "E20 it={it} divergence on {}", fmt_slice(&t));
        for v in d {
            if v != 0.0 && v.is_finite() && v.abs() < f32::MIN_POSITIVE {
                any_subnormal_out = true;
            }
        }
    }
    assert!(
        any_subnormal_out,
        "E20: never observed a subnormal OUTPUT — FTZ may be active, or the \
         generator never produced one (test would prove nothing)"
    );
}

#[test]
fn e21_signalling_nans_do_not_trap_and_quiet_consistently() {
    let p = pair();
    let snans = [0x7FA0_0000u32, 0xFFA0_0000, 0x7F80_0001, 0xFF80_0001, 0x7FBF_FFFF];
    let others = [
        0.0f32,
        -0.0f32,
        1.0,
        -1.0,
        2.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
    ];
    let mut ts = Vec::new();
    for &s in &snans {
        let sn = f32::from_bits(s);
        for &o in &others {
            for &q in &others {
                ts.push([sn, o, q]);
                ts.push([o, sn, q]);
                ts.push([o, q, sn]);
            }
        }
        for &s2 in &snans {
            let sn2 = f32::from_bits(s2);
            ts.push([sn, sn2, 1.0]);
            ts.push([sn, 1.0, sn2]);
            ts.push([1.0, sn, sn2]);
            ts.push([sn, sn2, sn]);
        }
    }
    eprintln!("E21: {} sNaN triples", ts.len());
    diff_each("E21 sNaN", &ts);

    // The verbatim-copy claim: whichever slot receives `dxy` gets src[2]'s bits
    // UNCHANGED (a plain movss, not an FP op), so an sNaN in lane 2 is NOT
    // quieted on that path. Both impls must agree on this.
    let mut verbatim = 0usize;
    for &s in &snans {
        let sn = f32::from_bits(s);
        for &o in &others {
            for &q in &others {
                let t = [o, q, sn];
                let dc = run_one(p.c.tfm, t);
                let dr = run_one(p.rs.tfm, t);
                assert_eq!(bits(&dc), bits(&dr), "E21: divergence on {}", fmt_slice(&t));
                let took_if = o < q;
                let slot = if took_if { dc[1] } else { dc[0] };
                assert_eq!(
                    slot.to_bits(),
                    s,
                    "E21: dxy was NOT copied verbatim (src[2]={:#010x}, got {:#010x}) for {}",
                    s,
                    slot.to_bits(),
                    fmt_slice(&t)
                );
                verbatim += 1;
            }
        }
    }
    eprintln!("E21: verbatim sNaN copy confirmed on {verbatim} cases");
}

#[test]
fn e22_noncanonical_nan_payloads_are_preserved_identically() {
    let p = pair();
    let mut rng = Rng::new(0xE022);

    // Hand-picked exotic payloads, including the minimal-payload NaNs that sit
    // one bit away from infinity.
    let exotic = [
        0x7F80_0001u32,
        0xFF80_0001,
        0x7FBF_FFFF,
        0xFFBF_FFFF,
        0x7FC0_0001,
        0xFFC0_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x7FD5_5555,
        0xFFAA_AAAA,
    ];
    let mut ts = Vec::new();
    for &x in &exotic {
        for &y in &exotic {
            for &z in &exotic {
                ts.push([f32::from_bits(x), f32::from_bits(y), f32::from_bits(z)]);
            }
        }
    }
    // Mixed with non-NaN partners so single-NaN propagation is covered too.
    for &x in &exotic {
        for &o in &[1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
            ts.push([f32::from_bits(x), o, o]);
            ts.push([o, f32::from_bits(x), o]);
            ts.push([o, o, f32::from_bits(x)]);
        }
    }
    eprintln!("E22: {} exotic-payload triples", ts.len());
    diff_each("E22 exotic NaN payloads", &ts);

    // Randomized payloads, in bulk.
    for it in 0..200 {
        let n = 2048usize;
        let src: Vec<f32> = (0..3 * n)
            .map(|i| if i % 4 == 3 { rng.wild_normal() } else { rng.any_nan() })
            .collect();
        diff(&format!("E22 random payloads it={it}"), &p, &src, n as i32);
    }
}

// ===========================================================================
// E23 — out-of-range "enum" values across FFI
// ===========================================================================

#[test]
fn e23_no_enum_parameter_but_full_int_range_of_count() {
    // The public API declares NO enum, so there is no invalid-variant input to
    // construct (ERRORS.md E23). The only non-pointer parameter is `int count`;
    // sweep its representable extremes and the values around the sole boundary
    // (`count <= 0` vs `count > 0`) to cover the same class of bug.
    let p = pair();
    let mut rng = Rng::new(0xE023);

    // Non-positive: must all be no-ops (verified against canaries).
    for &count in &[
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0000,
        -70_000,
        -3,
        -2,
        -1,
        0,
    ] {
        let src: Vec<f32> = (0..24).map(|_| rng.any_bits_f32()).collect();
        let mut dc = canary_buf(16);
        let mut dr = canary_buf(16);
        unsafe {
            (p.c.tfm)(dc.as_mut_ptr(), src.as_ptr(), count);
            (p.rs.tfm)(dr.as_mut_ptr(), src.as_ptr(), count);
        }
        assert_bits_eq(&format!("E23 count={count}"), &dc, &dr);
        for i in 0..16 {
            assert_eq!(dc[i].to_bits(), canary_bits(i), "E23 C wrote for count={count}");
            assert_eq!(dr[i].to_bits(), canary_bits(i), "E23 Rust wrote for count={count}");
        }
    }

    // One step past the boundary in the positive direction, with buffers sized
    // exactly for it (so it is defined behaviour, unlike count = INT_MAX).
    for &count in &[1i32, 2, 3] {
        for it in 0..2_000 {
            let src: Vec<f32> = (0..3 * count as usize).map(|_| rng.any_bits_f32()).collect();
            diff(&format!("E23 count={count} it={it}"), &p, &src, count);
        }
    }
}

// ===========================================================================
// E24–E26 — length and pointer boundaries
// ===========================================================================

#[test]
fn e24_oversized_count_relative_to_logical_data() {
    let p = pair();
    let mut rng = Rng::new(0xE024);
    for it in 0..1_000 {
        let logical = 1 + rng.below(48) as usize;
        let over = 1 + rng.below(48) as usize;
        let total = logical + over;
        // Fully allocated (so no OOB), but the tail is data the caller never
        // meant to process. The C has no bounds check; Rust must not add one.
        let src: Vec<f32> = (0..3 * total)
            .map(|i| {
                if i < 3 * logical {
                    rng.signed_unit()
                } else {
                    rng.any_bits_f32()
                }
            })
            .collect();
        diff(
            &format!("E24 it={it} logical={logical} count={total}"),
            &p,
            &src,
            total as i32,
        );
    }
}

#[test]
fn e25_count_one_touches_exactly_three_in_and_two_out() {
    let p = pair();
    let mut rng = Rng::new(0xE025);
    for it in 0..5_000 {
        let t = [rng.any_bits_f32(), rng.any_bits_f32(), rng.any_bits_f32()];

        // (a) dest must receive exactly 2 stores: guards on both sides are
        //     checked by `diff_disjoint`.
        diff_disjoint(&format!("E25 dest bounds it={it}"), &p, &t, 1, 2);

        // (b) src[3..] must NOT be read. Prove it by running twice with DIFFERENT
        //     trailing bytes and requiring identical output, for each impl.
        for extra in [0usize, 1, 2, 3, 7] {
            let mut s1 = vec![t[0], t[1], t[2]];
            let mut s2 = s1.clone();
            for k in 0..extra {
                s1.push(f32::from_bits(0xAAAA_0000 | k as u32));
                s2.push(f32::from_bits(0x5555_0000 | k as u32));
            }
            for (name, f) in [("C", p.c.tfm), ("Rust", p.rs.tfm)] {
                let mut d1 = canary_buf(2);
                let mut d2 = canary_buf(2);
                unsafe {
                    f(d1.as_mut_ptr(), s1.as_ptr(), 1);
                    f(d2.as_mut_ptr(), s2.as_ptr(), 1);
                }
                assert_eq!(
                    bits(&d1),
                    bits(&d2),
                    "E25: {name} output depends on src[3..] (extra={extra}) for src={}",
                    fmt_slice(&t)
                );
            }
        }
    }
}

#[test]
fn e26_float_aligned_but_unaligned_for_vectors() {
    let p = pair();
    let mut rng = Rng::new(0xE026);
    for src_off in 0..8usize {
        for dest_off in 0..8usize {
            for it in 0..80 {
                let count = 1 + rng.below(24) as i32;
                let src_data: Vec<f32> = (0..3 * count as usize)
                    .map(|_| match rng.below(4) {
                        0 => rng.any_bits_f32(),
                        1 => rng.wild_normal(),
                        2 => rng.huge(),
                        _ => rng.any_nan(),
                    })
                    .collect();
                diff_offsets(
                    &format!("E26 it={it}"),
                    &p,
                    &src_data,
                    src_off,
                    dest_off,
                    count,
                );
            }
        }
    }
}

// ===========================================================================
// Reachability lemma backing E15/E21.
//
// Mutation testing showed that removing the `quiet(x)` from `fsqrt`'s NaN branch
// in src/lib.rs is NOT detectable by any test here. That is not a coverage gap:
// it is because `sqrtf`'s argument can never be a *signalling* NaN, so `quiet()`
// there is defensive code for an unreachable state. Rather than leave that as an
// unverified assumption, prove it — if the lemma ever breaks, this test fails and
// the `quiet()` becomes load-bearing.
// ===========================================================================

#[test]
fn sqrt_argument_is_never_a_signalling_nan() {
    // Structural argument: `sqd = dxy_term + acc`, and every value feeding it is
    // the result of an SSE arithmetic op, all of which quiet their NaN output.
    // A NaN `sqd` therefore always has the mantissa MSB set. The clamp
    // `(0 > sqd) ? 0 : sqd` passes NaN through unchanged, so `sqrtf` only ever
    // sees a quiet NaN (or a non-negative number).
    fn is_signalling(x: f32) -> bool {
        x.is_nan() && (x.to_bits() & 0x0040_0000) == 0
    }

    let mut checked = 0usize;
    let mut nan_seen = 0usize;

    let check = |t: [f32; 3], checked: &mut usize, nan_seen: &mut usize| {
        let sqd = sqd_for_triple(t);
        // `clamp_nonneg_c` passes NaN and non-negative values through unchanged.
        let arg = if 0.0f32 > sqd { 0.0f32 } else { sqd };
        *checked += 1;
        if arg.is_nan() {
            *nan_seen += 1;
            assert!(
                !is_signalling(arg),
                "sqrtf would receive a SIGNALLING NaN ({:#010x}) for src={} — the \
                 `quiet()` in fsqrt IS load-bearing and needs its own differential \
                 test",
                arg.to_bits(),
                fmt_slice(&t)
            );
        }
        assert!(
            !(arg < 0.0f32),
            "clamp failed: sqrtf would receive {} for src={}",
            fmt_f32(arg),
            fmt_slice(&t)
        );
    };

    // Exhaustive 24³ special alphabet (contains sNaN inputs in every lane).
    let a = alphabet_f32();
    for &x in &a {
        for &y in &a {
            for &z in &a {
                check([x, y, z], &mut checked, &mut nan_seen);
            }
        }
    }
    // Plus heavy randomization, biased towards sNaN inputs.
    let mut rng = Rng::new(0x5_0A_0A_0A);
    for _ in 0..500_000 {
        let lane = |r: &mut Rng| match r.below(6) {
            0 => f32::from_bits(0x7FA0_0000),
            1 => f32::from_bits(0xFFA0_0000),
            2 => f32::from_bits(0x7F80_0000 | (r.next_u32() & 0x003F_FFFF).max(1)),
            3 => r.huge(),
            4 => r.any_bits_f32(),
            _ => r.wild_normal(),
        };
        check(
            [lane(&mut rng), lane(&mut rng), lane(&mut rng)],
            &mut checked,
            &mut nan_seen,
        );
    }

    assert!(
        nan_seen > 1000,
        "only {nan_seen} NaN sqrtf arguments seen out of {checked}; the lemma \
         would be vacuous"
    );
    eprintln!(
        "sqrt-argument lemma: {checked} triples checked, {nan_seen} NaN arguments, \
         0 signalling — `quiet()` in fsqrt is provably unreachable (defensive only)"
    );
}

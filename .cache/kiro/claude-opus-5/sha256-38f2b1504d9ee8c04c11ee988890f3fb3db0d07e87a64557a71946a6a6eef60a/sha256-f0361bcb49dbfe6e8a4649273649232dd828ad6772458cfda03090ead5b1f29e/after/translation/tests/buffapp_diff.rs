// Phase B — CONFIGS.md rows 27-33: `buffapp` return value AND stdout bytes.
//
// `buffapp` is the only entry point that writes to stdout, so fd 1 is captured
// around every call and compared byte-for-byte.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Model used ONLY to decide which inputs would raise SIGFPE (INT_MIN / -1).
// Those inputs are excluded here and covered out-of-process in
// tests/error_paths.rs (ERRORS.md rows 14 and 17).
// ---------------------------------------------------------------------------
fn model_op_name(code: i32) -> &'static str {
    match code {
        0 => "add",
        1 => "subtract",
        2 => "multiply",
        3 => "divide",
        _ => "unknown",
    }
}

fn model_perform(a: i32, b: i32, op: &str) -> Option<i32> {
    match op {
        "add" => Some(a.wrapping_add(b)),
        "subtract" => Some(a.wrapping_sub(b)),
        "multiply" => Some(a.wrapping_mul(b)),
        "divide" => {
            if b == 0 {
                Some(0)
            } else if a == i32::MIN && b == -1 {
                None
            } else {
                Some(a.wrapping_div(b))
            }
        }
        _ => Some(0),
    }
}

/// True when `buffapp(p1..p4)` would execute `INT_MIN / -1` and therefore trap.
pub fn would_trap(p1: i32, p2: i32, p3: i32, p4: i32) -> bool {
    let op1 = model_op_name(p1.wrapping_rem(4));
    let i1 = match model_perform(p1, p2, op1) {
        None => return true,
        Some(v) => v,
    };
    let op2 = model_op_name(p3.wrapping_rem(4));
    let i2 = match model_perform(p3, p4, op2) {
        None => return true,
        Some(v) => v,
    };
    let result = i1.wrapping_add(i2);
    let i3 = i1.wrapping_mul(i2);
    i3 != 0 && result == i32::MIN && i3 == -1
}

// ---------------------------------------------------------------------------
// The differential check itself
// ---------------------------------------------------------------------------
fn buffapp_diff(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (cv, cout) = capture_stdout(|| unsafe { (c().buffapp)(p1, p2, p3, p4) });
    let (rv, rout) = capture_stdout(|| unsafe { (rust().buffapp)(p1, p2, p3, p4) });
    assert_eq!(
        rv, cv,
        "buffapp({p1}, {p2}, {p3}, {p4}) return: rust={rv} c={cv}"
    );
    assert_eq!(
        rout,
        cout,
        "buffapp({p1}, {p2}, {p3}, {p4}) stdout differs\n  c   = {:?}\n  rust= {:?}",
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout)
    );
    assert!(!cout.is_empty(), "capture harness produced no bytes");
}

// ---------------------------------------------------------------------------
// row 27: param1 % 4 == 0 x param3 % 4 == 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_27_buffapp_add_add() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 27);
    let mut done = 0;
    while done < 300 {
        let p1 = rng.next_small_i32() & !3; // p1 % 4 == 0
        let p3 = rng.next_small_i32() & !3;
        let p2 = rng.next_small_i32();
        let p4 = rng.next_small_i32();
        if would_trap(p1, p2, p3, p4) {
            continue;
        }
        buffapp_diff(p1, p2, p3, p4);
        done += 1;
    }
}

// ---------------------------------------------------------------------------
// row 28: full 7x7 cross-product of the reachable residues of `% 4`
// (C's `%` keeps the dividend's sign, so -1, -2, -3 are reachable and select
// the `default` "unknown" arm.)
// ---------------------------------------------------------------------------
const RESIDUES: [i32; 7] = [0, 1, 2, 3, -1, -2, -3];

/// Smallest-magnitude representative plus randomized values with the requested
/// residue mod 4.
fn with_residue(rng: &mut Rng, r: i32) -> i32 {
    // pick k so that (4k + r) % 4 == r; keep the sign of r so the residue is
    // preserved by C's truncating %.
    let k = (rng.next_u64() % 1000) as i32;
    if r >= 0 {
        4 * k + r
    } else {
        -4 * k + r
    }
}

#[test]
fn cfg_28_buffapp_residue_cross_product() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 28);
    for &r1 in &RESIDUES {
        for &r3 in &RESIDUES {
            let mut done = 0;
            let mut attempts = 0;
            while done < 40 && attempts < 4000 {
                attempts += 1;
                let p1 = with_residue(&mut rng, r1);
                let p3 = with_residue(&mut rng, r3);
                debug_assert_eq!(p1 % 4, r1);
                debug_assert_eq!(p3 % 4, r3);
                let p2 = rng.next_small_i32();
                let p4 = rng.next_small_i32();
                if would_trap(p1, p2, p3, p4) {
                    continue;
                }
                buffapp_diff(p1, p2, p3, p4);
                done += 1;
            }
            assert!(done >= 40, "residue cell ({r1},{r3}) under-tested: {done}");
        }
    }
}

// ---------------------------------------------------------------------------
// row 29: intermediate3 == 0 -> result = p1+p2+p3+p4
// ---------------------------------------------------------------------------
#[test]
fn cfg_29_buffapp_intermediate3_zero_branch() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 29);
    let mut done = 0;
    let mut attempts = 0;
    while done < 400 && attempts < 200_000 {
        attempts += 1;
        let p1 = rng.next_small_i32();
        let p2 = rng.next_small_i32();
        let p3 = rng.next_small_i32();
        let p4 = rng.next_small_i32();
        if would_trap(p1, p2, p3, p4) {
            continue;
        }
        // recompute intermediate3 with the model to select the branch
        let op1 = model_op_name(p1.wrapping_rem(4));
        let op2 = model_op_name(p3.wrapping_rem(4));
        let i1 = model_perform(p1, p2, op1).unwrap();
        let i2 = model_perform(p3, p4, op2).unwrap();
        if i1.wrapping_mul(i2) != 0 {
            continue;
        }
        buffapp_diff(p1, p2, p3, p4);
        done += 1;
    }
    assert!(done >= 400, "only {done} intermediate3==0 cases found");

    // Deterministic hits: residue -1/-2/-3 -> "unknown" -> perform returns 0.
    for &r1 in &[-1i32, -2, -3] {
        for &r3 in &[-1i32, -2, -3] {
            buffapp_diff(r1, 7, r3, 11);
        }
    }
    // divide with b == 0 also yields 0.
    buffapp_diff(3, 0, 3, 0);
    buffapp_diff(3, 0, 4, 5);
}

// ---------------------------------------------------------------------------
// row 30: intermediate3 != 0 -> result / intermediate3
// ---------------------------------------------------------------------------
#[test]
fn cfg_30_buffapp_intermediate3_nonzero_branch() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 30);
    let mut done = 0;
    let mut attempts = 0;
    while done < 600 && attempts < 200_000 {
        attempts += 1;
        let p1 = rng.next_small_i32();
        let p2 = rng.next_small_i32();
        let p3 = rng.next_small_i32();
        let p4 = rng.next_small_i32();
        if would_trap(p1, p2, p3, p4) {
            continue;
        }
        let op1 = model_op_name(p1.wrapping_rem(4));
        let op2 = model_op_name(p3.wrapping_rem(4));
        let i1 = model_perform(p1, p2, op1).unwrap();
        let i2 = model_perform(p3, p4, op2).unwrap();
        if i1.wrapping_mul(i2) == 0 {
            continue;
        }
        buffapp_diff(p1, p2, p3, p4);
        done += 1;
    }
    assert!(done >= 600, "only {done} intermediate3!=0 cases found");
}

// ---------------------------------------------------------------------------
// row 31: 1000 fully randomized int quadruples
// ---------------------------------------------------------------------------
#[test]
fn cfg_31_buffapp_fully_randomized() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 31);
    let mut done = 0;
    let mut skipped = 0;
    while done < 1000 {
        let p1 = rng.next_i32();
        let p2 = rng.next_i32();
        let p3 = rng.next_i32();
        let p4 = rng.next_i32();
        if would_trap(p1, p2, p3, p4) {
            skipped += 1;
            continue;
        }
        buffapp_diff(p1, p2, p3, p4);
        done += 1;
    }
    eprintln!("cfg_31: {done} compared, {skipped} SIGFPE inputs skipped");
}

// ---------------------------------------------------------------------------
// row 32: extreme-value cross product 5^4
// ---------------------------------------------------------------------------
#[test]
fn cfg_32_buffapp_extreme_cross_product() {
    let extremes = [0i32, 1, -1, i32::MAX, i32::MIN];
    let mut n = 0;
    let mut skipped = 0;
    for &p1 in &extremes {
        for &p2 in &extremes {
            for &p3 in &extremes {
                for &p4 in &extremes {
                    if would_trap(p1, p2, p3, p4) {
                        skipped += 1;
                        continue;
                    }
                    buffapp_diff(p1, p2, p3, p4);
                    n += 1;
                }
            }
        }
    }
    eprintln!("cfg_32: {n} compared, {skipped} SIGFPE inputs skipped");
    // A wider set of boundary values around the % 4 and overflow edges.
    let more = [
        2i32, -2, 3, -3, 4, -4, 5, -5, 7, -7, 8, -8,
        i32::MAX - 1, i32::MAX - 2, i32::MAX - 3, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3,
        65536, -65536, 46341, -46341, 46340, -46340,
    ];
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 32);
    for &p1 in &more {
        for &p3 in &more {
            let p2 = rng.next_small_i32();
            let p4 = rng.next_small_i32();
            if would_trap(p1, p2, p3, p4) {
                continue;
            }
            buffapp_diff(p1, p2, p3, p4);
        }
    }
}

// ---------------------------------------------------------------------------
// row 33 sanity: the capture harness really does observe the printf output,
// and the log text is what the C source formats.
// ---------------------------------------------------------------------------
#[test]
fn cfg_33_stdout_capture_is_meaningful() {
    let (cv, cout) = capture_stdout(|| unsafe { (c().buffapp)(4, 5, 6, 7) });
    let (rv, rout) = capture_stdout(|| unsafe { (rust().buffapp)(4, 5, 6, 7) });
    assert_eq!(rv, cv);
    assert_eq!(rout, cout);
    let text = String::from_utf8(cout).expect("log is ASCII");
    for needle in [
        "Computation Log:\n",
        "Starting computation with 4 parameters\n",
        "Operation 1: add(4, 5)\n",
        "Operation 2: multiply(6, 7)\n",
        "Operation 3: multiply(9, 42)\n",
        "Final result: ",
    ] {
        assert!(text.contains(needle), "captured stdout missing {needle:?}: {text:?}");
    }
    // 9 + 42 = 51; 9 * 42 = 378; 51 / 378 == 0
    assert_eq!(cv, 0);
    assert!(text.contains("Final result: 0\n"));
}

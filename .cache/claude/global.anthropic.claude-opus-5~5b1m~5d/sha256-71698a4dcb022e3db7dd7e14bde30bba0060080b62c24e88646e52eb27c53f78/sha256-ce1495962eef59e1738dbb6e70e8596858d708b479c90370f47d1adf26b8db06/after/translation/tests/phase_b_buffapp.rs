// Phase B -- valid-path differential tests for the public one-shot wrapper
// `buffapp` (the only symbol in include/lib.h). CONFIGS.md rows 25..40.
//
// `buffapp` has two observable outputs: the returned `int` and the byte stream
// it writes to stdout via `printf`. BOTH are diffed.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// A faithful model of the C control flow, used only to (a) classify which
// branch an input exercises and (b) detect the one input class that traps, so
// it can be routed to the subprocess comparison instead of killing the runner.
// ---------------------------------------------------------------------------
fn m_op_name(code: i32) -> &'static [u8] {
    match code {
        0 => b"add",
        1 => b"subtract",
        2 => b"multiply",
        3 => b"divide",
        _ => b"unknown",
    }
}

fn m_perform(a: i32, b: i32, op: &[u8]) -> i32 {
    if op == b"add" {
        a.wrapping_add(b)
    } else if op == b"subtract" {
        a.wrapping_sub(b)
    } else if op == b"multiply" {
        a.wrapping_mul(b)
    } else if op == b"divide" {
        if b != 0 {
            a.wrapping_div(b)
        } else {
            0
        }
    } else {
        0
    }
}

struct Model {
    i1: i32,
    i2: i32,
    i3: i32,
    /// `intermediate3 != 0` -- the divide branch was taken (CONFIGS A8).
    divide_branch: bool,
    /// `result / intermediate3` would be `INT_MIN / -1`: a hardware trap in C.
    traps: bool,
}

fn model(p1: i32, p2: i32, p3: i32, p4: i32) -> Model {
    let op1 = m_op_name(p1.wrapping_rem(4));
    let i1 = m_perform(p1, p2, op1);
    let op2 = m_op_name(p3.wrapping_rem(4));
    let i2 = m_perform(p3, p4, op2);
    let acc = i1.wrapping_add(i2);
    let i3 = m_perform(i1, i2, b"multiply");
    Model {
        i1,
        i2,
        i3,
        divide_branch: i3 != 0,
        traps: i3 != 0 && acc == i32::MIN && i3 == -1,
    }
}

/// Inputs whose final division traps are excluded here and covered by the
/// subprocess signal test in phase_c_errors.rs.
fn usable(p: (i32, i32, i32, i32)) -> bool {
    !model(p.0, p.1, p.2, p.3).traps
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `buffapp` once in each library, capturing stdout separately, and diff
/// the return value and the printed bytes.
fn diff_one(p: (i32, i32, i32, i32)) {
    let (cl, rl) = pair();
    let (p1, p2, p3, p4) = p;
    let (cv, cout) = capture_stdout(|| unsafe { (cl.buffapp)(p1, p2, p3, p4) });
    let (rv, rout) = capture_stdout(|| unsafe { (rl.buffapp)(p1, p2, p3, p4) });
    assert_eq!(cv, rv, "buffapp{p:?}: return C {cv} != Rust {rv}");
    assert_eq!(
        cout,
        rout,
        "buffapp{p:?}: stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout)
    );
}

/// Batched form: run the whole input list under one stdout capture per library
/// (far fewer syscalls), then diff. On any mismatch, bisect down to the single
/// offending input with `diff_one` so failures name a concrete quadruple.
fn diff_batch(label: &str, inputs: &[(i32, i32, i32, i32)]) {
    let (cl, rl) = pair();
    let inputs: Vec<_> = inputs.iter().copied().filter(|&p| usable(p)).collect();
    assert!(!inputs.is_empty(), "{label}: no usable inputs");

    let (cvals, cout) = capture_stdout(|| {
        inputs
            .iter()
            .map(|&(a, b, c, d)| unsafe { (cl.buffapp)(a, b, c, d) })
            .collect::<Vec<i32>>()
    });
    let (rvals, rout) = capture_stdout(|| {
        inputs
            .iter()
            .map(|&(a, b, c, d)| unsafe { (rl.buffapp)(a, b, c, d) })
            .collect::<Vec<i32>>()
    });

    if cvals != rvals || cout != rout {
        for &p in &inputs {
            diff_one(p);
        }
        panic!("{label}: batch mismatch that did not reproduce per-input (nondeterminism?)");
    }
    assert_eq!(cvals.len(), inputs.len());
}

// ===========================================================================
// CONFIGS.md rows 25-28 -- A6 x A7 diagonal (same operation on both halves)
// ===========================================================================

/// Build a `param` whose `% 4` equals `residue`, keeping the C truncating-%
/// sign rule in mind: positive params give residues 0..3, negative params give
/// residues 0,-1,-2,-3.
fn param_with_residue(rng: &mut Rng, residue: i32) -> i32 {
    loop {
        let k = rng.next_i32();
        let cand = if residue >= 0 {
            // positive value congruent to `residue`
            let base = (k as i64).abs() as i64 % ((i32::MAX as i64) / 4);
            (base * 4 + residue as i64) as i32
        } else {
            let base = (k as i64).abs() as i64 % ((i32::MAX as i64) / 4);
            (-(base * 4) + residue as i64) as i32
        };
        if cand.wrapping_rem(4) == residue {
            return cand;
        }
    }
}

fn diagonal_inputs(residue: i32, n: usize, seed_salt: u64) -> Vec<(i32, i32, i32, i32)> {
    let mut rng = Rng::with_seed(SEED ^ seed_salt);
    let mut v = Vec::new();
    for _ in 0..n {
        let p1 = param_with_residue(&mut rng, residue);
        let p3 = param_with_residue(&mut rng, residue);
        v.push((p1, rng.spicy_i32(), p3, rng.spicy_i32()));
    }
    // Zero divisors / zero operands explicitly.
    for &(b, d) in &[(0, 0), (0, 1), (1, 0), (-1, 0), (0, -1)] {
        let p1 = param_with_residue(&mut rng, residue);
        let p3 = param_with_residue(&mut rng, residue);
        v.push((p1, b, p3, d));
    }
    v
}

#[test]
fn row25_buffapp_add_add() {
    let ins = diagonal_inputs(0, 600, 0x25);
    for &p in &ins {
        if usable(p) {
            let m = model(p.0, p.1, p.2, p.3);
            let _ = m.i1;
        }
    }
    diff_batch("row25 add/add", &ins);
}

#[test]
fn row26_buffapp_subtract_subtract() {
    diff_batch("row26 sub/sub", &diagonal_inputs(1, 600, 0x26));
}

#[test]
fn row27_buffapp_multiply_multiply() {
    diff_batch("row27 mul/mul", &diagonal_inputs(2, 600, 0x27));
}

#[test]
fn row28_buffapp_divide_divide_incl_zero_divisor() {
    let mut ins = diagonal_inputs(3, 600, 0x28);
    // Explicit zero-divisor sub-case for both halves.
    let mut rng = Rng::with_seed(SEED ^ 0x28_0000);
    for _ in 0..200 {
        let p1 = param_with_residue(&mut rng, 3);
        let p3 = param_with_residue(&mut rng, 3);
        ins.push((p1, 0, p3, 0));
        ins.push((p1, 0, p3, rng.spicy_i32()));
        ins.push((p1, rng.spicy_i32(), p3, 0));
    }
    diff_batch("row28 div/div", &ins);
}

// ===========================================================================
// CONFIGS.md rows 29-33 -- the full A6 x A7 cross-product
// ===========================================================================

const RESIDUES_NONNEG: [i32; 4] = [0, 1, 2, 3];
const RESIDUES_NEG: [i32; 3] = [-1, -2, -3];

fn cross_inputs(r1s: &[i32], r3s: &[i32], per_cell: usize, salt: u64) -> Vec<(i32, i32, i32, i32)> {
    let mut rng = Rng::with_seed(SEED ^ salt);
    let mut v = Vec::new();
    for &r1 in r1s {
        for &r3 in r3s {
            for _ in 0..per_cell {
                let p1 = param_with_residue(&mut rng, r1);
                let p3 = param_with_residue(&mut rng, r3);
                v.push((p1, rng.spicy_i32(), p3, rng.spicy_i32()));
            }
        }
    }
    v
}

#[test]
fn row29_buffapp_cross_nonneg_16_combos() {
    diff_batch(
        "row29 4x4",
        &cross_inputs(&RESIDUES_NONNEG, &RESIDUES_NONNEG, 100, 0x29),
    );
}

#[test]
fn row30_buffapp_neg_residue_op1_unknown() {
    let ins = cross_inputs(&RESIDUES_NEG, &RESIDUES_NONNEG, 120, 0x30);
    // op1 must be "unknown" => intermediate1 == 0.
    for &(p1, p2, p3, p4) in &ins {
        assert_eq!(model(p1, p2, p3, p4).i1, 0, "row30: i1 should be 0");
    }
    diff_batch("row30 neg x nonneg", &ins);
}

#[test]
fn row31_buffapp_neg_residue_op2_unknown() {
    let ins = cross_inputs(&RESIDUES_NONNEG, &RESIDUES_NEG, 120, 0x31);
    for &(p1, p2, p3, p4) in &ins {
        assert_eq!(model(p1, p2, p3, p4).i2, 0, "row31: i2 should be 0");
    }
    diff_batch("row31 nonneg x neg", &ins);
}

#[test]
fn row32_buffapp_both_unknown_forces_fallback() {
    let ins = cross_inputs(&RESIDUES_NEG, &RESIDUES_NEG, 150, 0x32);
    for &(p1, p2, p3, p4) in &ins {
        let m = model(p1, p2, p3, p4);
        assert_eq!((m.i1, m.i2, m.i3), (0, 0, 0), "row32: all intermediates 0");
        assert!(!m.divide_branch, "row32 must take the fallback branch");
    }
    diff_batch("row32 neg x neg", &ins);
}

#[test]
fn row33_buffapp_all_49_residue_classes() {
    let all: Vec<i32> = vec![0, 1, 2, 3, -1, -2, -3];
    diff_batch("row33 7x7", &cross_inputs(&all, &all, 60, 0x33));
}

// ===========================================================================
// CONFIGS.md rows 34-36 -- A8, the intermediate3 branch
// ===========================================================================

#[test]
fn row34_buffapp_divide_branch() {
    let mut rng = Rng::with_seed(SEED ^ 0x34);
    let mut ins = Vec::new();
    let mut tries = 0;
    while ins.len() < 1500 && tries < 400_000 {
        tries += 1;
        let p = (
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
        let m = model(p.0, p.1, p.2, p.3);
        if m.divide_branch && !m.traps {
            ins.push(p);
        }
    }
    assert!(ins.len() > 500, "row34: only found {} inputs", ins.len());
    diff_batch("row34 divide branch", &ins);
}

#[test]
fn row35_buffapp_fallback_via_i1_zero() {
    let mut rng = Rng::with_seed(SEED ^ 0x35);
    let mut ins = Vec::new();
    let mut tries = 0;
    while ins.len() < 1000 && tries < 400_000 {
        tries += 1;
        let p = (
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
        let m = model(p.0, p.1, p.2, p.3);
        if m.i1 == 0 && !m.divide_branch {
            ins.push(p);
        }
    }
    assert!(ins.len() > 300, "row35: only found {} inputs", ins.len());
    diff_batch("row35 fallback via i1==0", &ins);
}

#[test]
fn row36_buffapp_fallback_via_i2_zero() {
    let mut rng = Rng::with_seed(SEED ^ 0x36);
    let mut ins = Vec::new();
    let mut tries = 0;
    while ins.len() < 1000 && tries < 400_000 {
        tries += 1;
        let p = (
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
        let m = model(p.0, p.1, p.2, p.3);
        if m.i2 == 0 && !m.divide_branch {
            ins.push(p);
        }
    }
    assert!(ins.len() > 300, "row36: only found {} inputs", ins.len());
    diff_batch("row36 fallback via i2==0", &ins);
}

// ===========================================================================
// CONFIGS.md rows 37-40 -- stdout bytes, widest sprintf, boundary sweep, bulk
// ===========================================================================

#[test]
fn row37_buffapp_stdout_bytes_per_call() {
    // Un-batched, one capture per call: proves the exact
    // "Computation Log:\n%s\n" stream matches for a call in isolation, and that
    // every one of the five appended log lines is byte-identical.
    let mut rng = Rng::with_seed(SEED ^ 0x37);
    let mut checked = 0;
    for r1 in [0, 1, 2, 3, -1, -2, -3] {
        for r3 in [0, 1, 2, 3, -1, -2, -3] {
            for _ in 0..3 {
                let p1 = param_with_residue(&mut rng, r1);
                let p3 = param_with_residue(&mut rng, r3);
                let p = (p1, rng.spicy_i32(), p3, rng.spicy_i32());
                if !usable(p) {
                    continue;
                }
                diff_one(p);
                checked += 1;
            }
        }
    }
    assert!(checked >= 100, "row37: only {checked} calls checked");

    // Also assert the C output has the shape the source implies, so a
    // both-empty-output false pass is impossible.
    let (cl, _) = pair();
    let (_, out) = capture_stdout(|| unsafe { (cl.buffapp)(4, 5, 6, 7) });
    let s = String::from_utf8_lossy(&out).to_string();
    assert!(s.starts_with("Computation Log:\n"), "unexpected header: {s:?}");
    assert!(s.contains("Starting computation with 4 parameters\n"), "{s:?}");
    assert!(s.contains("Operation 1: "), "{s:?}");
    assert!(s.contains("Operation 2: "), "{s:?}");
    assert!(s.contains("Operation 3: multiply("), "{s:?}");
    assert!(s.contains("Final result: "), "{s:?}");
}

#[test]
fn row38_buffapp_widest_sprintf_renderings() {
    // Longest possible %d renderings -> largest fill of the 64-byte `temp`.
    let extremes = [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1000000000];
    let mut ins = Vec::new();
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                for &d in &extremes {
                    ins.push((a, b, c, d));
                }
            }
        }
    }
    diff_batch("row38 extremes", &ins);
    // And per-call, so the printed bytes are compared in isolation too.
    for &p in ins.iter().take(60) {
        if usable(p) {
            diff_one(p);
        }
    }
}

#[test]
fn row39_buffapp_boundary_sweep() {
    let edges = [
        i32::MIN,
        i32::MIN + 1,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        i32::MAX - 1,
        i32::MAX,
    ];
    // Full 4-way sweep would be 13^4 = 28561 calls; that is cheap when batched.
    let mut ins = Vec::with_capacity(28_561);
    for &a in &edges {
        for &b in &edges {
            for &c in &edges {
                for &d in &edges {
                    ins.push((a, b, c, d));
                }
            }
        }
    }
    diff_batch("row39 boundary sweep", &ins);
}

#[test]
fn row40_buffapp_bulk_random() {
    let mut rng = Rng::with_seed(SEED ^ 0x40);
    let mut ins = Vec::with_capacity(4096);
    for _ in 0..4096 {
        ins.push((
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        ));
    }
    diff_batch("row40 bulk random", &ins);

    // A second, purely-uniform batch (no boundary bias) for good measure.
    let mut rng2 = Rng::with_seed(SEED ^ 0x4040);
    let mut ins2 = Vec::with_capacity(4096);
    for _ in 0..4096 {
        ins2.push((
            rng2.next_i32(),
            rng2.next_i32(),
            rng2.next_i32(),
            rng2.next_i32(),
        ));
    }
    diff_batch("row40 uniform random", &ins2);
}

// Phase B — CONFIGS.md rows 25-29: the full `checkshift` pipeline, plus the same
// pipeline reassembled by the test out of the low-level exports (including every
// C/Rust stage-assignment mask), which is where a divergence in the *composition*
// rather than in an individual function would show up.
//
// `harness = false`: these cases compare the exact bytes written to fd 1.

mod common;

use common::*;
use std::ffi::c_int;

fn main() {
    let mut t = Runner::new();
    t.case("cfg25_checkshift_random", cfg25_checkshift_random);
    t.case("cfg26_checkshift_boundary_grid", cfg26_checkshift_boundary_grid);
    t.case("cfg26b_checkshift_interesting_sampled", cfg26b_checkshift_interesting_sampled);
    t.case("cfg27_checkshift_stdout_verbatim", cfg27_checkshift_stdout_verbatim);
    t.case("cfg27b_checkshift_format_pinning", cfg27b_checkshift_format_pinning);
    t.case("cfg28_manual_pipeline_matches_checkshift", cfg28_manual_pipeline_matches_checkshift);
    t.case("cfg29_manual_pipeline_mixed_stages", cfg29_manual_pipeline_mixed_stages);
    t.case("cfg25b_checkshift_repeatable", cfg25b_checkshift_repeatable);
    t.finish();
}

type Quad = (c_int, c_int, c_int, c_int);

/// Run `checkshift` over `cases` in both libraries, comparing return values and
/// the complete emitted stdout byte stream.
fn diff_checkshift(cases: &[Quad], ctx: &str) {
    let (c, r) = both();
    let (c_res, c_out, r_res, r_out) = serial(|| {
        let (cv, co) = capture_stdout(|| {
            cases
                .iter()
                .map(|&(a, b, d, e)| unsafe { (c.checkshift)(a, b, d, e) })
                .collect::<Vec<c_int>>()
        });
        let (rv, ro) = capture_stdout(|| {
            cases
                .iter()
                .map(|&(a, b, d, e)| unsafe { (r.checkshift)(a, b, d, e) })
                .collect::<Vec<c_int>>()
        });
        (cv, co, rv, ro)
    });

    for (i, (&q, (cv, rv))) in cases.iter().zip(c_res.iter().zip(r_res.iter())).enumerate() {
        assert_eq!(cv, rv, "checkshift{q:?} [{ctx}, case {i}]: C={cv} Rust={rv}");
    }
    assert_same_output(&c_out, &r_out, ctx);
}

// --- row 25 -------------------------------------------------------------------
fn cfg25_checkshift_random() {
    let mut rng = Rng::new(0x2500_0025);
    let cases: Vec<Quad> = (0..20_000)
        .map(|_| {
            (
                rng.next_i32_biased(),
                rng.next_i32_biased(),
                rng.next_i32_biased(),
                rng.next_i32_biased(),
            )
        })
        .collect();
    diff_checkshift(&cases, "row 25 random");
}

/// The library keeps a lazily-filled `static` dispatch table and a heap-allocated
/// state per call; repeated invocations with the same inputs must be identical,
/// and must not drift after other inputs have been processed.
fn cfg25b_checkshift_repeatable() {
    let (c, r) = both();
    let probe: Quad = (-31337, 4919, i32::MIN, 0xABCD);
    let mut rng = Rng::new(0x25B0_0025);
    serial(|| {
        let (first, _) = capture_stdout(|| unsafe {
            ((c.checkshift)(probe.0, probe.1, probe.2, probe.3), (r.checkshift)(probe.0, probe.1, probe.2, probe.3))
        });
        assert_eq!(first.0, first.1, "checkshift{probe:?}: C={} Rust={}", first.0, first.1);
        for round in 0..500 {
            let noise: Quad =
                (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
            let (again, _) = capture_stdout(|| unsafe {
                (c.checkshift)(noise.0, noise.1, noise.2, noise.3);
                (r.checkshift)(noise.0, noise.1, noise.2, noise.3);
                (
                    (c.checkshift)(probe.0, probe.1, probe.2, probe.3),
                    (r.checkshift)(probe.0, probe.1, probe.2, probe.3),
                )
            });
            assert_eq!(again.0, first.0, "C checkshift drifted on round {round}");
            assert_eq!(again.1, first.1, "Rust checkshift drifted on round {round}");
        }
    });
}

// --- row 26 -------------------------------------------------------------------
fn cfg26_checkshift_boundary_grid() {
    // Full 4-way cross product over a 14-value boundary set (38 416 cases).
    const G: [c_int; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        4,
        100,
        -100,
        0xABCD,
        0xFFFF,
        0x1_0000,
        0x4000_0000,
        i32::MAX,
        i32::MIN,
    ];
    let mut cases: Vec<Quad> = Vec::with_capacity(G.len().pow(4));
    for &a in &G {
        for &b in &G {
            for &c in &G {
                for &d in &G {
                    cases.push((a, b, c, d));
                }
            }
        }
    }
    diff_checkshift(&cases, "row 26 boundary grid");
}

fn cfg26b_checkshift_interesting_sampled() {
    // Sampled cross product over the wider `INTERESTING` set, plus the
    // all-parameters-equal diagonal.
    let mut rng = Rng::new(0x26B0_0026);
    let mut cases: Vec<Quad> = INTERESTING.iter().map(|&v| (v, v, v, v)).collect();
    for _ in 0..30_000 {
        let pick = |rng: &mut Rng| INTERESTING[(rng.next_u32() as usize) % INTERESTING.len()];
        cases.push((pick(&mut rng), pick(&mut rng), pick(&mut rng), pick(&mut rng)));
    }
    diff_checkshift(&cases, "row 26b interesting sampled");
}

// --- row 27 -------------------------------------------------------------------
fn cfg27_checkshift_stdout_verbatim() {
    // A small hand-picked spread, each captured on its own so a divergence is
    // reported against a single call rather than a concatenated stream.
    let cases: [Quad; 12] = [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -1, -1, -1),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, 1, -1, i32::MAX),
        (0xABCD, 0xFFFF, 0x1_0000, -0x1_0000),
        (-2147483648, 0, 0, 0),
        (100, -100, 100, -100),
        (0x4000_0000, 3, 2, 1),
        (12345, -67890, 24680, -13579),
        (0xDEAD_BEEF_u32 as c_int, 0xDEAD_BEEF_u32 as c_int, 7, -7),
    ];
    for q in cases {
        diff_checkshift(&[q], &format!("row 27 verbatim {q:?}"));
    }
}

/// Pins the actual literal format of the emitted text against the C source, so a
/// future edit that changed *both* sides identically would still be caught.
fn cfg27b_checkshift_format_pinning() {
    let (c, r) = both();
    let (c_out, r_out) = serial(|| {
        let (_, co) = capture_stdout(|| unsafe { (c.checkshift)(-5, 7, -9, 11) });
        let (_, ro) = capture_stdout(|| unsafe { (r.checkshift)(-5, 7, -9, 11) });
        (co, ro)
    });
    assert_same_output(&c_out, &r_out, "row 27b format pinning");
    let s = String::from_utf8(c_out).expect("output is not valid UTF-8");
    for needle in [
        "\n=== Starting foo function ===\n",
        "Parameters: -5, 7, -9, 11\n",
        "State initialized with accumulator = -5\n",
        "\n--- Operation 1: Multiply ---\n",
        "\n--- Operation 2: Add ---\n",
        "\n--- Operation 3: XOR ---\n",
        "\n--- Operation 4: Shift ---\n",
        "Variable a = ",
        "Variable b = ",
        "Result of XOR: ",
        "Result of SHIFT: ",
        "\nComputed checksum: 0x",
        "\nFinal accumulator: ",
        "Operation count: 2\n",
        "Final result: ",
        "=== Ending foo function ===\n\n",
    ] {
        assert!(s.contains(needle), "C output is missing {needle:?}; full output:\n{s}");
    }
    // `printf("0x%04X", checksum)` — masked to 16 bits, so exactly 4 upper-case
    // hex digits, always.
    let cs = s.split("Computed checksum: 0x").nth(1).unwrap().lines().next().unwrap();
    assert_eq!(cs.len(), 4, "checksum field was {cs:?}, expected 4 hex digits");
    assert!(
        cs.chars().all(|ch| ch.is_ascii_digit() || ('A'..='F').contains(&ch)),
        "checksum field {cs:?} is not upper-case hex"
    );
}

// ---------------------------------------------------------------------------
// rows 28-29: `checkshift` reassembled from the low-level exports.
//
// This mirrors lines 145-190 of c_src/src/lib.c exactly. `stages` selects, per
// stage, which library provides that step (false = C, true = Rust).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
struct Stages {
    init_state: bool,
    get_operation: bool,
    apply1: bool,
    apply2: bool,
    execute: bool,
    checksum: bool,
}

impl Stages {
    fn from_mask(m: u32) -> Stages {
        Stages {
            init_state: m & 1 != 0,
            get_operation: m & 2 != 0,
            apply1: m & 4 != 0,
            apply2: m & 8 != 0,
            execute: m & 16 != 0,
            checksum: m & 32 != 0,
        }
    }
}

fn manual_pipeline(q: Quad, s: Stages) -> c_int {
    let (c, r) = both();
    let pick = |b: bool| if b { r } else { c };
    let (p1, p2, p3, p4) = q;

    let mut state = StateBuf::poisoned();
    unsafe { (pick(s.init_state).init_state)(state.as_mut_ptr(), p1) };

    let mut params: [c_int; 4] = [p1, p2, p3, p4];

    let g = pick(s.get_operation);
    let (mult_op, add_op, xor_op, shift_op) = unsafe {
        (
            (g.get_operation)(0),
            (g.get_operation)(1),
            (g.get_operation)(2),
            (g.get_operation)(3),
        )
    };

    unsafe {
        (pick(s.apply1).apply_operation)(state.as_mut_ptr(), p2, mult_op);
        (pick(s.apply2).apply_operation)(state.as_mut_ptr(), p3, add_op);
    }

    let ex = pick(s.execute);
    let xor_name = cstring("XOR");
    let shift_name = cstring("SHIFT");
    let xor_result = unsafe {
        (ex.execute_operation)(xor_op, state.accumulator(), p4, xor_name.as_ptr())
    };
    let shift_result =
        unsafe { (ex.execute_operation)(shift_op, xor_result, p2, shift_name.as_ptr()) };

    let checksum =
        unsafe { (pick(s.checksum).compute_checksum)(params.as_mut_ptr(), 4) };

    assert_eq!(state.operation_count(), 2, "manual pipeline lost an operation count");

    // int final_result = (state->accumulator + shift_result) ^ state->checksum;
    // The int operand converts to unsigned for the xor, then back to int.
    ((state.accumulator().wrapping_add(shift_result) as u32) ^ checksum) as c_int
}

fn pipeline_cases() -> Vec<Quad> {
    let mut rng = Rng::new(0x2800_0028);
    let mut cases: Vec<Quad> = vec![
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -1, -1, -1),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MAX, 0, -1),
        (0xABCD, 0xFFFF, 0x4000_0000, -100),
    ];
    cases.extend((0..800).map(|_| {
        (
            rng.next_i32_biased(),
            rng.next_i32_biased(),
            rng.next_i32_biased(),
            rng.next_i32_biased(),
        )
    }));
    cases
}

/// row 28 — the hand-assembled pipeline, run wholly against C and wholly against
/// Rust, must reproduce each library's own `checkshift` return value.
fn cfg28_manual_pipeline_matches_checkshift() {
    let (c, r) = both();
    let cases = pipeline_cases();
    let all_c = Stages::from_mask(0);
    let all_r = Stages::from_mask(0b11_1111);
    serial(|| {
        for q in cases {
            let (vals, _) = capture_stdout(|| {
                (
                    unsafe { (c.checkshift)(q.0, q.1, q.2, q.3) },
                    unsafe { (r.checkshift)(q.0, q.1, q.2, q.3) },
                    manual_pipeline(q, all_c),
                    manual_pipeline(q, all_r),
                )
            });
            let (c_ck, r_ck, c_manual, r_manual) = vals;
            assert_eq!(c_ck, r_ck, "checkshift{q:?}: C={c_ck} Rust={r_ck}");
            assert_eq!(
                c_ck, c_manual,
                "all-C manual pipeline{q:?} = {c_manual}, but C checkshift = {c_ck}"
            );
            assert_eq!(
                r_ck, r_manual,
                "all-Rust manual pipeline{q:?} = {r_manual}, but Rust checkshift = {r_ck}"
            );
        }
    });
}

/// row 29 — every one of the 64 C/Rust stage-assignment masks must produce the
/// same value as `checkshift`, for every case.
fn cfg29_manual_pipeline_mixed_stages() {
    let (c, _) = both();
    let cases = pipeline_cases();
    serial(|| {
        for q in cases {
            let (results, _) = capture_stdout(|| {
                let reference = unsafe { (c.checkshift)(q.0, q.1, q.2, q.3) };
                let mixed: Vec<c_int> =
                    (0..64u32).map(|m| manual_pipeline(q, Stages::from_mask(m))).collect();
                (reference, mixed)
            });
            let (reference, mixed) = results;
            for (m, v) in mixed.iter().enumerate() {
                assert_eq!(
                    *v, reference,
                    "stage mask {m:#08b} ({:?}) on {q:?} gave {v}, expected {reference}",
                    Stages::from_mask(m as u32)
                );
            }
        }
    });
}

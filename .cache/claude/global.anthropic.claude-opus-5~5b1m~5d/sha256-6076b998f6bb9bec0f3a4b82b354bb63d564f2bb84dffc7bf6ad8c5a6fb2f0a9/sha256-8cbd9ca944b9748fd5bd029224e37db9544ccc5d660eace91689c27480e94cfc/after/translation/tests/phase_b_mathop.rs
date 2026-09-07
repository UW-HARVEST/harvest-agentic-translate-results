//! Phase B — `mathop`, the one entry point declared in `include/lib.h`.
//! CONFIGS.md rows 19-24.
//!
//! `mathop` keeps two function-`static`s (`computation_history`,
//! `history_count`), so it is STATEFUL across calls and each `.so` has its own
//! copy. Everything here therefore lives in a SINGLE `#[test]` in its own test
//! binary, so the process starts with both statics freshly zeroed and the two
//! libraries are driven in strict 1:1 lockstep. Both the return value AND the
//! four `printf` lines are compared byte-for-byte.

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0003;

struct Driver {
    p: Pair,
    calls: usize,
    skipped: usize,
}

impl Driver {
    /// One lockstep step: call C, capture its stdout; call Rust, capture its
    /// stdout; compare the return value and the raw stdout bytes.
    ///
    /// Inputs that make the C reach `INT_MIN / -1` are routed to the
    /// fork-based `phase_c_trap` test instead — running them here would kill
    /// this process (both libraries raise `SIGFPE`, see ERRORS.md row 20).
    fn step(&mut self, ctx: &str, p1: i32, p2: i32, p3: i32, p4: i32) -> (i32, String) {
        if mathop_traps(p1, p2, p3, p4) {
            self.skipped += 1;
            return (0, String::new());
        }
        let (vc, oc) = self.p.c.mathop_capture(p1, p2, p3, p4);
        let (vr, or) = self.p.rs.mathop_capture(p1, p2, p3, p4);
        self.calls += 1;
        assert_eq!(
            vc, vr,
            "{ctx}: mathop({p1}, {p2}, {p3}, {p4}) return value: C={vc} Rust={vr}"
        );
        assert_eq!(
            oc.as_bytes(),
            or.as_bytes(),
            "{ctx}: mathop({p1}, {p2}, {p3}, {p4}) stdout differs.\n--- C ---\n{oc}--- Rust ---\n{or}"
        );
        (vc, oc)
    }
}

fn history_line(out: &str) -> i32 {
    out.lines()
        .find_map(|l| l.strip_prefix("History entries: "))
        .unwrap_or_else(|| panic!("no `History entries:` line in:\n{out}"))
        .trim()
        .parse()
        .expect("history count parses")
}

#[test]
fn mathop_all_rows() {
    let mut d = Driver { p: pair(), calls: 0, skipped: 0 };

    // -----------------------------------------------------------------------
    // Row 23 — stateful sequence from a FRESH process. Each mathop call
    // appends 2 entries, so `History entries:` must walk 2,4,6,8,10 and then
    // saturate at 10 (the `*history_count < 10` bound). This MUST be the first
    // thing the process does.
    // -----------------------------------------------------------------------
    let expected_walk = [2, 4, 6, 8, 10, 10, 10, 10];
    for (i, &want) in expected_walk.iter().enumerate() {
        let (_, out) = d.step(&format!("row23 call {i}"), 7, 3, 1, 2);
        assert_eq!(
            history_line(&out),
            want,
            "row23: call {i} should report {want} history entries; full output:\n{out}"
        );
    }

    // -----------------------------------------------------------------------
    // Row 24 — stdout byte-for-byte is already asserted inside `step`; verify
    // the shape of the captured text so the comparison cannot be vacuous.
    // -----------------------------------------------------------------------
    let (v, out) = d.step("row24", 50, 9, 3, 4);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 4, "mathop must print exactly 4 lines, got:\n{out}");
    assert!(lines[0].starts_with("Computation performed at timestamp: "), "{out}");
    assert!(lines[1].starts_with("Operation priority: "), "{out}");
    assert!(lines[2].starts_with("History entries: "), "{out}");
    assert_eq!(lines[3], format!("Final result: {v}"), "{out}");
    assert!(out.ends_with('\n'), "output must end with a newline");

    // -----------------------------------------------------------------------
    // Row 19 — 5x5 grid of (first op, second op) with a VALID validation char.
    // param3 % 5 + 1 picks op1; ((param4 + 1) % 5) + 1 picks op2.
    // param1 is the left operand AND the validation-char source: param1 in
    // '1'..'5' (49..53) makes `is_valid_operation` true.
    // -----------------------------------------------------------------------
    for p3 in 0..5i32 {
        for p4 in 0..5i32 {
            for &p1 in &[49i32, 50, 51, 52, 53] {
                for &p2 in &[0i32, 1, -1, 7, -7, 100] {
                    d.step(&format!("row19 p3={p3} p4={p4}"), p1, p2, p3, p4);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Row 20 — same 5x5 op grid, but param1 makes the validation char INVALID
    // (the dead-store branch): param1 % 128 outside '1'..'5', including 0.
    // -----------------------------------------------------------------------
    for p3 in 0..5i32 {
        for p4 in 0..5i32 {
            for &p1 in &[0i32, 48, 54, 128, 256, 127, -1, -49, -128, i32::MIN] {
                for &p2 in &[0i32, 1, -1, 13] {
                    d.step(&format!("row20 p3={p3} p4={p4}"), p1, p2, p3, p4);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Row 22 — boundary values in all four parameter positions.
    // -----------------------------------------------------------------------
    let b = [
        0i32,
        1,
        -1,
        2,
        -2,
        4,
        -4,
        5,
        -5,
        6,
        -6,
        127,
        128,
        -128,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    // Full 4-way cross product is 18^4 = 104976 calls; walk the axes instead so
    // every boundary value is exercised in every position.
    for &v in &b {
        for &w in &b {
            d.step("row22 p1/p2", v, w, 1, 1);
            d.step("row22 p3/p4", 7, 3, v, w);
            d.step("row22 p1/p3", v, 3, w, 1);
            d.step("row22 p2/p4", 7, v, 1, w);
            d.step("row22 p1/p4", v, 3, 1, w);
            d.step("row22 p2/p3", 7, v, w, 1);
        }
    }

    // -----------------------------------------------------------------------
    // Row 21 — randomized over the full i32 range, fixed seed.
    // -----------------------------------------------------------------------
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..700 {
        let p1 = rng.i32_interesting();
        let p2 = rng.i32_interesting();
        let p3 = rng.i32_interesting();
        let p4 = rng.i32_interesting();
        d.step(&format!("row21 #{i}"), p1, p2, p3, p4);
    }

    // -----------------------------------------------------------------------
    // Row 25 (part) — mathop composed with the low-level API in the same
    // process: the manual pipeline must reproduce mathop's arithmetic exactly.
    // -----------------------------------------------------------------------
    for i in 0..300 {
        let p1 = rng.i32_interesting();
        let p2 = rng.i32_interesting();
        let p3 = rng.i32_interesting();
        let p4 = rng.i32_interesting();
        if mathop_traps(p1, p2, p3, p4) {
            continue;
        }
        let (v, _) = d.step(&format!("row25 #{i}"), p1, p2, p3, p4);

        // Re-derive using the LOW-LEVEL exports of each .so and confirm the
        // composed pipeline reproduces mathop exactly, in both libraries.
        let op1 = mathop_op1(p3);
        let op2 = mathop_op2(p4);
        for lib in [&d.p.c, &d.p.rs] {
            let f1 = lib.select_operation(op1);
            let f2 = lib.select_operation(op2);
            let mid = unsafe { f1(p1, p2, 0) };
            let fin = unsafe { f2(mid, p4, 0) };
            let expect = fin
                .wrapping_add(lib.get_operation_priority(op1))
                .wrapping_add((lib.get_computation_timestamp() % 100) as i32);
            assert_eq!(
                v, expect,
                "row25 #{i} ({}): mathop({p1},{p2},{p3},{p4})={v} but the composed \
                 low-level pipeline gives {expect} (op1={op1} op2={op2} mid={mid})",
                lib.name
            );
        }

        // Third opinion: the independent reference model in common/mod.rs.
        let modelled = mathop_model(p1, p2, p3, p4, d.p.c.get_computation_timestamp());
        assert_eq!(
            Some(v),
            modelled,
            "row25 #{i}: mathop({p1},{p2},{p3},{p4})={v} disagrees with the reference model"
        );
    }

    assert!(d.calls > 2000, "expected a large call count, ran {}", d.calls);
    eprintln!(
        "mathop lockstep calls compared: {} (skipped {} SIGFPE inputs, \
         covered by phase_c_trap)",
        d.calls, d.skipped
    );
}

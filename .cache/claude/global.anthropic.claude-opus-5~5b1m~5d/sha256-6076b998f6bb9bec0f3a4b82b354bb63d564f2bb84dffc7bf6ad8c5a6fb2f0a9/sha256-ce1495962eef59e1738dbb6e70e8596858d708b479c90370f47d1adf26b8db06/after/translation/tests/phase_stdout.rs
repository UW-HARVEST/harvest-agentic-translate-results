// PHASE B rows 17-24 + PHASE C rows 20, 23, 24 -- everything that drives
// `mathop`, whose observable behaviour includes the four lines it prints.
//
// `harness = false`: this binary runs its checks STRICTLY SEQUENTIALLY from a
// hand-written `main`. That matters for two reasons:
//
//  1. The libtest harness writes its own progress lines ("test <name> ... ok")
//     to fd 1. With the default harness those writes interleave with -- and
//     corrupt -- a stdout region captured by a concurrently running test.
//  2. `mathop` hides two function-local `static`s. A fixed call order makes the
//     hidden history counter deterministic, so the exact expected values (2, 4,
//     6, 8, 10, 10, ...) can be asserted rather than merely compared.
//
// Nothing here prints to fd 1 outside a capture region: all progress reporting
// goes to stderr.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// tiny sequential test driver
// ---------------------------------------------------------------------------

struct Driver {
    passed: usize,
    failed: Vec<String>,
}

impl Driver {
    fn run(&mut self, name: &str, f: impl FnOnce() + std::panic::UnwindSafe) {
        eprint!("test {name} ... ");
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                eprintln!("ok");
                self.passed += 1;
            }
            Err(_) => {
                eprintln!("FAILED");
                self.failed.push(name.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// mathop stdout helpers
// ---------------------------------------------------------------------------

const PREFIXES: [&str; 4] = [
    "Computation performed at timestamp: ",
    "Operation priority: ",
    "History entries: ",
    "Final result: ",
];

/// Split captured stdout into the 4-line blocks `mathop` emits per call, and
/// validate that every line is one of the library's own lines in the right
/// order (so foreign output can never be silently absorbed).
fn blocks(bytes: &[u8]) -> Vec<Vec<String>> {
    let text = String::from_utf8_lossy(bytes).into_owned();
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    assert_eq!(
        lines.len() % 4,
        0,
        "mathop prints exactly 4 lines per call; captured {} lines:\n{text}",
        lines.len()
    );
    for (i, line) in lines.iter().enumerate() {
        let want = PREFIXES[i % 4];
        assert!(
            line.starts_with(want),
            "captured line {i} should start with {want:?} but is {line:?}\n\
             (foreign output leaked into the captured region)"
        );
    }
    lines.chunks(4).map(|c| c.to_vec()).collect()
}

/// Call `mathop` on both libraries for every case, strictly alternating C then
/// Rust, with fd 1 captured for the whole batch. Returns the return-value pairs
/// and the per-call stdout blocks.
fn mathop_batch(cases: &[(i32, i32, i32, i32)]) -> (Vec<(i32, i32)>, Vec<Vec<String>>) {
    let l = libs();
    let mut results = Vec::with_capacity(cases.len());
    let (_, bytes) = capture_stdout(|| {
        for &(p1, p2, p3, p4) in cases {
            let c = unsafe { (l.c.mathop)(p1, p2, p3, p4) };
            let r = unsafe { (l.rust.mathop)(p1, p2, p3, p4) };
            results.push((c, r));
        }
    });
    let b = blocks(&bytes);
    assert_eq!(
        b.len(),
        cases.len() * 2,
        "expected 2 blocks (C then Rust) per call"
    );
    (results, b)
}

/// Assert both libraries agree on every return value and every printed line,
/// and that both match the independent model transcribed from the C source.
fn assert_batch_matches(cases: &[(i32, i32, i32, i32)], tag: &str) {
    let ts = unsafe { (libs().c.get_computation_timestamp)() };
    let (results, b) = mathop_batch(cases);
    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        let ctx = format!("[{tag}] mathop({p1}, {p2}, {p3}, {p4})");
        let (c, r) = results[i];
        assert_eq!(c, r, "{ctx}: return value differs");
        assert_eq!(
            b[2 * i],
            b[2 * i + 1],
            "{ctx}: stdout differs\n  C:    {:?}\n  Rust: {:?}",
            b[2 * i],
            b[2 * i + 1]
        );
        assert_eq!(
            c,
            mathop_expected(p1, p2, p3, p4, ts),
            "{ctx}: disagrees with the model transcribed from the C source"
        );
        let (op1, _) = mathop_ops(p3, p4);
        assert_eq!(
            priority(&b[2 * i]),
            op1.wrapping_mul(10),
            "{ctx}: priority line"
        );
        assert_eq!(
            b[2 * i][3],
            format!("Final result: {c}"),
            "{ctx}: the printed result must equal the returned one"
        );
    }
}

fn history_entries(block: &[String]) -> i32 {
    block[2]
        .strip_prefix("History entries: ")
        .expect("History entries line")
        .parse()
        .expect("integer")
}

fn priority(block: &[String]) -> i32 {
    block[1]
        .strip_prefix("Operation priority: ")
        .expect("Operation priority line")
        .parse()
        .expect("integer")
}

fn random_cases(n: usize, seed: u64) -> Vec<(i32, i32, i32, i32)> {
    let mut rng = Rng::with_seed(seed);
    let mut cases = Vec::with_capacity(n);
    while cases.len() < n {
        let (p1, p2, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        if mathop_would_trap(p1, p2, p3, p4) {
            continue; // ERRORS.md row 25: the C traps, there is nothing to compare
        }
        cases.push((p1, p2, p3, p4));
    }
    cases
}

// ---------------------------------------------------------------------------
// CONFIGS row 17 -- the VERY FIRST mathop call, on pristine statics.
// Must run first in this process; `main` guarantees that.
// ---------------------------------------------------------------------------
fn cfg_17_mathop_first_call_fresh_state() {
    // param1 = 50 -> validation char '2' -> is_valid true
    // param3 = 3  -> selected_op = 4 (DIVIDE), priority 40
    // param4 = 7  -> second_op   = ((7+1) % 5) + 1 = 4 (DIVIDE)
    let cases = [(50, 7, 3, 7), (50, 7, 3, 7), (50, 7, 3, 7)];
    let (results, b) = mathop_batch(&cases);

    for (i, (c, r)) in results.iter().enumerate() {
        assert_eq!(c, r, "call {i}: return value differs");
        assert_eq!(b[2 * i], b[2 * i + 1], "call {i}: stdout differs");
    }

    // The statics start NULL/0, and each mathop call appends exactly 2 records.
    for (i, expected) in [2, 4, 6].iter().enumerate() {
        assert_eq!(
            history_entries(&b[2 * i]),
            *expected,
            "C: call {} should report {expected} history entries, got {:?}",
            i + 1,
            b[2 * i]
        );
        assert_eq!(
            history_entries(&b[2 * i + 1]),
            *expected,
            "Rust: call {} should report {expected} history entries, got {:?}",
            i + 1,
            b[2 * i + 1]
        );
    }
    assert_eq!(priority(&b[0]), 40, "selected_op 4 -> priority 40");
    assert_eq!(
        results[0].0, results[1].0,
        "mathop's result must not depend on the accumulated history state"
    );
}

// ---------------------------------------------------------------------------
// CONFIGS row 18 -- long randomized sequence; drives the statics from their
// (already warm) state into saturation and keeps them there.
// ---------------------------------------------------------------------------
fn cfg_18_mathop_long_random_sequence_stdout() {
    let cases = random_cases(400, Rng::SEED);
    let (results, b) = mathop_batch(&cases);
    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        let ctx = format!("mathop({p1}, {p2}, {p3}, {p4})");
        assert_eq!(results[i].0, results[i].1, "{ctx}: return differs");
        assert_eq!(b[2 * i], b[2 * i + 1], "{ctx}: stdout differs");
    }
    // 2 records per call, capacity 10. Row 17 already ran 3 calls (count 6), so
    // this batch climbs 8 -> 10 and is then pinned. Assert the counter is
    // monotonic, never exceeds capacity, saturates, and stays saturated.
    let mut prev = 0;
    for i in 0..cases.len() {
        let n = history_entries(&b[2 * i]);
        assert_eq!(
            n,
            history_entries(&b[2 * i + 1]),
            "call {i}: the two libraries' history counters diverged"
        );
        assert!(n >= prev, "call {i}: the counter went backwards ({prev} -> {n})");
        assert!(n <= HISTORY_CAPACITY, "call {i}: counter {n} exceeded capacity");
        assert!(n % 2 == 0, "call {i}: 2 records per call, got {n}");
        prev = n;
    }
    assert_eq!(
        prev, HISTORY_CAPACITY,
        "the static history must end saturated at capacity"
    );
    // Saturated for the overwhelming majority of a 400-call batch.
    let saturated = (0..cases.len())
        .filter(|&i| history_entries(&b[2 * i]) == HISTORY_CAPACITY)
        .count();
    assert!(
        saturated >= cases.len() - 5,
        "expected the counter to saturate within a few calls, {saturated}/{} saturated",
        cases.len()
    );
}

// ---------------------------------------------------------------------------
// CONFIGS row 19 -- every first-operation residue of param3 (selected_op 1..5)
// ---------------------------------------------------------------------------
fn cfg_19_mathop_all_first_ops() {
    let mut rng = Rng::with_seed(0x1111_2222_3333_4444);
    let mut cases = Vec::new();
    let mut expect_op = Vec::new();

    for residue in 0..5i32 {
        let mut n = 0;
        while n < 40 {
            let p3 = (rng.below(1_000_000) as i32) * 5 + residue; // p3 >= 0
            let p1 = rng.interesting_i32();
            // Force the b == 0 guard through the DIV/MOD arms every so often.
            let p2 = if n % 7 == 0 { 0 } else { rng.interesting_i32() };
            let p4 = rng.interesting_i32();
            if mathop_would_trap(p1, p2, p3, p4) {
                continue;
            }
            assert_eq!(p3 % 5 + 1, residue + 1);
            cases.push((p1, p2, p3, p4));
            expect_op.push(residue + 1);
            n += 1;
        }
    }

    let (results, b) = mathop_batch(&cases);
    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        let ctx = format!("mathop({p1}, {p2}, {p3}, {p4}) first_op={}", expect_op[i]);
        assert_eq!(results[i].0, results[i].1, "{ctx}: return differs");
        assert_eq!(b[2 * i], b[2 * i + 1], "{ctx}: stdout differs");
        // The priority line proves which switch arm ran.
        assert_eq!(priority(&b[2 * i]), expect_op[i] * 10, "{ctx}: priority");
    }
    let seen: std::collections::BTreeSet<i32> = expect_op.iter().copied().collect();
    assert_eq!(
        seen,
        (1..=5).collect(),
        "all five first-operation arms must be covered"
    );
}

// ---------------------------------------------------------------------------
// CONFIGS row 20 -- every second-operation residue of param4 (second_op 1..5)
// ---------------------------------------------------------------------------
fn cfg_20_mathop_all_second_ops() {
    let mut rng = Rng::with_seed(0x5555_6666_7777_8888);
    let mut cases = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for residue in 0..5i32 {
        let mut n = 0;
        while n < 40 {
            // want ((p4 + 1) % 5) + 1 == residue + 1
            let p4 = (rng.below(1_000_000) as i32) * 5 + residue + 4;
            let p1 = rng.interesting_i32();
            let p2 = rng.interesting_i32();
            let p3 = rng.interesting_i32();
            if mathop_would_trap(p1, p2, p3, p4) {
                continue;
            }
            assert_eq!((p4 + 1) % 5 + 1, residue + 1);
            cases.push((p1, p2, p3, p4));
            seen.insert(residue + 1);
            n += 1;
        }
    }
    assert_eq!(
        seen,
        (1..=5).collect(),
        "all five second-operation arms must be covered"
    );
    assert_batch_matches(&cases, "second ops");
}

// ---------------------------------------------------------------------------
// CONFIGS rows 21/22 -- the validation-char axis
// ---------------------------------------------------------------------------
fn mathop_validation_char_axis(want_valid: bool, tag: &str) {
    let mut rng = Rng::with_seed(0x9999_AAAA_BBBB_CCCC ^ want_valid as u64);
    let mut cases = Vec::new();
    while cases.len() < 150 {
        let p1 = rng.next_i32();
        let vc = (p1 % 128) as i8;
        let is_valid = vc != 0 && vc >= b'1' as i8 && vc <= b'5' as i8;
        if is_valid != want_valid {
            continue;
        }
        let (p2, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        if mathop_would_trap(p1, p2, p3, p4) {
            continue;
        }
        cases.push((p1, p2, p3, p4));
    }
    assert_batch_matches(&cases, tag);
}

fn cfg_21_mathop_valid_validation_char() {
    mathop_validation_char_axis(true, "valid validation char");
}

fn cfg_22_mathop_invalid_validation_char() {
    mathop_validation_char_axis(false, "invalid validation char");
}

// ---------------------------------------------------------------------------
// CONFIGS row 23 -- full 4-fold corner cross-product (9^4 = 6561 combinations)
// ---------------------------------------------------------------------------
fn cfg_23_mathop_corner_cross_product() {
    const CORNERS: [i32; 9] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    let mut cases = Vec::new();
    let mut skipped = 0usize;
    for &p1 in &CORNERS {
        for &p2 in &CORNERS {
            for &p3 in &CORNERS {
                for &p4 in &CORNERS {
                    if mathop_would_trap(p1, p2, p3, p4) {
                        skipped += 1;
                        continue;
                    }
                    cases.push((p1, p2, p3, p4));
                }
            }
        }
    }
    assert_eq!(cases.len() + skipped, 9usize.pow(4));
    // As it happens no corner combination reaches a trapping idiv: op1 == 4/5
    // needs p3 % 5 in {3,4}, which none of these corners produce, and op2 == 4/5
    // only occurs for p4 == 2, where the divisor is 2 rather than -1. So the
    // whole 6561-combination cross-product is compared.
    assert_eq!(skipped, 0, "unexpected trap in the corner set");
    eprint!("({} combos, {skipped} C traps skipped) ", cases.len());
    assert_batch_matches(&cases, "corner cross-product");
}

// ---------------------------------------------------------------------------
// ERRORS row 20 -- mathop's invalid-char fallback is a dead store: forcing it
// must not change anything observable.
// ---------------------------------------------------------------------------
fn err_20_mathop_invalid_char_has_no_effect() {
    let mut rng = Rng::with_seed(0xDEAD_BEEF_CAFE_1234);
    let mut cases = Vec::new();
    let mut kinds = Vec::new();
    for _ in 0..60 {
        let base = (rng.below(10_000) as i32) * 128; // % 128 == 0   -> invalid (NUL)
        for (kind, p1) in [
            ("nul", base),
            ("valid '1'", base + 49),
            ("valid '5'", base + 53),
            ("invalid '<'", base + 60),
            ("invalid '0'", base + 48),
        ] {
            let (p2, p3, p4) = (5, 1, 3); // op1 = 2 (MUL), op2 = 5 (MOD)
            if mathop_would_trap(p1, p2, p3, p4) {
                continue;
            }
            cases.push((p1, p2, p3, p4));
            kinds.push(kind);
        }
    }

    let (results, b) = mathop_batch(&cases);
    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        let ctx = format!("[{}] mathop({p1}, {p2}, {p3}, {p4})", kinds[i]);
        assert_eq!(results[i].0, results[i].1, "{ctx}: return differs");
        assert_eq!(b[2 * i], b[2 * i + 1], "{ctx}: stdout differs");
    }

    // The fallback only ever writes `validation_char`, which is never read
    // again. param1 itself IS an operand, so the final result legitimately
    // changes with it -- but everything that does NOT depend on param1 (the
    // selected operation, hence the priority line, and the history counter)
    // must be identical across all five validity variants of a group.
    for (cs, rs) in cases.chunks(5).zip(b.chunks(10)) {
        if cs.len() != 5 {
            continue;
        }
        let want_priority = priority(&rs[0]);
        let want_entries = history_entries(&rs[0]);
        for j in 0..5 {
            assert_eq!(
                priority(&rs[2 * j]),
                want_priority,
                "validity must not change the selected operation: {:?} (variant {j})",
                cs[j]
            );
            assert_eq!(
                history_entries(&rs[2 * j]),
                want_entries,
                "validity must not change the history counter: {:?} (variant {j})",
                cs[j]
            );
        }
    }

    // And every result must equal the independent model, which contains no
    // validation-char term at all -- direct proof the fallback is unobservable.
    let ts = unsafe { (libs().c.get_computation_timestamp)() };
    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        assert_eq!(
            results[i].0,
            mathop_expected(p1, p2, p3, p4, ts),
            "[{}] mathop({p1}, {p2}, {p3}, {p4}): the model (which ignores the \
             validation char entirely) must predict the result exactly",
            kinds[i]
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 23 -- negative param3 -> out-of-range Operation (0, -1, -2, -3)
// with a zero/negative priority
// ---------------------------------------------------------------------------
fn err_23_mathop_negative_param3_out_of_range_op() {
    let mut rng = Rng::with_seed(0x0F0F_0F0F_1234_5678);
    let mut cases = Vec::new();
    let mut expect_op = Vec::new();

    for residue in 0..5i32 {
        for _ in 0..30 {
            let p3 = -((rng.below(1_000_000) as i32 + 1) * 5 + residue);
            let p1 = rng.interesting_i32();
            let p2 = rng.interesting_i32();
            let p4 = rng.interesting_i32();
            if mathop_would_trap(p1, p2, p3, p4) {
                continue;
            }
            let op = p3 % 5 + 1;
            assert!(op <= 1, "negative param3 must yield op <= 1, got {op}");
            cases.push((p1, p2, p3, p4));
            expect_op.push(op);
        }
    }
    for p3 in [-1i32, -2, -3, -4, -5, -6, i32::MIN, i32::MIN + 1] {
        if !mathop_would_trap(7, 3, p3, 11) {
            cases.push((7, 3, p3, 11));
            expect_op.push(p3 % 5 + 1);
        }
    }

    let (results, b) = mathop_batch(&cases);
    for (i, &(p1, p2, p3, p4)) in cases.iter().enumerate() {
        let ctx = format!("mathop({p1}, {p2}, {p3}, {p4}) op={}", expect_op[i]);
        assert_eq!(results[i].0, results[i].1, "{ctx}: return differs");
        assert_eq!(b[2 * i], b[2 * i + 1], "{ctx}: stdout differs");
        assert_eq!(
            priority(&b[2 * i]),
            expect_op[i].wrapping_mul(10),
            "{ctx}: out-of-range op must give a zero/negative priority"
        );
        assert!(priority(&b[2 * i]) <= 10, "{ctx}: priority should be <= 10");
    }
    let seen: std::collections::BTreeSet<i32> = expect_op.iter().copied().collect();
    assert!(
        seen.contains(&0) && seen.contains(&-1) && seen.contains(&-2) && seen.contains(&-3),
        "expected out-of-range ops 0, -1, -2, -3; saw {seen:?}"
    );
}

// ---------------------------------------------------------------------------
// ERRORS row 24 -- param4 == INT_MAX overflows `param4 + 1`; negative param4
// gives a negative residue. Either way second_op leaves the enum range.
// ---------------------------------------------------------------------------
fn err_24_mathop_param4_overflow_and_negative() {
    // INT_MAX + 1 wraps to INT_MIN; INT_MIN % 5 == -3; -3 + 1 == -2 -> ADD arm.
    assert_eq!(i32::MAX.wrapping_add(1).wrapping_rem(5).wrapping_add(1), -2);

    let mut rng = Rng::with_seed(0xABCD_EF01_2345_6789);
    let mut cases = Vec::new();
    for &p4 in &[
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        -1,
        -2,
        -3,
        -4,
        -5,
        -6,
        -7,
    ] {
        for _ in 0..20 {
            let (p1, p2, p3) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            if mathop_would_trap(p1, p2, p3, p4) {
                continue;
            }
            cases.push((p1, p2, p3, p4));
        }
    }
    assert!(!cases.is_empty());
    assert_batch_matches(&cases, "param4 overflow/negative");
}

// ---------------------------------------------------------------------------
// CONFIGS row 24 -- the stdout channel itself: exact formatting of all four
// lines, including `%ld` for time_t and the counter as it saturates.
// ---------------------------------------------------------------------------
fn cfg_24_mathop_stdout_formatting() {
    let l = libs();
    let ts = unsafe { (l.c.get_computation_timestamp)() };
    let cases = [(49, 6, 0, 4)]; // op1 = 1 (ADD), priority 10; op2 = 1 (ADD)
    let (results, b) = mathop_batch(&cases);
    let (c, r) = results[0];
    assert_eq!(c, r, "return differs");
    assert_eq!(b[0], b[1], "stdout differs");

    let expected = vec![
        format!("Computation performed at timestamp: {ts}"),
        "Operation priority: 10".to_string(),
        format!("History entries: {HISTORY_CAPACITY}"),
        format!("Final result: {c}"),
    ];
    assert_eq!(
        b[0], expected,
        "the C's exact printf formatting must be reproduced"
    );

    // 49 + 6 = 55; 55 + 4 = 59; + priority 10; + (ts % 100)
    let want = 59i32
        .wrapping_add(10)
        .wrapping_add((ts % 100) as i32);
    assert_eq!(c, want, "hand-computed result from the C source");
}

// ---------------------------------------------------------------------------
fn main() {
    // Force both libraries to load (and, on a clean tree, build) BEFORE any
    // stdout capture region, so no build-tool output can land in a capture.
    let l = libs();
    eprintln!("C   .so: {}", l.c.path.display());
    eprintln!("Rust .so: {}", l.rust.path.display());
    eprintln!("\nrunning 11 sequential mathop/stdout checks");

    let mut d = Driver {
        passed: 0,
        failed: Vec::new(),
    };

    // Row 17 MUST be first: it observes the pristine `static` state.
    d.run("cfg_17_mathop_first_call_fresh_state", cfg_17_mathop_first_call_fresh_state);
    d.run("cfg_18_mathop_long_random_sequence_stdout", cfg_18_mathop_long_random_sequence_stdout);
    d.run("cfg_19_mathop_all_first_ops", cfg_19_mathop_all_first_ops);
    d.run("cfg_20_mathop_all_second_ops", cfg_20_mathop_all_second_ops);
    d.run("cfg_21_mathop_valid_validation_char", cfg_21_mathop_valid_validation_char);
    d.run("cfg_22_mathop_invalid_validation_char", cfg_22_mathop_invalid_validation_char);
    d.run("cfg_23_mathop_corner_cross_product", cfg_23_mathop_corner_cross_product);
    d.run("cfg_24_mathop_stdout_formatting", cfg_24_mathop_stdout_formatting);
    d.run("err_20_mathop_invalid_char_has_no_effect", err_20_mathop_invalid_char_has_no_effect);
    d.run("err_23_mathop_negative_param3_out_of_range_op", err_23_mathop_negative_param3_out_of_range_op);
    d.run("err_24_mathop_param4_overflow_and_negative", err_24_mathop_param4_overflow_and_negative);

    eprintln!(
        "\nphase_stdout result: {}. {} passed; {} failed",
        if d.failed.is_empty() { "ok" } else { "FAILED" },
        d.passed,
        d.failed.len()
    );
    if !d.failed.is_empty() {
        for f in &d.failed {
            eprintln!("  failed: {f}");
        }
        std::process::exit(1);
    }
}

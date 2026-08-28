//! Two things live here.
//!
//! First, a golden table: one input per reachable `return` in the C, with the
//! stdout, stderr and exit status the C program actually produces. Both
//! programs are checked against it, so a corpus that silently stops reaching a
//! branch shows up as a failure rather than as a quietly smaller test run.
//!
//! Second, a deterministic pseudo-random sweep over structured inputs and over
//! arbitrary bytes. The generator is seeded, so a failure is reproducible.

mod harness;
use harness::{compare_all, describe, run, c_binary, rust_binary};

/// One golden expectation: stdin, then the stdout, stderr and exit status
/// the C program produces for it.
type Golden = (&'static [u8], &'static [u8], &'static [u8], i32);

/// (stdin, expected stdout, expected stderr, expected exit status)
const GOLDEN: &[Golden] = &[
    (b"", b"", b"Error reading operation\n", 1),
    (b"0", b"", b"Error reading parameter\n", 1),
    (b"0\n", b"", b"Error reading parameter\n", 1),
    (b"0\n0", b"", b"Error reading decision string\n", 1),
    (b"0\n0\n", b"", b"Error reading decision string\n", 1),
    (b"0\n0\n\n", b"-1\n", b"", 0),
    (b"9\n0\nyyy\n", b"-3\n", b"", 0),
    (b"-1\n0\nyyy\n", b"-3\n", b"", 0),
    (b"4\n0\ny\n", b"-3\n", b"", 0),
    (b"0\n0\nyyy\n", b"107\n", b"", 0),
    (b"0\n0\nyyn\n", b"56\n", b"", 0),
    (b"0\n0\nyny\n", b"35\n", b"", 0),
    (b"0\n0\nynn\n", b"14\n", b"", 0),
    (b"0\n0\nnyy\n", b"23\n", b"", 0),
    (b"0\n0\nnyn\n", b"-10\n", b"", 0),
    (b"0\n0\nnny\n", b"-20\n", b"", 0),
    (b"0\n0\nnnn\n", b"0\n", b"", 0),
    (b"0\n0\ny\n", b"-2\n", b"", 0),
    (b"0\n0\nyy\n", b"-2\n", b"", 0),
    (b"1\n0\nyyy\n", b"100\n", b"", 0),
    (b"1\n0\nyyn\n", b"50\n", b"", 0),
    (b"1\n0\nyny\n", b"51\n", b"", 0),
    (b"1\n0\nnyy\n", b"52\n", b"", 0),
    (b"1\n0\nynn\n", b"10\n", b"", 0),
    (b"1\n0\nnyn\n", b"11\n", b"", 0),
    (b"1\n0\nnny\n", b"12\n", b"", 0),
    (b"1\n0\nnnn\n", b"0\n", b"", 0),
    (b"1\n0\ny\n", b"-2\n", b"", 0),
    (b"1\n0\n\n", b"-1\n", b"", 0),
    (b"1\n1\nyyy\n", b"103\n", b"", 0),
    (b"1\n1\nyyn\n", b"102\n", b"", 0),
    (b"1\n1\nyny\n", b"102\n", b"", 0),
    (b"1\n1\nnyy\n", b"102\n", b"", 0),
    (b"1\n1\nynn\n", b"101\n", b"", 0),
    (b"1\n1\nnyn\n", b"101\n", b"", 0),
    (b"1\n1\nnny\n", b"101\n", b"", 0),
    (b"1\n1\nnnn\n", b"0\n", b"", 0),
    (b"1\n1\ny\n", b"-2\n", b"", 0),
    (b"1\n1\n\n", b"-1\n", b"", 0),
    (b"1\n2\nyyy\n", b"7\n", b"", 0),
    (b"1\n2\nyyn\n", b"0\n", b"", 0),
    (b"1\n2\nyny\n", b"0\n", b"", 0),
    (b"1\n2\nnyy\n", b"0\n", b"", 0),
    (b"1\n2\nynn\n", b"1\n", b"", 0),
    (b"1\n2\nnyn\n", b"2\n", b"", 0),
    (b"1\n2\nnny\n", b"3\n", b"", 0),
    (b"1\n2\nnnn\n", b"0\n", b"", 0),
    (b"1\n2\ny\n", b"-2\n", b"", 0),
    (b"1\n2\n\n", b"-1\n", b"", 0),
    (b"1\n3\nyyy\n", b"0\n", b"", 0),
    (b"1\n3\nyyn\n", b"152\n", b"", 0),
    (b"1\n3\nyny\n", b"151\n", b"", 0),
    (b"1\n3\nnyy\n", b"150\n", b"", 0),
    (b"1\n3\nynn\n", b"151\n", b"", 0),
    (b"1\n3\nnyn\n", b"150\n", b"", 0),
    (b"1\n3\nnny\n", b"150\n", b"", 0),
    (b"1\n3\nnnn\n", b"200\n", b"", 0),
    (b"1\n3\ny\n", b"-2\n", b"", 0),
    (b"1\n3\n\n", b"-1\n", b"", 0),
    (b"1\n4\nyyy\n", b"-1\n", b"", 0),
    (b"1\n4\nyyn\n", b"-1\n", b"", 0),
    (b"1\n4\nyny\n", b"-1\n", b"", 0),
    (b"1\n4\nnyy\n", b"-1\n", b"", 0),
    (b"1\n4\nynn\n", b"-1\n", b"", 0),
    (b"1\n4\nnyn\n", b"-1\n", b"", 0),
    (b"1\n4\nnny\n", b"-1\n", b"", 0),
    (b"1\n4\nnnn\n", b"-1\n", b"", 0),
    (b"1\n4\ny\n", b"-2\n", b"", 0),
    (b"1\n4\n\n", b"-1\n", b"", 0),
    (b"2\n0\nn\n", b"0\n", b"", 0),
    (b"2\n0\ny\n", b"1001\n", b"", 0),
    (b"2\n0\nnn\n", b"0\n", b"", 0),
    (b"2\n0\nyy\n", b"1002\n", b"", 0),
    (b"2\n0\nyn\n", b"100\n", b"", 0),
    (b"2\n0\nny\n", b"101\n", b"", 0),
    (b"2\n0\nnyn\n", b"101\n", b"", 0),
    (b"2\n0\nyny\n", b"201\n", b"", 0),
    (b"2\n0\nynn\n", b"100\n", b"", 0),
    (b"2\n0\nyyn\n", b"202\n", b"", 0),
    (b"2\n0\nyyy\n", b"1003\n", b"", 0),
    (b"2\n0\nnnn\n", b"0\n", b"", 0),
    (b"2\n0\nynyn\n", b"502\n", b"", 0),
    (b"2\n0\nnyny\n", b"502\n", b"", 0),
    (b"2\n0\nyyyn\n", b"203\n", b"", 0),
    (b"2\n0\nynnn\n", b"100\n", b"", 0),
    (b"2\n0\nyyynnn\n", b"303\n", b"", 0),
    (b"2\n0\nyynyn\n", b"3\n", b"", 0),
    (b"2\n0\nynnyy\n", b"3\n", b"", 0),
    (b"2\n0\nyyynnnyyy\n", b"303\n", b"", 0),
    (b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n", b"1031\n", b"", 0),
    (b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n", b"1032\n", b"", 0),
    (b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n", b"1032\n", b"", 0),
    (b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn\n", b"0\n", b"", 0),
    (b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn\n", b"0\n", b"", 0),
    (b"2\n0\nynynynynynynynynynynynynynynynyn\n", b"516\n", b"", 0),
    (b"2\n0\nynynynynynynynynynynynynynynynynyn\n", b"516\n", b"", 0),
    (b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyynn\n", b"330\n", b"", 0),
    (b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnny\n", b"131\n", b"", 0),
    (b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyn\n", b"231\n", b"", 0),
    (b"2\n0\nyyyyn\n", b"204\n", b"", 0),
    (b"2\n0\nnyyyy\n", b"200\n", b"", 0),
    (b"2\n0\nyynnyy\n", b"4\n", b"", 0),
    (b"3\n0\ny\n", b"1\n", b"", 0),
    (b"3\n0\nn\n", b"-10\n", b"", 0),
    (b"3\n0\nyn\n", b"2\n", b"", 0),
    (b"3\n0\nyy\n", b"-11\n", b"", 0),
    (b"3\n0\nny\n", b"-10\n", b"", 0),
    (b"3\n0\nynn\n", b"11\n", b"", 0),
    (b"3\n0\nyny\n", b"-11\n", b"", 0),
    (b"3\n0\nyyy\n", b"-11\n", b"", 0),
    (b"3\n0\nnnn\n", b"-10\n", b"", 0),
    (b"3\n0\nyyyy\n", b"-11\n", b"", 0),
    (b"3\n0\nynyn\n", b"30\n", b"", 0),
    (b"3\n0\nyynn\n", b"25\n", b"", 0),
    (b"3\n0\nynnn\n", b"25\n", b"", 0),
    (b"3\n0\nyyynnn\n", b"20\n", b"", 0),
    (b"3\n0\nyyyyn\n", b"-12\n", b"", 0),
    (b"3\n0\nynnnn\n", b"-12\n", b"", 0),
    (b"3\n0\nynynynynyn\n", b"30\n", b"", 0),
    (b"3\n0\nynnyynnyn\n", b"30\n", b"", 0),
    (b"3\n0\nyynnyynnyy\n", b"-11\n", b"", 0),
    (b"3\n0\nynynynynynyn\n", b"50\n", b"", 0),
    (b"3\n0\nynnyynnyynn\n", b"45\n", b"", 0),
    (b"3\n0\nyyynnnyyynnn\n", b"45\n", b"", 0),
    (b"3\n0\nynynynynynynynyn\n", b"50\n", b"", 0),
    (b"3\n0\nynnynnynnynnynn\n", b"45\n", b"", 0),
    (b"3\n0\nyynnyynnyynnyynn\n", b"45\n", b"", 0),
    (b"3\n0\nynnnnn\n", b"-12\n", b"", 0),
    (b"4294967296\n0\nyyy\n", b"107\n", b"", 0),
    (b"9223372036854775808\n0\nyyy\n", b"-3\n", b"", 0),
    (b"1\n4294967296\nyyy\n", b"100\n", b"", 0),
    (b"  2 \n0\nynyn\n", b"502\n", b"", 0),
    (b"abc\n0\nyyy\n", b"107\n", b"", 0),
];


/// Every reachable `return` in the C, pinned to the value the C produces.
#[test]
fn golden_values_match_the_c_program() {
    let c = c_binary();
    let rust = rust_binary();
    let mut failures = Vec::new();

    for &(input, want_stdout, want_stderr, want_status) in GOLDEN {
        let from_c = run(c, input);
        // The table is generated from the C, so a mismatch here means the
        // C's behaviour changed, not that the translation is wrong.
        if from_c.stdout != want_stdout
            || from_c.stderr != want_stderr
            || from_c.status != Some(want_status)
        {
            failures.push(format!(
                "C program drifted from the recorded table for {}\n  recorded: {:?} / {:?} / {}\n  actual  : {:?}",
                describe(input),
                String::from_utf8_lossy(want_stdout),
                String::from_utf8_lossy(want_stderr),
                want_status,
                from_c
            ));
            continue;
        }

        let from_rust = run(rust, input);
        if from_rust.stdout != want_stdout {
            failures.push(format!(
                "{}: stdout was {:?}, expected {:?}",
                describe(input),
                String::from_utf8_lossy(&from_rust.stdout),
                String::from_utf8_lossy(want_stdout)
            ));
        }
        if from_rust.stderr != want_stderr {
            failures.push(format!(
                "{}: stderr was {:?}, expected {:?}",
                describe(input),
                String::from_utf8_lossy(&from_rust.stderr),
                String::from_utf8_lossy(want_stderr)
            ));
        }
        if from_rust.status != Some(want_status) {
            failures.push(format!(
                "{}: exit status was {:?}, expected {}",
                describe(input),
                from_rust.status,
                want_status
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} golden expectations failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The table above is the coverage claim; assert it still spans every
/// distinct result value the C is able to print.
#[test]
fn golden_table_spans_the_expected_result_values() {
    let mut values: Vec<i64> = GOLDEN
        .iter()
        .filter_map(|(_, stdout, _, _)| {
            std::str::from_utf8(stdout).ok()?.trim().parse::<i64>().ok()
        })
        .collect();
    values.sort_unstable();
    values.dedup();

    // Sanity: the interesting return values from each C function.
    let required = [
        -3, -2, -1,          // process_decisions guards and default arm
        -20, -10, 0, 14, 23, 35, 56, 107, // apply_permissions
        10, 11, 12, 50, 51, 52, 100,      // evaluate_conditions, AND
        101, 102, 103,                    // evaluate_conditions, OR
        1, 2, 3, 7,                       // evaluate_conditions, XOR
        150, 151, 152, 200,               // evaluate_conditions, NAND
        -12, -11,                         // validate_sequence rules
        20, 25, 30, 45,                   // validate_sequence bands
    ];
    for want in required {
        assert!(
            values.contains(&want),
            "the golden table no longer reaches the C result {want}; values present: {values:?}"
        );
    }
    // configure_flags families: 100+i, 200+i, 300+n, 500+n, 1000+count.
    for (lo, hi, what) in [
        (100, 132, "configure_flags 100 + index"),
        (200, 232, "configure_flags 200 + index"),
        (300, 332, "configure_flags 300 + max_consecutive"),
        (500, 532, "configure_flags 500 + special_count"),
        (1000, 1032, "configure_flags 1000 + count"),
    ] {
        assert!(
            values.iter().any(|v| *v >= lo && *v <= hi),
            "the golden table no longer reaches {what}; values present: {values:?}"
        );
    }
}

/// A tiny deterministic generator, so every random case is reproducible.
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Structured random inputs: plausible operations and parameters with
/// random decision strings of varying length.
#[test]
fn random_structured_inputs() {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let ops = [
        "0", "1", "2", "3", "4", "-1", "-3", "99", "2147483648", "4294967296",
        "9223372036854775808", "abc", "", " 2", "3junk",
    ];
    let params = ["0", "1", "2", "3", "4", "-1", "99", "", "abc", "2147483648"];
    let alphabet = [b'y', b'n', b'Y', b'N', b'x', b'?', b'\t', b'0'];

    let mut inputs = Vec::with_capacity(6000);
    for _ in 0..6000 {
        let len = rng.below(45);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(&alphabet)).collect();
        let mut input = Vec::new();
        input.extend_from_slice(rng.pick(&ops).as_bytes());
        input.push(b'\n');
        input.extend_from_slice(rng.pick(&params).as_bytes());
        input.push(b'\n');
        input.extend_from_slice(&s);
        if rng.below(10) != 0 {
            input.push(b'\n');
        }
        inputs.push(input);
    }
    compare_all("random_structured_inputs", inputs);
}

/// Arbitrary bytes, including short and truncated streams, to shake out any
/// difference in the read paths or in how non-`y`/`n` bytes are handled.
#[test]
fn random_arbitrary_bytes() {
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let mut inputs = Vec::with_capacity(5000);

    // Completely arbitrary byte streams.
    for _ in 0..2500 {
        let len = rng.below(60);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xff) as u8).collect();
        inputs.push(bytes);
    }

    // Arbitrary bytes, but arranged into three newline-separated lines.
    for _ in 0..2500 {
        let mut input = Vec::new();
        for line in 0..3 {
            let len = rng.below(25);
            for _ in 0..len {
                let mut b = (rng.next_u32() & 0xff) as u8;
                if b == b'\n' {
                    b = b'z';
                }
                input.push(b);
            }
            if line < 2 || rng.below(3) != 0 {
                input.push(b'\n');
            }
        }
        inputs.push(input);
    }

    compare_all("random_arbitrary_bytes", inputs);
}

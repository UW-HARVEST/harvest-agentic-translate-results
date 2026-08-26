// Phase C — error-path differential tests: one test per row of ERRORS.md.
//
// Each row constructs the exact invalid input/condition, calls the exported
// `main` of BOTH shared objects and asserts that the return value (the C error
// code, 1 vs 0) and the exact stdout bytes (the specific error message) agree.
//
// stdout capture is process wide, so all rows run from a single `#[test]`.

mod common;

use common::*;
use std::ffi::c_int;

const ERR_ARGC: &[u8] = b"Error: should only be two (integer) arguments!\n";
const ERR_ARG1: &[u8] = b"Error: first argument must be an integer!\n";
const ERR_ARG2: &[u8] = b"Error: second argument must be an integer!\n";

struct Harness {
    failures: Vec<String>,
    rows: Vec<(&'static str, usize)>,
    calls: usize,
}

impl Harness {
    fn new() -> Harness {
        Harness {
            failures: Vec::new(),
            rows: Vec::new(),
            calls: 0,
        }
    }

    fn row(&mut self, name: &'static str) {
        self.rows.push((name, self.failures.len()));
    }

    /// Compares `main(argc, argv)` between both images. When `expect` is given,
    /// it additionally pins the C behaviour (result code + stdout) so the row
    /// asserts the *specific* error, not just "both did the same thing".
    fn check(
        &mut self,
        row: &str,
        argc: c_int,
        args: &[&[u8]],
        expect: Option<(c_int, &[u8])>,
    ) -> (c_int, Vec<u8>) {
        self.calls += 1;
        let pair = load_pair(&format!("e{}", self.calls));
        let mut argv = Argv::new(args);
        let (crc, cout) = call_main(&pair.c, argc, &mut argv);
        let (rrc, rout) = call_main(&pair.rust, argc, &mut argv);
        let pretty: Vec<String> = args
            .iter()
            .map(|a| {
                let s = String::from_utf8_lossy(a).into_owned();
                if s.len() > 40 {
                    format!("{}...<{} bytes>", &s[..20], a.len())
                } else {
                    s
                }
            })
            .collect();
        if crc != rrc || cout != rout {
            self.failures.push(format!(
                "[{row}] argc={argc} argv={pretty:?}\n     C   : rc={crc} out={:?}\n     Rust: rc={rrc} out={:?}",
                show(&cout),
                show(&rout)
            ));
        }
        if let Some((erc, eout)) = expect {
            if crc != erc || cout != eout {
                self.failures.push(format!(
                    "[{row}] argc={argc} argv={pretty:?}: C behaviour is not the expected one\n     expected: rc={erc} out={:?}\n     C got   : rc={crc} out={:?}",
                    show(eout),
                    show(&cout)
                ));
            }
        }
        (crc, cout)
    }

    fn finish(self) {
        eprintln!("ffi_errors: {} rows, {} call pairs", self.rows.len(), self.calls);
        for (name, before) in &self.rows {
            let mark = if self.failures.len() > *before { "FAIL" } else { "ok" };
            eprintln!("  {name:<38} {mark}");
        }
        if !self.failures.is_empty() {
            panic!(
                "{} divergence(s)/mismatch(es):\n{}",
                self.failures.len(),
                self.failures.join("\n")
            );
        }
    }
}

/// Strings for which `strtol(s, &end, 10)` performs no conversion at all
/// (`end == s`), i.e. every distinct way of triggering ERRORS.md rows 2 and 3.
const UNPARSABLE: &[&[u8]] = &[
    b"",
    b" ",
    b"   ",
    b"\t",
    b"\n",
    b"\x0b",
    b"\x0c",
    b"\r",
    b" \t\n\x0b\x0c\r",
    b"+",
    b"-",
    b"+ 1",
    b"- 1",
    b"--1",
    b"++1",
    b"+-1",
    b"abc",
    b"A",
    b".5",
    b"x10",
    b"e5",
    b",",
    b"O",
    b"o",
    b"#",
    b"/",
    b":",
    b"\x80\xff",
    b"\xc2\xb3",     // superscript three (UTF-8)
    b"\xd9\xa3",     // ARABIC-INDIC DIGIT THREE (UTF-8)
    b" - 3",
    b"\tzero",
    b"one",
    b"NULL",
    b"0.0e0x",       // parses "0" -> NOT unparsable; kept out below
];

fn unparsable_strings() -> Vec<&'static [u8]> {
    // drop the last entry: "0.0e0x" does parse a leading 0
    UNPARSABLE[..UNPARSABLE.len() - 1].to_vec()
}

/// Random strings from an alphabet that contains no digit, so `strtol` can never
/// perform a conversion whatever the random composition is.
fn random_unparsable(rng: &mut Rng) -> Vec<u8> {
    const POOL: &[u8] = b" \t\n\x0b\x0c\r+-.,:;/eExXaAoO#_'\"\x80\xa0\xff";
    let len = 1 + rng.below(8);
    (0..len)
        .map(|_| POOL[rng.below(POOL.len() as u64) as usize])
        .collect()
}

/// A random magnitude that is guaranteed to overflow `long` (20..=40 digits, no
/// leading zero), optionally negative.
fn random_saturating(rng: &mut Rng, negative: bool) -> Vec<u8> {
    let mut s = Vec::new();
    if negative {
        s.push(b'-');
    }
    s.push(b'1' + rng.below(9) as u8);
    for _ in 0..(19 + rng.below(21)) {
        s.push(b'0' + rng.below(10) as u8);
    }
    s
}

#[test]
fn error_surface_differential() {
    let mut h = Harness::new();

    // ---------------------------------------------------------------- ERRORS #1
    // argc != 3
    h.row("#1 argc != 3");
    for argc in [0, 1, 2, 4, 5, 17] {
        // argv holds as many real entries as argc where possible
        let all: Vec<&[u8]> = vec![
            b"driver".as_slice(),
            b"7".as_slice(),
            b"3".as_slice(),
            b"extra".as_slice(),
            b"more".as_slice(),
        ];
        let n = (argc as usize).min(all.len());
        h.check("#1 argc != 3", argc, &all[..n], Some((1, ERR_ARGC)));
    }

    // ------------------------------------------------------------ ERRORS #1/#16
    // negative / wild argc values coming across the FFI boundary
    h.row("#1 negative argc");
    for argc in [-1, -2, i32::MIN, i32::MAX, 1 << 20] {
        h.check(
            "#1 negative argc",
            argc,
            &[b"driver".as_slice(), b"7".as_slice(), b"3".as_slice()],
            Some((1, ERR_ARGC)),
        );
    }

    // --------------------------------------------------------------- ERRORS #12
    // argc == 0 with argv[0] == NULL (empty argv array)
    h.row("#12 argc == 0, argv[0] == NULL");
    h.check(
        "#12 argc == 0, argv[0] == NULL",
        0,
        &[] as &[&[u8]],
        Some((1, ERR_ARGC)),
    );

    // ---------------------------------------------------------------- ERRORS #2
    // strtol performs no conversion on argv[1]
    h.row("#2 argv[1] unparsable");
    for s in unparsable_strings() {
        h.check(
            "#2 argv[1] unparsable",
            3,
            &[b"driver".as_slice(), s, b"3".as_slice()],
            Some((1, ERR_ARG1)),
        );
    }
    // argv[2] is not even looked at: an equally invalid argv[2] must still give
    // the *first* argument's message
    h.check(
        "#2 argv[1] unparsable",
        3,
        &[b"driver".as_slice(), b"".as_slice(), b"".as_slice()],
        Some((1, ERR_ARG1)),
    );
    h.check(
        "#2 argv[1] unparsable",
        3,
        &[b"driver".as_slice(), b"junk".as_slice(), b"junk".as_slice()],
        Some((1, ERR_ARG1)),
    );
    // randomized digit-free strings
    {
        let mut rng = Rng::new(0x2002_0002);
        for _ in 0..40 {
            let s = random_unparsable(&mut rng);
            h.check(
                "#2 argv[1] unparsable",
                3,
                &[b"driver".as_slice(), &s, b"3".as_slice()],
                Some((1, ERR_ARG1)),
            );
        }
    }

    // ---------------------------------------------------------------- ERRORS #3
    // strtol performs no conversion on argv[2] while argv[1] is valid
    h.row("#3 argv[2] unparsable");
    for s in unparsable_strings() {
        h.check(
            "#3 argv[2] unparsable",
            3,
            &[b"driver".as_slice(), b"7".as_slice(), s],
            Some((1, ERR_ARG2)),
        );
    }
    // randomized digit-free strings, with randomized (valid) argv[1]
    {
        let mut rng = Rng::new(0x3003_0003);
        for _ in 0..40 {
            let s = random_unparsable(&mut rng);
            let v = rng.next_i32().to_string();
            h.check(
                "#3 argv[2] unparsable",
                3,
                &[b"driver".as_slice(), v.as_bytes(), &s],
                Some((1, ERR_ARG2)),
            );
        }
    }

    // ---------------------------------------------------------------- ERRORS #4
    // iterations <= 0: valid input, empty loop, no output, rc 0
    h.row("#4 iterations <= 0");
    for s in [
        b"0".as_slice(),
        b"-0".as_slice(),
        b"+0".as_slice(),
        b"-1".as_slice(),
        b"-2".as_slice(),
        b"-2147483648".as_slice(),
        b"-99999999999999999999".as_slice(),
        b"-0000".as_slice(),
    ] {
        h.check(
            "#4 iterations <= 0",
            3,
            &[b"driver".as_slice(), b"7".as_slice(), s],
            Some((0, b"")),
        );
    }

    // ------------------------------------------------------------- ERRORS #5/#6
    // strtol saturation on argv[1]: LONG_MAX -> int -1, LONG_MIN -> int 0.
    h.row("#5/#6 argv[1] strtol saturation");
    let big = "9".repeat(400);
    let big_neg = format!("-{}", "9".repeat(400));
    let padded = format!("+1{}", "0".repeat(300));
    for (s, expect_first) in [
        ("9223372036854775808", -1i32),
        ("99999999999999999999", -1),
        (big.as_str(), -1),
        (padded.as_str(), -1),
        ("-9223372036854775809", 0),
        ("-99999999999999999999", 0),
        (big_neg.as_str(), 0),
    ] {
        // one iteration: prints the outcome of the very first static_alias call,
        // which reveals the converted initial value
        let want = if expect_first >= 1 {
            format!("{}\n", (expect_first as i64) + 1)
        } else {
            format!("{}\n", expect_first + 1)
        };
        h.check(
            "#5/#6 argv[1] strtol saturation",
            3,
            &[b"driver".as_slice(), s.as_bytes(), b"1".as_slice()],
            Some((0, want.as_bytes())),
        );
    }

    // randomized saturating magnitudes on argv[1]
    {
        let mut rng = Rng::new(0x5005_0005);
        for _ in 0..25 {
            // positive: LONG_MAX -> int -1 -> else branch -> prints 0
            let s = random_saturating(&mut rng, false);
            h.check(
                "#5/#6 argv[1] strtol saturation",
                3,
                &[b"driver".as_slice(), &s, b"1".as_slice()],
                Some((0, b"0\n")),
            );
            // negative: LONG_MIN -> int 0 -> else branch -> prints 1
            let s = random_saturating(&mut rng, true);
            h.check(
                "#5/#6 argv[1] strtol saturation",
                3,
                &[b"driver".as_slice(), &s, b"1".as_slice()],
                Some((0, b"1\n")),
            );
        }
    }

    // ---------------------------------------------------------------- ERRORS #7
    // saturation on argv[2]: LONG_MAX -> -1 and LONG_MIN -> 0 iterations.
    h.row("#7 argv[2] strtol saturation");
    for s in [
        "9223372036854775808",
        "99999999999999999999",
        big.as_str(),
        "-9223372036854775809",
        "-99999999999999999999",
        big_neg.as_str(),
        "9223372036854775807",
        "-9223372036854775808",
    ] {
        h.check(
            "#7 argv[2] strtol saturation",
            3,
            &[b"driver".as_slice(), b"7".as_slice(), s.as_bytes()],
            Some((0, b"")),
        );
    }

    // randomized saturating magnitudes on argv[2]: LONG_MAX -> -1 and
    // LONG_MIN -> 0 iterations, so the loop never runs.
    {
        let mut rng = Rng::new(0x7007_0007);
        for _ in 0..25 {
            let negative = rng.bool();
            let s = random_saturating(&mut rng, negative);
            let v = rng.next_i32().to_string();
            h.check(
                "#7 argv[2] strtol saturation",
                3,
                &[b"driver".as_slice(), v.as_bytes(), &s],
                Some((0, b"")),
            );
        }
    }

    // ---------------------------------------------------------------- ERRORS #8
    // one step past the int boundaries: implicit long -> int truncation.
    h.row("#8 long -> int narrowing");
    for (s, narrowed) in [
        ("2147483648", i32::MIN),
        ("-2147483649", i32::MAX),
        ("4294967296", 0),
        ("4294967295", -1),
        ("4294967297", 1),
        ("-4294967296", 0),
        ("2147483647", i32::MAX),
        ("-2147483648", i32::MIN),
    ] {
        // inner == 1: then branch iff narrowed >= 1 (prints 1 + narrowed),
        // else branch (prints narrowed + 1). Both are `narrowed + 1` modulo 2^32.
        let want = format!("{}\n", narrowed.wrapping_add(1));
        h.check(
            "#8 long -> int narrowing",
            3,
            &[b"driver".as_slice(), s.as_bytes(), b"1".as_slice()],
            Some((0, want.as_bytes())),
        );
    }

    // randomized values outside int range but inside long range: the narrowed
    // value is `value mod 2^32`, and one iteration prints `narrowed + 1`.
    {
        let mut rng = Rng::new(0x8008_0008);
        for _ in 0..30 {
            let high = rng.range_i32(1, 1 << 20) as i64;
            let low = rng.next_u64() as u32 as i64;
            let value = if rng.bool() {
                high * (1i64 << 32) + low
            } else {
                -(high * (1i64 << 32) + low)
            };
            let narrowed = value as i32;
            let want = format!("{}\n", narrowed.wrapping_add(1));
            h.check(
                "#8 long -> int narrowing",
                3,
                &[
                    b"driver".as_slice(),
                    value.to_string().as_bytes(),
                    b"1".as_slice(),
                ],
                Some((0, want.as_bytes())),
            );
        }
    }

    // ---------------------------------------------------------------- ERRORS #9
    // trailing garbage after a valid prefix is accepted
    h.row("#9 trailing garbage accepted");
    for (s, prefix) in [
        ("12abc", 12i32),
        ("5 5", 5),
        ("0x10", 0),
        ("1.9", 1),
        ("-3junk", -3),
        ("08", 8),
        ("1e5", 1),
        ("7\t", 7),
        ("+9-", 9),
        ("0.0e0x", 0),
    ] {
        let want = format!("{}\n", prefix.wrapping_add(1));
        h.check(
            "#9 trailing garbage accepted",
            3,
            &[b"driver".as_slice(), s.as_bytes(), b"1".as_slice()],
            Some((0, want.as_bytes())),
        );
    }

    // --------------------------------------------------------------- ERRORS #10
    // empty-string combinations
    h.row("#10 empty string combinations");
    h.check(
        "#10 empty string combinations",
        3,
        &[b"driver".as_slice(), b"".as_slice(), b"3".as_slice()],
        Some((1, ERR_ARG1)),
    );
    h.check(
        "#10 empty string combinations",
        3,
        &[b"driver".as_slice(), b"3".as_slice(), b"".as_slice()],
        Some((1, ERR_ARG2)),
    );
    h.check(
        "#10 empty string combinations",
        3,
        &[b"".as_slice(), b"".as_slice(), b"".as_slice()],
        Some((1, ERR_ARG1)),
    );

    // --------------------------------------------------------------- ERRORS #11
    // oversized argument strings
    h.row("#11 oversized strings");
    let digits4k = "1".repeat(4096);
    let blanks4k = format!("{}{}", " ".repeat(4096), "42");
    let zeros4k = format!("{}7", "0".repeat(4096));
    let digits64k = "9".repeat(65536);
    let zeros64k = format!("{}{}", "0".repeat(65536), "12");
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), digits4k.as_bytes(), b"1".as_slice()],
        Some((0, b"0\n")), // saturates to LONG_MAX -> int -1 -> else branch -> 0
    );
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), blanks4k.as_bytes(), b"1".as_slice()],
        Some((0, b"43\n")),
    );
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), zeros4k.as_bytes(), b"1".as_slice()],
        Some((0, b"8\n")),
    );
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), digits64k.as_bytes(), b"1".as_slice()],
        Some((0, b"0\n")),
    );
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), zeros64k.as_bytes(), b"2".as_slice()],
        Some((0, b"13\n26\n")),
    );
    // oversized on argv[2] as well
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), b"5".as_slice(), zeros4k.as_bytes()],
        Some((0, b"6\n12\n24\n48\n96\n192\n384\n")),
    );
    let blanks_only = " ".repeat(4096);
    h.check(
        "#11 oversized strings",
        3,
        &[b"driver".as_slice(), blanks_only.as_bytes(), b"1".as_slice()],
        Some((1, ERR_ARG1)),
    );

    // --------------------------------------------------------------- ERRORS #13
    // argc == 3 with extra argv entries: they are ignored
    h.row("#13 extra argv entries ignored");
    h.check(
        "#13 extra argv entries ignored",
        3,
        &[
            b"driver".as_slice(),
            b"4".as_slice(),
            b"2".as_slice(),
            b"ignored".as_slice(),
            b"".as_slice(),
        ],
        Some((0, b"5\n10\n")),
    );

    // ------------------------------------------------------------ ERRORS #14-16
    // `static_alias` has no rejection path; its boundaries are checked here as
    // well (and exhaustively in tests/ffi_static_alias.rs).
    h.row("#14-16 static_alias boundaries");
    {
        let pair = load_pair("e_alias_boundaries");
        for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            let mut cv = v;
            let mut rv = v;
            let cret = unsafe { (pair.c.static_alias)(&mut cv) };
            let rret = unsafe { (pair.rust.static_alias)(&mut rv) };
            let (cval, rval) = unsafe { (*cret, *rret) };
            let cown = cret == &mut cv as *mut i32;
            let rown = rret == &mut rv as *mut i32;
            assert!(!cret.is_null() && !rret.is_null(), "static_alias never returns NULL");
            if cval != rval || cv != rv || cown != rown {
                h.failures.push(format!(
                    "[#14-16 static_alias boundaries] *outer={v}: C(*ret={cval}, cell={cv}, own={cown}) vs Rust(*ret={rval}, cell={rv}, own={rown})"
                ));
            }
        }
    }

    h.finish();
}

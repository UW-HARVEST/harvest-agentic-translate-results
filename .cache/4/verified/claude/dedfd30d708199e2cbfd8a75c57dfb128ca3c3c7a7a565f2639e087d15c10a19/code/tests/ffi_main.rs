// Phase B — differential tests for the exported `int main(int, char**)` of the
// C translation unit, called directly through the `.so` exports (CONFIGS.md
// rows 10-20).
//
// stdout is captured at the file-descriptor level, which is a process-wide
// operation, so this binary runs every scenario from a single `#[test]` (test
// binaries are separate processes, so they cannot interfere with each other).

mod common;

use common::*;
use std::ffi::c_int;

struct Harness {
    failures: Vec<String>,
    cases: usize,
    calls: usize,
}

impl Harness {
    fn new() -> Harness {
        Harness {
            failures: Vec::new(),
            cases: 0,
            calls: 0,
        }
    }

    /// Calls `main` in both images with the identical `argc`/`argv` and compares
    /// the return value and every byte written to stdout.
    fn check(&mut self, pair: &Pair, tag: &str, argc: c_int, args: &[&[u8]]) {
        self.calls += 1;
        let mut argv = Argv::new(args);
        let (crc, cout) = call_main(&pair.c, argc, &mut argv);
        let (rrc, rout) = call_main(&pair.rust, argc, &mut argv);
        if crc != rrc || cout != rout {
            let pretty: Vec<String> = args
                .iter()
                .map(|a| String::from_utf8_lossy(a).into_owned())
                .collect();
            self.failures.push(format!(
                "[{tag}] argc={argc} argv={pretty:?}\n     C   : rc={crc} out={:?}\n     Rust: rc={rrc} out={:?}",
                show(&cout),
                show(&rout)
            ));
        }
    }

    /// Fresh image (pristine `inner == 1`) per case.
    fn check_fresh(&mut self, tag: &str, args: &[&[u8]]) {
        self.cases += 1;
        let pair = load_pair(&format!("m{}", self.cases));
        let argc = args.len() as c_int;
        self.check(&pair, tag, argc, args);
    }

    fn finish(self) {
        eprintln!(
            "ffi_main: {} scenarios, {} main() call pairs compared",
            self.cases, self.calls
        );
        if !self.failures.is_empty() {
            panic!(
                "{} divergence(s):\n{}",
                self.failures.len(),
                self.failures.join("\n")
            );
        }
    }
}

fn bs(s: &str) -> &[u8] {
    s.as_bytes()
}

#[test]
fn main_entry_point_differential() {
    let mut h = Harness::new();

    // ------------------------------------------------------------- CONFIGS #10
    // argc == 3, iterations == 1, random initial value: a single static_alias
    // call, both branches.
    {
        let mut rng = Rng::new(0x1234_5678);
        let mut vals: Vec<String> = vec![
            "0".into(),
            "1".into(),
            "-1".into(),
            "2".into(),
            "-2".into(),
            i32::MAX.to_string(),
            i32::MIN.to_string(),
        ];
        for _ in 0..30 {
            vals.push(rng.next_i32().to_string());
        }
        for v in &vals {
            h.check_fresh("cfg10_single_iteration", &[bs("driver"), bs(v), bs("1")]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #11
    // iterations == 0: valid, empty loop, no output.
    for v in ["0", "1", "-1", "2147483647", "-2147483648"] {
        h.check_fresh("cfg11_iterations_zero", &[bs("driver"), bs(v), bs("0")]);
    }
    h.check_fresh("cfg11_iterations_zero", &[bs("driver"), bs("5"), bs("-0")]);
    h.check_fresh("cfg11_iterations_zero", &[bs("driver"), bs("5"), bs("+0")]);
    {
        let mut rng = Rng::new(0x0B0B_0011);
        for _ in 0..10 {
            let v = rng.next_i32().to_string();
            h.check_fresh("cfg11_iterations_zero", &[bs("driver"), bs(&v), bs("0")]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #12
    // initial_value >= inner on the first iteration (then branch first), then
    // the chained doubling, including its signed overflow (>= 32 iterations).
    for iters in ["2", "3", "5", "10", "31", "32", "33", "40", "64"] {
        for v in ["1", "2", "7", "1073741824", "2147483647"] {
            h.check_fresh("cfg12_then_first", &[bs("driver"), bs(v), bs(iters)]);
        }
    }
    {
        // randomized: initial_value >= 1 (then branch first) with a random count
        let mut rng = Rng::new(0x0C0C_0012);
        for _ in 0..25 {
            let v = rng.range_i32(1, i32::MAX).to_string();
            let iters = rng.range_i32(2, 70).to_string();
            h.check_fresh("cfg12_then_first", &[bs("driver"), bs(&v), bs(&iters)]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #13
    // initial_value < inner on the first iteration (else branch first): the
    // caller's local grows by inner until it catches up.
    for v in ["0", "-1", "-5", "-100", "-2147483648"] {
        for iters in ["1", "3", "10", "120"] {
            h.check_fresh("cfg13_else_first", &[bs("driver"), bs(v), bs(iters)]);
        }
    }
    {
        // randomized: initial_value <= 0 (else branch first) with a random count
        let mut rng = Rng::new(0x0D0D_0013);
        for _ in 0..25 {
            let v = rng.range_i32(i32::MIN, 0).to_string();
            let iters = rng.range_i32(1, 130).to_string();
            h.check_fresh("cfg13_else_first", &[bs("driver"), bs(&v), bs(&iters)]);
        }
        // small negative values, where the else branch repeats several times
        for _ in 0..15 {
            let v = rng.range_i32(-130, 0).to_string();
            let iters = rng.range_i32(1, 200).to_string();
            h.check_fresh("cfg13_else_first", &[bs("driver"), bs(&v), bs(&iters)]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #14
    // boundary initial value x iteration count.
    for v in ["2147483647", "-2147483648", "-1", "0", "1"] {
        for iters in ["1", "2", "5", "33"] {
            h.check_fresh("cfg14_boundary_matrix", &[bs("driver"), bs(v), bs(iters)]);
        }
    }
    {
        // randomized neighbourhoods of the int / long boundaries for argv[1]
        let mut rng = Rng::new(0x0E0E_0014);
        for _ in 0..30 {
            let d = rng.range_i32(-64, 64) as i64;
            let base = match rng.below(6) {
                0 => i32::MAX as i64,
                1 => i32::MIN as i64,
                2 => 1i64 << 32,
                3 => -(1i64 << 32),
                4 => i64::MAX - 64,
                _ => i64::MIN + 64,
            };
            let v = base.wrapping_add(d).to_string();
            let iters = rng.range_i32(1, 40).to_string();
            h.check_fresh("cfg14_boundary_matrix", &[bs("driver"), bs(&v), bs(&iters)]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #15
    // argv[1] string-shape sweep (strtol semantics), fixed valid argv[2].
    let arg1_shapes: &[&str] = &[
        "0",
        "1",
        "-1",
        "+1",
        "007",
        "-007",
        " 12",
        "\t13",
        "\n14",
        "\u{b}15",
        "\u{c}16",
        "\r17",
        " \t\n\u{b}\u{c}\r-18",
        "12abc",
        "-3junk",
        "5 5",
        "0x10",
        "1.9",
        "1e5",
        "08",
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999",
        "-99999999999999999999",
        "+0000000000000000000000000000009",
    ];
    for s in arg1_shapes {
        for iters in ["1", "4", "35"] {
            h.check_fresh("cfg15_arg1_shapes", &[bs("driver"), bs(s), bs(iters)]);
        }
    }
    {
        // The same shapes with randomized digit content: a random prefix from the
        // shape alphabet, random digits, and a random trailing-garbage suffix.
        let mut rng = Rng::new(0x0F0F_0015);
        const PREFIX: &[&str] = &["", " ", "  ", "\t", "\n", "\u{b}", "\u{c}", "\r", "+", "-", " -", "+0", "000"];
        const SUFFIX: &[&str] = &["", "abc", ".5", "e9", " 7", "x10", "-", "+"];
        for _ in 0..60 {
            let mut s = String::from(PREFIX[rng.below(PREFIX.len() as u64) as usize]);
            let ndigits = 1 + rng.below(22);
            for _ in 0..ndigits {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
            s.push_str(SUFFIX[rng.below(SUFFIX.len() as u64) as usize]);
            let iters = rng.range_i32(1, 40).to_string();
            h.check_fresh("cfg15_arg1_shapes", &[bs("driver"), bs(&s), bs(&iters)]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #16
    // argv[2] string-shape sweep. The predicted iteration count is noted for
    // each; shapes whose narrowed value would be a huge positive number are
    // deliberately excluded (they would need 2^31 printf calls -- see
    // ERRORS.md, "Undefined behaviour / untestable").
    let arg2_shapes: &[&str] = &[
        "0",            // 0
        "1",            // 1
        "2",            // 2
        "+3",           // 3
        " \t\n\u{b}\u{c}\r5", // 5
        "007",          // 7
        "12abc",        // 12
        "-0",           // 0
        "-1",           // none
        "1.9",          // 1
        "1e5",          // 1
        "0x10",         // 0
        "2147483648",   // INT_MIN -> none
        "-2147483648",  // INT_MIN -> none
        "4294967296",   // 0
        "4294967297",   // 1
        "4294967300",   // 4
        "8589934596",   // 4
        "-4294967292",  // 4
        "9223372036854775807",  // -1 -> none
        "9223372036854775808",  // saturates -> -1 -> none
        "-9223372036854775808", // 0 -> none
        "-9223372036854775809", // saturates -> 0 -> none
        "99999999999999999999", // -1 -> none
        "-99999999999999999999", // 0 -> none
        "  -3junk",     // none
    ];
    for s in arg2_shapes {
        for v in ["1", "-7", "2147483647"] {
            h.check_fresh("cfg16_arg2_shapes", &[bs("driver"), bs(v), bs(s)]);
        }
    }
    {
        // Randomized argv[2] shapes: a random prefix from the shape alphabet plus
        // a small random count (kept small so the iteration count stays bounded),
        // combined with a randomized argv[1].
        let mut rng = Rng::new(0x1010_0016);
        const PREFIX: &[&str] = &["", " ", "\t", "\n", "\u{b}", "\u{c}", "\r", "+", "-", "00", "+0", "-0"];
        const SUFFIX: &[&str] = &["", "abc", ".5", "e9", " 1", "x2"];
        for _ in 0..60 {
            let mut s = String::from(PREFIX[rng.below(PREFIX.len() as u64) as usize]);
            s.push_str(&rng.below(200).to_string());
            s.push_str(SUFFIX[rng.below(SUFFIX.len() as u64) as usize]);
            let v = rng.next_i32().to_string();
            h.check_fresh("cfg16_arg2_shapes", &[bs("driver"), bs(&v), bs(&s)]);
        }
    }

    // ------------------------------------------------------------- CONFIGS #17
    // randomized (argv[1], argv[2]) pairs from a shape generator.
    {
        let mut rng = Rng::new(0x0FED_CBA9);
        for i in 0..200 {
            let a1 = random_int_string(&mut rng);
            let a2 = random_small_count_string(&mut rng);
            h.check_fresh(
                &format!("cfg17_random_pair{i}"),
                &[bs("driver"), bs(&a1), bs(&a2)],
            );
        }
    }

    // ------------------------------------------------------------- CONFIGS #18
    // Repeated main() calls in one loaded image: `inner` carries over, so the
    // same arguments must produce different (but identical between C and Rust)
    // output each time.
    {
        h.cases += 1;
        let pair = load_pair("m_repeat");
        for round in 0..6 {
            h.check(
                &pair,
                &format!("cfg18_repeat_round{round}"),
                3,
                &[bs("driver"), bs("3"), bs("4")],
            );
        }
        for round in 0..4 {
            h.check(
                &pair,
                &format!("cfg18_repeat_neg{round}"),
                3,
                &[bs("driver"), bs("-9"), bs("3")],
            );
        }
        // error paths in between must not disturb the shared state
        h.check(&pair, "cfg18_repeat_err", 1, &[bs("driver")]);
        h.check(&pair, "cfg18_repeat_err2", 3, &[bs("driver"), bs("x"), bs("2")]);
        h.check(&pair, "cfg18_repeat_after_err", 3, &[bs("driver"), bs("3"), bs("4")]);
    }
    {
        // randomized argument sequences against a single shared image
        h.cases += 1;
        let pair = load_pair("m_repeat_rand");
        let mut rng = Rng::new(0x1212_0018);
        for round in 0..40 {
            let v = rng.next_i32().to_string();
            let n = rng.range_i32(0, 20).to_string();
            h.check(
                &pair,
                &format!("cfg18_repeat_rand{round}"),
                3,
                &[bs("driver"), bs(&v), bs(&n)],
            );
        }
    }

    // ------------------------------------------------------------- CONFIGS #19
    // `main` and `static_alias` interleaved on the same image (shared state).
    {
        h.cases += 1;
        let pair = load_pair("m_interleaved");
        let mut rng = Rng::new(0x5151_5151);
        for step in 0..40 {
            if rng.bool() {
                let mut cv: i32 = rng.range_i32(-50, 50);
                let mut rv: i32 = cv;
                let cret = unsafe { (pair.c.static_alias)(&mut cv) };
                let rret = unsafe { (pair.rust.static_alias)(&mut rv) };
                let (cval, rval) = unsafe { (*cret, *rret) };
                let cid = cret == &mut cv as *mut i32;
                let rid = rret == &mut rv as *mut i32;
                if cval != rval || cv != rv || cid != rid {
                    h.failures.push(format!(
                        "[cfg19_interleaved step {step}] static_alias: C(*ret={cval}, cell={cv}, own={cid}) vs Rust(*ret={rval}, cell={rv}, own={rid})"
                    ));
                }
            } else {
                let v = rng.range_i32(-20, 20).to_string();
                let n = rng.range_i32(0, 6).to_string();
                h.check(
                    &pair,
                    &format!("cfg19_interleaved_main{step}"),
                    3,
                    &[bs("driver"), bs(&v), bs(&n)],
                );
            }
        }
    }

    // ------------------------------------------------------------- CONFIGS #20
    // argv[0] is never looked at; entries past index 2 are ignored.
    h.check_fresh("cfg20_argv0", &[bs(""), bs("4"), bs("3")]);
    h.check_fresh("cfg20_argv0", &[bs("/some/other/name"), bs("4"), bs("3")]);
    {
        h.cases += 1;
        let pair = load_pair("m_extra_argv");
        // argc == 3 but the array holds more entries
        h.check(
            &pair,
            "cfg20_extra_argv",
            3,
            &[bs("driver"), bs("4"), bs("3"), bs("ignored"), bs("also-ignored")],
        );
    }

    // ------------------------------------------------------------- CONFIGS #25
    // Random byte strings for argv[1] built from the alphabet strtol reacts to
    // (digits, all six C-locale space characters, signs, letters, punctuation and
    // non-ASCII bytes). Compares the parse/no-parse decision AND the converted
    // value (visible through the single iteration's output).
    {
        let mut rng = Rng::new(0x2222_3333);
        for i in 0..400 {
            let s = random_byte_string(&mut rng, 12);
            h.check_fresh(
                &format!("cfg25_random_bytes1_{i}"),
                &[b"driver".as_slice(), &s, b"1".as_slice()],
            );
        }
    }

    // ------------------------------------------------------------- CONFIGS #26
    // The same for argv[2] (kept to at most 3 characters so the iteration count
    // stays below 1000).
    {
        let mut rng = Rng::new(0x4444_5555);
        for i in 0..250 {
            let s = random_byte_string(&mut rng, 3);
            h.check_fresh(
                &format!("cfg26_random_bytes2_{i}"),
                &[b"driver".as_slice(), b"6".as_slice(), &s],
            );
        }
    }

    h.finish();
}

/// Random bytes drawn from the alphabet `strtol` distinguishes.
fn random_byte_string(rng: &mut Rng, max_len: u64) -> Vec<u8> {
    const POOL: &[u8] = b"0123456789 \t\n\x0b\x0c\r+-.eExXaA,_/:0\x80\xff\xa0";
    let len = rng.below(max_len + 1);
    (0..len)
        .map(|_| POOL[rng.below(POOL.len() as u64) as usize])
        .collect()
}

/// A random decimal-ish string covering the shapes `strtol` distinguishes.
fn random_int_string(rng: &mut Rng) -> String {
    let mut s = String::new();
    // leading whitespace
    for _ in 0..rng.below(3) {
        s.push(match rng.below(6) {
            0 => ' ',
            1 => '\t',
            2 => '\n',
            3 => '\u{b}',
            4 => '\u{c}',
            _ => '\r',
        });
    }
    match rng.below(3) {
        0 => s.push('+'),
        1 => s.push('-'),
        _ => {}
    }
    let ndigits = 1 + rng.below(24);
    for _ in 0..ndigits {
        s.push((b'0' + (rng.below(10) as u8)) as char);
    }
    // trailing garbage
    if rng.below(4) == 0 {
        s.push_str(match rng.below(5) {
            0 => "abc",
            1 => ".5",
            2 => "e9",
            3 => " 7",
            _ => "x10",
        });
    }
    s
}

/// A random string for `argv[2]`, kept to a small iteration count so the test
/// runtime stays bounded (0..=64 iterations, or a non-positive value).
fn random_small_count_string(rng: &mut Rng) -> String {
    let mut s = String::new();
    for _ in 0..rng.below(2) {
        s.push(if rng.bool() { ' ' } else { '\t' });
    }
    let negative = rng.below(4) == 0;
    if negative {
        s.push('-');
    } else if rng.below(4) == 0 {
        s.push('+');
    }
    for _ in 0..rng.below(3) {
        s.push('0');
    }
    s.push_str(&rng.below(65).to_string());
    if rng.below(5) == 0 {
        s.push_str("junk");
    }
    s
}

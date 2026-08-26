//! Phase B rows C41..C48 and Phase C rows E31..E43: the `main` driver, i.e.
//! the `scanf` front end and the `printf("%.9g")` back end.
//!
//! Two independent paths are compared:
//!   * the C executable from `c_src/CMakeLists.txt` vs the Rust `driver` binary
//!   * the `main` symbol *exported by the two shared objects*, called through
//!     `src/bin/so_main_runner.rs` (which dlopens the library)

mod common;

use common::{classify_nonpow2, Diff, Nonpow2Class, Rng};

/// Compares the C executable and the Rust binary on one stdin.
fn cmp(d: &mut Diff, input: &str) {
    let c = common::run_c_driver(input);
    let r = common::run_rust_driver(input);
    d.check_bytes(format_args!("stdin={input:?}"), &c.stdout, &r.stdout);
    d.check_eq(format_args!("exit status for stdin={input:?}"), c.code, r.code);
}

/// Compares the `main` export of both shared objects on one stdin.
fn cmp_so(d: &mut Diff, input: &str) {
    let c = common::run_so_main(&common::c_so(), input);
    let r = common::run_so_main(&common::rust_so_path(), input);
    d.check_bytes(format_args!(".so main stdin={input:?}"), &c.stdout, &r.stdout);
    d.check_eq(
        format_args!(".so main exit status for stdin={input:?}"),
        c.code,
        r.code,
    );
}

/// Builds a well-formed twelve token line.
#[allow(clippy::too_many_arguments)]
fn line(
    which: i32,
    x: f32,
    y: f32,
    z: f32,
    xw: i32,
    yw: i32,
    zw: i32,
    seed: i32,
    lac: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
) -> String {
    format!("{which} {x} {y} {z} {xw} {yw} {zw} {seed} {lac} {gain} {offset} {octaves}\n")
}

/// Random inputs that stay clear of the undefined-behaviour corners of case 5.
fn random_case(rng: &mut Rng) -> String {
    let which = rng.range(0, 5);
    let coord = |rng: &mut Rng| match rng.below(4) {
        0 => rng.coord(16),
        1 => rng.coord(1000),
        2 => (rng.range(-8, 8) as f32) / 4.0,
        _ => rng.finite_f32(),
    };
    let (mut x, mut y, mut z) = (coord(rng), coord(rng), coord(rng));
    let (mut xw, mut yw, mut zw) = (
        *rng.pick(&[0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 3, 5, 7, 100, 255, -1, -5]),
        *rng.pick(&[0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 3, 5, 7, 100, 255, -1, -5]),
        *rng.pick(&[0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 3, 5, 7, 100, 255, -1, -5]),
    );
    let mut seed = rng.next_i32();
    let (lac, gain, offset) = (rng.lac_gain(), rng.lac_gain(), rng.lac_gain());
    let octaves = rng.range(0, 8);
    if which == 5 {
        // Keep the shared-object read inside the reproducible window.
        let mut tries = 0;
        while classify_nonpow2(x, y, z, xw, yw, zw, seed as u8) != Nonpow2Class::Reproducible {
            xw = rng.range(1, 256);
            yw = rng.range(1, 256);
            zw = rng.range(1, 256);
            x = rng.coord(300);
            y = rng.coord(300);
            z = rng.coord(300);
            seed = rng.next_i32();
            tries += 1;
            assert!(tries < 100, "could not build an in-window case");
        }
    }
    line(
        which, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves,
    )
}

/// C41: randomised well-formed inputs.
#[test]
fn c41_driver_random_valid_inputs() {
    let mut d = Diff::new("C41 driver randomised valid inputs");
    let mut rng = Rng::new(0x41);
    for _ in 0..300 {
        cmp(&mut d, &random_case(&mut rng));
    }
    d.finish();
}

/// C42: whitespace shapes.
#[test]
fn c42_driver_whitespace_shapes() {
    let mut d = Diff::new("C42 driver whitespace shapes");
    let toks = [
        "1", "1.5", "2.25", "3.125", "4", "8", "16", "42", "2.0", "0.5", "1.0", "6",
    ];
    let seps: [&str; 8] = [" ", "  ", "\t", "\n", "\r\n", " \n\t ", "\x0b", "\x0c"];
    let mut d_inputs: Vec<String> = Vec::new();
    for sep in seps {
        d_inputs.push(toks.join(sep));
        d_inputs.push(format!("{}{}", sep, toks.join(sep)));
        d_inputs.push(format!("{}{}", toks.join(sep), sep));
    }
    // No trailing newline at all, and a blank first line.
    d_inputs.push(toks.join(" "));
    d_inputs.push(format!("\n\n\n{}", toks.join(" ")));
    d_inputs.push(format!("{}\n\n\n", toks.join("\n")));
    for input in &d_inputs {
        cmp(&mut d, input);
    }
    d.finish();
}

/// C43 / E36..E40: number spellings accepted (or rejected) by `scanf`.
#[test]
fn c43_driver_number_spellings() {
    let mut d = Diff::new("C43 driver number spellings");
    let floats = [
        "1.5", "+1.5", "-1.5", ".5", "-.5", "5.", "1e3", "1E-3", "1e+3", "0x1p4", "0X1.8p+3",
        "0x10", "inf", "-inf", "INF", "infinity", "INFINITY", "-infinity", "nan", "-nan", "NaN",
        "1e", "1e+", "1e-", "0x", "0x.", "0xg", ".", "-", "+", "1e400", "-1e400", "1e-400",
        "3.4028235e38", "3.5e38", "1.4e-45", "7e-46", "0.000000000000000000000000000000000000001",
        "00001.5", "1.5e", "1_5", "١٢٣",
    ];
    let ints = [
        "4",
        "+4",
        "-4",
        "0",
        "-0",
        "007",
        "0x10",
        "2147483647",
        "-2147483648",
        "2147483648",
        "99999999999999999999999",
        "-99999999999999999999999",
        "4294967296",
        "1e3",
        "-",
        "+",
        "abc",
    ];
    // Each float spelling in the `x` slot ...
    for f in floats {
        cmp(
            &mut d,
            &format!("1 {f} 2.25 3.125 4 8 16 42 2.0 0.5 1.0 6\n"),
        );
        // ... and in the `lacunarity` slot (which reaches the fractal code).
        cmp(
            &mut d,
            &format!("3 0.25 0.5 0.75 0 0 0 0 {f} 0.5 1.0 4\n"),
        );
    }
    // Each int spelling in the `x_wrap` and `which` slots.
    for i in ints {
        cmp(
            &mut d,
            &format!("1 1.5 2.25 3.125 {i} 8 16 42 2.0 0.5 1.0 6\n"),
        );
        cmp(&mut d, &format!("{i} 1.5 2.25 3.125 4 8 16 42 2 0.5 1 6\n"));
    }
    // The `octaves` slot is the loop bound of the fractal functions: a huge
    // positive value makes *both* implementations run for hours, so the
    // spellings that saturate are used with values that stay bounded
    // (`2147483648` truncates to `INT_MIN`, `99999...` to `-1`).
    let octave_ints = [
        "4",
        "+4",
        "-4",
        "0",
        "-0",
        "007",
        "0x10",
        "300",
        "-2147483648",
        "2147483648",
        "99999999999999999999999",
        "-99999999999999999999999",
        "4294967296",
        "1e3",
        "-",
        "+",
        "abc",
    ];
    for i in octave_ints {
        cmp(
            &mut d,
            &format!("3 0.25 0.5 0.75 0 0 0 0 2.0 0.5 1.0 {i}\n"),
        );
        cmp(
            &mut d,
            &format!("2 0.25 0.5 0.75 0 0 0 0 2.0 0.5 1.0 {i}\n"),
        );
        cmp(
            &mut d,
            &format!("4 0.25 0.5 0.75 0 0 0 0 2.0 0.5 1.0 {i}\n"),
        );
    }
    d.finish();
}

/// C44: magnitudes that stress `%.9g` (style switch, subnormals, overflow).
#[test]
fn c44_driver_extreme_magnitudes() {
    let mut d = Diff::new("C44 driver extreme magnitudes");
    let mut rng = Rng::new(0x44);
    // `ridge`/`fbm`/`turbulence` with huge or tiny parameters produce results
    // across the whole float range, which is what exercises `%.9g`.
    let extremes = [
        "1e30", "1e38", "3.4028235e38", "1e-30", "1e-38", "1e-45", "1e45", "-1e30", "0", "-0",
        "1", "-1", "16777216", "16777217", "1.0000001", "0.99999994", "inf", "-inf", "nan",
        "-nan", "1e19", "1.00000001e9", "123456789", "1234567890", "0.000123456789", "1e-5",
    ];
    for which in [2, 3, 4] {
        for lac in extremes {
            for gain in ["0.5", "2", "1e30", "1e-30", "-1", "0", "inf", "nan"] {
                let octaves = rng.range(1, 6);
                cmp(
                    &mut d,
                    &format!("{which} 0.25 0.5 0.75 0 0 0 0 {lac} {gain} 1e19 {octaves}\n"),
                );
            }
        }
    }
    // Coordinates that make the noise itself return the extremes.
    for v in extremes {
        cmp(&mut d, &format!("0 {v} {v} {v} 0 0 0 0 2 0.5 1 6\n"));
        cmp(&mut d, &format!("2 0.5 0.5 0.5 0 0 0 0 2 0.5 {v} 6\n"));
    }
    d.finish();
}

/// C45 / E31..E33: only part of the twelve tokens is present.
#[test]
fn c45_driver_short_input() {
    let mut d = Diff::new("C45 driver short input");
    let toks = [
        "2", "1.5", "2.25", "3.125", "4", "8", "16", "42", "2.0", "0.5", "1.0", "6",
    ];
    for take in 0..=12 {
        let input = toks[..take].join(" ");
        cmp(&mut d, &input);
        cmp(&mut d, &format!("{input}\n"));
        // A rejected token right after the prefix must behave the same way.
        cmp(&mut d, &format!("{input} zzz\n"));
        cmp(&mut d, &format!("{input} -\n"));
    }
    // Every `which` value with an otherwise empty input.
    for which in -2..=7 {
        cmp(&mut d, &format!("{which}\n"));
    }
    d.finish();
}

/// C46 / E41: more than twelve tokens.
#[test]
fn c46_driver_extra_tokens() {
    let mut d = Diff::new("C46 driver extra tokens");
    let base = "1 1.5 2.25 3.125 4 8 16 42 2.0 0.5 1.0 6";
    for extra in [
        "7", "7 8 9", "junk", "0x", "nan", "-", "\n\n999", " 1e400", "\t\tzzz\n", "999999999999",
    ] {
        cmp(&mut d, &format!("{base} {extra}\n"));
        cmp(&mut d, &format!("{base}{extra}\n"));
    }
    d.finish();
}

/// C47: exit status and stdout of both executables over randomised inputs.
#[test]
fn c47_driver_exit_status_and_stdout() {
    let mut d = Diff::new("C47 driver exit status + stdout");
    let mut rng = Rng::new(0x47);
    for _ in 0..150 {
        let input = random_case(&mut rng);
        let c = common::run_c_driver(&input);
        let r = common::run_rust_driver(&input);
        assert_eq!(c.code, Some(0), "C driver failed on {input:?}");
        d.check_eq(format_args!("exit for {input:?}"), c.code, r.code);
        d.check_bytes(format_args!("stdout for {input:?}"), &c.stdout, &r.stdout);
        // The output is always exactly one line.
        assert!(
            c.stdout.ends_with(b"\n"),
            "C output must end with a newline: {:?}",
            String::from_utf8_lossy(&c.stdout)
        );
    }
    d.finish();
}

/// C48: the `main` symbol exported by the two shared objects.
#[test]
fn c48_so_main_export() {
    let mut d = Diff::new("C48 main exported from the shared objects");
    let mut rng = Rng::new(0x48);
    for _ in 0..60 {
        cmp_so(&mut d, &random_case(&mut rng));
    }
    // ... plus the rejection shapes.
    for input in [
        "",
        "abc",
        "6 1 2 3 4 5 6 7 8 9 10 11\n",
        "0 nan 1 -nan 0 0 0 0 0 0 0 0\n",
        "3 0.25 0.5 0.75 0 0 0 0 1e400 0.5 1.0 4\n",
        "1 1.5",
    ] {
        cmp_so(&mut d, input);
    }
    d.finish();
}

/// E31..E43: the `scanf`/`printf` rejection surface, one input per row.
#[test]
fn e31_e40_scanf_rejections() {
    let mut d = Diff::new("E31-E43 scanf/printf rejections");
    let cases: &[&str] = &[
        // E31 empty stdin
        "",
        "\n",
        "   \t\n  ",
        // E32 first token invalid
        "abc",
        "abc 1 2 3",
        "-",
        "+",
        ".",
        "x",
        // E33 matching failure at conversion k = 2..12
        "0 x 2 3 4 5 6 7 8 9 10 11",
        "0 1 x 3 4 5 6 7 8 9 10 11",
        "0 1 2 x 4 5 6 7 8 9 10 11",
        "0 1 2 3 x 5 6 7 8 9 10 11",
        "0 1 2 3 4 x 6 7 8 9 10 11",
        "0 1 2 3 4 5 x 7 8 9 10 11",
        "0 1 2 3 4 5 6 x 8 9 10 11",
        "2 1 2 3 4 5 6 7 x 9 10 11",
        "2 1 2 3 4 5 6 7 8 x 10 11",
        "2 1 2 3 4 5 6 7 8 9 x 11",
        "2 1 2 3 4 5 6 7 8 9 10 x",
        // E34/E35 %d saturation
        "0 1 2 3 99999999999999999999999 0 0 0 0 0 0 0",
        "0 1 2 3 -99999999999999999999999 0 0 0 0 0 0 0",
        "99999999999999999999999 1 2 3 0 0 0 0 0 0 0 0",
        "-99999999999999999999999 1 2 3 0 0 0 0 0 0 0 0",
        "0 1 2 3 0 0 0 99999999999999999999999 0 0 0 0",
        "3 0.25 0.5 0.75 0 0 0 0 2 0.5 1 99999999999999999999999",
        "3 0.25 0.5 0.75 0 0 0 0 2 0.5 1 -99999999999999999999999",
        // E36 partial inf/nan
        "0 in 2 3 0 0 0 0 0 0 0 0",
        "0 na 2 3 0 0 0 0 0 0 0 0",
        "0 infin 2 3 0 0 0 0 0 0 0 0",
        "0 infinit 2 3 0 0 0 0 0 0 0 0",
        "0 i 2 3 0 0 0 0 0 0 0 0",
        "0 n 2 3 0 0 0 0 0 0 0 0",
        "0 -in 2 3 0 0 0 0 0 0 0 0",
        // E37 "0x" without hex digits
        "0 0x 2 3 0 0 0 0 0 0 0 0",
        "0 0x. 2 3 0 0 0 0 0 0 0 0",
        "0 0xz 2 3 0 0 0 0 0 0 0 0",
        "0 -0x 2 3 0 0 0 0 0 0 0 0",
        // E38 exponent marker without digits
        "0 1e 2 3 0 0 0 0 0 0 0 0",
        "0 1e+ 2 3 0 0 0 0 0 0 0 0",
        "0 1e- 2 3 0 0 0 0 0 0 0 0",
        "0 1.5e 2 3 0 0 0 0 0 0 0 0",
        "0 0x1p 2 3 0 0 0 0 0 0 0 0",
        "0 0x1p+ 2 3 0 0 0 0 0 0 0 0",
        // E39 float over/underflow
        "0 1e400 2 3 0 0 0 0 0 0 0 0",
        "0 -1e400 2 3 0 0 0 0 0 0 0 0",
        "0 1e-400 2 3 0 0 0 0 0 0 0 0",
        "3 0.25 0.5 0.75 0 0 0 0 1e400 1e-400 0 4",
        "3 0.25 0.5 0.75 0 0 0 0 1e39 1e39 0 4",
        // E40 sign only
        "0 - 2 3 0 0 0 0 0 0 0 0",
        "0 + 2 3 0 0 0 0 0 0 0 0",
        "- 1 2 3 0 0 0 0 0 0 0 0",
        // E41 extra tokens
        "0 1 2 3 0 0 0 0 0 0 0 0 13 14 15",
        // E42 invalid `which` read successfully
        "6 1 2 3 4 5 6 7 8 9 10 11",
        "7 1 2 3 4 5 6 7 8 9 10 11",
        "-1 1 2 3 4 5 6 7 8 9 10 11",
        "2147483647 1 2 3 4 5 6 7 8 9 10 11",
        "-2147483648 1 2 3 4 5 6 7 8 9 10 11",
        // E43 negative NaN must print as "-nan"
        "0 nan 1 -nan 0 0 0 0 0 0 0 0",
        "0 -nan 1 nan 0 0 0 0 0 0 0 0",
        "0 inf 1 2 0 0 0 0 0 0 0 0",
        "0 -inf 1 2 0 0 0 0 0 0 0 0",
        "3 0.5 0.5 0.5 0 0 0 0 nan -nan 0 4",
        "2 0.5 0.5 0.5 0 0 0 0 2 0.5 -nan 4",
    ];
    for input in cases {
        cmp(&mut d, input);
        cmp(&mut d, &format!("{input}\n"));
    }
    d.finish();
}

/// Corpus of float spellings (hand written + randomly generated hex and
/// decimal ones) put into every float slot of the input line.
#[test]
fn float_token_corpus() {
    let mut d = Diff::new("float token corpus in every float slot");
    let mut rng = Rng::new(0x51);
    let mut tokens: Vec<String> = [
        "0x.", "0x", "0x.p1", "0xp1", "0x1p", "0x1p+", "0x1p-", "0x1p 3", "0x1.8.8", "0x.5",
        "0x.8p1", "0x8.p1", "0x1p--2", "0x.g", "0x.0", "0X.", "0X.A", "0x1e5", "0x1E5", "0X1P3",
        "0x1p3x", "0x1p2147483648", "0x1p-149", "0x1p-150", "0x1p128", "0x0", "0x00", "0.", "00",
        "1e", "1e+", "1e-", "1.5e", "1E5", "1e05", ".5e3", "5.e3", ".5", ".", "-.", "-.5", "1_5",
        "1.5.5", "--1", "1,5", "1.5f", "inf", "-inf", "INF", "infinity", "INFINITY", "infinit",
        "infinityx", "nan", "-nan", "NaN", "nan(abc)", "nanx", "in", "na", "1e2147483648",
        "1e-2147483648", "1e999999999999999999999", "340282350000000000000000000000000000000",
        "340282360000000000000000000000000000000", "1e38", "1e39", "1e-38", "1e-45", "1e-46",
        "7e-46", "0.0000000000000000000000000000000000000000000000001",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // Randomly generated hex floats (17+ digit mantissas exercise the rounding).
    for _ in 0..80 {
        let mut t = String::from(if rng.boolean() { "0x" } else { "0X" });
        let int_digits = rng.below(20) as usize;
        for _ in 0..int_digits {
            t.push(*rng.pick(&[
                '0', '1', '2', '7', '8', '9', 'a', 'b', 'e', 'f', 'A', 'F',
            ]));
        }
        if rng.boolean() {
            t.push('.');
            for _ in 0..rng.below(20) {
                t.push(*rng.pick(&['0', '1', '8', '9', 'c', 'd', 'F']));
            }
        }
        match rng.below(4) {
            0 => {}
            1 => t.push('p'),
            2 => t.push_str(&format!("p{}", rng.range(-160, 160))),
            _ => t.push_str(&format!("P+{}", rng.below(200))),
        }
        tokens.push(t);
    }
    // Randomly generated decimal floats.
    for _ in 0..80 {
        let mut t = String::new();
        if rng.boolean() {
            t.push(*rng.pick(&['+', '-']));
        }
        for _ in 0..rng.below(25) {
            t.push((b'0' + rng.below(10) as u8) as char);
        }
        if rng.boolean() {
            t.push('.');
            for _ in 0..rng.below(25) {
                t.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        match rng.below(4) {
            0 => {}
            1 => t.push('e'),
            2 => t.push_str(&format!("e{}", rng.range(-60, 60))),
            _ => t.push_str(&format!("E-{}", rng.below(400))),
        }
        tokens.push(t);
    }
    // Slots 1,2,3 are x/y/z; 8,9,10 are lacunarity/gain/offset (0-based).
    let slots = [1usize, 2, 3, 8, 9, 10];
    for token in &tokens {
        for &slot in &slots {
            let which = if slot < 4 { rng.range(0, 1) } else { rng.range(2, 4) };
            let mut fields: Vec<String> = vec![
                which.to_string(),
                "1.5".into(),
                "2.25".into(),
                "3.125".into(),
                "4".into(),
                "8".into(),
                "16".into(),
                "42".into(),
                "2.0".into(),
                "0.5".into(),
                "1.0".into(),
                "5".into(),
            ];
            fields[slot] = token.clone();
            cmp(&mut d, &format!("{}\n", fields.join(" ")));
        }
    }
    d.finish();
}

/// Randomised token soup: every token is a random spelling, valid or not.
#[test]
fn scanf_random_token_soup() {
    let mut d = Diff::new("randomised scanf token soup");
    let toks = [
        "0", "1", "2", "3", "4", "5", "6", "-1", "0.5", "-0.5", "1e3", "1e", "1e-", ".5", "5.",
        "0x", "0x1p3", "inf", "-inf", "nan", "-nan", "in", "na", "abc", "-", "+", ".", "",
        "2147483648", "-2147483649", "99999999999999999999", "007", "0x10", "1_0", "1.5.5",
        "--1", "+-1", "1e400", "1e-400", "255", "256", "-256",
    ];
    let seps = [" ", "\t", "\n", "  ", "\r\n"];
    let mut rng = Rng::new(0x50);
    for _ in 0..400 {
        let n = rng.range(0, 15) as usize;
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push_str(rng.pick::<&str>(&seps));
            }
            s.push_str(rng.pick::<&str>(&toks));
        }
        if rng.boolean() {
            s.push('\n');
        }
        cmp(&mut d, &s);
    }
    d.finish();
}

//! CONFIGS.md rows 61-62 and ERRORS.md rows 48-49: the whole program.
//!
//! `c_src/src/main.c` is the only entry point that is not reachable through
//! `dlsym` (calling the exported `main` in-process would `exit()` the test
//! runner), so both `main`s are compared by *running* them:
//!
//! * `cbuild/cdriver`        -- the CMake-built C executable
//! * `target/<profile>/driver` -- the Rust binary, which is `#![no_main]` and
//!   uses the very same `main` symbol that the Rust `.so` exports.
//!
//! stdout, stderr and the exit status are compared byte for byte.  `argv[0]` is
//! forced to the same string on both sides with `CommandExt::arg0`, because
//! `main.c` prints it in the error path.

mod harness;

use harness::*;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::process::Command;

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

fn run(prog: &std::path::Path, args: &[OsString]) -> Run {
    let out = Command::new(prog)
        .arg0("driver")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

#[track_caller]
fn compare(args: &[OsString]) {
    let (c_exe, r_exe) = drivers();
    assert!(
        c_exe.exists() && r_exe.exists(),
        "run ./build_c.sh && cargo build first ({} / {})",
        c_exe.display(),
        r_exe.display()
    );
    let c = run(&c_exe, args);
    let r = run(&r_exe, args);
    let show = |v: &[u8]| String::from_utf8_lossy(v).into_owned();
    let pretty: Vec<String> = args
        .iter()
        .map(|a| String::from_utf8_lossy(a.as_bytes()).into_owned())
        .collect();
    assert_eq!(
        c.code, r.code,
        "exit status differs for args {pretty:?}\n  C stderr: {}\n  R stderr: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout differs for args {pretty:?}"
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout bytes differ for args {pretty:?}"
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "stderr differs for args {pretty:?}"
    );
    assert_eq!(c.stderr, r.stderr, "stderr bytes differ for args {pretty:?}");
}

fn a(s: &str) -> OsString {
    OsString::from(s)
}

/// ERRORS.md row 48 -- `if(argc != 4) { fprintf(stderr, ...); exit(1); }`
#[test]
fn driver_wrong_argc() {
    // argc counts argv[0], so 0, 1, 2, 4, 5 extra arguments are all errors
    compare(&[]);
    compare(&[a("1")]);
    compare(&[a("1"), a("2")]);
    compare(&[a("1"), a("2"), a("3"), a("4")]);
    compare(&[a("1"), a("2"), a("3"), a("4"), a("5")]);
    compare(&[a(""), a(""), a(""), a("")]);
    // ... and exactly 3 is the only accepted count
    compare(&[a("1"), a("2"), a("3")]);

    // pin the actual sentinel: exit code 1, empty stdout, the message on stderr
    let (c_exe, _) = drivers();
    let out = run(&c_exe, &[a("1")]);
    assert_eq!(out.code, Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr, b"driver requires 4 inputs\n");
}

/// CONFIGS.md row 61 -- the accepted path, over every interesting numeric shape.
#[test]
fn driver_valid_arguments() {
    let vals = [
        "3", "4", "5", "0", "-0", "0.0", "-0.0", "1", "-1", "1.5", "-1.5", "0.5", "1e10", "1E10",
        "1e-10", "1e30", "-1e30", "1e38", "3.4028235e38", "3.4028236e38", "1e39", "1e-38",
        "1e-45", "1e-46", "1.1754944e-38", "5.877472e-39", "99999", "-99999", "65536",
        "0.30000000000000004", "0.1", "0.2", "0.3", "1.0000001", "0.99999994", "16777216",
        "16777217", "2147483647", "2147483648", "4294967296", "1e-320", "2.2250738585072014e-308",
    ];
    // every triple of a rotating window keeps the number of runs sane while
    // still covering all values in all three positions
    for i in 0..vals.len() {
        let x = vals[i];
        let y = vals[(i + 7) % vals.len()];
        let z = vals[(i + 13) % vals.len()];
        compare(&[a(x), a(y), a(z)]);
    }
    // all three equal, and the axis-aligned / zero / mixed-sign shapes
    for v in vals {
        compare(&[a(v), a("0"), a("0")]);
        compare(&[a(v), a(v), a(v)]);
    }
    compare(&[a("1"), a("0"), a("0")]);
    compare(&[a("0"), a("0"), a("0")]);
    compare(&[a("-0"), a("-0"), a("-0")]);
    compare(&[a("1"), a("-1"), a("1")]);
}

/// ERRORS.md row 49 -- `atof` never reports an error, so every string is
/// "valid"; these are the shapes where a hand-written parser typically diverges
/// from C99 `strtod`.
#[test]
fn driver_odd_arguments() {
    let odd = [
        "",
        " ",
        "  \t\n 12",
        "\t-3.5",
        "\r\n5",
        "+5",
        "+.5",
        ".5",
        "5.",
        ".",
        "-",
        "+",
        "abc",
        "12abc",
        "12e",
        "12e+",
        "12e-",
        "1e",
        "0e0",
        "00012.500",
        "1_000",
        "1,5",
        "--1",
        "1-",
        "inf",
        "INF",
        "Inf",
        "infinity",
        "INFINITY",
        "-inf",
        "-INFINITY",
        "inf-",
        "infin",
        "nan",
        "NAN",
        "NaN",
        "-nan",
        "nan(123)",
        "nan()",
        "nanq",
        "0x10",
        "0X10",
        "-0x1p3",
        "0x1P-3",
        "0x.8p1",
        "0x1.8p+1",
        "0x",
        "0xg",
        "0x0",
        "0x1p",
        "1e999",
        "-1e999",
        "1e-999",
        "1e310",
        "1e-310",
        "179769313486231580000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "0.000000000000000000000000000000000000000000001",
        "123456789012345678901234567890",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "4.9406564584124654e-324",
        "2.4703282292062327e-324",
        "1e",
        "e5",
        "E5",
        "1d5",
        "\u{a0}5", // non-ASCII whitespace: NOT skipped by strtod
    ];
    for s in odd {
        compare(&[a(s), a("1"), a("2")]);
        compare(&[a("1"), a(s), a("2")]);
        compare(&[a("1"), a("2"), a(s)]);
    }
    // three odd ones at once
    compare(&[a("nan"), a("inf"), a("-inf")]);
    compare(&[a(""), a("abc"), a("12abc")]);
    compare(&[a("0x1p3"), a("0x2p3"), a("0x3p3")]);

    // arguments that are not valid UTF-8 must be handled as raw bytes
    for bytes in [
        &b"\xff\xfe"[..],
        &b"12\xff"[..],
        &b"\xff12"[..],
        &b"1\x002"[..],
    ] {
        // a NUL byte cannot be passed through execve, skip that one
        if bytes.contains(&0) {
            continue;
        }
        let arg = OsStr::from_bytes(bytes).to_os_string();
        compare(&[arg.clone(), a("1"), a("2")]);
        compare(&[a("1"), arg.clone(), a("2")]);
    }
}

/// Randomized: thousands of `printf("%f")` round trips through
/// `atof` -> `VectorNormalizeFast` -> `printf`, with a fixed seed.
#[test]
fn driver_randomized() {
    let mut rng = Rng::new(0xD8146E);
    for _ in 0..300 {
        let mk = |rng: &mut Rng| -> OsString {
            let v = rng.f32_any();
            let s = match rng.below(6) {
                0 => format!("{v}"),
                1 => format!("{v:e}"),
                2 => format!("{v:.9}"),
                3 => format!("{v:.17e}"),
                4 => format!("{}", v as f64),
                _ => format!("{:x}", (v.to_bits() as u64)), // plain integer digits
            };
            OsString::from(s)
        };
        let args = [mk(&mut rng), mk(&mut rng), mk(&mut rng)];
        compare(&args);
    }
    // pure integer strings of random length
    for _ in 0..100 {
        let mk = |rng: &mut Rng| -> OsString {
            let n = 1 + rng.below(25) as usize;
            let mut s = String::new();
            if rng.bool() {
                s.push('-');
            }
            for _ in 0..n {
                s.push((b'0' + (rng.below(10) as u8)) as char);
            }
            if rng.bool() {
                s.push('.');
                for _ in 0..rng.below(20) {
                    s.push((b'0' + (rng.below(10) as u8)) as char);
                }
            }
            if rng.below(3) == 0 {
                s.push('e');
                if rng.bool() {
                    s.push('-');
                }
                s.push_str(&format!("{}", rng.below(400)));
            }
            OsString::from(s)
        };
        let args = [mk(&mut rng), mk(&mut rng), mk(&mut rng)];
        compare(&args);
    }
}

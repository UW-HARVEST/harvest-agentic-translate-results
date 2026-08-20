// Phase B/C at the process level — the interface an actual user of this project
// sees: the `driver` program built from c_src/CMakeLists.txt versus the `driver`
// program built by cargo.  Covers CONFIGS.md rows 21-24 and mirrors the
// ERRORS.md rows through the CLI.
//
// Every invocation is a fresh process, so `static int inner` starts at 1 again.

mod common;

use common::*;
use std::os::unix::ffi::OsStrExt;
use std::process::{Command, Output};

fn run(exe: &std::path::Path, args: &[&[u8]]) -> Output {
    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(std::ffi::OsStr::from_bytes(a));
    }
    cmd.output().expect("spawn driver")
}

fn diff(args: &[&[u8]]) -> Option<String> {
    let c = run(&c_exe(), args);
    let r = run(&rust_exe(), args);
    if c.status.code() == r.status.code() && c.stdout == r.stdout && c.stderr == r.stderr {
        return None;
    }
    let pretty: Vec<String> = args
        .iter()
        .map(|a| {
            let s = String::from_utf8_lossy(a).into_owned();
            if a.len() > 40 {
                format!("<{} bytes>", a.len())
            } else {
                s
            }
        })
        .collect();
    Some(format!(
        "argv={pretty:?}\n   C   : status={:?} stdout={:?} stderr={:?}\n   Rust: status={:?} stdout={:?} stderr={:?}",
        c.status.code(),
        show(&c.stdout),
        show(&c.stderr),
        r.status.code(),
        show(&r.stdout),
        show(&r.stderr)
    ))
}

fn check_all(cases: &[Vec<Vec<u8>>]) {
    let mut failures = Vec::new();
    for case in cases {
        let refs: Vec<&[u8]> = case.iter().map(|v| v.as_slice()).collect();
        if let Some(f) = diff(&refs) {
            failures.push(f);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} CLI cases diverged:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

fn case(args: &[&str]) -> Vec<Vec<u8>> {
    args.iter().map(|s| s.as_bytes().to_vec()).collect()
}

// ------------------------------------------------------------------ CONFIGS #21

#[test]
fn cli_random_pairs() {
    let mut rng = Rng::new(0xC0FF_EE01);
    let mut cases = Vec::new();
    for _ in 0..300 {
        let initial = match rng.below(8) {
            0 => i32::MIN.to_string(),
            1 => i32::MAX.to_string(),
            2 => rng.range_i32(-20, 20).to_string(),
            3 => rng.range_i32(i32::MAX - 8, i32::MAX).to_string(),
            4 => rng.range_i32(i32::MIN, i32::MIN + 8).to_string(),
            5 => format!("{}", rng.next_i32() as i64 * 3), // outside int range
            6 => format!("{:+}", rng.next_i32()),
            _ => rng.next_i32().to_string(),
        };
        let iterations = rng.below(70).to_string();
        cases.push(case(&[&initial, &iterations]));
    }
    check_all(&cases);
}

// ------------------------------------------------------------------ CONFIGS #22

#[test]
fn cli_shape_matrix() {
    let mut cases = Vec::new();
    let arg1: &[&str] = &[
        "0",
        "1",
        "-1",
        "+7",
        " 12",
        "\t-13",
        "\n14",
        "\u{b}15",
        "\u{c}16",
        "\r17",
        "007",
        "12abc",
        "0x10",
        "1e5",
        "1.9",
        "5 5",
        "1073741824",
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999",
        "-99999999999999999999",
    ];
    let arg2: &[&str] = &["0", "1", "2", "5", "31", "32", "33", "40", "64", "-1", "007", "12abc"];
    for a in arg1 {
        for b in arg2 {
            cases.push(case(&[a, b]));
        }
    }
    // randomized shape content: random prefix/digits/suffix for both arguments
    let mut rng = Rng::new(0x5AAE_0022);
    const PREFIX: &[&str] = &["", " ", "\t", "\n", "\u{b}", "\u{c}", "\r", "+", "-", "00", " -"];
    const SUFFIX: &[&str] = &["", "abc", ".5", "e9", " 7", "x10"];
    for _ in 0..80 {
        let mut a = String::from(PREFIX[rng.below(PREFIX.len() as u64) as usize]);
        for _ in 0..=rng.below(22) {
            a.push((b'0' + rng.below(10) as u8) as char);
        }
        a.push_str(SUFFIX[rng.below(SUFFIX.len() as u64) as usize]);
        let mut b = String::from(PREFIX[rng.below(PREFIX.len() as u64) as usize]);
        b.push_str(&rng.below(120).to_string());
        b.push_str(SUFFIX[rng.below(SUFFIX.len() as u64) as usize]);
        cases.push(case(&[&a, &b]));
    }
    check_all(&cases);
}

// ------------------------------------------------------------------ CONFIGS #23

#[test]
fn cli_err_argc_sweep() {
    let mut cases = Vec::new();
    cases.push(case(&[]));
    cases.push(case(&["1"]));
    cases.push(case(&["1", "2", "3"]));
    cases.push(case(&["1", "2", "3", "4"]));
    cases.push(case(&["1", "2", "3", "4", "5"]));
    cases.push(case(&["", ""]));
    check_all(&cases);
}

#[test]
fn cli_err_unparsable() {
    let bad = [
        "", " ", "\t", "\n", "\u{b}", "\u{c}", "\r", " \t\n\u{b}\u{c}\r", "+", "-", "+ 1", "--1",
        "++1", "abc", ".5", "x10", "e5", ",", "O", "#", " - 3",
    ];
    let mut cases = Vec::new();
    for b in bad {
        cases.push(case(&[b, "3"])); // first argument rejected
        cases.push(case(&["3", b])); // second argument rejected
        cases.push(case(&[b, b]));
    }
    // non-UTF-8 arguments
    cases.push(vec![b"\x80\xff".to_vec(), b"3".to_vec()]);
    cases.push(vec![b"3".to_vec(), b"\x80\xff".to_vec()]);
    cases.push(vec![b"\xd9\xa3".to_vec(), b"3".to_vec()]);
    // randomized strings from an alphabet without digits: strtol can never
    // convert them, so both must produce the corresponding rejection message
    let mut rng = Rng::new(0xBAD5_0023);
    const POOL: &[u8] = b" \t\n\x0b\x0c\r+-.,:/eExXaAoO#_\x80\xa0\xff";
    for _ in 0..60 {
        let mk = |rng: &mut Rng| -> Vec<u8> {
            let len = 1 + rng.below(6);
            (0..len)
                .map(|_| POOL[rng.below(POOL.len() as u64) as usize])
                .collect()
        };
        let a = mk(&mut rng);
        let b = mk(&mut rng);
        cases.push(vec![a.clone(), b"3".to_vec()]);
        cases.push(vec![b"3".to_vec(), b.clone()]);
        cases.push(vec![a, b]);
    }
    check_all(&cases);
}

#[test]
fn cli_err_iterations_non_positive() {
    let mut cases = Vec::new();
    for it in ["0", "-0", "+0", "-1", "-2147483648", "-99999999999999999999"] {
        for v in ["1", "-1", "0", "2147483647"] {
            cases.push(case(&[v, it]));
        }
    }
    check_all(&cases);
}

// ------------------------------------------------------------------ CONFIGS #24

#[test]
fn cli_long_output() {
    let mut cases = vec![
        case(&["1", "1000"]),
        case(&["-1000", "3000"]),
        case(&["-2147483648", "5000"]),
        case(&["2147483647", "2000"]),
    ];
    // randomized long streams
    let mut rng = Rng::new(0x1016_0024);
    for _ in 0..10 {
        let v = rng.next_i32().to_string();
        let n = rng.range_i32(500, 4000).to_string();
        cases.push(case(&[&v, &n]));
    }
    for _ in 0..10 {
        let v = rng.range_i32(-4000, 0).to_string();
        let n = rng.range_i32(500, 4000).to_string();
        cases.push(case(&[&v, &n]));
    }
    check_all(&cases);
}

// ------------------------------------------------------------------ CONFIGS #27
// A reader that goes away in the middle of a long output stream: the C program
// dies from the default SIGPIPE disposition, so the Rust program must too (the
// Rust runtime sets SIGPIPE to SIG_IGN before main, which src/main.rs undoes).

#[test]
fn cli_closed_pipe_sigpipe() {
    use std::io::Read;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Stdio;

    fn run_until_reader_closes(exe: &std::path::Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(exe)
            .args(["1", "20000000"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        {
            let mut out = child.stdout.take().expect("stdout");
            let mut buf = [0u8; 8];
            let _ = out.read(&mut buf);
            // dropping `out` closes the read end while the child keeps writing
        }
        let st = child.wait().expect("wait");
        (st.code(), st.signal())
    }

    let c = run_until_reader_closes(&c_exe());
    let r = run_until_reader_closes(&rust_exe());
    assert_eq!(c, r, "closed-pipe outcome (code, signal): C {c:?} vs Rust {r:?}");
    assert_eq!(c.1, Some(13), "expected the C program to die from SIGPIPE");
}

// ------------------------------------------------------------------ CONFIGS #28
// stdout closed before the program starts: printf/write fails, the C code
// ignores the error and still returns 0.

#[test]
fn cli_closed_stdout() {
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;

    extern "C" {
        fn close(fd: std::ffi::c_int) -> std::ffi::c_int;
    }

    fn run_with_closed_stdout(exe: &std::path::Path, args: &[&str]) -> (Option<i32>, Option<i32>) {
        let mut cmd = Command::new(exe);
        cmd.args(args).stderr(std::process::Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                close(1);
                Ok(())
            });
        }
        let st = cmd.status().expect("spawn");
        (st.code(), st.signal())
    }

    for args in [
        ["5", "10"].as_slice(),
        ["-3", "4"].as_slice(),
        ["x", "4"].as_slice(),
        ["5"].as_slice(),
    ] {
        let c = run_with_closed_stdout(&c_exe(), args);
        let r = run_with_closed_stdout(&rust_exe(), args);
        assert_eq!(c, r, "closed-stdout outcome for {args:?}: C {c:?} vs Rust {r:?}");
    }
}

// ------------------------------------------------------------------ CONFIGS #29
// The program never calls setlocale(), so locale environment variables must not
// influence strtol()'s whitespace/digit handling or printf's "%d" formatting.

// ------------------------------------------------------------------ CONFIGS #30
// The program executed with a completely empty argv (argc == 0), which the
// kernel allows: `main` must take the `argc != 3` path in both implementations.

#[test]
fn cli_empty_argv() {
    use std::ffi::{c_char, CString};
    use std::os::unix::process::CommandExt;

    extern "C" {
        fn execv(path: *const c_char, argv: *const *const c_char) -> std::ffi::c_int;
    }

    fn run_with_empty_argv(exe: &std::path::Path) -> std::process::Output {
        let path = CString::new(exe.as_os_str().as_bytes()).expect("path");
        let mut cmd = Command::new(exe);
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        unsafe {
            cmd.pre_exec(move || {
                let argv: [*const c_char; 1] = [std::ptr::null()];
                execv(path.as_ptr(), argv.as_ptr());
                Err(std::io::Error::last_os_error())
            });
        }
        cmd.output().expect("spawn")
    }

    let c = run_with_empty_argv(&c_exe());
    let r = run_with_empty_argv(&rust_exe());
    assert_eq!(
        (c.status.code(), &c.stdout, &c.stderr),
        (r.status.code(), &r.stdout, &r.stderr),
        "empty argv: C(status={:?}, out={:?}) vs Rust(status={:?}, out={:?})",
        c.status.code(),
        show(&c.stdout),
        r.status.code(),
        show(&r.stdout)
    );
    assert_eq!(
        c.stdout, b"Error: should only be two (integer) arguments!\n",
        "expected the argc != 3 message"
    );
}

#[test]
fn cli_locale_environment() {
    let mut failures = Vec::new();
    for locale in ["C", "POSIX", "de_DE.UTF-8", "tr_TR.UTF-8", "en_US.UTF-8", "invalid"] {
        for args in [
            ["1234567", "3"].as_slice(),
            [" 1234567", "3"].as_slice(),
            ["-2147483648", "2"].as_slice(),
            ["1", "35"].as_slice(),
        ] {
            let run_one = |exe: &std::path::Path| {
                let mut cmd = Command::new(exe);
                cmd.args(args)
                    .env("LC_ALL", locale)
                    .env("LANG", locale)
                    .env("LC_NUMERIC", locale);
                cmd.output().expect("spawn")
            };
            let c = run_one(&c_exe());
            let r = run_one(&rust_exe());
            if c.status.code() != r.status.code() || c.stdout != r.stdout || c.stderr != r.stderr {
                failures.push(format!(
                    "LC_ALL={locale} argv={args:?}: C(status={:?}, out={:?}) vs Rust(status={:?}, out={:?})",
                    c.status.code(),
                    show(&c.stdout),
                    r.status.code(),
                    show(&r.stdout)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn cli_oversized_arguments() {
    let digits4k = "1".repeat(4096);
    let zeros4k = format!("{}7", "0".repeat(4096));
    let blanks4k = format!("{}42", " ".repeat(4096));
    let digits64k = "9".repeat(65536);
    check_all(&[
        case(&[&digits4k, "3"]),
        case(&[&zeros4k, "3"]),
        case(&[&blanks4k, "3"]),
        case(&[&digits64k, "3"]),
        case(&["5", &zeros4k]),
        case(&["5", &digits4k]),
    ]);
}

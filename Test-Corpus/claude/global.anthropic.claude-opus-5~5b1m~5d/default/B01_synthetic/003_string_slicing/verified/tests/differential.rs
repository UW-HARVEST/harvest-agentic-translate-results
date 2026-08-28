//! Differential tests: run the ORIGINAL C binary and the Rust binary as
//! subprocesses with identical argument vectors and require byte-identical
//! stdout, byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked as a library here; both programs are driven
//! exactly the way a shell drives them, because that is how they are compared.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // translation/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C reference program.
///
/// Prefers an already-built `c_src/build/driver`. Otherwise configures and
/// builds out-of-tree into `translation/target/c_build` so that nothing inside
/// `c_src/` is ever written to or modified.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = repo_root();
        let prebuilt = root.join("c_src/build/driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let src = root.join("c_src");
        let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/c_build");
        std::fs::create_dir_all(&build).expect("create out-of-tree C build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("failed to spawn cmake; is cmake installed?");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("failed to spawn cmake --build");
        assert!(
            bld.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let out = build.join("driver");
        assert!(out.is_file(), "C driver binary not produced at {out:?}");
        out
    })
}

/// Path to the compiled Rust program under test (built by cargo for us).
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    signal: Option<i32>,
}

fn describe(o: &Outcome) -> String {
    format!(
        "exit={:?} signal={:?}\n    stdout({} bytes) = {:?}\n      hex = {}\n    stderr({} bytes) = {:?}\n      hex = {}",
        o.code,
        o.signal,
        o.stdout.len(),
        String::from_utf8_lossy(&o.stdout),
        hex(&o.stdout),
        o.stderr.len(),
        String::from_utf8_lossy(&o.stderr),
        hex(&o.stderr),
    )
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn run(exe: &Path, args: &[&[u8]], stdin_data: &[u8]) -> Outcome {
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt;

    let os_args: Vec<OsString> = args
        .iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect();

    let mut child = Command::new(exe)
        .args(&os_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {exe:?}: {e}"));

    {
        let mut sin = child.stdin.take().expect("piped stdin");
        // The program may exit without reading stdin; a broken pipe is fine.
        let _ = sin.write_all(stdin_data);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Core assertion: the C program and the Rust program must agree on all three
/// observable channels for this argument vector.
fn assert_same(args: &[&[u8]]) {
    assert_same_with_stdin(args, b"");
}

fn assert_same_with_stdin(args: &[&[u8]], stdin_data: &[u8]) {
    let c = run(c_binary(), args, stdin_data);
    let r = run(rust_binary(), args, stdin_data);

    if c == r {
        return;
    }

    let pretty: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
        .collect();
    panic!(
        "\nDIFFERENTIAL MISMATCH\n  argv[1..] = [{}]\n  raw hex   = [{}]\n  stdin     = {:?}\n  C  : {}\n  RUST: {}\n",
        pretty.join(", "),
        args.iter().map(|a| hex(a)).collect::<Vec<_>>().join(", "),
        String::from_utf8_lossy(stdin_data),
        describe(&c),
        describe(&r),
    );
}

fn assert_same_many(cases: &[&[&[u8]]]) {
    for case in cases {
        assert_same(case);
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: one test per input class the C program actually branches on
// ---------------------------------------------------------------------------

/// `if ((argc > 4) || (argc == 1))` -- the usage error path.
#[test]
fn argc_out_of_range_prints_usage_and_exits_1() {
    assert_same_many(&[
        // argc == 1: no arguments at all.
        &[],
        // argc == 5, 6, 7 ...: too many arguments.
        &[b"hello", b"1", b"3", b"extra"],
        &[b"hello", b"1", b"3", b"extra", b"more"],
        &[b"", b"", b"", b"", b""],
        &[b"a", b"b", b"c", b"d", b"e", b"f"],
        &[b"hello", b"0", b"5", b"", b""],
    ]);

    // Spot-check the observable contract directly, so a change of message or
    // exit code cannot slip through by "both being wrong the same way".
    let c = run(c_binary(), &[], b"");
    assert_eq!(c.code, Some(1), "usage error must exit 1");
    assert_eq!(
        c.stdout,
        b"Error: there should be one to three arguments passed:\n<string> [start] [stop]\n"
    );
    assert!(c.stderr.is_empty(), "C writes the usage error to stdout");
}

/// `argc == 2`: only the string, so start = 0 and stop = len.
#[test]
fn single_argument_prints_whole_string() {
    assert_same_many(&[
        &[b"hello"],
        &[b""],
        &[b"a"],
        &[b"hello world"],
        &[b"0123456789"],
        &[b"   "],
        &[b"\t\t"],
        &[b"a\nb"],
        &[b"line1\nline2\n"],
        &[b"h\xc3\xa9llo"],
        &[b"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e"],
    ]);
}

/// `if (end == argv[2])` -- strtol performed no conversion on the 2nd argument.
/// Note: the message has NO trailing newline in the C.
#[test]
fn second_argument_not_an_integer() {
    assert_same_many(&[
        &[b"hello", b"abc"],
        &[b"hello", b""],
        &[b"hello", b"-"],
        &[b"hello", b"+"],
        &[b"hello", b" "],
        &[b"hello", b"   "],
        &[b"hello", b"\t"],
        &[b"hello", b"\n"],
        &[b"hello", b"\x0b"],
        &[b"hello", b"\x0c"],
        &[b"hello", b"\r"],
        &[b"hello", b"  -"],
        &[b"hello", b"  +"],
        &[b"hello", b"x1"],
        &[b"hello", b".5"],
        &[b"hello", b"e3"],
        &[b"hello", b"_1"],
        &[b"hello", b"\xff"],
        // Arabic-Indic digit: not an ASCII digit, so no conversion.
        &[b"hello", b"\xd9\xa3"],
        // Same error, reached from the 3-argument form too.
        &[b"hello", b"abc", b"3"],
        &[b"hello", b"", b"3"],
        &[b"", b"abc"],
    ]);

    // The C prints this message without a newline; assert that literally.
    let c = run(c_binary(), &[b"hello", b"abc"], b"");
    assert_eq!(c.stdout, b"Second argument must be an integer!");
    assert_eq!(c.code, Some(1));
}

/// strtol accepts leading whitespace, a sign, and stops at the first
/// non-digit -- all of which are *successful* partial conversions.
#[test]
fn second_argument_partial_and_signed_conversions() {
    assert_same_many(&[
        &[b"hello", b" 2"],
        &[b"hello", b"  2"],
        &[b"hello", b"\t2"],
        &[b"hello", b"\n2"],
        &[b"hello", b"\x0b2"],
        &[b"hello", b"\x0c2"],
        &[b"hello", b"\r2"],
        &[b"hello", b"+2"],
        &[b"hello", b"-0"],
        &[b"hello", b"  -0"],
        &[b"hello", b"+0"],
        &[b"hello", b"007"],
        &[b"hello", b"0x3"],  // base 10: parses "0", stops at 'x'
        &[b"hello", b"2abc"], // parses 2, stops at 'a'
        &[b"hello", b"2 "],
        &[b"hello", b"1_0"],
        &[b"hello", b"1e3"],
        &[b"hello", b"0b1"],
        &[b"hello", b"2\xff"],
        &[b"hello", b"3", b"4"],
        &[b"hello", b" 1", b" 4"],
    ]);
}

/// `if (start > len)` -- an `int` compared against a `size_t`, so the int is
/// converted to unsigned and every NEGATIVE start is "off the end".
#[test]
fn start_bounds_including_negative_becoming_huge_unsigned() {
    assert_same_many(&[
        // in range
        &[b"hello", b"0"],
        &[b"hello", b"1"],
        &[b"hello", b"4"],
        // start == len is allowed (not `>`), prints an empty line
        &[b"hello", b"5"],
        // start > len
        &[b"hello", b"6"],
        &[b"hello", b"7"],
        &[b"hello", b"100"],
        // negative -> huge unsigned -> "off the end"
        &[b"hello", b"-1"],
        &[b"hello", b"-2"],
        &[b"hello", b"-5"],
        &[b"hello", b"-100"],
        &[b"hello", b"-2147483648"],
        // empty string: only 0 is in range
        &[b"", b"0"],
        &[b"", b"1"],
        &[b"", b"-1"],
    ]);

    let c = run(c_binary(), &[b"hello", b"-1"], b"");
    assert_eq!(c.stdout, b"Error: start is off the end of the string!\n");
    assert_eq!(c.code, Some(1));

    // start == len: precision is 0, so only the newline is printed.
    let c = run(c_binary(), &[b"hello", b"5"], b"");
    assert_eq!(c.stdout, b"\n");
    assert_eq!(c.code, Some(0));
}

/// `long` -> `int` truncation on assignment, plus strtol's saturation at
/// LONG_MIN / LONG_MAX on overflow.
#[test]
fn start_long_to_int_truncation_and_strtol_saturation() {
    assert_same_many(&[
        // 2^32 truncates to 0 -> a VALID start of 0
        &[b"hello", b"4294967296"],
        &[b"hello", b"4294967297"],
        &[b"hello", b"4294967299"],
        &[b"hello", b"4294967301"], // -> 5 == len
        &[b"hello", b"4294967302"], // -> 6 > len
        &[b"hello", b"8589934592"],  // 2^33 -> 0
        &[b"hello", b"12884901888"], // 3*2^32 -> 0
        // INT_MAX / INT_MIN boundaries
        &[b"hello", b"2147483647"],
        &[b"hello", b"2147483648"], // -> INT_MIN
        &[b"hello", b"-2147483649"],
        &[b"hello", b"4294967295"], // -> -1
        &[b"hello", b"-4294967296"],
        // strtol saturates to LONG_MAX/LONG_MIN, then truncates
        &[b"hello", b"9223372036854775807"], // LONG_MAX -> -1
        &[b"hello", b"9223372036854775808"], // ERANGE -> LONG_MAX -> -1
        &[b"hello", b"-9223372036854775808"],
        &[b"hello", b"-9223372036854775809"],
        &[b"hello", b"18446744073709551616"],
        &[b"hello", b"99999999999999999999999"],
        &[b"hello", b"-99999999999999999999999"],
        &[b"hello", b"1000000000000000000000000000000"],
        // and the same values in the stop position
        &[b"hello", b"0", b"4294967296"],
        &[b"hello", b"0", b"4294967301"],
        &[b"hello", b"0", b"9223372036854775807"],
        &[b"hello", b"0", b"99999999999999999999999"],
        &[b"hello", b"0", b"-99999999999999999999999"],
        &[b"hello", b"0", b"2147483648"],
    ]);
}

/// `if (stop > len)` and `if (stop <= start)` -- the two stop checks, in the
/// order the C performs them (bounds first, ordering second).
#[test]
fn stop_bounds_and_ordering_checks() {
    assert_same_many(&[
        // valid windows
        &[b"hello", b"0", b"5"],
        &[b"hello", b"0", b"1"],
        &[b"hello", b"1", b"3"],
        &[b"hello", b"4", b"5"],
        // stop > len
        &[b"hello", b"0", b"6"],
        &[b"hello", b"0", b"100"],
        &[b"hello", b"5", b"6"],
        // negative stop -> huge unsigned -> "stop is off the end"
        &[b"hello", b"0", b"-1"],
        &[b"hello", b"2", b"-1"],
        // stop <= start
        &[b"hello", b"0", b"0"],
        &[b"hello", b"2", b"2"],
        &[b"hello", b"3", b"2"],
        &[b"hello", b"5", b"5"],
        &[b"hello", b"5", b"1"],
        // BOTH would fail: bounds is checked first, so expect the stop-bounds
        // message even though stop <= start also holds.
        &[b"hello", b"3", b"-1"],
        &[b"hello", b"0", b"-5"],
        // empty string
        &[b"", b"0", b"0"],
        &[b"", b"0", b"1"],
        &[b"", b"-1", b"0"],
    ]);

    let c = run(c_binary(), &[b"hello", b"0", b"6"], b"");
    assert_eq!(c.stdout, b"Error: stop is off the end of the string!\n");
    assert_eq!(c.code, Some(1));

    let c = run(c_binary(), &[b"hello", b"3", b"2"], b"");
    assert_eq!(c.stdout, b"Error: stop must come after start!\n");
    assert_eq!(c.code, Some(1));

    // Ordering proof: stop = -1 is both out of bounds AND <= start.
    let c = run(c_binary(), &[b"hello", b"3", b"-1"], b"");
    assert_eq!(
        c.stdout,
        b"Error: stop is off the end of the string!\n",
        "the stop-bounds check must run before the ordering check"
    );
}

/// `if (end == argv[3])` tests the STALE `end` left over from parsing argv[2]
/// (strtol is called with a NULL endptr for argv[3]). `end` can point at most
/// to argv[2]'s NUL terminator, i.e. `argv[3] - 1`, so the branch is dead and
/// "Third argument must be an integer!" can never be printed. A non-numeric
/// third argument therefore silently becomes stop = 0.
#[test]
fn third_argument_integer_check_is_dead_code() {
    let cases: &[&[&[u8]]] = &[
        &[b"hello", b"0", b"abc"],
        &[b"hello", b"0", b""],
        &[b"hello", b"0", b"-"],
        &[b"hello", b"0", b"+"],
        &[b"hello", b"0", b"   "],
        &[b"hello", b"0", b"\xff"],
        &[b"hello", b"0", b"x9"],
        &[b"hello", b"1", b"abc"],
        &[b"hello", b"1", b""],
        &[b"hello", b"2", b"nope"],
        &[b"hello", b"12", b"abc"], // start also out of range
        &[b"", b"0", b"abc"],
    ];
    assert_same_many(cases);

    // Never emitted by either program, for any of the above.
    for case in cases {
        for exe in [c_binary(), rust_binary()] {
            let o = run(exe, case, b"");
            let joined = [o.stdout.as_slice(), o.stderr.as_slice()].concat();
            assert!(
                !joined
                    .windows(b"Third argument".len())
                    .any(|w| w == b"Third argument"),
                "unreachable branch fired for {case:?} in {exe:?}"
            );
        }
    }

    // A non-numeric third argument falls through to stop = 0, which then trips
    // the `stop <= start` check (start = 0).
    let c = run(c_binary(), &[b"hello", b"0", b"abc"], b"");
    assert_eq!(c.stdout, b"Error: stop must come after start!\n");
    assert_eq!(c.code, Some(1));
}

/// The empty string as argv[1]: len = 0, so nearly everything is out of range.
#[test]
fn empty_string_input() {
    assert_same_many(&[
        &[b""],
        &[b"", b"0"],
        &[b"", b"1"],
        &[b"", b"-1"],
        &[b"", b""],
        &[b"", b"", b""],
        &[b"", b"0", b"0"],
        &[b"", b"0", b"1"],
        &[b"", b"0", b"-1"],
        &[b"", b"0", b"abc"],
        &[b"", b"abc", b"0"],
    ]);

    // argc == 2 with an empty string prints just the newline.
    let c = run(c_binary(), &[b""], b"");
    assert_eq!(c.stdout, b"\n");
    assert_eq!(c.code, Some(0));
}

/// The program indexes and prints BYTES, not characters: a multi-byte UTF-8
/// sequence can be sliced in half, and argv need not be valid UTF-8 at all.
#[test]
fn byte_oriented_slicing_and_non_utf8_arguments() {
    assert_same_many(&[
        // splits a 2-byte sequence
        &[b"h\xc3\xa9llo", b"0", b"2"],
        &[b"h\xc3\xa9llo", b"1", b"2"],
        &[b"h\xc3\xa9llo", b"2", b"3"],
        &[b"h\xc3\xa9llo", b"1"],
        &[b"h\xc3\xa9llo", b"6"], // == len in bytes
        &[b"h\xc3\xa9llo", b"7"],
        // splits a 3-byte sequence
        &[b"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e", b"3", b"6"],
        &[b"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e", b"1", b"5"],
        // outright invalid UTF-8 in every position
        &[b"a\xffb", b"0", b"3"],
        &[b"\xff\xfe\xfd"],
        &[b"\xff\xfe\xfd", b"1"],
        &[b"\xff\xfe\xfd", b"1", b"3"],
        &[b"\xc3"],
        &[b"\x80\x81\x82", b"0", b"2"],
        &[b"hello", b"\xff"],
        &[b"hello", b"1", b"\xff"],
        &[b"\xff", b"\xff", b"\xff"],
        // embedded newlines / tabs survive verbatim
        &[b"a\nb\tc", b"0", b"5"],
        &[b"a\nb\tc", b"2", b"4"],
    ]);
}

/// Longer inputs, including exactly-at-the-boundary windows.
#[test]
fn long_input_windows() {
    let big = vec![b'z'; 120_000];
    let big_len = big.len().to_string().into_bytes();
    let big_len_minus_1 = (big.len() - 1).to_string().into_bytes();
    let big_len_plus_1 = (big.len() + 1).to_string().into_bytes();

    assert_same(&[&big]);
    assert_same(&[&big, &big_len_minus_1]);
    assert_same(&[&big, &big_len]);
    assert_same(&[&big, &big_len_plus_1]);
    assert_same(&[&big, b"0", &big_len]);
    assert_same(&[&big, b"0", &big_len_plus_1]);
    assert_same(&[&big, b"59999", b"60000"]);

    // Every byte value except NUL (a NUL cannot appear inside an argv string).
    let mixed: Vec<u8> = (1u8..=255).cycle().take(4096).collect();
    assert_same(&[&mixed]);
    assert_same(&[&mixed, b"1000", b"2000"]);
    assert_same(&[&mixed, b"4096"]);
    assert_same(&[&mixed, b"4097"]);
}

/// Exhaustive grid over start/stop around the string bounds -- this is what
/// catches an off-by-one or a flipped comparison in either check.
#[test]
fn exhaustive_start_stop_grid() {
    for s in &["hello", "", "a", "ab", "0123456789"] {
        let sb = s.as_bytes();
        for start in -3i32..=12 {
            let sa = start.to_string().into_bytes();
            // two-argument form
            assert_same(&[sb, &sa]);
            for stop in -3i32..=12 {
                let st = stop.to_string().into_bytes();
                assert_same(&[sb, &sa, &st]);
            }
        }
    }
}

/// A broad structured matrix of "interesting" numeric spellings crossed into
/// both numeric positions.
#[test]
fn numeric_spelling_matrix() {
    let strings: &[&[u8]] = &[b"", b"a", b"hello", b"hello world", b"\xff\xfe", b"\t \n"];
    let nums: &[&[u8]] = &[
        b"",
        b"0",
        b"1",
        b"-1",
        b"5",
        b"6",
        b"abc",
        b"-",
        b"+",
        b" 3",
        b"3 ",
        b"0x2",
        b"2a",
        b"007",
        b"4294967296",
        b"4294967301",
        b"2147483648",
        b"-2147483648",
        b"9223372036854775807",
        b"-9223372036854775808",
        b"99999999999999999999999",
        b"\xff",
        b"\t2",
        b"\r2",
    ];

    for s in strings {
        assert_same(&[s]);
        for a in nums {
            assert_same(&[s, a]);
        }
    }
    // Cross product in both positions for the shorter, most interesting subset.
    let short: &[&[u8]] = &[b"", b"0", b"1", b"-1", b"5", b"6", b"abc", b"4294967296", b"\xff"];
    for s in strings {
        for a in short {
            for b in short {
                assert_same(&[s, a, b]);
            }
        }
    }
}

/// Neither program reads stdin, so piping data in must change nothing.
#[test]
fn stdin_is_ignored() {
    let feed: &[&[u8]] = &[b"", b"garbage\n", b"1 2 3\n4 5 6\n", b"\x00\x01\x02", &[b'x'; 8192]];
    for data in feed {
        assert_same_with_stdin(&[b"hello", b"1", b"3"], data);
        assert_same_with_stdin(&[b"hello"], data);
        assert_same_with_stdin(&[], data);
        assert_same_with_stdin(&[b"hello", b"abc"], data);
        assert_same_with_stdin(&[b"hello", b"0", b"9"], data);
    }
}

/// Nothing is ever written to stderr by either program.
#[test]
fn stderr_is_always_empty() {
    let cases: &[&[&[u8]]] = &[
        &[],
        &[b"hello"],
        &[b"hello", b"abc"],
        &[b"hello", b"-1"],
        &[b"hello", b"0", b"6"],
        &[b"hello", b"3", b"2"],
        &[b"hello", b"0", b"abc"],
        &[b"a", b"b", b"c", b"d"],
    ];
    for case in cases {
        assert_same(case);
        for exe in [c_binary(), rust_binary()] {
            let o = run(exe, case, b"");
            assert!(o.stderr.is_empty(), "{exe:?} wrote to stderr for {case:?}");
        }
    }
}

/// Exit statuses: 0 on the success path, 1 on every error path, and never a
/// signal death or a Rust panic code (101).
#[test]
fn exit_statuses_are_0_or_1() {
    let success: &[&[&[u8]]] = &[
        &[b"hello"],
        &[b""],
        &[b"hello", b"0"],
        &[b"hello", b"5"],
        &[b"hello", b"1", b"3"],
        &[b"hello", b"4294967296"],
        &[b"\xff\xfe\xfd", b"1", b"3"],
    ];
    let failure: &[&[&[u8]]] = &[
        &[],
        &[b"a", b"b", b"c", b"d"],
        &[b"hello", b"abc"],
        &[b"hello", b"-1"],
        &[b"hello", b"6"],
        &[b"hello", b"0", b"6"],
        &[b"hello", b"0", b"-1"],
        &[b"hello", b"3", b"2"],
        &[b"hello", b"0", b"abc"],
    ];
    for (cases, want) in [(success, 0), (failure, 1)] {
        for case in cases {
            assert_same(case);
            for exe in [c_binary(), rust_binary()] {
                let o = run(exe, case, b"");
                assert_eq!(o.signal, None, "{exe:?} died from a signal on {case:?}");
                assert_eq!(o.code, Some(want), "{exe:?} on {case:?}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Output-channel behaviour (this is where the real translation bug was)
// ---------------------------------------------------------------------------

/// Run `exe` with its stdout wired to a socket/pipe whose peer is already
/// closed, so the very first write hits a broken pipe. Returns
/// `(exit_code, signal)`.
///
/// `std::process::Command` resets SIGPIPE to SIG_DFL in the child, exactly as a
/// shell does, so this reproduces `driver hello | true`.
fn run_with_broken_stdout(exe: &Path, args: &[&[u8]]) -> (Option<i32>, Option<i32>) {
    use std::os::unix::io::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::os::unix::process::ExitStatusExt;

    let (reader, writer) = UnixStream::pair().expect("socketpair");
    drop(reader); // the peer is gone: writes now raise SIGPIPE / EPIPE

    let os_args: Vec<OsString> = args
        .iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect();

    let status = Command::new(exe)
        .args(&os_args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {exe:?}: {e}"));

    (status.code(), status.signal())
}

/// A C program writing to a broken stdout is killed by SIGPIPE. The Rust
/// runtime sets SIGPIPE to SIG_IGN before `main`, so without explicitly
/// restoring SIG_DFL the Rust program would survive and exit 0/1 instead.
#[test]
fn broken_stdout_pipe_kills_both_with_sigpipe() {
    let cases: &[&[&[u8]]] = &[
        &[b"hello"],
        &[b""],
        &[b"hello", b"1", b"3"],
        &[b"hello", b"5"],
        // every error path also writes to stdout, so it dies the same way
        &[],
        &[b"hello", b"abc"],
        &[b"hello", b"-1"],
        &[b"hello", b"0", b"6"],
        &[b"hello", b"3", b"2"],
        &[b"a", b"b", b"c", b"d"],
    ];
    for case in cases {
        let c = run_with_broken_stdout(c_binary(), case);
        let r = run_with_broken_stdout(rust_binary(), case);
        assert_eq!(
            c, r,
            "broken-pipe behaviour differs for {case:?}: C={c:?} RUST={r:?}"
        );
        assert_eq!(
            c,
            (None, Some(13)),
            "the C program must be killed by SIGPIPE for {case:?}"
        );
    }
}

// Minimal `signal()` binding so the test can choose the SIGPIPE disposition the
// child inherits, the way different parent processes do.
const SIGPIPE: i32 = 13;
const SIG_IGN: usize = 1;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Like `run_with_broken_stdout`, but the child inherits SIGPIPE = SIG_IGN.
fn run_with_broken_stdout_sigpipe_ignored(
    exe: &Path,
    args: &[&[u8]],
) -> (Option<i32>, Option<i32>) {
    use std::os::unix::io::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;

    let (reader, writer) = UnixStream::pair().expect("socketpair");
    drop(reader);

    let os_args: Vec<OsString> = args
        .iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect();

    let mut cmd = Command::new(exe);
    cmd.args(&os_args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::null());
    unsafe {
        // Runs in the child after fork, before exec: make SIGPIPE ignored so the
        // exec'd program inherits SIG_IGN (ignored dispositions survive execve).
        cmd.pre_exec(|| {
            signal(SIGPIPE, SIG_IGN);
            Ok(())
        });
    }
    let status = cmd.status().unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"));
    (status.code(), status.signal())
}

/// The mirror image of the previous test. When the PARENT has already set
/// SIGPIPE to SIG_IGN, a C program inherits SIG_IGN and *survives* a broken
/// pipe, exiting 0/1 normally. So the Rust must not unconditionally force
/// SIG_DFL -- it has to restore the disposition it actually inherited.
#[test]
fn broken_stdout_pipe_with_inherited_sig_ign_survives_in_both() {
    let cases: &[(&[&[u8]], i32)] = &[
        (&[b"hello"], 0),
        (&[b""], 0),
        (&[b"hello", b"1", b"3"], 0),
        (&[], 1),
        (&[b"hello", b"abc"], 1),
        (&[b"hello", b"9"], 1),
        (&[b"hello", b"3", b"2"], 1),
    ];
    for (args, want) in cases {
        let c = run_with_broken_stdout_sigpipe_ignored(c_binary(), args);
        let r = run_with_broken_stdout_sigpipe_ignored(rust_binary(), args);
        assert_eq!(
            c, r,
            "inherited-SIG_IGN broken-pipe behaviour differs for {args:?}"
        );
        assert_eq!(
            c,
            (Some(*want), None),
            "with SIGPIPE ignored the C program must survive and exit {want} for {args:?}"
        );
    }
}

/// A long payload forces a real `write(2)` mid-run rather than only at exit,
/// exercising the broken-pipe path from inside the printf rather than the
/// final flush.
#[test]
fn broken_stdout_pipe_with_large_payload() {
    let big = vec![b'z'; 60_000];
    let cases: &[&[&[u8]]] = &[&[&big], &[&big, b"0", b"60000"], &[&big, b"30000"]];
    for case in cases {
        let c = run_with_broken_stdout(c_binary(), case);
        let r = run_with_broken_stdout(rust_binary(), case);
        assert_eq!(c, r, "broken-pipe behaviour differs on a large payload");
        assert_eq!(c, (None, Some(13)));
    }
}

/// `printf`'s return value is never checked in the C, so an unwritable stdout
/// changes nothing about the exit status. `/dev/full` fails every write with
/// ENOSPC without raising a signal.
#[test]
fn unwritable_stdout_dev_full_is_ignored_by_both() {
    use std::fs::OpenOptions;
    use std::os::unix::process::ExitStatusExt;

    let cases: &[(&[&[u8]], i32)] = &[
        (&[b"hello"], 0),
        (&[b"hello", b"1", b"3"], 0),
        (&[], 1),
        (&[b"hello", b"abc"], 1),
        (&[b"hello", b"0", b"6"], 1),
    ];
    for (args, want) in cases {
        let mut results = Vec::new();
        for exe in [c_binary(), rust_binary()] {
            let dev_full = OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full");
            let os_args: Vec<OsString> = args
                .iter()
                .map(|a| OsStr::from_bytes(a).to_os_string())
                .collect();
            let st = Command::new(exe)
                .args(&os_args)
                .stdin(Stdio::null())
                .stdout(Stdio::from(dev_full))
                .stderr(Stdio::null())
                .status()
                .expect("spawn");
            results.push((st.code(), st.signal()));
        }
        assert_eq!(
            results[0], results[1],
            "/dev/full behaviour differs for {args:?}"
        );
        assert_eq!(
            results[0],
            (Some(*want), None),
            "write errors must not change the exit status for {args:?}"
        );
    }
}

/// The C never calls `setlocale`, so it stays in the "C" locale and strtol's
/// notion of whitespace/digits cannot be changed by the environment.
#[test]
fn locale_environment_does_not_change_parsing() {
    let envs: &[&[(&str, &str)]] = &[
        &[],
        &[("LC_ALL", "C")],
        &[("LC_ALL", "en_US.UTF-8")],
        &[("LC_NUMERIC", "de_DE.UTF-8")],
        &[("LANG", "tr_TR.UTF-8")],
    ];
    let cases: &[&[&[u8]]] = &[
        &[b"hello", b"1", b"3"],
        &[b"hello", b" 2"],
        &[b"hello", b"1,5"],
        &[b"hello", b"1.5"],
        &[b"hello", b"\xc2\xa02"], // NBSP is not whitespace in the C locale
        &[b"hello", b"abc"],
    ];
    for env in envs {
        for case in cases {
            let mut outs = Vec::new();
            for exe in [c_binary(), rust_binary()] {
                let os_args: Vec<OsString> = case
                    .iter()
                    .map(|a| OsStr::from_bytes(a).to_os_string())
                    .collect();
                let mut cmd = Command::new(exe);
                cmd.args(&os_args).stdin(Stdio::null());
                for (k, v) in env.iter() {
                    cmd.env(k, v);
                }
                let o = cmd.output().expect("spawn");
                outs.push((o.stdout, o.stderr, o.status.code()));
            }
            assert_eq!(
                outs[0], outs[1],
                "locale {env:?} changed behaviour for {case:?}"
            );
        }
    }
}

/// End-to-end spot checks of the exact success-path bytes, so the differential
/// suite cannot pass by both programs being identically wrong.
#[test]
fn success_output_is_exact() {
    let expect: &[(&[&[u8]], &[u8])] = &[
        (&[b"hello"], b"hello\n"),
        (&[b"hello", b"1"], b"ello\n"),
        (&[b"hello", b"1", b"3"], b"el\n"),
        (&[b"hello", b"0", b"5"], b"hello\n"),
        (&[b"hello", b"4", b"5"], b"o\n"),
        (&[b"hello", b"5"], b"\n"),
        (&[b""], b"\n"),
        (&[b"hello", b"4294967296"], b"hello\n"),
        (&[b"hello world", b"6"], b"world\n"),
    ];
    for (args, want) in expect {
        assert_same(args);
        for exe in [c_binary(), rust_binary()] {
            let o = run(exe, args, b"");
            assert_eq!(o.stdout, *want, "{exe:?} on {args:?}");
            assert_eq!(o.code, Some(0));
        }
    }
}

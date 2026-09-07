//! Phase B rows 20-21 and Phase C rows 1-9: the `driver` executable.
//!
//! The C `driver` and the Rust `driver` are run on identical argv and their
//! stdout, stderr and exit status are compared byte-for-byte.

mod harness;

use std::process::Command;

fn run(bin: &std::path::Path, args: &[&str]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    (out.stdout, out.stderr, out.status.code())
}

/// Compares both binaries on one argv. `stderr` is normalized only for
/// `argv[0]`, which necessarily differs (the two executables have different
/// paths); the rest of the message must match exactly.
fn compare(args: &[&str]) {
    let c_bin = harness::c_bin();
    let r_bin = harness::rust_bin();

    let (co, ce, cs) = run(&c_bin, args);
    let (ro, re, rs) = run(&r_bin, args);

    assert_eq!(
        cs,
        rs,
        "exit status diverged for {args:?}: C={cs:?} Rust={rs:?}\nC stdout: {:?}\nR stdout: {:?}",
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro),
    );
    assert_eq!(
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro),
        "stdout diverged for {args:?}"
    );

    let norm = |bytes: &[u8], bin: &std::path::Path| -> Vec<u8> {
        let needle = bin.to_string_lossy().into_owned().into_bytes();
        let mut s = bytes.to_vec();
        if let Some(pos) = s
            .windows(needle.len())
            .position(|w| w == needle.as_slice())
        {
            s.splice(pos..pos + needle.len(), b"<argv0>".iter().copied());
        }
        s
    };
    assert_eq!(
        String::from_utf8_lossy(&norm(&ce, &c_bin)),
        String::from_utf8_lossy(&norm(&re, &r_bin)),
        "stderr diverged for {args:?}"
    );
}

/// Row 20: 20 hand-picked argv shapes + 20 randomized decimal pairs.
#[test]
fn row20_driver_stdout_parity() {
    let fixed: &[&[&str]] = &[
        &["0", "0"],
        &["1", "2"],
        &["-3", "7"],
        &["7", "-3"],
        &["2147483647", "1"],
        &["-2147483648", "-1"],
        &["2147483647", "2147483647"],
        &["-2147483648", "-2147483648"],
        &["65536", "65536"],
        &["46341", "46341"],
        &["abc", "def"],
        &["", ""],
        &["12abc", "3x"],
        &["  -7", "+9"],
        &["--3", "0x10"],
        &["99999999999", "1"],
        &["-99999999999", "1"],
        &["9223372036854775808", "0"],
        &["-9223372036854775808", "0"],
        &["-9223372036854775809", "0"],
        &["0000000000000000005", "-0"],
        &["+", "-"],
        &["\t\n 42", " -042 "],
        &["4294967296", "4294967297"],
    ];
    for args in fixed {
        compare(args);
    }

    let mut rng = harness::Rng::new();
    for _ in 0..20 {
        let a = rng.next_mixed_i32().to_string();
        let b = rng.next_mixed_i32().to_string();
        compare(&[&a, &b]);
    }
}

/// Row 21 / `ERRORS.md` rows 1, 2, 9: argc shapes.
#[test]
fn row21_driver_argc_shapes() {
    compare(&[]); // argc == 1  -> usage, exit 2
    compare(&["5"]); // argc == 2  -> usage, exit 2
    compare(&["5", "6", "7"]); // argc == 4  -> extra arg ignored
    compare(&["5", "6", "7", "8", "9"]); // argc == 6 -> extra args ignored
}

/// `ERRORS.md` rows 1-2 spelled out: the usage path must put nothing on stdout
/// and exit with status 2 on both sides.
#[test]
fn err_rows_1_2_usage_path() {
    for args in [vec![], vec!["only-one".to_string()]] {
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        for bin in [harness::c_bin(), harness::rust_bin()] {
            let (out, err, status) = run(&bin, &refs);
            assert!(
                out.is_empty(),
                "{} wrote to stdout on the usage path: {:?}",
                bin.display(),
                String::from_utf8_lossy(&out)
            );
            assert_eq!(status, Some(2), "{} exit status", bin.display());
            let text = String::from_utf8_lossy(&err);
            assert!(
                text.starts_with("usage: ") && text.ends_with(" A B\n"),
                "{} usage message: {text:?}",
                bin.display()
            );
        }
        compare(&refs);
    }
}

/// `ERRORS.md` rows 3-9: `atoi` never rejects -- it clamps or yields 0 -- and the
/// exact clamped value must match glibc. Exercised through the driver, which is
/// the only public consumer of `atoi`.
#[test]
fn err_rows_3_to_9_atoi_surface() {
    let cases: &[&[&str]] = &[
        // row 3: no digits
        &["abc", "1"],
        &["1", "abc"],
        &["!", "~"],
        // row 4: empty
        &["", "1"],
        &["1", ""],
        // row 5: valid prefix only
        &["12abc", "1"],
        &["  -7x", "1"],
        &["+9", "1"],
        &["--3", "1"],
        &["0x10", "1"],
        &["9,8", "1"],
        &[" +", "1"],
        &["-", "1"],
        // row 6: positive overflow
        &["99999999999", "0"],
        &["2147483648", "0"],
        &["4294967296", "0"],
        &["9223372036854775807", "0"],
        &["9223372036854775808", "0"],
        &["99999999999999999999999999999999999999999", "0"],
        // row 7: negative overflow
        &["-99999999999", "0"],
        &["-2147483649", "0"],
        &["-4294967296", "0"],
        &["-9223372036854775809", "0"],
        &["-99999999999999999999999999999999999999999", "0"],
        // row 8: exactly LONG_MIN
        &["-9223372036854775808", "0"],
        &["-9223372036854775807", "0"],
        // both operands weird at once
        &["-9223372036854775808", "9223372036854775808"],
    ];
    for args in cases {
        compare(args);
    }
}

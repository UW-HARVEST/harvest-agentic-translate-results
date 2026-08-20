//! Phase C — error-path differential tests at the FFI boundary
//! (every row of ERRORS.md that is reachable through `main` in the `.so`).
//!
//! Single `#[test]` function on purpose: fd 1 is redirected while `main` runs
//! (see `common::capture_fd1`).

mod common;

use common::{argv1, fresh_pair, Pair, Rng, SEED};

const TAG: &str = "ffi_errors";

const E1: &[u8] = b"Error: should only be a single (integer) argument!\n";
const E2: &[u8] = b"Error: first argument must be an integer!\n";

/// Asserts that C and Rust agree *and* that they produce the documented
/// rejection (exact message + exit status 1), not merely "some failure".
fn expect_error(pair: &Pair, argc: i32, args: &[Vec<u8>], expected: &[u8], ctx: &str) {
    let (rc, out) = pair.assert_main_same(argc, args, ctx);
    assert_eq!(rc, 1, "{ctx}: expected return value 1, got {rc}");
    assert_eq!(
        out,
        expected,
        "{ctx}: wrong rejection message\n got: {:?}\n want: {:?}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expected)
    );
}

fn expect_ok(pair: &Pair, arg: &[u8], ctx: &str) {
    let (rc, out) = pair.assert_main_same_auto(&argv1(arg), ctx);
    assert_eq!(rc, 0, "{ctx}: expected acceptance (0), got {rc}");
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        10,
        "{ctx}: expected 10 printed lines, got {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn ffi_error_paths_differential() {
    let pair = fresh_pair(TAG);

    // ---- ERRORS.md rows 1-4: argc != 2 -------------------------------------
    let mut args: Vec<Vec<u8>> = vec![b"driver".to_vec()];
    // row 1: argc == 1
    expect_error(&pair, 1, &args, E1, "row 1 (argc == 1)");
    // row 2: argc == 0 (empty argv)
    expect_error(&pair, 0, &args, E1, "row 2 (argc == 0)");
    expect_error(&pair, 0, &[], E1, "row 2 (argc == 0, empty argv array)");
    // rows 3-4: argc == 3, 4 ... 64
    for n in 2..=64usize {
        args.push(format!("arg{n}").into_bytes());
        if args.len() == 2 {
            continue; // argc == 2 is the accepted case, covered elsewhere
        }
        expect_error(
            &pair,
            args.len() as i32,
            &args,
            E1,
            &format!("rows 3/4 (argc == {})", args.len()),
        );
    }

    // ---- ERRORS.md row 5: negative / bogus argc ----------------------------
    // A C `int` accepts any value across the FFI boundary; `argc != 2` must
    // take the error branch for every one of them.
    for argc in [
        -1i32,
        -2,
        -100,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        3,
        1_000_000,
    ] {
        let argv = vec![b"driver".to_vec(), b"1".to_vec(), b"2".to_vec()];
        expect_error(&pair, argc, &argv, E1, &format!("row 5 (argc == {argc})"));
    }

    // ---- ERRORS.md row 6: empty argument -----------------------------------
    expect_error(&pair, 2, &argv1(b""), E2, "row 6 (empty argv[1])");

    // ---- ERRORS.md row 7: whitespace only ----------------------------------
    for arg in [
        &b" "[..],
        b"\t",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\r",
        b"  ",
        b" \t\n\x0b\x0c\r",
        b"\r\n",
        b"          ",
    ] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 7 (whitespace only {:?})", String::from_utf8_lossy(arg)),
        );
    }

    // ---- ERRORS.md row 8: sign only ----------------------------------------
    for arg in [
        &b"+"[..], b"-", b"--", b"++", b"+-", b"-+", b"-+3", b"+-3", b"---5", b" - ", b"\t+",
    ] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 8 (sign only {:?})", String::from_utf8_lossy(arg)),
        );
    }

    // ---- ERRORS.md row 9: first character neither space, sign nor digit ----
    for arg in [
        &b"abc"[..],
        b"x1",
        b".5",
        b"/9",
        b":0",
        b"e5",
        b"#",
        b"A",
        b"z",
        b"_1",
        b"(3)",
        b"\x00",
        b"'7'",
        b"\"8\"",
        b"[1]",
        b"%2",
        b"*3",
        b"$4",
        b"@5",
        b"~6",
        b"^7",
        b"&8",
        b"!9",
        b"?0",
        b"<1",
        b">2",
        b"=3",
        b"|4",
        b"\\5",
        b";6",
        b"`7",
        b"{8",
        b"}9",
    ] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 9 (leading {:?})", String::from_utf8_lossy(arg)),
        );
    }

    // The exact digit-range boundaries: '/' == '0'-1 and ':' == '9'+1.
    for b in [b'/', b':'] {
        expect_error(
            &pair,
            2,
            &argv1(&[b]),
            E2,
            &format!("row 9 (digit boundary char {:?})", b as char),
        );
        expect_error(
            &pair,
            2,
            &argv1(&[b, b'1']),
            E2,
            &format!("row 9 (digit boundary char {:?} + digit)", b as char),
        );
    }
    // ... while '0' and '9' themselves are accepted.
    expect_ok(&pair, b"0", "row 9 control ('0' is a digit)");
    expect_ok(&pair, b"9", "row 9 control ('9' is a digit)");

    // ---- ERRORS.md row 10: sign followed by a non-digit --------------------
    for arg in [
        &b"+a"[..], b"-x", b"+ 1", b"- 1", b"+.1", b"-.1", b"+/", b"-:", b"+_", b"-#",
    ] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 10 ({:?})", String::from_utf8_lossy(arg)),
        );
    }

    // ---- ERRORS.md row 11: whitespace, sign, non-digit ---------------------
    for arg in [
        &b" +z"[..], b"\t-", b" + 1", b" - 1", b"\n\r+q", b"  -  7", b" \t+\t7",
    ] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 11 ({:?})", String::from_utf8_lossy(arg)),
        );
    }

    // ---- ERRORS.md row 12: base-10 prefixes --------------------------------
    for arg in [&b"x10"[..], b"#10", b"b1", b"o7", b"h9", b"0b"] {
        // "0b" starts with a digit, so it is *accepted* as 0.
        if arg.starts_with(b"0") {
            expect_ok(&pair, arg, "row 12 (digit prefix is accepted)");
        } else {
            expect_error(
                &pair,
                2,
                &argv1(arg),
                E2,
                &format!("row 12 ({:?})", String::from_utf8_lossy(arg)),
            );
        }
    }
    expect_ok(&pair, b"0x10", "row 12 control (0x10 parses as 0)");
    expect_ok(&pair, b"0X10", "row 12 control (0X10 parses as 0)");

    // ---- ERRORS.md row 13: non-ASCII / high-bit bytes ----------------------
    for arg in [
        &b"\xff"[..],
        b"\x80",
        b"\x80\x81",
        b"\xc3\xa9",
        b"\xff1",
        b"\xa0 1",
        b"\xe2\x82\xac5",
        b"\xc2\xa0" ,  // NBSP: not isspace() in the "C" locale
    ] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 13 (high-bit bytes {arg:?})"),
        );
    }

    // ---- ERRORS.md row 14: separators that are not accepted ----------------
    for arg in [&b","[..], b"_", b"'", b",5", b"_5", b"'5", b".", b"..1"] {
        expect_error(
            &pair,
            2,
            &argv1(arg),
            E2,
            &format!("row 14 ({:?})", String::from_utf8_lossy(arg)),
        );
    }

    // ---- ERRORS.md row 15: embedded NUL ------------------------------------
    // `main` receives a `char *`, so everything from the NUL on is invisible.
    // (`call_main` appends its own terminator, the NUL here is interior.)
    expect_error(&pair, 2, &argv1(b"\x00"), E2, "row 15 (leading NUL)");
    expect_error(&pair, 2, &argv1(b"\x00123"), E2, "row 15 (NUL then digits)");
    expect_error(&pair, 2, &argv1(b"\x00 abc"), E2, "row 15 (NUL then junk)");
    // A NUL *after* digits truncates the string but keeps the conversion.
    expect_ok(&pair, b"1\x00 2", "row 15 (digits then NUL)");
    expect_ok(&pair, b"-2\x00garbage", "row 15 (negative then NUL)");
    {
        // "1\0" and "1" must produce identical output.
        let a = pair.assert_main_same_auto(&argv1(b"1\x00999"), "row 15 truncation A");
        let b = pair.assert_main_same_auto(&argv1(b"1"), "row 15 truncation B");
        assert_eq!(
            a.1.len(),
            b.1.len(),
            "row 15: interior NUL must truncate the argument"
        );
    }

    // ---- ERRORS.md row 16/17: out-of-range values are NOT rejected ---------
    for arg in [
        "9223372036854775808",   // LONG_MAX + 1  -> saturates, truncates to -1
        "-9223372036854775809",  // LONG_MIN - 1  -> saturates, truncates to 0
        "99999999999999999999999999999999999999",
        "-99999999999999999999999999999999999999",
        "2147483648",            // INT_MAX + 1
        "-2147483649",           // INT_MIN - 1
        "4294967296",            // 2^32 -> 0
    ] {
        expect_ok(&pair, arg.as_bytes(), &format!("row 16/17 ({arg} accepted)"));
    }

    // ---- ERRORS.md row 18: trailing garbage is NOT rejected ----------------
    for arg in [&b"5abc"[..], b"3 4", b"7\n", b"1)"] {
        expect_ok(&pair, arg, "row 18 (prefix accepted)");
    }

    // ---- ERRORS.md row 19: static_sum has no error path --------------------
    for u in [0, 1, -1, i32::MAX, i32::MIN] {
        assert_eq!(
            pair.c.static_sum(u),
            pair.rust.static_sum(u),
            "row 19 (static_sum({u}) diverged)"
        );
    }

    // ---- randomized fuzz over the accept/reject decision -------------------
    // Whatever the C decides (accept, E1, E2), Rust must decide identically.
    let mut rng = Rng::new(SEED ^ 0xE1);
    let alphabet: &[u8] =
        b"0123456789+- \t\n\r\x0b\x0cabcxXeE.,_'/:\xff\x80\xc3\xa9()[]{}#$%&*";
    let mut seen_ok = 0usize;
    let mut seen_e2 = 0usize;
    for _ in 0..600 {
        let len = rng.below(8) as usize;
        let arg: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        let (rc, out) = pair.assert_main_same_auto(&argv1(&arg), "randomized accept/reject");
        if rc == 1 {
            assert_eq!(out, E2, "unexpected rejection message for {arg:?}");
            seen_e2 += 1;
        } else {
            seen_ok += 1;
        }
    }
    assert!(
        seen_ok > 20 && seen_e2 > 20,
        "fuzzing should hit both outcomes (ok = {seen_ok}, rejected = {seen_e2})"
    );

    // ---- randomized bogus argc --------------------------------------------
    for _ in 0..200 {
        let argc = rng.next_i32();
        let argv = vec![
            b"driver".to_vec(),
            b"5".to_vec(),
            b"x".to_vec(),
            b"y".to_vec(),
        ];
        if argc == 2 {
            continue;
        }
        expect_error(
            &pair,
            argc,
            &argv,
            E1,
            &format!("randomized bogus argc ({argc})"),
        );
    }

    // ---- generic FFI boundary: argv == NULL --------------------------------
    // The C code touches `argv` only when `argc == 2`, so `main(argc, NULL)` is
    // a well-defined input for every other argc and must yield E1.
    for argc in [0i32, 1, 3, 4, 64, -1, i32::MIN, i32::MAX] {
        let (rc, out) = pair.assert_main_same_null_argv(argc, "argv == NULL");
        assert_eq!(rc, 1, "main({argc}, NULL): expected 1");
        assert_eq!(
            out, E1,
            "main({argc}, NULL): expected the argc error message"
        );
    }

    // ---- generic FFI boundary: oversized argument ---------------------------
    // Very long inputs (far beyond any buffer the C code has) must behave the
    // same: a huge digit run saturates, a huge run of junk is rejected.
    for len in [1_000usize, 10_000, 100_000] {
        let digits = vec![b'7'; len];
        expect_ok(&pair, &digits, &format!("oversized digits ({len})"));

        let mut spaces = vec![b' '; len];
        spaces.push(b'5');
        expect_ok(&pair, &spaces, &format!("oversized whitespace ({len})"));

        let junk = vec![b'z'; len];
        expect_error(
            &pair,
            2,
            &argv1(&junk),
            E2,
            &format!("oversized junk ({len})"),
        );

        let mut only_spaces = vec![b' '; len];
        only_spaces[len / 2] = b'\t';
        expect_error(
            &pair,
            2,
            &argv1(&only_spaces),
            E2,
            &format!("oversized whitespace only ({len})"),
        );
    }

    // ERRORS.md row 21 (argc == 2 with argv[1] == NULL) is undefined behaviour
    // in C (`strtol(NULL, ...)` dereferences a null pointer) and faults in both
    // builds; it is documented rather than asserted.
}

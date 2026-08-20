//! Phase C — error-path differential tests, one per row of ERRORS.md.
//!
//! Both implementations are driven through their exported `main` symbol with a
//! hand-built `argv`, with fds 1 and 2 redirected to temp files; the captured
//! bytes and the returned `int` must be identical (same exit code, same message,
//! not merely "both failed").
//!
//! Every `argc == 2` case here is asserted (via real glibc `strtoul`) to be one
//! the C *rejects*, so no test accidentally starts the ~5-minute compute loop.

mod common;

use common::{assert_main_matches, pairs, Pairs, Rng};
use std::ffi::CString;
use std::os::raw::c_char;

const UINT_MAX: u64 = u32::MAX as u64;

/// True when the C source's validation block rejects `arg` (real glibc strtoul).
fn c_rejects(arg: &[u8]) -> bool {
    let c = CString::new(arg).expect("no interior NUL");
    unsafe {
        *libc::__errno_location() = 0;
        let mut endptr: *mut c_char = std::ptr::null_mut();
        let temp_seed = libc::strtoul(c.as_ptr(), &mut endptr, 10);
        let errno = *libc::__errno_location();
        *endptr != 0 || errno != 0 || temp_seed > UINT_MAX
    }
}

/// `main(2, {argv0, arg})` on both sides — refuses to run if C would accept
/// `arg` (that would take ~5 minutes per side).
fn assert_invalid_seed(p: &Pairs, arg: &[u8]) {
    assert!(
        c_rejects(arg),
        "test bug: {:?} is *accepted* by C; it must not be used as a fast error case",
        String::from_utf8_lossy(arg)
    );
    assert_main_matches(
        &format!("invalid seed {:?}", String::from_utf8_lossy(arg)),
        p,
        2,
        &[Some(b"driver"), Some(arg)],
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md site 1 — argc != 2
// ---------------------------------------------------------------------------

/// ERRORS.md row 1.
#[test]
fn argc_zero_null_argv0() {
    let p = pairs();
    assert_main_matches("argc=0, argv[0]=NULL", &p, 0, &[None]);

    // and make sure it really is glibc's "(null)" rendering
    let argv = common::Argv::new(&[None]);
    let c = common::run_main(&p.c[0], 0, &argv);
    assert_eq!(c.status, 1);
    assert_eq!(c.stderr, b"Usage: (null) <seed>\n");
    assert!(c.stdout.is_empty());
}

/// ERRORS.md row 2 — C reads `argv[0]` even when `argc == 0`.
#[test]
fn argc_zero_with_argv0() {
    let p = pairs();
    assert_main_matches("argc=0, argv[0]=driver", &p, 0, &[Some(b"driver")]);
    assert_main_matches(
        "argc=0, argv[0]=./x, argv[1]=42",
        &p,
        0,
        &[Some(b"./x"), Some(b"42")],
    );
}

/// ERRORS.md rows 3, 4, 5, 8.
#[test]
fn argc_wrong_counts() {
    let p = pairs();

    // argc == 1
    assert_main_matches("argc=1", &p, 1, &[Some(b"driver")]);
    assert_main_matches("argc=1, empty argv0", &p, 1, &[Some(b"")]);
    assert_main_matches("argc=1, long argv0", &p, 1, &[Some(&vec![b'p'; 300])]);

    // argc == 3 .. 8 (the extra args, valid or not, are never looked at)
    let all: Vec<Option<&[u8]>> = vec![
        Some(b"driver"),
        Some(b"42"),
        Some(b"extra"),
        Some(b"4294967296"),
        Some(b""),
        Some(b"-1"),
        Some(b"abc"),
        Some(b"7"),
    ];
    for argc in 3..=8usize {
        assert_main_matches(&format!("argc={argc}"), &p, argc as i32, &all[..argc]);
    }

    // argc != 2 with an empty argv[0] -> "Usage:  <seed>\n"
    assert_main_matches(
        "argc=3, empty argv0",
        &p,
        3,
        &[Some(b""), Some(b"1"), Some(b"2")],
    );
}

/// ERRORS.md rows 6, 7 — out-of-domain `int` values for `argc` across the FFI
/// boundary (the C only ever tests `argc != 2`).
#[test]
fn argc_out_of_range() {
    let p = pairs();
    for argc in [-1i32, -2, -1000, i32::MIN, i32::MAX, i32::MAX - 1, 100, 65_536] {
        assert_main_matches(
            &format!("argc={argc} (out of domain)"),
            &p,
            argc,
            &[Some(b"driver"), Some(b"42")],
        );
    }
}

/// ERRORS.md row 9 — non-UTF-8 bytes in `argv[0]` must be echoed unchanged.
#[test]
fn argv0_non_utf8() {
    let p = pairs();
    for argv0 in [
        &b"\xff\xfe"[..],
        &b"pre\xffpost"[..],
        &b"\x80\x81\x82"[..],
        &b"\xc3\x28"[..],
        &b"\xed\xa0\x80"[..], // UTF-16 surrogate encoded in UTF-8
    ] {
        assert_main_matches("non-UTF-8 argv[0]", &p, 1, &[Some(argv0)]);
        assert_main_matches("non-UTF-8 argv[0], argc=3", &p, 3, &[Some(argv0), Some(b"1"), Some(b"2")]);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md site 2 — the three terms of the seed-validation condition
// ---------------------------------------------------------------------------

/// ERRORS.md rows 10–18 (`*endptr != '\0'`), 19–22 (`ERANGE`),
/// 23–27 (`> UINT_MAX`) — all the way through `main`, comparing the exact
/// `Invalid seed: '...'` bytes.
#[test]
fn invalid_seed_strings() {
    let p = pairs();
    let cases: Vec<&[u8]> = vec![
        // *endptr != '\0'
        b"abc",
        b"42abc",
        b"1x",
        b"12 34",
        b"   ",
        b" ",
        b"\t",
        b"\n",
        b"-",
        b"+",
        b"--5",
        b"- 5",
        b"+ 42",
        b"0x10",
        b"0b1",
        b"010z",
        b"42 ",
        b"42\n",
        b" 42 ",
        b"1,000",
        b"1_000",
        b"1.0",
        b"1e3",
        b".5",
        b"4\xff",
        b"\xff",
        b"\xc3\xa9",
        b"\xd9\xa1\xd9\xa2\xd9\xa3",
        b"seed",
        // errno == ERANGE
        b"18446744073709551616",
        b"99999999999999999999",
        b"-18446744073709551616",
        // temp_seed > UINT_MAX
        b"4294967296",
        b"4294967297",
        b"18446744073709551615",
        b"9223372036854775808",
        b"-1",
        b"-2",
        b"-4294967295",
        b"-4294967296",
    ];
    for c in cases {
        assert_invalid_seed(&p, c);
    }
}

/// ERRORS.md row 35 — a very long `argv[1]`, echoed in full.
#[test]
fn long_argument() {
    let p = pairs();
    assert_invalid_seed(&p, &vec![b'9'; 4096]);
    assert_invalid_seed(&p, &vec![b'x'; 4096]);
    assert_invalid_seed(&p, &[vec![b'0'; 4000], b"4294967296".to_vec()].concat());
    assert_invalid_seed(&p, &[vec![b' '; 2048], vec![b'9'; 2048]].concat());
}

/// Randomised error-path sweep: 600 pseudo-random arguments, keeping only those
/// the C rejects (the accepted ones are covered by `parse.rs` / `pipeline.rs`).
#[test]
fn random_invalid_arguments() {
    let p = pairs();
    let alphabet: &[u8] = b"0123456789+- \t\n\x0b\x0c\raAxX.,eE_\xff\x80";
    let mut rng = Rng::new(0x5EED_7000);
    let mut checked = 0usize;
    let mut attempts = 0usize;
    while checked < 600 && attempts < 20_000 {
        attempts += 1;
        let len = 1 + rng.below(20) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
            .collect();
        if !c_rejects(&s) {
            continue; // would run for 5 minutes
        }
        assert_invalid_seed(&p, &s);
        checked += 1;
    }
    assert_eq!(checked, 600, "generated only {checked} rejected arguments");
}

/// Message-shape regression: the error text is produced by two different
/// `fprintf`s; make sure neither implementation confuses them.
#[test]
fn error_messages_are_distinct_and_exact() {
    let p = pairs();

    let argv = common::Argv::from_strs(&[b"driver"]);
    for imp in p.c.iter().chain(std::iter::once(&p.rust)) {
        let got = common::run_main(imp, 1, &argv);
        assert_eq!(got.status, 1, "{}", imp.name);
        assert_eq!(got.stderr, b"Usage: driver <seed>\n", "{}", imp.name);
        assert!(got.stdout.is_empty(), "{}", imp.name);
    }

    let argv = common::Argv::from_strs(&[b"driver", b"abc"]);
    for imp in p.c.iter().chain(std::iter::once(&p.rust)) {
        let got = common::run_main(imp, 2, &argv);
        assert_eq!(got.status, 1, "{}", imp.name);
        assert_eq!(got.stderr, b"Invalid seed: 'abc'\n", "{}", imp.name);
        assert!(got.stdout.is_empty(), "{}", imp.name);
    }
}

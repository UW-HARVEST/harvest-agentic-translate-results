//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlopen`ed
//! exports of both the C `.so` and the Rust `.so`; the return value and the
//! exact bytes written to fd 1 / fd 2 must match byte for byte.

mod common;

use common::*;
use std::ffi::{CString, c_int};

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// Row 1 — forward_goto_example, x == 0 (branch boundary)
// ===========================================================================
#[test]
fn row01_fge_zero() {
    diff_fge(0);
}

// ===========================================================================
// Row 2 — forward_goto_example, 400 random x in 1..2^30 (no overflow)
// ===========================================================================
#[test]
fn row02_fge_positive_no_overflow() {
    let mut rng = Rng::new(SEED ^ 2);
    diff_fge(1);
    diff_fge(2);
    for _ in 0..400 {
        let x = rng.range_i64(1, (1i64 << 30) - 1) as c_int;
        diff_fge(x);
    }
}

// ===========================================================================
// Row 3 — forward_goto_example, largest non-overflowing inputs
// ===========================================================================
#[test]
fn row03_fge_half_intmax_boundary() {
    for x in [0x3FFF_FFFEi32, 0x3FFF_FFFF] {
        diff_fge(x);
    }
}

// ===========================================================================
// Row 4 — forward_goto_example, x*2 overflows (wraps negative)
// ===========================================================================
#[test]
fn row04_fge_overflow() {
    for x in [
        0x4000_0000i32,
        0x4000_0001,
        0x5555_5555,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_fge(x);
    }
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..200 {
        let x = rng.range_i64(1 << 30, i32::MAX as i64) as c_int;
        diff_fge(x);
    }
}

// ===========================================================================
// Row 5 — forward_goto_example, 4096 random i32 over the whole range,
//         all inside one process (mixed classes, sequenced)
// ===========================================================================
#[test]
fn row05_fge_random_full_range() {
    let mut rng = Rng::new(SEED ^ 5);
    let xs: Vec<i32> = (0..4096).map(|_| rng.i32()).collect();

    // Sequenced: every value, one capture each.
    for &x in &xs[..256] {
        diff_fge(x);
    }

    // And all of them inside a single capture, so the cumulative stdout buffer
    // state is part of the comparison.
    diff_batch("forward_goto_example/4096-in-one-capture", false, |api| {
        let mut rets = Vec::with_capacity(xs.len());
        for &x in &xs {
            rets.push(unsafe { (api.forward_goto_example)(x) });
        }
        rets
    });
}

// ===========================================================================
// Row 6 — open_with_cleanup, empty regular file
// ===========================================================================
#[test]
fn row06_owc_empty_file() {
    diff_owc_content("empty", b"");
}

// ===========================================================================
// Row 7 — open_with_cleanup, one newline-terminated line, random bodies
// ===========================================================================
#[test]
fn row07_owc_single_line_newline() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..200 {
        let len = rng.below(80) as usize + 1;
        let mut c = rng.ascii_line(len);
        c.push(b'\n');
        diff_owc_content("line-nl", &c);
    }
}

// ===========================================================================
// Row 8 — open_with_cleanup, one line WITHOUT trailing newline
// ===========================================================================
#[test]
fn row08_owc_single_line_no_newline() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..200 {
        let len = rng.below(80) as usize + 1;
        let c = rng.ascii_line(len);
        diff_owc_content("line-nonl", &c);
    }
}

// ===========================================================================
// Row 9 — open_with_cleanup, exact fgets(…, 100) boundaries
// ===========================================================================
#[test]
fn row09_owc_fgets_buffer_boundaries() {
    let mut rng = Rng::new(SEED ^ 9);
    for len in [
        0usize, 1, 2, 96, 97, 98, 99, 100, 101, 102, 196, 197, 198, 199, 200, 201, 297, 298, 299,
        300,
    ] {
        for nl in [false, true] {
            let mut c = rng.ascii_line(len);
            if nl {
                c.push(b'\n');
            }
            diff_owc_content(&format!("bnd-{len}-{nl}"), &c);

            // Same boundary but as the *second* line, so the loop has already
            // iterated once.
            let mut c2 = b"first\n".to_vec();
            c2.extend_from_slice(&rng.ascii_line(len));
            if nl {
                c2.push(b'\n');
            }
            diff_owc_content(&format!("bnd2-{len}-{nl}"), &c2);
        }
    }
}

// ===========================================================================
// Row 10 — open_with_cleanup, many random lines
// ===========================================================================
#[test]
fn row10_owc_many_random_lines() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..150 {
        let lines = rng.below(40) + 1;
        let mut c = Vec::new();
        for _ in 0..lines {
            let len = rng.below(251) as usize;
            c.extend_from_slice(&rng.ascii_line(len));
            c.push(b'\n');
        }
        if !rng.bool() {
            // drop the final newline
            c.pop();
        }
        diff_owc_content("many-lines", &c);
    }
}

// ===========================================================================
// Row 11 — open_with_cleanup, embedded NUL bytes (printf("%s") truncates)
// ===========================================================================
#[test]
fn row11_owc_embedded_nuls() {
    diff_owc_content("nul-only", b"\0");
    diff_owc_content("nul-start", b"\0abc\n");
    diff_owc_content("nul-mid", b"ab\0cd\n");
    diff_owc_content("nul-end", b"abcd\0\n");
    diff_owc_content("nul-lines", b"a\0b\nc\0d\ne\0f\n");
    diff_owc_content("nul-no-nl", b"abc\0def");
    // A NUL exactly at the fgets boundary.
    let mut c = vec![b'z'; 98];
    c.push(0);
    c.extend_from_slice(b"tail\n");
    diff_owc_content("nul-at-98", &c);
    let mut c = vec![b'z'; 99];
    c.push(0);
    c.extend_from_slice(b"tail\n");
    diff_owc_content("nul-at-99", &c);

    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..100 {
        let len = rng.below(400) as usize + 1;
        let mut c = rng.ascii_line(len);
        // pepper with NULs and newlines
        let holes = rng.below(8) + 1;
        for _ in 0..holes {
            let i = rng.below(c.len() as u64) as usize;
            c[i] = if rng.bool() { 0 } else { b'\n' };
        }
        diff_owc_content("nul-random", &c);
    }
}

// ===========================================================================
// Row 12 — open_with_cleanup, fully random binary content
// ===========================================================================
#[test]
fn row12_owc_random_binary() {
    // Every byte value, and printf-conversion look-alikes that must be copied
    // verbatim because the C code passes the buffer as an argument, not as the
    // format string.
    let all: Vec<u8> = (0u8..=255).collect();
    diff_owc_content("all-bytes", &all);
    diff_owc_content("pct-s", b"%s %d %n %%\n");
    diff_owc_content("pct-only", b"%");
    diff_owc_content("crlf", b"a\r\nb\r\n");

    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..100 {
        let len = rng.below(4096) as usize + 1;
        let c = rng.bytes(len);
        diff_owc_content("binary", &c);
    }
}

// ===========================================================================
// Row 13 — open_with_cleanup, file of only newlines
// ===========================================================================
#[test]
fn row13_owc_only_newlines() {
    for n in [1usize, 2, 3, 100, 5000] {
        diff_owc_content(&format!("nl-{n}"), &vec![b'\n'; n]);
    }
}

// ===========================================================================
// Row 14 — open_with_cleanup, large file (many stdout buffer flushes)
// ===========================================================================
#[test]
fn row14_owc_large_file() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut c = Vec::with_capacity(80 * 1024);
    while c.len() < 80 * 1024 {
        let len = rng.below(120) as usize;
        c.extend_from_slice(&rng.ascii_line(len));
        c.push(b'\n');
    }
    diff_owc_content("large-lines", &c);

    // 64 KiB with no newline at all: every fgets fills the whole buffer.
    let c2 = rng.ascii_line(64 * 1024);
    diff_owc_content("large-oneline", &c2);
}

// ===========================================================================
// Row 15 — open_with_cleanup, /dev/null
// ===========================================================================
#[test]
fn row15_owc_dev_null() {
    let p = CString::new("/dev/null").unwrap();
    diff_owc(Some(&p));
}

// ===========================================================================
// Row 16 — open_with_cleanup, file name containing non-UTF-8 bytes
// ===========================================================================
#[test]
fn row16_owc_non_utf8_name() {
    let f = fixture_raw_name(b"na\xff\xfeme-\x80", b"contents\n");
    diff_owc(Some(&f));
    // and the same weird name on the failure path
    let missing = {
        let mut b = f.as_bytes().to_vec();
        b.extend_from_slice(b"-nope");
        CString::new(b).unwrap()
    };
    diff_owc(Some(&missing));
}

// ===========================================================================
// Row 17 — open_with_cleanup success: state of the returned FILE*
//          (already compared by diff_owc; this row pins the interesting
//           shapes explicitly and asserts the stream really is usable)
// ===========================================================================
#[test]
fn row17_owc_returned_stream_state() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", b"".to_vec()),
        ("one-nl", b"hello\n".to_vec()),
        ("one-nonl", b"hello".to_vec()),
        ("multi", b"a\nb\nc\n".to_vec()),
        ("big", vec![b'q'; 70 * 1024]),
    ];
    for (tag, content) in cases {
        // The differential comparison covers ferror/feof/ftell/fgetc/fclose.
        diff_owc_content(tag, &content);

        // Sanity: the C library really does hand back a live stream at EOF, so
        // the comparison above is not vacuous.
        let f = fixture(tag, &content);
        let (state, _) = capture(|| owc_and_close(c_api(), f.as_ptr()));
        assert!(!state.is_null, "{tag}: C returned NULL unexpectedly");
        assert_eq!(state.ferror, 0, "{tag}");
        assert_eq!(state.next_char, -1, "{tag}: expected EOF");
        assert_eq!(state.ftell, content.len() as i64, "{tag}");
        assert_eq!(state.fclose_ret, 0, "{tag}");
    }
}

// ===========================================================================
// Row 18 — driver, num < 0 with a perfectly good file: the file must never be
//          opened (short circuit) → -1
// ===========================================================================
#[test]
fn row18_driver_negative_num_valid_file() {
    let f = fixture("driver-neg", b"line one\nline two\n");
    diff_driver(-1, Some(&f));
    diff_driver(i32::MIN, Some(&f));
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..200 {
        let n = rng.range_i64(i32::MIN as i64, -1) as c_int;
        diff_driver(n, Some(&f));
    }
}

// ===========================================================================
// Row 19 — driver, num == 0 with a single-line file
// ===========================================================================
#[test]
fn row19_driver_zero_single_line() {
    let f = fixture("driver-zero", b"only line\n");
    diff_driver(0, Some(&f));
}

// ===========================================================================
// Row 20 — driver, random num × random multi-line files
// ===========================================================================
#[test]
fn row20_driver_random_num_and_file() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..150 {
        let lines = rng.below(20) + 1;
        let mut c = Vec::new();
        for _ in 0..lines {
            let n = rng.below(220) as usize;
            c.extend_from_slice(&rng.ascii_line(n));
            c.push(b'\n');
        }
        let num = rng.range_i64(0, (1i64 << 30) - 1) as c_int;
        diff_driver_content("driver-multi", num, &c);
    }
}

// ===========================================================================
// Row 21 — driver, valid num × empty file
// ===========================================================================
#[test]
fn row21_driver_empty_file() {
    diff_driver_content("driver-empty", 7, b"");
    diff_driver_content("driver-empty0", 0, b"");
}

// ===========================================================================
// Row 22 — driver, num whose doubling overflows → res is negative but not -1
// ===========================================================================
#[test]
fn row22_driver_overflowing_num() {
    let f = fixture("driver-ovf", b"data\n");
    for n in [
        0x3FFF_FFFFi32,
        0x4000_0000,
        0x4000_0001,
        0x7FFF_FFFE,
        i32::MAX,
    ] {
        diff_driver(n, Some(&f));
    }
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..100 {
        let n = rng.range_i64(1 << 30, i32::MAX as i64) as c_int;
        diff_driver(n, Some(&f));
    }
}

// ===========================================================================
// Row 23 — driver, valid num × random binary file
// ===========================================================================
#[test]
fn row23_driver_binary_file() {
    let mut rng = Rng::new(SEED ^ 23);
    diff_driver_content("driver-allbytes", 3, &(0u8..=255).collect::<Vec<u8>>());
    for _ in 0..60 {
        let len = rng.below(3000) as usize + 1;
        let c = rng.bytes(len);
        let num = rng.range_i64(0, 1000) as c_int;
        diff_driver_content("driver-bin", num, &c);
    }
}

// ===========================================================================
// Row 24 — driver, /dev/null and a non-UTF-8 file name
// ===========================================================================
#[test]
fn row24_driver_devnull_and_weird_name() {
    let devnull = CString::new("/dev/null").unwrap();
    diff_driver(5, Some(&devnull));
    diff_driver(-5, Some(&devnull));

    let f = fixture_raw_name(b"dr\xc3(iver\xff", b"weird name\ncontent\n");
    diff_driver(11, Some(&f));
}

// ===========================================================================
// Row 25 — stdout and stderr on the SAME fd: interleaving of the buffered
//          stdout writes and the unbuffered stderr writes is observable
// ===========================================================================
#[test]
fn row25_merged_streams_interleaving() {
    let good = fixture("merged-good", b"alpha\nbeta\ngamma\n");
    let empty = fixture("merged-empty", b"");
    let big = fixture("merged-big", &vec![b'K'; 9000]);
    let missing = missing_path();
    let adir = dir_path();

    let goodp = good.as_ptr() as usize;
    let emptyp = empty.as_ptr() as usize;
    let bigp = big.as_ptr() as usize;
    let missingp = missing.as_ptr() as usize;
    let adirp = adir.as_ptr() as usize;

    diff_batch("mixed-merged", true, move |api| {
        let mut rets: Vec<i64> = Vec::new();
        unsafe {
            rets.push((api.forward_goto_example)(21) as i64);
            rets.push((api.forward_goto_example)(-21) as i64);
            rets.push((api.driver)(3, goodp as *const _) as i64);
            rets.push((api.driver)(-3, goodp as *const _) as i64);
            rets.push((api.driver)(4, missingp as *const _) as i64);
            rets.push((api.driver)(4, adirp as *const _) as i64);
            rets.push((api.driver)(4, std::ptr::null()) as i64);
            rets.push(owc_and_close(api, emptyp as *const _).is_null as i64);
            rets.push(owc_and_close(api, bigp as *const _).ftell);
            rets.push(owc_and_close(api, missingp as *const _).is_null as i64);
            rets.push((api.forward_goto_example)(i32::MAX) as i64);
            rets.push((api.driver)(0, bigp as *const _) as i64);
        }
        rets
    });
}

// ===========================================================================
// Row 26 — long mixed session: 300 randomly chosen calls inside one capture
// ===========================================================================
#[test]
fn row26_long_mixed_session() {
    let mut rng = Rng::new(SEED ^ 26);

    // Pre-create a pool of fixtures (valid and invalid).
    let mut paths: Vec<CString> = Vec::new();
    for _ in 0..12 {
        let lines = rng.below(6) + 1;
        let mut c = Vec::new();
        for _ in 0..lines {
            let n = rng.below(150) as usize;
            c.extend_from_slice(&rng.ascii_line(n));
            c.push(b'\n');
        }
        paths.push(fixture("sess", &c));
    }
    paths.push(fixture("sess-empty", b""));
    paths.push(fixture("sess-nul", b"a\0b\nc\0d\n"));
    paths.push(missing_path());
    paths.push(dir_path());
    paths.push(CString::new("/dev/null").unwrap());
    paths.push(CString::new("").unwrap());

    // Script the session once so both libraries execute exactly the same
    // sequence of calls.
    enum Op {
        Fge(i32),
        Owc(usize),
        Drv(i32, usize),
        DrvNull(i32),
        OwcNull,
    }
    let script: Vec<Op> = (0..300)
        .map(|_| {
            let x = rng.i32();
            let pi = rng.below(paths.len() as u64) as usize;
            match rng.below(5) {
                0 => Op::Fge(x),
                1 => Op::Owc(pi),
                2 => Op::Drv(x >> rng.below(31) as u32, pi),
                3 => Op::DrvNull(x),
                _ => {
                    if rng.below(10) == 0 {
                        Op::OwcNull
                    } else {
                        Op::Drv(x.wrapping_abs().wrapping_add(1), pi)
                    }
                }
            }
        })
        .collect();

    let ptrs: Vec<usize> = paths.iter().map(|p| p.as_ptr() as usize).collect();

    diff_batch("long-mixed-session", false, |api| {
        let mut rets: Vec<i64> = Vec::new();
        for op in &script {
            unsafe {
                match *op {
                    Op::Fge(x) => rets.push((api.forward_goto_example)(x) as i64),
                    Op::Owc(i) => {
                        let s = owc_and_close(api, ptrs[i] as *const _);
                        rets.push(s.is_null as i64);
                        rets.push(s.ftell);
                        rets.push(s.next_char as i64);
                        rets.push(s.fclose_ret as i64);
                    }
                    Op::Drv(n, i) => rets.push((api.driver)(n, ptrs[i] as *const _) as i64),
                    Op::DrvNull(n) => rets.push((api.driver)(n, std::ptr::null()) as i64),
                    Op::OwcNull => {
                        rets.push(owc_and_close(api, std::ptr::null()).is_null as i64)
                    }
                }
            }
        }
        rets
    });

    // …and again with both streams merged onto one fd.
    diff_batch("long-mixed-session-merged", true, |api| {
        let mut rets: Vec<i64> = Vec::new();
        for op in &script {
            unsafe {
                match *op {
                    Op::Fge(x) => rets.push((api.forward_goto_example)(x) as i64),
                    Op::Owc(i) => rets.push(owc_and_close(api, ptrs[i] as *const _).ftell),
                    Op::Drv(n, i) => rets.push((api.driver)(n, ptrs[i] as *const _) as i64),
                    Op::DrvNull(n) => rets.push((api.driver)(n, std::ptr::null()) as i64),
                    Op::OwcNull => {
                        rets.push(owc_and_close(api, std::ptr::null()).is_null as i64)
                    }
                }
            }
        }
        rets
    });
}

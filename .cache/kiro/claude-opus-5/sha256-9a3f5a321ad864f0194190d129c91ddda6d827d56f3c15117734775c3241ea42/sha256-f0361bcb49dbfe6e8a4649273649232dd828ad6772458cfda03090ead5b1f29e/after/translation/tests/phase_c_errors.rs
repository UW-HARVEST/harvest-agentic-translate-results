//! Phase C — error / rejection-path differential tests. One test per row of
//! `ERRORS.md`, plus the generic FFI boundary cases (null pointers, zero and
//! oversized lengths, and scalars one step past / far outside the valid range).
//!
//! Cases that terminate the process (`exit(30)` on allocation failure, SIGSEGV
//! on a null pointer) are run in a subprocess for BOTH libraries and compared
//! on exit status, terminating signal, and stderr bytes — not merely "both
//! failed somehow".

mod common;

use common::*;
use std::os::raw::{c_char, c_int};
use std::process::Command;

// =============================== E1 .. E5 ==================================

// E1
#[test]
fn e1_extract_no_match_returns_input_pointer() {
    let mut rng = Rng::new(0xE001);
    for _ in 0..2000 {
        let body = rand_from_range(&mut rng, 1, 64, b"abcdefghijklmnopqrstuvwxyz0123456789.-_");
        let path = cstr(&body);
        assert_eq!(
            diff_extract(&path, b'/'),
            0,
            "no-match must return the identical input pointer"
        );
        assert_eq!(diff_extract(&path, b'\\'), 0);
        assert_eq!(diff_extract(&path, 0x80), 0);
    }
}

// E2
#[test]
fn e2_extract_empty_path() {
    let path = cstr(b"");
    for s in [b'/', b'\\', b'a', 0x7f, 0x80, 0xff] {
        assert_eq!(diff_extract(&path, s), 0);
    }
}

// E3
#[test]
fn e3_extract_nul_separator() {
    for body in [
        &b""[..],
        b"a",
        b"/",
        b"abc",
        b"a/b/c",
        b"trailing/",
        &[0xffu8, 0x80, 0x01][..],
    ] {
        let path = cstr(body);
        let off = diff_extract(&path, 0);
        assert_eq!(
            off,
            body.len() as isize + 1,
            "NUL separator must return one byte past the terminator"
        );
    }
    // and via the `int`-typed signature, with every value congruent to 0 mod 256
    let path = cstr(b"a/b/c");
    for k in 0..=8i32 {
        diff_extract_int(&path, k * 256);
        diff_extract_int(&path, -k * 256);
    }
}

// E4
#[test]
fn e4_extract_high_bit_separator() {
    let mut rng = Rng::new(0xE004);
    let high: Vec<u8> = (0x80u16..=0xff).map(|b| b as u8).collect();
    for _ in 0..2000 {
        let mut body = rand_from_range(&mut rng, 1, 48, b"abc/def");
        if rng.bool() {
            let at = rng.below(body.len());
            body[at] = high[rng.below(high.len())];
        }
        let path = cstr(&body);
        diff_extract(&path, high[rng.below(high.len())]);
    }
    // exhaustive: every high separator against a path holding every high byte
    let all_high = cstr(&high);
    for &s in &high {
        diff_extract(&all_high, s);
    }
    // and against a path with none of them
    let none = cstr(b"plain/path");
    for &s in &high {
        assert_eq!(diff_extract(&none, s), 0);
    }
}

// E5
#[test]
fn e5_extract_trailing_separator() {
    for body in [&b"/"[..], b"a/", b"dir/", b"a/b/", b"//"] {
        let path = cstr(body);
        assert_eq!(
            diff_extract(&path, b'/'),
            body.len() as isize,
            "trailing separator yields an empty filename"
        );
    }
}

// E6 / E9 — null pointers. Differential, executed in subprocesses so the crash
// is contained; C and Rust must terminate identically.
#[test]
fn e6_null_path_documented_only() {
    let c = run_child("c", "null_path_extract", 0);
    let r = run_child("rust", "null_path_extract", 0);
    assert_termination_matches("extractFilename(NULL, '/')", &c, &r);
    // Both must die on a memory-access fault, not return a sentinel.
    assert_eq!(c.signal, Some(11), "expected SIGSEGV from C: {c:?}");

    let c = run_child("c", "null_path_create", 0);
    let r = run_child("rust", "null_path_create", 0);
    assert_termination_matches("FIO_createFilename_fromOutDir(NULL, \"o\", 0)", &c, &r);
    assert_eq!(c.signal, Some(11), "expected SIGSEGV from C: {c:?}");
}

// E9
#[test]
fn e9_null_outdir_documented_only() {
    let c = run_child("c", "null_outdir", 0);
    let r = run_child("rust", "null_outdir", 0);
    assert_termination_matches("FIO_createFilename_fromOutDir(\"p\", NULL, 0)", &c, &r);
    assert_eq!(c.signal, Some(11), "expected SIGSEGV from C: {c:?}");

    let c = run_child("c", "null_both", 0);
    let r = run_child("rust", "null_both", 0);
    assert_termination_matches("FIO_createFilename_fromOutDir(NULL, NULL, 0)", &c, &r);
    assert_eq!(c.signal, Some(11), "expected SIGSEGV from C: {c:?}");
}

// E7 — calloc failure => stderr message + exit(30).
#[test]
fn e7_calloc_failure_exit_30() {
    // usize::MAX/2 is unallocatable on every real system, so calloc returns NULL
    // without the request having wrapped.
    for suffix in [usize::MAX / 2, 1usize << 62, (usize::MAX / 2) - 12345] {
        let c = run_child("c", "alloc_fail", suffix);
        let r = run_child("rust", "alloc_fail", suffix);
        assert_termination_matches(&format!("alloc_fail suffixLen={suffix}"), &c, &r);
        assert_eq!(
            c.code,
            Some(30),
            "C must exit(30) on allocation failure, got {c:?}"
        );
        assert_eq!(
            r.code,
            Some(30),
            "Rust must exit(30) on allocation failure, got {r:?}"
        );
        let expected = b"zstd: FIO_createFilename_fromOutDir: Cannot allocate memory";
        assert!(
            contains(&c.stderr, expected),
            "C stderr missing expected message: {:?}",
            String::from_utf8_lossy(&c.stderr)
        );
        assert!(
            contains(&r.stderr, expected),
            "Rust stderr missing expected message: {:?}",
            String::from_utf8_lossy(&r.stderr)
        );
    }
}

// E11 — same sink, reached via a huge-but-non-wrapping suffixLen. Kept separate
// because ERRORS.md lists it as its own row.
#[test]
fn e11_huge_suffixlen_exit_30() {
    for suffix in [usize::MAX / 2 + 1, 1usize << 63, (1usize << 63) + 4096] {
        // These are still non-wrapping for our tiny path/outDir (total stays
        // below 2^64), so calloc simply fails.
        let c = run_child("c", "alloc_fail", suffix);
        let r = run_child("rust", "alloc_fail", suffix);
        assert_termination_matches(&format!("huge suffixLen={suffix}"), &c, &r);
        assert_eq!(c.code, Some(30), "C: {c:?}");
        assert_eq!(r.code, Some(30), "Rust: {r:?}");
    }
}

// E8 — empty outDirName: `outDirName[strlen(outDirName)-1]` reads the byte
// BEFORE the buffer. With a controlled sentinel we can pin down which branch the
// C takes and require Rust to take the same one.
#[test]
fn e8_empty_outdir_reads_byte_before_buffer() {
    let path = cstr(b"some/dir/file.txt");

    // sentinel == '/' -> the "already ends with separator" branch
    let buf = [b'/', 0u8];
    let out = unsafe { (buf.as_ptr() as *const c_char).add(1) };
    let got = diff_create_ptr(
        path.as_ptr() as *const c_char,
        out,
        0,
        "some/dir/file.txt",
        "<empty, preceded by '/'>",
    );
    assert_eq!(&got[..8], b"file.txt", "separator must NOT be inserted");
    assert_eq!(got.len(), 0 + 1 + 8 + 0 + 1);

    // sentinel != '/' -> the "insert separator" branch
    let buf = [b'Z', 0u8];
    let out = unsafe { (buf.as_ptr() as *const c_char).add(1) };
    let got = diff_create_ptr(
        path.as_ptr() as *const c_char,
        out,
        0,
        "some/dir/file.txt",
        "<empty, preceded by 'Z'>",
    );
    assert_eq!(&got[..9], b"/file.txt", "separator must be inserted");

    // exhaustive over the sentinel byte, for several suffix lengths
    let mut rng = Rng::new(0xE008);
    for s in 0u16..=255 {
        let buf = [s as u8, 0u8];
        let out = unsafe { (buf.as_ptr() as *const c_char).add(1) };
        for suffix in [0usize, 1, 7, rng.below(64)] {
            diff_create_ptr(
                path.as_ptr() as *const c_char,
                out,
                suffix,
                "some/dir/file.txt",
                &format!("<empty, preceded by 0x{s:02x}>"),
            );
        }
    }
}

// E10 — suffixLen large enough that the allocation-size arithmetic wraps.
// The wrap values are chosen so the subsequent memcpys stay INSIDE the (tiny)
// wrapped allocation, making the case observable without corrupting the heap.
#[test]
fn e10_suffixlen_overflow_wraps() {
    // (path, outDirName, suffixLen, wrapped alloc size, bytes actually written)
    let cases: &[(&[u8], &[u8], usize)] = &[
        // branch F2 (separator inserted): writes outDirLen + 1 + filenameLen
        (b"f", b"o", usize::MAX),               // alloc wraps to 3, writes 3
        (b"a/b/file", b"outdir", usize::MAX),   // alloc wraps to 11, writes 11
        (b"", b"o", usize::MAX),                // alloc wraps to 2, writes 2
        (b"x", b"abcdefghij", usize::MAX),      // alloc wraps to 12, writes 12
        // branch F1 (outDirName already ends in '/'): writes outDirLen + filenameLen
        (b"f", b"o/", usize::MAX),              // alloc wraps to 4, writes 3
        (b"f", b"o/", usize::MAX - 1),          // alloc wraps to 3, writes 3
        (b"a/b/file", b"outdir/", usize::MAX),  // alloc wraps to 12, writes 11
        (b"a/b/file", b"outdir/", usize::MAX - 1), // alloc wraps to 11, writes 11
        (b"", b"/", usize::MAX),                // alloc wraps to 2, writes 1
        (b"", b"/", usize::MAX - 1),            // alloc wraps to 1, writes 1
    ];
    for &(path, out, suffix) in cases {
        let pb = cstr(path);
        let ob = cstr(out);
        let filename_len = match path.iter().rposition(|&c| c == b'/') {
            Some(i) => path.len() - i - 1,
            None => path.len(),
        };
        let n = expected_alloc_len(out.len(), filename_len, suffix);
        assert!(
            n < 64,
            "test case must wrap to a small size; got {n} for suffixLen={suffix}"
        );
        let written = if out.last() == Some(&b'/') {
            out.len() + filename_len
        } else {
            out.len() + 1 + filename_len
        };
        assert!(
            written <= n,
            "test case would overflow the wrapped allocation ({written} > {n})"
        );
        // The real assertion: both libraries wrap identically and produce the
        // same bytes. In particular Rust must NOT panic on integer overflow.
        diff_create(&pb, &ob, suffix);
    }
}

// E12
#[test]
fn e12_empty_path_accepted() {
    let path = cstr(b"");
    for out in [&b"o"[..], b"o/", b"/", b"long/out/dir", b"long/out/dir/"] {
        for suffix in [0usize, 1, 9] {
            diff_create(&path, &cstr(out), suffix);
        }
    }
}

// E13
#[test]
fn e13_path_is_single_separator() {
    let path = cstr(b"/");
    for out in [&b"o"[..], b"o/", b"/", b"//", b"dir/sub", b"dir/sub/"] {
        for suffix in [0usize, 1, 5, 100] {
            diff_create(&path, &cstr(out), suffix);
        }
    }
}

// E14
#[test]
fn e14_outdir_is_single_separator() {
    let out = cstr(b"/");
    for path in [&b""[..], b"/", b"f", b"a/f", b"a/", b"//f"] {
        for suffix in [0usize, 1, 3, 64] {
            let got = diff_create(&cstr(path), &out, suffix);
            // allocation is 1 + 1 + filenameLen + suffix + 1
            let fl = match path.iter().rposition(|&c| c == b'/') {
                Some(i) => path.len() - i - 1,
                None => path.len(),
            };
            assert_eq!(got.len(), 1 + 1 + fl + suffix + 1);
        }
    }
}

// E15
#[test]
fn e15_outdir_non_ascii_last_byte() {
    let path = cstr(b"dir/file");
    for last in 0u16..=255 {
        if last == 0 {
            continue; // would terminate the string early
        }
        let out = cstr(&[0xC3, 0xA9, 0xFF, last as u8]);
        diff_create(&path, &out, 3);
    }
    // last byte '/' preceded by high-bit bytes
    diff_create(&path, &cstr(&[0xFF, 0xFE, b'/']), 3);
    diff_create(&path, &cstr(&[0x80, 0x7F, b'/']), 0);
}

// E16 — `char` parameter driven with out-of-range `int` values across FFI.
#[test]
fn e16_out_of_range_separator_int() {
    let mut body: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    body.extend_from_slice(b"/x/y");
    let path = cstr(&body);

    let mut cases: Vec<c_int> = vec![
        128, 129, 255, 256, 257, -129, -130, -255, -256, -257, 0x100, 0x1FF, 0x12F, 0xFFFF, -0xFFFF,
        c_int::MIN, c_int::MAX, c_int::MIN + 1, c_int::MAX - 1,
    ];
    for k in -40..=40i32 {
        cases.push(k * 256);
        cases.push(k * 256 + 0x2F); // == '/' mod 256
        cases.push(k * 256 + 0xFF);
        cases.push(k * 256 + 0x80);
    }
    for s in cases {
        diff_extract_int(&path, s);
    }
    // also on a path where nothing can match
    let none = cstr(b"abc");
    for s in [256, 512, -256, c_int::MIN, c_int::MAX, 0x2F + 256] {
        diff_extract_int(&none, s);
    }
}

// E17
#[test]
fn e17_zero_suffixlen() {
    let mut rng = Rng::new(0xE017);
    for _ in 0..2000 {
        let path = cstr(&rand_from_range(&mut rng, 0, 40, b"abc/def/ghi"));
        let out = cstr(&rand_from_range(&mut rng, 1, 40, b"xyz/uvw"));
        let got = diff_create(&path, &out, 0);
        assert_eq!(*got.last().unwrap(), 0, "must be NUL-terminated");
    }
}

// ===================== generic FFI boundary sweep ==========================

#[test]
fn boundary_suffixlen_sweep_small_and_powers_of_two() {
    let path = cstr(b"a/b/c/file.name");
    let out_f1 = cstr(b"out/dir/");
    let out_f2 = cstr(b"out/dir");
    for suffix in 0usize..=64 {
        diff_create(&path, &out_f1, suffix);
        diff_create(&path, &out_f2, suffix);
    }
    for k in 0..24u32 {
        let suffix = 1usize << k;
        diff_create(&path, &out_f1, suffix);
        diff_create(&path, &out_f2, suffix);
        diff_create(&path, &out_f1, suffix - 1);
        diff_create(&path, &out_f2, suffix + 1);
    }
}

#[test]
fn boundary_separator_exhaustive_char_range() {
    // every representable `char` value, incl. the signed extremes
    let mut body: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    body.extend((1u16..=255).map(|b| b as u8));
    let path = cstr(&body);
    for s in 0u16..=255 {
        diff_extract(&path, s as u8);
    }
    for s in [i8::MIN, -1, 0, 1, i8::MAX] {
        diff_extract_int(&path, s as c_int);
    }
}

// =================== subprocess plumbing / child helper ====================

#[derive(Debug)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Strip the libtest banner lines so only the library's own output is compared.
fn library_stderr(raw: &[u8]) -> Vec<u8> {
    raw.split(|&b| b == b'\n')
        .filter(|l| !l.starts_with(b"[harness]") && !l.starts_with(b"running "))
        .flat_map(|l| l.to_vec())
        .collect()
}

fn assert_termination_matches(what: &str, c: &Outcome, r: &Outcome) {
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "{what}: termination differs\n  C   = {c:?}\n  Rust= {r:?}"
    );
    assert_eq!(
        library_stderr(&c.stderr),
        library_stderr(&r.stderr),
        "{what}: stderr differs\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
}

fn run_child(which: &str, case: &str, suffix: usize) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "zz_child_helper",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DIFF_WHICH", which)
        .env("DIFF_CASE", case)
        .env("DIFF_SUFFIX", suffix.to_string())
        .output()
        .expect("spawn child");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Not a real test: when `DIFF_CASE` is set this process was spawned by
/// `run_child` and must perform exactly one (process-terminating) FFI call.
/// Without the env var it does nothing and passes.
#[test]
fn zz_child_helper() {
    let case = match std::env::var("DIFF_CASE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = std::env::var("DIFF_WHICH").unwrap();
    let suffix: usize = std::env::var("DIFF_SUFFIX").unwrap().parse().unwrap();
    let lib = open_one(&which);

    let path = cstr(b"file.txt");
    let out = cstr(b"outdir");
    let p = path.as_ptr() as *const c_char;
    let o = out.as_ptr() as *const c_char;

    let ret: *mut c_char = unsafe {
        match case.as_str() {
            "alloc_fail" => (lib.create_filename)(p, o, suffix),
            "null_path_extract" => (lib.extract_filename)(std::ptr::null(), b'/' as i8) as *mut c_char,
            "null_path_create" => (lib.create_filename)(std::ptr::null(), o, suffix),
            "null_outdir" => (lib.create_filename)(p, std::ptr::null(), suffix),
            "null_both" => (lib.create_filename)(std::ptr::null(), std::ptr::null(), suffix),
            other => panic!("unknown child case {other:?}"),
        }
    };
    // Reaching here means the library did NOT terminate; report distinctly so
    // the parent's differential comparison still sees a difference if only one
    // side survived.
    println!("UNEXPECTED_RETURN {ret:p}");
    std::process::exit(99);
}

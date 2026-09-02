//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary cases (null
//! pointers, zero/oversized lengths, one-past-range values, and the full
//! `char`-domain "out-of-range enum" analogue).
//!
//! Fatal paths (`exit(30)`, `SIGSEGV`) cannot be observed in-process, so they
//! are driven in child processes: this same test binary is re-executed with
//! `--exact <helper>` and `DIFFTEST_CHILD_IMPL=c|rust`, and the two
//! terminations are required to be identical.

mod harness;

use harness::*;
use std::ffi::c_char;

// ===========================================================================
// Row 1 — extractFilename: separator absent -> returns `path` itself
// ===========================================================================

#[test]
fn err_01_extract_separator_absent_returns_path() {
    let p = pair();
    let mut rng = Rng::new(0xA1);
    for i in 0..256 {
        let len = rng.range(0, 48);
        let path = cstr(&rand_bytes_without(&mut rng, len, b'/'));
        let base = path.as_ptr() as *const c_char;
        let c_ret = unsafe { (p.c.extract_filename)(base, b'/' as c_char) };
        let r_ret = unsafe { (p.rs.extract_filename)(base, b'/' as c_char) };
        assert_eq!(c_ret as usize, base as usize, "row1 iter{i}: C must return `path`");
        assert_eq!(r_ret as usize, base as usize, "row1 iter{i}: Rust must return `path`");
    }
    // Also the explicit empty-string case (zero length input).
    let empty = cstr(b"");
    diff_extract(&empty, b'/', "row1 empty");
}

// ===========================================================================
// Row 2 — extractFilename(NULL, sep) -> SIGSEGV in both
// ===========================================================================

#[test]
fn child_helper_extract_null_path() {
    let Some(imp) = child_impl() else { return };
    let r = unsafe { (imp.extract_filename)(std::ptr::null(), b'/' as c_char) };
    // Unreachable in practice; keep the call from being optimised away.
    std::process::exit(if r.is_null() { 11 } else { 12 });
}

#[test]
fn err_02_extract_null_path_segv() {
    diff_fatal("child_helper_extract_null_path", None, Some(11 /* SIGSEGV */));
}

// ===========================================================================
// Row 3 — extractFilename with separator == '\0'
// ===========================================================================

#[test]
fn err_03_extract_nul_separator() {
    let p = pair();
    let mut rng = Rng::new(0xA3);
    for i in 0..256 {
        let len = rng.range(0, 48);
        let path = cstr(&rand_bytes(&mut rng, len));
        let base = path.as_ptr() as *const c_char;
        let c_ret = unsafe { (p.c.extract_filename)(base, 0) };
        let r_ret = unsafe { (p.rs.extract_filename)(base, 0) };
        assert_eq!(c_ret as usize, r_ret as usize, "row3 iter{i}: pointer mismatch");
        // The C behaviour: strrchr matches the terminator -> path + len + 1.
        let expect = unsafe { base.add(len + 1) };
        assert_eq!(c_ret as usize, expect as usize, "row3 iter{i}: unexpected C result");
    }
}

// ===========================================================================
// Row 4 — calloc failure -> stderr message + exit(30)
// ===========================================================================

#[test]
fn child_helper_calloc_failure() {
    let Some(imp) = child_impl() else { return };
    let path = cstr(b"dir/file.txt");
    let out = cstr(b"outdir");
    // 1<<63 bytes cannot be allocated: calloc returns NULL -> exit(30).
    let r = unsafe {
        (imp.fio_create)(
            path.as_ptr() as *const c_char,
            out.as_ptr() as *const c_char,
            1usize << 63,
        )
    };
    // If we get here the allocation unexpectedly succeeded.
    std::process::exit(if r.is_null() { 41 } else { 42 });
}

#[test]
fn err_04_calloc_failure_exits_30() {
    diff_fatal("child_helper_calloc_failure", Some(30), None);
}

#[test]
fn err_04b_calloc_failure_stderr_message() {
    // The C code prints "zstd: FIO_createFilename_fromOutDir: <strerror(errno)>"
    // with no trailing newline. Compare the two implementations' stderr bytes.
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let run = |which: &str| {
        std::process::Command::new(&exe)
            .args([
                "--exact",
                "child_helper_calloc_failure",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_ENV, which)
            .env("RUST_BACKTRACE", "0")
            .output()
            .unwrap()
    };
    let c = run("c");
    let r = run("rust");
    assert_eq!(c.status.code(), Some(30));
    assert_eq!(r.status.code(), Some(30));
    assert_eq!(c.status.signal(), r.status.signal());
    let needle = b"zstd: FIO_createFilename_fromOutDir: ";
    let find = |h: &[u8]| h.windows(needle.len()).position(|w| w == needle);
    let ci = find(&c.stderr).expect("C stderr lacks the diagnostic prefix");
    let ri = find(&r.stderr).expect("Rust stderr lacks the diagnostic prefix");
    assert_eq!(
        &c.stderr[ci..],
        &r.stderr[ri..],
        "stderr diagnostic mismatch:\n C   = {:?}\n Rust= {:?}",
        String::from_utf8_lossy(&c.stderr[ci..]),
        String::from_utf8_lossy(&r.stderr[ri..])
    );
}

// ===========================================================================
// Row 5 — empty outDirName: outDirName[-1] out-of-bounds read
// ===========================================================================

#[test]
fn err_05_empty_outdir_oob_read_both_branches() {
    // Hand BOTH libraries the *same* pointer into a buffer whose preceding byte
    // we control, so the unchecked `outDirName[strlen(outDirName)-1]` read is
    // deterministic and both branches are reachable.
    let mut rng = Rng::new(0xA5);
    for (label, prev) in [("prev='/'", b'/'), ("prev='X'", b'X'), ("prev=0xFF", 0xFFu8)] {
        for i in 0..128 {
            // buf = [prev, 0, ...padding]; out_dir = &buf[1] == ""
            let mut buf = vec![prev, 0u8, 0, 0, 0, 0, 0, 0];
            let out_ptr = unsafe { buf.as_mut_ptr().add(1) } as *const c_char;

            let plen = rng.range(0, 24);
            let pb: Vec<u8> = (0..plen)
                .map(|_| if rng.below(4) == 0 { b'/' } else { rng.nonzero_byte() })
                .collect();
            let path = cstr(&pb);
            let sfx = rng.range(0, 16);

            let fstart =
                unsafe { (pair().c.extract_filename)(path.as_ptr() as *const c_char, b'/' as c_char) };
            // out_dir_len == 0
            let n = 0usize + 1 + unsafe { c_strlen(fstart) } + sfx + 1;
            diff_fio_ptr(
                path.as_ptr() as *const c_char,
                out_ptr,
                sfx,
                n,
                &format!("row5 {label} iter{i} sfx={sfx}"),
            );
        }
    }
}

// ===========================================================================
// Rows 6 & 7 — NULL path / NULL outDirName -> SIGSEGV in both
// ===========================================================================

#[test]
fn child_helper_fio_null_path() {
    let Some(imp) = child_impl() else { return };
    let out = cstr(b"outdir/");
    let r = unsafe { (imp.fio_create)(std::ptr::null(), out.as_ptr() as *const c_char, 0) };
    std::process::exit(if r.is_null() { 51 } else { 52 });
}

#[test]
fn err_06_fio_null_path_segv() {
    diff_fatal("child_helper_fio_null_path", None, Some(11 /* SIGSEGV */));
}

#[test]
fn child_helper_fio_null_outdir() {
    let Some(imp) = child_impl() else { return };
    let path = cstr(b"dir/file.txt");
    let r = unsafe { (imp.fio_create)(path.as_ptr() as *const c_char, std::ptr::null(), 0) };
    std::process::exit(if r.is_null() { 61 } else { 62 });
}

#[test]
fn err_07_fio_null_outdir_segv() {
    diff_fatal("child_helper_fio_null_outdir", None, Some(11 /* SIGSEGV */));
}

#[test]
fn child_helper_fio_both_null() {
    let Some(imp) = child_impl() else { return };
    let r = unsafe { (imp.fio_create)(std::ptr::null(), std::ptr::null(), 0) };
    std::process::exit(if r.is_null() { 71 } else { 72 });
}

#[test]
fn err_07b_fio_both_null_segv() {
    diff_fatal("child_helper_fio_both_null", None, Some(11 /* SIGSEGV */));
}

// ===========================================================================
// Row 8 — suffixLen == SIZE_MAX: the size expression wraps but still fits
// ===========================================================================

#[test]
fn err_08_suffixlen_size_max_wraps() {
    let mut rng = Rng::new(0xA8);
    for i in 0..128 {
        for ends in [true, false] {
            let olen = rng.range(1, 24);
            let mut o = rand_bytes_without(&mut rng, olen, b'/');
            let last = o.len() - 1;
            o[last] = if ends { b'/' } else { rng.nonzero_byte_except(b'/') };
            let out = cstr(&o);
            let plen = rng.range(0, 24);
            let pb: Vec<u8> = (0..plen)
                .map(|_| if rng.below(4) == 0 { b'/' } else { rng.nonzero_byte() })
                .collect();
            let path = cstr(&pb);

            // size = a + 1 + b + SIZE_MAX + 1  ==  a + b + 1  (mod 2^64)
            let fstart = unsafe {
                (pair().c.extract_filename)(path.as_ptr() as *const c_char, b'/' as c_char)
            };
            let a = unsafe { c_strlen(out.as_ptr() as *const c_char) };
            let b = unsafe { c_strlen(fstart) };
            let n = a + b + 1;
            // In the separator-inserting branch the memcpys write a + 1 + b ==
            // n bytes, i.e. the wrapped allocation is filled EXACTLY and no NUL
            // terminator is written. `strlen` would then run off the end of the
            // block into indeterminate heap bytes, so the byte-for-byte
            // comparison of the whole allocation is the meaningful assertion
            // here and the strlen cross-check is disabled.
            diff_fio_ptr_raw(
                path.as_ptr() as *const c_char,
                out.as_ptr() as *const c_char,
                usize::MAX,
                n,
                ends, // slash branch leaves the final byte zero => strlen is well defined
                &format!("row8 iter{i} ends={ends} a={a} b={b}"),
            );
        }
    }
}

// ===========================================================================
// Row 9 — suffixLen chosen so the size expression wraps to exactly 0
// ===========================================================================

#[test]
fn err_09_suffixlen_wraps_to_zero() {
    // outDirName = "a" (len 1), path = "b" (len 1) -> a + 1 + b + sfx + 1 == 0
    // requires sfx = SIZE_MAX - 3 + 1 - 1 ... solved directly below.
    // calloc(1, 0) returns a unique non-NULL block; the subsequent memcpys
    // write only a+1+b == 3 bytes, which stays inside glibc's minimum usable
    // chunk, so the comparison itself is safe.
    let out = cstr(b"a");
    let path = cstr(b"b");
    let a = 1usize; // strlen("a")
    let b = 1usize; // strlen("b")
    let sfx = 0usize.wrapping_sub(a + 1 + b + 1); // makes the total wrap to 0
    assert_eq!(a.wrapping_add(1).wrapping_add(b).wrapping_add(sfx).wrapping_add(1), 0);
    diff_fio_ptr_raw(
        path.as_ptr() as *const c_char,
        out.as_ptr() as *const c_char,
        sfx,
        a + 1 + b, // only the bytes both implementations actually write
        false,     // no NUL terminator was written: strlen would read garbage
        "row9 wrap-to-zero",
    );
}

// ===========================================================================
// Generic boundary: the full `char` domain for `separator`
// (this API's stand-in for an out-of-range enum value crossing the FFI edge)
// ===========================================================================

#[test]
fn err_10_extract_full_char_domain() {
    let paths: Vec<Vec<u8>> = vec![
        cstr(b""),
        cstr(b"a"),
        cstr(b"/"),
        cstr(b"//"),
        cstr(b"a/b"),
        cstr(b"/a/b/c"),
        cstr(b"a/b/"),
        cstr(&[0x80, 0x2F, 0xFF, 0x01, 0x7F]),
        cstr(&(1u8..=255).collect::<Vec<u8>>()),
    ];
    for (pi, path) in paths.iter().enumerate() {
        for sep in 0u16..=255 {
            diff_extract(path, sep as u8, &format!("row10 path{pi} sep={sep:#04x}"));
        }
    }
}

#[test]
fn err_11_extract_separator_int_widening() {
    // `char` is signed on x86-64 Linux, so 0x80..=0xFF arrive as negative ints
    // at the `strrchr` call. Values with no meaningful separator interpretation
    // must still behave identically. Every byte value is present in the path
    // exactly once, so a correct implementation returns a distinct offset for
    // every separator.
    let path = cstr(&(1u8..=255).collect::<Vec<u8>>());
    let p = pair();
    let base = path.as_ptr() as *const c_char;
    for sep in 1u16..=255 {
        let c_ret = unsafe { (p.c.extract_filename)(base, sep as u8 as c_char) };
        let r_ret = unsafe { (p.rs.extract_filename)(base, sep as u8 as c_char) };
        assert_eq!(c_ret as usize, r_ret as usize, "row11 sep={sep:#04x}");
        // Expected: index of `sep` is sep-1, so result is base + sep.
        assert_eq!(
            (c_ret as isize) - (base as isize),
            sep as isize,
            "row11 sep={sep:#04x}: unexpected C offset"
        );
    }
}

// ===========================================================================
// Generic boundary: zero and oversized lengths on FIO
// ===========================================================================

#[test]
fn err_12_fio_zero_and_boundary_lengths() {
    let cases: Vec<(&[u8], &[u8])> = vec![
        (b"", b"/"),
        (b"", b"o"),
        (b"/", b"/"),
        (b"/", b"o"),
        (b"//", b"//"),
        (b"a", b"/"),
        (b"a/", b"/"),
        (b"/a", b"o/"),
        (b"a/b/c", b"o"),
        (b"a/b/c/", b"o"),
    ];
    for (pi, (p, o)) in cases.iter().enumerate() {
        let path = cstr(p);
        let out = cstr(o);
        for sfx in [0usize, 1, 2, 7, 8, 63, 64, 65, 4095, 4096, 4097] {
            diff_fio(&path, &out, sfx, &format!("row12 case{pi} sfx={sfx}"));
        }
    }
}

#[test]
fn err_13_fio_oversized_suffixlen_boundaries() {
    // Values one step around allocation-relevant boundaries that still succeed.
    let path = cstr(b"dir/file");
    let out = cstr(b"out/");
    for sfx in [
        (1usize << 20) - 1,
        1usize << 20,
        (1usize << 20) + 1,
        (1usize << 24) - 1,
        1usize << 24,
        (1usize << 24) + 1,
    ] {
        diff_fio(&path, &out, sfx, &format!("row13 sfx={sfx}"));
    }
}

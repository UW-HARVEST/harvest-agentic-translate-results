//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Fatal outcomes (`exit(30)`, `SIGSEGV`) are
//! observed by running the call in the `child` helper executable against each
//! `.so` in turn and comparing exit status, stdout and stderr.

mod common;

use common::*;
use std::ffi::c_char;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Output};

const SEED: u64 = 0x5EED_1234_ABCD_0002;

// ---------------------------------------------------------------------------
// subprocess plumbing
// ---------------------------------------------------------------------------

fn run_child(args: &[&str]) -> Output {
    let bin = child_bin_path();
    let out = Command::new(&bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {bin:?}: {e}"));
    out
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Run the same child invocation against the C `.so` and the Rust `.so` and
/// assert exit status, stdout and stderr are identical.
fn diff_child(mode: &str, tail: &[String], ctx: &str) -> Output {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    let run = |so: &Path| {
        let mut args: Vec<String> = vec![mode.to_string(), so.display().to_string()];
        args.extend(tail.iter().cloned());
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run_child(&refs)
    };
    let c = run(&c_so);
    let r = run(&r_so);

    assert_eq!(
        (c.status.code(), c.status.signal()),
        (r.status.code(), r.status.signal()),
        "[{ctx}] exit status differs: C={:?}/{:?} Rust={:?}/{:?}\n C stderr: {}\n R stderr: {}",
        c.status.code(),
        c.status.signal(),
        r.status.code(),
        r.status.signal(),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.stdout, r.stdout,
        "[{ctx}] stdout differs:\n C: {}\n R: {}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{ctx}] stderr differs:\n C: {:?}\n R: {:?}",
        Escaped(&c.stderr),
        Escaped(&r.stderr)
    );
    c
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1
// ---------------------------------------------------------------------------

/// Row 1: `strrchr` returns NULL (separator absent) ⇒ `extractFilename` returns
/// `path` itself. Exercised with many shapes, including the empty path.
#[test]
fn err_01_extract_separator_absent_returns_path() {
    let mut rng = Rng::new(SEED ^ 1);

    // empty path, every non-NUL separator value
    let empty = [0u8];
    for sep in 1..=255u8 {
        let off = diff_extract_filename(&empty, sep, &format!("row1 empty sep=0x{sep:02x}"));
        assert_eq!(off, 0, "sentinel: returns `path`");
    }

    // non-empty paths that provably do not contain the separator
    for i in 0..400 {
        let len = rng.range(1, 64);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.ascii_byte()).collect();
        // pick a separator guaranteed absent
        let mut sep = rng.byte();
        while sep == 0 || buf.contains(&sep) {
            sep = sep.wrapping_add(1);
            if sep == 0 {
                sep = 1;
            }
            if !buf.contains(&sep) && sep != 0 {
                break;
            }
        }
        if buf.contains(&sep) || sep == 0 {
            continue;
        }
        buf.push(0);
        let off = diff_extract_filename(&buf, sep, &format!("row1 iter{i}"));
        assert_eq!(off, 0, "sentinel: returns `path`");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 + 13
// ---------------------------------------------------------------------------

/// Row 2: `calloc` failure ⇒ message on stderr + `exit(30)`.
#[test]
fn err_02_alloc_failure_exits_30() {
    let path = hex(b"some/dir/file.bin");
    let dir = hex(b"outdir");
    let suffix = (usize::MAX / 2).to_string();
    let out = diff_child("create", &[path, dir, suffix], "row2");

    assert_eq!(out.status.code(), Some(30), "C contract: exit(30)");
    let mut expect = b"zstd: FIO_createFilename_fromOutDir: ".to_vec();
    expect.extend(strerror_bytes(ENOMEM));
    assert_eq!(
        out.stderr,
        expect,
        "stderr text must match the C fprintf exactly (no newline)\n got: {:?}",
        Escaped(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "nothing is printed on the error path");
}

/// Row 13: several oversized `suffixLen` magnitudes, all of which must behave
/// identically in both libraries (either both allocate or both `exit(30)`).
#[test]
fn err_13_fio_oversized_suffixlen_exits_30() {
    let cases: [usize; 4] = [usize::MAX / 2, usize::MAX / 4, 1usize << 62, 1usize << 48];
    for (n, suffix) in cases.iter().enumerate() {
        for (path, dir) in [
            (&b"a/b/c.txt"[..], &b"out"[..]),
            (&b"file"[..], &b"out/"[..]),
            (&b""[..], &b"/"[..]),
        ] {
            let out = diff_child(
                "create",
                &[hex(path), hex(dir), suffix.to_string()],
                &format!("row13 case{n} suffix={suffix}"),
            );
            if out.status.code() == Some(30) {
                let mut expect = b"zstd: FIO_createFilename_fromOutDir: ".to_vec();
                expect.extend(strerror_bytes(ENOMEM));
                assert_eq!(out.stderr, expect, "row13 case{n}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 3-5 (separator boundary values)
// ---------------------------------------------------------------------------

/// Row 3: `separator == '\0'` is *found* by `strrchr` (the NUL terminator is
/// part of the searched string), so the result is `path + strlen(path) + 1`.
#[test]
fn err_03_extract_nul_separator() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..300 {
        let len = rng.below(64);
        let mut buf = rng.path_like(len, 25, false);
        let expect = buf.len() + 1;
        buf.push(0);
        let off = diff_extract_filename(&buf, 0, &format!("row3 iter{i}"));
        assert_eq!(off as usize, expect, "one past the terminator");
    }
}

/// Row 4: empty path with `separator == '\0'`.
#[test]
fn err_04_extract_empty_path_nul_separator() {
    let buf = [0u8];
    let off = diff_extract_filename(&buf, 0, "row4");
    assert_eq!(off, 1);
}

/// Row 5: every one of the 256 `char` values a caller can pass across the FFI
/// boundary (the analogue of an out-of-range enum value), against paths that do
/// and do not contain them.
#[test]
fn err_05_extract_all_256_separator_values() {
    let mut rng = Rng::new(SEED ^ 5);
    for sep in 0..=255u8 {
        // (a) separator present at a known position
        for i in 0..4 {
            let len = rng.range(1, 32);
            let mut buf: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
            if sep != 0 {
                let pos = rng.below(len);
                buf[pos] = sep;
            }
            buf.push(0);
            diff_extract_filename(&buf, sep, &format!("row5a sep=0x{sep:02x} iter{i}"));
        }
        // (b) alphabet made only of high-bit bytes, to catch sign-extension bugs
        for i in 0..4 {
            let len = rng.range(1, 32);
            let mut buf: Vec<u8> = (0..len).map(|_| rng.plain_byte() | 0x80).collect();
            buf.push(0);
            diff_extract_filename(&buf, sep, &format!("row5b sep=0x{sep:02x} iter{i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 6-8 (NULL pointers)
// ---------------------------------------------------------------------------

/// Row 6: `extractFilename(NULL, sep)` — both libraries must die the same way.
#[test]
fn err_06_extract_null_path_segv() {
    for sep in ["47", "0", "255"] {
        let out = diff_child("extract_null", &[sep.to_string()], &format!("row6 sep={sep}"));
        assert_eq!(
            out.status.signal(),
            Some(11),
            "row6 sep={sep}: expected SIGSEGV, got {:?}/{:?}",
            out.status.code(),
            out.status.signal()
        );
    }
}

/// Row 7: `FIO_createFilename_fromOutDir(NULL, dir, n)`.
#[test]
fn err_07_fio_null_path_segv() {
    for dir in [&b"out"[..], &b"out/"[..], &b""[..]] {
        let out = diff_child(
            "null_path",
            &[hex(dir), "0".to_string()],
            &format!("row7 dir={:?}", Escaped(dir)),
        );
        assert_eq!(out.status.signal(), Some(11), "row7: expected SIGSEGV");
    }
}

/// Row 8: `FIO_createFilename_fromOutDir(path, NULL, n)`.
#[test]
fn err_08_fio_null_outdir_segv() {
    for path in [&b"a/b/c"[..], &b"file"[..], &b""[..]] {
        let out = diff_child(
            "null_dir",
            &[hex(path), "0".to_string()],
            &format!("row8 path={:?}", Escaped(path)),
        );
        assert_eq!(out.status.signal(), Some(11), "row8: expected SIGSEGV");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 (empty outDirName ⇒ out-of-bounds read at outDirName[-1])
// ---------------------------------------------------------------------------

#[test]
fn err_09_fio_empty_outdir_oob_read() {
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..200 {
        // the byte preceding the empty string decides the branch
        let prev = if i % 2 == 0 { b'/' } else { rng.plain_byte() };
        let dir_buf: Vec<u8> = vec![prev, 0];
        let mut file = rng.path_b(24, 20, false);
        file.push(0);
        let suffix = rng.below(8);
        let flen = filename_component_len(&file[..file.len() - 1]);
        let size = expected_alloc_size(0, flen, suffix);
        let out = diff_create_filename_ptrs(
            file.as_ptr() as *const c_char,
            unsafe { (dir_buf.as_ptr() as *const c_char).add(1) },
            suffix,
            size,
            &format!("row9 iter{i} prev=0x{prev:02x}"),
        );
        if prev == b'/' {
            // trailing-separator branch: filename copied straight to offset 0
            assert_eq!(&out[..flen], &file[file.len() - 1 - flen..file.len() - 1]);
        } else {
            assert_eq!(out[0], b'/');
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 10-12 (suffixLen boundaries)
// ---------------------------------------------------------------------------

/// Row 10: `suffixLen == 0` (minimum).
#[test]
fn err_10_fio_zero_suffixlen() {
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..300 {
        let mut dir = rng.path_r(1, 32, 10, false);
        if i % 3 == 0 {
            dir.push(b'/');
        }
        let file = rng.path_b(32, 20, false);
        let out = diff_create_filename(&file, &dir, 0, &format!("row10 iter{i}"));
        assert_eq!(*out.last().unwrap(), 0, "result is NUL terminated");
    }
}

/// Row 11: `suffixLen == SIZE_MAX` — the `size_t` sum wraps, `calloc` succeeds
/// with a buffer that the payload fills exactly (so the result is NOT
/// NUL-terminated). Both branches of line 45 are covered; both are provably
/// within the (wrapped) allocation, so this is safe to run in-process.
#[test]
fn err_11_fio_suffixlen_size_max_wraps() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..200 {
        let trailing = i % 2 == 0;
        let mut dir = rng.path_r(1, 24, 10, false);
        if trailing {
            dir.push(b'/');
        } else if dir.last() == Some(&b'/') {
            *dir.last_mut().unwrap() = b'k';
        }
        let file = rng.path_b(24, 20, false);
        let flen = filename_component_len(&file);
        let size = expected_alloc_size(dir.len(), flen, usize::MAX);
        assert_eq!(size, dir.len() + flen + 1);
        let out = diff_create_filename_raw(
            &file,
            &dir,
            usize::MAX,
            size,
            &format!("row11 iter{i} trailing={trailing}"),
        );
        if trailing {
            assert_eq!(*out.last().unwrap(), 0, "1 spare byte in this branch");
        } else {
            // payload fills the buffer exactly: last byte is the last byte of
            // the filename (or the inserted separator when the name is empty)
            let expect_last = if flen == 0 {
                b'/'
            } else {
                file[file.len() - 1]
            };
            assert_eq!(*out.last().unwrap(), expect_last, "no room for the NUL");
        }
    }
}

/// Row 12: `suffixLen == SIZE_MAX - 1` and `SIZE_MAX - 2`. These wrap to sizes
/// that are 1 / 2 bytes *smaller* than the payload, so the C overflows the heap
/// block by a byte or two. Run in the isolated child process, comparing the
/// bytes both libraries produce (and the exit status) rather than risking the
/// test harness's heap.
#[test]
fn err_12_fio_suffixlen_near_size_max_wraps() {
    for suffix in [usize::MAX - 1, usize::MAX - 2] {
        for (path, dir) in [
            (&b"aaaa/bbbb"[..], &b"outdir"[..]),
            (&b"aaaa/bbbb"[..], &b"outdir/"[..]),
            (&b"x"[..], &b"d"[..]),
        ] {
            diff_child(
                "create",
                &[hex(path), hex(dir), suffix.to_string()],
                &format!("row12 suffix=MAX-{} dir={:?}", usize::MAX - suffix, Escaped(dir)),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 14 (empty filename component)
// ---------------------------------------------------------------------------

#[test]
fn err_14_fio_empty_filename_component() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..200 {
        let mut dir = rng.path_r(1, 24, 10, false);
        if i % 2 == 0 {
            dir.push(b'/');
        } else if dir.last() == Some(&b'/') {
            *dir.last_mut().unwrap() = b'm';
        }
        // (a) path == ""
        diff_create_filename(b"", &dir, rng.below(4), &format!("row14a iter{i}"));
        // (b) path ends with the separator ⇒ empty component
        let mut file = rng.path_b(24, 20, false);
        file.push(b'/');
        diff_create_filename(&file, &dir, rng.below(4), &format!("row14b iter{i}"));
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundaries beyond the table
// ---------------------------------------------------------------------------

/// Interior and one-past-the-end pointers fed back into `extractFilename`
/// (exactly what the Windows branch of the C does with the result of the first
/// call, and what a real caller does when chaining).
#[test]
fn err_15_extract_chained_interior_pointers() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..300 {
        let mut buf = rng.path_r(1, 48, 25, false);
        buf.push(0);
        let base = buf.as_ptr() as *const c_char;
        let off = diff_extract_filename(&buf, b'/', &format!("row15 first iter{i}"));
        // feed the returned (interior) pointer back in, with several separators
        for sep in [b'/', b'.', 0u8] {
            diff_extract_filename_at(
                base,
                unsafe { base.offset(off) },
                sep,
                &format!("row15 chain iter{i} sep=0x{sep:02x}"),
            );
        }
    }
}

/// `suffixLen` exactly at the point where the wrapped size is still large
/// enough (`SIZE_MAX` minus the payload) is covered by row 11; here we sweep the
/// small-but-nonzero end plus a few powers of two that are still allocatable.
#[test]
fn err_16_fio_suffixlen_sweep() {
    let mut rng = Rng::new(SEED ^ 16);
    let mut suffixes: Vec<usize> = (0..40).collect();
    suffixes.extend([1usize << 10, 1 << 12, 1 << 16, 1 << 20]);
    for (n, s) in suffixes.iter().enumerate() {
        let mut dir = rng.path_r(1, 16, 10, false);
        if n % 2 == 0 {
            dir.push(b'/');
        }
        let file = rng.path_b(16, 20, false);
        diff_create_filename(&file, &dir, *s, &format!("row16 suffix={s}"));
    }
}

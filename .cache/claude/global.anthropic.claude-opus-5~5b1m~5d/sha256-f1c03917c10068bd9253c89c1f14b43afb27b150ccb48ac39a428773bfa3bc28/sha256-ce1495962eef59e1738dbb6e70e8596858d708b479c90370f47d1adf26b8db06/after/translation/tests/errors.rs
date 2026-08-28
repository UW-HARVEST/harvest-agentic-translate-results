// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md. Each test constructs the exact invalid input /
// condition, calls BOTH `.so`s, and asserts they produce the SAME rejection:
// the same sentinel (`-1`, `-2`, `NULL`) AND the same diagnostic bytes on
// stderr — not merely "both failed somehow".
//
// Test names are prefixed `err_NN_` matching the ERRORS.md row numbers.

mod common;

use common::*;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

// ---------------------------------------------------------------------------
// Helpers that additionally pin down the ABSOLUTE expected C behaviour, so a
// test cannot pass by both sides being wrong in the same new way.
// ---------------------------------------------------------------------------

const NEG_MSG: &[u8] = b"Error: negative input\n";

fn open_fail_msg(name: &str) -> Vec<u8> {
    format!("Error: opening or processing file {name}\n").into_bytes()
}

/// Call the C symbol only, to record the ground-truth observation.
fn c_obs_forward(x: i32) -> (i32, Vec<u8>, Vec<u8>) {
    let f = c_forward();
    let (r, o, e) = capture(|| unsafe { f(x) });
    (r, o, e)
}

fn c_obs_open(name: Option<&[u8]>) -> (bool, Vec<u8>, Vec<u8>) {
    let f = c_open();
    let cs = name.map(|n| CString::new(n).unwrap());
    let p = cs.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    let (nullp, o, e) = capture(|| unsafe { f(p).is_null() });
    (nullp, o, e)
}

fn c_obs_driver(num: i32, name: Option<&[u8]>) -> (i32, Vec<u8>, Vec<u8>) {
    let f = c_driver();
    let cs = name.map(|n| CString::new(n).unwrap());
    let p = cs.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    let (r, o, e) = capture(|| unsafe { f(num, p) });
    (r, o, e)
}

// ---------------------------------------------------------------------------
// Rows 1-3 — forward_goto_example, the `goto error` branch
// ---------------------------------------------------------------------------

/// Row 1: `x < 0` → stderr "Error: negative input\n", nothing on stdout, -1.
#[test]
fn err_01_fge_negative() {
    let mut rng = Rng::new(101);
    for i in 0..300 {
        let x = rng.range_i64(i32::MIN as i64, -1) as i32;
        diff_forward(x, &format!("err1/rand#{i}"));
    }
    // Pin the absolute C contract.
    let (ret, out, err) = c_obs_forward(-42);
    assert_eq!(ret, -1, "C forward_goto_example(-42) must return -1");
    assert!(out.is_empty(), "C must print nothing to stdout: {}", esc(&out));
    assert_eq!(err, NEG_MSG, "C stderr text changed: {}", esc(&err));
}

/// Row 2: `x == INT_MIN` — the most-negative boundary value.
#[test]
fn err_02_fge_int_min() {
    diff_forward(i32::MIN, "err2/INT_MIN");
    let (ret, _, err) = c_obs_forward(i32::MIN);
    assert_eq!(ret, -1);
    assert_eq!(err, NEG_MSG);
}

/// Row 3: `x == -1` — exactly one step past the valid `x >= 0` range.
#[test]
fn err_03_fge_minus_one() {
    diff_forward(-1, "err3/minus-one");
    let (ret, _, err) = c_obs_forward(-1);
    assert_eq!(ret, -1);
    assert_eq!(err, NEG_MSG);
}

// ---------------------------------------------------------------------------
// Rows 4-10 — open_with_cleanup, the `!fp` branch (fopen failed)
// ---------------------------------------------------------------------------

/// Row 4: ENOENT — the path simply does not exist.
#[test]
fn err_04_owc_enoent() {
    let mut rng = Rng::new(104);
    let p = missing_path("err04");
    diff_open_path(&p, "err4/missing");

    let (nullp, out, err) = c_obs_open(Some(p.as_os_str().as_bytes()));
    assert!(nullp, "C must return NULL when fopen fails");
    assert!(out.is_empty(), "no stdout expected: {}", esc(&out));
    assert_eq!(err, open_fail_msg(p.to_str().unwrap()));

    // Randomized nonexistent names, including odd bytes in the path.
    for i in 0..150 {
        let n = rng.range(1, 30);
        let mut name: Vec<u8> = b"/nonexistent-dir-".to_vec();
        name.extend(rng.text(n).into_iter().map(|c| {
            // keep it a single path component and NUL/'/'-free
            if c == b'/' {
                b'_'
            } else {
                c
            }
        }));
        diff_open(Some(&name), &format!("err4/rand#{i}"));
    }
}

/// Row 5: ENOENT via a zero-length filename.
#[test]
fn err_05_owc_empty_name() {
    diff_open(Some(b""), "err5/empty-name");
    let (nullp, _, err) = c_obs_open(Some(b""));
    assert!(nullp);
    assert_eq!(err, open_fail_msg(""));
}

/// Row 6: `filename == NULL` — glibc's fopen rejects it (EFAULT) and the
/// cleanup `fprintf` formats the null `%s` as "(null)".
#[test]
fn err_06_owc_null_ptr() {
    diff_open(None, "err6/NULL");
    let (nullp, out, err) = c_obs_open(None);
    assert!(nullp, "C must return NULL for a NULL filename");
    assert!(out.is_empty());
    assert_eq!(
        err,
        open_fail_msg("(null)"),
        "C stderr for NULL filename: {}",
        esc(&err)
    );
}

/// Row 7: EACCES — an existing file the process may not read.
#[test]
fn err_07_owc_eacces() {
    if is_root() {
        eprintln!("skipping err_07: running as root bypasses the permission check");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let p = put_file("err07-noperm", b"secret\n");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();

    diff_open_path(&p, "err7/eacces");
    let (nullp, _, err) = c_obs_open(Some(p.as_os_str().as_bytes()));
    assert!(nullp, "C must return NULL on EACCES");
    assert_eq!(err, open_fail_msg(p.to_str().unwrap()));

    // driver() must map it to -2.
    diff_driver_path(5, &p, "err7/driver-eacces");

    // Write-only (0o200): unreadable, so mode "r" fails here too. Contrast with
    // read-only 0o444, which MUST succeed (see cfg_25_permission_modes) — the
    // pair of rows is what pins the fopen mode string to exactly "r".
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o200)).unwrap();
    diff_open_path(&p, "err7/write-only");
    let (nullp, _, err) = c_obs_open(Some(p.as_os_str().as_bytes()));
    assert!(nullp, "C must return NULL for a write-only file");
    assert_eq!(err, open_fail_msg(p.to_str().unwrap()));
    diff_driver_path(5, &p, "err7/driver-write-only");

    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
}

/// Row 8: ENAMETOOLONG — an oversized (5000-byte) path component.
#[test]
fn err_08_owc_enametoolong() {
    for len in [255usize, 256, 1000, 5000] {
        let mut name = scratch_dir().as_os_str().as_bytes().to_vec();
        name.push(b'/');
        name.extend(std::iter::repeat(b'a').take(len));
        diff_open(Some(&name), &format!("err8/len={len}"));
    }
    let mut name = b"/".to_vec();
    name.extend(std::iter::repeat(b'z').take(5000));
    diff_open(Some(&name), "err8/root-5000");
    let (nullp, _, _) = c_obs_open(Some(&name));
    assert!(nullp, "C must return NULL for an over-long path");
}

/// Row 9: ENOTDIR — a regular file used as an intermediate path component.
#[test]
fn err_09_owc_enotdir() {
    let f = put_file("err09-file", b"data\n");
    let mut name = f.as_os_str().as_bytes().to_vec();
    name.extend_from_slice(b"/child");
    diff_open(Some(&name), "err9/enotdir");
    let (nullp, _, err) = c_obs_open(Some(&name));
    assert!(nullp, "C must return NULL on ENOTDIR");
    assert_eq!(err, open_fail_msg(std::str::from_utf8(&name).unwrap()));
}

/// Row 10: ELOOP — a self-referential symlink.
#[test]
fn err_10_owc_eloop() {
    let link = scratch_dir().join("err10-loop");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink("err10-loop", &link).expect("create self-symlink");
    diff_open_path(&link, "err10/eloop");
    let (nullp, _, err) = c_obs_open(Some(link.as_os_str().as_bytes()));
    assert!(nullp, "C must return NULL on ELOOP");
    assert_eq!(err, open_fail_msg(link.to_str().unwrap()));
}

// ---------------------------------------------------------------------------
// Row 11 — open_with_cleanup, the `ferror(fp)` branch
// ---------------------------------------------------------------------------

/// Row 11: `fopen` SUCCEEDS but reading fails. On Linux `fopen("<dir>", "r")`
/// returns a valid stream and `fgets` then fails with EISDIR, so `ferror(fp)`
/// is non-zero and the second `goto cleanup` is taken — this is the only path
/// where `fclose(fp)` actually runs inside the cleanup block.
#[test]
fn err_11_owc_ferror_directory() {
    let d = make_dir("err11");
    diff_open_path(&d, "err11/directory");

    let (nullp, out, err) = c_obs_open(Some(d.as_os_str().as_bytes()));
    assert!(nullp, "C must return NULL when ferror() is set");
    assert!(out.is_empty(), "no stdout expected: {}", esc(&out));
    assert_eq!(err, open_fail_msg(d.to_str().unwrap()));

    // Also the scratch root and a few well-known directories.
    for dir in [scratch_dir(), std::path::PathBuf::from("/"), std::env::temp_dir()] {
        diff_open_path(&dir, "err11/other-dir");
    }
    // Repeat: the fclose() inside cleanup must not leak fds differently.
    for i in 0..100 {
        diff_open_path(&d, &format!("err11/repeat#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 12-17 — driver's two sentinel returns
// ---------------------------------------------------------------------------

/// Row 12: `res == -1` → `driver` returns -1 and never calls fopen.
#[test]
fn err_12_driver_negative_num() {
    let good = put_file("err12-good", b"content\n");
    let mut rng = Rng::new(112);
    for i in 0..150 {
        let num = rng.range_i64(i32::MIN as i64, -1) as i32;
        diff_driver_path(num, &good, &format!("err12/rand#{i}"));
    }
    let (ret, out, err) = c_obs_driver(-3, Some(good.as_os_str().as_bytes()));
    assert_eq!(ret, -1, "C driver must return -1 for negative num");
    assert!(out.is_empty(), "no stdout expected: {}", esc(&out));
    assert_eq!(err, NEG_MSG, "only the negative-input line: {}", esc(&err));
}

/// Row 13: negative `num` AND an invalid filename — proves the file is never
/// touched (no open-failure message appears).
#[test]
fn err_13_driver_negative_num_bad_file() {
    let gone = missing_path("err13");
    let d = make_dir("err13");
    for name in [
        Some(gone.as_os_str().as_bytes()),
        Some(d.as_os_str().as_bytes()),
        Some(&b""[..]),
        None,
    ] {
        diff_driver(-1, name, "err13/bad-file");
        diff_driver(i32::MIN, name, "err13/bad-file-min");
    }
    let (ret, _, err) = c_obs_driver(-1, None);
    assert_eq!(ret, -1);
    assert_eq!(
        err, NEG_MSG,
        "driver must not attempt to open the file: {}",
        esc(&err)
    );
}

/// Row 14: `out == NULL` through the `!fp` branch → `driver` returns -2, after
/// having printed both stdout lines.
#[test]
fn err_14_driver_open_fail() {
    let gone = missing_path("err14");
    let mut rng = Rng::new(114);
    for i in 0..150 {
        let num = rng.range_i64(0, (1i64 << 30) - 1) as i32;
        diff_driver_path(num, &gone, &format!("err14/rand#{i}"));
    }
    let (ret, out, err) = c_obs_driver(21, Some(gone.as_os_str().as_bytes()));
    assert_eq!(ret, -2, "C driver must return -2 when the file cannot be used");
    assert_eq!(
        out,
        b"Processing: 21\nGoto output: 42\n".to_vec(),
        "stdout: {}",
        esc(&out)
    );
    assert_eq!(err, open_fail_msg(gone.to_str().unwrap()));
}

/// Row 15: `out == NULL` through the `ferror` branch → -2.
#[test]
fn err_15_driver_ferror_fail() {
    let d = make_dir("err15");
    for num in [0i32, 1, 7, 1 << 30, i32::MAX] {
        diff_driver_path(num, &d, "err15/dir");
    }
    let (ret, out, err) = c_obs_driver(7, Some(d.as_os_str().as_bytes()));
    assert_eq!(ret, -2, "C driver must return -2 on the ferror path");
    assert_eq!(out, b"Processing: 7\nGoto output: 14\n".to_vec());
    assert_eq!(err, open_fail_msg(d.to_str().unwrap()));
}

/// Row 16: `filename == NULL` with a non-negative `num` → -2, "(null)".
#[test]
fn err_16_driver_null_filename() {
    for num in [0i32, 1, 123, 1 << 30, i32::MAX] {
        diff_driver(num, None, "err16/null-name");
    }
    let (ret, out, err) = c_obs_driver(0, None);
    assert_eq!(ret, -2);
    assert_eq!(out, b"Processing: 0\nGoto output: 0\n".to_vec());
    assert_eq!(err, open_fail_msg("(null)"));
}

/// Row 17: `num == INT_MIN` through `driver`.
#[test]
fn err_17_driver_int_min() {
    let good = put_file("err17-good", b"x\n");
    diff_driver_path(i32::MIN, &good, "err17/int-min");
    diff_driver(i32::MIN, None, "err17/int-min-null");
    let (ret, _, err) = c_obs_driver(i32::MIN, Some(good.as_os_str().as_bytes()));
    assert_eq!(ret, -1);
    assert_eq!(err, NEG_MSG);
}

// ---------------------------------------------------------------------------
// Rows 18-24 — generic FFI-boundary cases
// ---------------------------------------------------------------------------

/// Rows 18/19/20: null pointers, zero lengths and oversized lengths, gathered
/// so the boundary matrix is visible in one place.
#[test]
fn err_20_boundary_matrix() {
    // NULL pointer into both pointer-taking entry points.
    diff_open(None, "err18/open-null");
    diff_driver(0, None, "err18/driver-null");
    diff_driver(-1, None, "err18/driver-null-neg");

    // Zero length: empty filename, and a zero-byte input file.
    diff_open(Some(b""), "err19/empty-name");
    diff_driver(0, Some(b""), "err19/driver-empty-name");
    let zero = put_file("err19-zero", b"");
    diff_open_path(&zero, "err19/zero-byte-file");
    diff_driver_path(0, &zero, "err19/driver-zero-byte-file");

    // Oversized: a 5000-byte path and a 256 KiB single-line file.
    let mut long = b"/".to_vec();
    long.extend(std::iter::repeat(b'q').take(5000));
    diff_open(Some(&long), "err20/long-path");
    diff_driver(1, Some(&long), "err20/driver-long-path");

    let mut rng = Rng::new(120);
    let big = put_file("err20-big", &rng.text(256 * 1024));
    diff_open_path(&big, "err20/big-file");
    diff_driver_path(1, &big, "err20/driver-big-file");
}

/// Rows 21/22/23/24: the full `int` domain.
///
/// `goto.h` declares no `enum` and no `bool`, so there is no "invalid variant"
/// to smuggle across the FFI boundary — the only scalar parameter is a plain
/// `int` whose entire `INT_MIN..=INT_MAX` domain is accepted. The equivalent
/// coverage is therefore a randomized sweep of that whole domain, plus the
/// specific values where the C behaves surprisingly:
///
///  * `x >= 2^30` makes `x * 2` wrap negative (gcc -O0 emits `add %eax,%eax`),
///    yet `driver` keeps going because it only tests `res == -1`;
///  * no `x >= 0` can produce `x * 2 == -1`, so the `-1` sentinel is never
///    aliased by a success value — asserted explicitly below.
#[test]
fn err_23_int_full_range_sweep() {
    let good = put_file("err23-good", b"line\n");
    let gone = missing_path("err23");
    let mut rng = Rng::new(123);

    // Explicit boundaries.
    for x in [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        (1 << 30) - 1,
        1 << 30,
        (1 << 30) + 1,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_forward(x, &format!("err23/boundary x={x}"));
        diff_driver_path(x, &good, &format!("err23/driver-good x={x}"));
        diff_driver_path(x, &gone, &format!("err23/driver-gone x={x}"));
    }

    // Randomized over the ENTIRE int domain.
    for i in 0..400 {
        let x = rng.i32_any();
        diff_forward(x, &format!("err23/rand#{i} x={x}"));
    }

    // Sentinel-aliasing property, taken from the C itself.
    for x in [0i32, 1, 2, 3, 1 << 29, (1 << 30) - 1, 1 << 30, i32::MAX] {
        let (ret, _, err) = c_obs_forward(x);
        assert_ne!(
            ret, -1,
            "C forward_goto_example({x}) must not alias the -1 sentinel"
        );
        assert_eq!(ret, x.wrapping_mul(2), "C computes a wrapping x*2 for {x}");
        assert!(err.is_empty(), "no stderr on the success path for {x}");
    }
    // ... and INT_MAX really does wrap to -2 while driver still succeeds.
    let (ret, _, _) = c_obs_forward(i32::MAX);
    assert_eq!(ret, -2, "C: INT_MAX * 2 wraps to -2");
    let (dret, dout, _) = c_obs_driver(i32::MAX, Some(good.as_os_str().as_bytes()));
    assert_eq!(dret, 0, "driver still succeeds when x*2 wraps negative");
    assert_eq!(dout, b"Processing: 2147483647\nGoto output: -2\nline\n".to_vec());
}

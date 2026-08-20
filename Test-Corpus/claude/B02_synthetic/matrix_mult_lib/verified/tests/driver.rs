//! `driver()` differential tests — `CONFIGS.md` rows C35–C39 (valid paths) and
//! `ERRORS.md` rows E26–E30 (error paths).
//!
//! `driver()` hardcodes `OUT_FILE "matrix.txt"`, i.e. it writes relative to the
//! process' working directory, so these tests live in their own test binary and
//! serialise on a CWD mutex.

mod common;

use common::*;
use std::ffi::c_int;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct DriverRun {
    rc: c_int,
    stderr: Vec<u8>,
    file: Option<Vec<u8>>,
}

/// Runs `driver()` on both libraries inside `dir` (which must be the CWD) and
/// asserts the return code, the `stderr` bytes and the produced `matrix.txt`
/// are identical.
fn run_pair_in(
    dir: &Path,
    width_a: c_int,
    height_a: c_int,
    matrix_a: &CBuf,
    width_b: c_int,
    height_b: c_int,
    matrix_b: &CBuf,
    ctx: &str,
) -> DriverRun {
    let (c, r) = both();
    let out = dir.join("matrix.txt");

    let (crc, c_err) = capture_stderr(|| unsafe {
        (c.driver)(
            width_a,
            height_a,
            matrix_a.as_ptr(),
            width_b,
            height_b,
            matrix_b.as_ptr(),
        )
    });
    let c_file = fs::read(&out).ok();
    let _ = fs::remove_file(&out);

    let (rrc, r_err) = capture_stderr(|| unsafe {
        (r.driver)(
            width_a,
            height_a,
            matrix_a.as_ptr(),
            width_b,
            height_b,
            matrix_b.as_ptr(),
        )
    });
    let r_file = fs::read(&out).ok();
    let _ = fs::remove_file(&out);

    assert_eq!(crc, rrc, "driver return mismatch [{ctx}]");
    assert_bytes_eq(&c_err, &r_err, &format!("driver stderr mismatch [{ctx}]"));
    assert_opt_bytes_eq(
        &c_file,
        &r_file,
        &format!("driver matrix.txt mismatch [{ctx}]"),
    );

    DriverRun {
        rc: crc,
        stderr: c_err,
        file: c_file,
    }
}

fn diff_driver(
    width_a: c_int,
    height_a: c_int,
    matrix_a: &str,
    width_b: c_int,
    height_b: c_int,
    matrix_b: &str,
    ctx: &str,
) -> DriverRun {
    let a = CBuf::new(matrix_a);
    let b = CBuf::new(matrix_b);
    in_temp_cwd(|dir| {
        run_pair_in(dir, width_a, height_a, &a, width_b, height_b, &b, ctx)
    })
}

/// Independent reference implementation of the whole pipeline.
fn reference(av: &[Vec<i32>], bv: &[Vec<i32>], m: usize, k: usize, n: usize) -> String {
    let mut s = String::new();
    for i in 0..m {
        let mut cols = Vec::new();
        for j in 0..n {
            let mut acc: i32 = 0;
            for kk in 0..k {
                acc = acc.wrapping_add(av[i][kk].wrapping_mul(bv[kk][j]));
            }
            cols.push(acc.to_string());
        }
        s.push_str(&cols.join(" "));
        s.push('\n');
    }
    s
}

// ---------------------------------------------------------------------------
// C35 / C36 — happy paths
// ---------------------------------------------------------------------------

#[test]
fn c35_driver_1x1() {
    let run = diff_driver(1, 1, "6\n", 1, 1, "7\n", "C35");
    assert_eq!(run.rc, EXIT_SUCCESS);
    assert_eq!(run.file.as_deref(), Some(&b"42\n"[..]));
    assert!(run.stderr.is_empty());
}

#[test]
fn c36_driver_random_shapes() {
    let mut rng = Rng::new(0x3636);
    for iter in 0..80 {
        let m = rng.i32_in(1, 8);
        let k = rng.i32_in(1, 8);
        let n = rng.i32_in(1, 8);
        let av = random_values(&mut rng, k, m, -5000, 5000);
        let bv = random_values(&mut rng, n, k, -5000, 5000);
        let ctx = format!("C36 #{iter} {m}x{k}*{k}x{n}");
        let run = diff_driver(
            k,
            m,
            &matrix_text(&av),
            n,
            k,
            &matrix_text(&bv),
            &ctx,
        );
        assert_eq!(
            run.rc,
            EXIT_SUCCESS,
            "{ctx}: stderr={:?} a={:?} b={:?}",
            String::from_utf8_lossy(&run.stderr),
            matrix_text(&av),
            matrix_text(&bv)
        );
        let expect = reference(&av, &bv, m as usize, k as usize, n as usize);
        assert_bytes_eq(
            run.file.as_deref().unwrap_or_default(),
            expect.as_bytes(),
            &format!("{ctx}: wrong contents"),
        );
    }
}

// ---------------------------------------------------------------------------
// C37 — degenerate but valid shapes
// ---------------------------------------------------------------------------

#[test]
fn c37_driver_degenerate_shapes() {
    let mut rng = Rng::new(0x3737);
    for _ in 0..15 {
        let m = rng.i32_in(1, 5);
        let n = rng.i32_in(1, 5);
        let k = rng.i32_in(1, 5);

        // k == 0: A is m x 0 (m row tokens needed), B is 0 x n.
        let ctx = format!("C37 k=0 {m}x0*0x{n}");
        let run = diff_driver(0, m, &"row\n".repeat(m as usize), n, 0, "", &ctx);
        assert_eq!(run.rc, EXIT_SUCCESS, "{ctx}");
        let expect: String = (0..m)
            .map(|_| {
                let z: Vec<&str> = (0..n).map(|_| "0").collect();
                format!("{}\n", z.join(" "))
            })
            .collect();
        assert_bytes_eq(
            run.file.as_deref().unwrap_or_default(),
            expect.as_bytes(),
            &ctx,
        );

        // m == 0: A has no rows ⇒ empty output file.
        let bv = random_values(&mut rng, n, k, -100, 100);
        let ctx = format!("C37 m=0 0x{k}*{k}x{n}");
        let run = diff_driver(k, 0, "", n, k, &matrix_text(&bv), &ctx);
        assert_eq!(run.rc, EXIT_SUCCESS, "{ctx}");
        assert_bytes_eq(run.file.as_deref().unwrap_or_default(), b"", &ctx);

        // n == 0: B has no columns ⇒ m bare newlines.
        let av = random_values(&mut rng, k, m, -100, 100);
        let ctx = format!("C37 n=0 {m}x{k}*{k}x0");
        let run = diff_driver(
            k,
            m,
            &matrix_text(&av),
            0,
            k,
            &"row\n".repeat(k as usize),
            &ctx,
        );
        assert_eq!(run.rc, EXIT_SUCCESS, "{ctx}");
        assert_bytes_eq(
            run.file.as_deref().unwrap_or_default(),
            "\n".repeat(m as usize).as_bytes(),
            &ctx,
        );

        // both zero
        let ctx = "C37 0x0*0x0".to_string();
        let run = diff_driver(0, 0, "", 0, 0, "", &ctx);
        assert_eq!(run.rc, EXIT_SUCCESS, "{ctx}");
        assert_bytes_eq(run.file.as_deref().unwrap_or_default(), b"", &ctx);
    }
}

// ---------------------------------------------------------------------------
// C38 — quirky-but-valid input strings
// ---------------------------------------------------------------------------

#[test]
fn c38_driver_quirky_inputs() {
    // (width_a, height_a, matrix_a, width_b, height_b, matrix_b)
    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        // extra rows and columns
        (2, 2, "1 2 99\n3 4 99\n9 9 9\n", 2, 2, "5 6 7\n7 8 9\n1 1 1\n"),
        // runs of spaces, leading/trailing whitespace
        (2, 2, "  1   2  \n  3   4  \n", 2, 2, "5  6\n7  8\n"),
        // blank lines everywhere
        (2, 2, "\n\n1 2\n\n3 4\n\n", 2, 2, "\n1 0\n\n0 1\n"),
        // CRLF line endings
        (2, 2, "1 2\r\n3 4\r\n", 2, 2, "1 0\r\n0 1\r\n"),
        // no trailing newline at all
        (2, 2, "1 2\n3 4", 2, 2, "1 0\n0 1"),
        // tabs inside tokens (one space-delimited token each)
        (1, 2, "\t5\n\t6\n", 2, 1, "3 4\n"),
        // atoi quirks: non-numeric ⇒ 0, partially numeric ⇒ prefix
        (3, 1, "12abc x9 -7z\n", 1, 3, "2\n3\n4\n"),
        // atoi range clamping
        (1, 1, "99999999999999999999\n", 1, 1, "1\n"),
        (1, 1, "-2147483648\n", 1, 1, "1\n"),
        (1, 1, "2147483647\n", 1, 1, "1\n"),
        (1, 1, "  +42\n", 1, 1, "-1\n"),
        (1, 1, "000123\n", 1, 1, "0\n"),
        // 1xN * Nx1 and Nx1 * 1xN
        (4, 1, "1 2 3 4\n", 1, 4, "5\n6\n7\n8\n"),
        (1, 4, "1\n2\n3\n4\n", 4, 1, "5 6 7 8\n"),
    ];
    for (i, (wa, ha, a, wb, hb, b)) in cases.iter().enumerate() {
        let ctx = format!("C38 #{i}");
        let run = diff_driver(*wa, *ha, a, *wb, *hb, b, &ctx);
        assert_eq!(run.rc, EXIT_SUCCESS, "{ctx}: {run:?}");
        assert!(run.file.is_some(), "{ctx}");
    }
}

// ---------------------------------------------------------------------------
// C39 — bigger shapes and wrap-around products
// ---------------------------------------------------------------------------

#[test]
fn c39_driver_large_and_wrapping() {
    let mut rng = Rng::new(0x3939);

    // 8x8 * 8x8 with moderate values
    for _ in 0..10 {
        let av = random_values(&mut rng, 8, 8, -20_000, 20_000);
        let bv = random_values(&mut rng, 8, 8, -20_000, 20_000);
        let run = diff_driver(8, 8, &matrix_text(&av), 8, 8, &matrix_text(&bv), "C39 8x8");
        assert_eq!(run.rc, EXIT_SUCCESS);
        let expect = reference(&av, &bv, 8, 8, 8);
        assert_bytes_eq(
            run.file.as_deref().unwrap_or_default(),
            expect.as_bytes(),
            "C39 8x8 contents",
        );
    }

    // Wrap-around accumulation. The result is kept one column wide so that the
    // 11-character renderings of wrapped values still fit into the C library's
    // (tight) output buffer.
    let big = [
        i32::MAX,
        i32::MIN,
        1_000_000_000,
        -1_000_000_000,
        123_456_789,
        -987_654_321,
        65_536,
        -65_537,
    ];
    for iter in 0..40 {
        let m = rng.i32_in(1, 8);
        let k = rng.i32_in(1, 8);
        let av: Vec<Vec<i32>> = (0..m)
            .map(|_| (0..k).map(|_| *rng.pick(&big)).collect())
            .collect();
        let bv: Vec<Vec<i32>> = (0..k).map(|_| vec![*rng.pick(&big)]).collect();
        let ctx = format!("C39 wrap #{iter} {m}x{k}*{k}x1");
        let run = diff_driver(k, m, &matrix_text(&av), 1, k, &matrix_text(&bv), &ctx);
        assert_eq!(run.rc, EXIT_SUCCESS, "{ctx}");
        let expect = reference(&av, &bv, m as usize, k as usize, 1);
        assert_bytes_eq(
            run.file.as_deref().unwrap_or_default(),
            expect.as_bytes(),
            &ctx,
        );
    }
}

// ---------------------------------------------------------------------------
// E26 — matrix_a cannot be parsed
// ---------------------------------------------------------------------------

#[test]
fn err_e26_driver_bad_a() {
    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        // insufficient rows in A
        (2, 2, "1 2\n", 2, 2, "1 0\n0 1\n"),
        // insufficient columns in A
        (2, 2, "1\n2\n", 2, 2, "1 0\n0 1\n"),
        // empty A
        (1, 1, "", 1, 1, "1\n"),
        // negative width in A
        (-1, 2, "1 2\n3 4\n", 2, 2, "1 0\n0 1\n"),
        // negative height in A
        (2, -2, "1 2\n3 4\n", 2, 2, "1 0\n0 1\n"),
    ];
    for (i, (wa, ha, a, wb, hb, b)) in cases.iter().enumerate() {
        let ctx = format!("E26 #{i}");
        let run = diff_driver(*wa, *ha, a, *wb, *hb, b, &ctx);
        assert_eq!(run.rc, EXIT_FAILURE, "{ctx}: expected EXIT_FAILURE");
        assert!(run.file.is_none(), "{ctx}: no file must be written");
    }
}

// ---------------------------------------------------------------------------
// E27 — matrix_b cannot be parsed
// ---------------------------------------------------------------------------

#[test]
fn err_e27_driver_bad_b() {
    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        (2, 2, "1 2\n3 4\n", 2, 2, "1 0\n"),
        (2, 2, "1 2\n3 4\n", 2, 2, "1\n0\n"),
        (2, 2, "1 2\n3 4\n", 2, 2, ""),
        (2, 2, "1 2\n3 4\n", -1, 2, "1 0\n0 1\n"),
        (2, 2, "1 2\n3 4\n", 2, -2, "1 0\n0 1\n"),
    ];
    for (i, (wa, ha, a, wb, hb, b)) in cases.iter().enumerate() {
        let ctx = format!("E27 #{i}");
        let run = diff_driver(*wa, *ha, a, *wb, *hb, b, &ctx);
        assert_eq!(run.rc, EXIT_FAILURE, "{ctx}: expected EXIT_FAILURE");
        assert!(run.file.is_none(), "{ctx}: no file must be written");
    }
}

// ---------------------------------------------------------------------------
// E28 — dimension mismatch between A and B
// ---------------------------------------------------------------------------

#[test]
fn err_e28_driver_dim_mismatch() {
    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        // width_a (2) != height_b (3)
        (2, 2, "1 2\n3 4\n", 2, 3, "1 0\n0 1\n1 1\n"),
        // width_a (3) != height_b (1)
        (3, 1, "1 2 3\n", 1, 1, "5\n"),
        // width_a (1) != height_b (2)
        (1, 1, "1\n", 1, 2, "5\n6\n"),
        // width_a (0) != height_b (1)
        (0, 1, "row\n", 1, 1, "5\n"),
        // width_a (2) != height_b (0)
        (2, 1, "1 2\n", 1, 0, ""),
    ];
    for (i, (wa, ha, a, wb, hb, b)) in cases.iter().enumerate() {
        let ctx = format!("E28 #{i}");
        let run = diff_driver(*wa, *ha, a, *wb, *hb, b, &ctx);
        assert_eq!(run.rc, EXIT_FAILURE, "{ctx}: expected EXIT_FAILURE");
        assert!(run.file.is_none(), "{ctx}: no file must be written");
        assert!(
            String::from_utf8_lossy(&run.stderr).contains("do not allow multiplication"),
            "{ctx}: unexpected stderr {:?}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
}

// ---------------------------------------------------------------------------
// E30 — write_to_file fails inside driver()
// ---------------------------------------------------------------------------

#[test]
fn err_e30_driver_write_fails() {
    let a = CBuf::new("1 2\n3 4\n");
    let b = CBuf::new("5 6\n7 8\n");

    // (a) "matrix.txt" already exists as a DIRECTORY ⇒ fopen fails (EISDIR).
    let run = in_temp_cwd(|dir| {
        fs::create_dir(dir.join("matrix.txt")).unwrap();
        run_pair_in(dir, 2, 2, &a, 2, 2, &b, "E30 dir")
    });
    assert_eq!(run.rc, EXIT_FAILURE, "E30 dir: {run:?}");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("Error opening file 'matrix.txt'"),
        "E30 dir: unexpected stderr {:?}",
        String::from_utf8_lossy(&run.stderr)
    );

    // (b) the working directory is read-only ⇒ fopen fails (EACCES).
    let run = in_temp_cwd(|dir| {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o500)).unwrap();
        let out = run_pair_in(dir, 2, 2, &a, 2, 2, &b, "E30 ro");
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        out
    });
    assert_eq!(run.rc, EXIT_FAILURE, "E30 ro: {run:?}");
    assert!(run.file.is_none(), "E30 ro");
}

// ---------------------------------------------------------------------------
// Extra driver coverage: repeated invocations (the `"w"` mode must truncate the
// previously written matrix.txt) and a Rust-first ordering.
// ---------------------------------------------------------------------------

#[test]
fn c35b_driver_repeated_calls_truncate() {
    let (c, r) = both();
    let big_a = CBuf::new(matrix_text(&vec![vec![1234567; 6]; 6]));
    let big_b = CBuf::new(matrix_text(&vec![vec![7; 6]; 6]));
    let small_a = CBuf::new("2\n");
    let small_b = CBuf::new("3\n");

    let (c_bytes, r_bytes) = in_temp_cwd(|dir| {
        let out = dir.join("matrix.txt");
        // C: big result, then a small one into the same file
        assert_eq!(
            unsafe { (c.driver)(6, 6, big_a.as_ptr(), 6, 6, big_b.as_ptr()) },
            EXIT_SUCCESS
        );
        let big_len = fs::read(&out).unwrap().len();
        assert_eq!(
            unsafe { (c.driver)(1, 1, small_a.as_ptr(), 1, 1, small_b.as_ptr()) },
            EXIT_SUCCESS
        );
        let c_bytes = fs::read(&out).unwrap();
        assert!(c_bytes.len() < big_len, "file was not truncated");
        fs::remove_file(&out).unwrap();

        // Rust: same sequence
        assert_eq!(
            unsafe { (r.driver)(6, 6, big_a.as_ptr(), 6, 6, big_b.as_ptr()) },
            EXIT_SUCCESS
        );
        assert_eq!(
            unsafe { (r.driver)(1, 1, small_a.as_ptr(), 1, 1, small_b.as_ptr()) },
            EXIT_SUCCESS
        );
        let r_bytes = fs::read(&out).unwrap();
        (c_bytes, r_bytes)
    });
    assert_bytes_eq(&c_bytes, &r_bytes, "repeated driver() calls diverge");
    assert_bytes_eq(&c_bytes, b"6\n", "wrong final contents");
}

#[test]
fn c36b_driver_rust_first() {
    let (c, r) = both();
    let mut rng = Rng::new(0x36B);
    for iter in 0..40 {
        let m = rng.i32_in(1, 6);
        let k = rng.i32_in(1, 6);
        let n = rng.i32_in(1, 6);
        let av = random_values(&mut rng, k, m, -5000, 5000);
        let bv = random_values(&mut rng, n, k, -5000, 5000);
        let a = CBuf::new(matrix_text(&av));
        let b = CBuf::new(matrix_text(&bv));
        let ctx = format!("C36b #{iter} {m}x{k}*{k}x{n}");
        let (rrc, r_file, crc, c_file) = in_temp_cwd(|dir| {
            let out = dir.join("matrix.txt");
            let rrc = unsafe { (r.driver)(k, m, a.as_ptr(), n, k, b.as_ptr()) };
            let r_file = fs::read(&out).ok();
            let _ = fs::remove_file(&out);
            let crc = unsafe { (c.driver)(k, m, a.as_ptr(), n, k, b.as_ptr()) };
            let c_file = fs::read(&out).ok();
            let _ = fs::remove_file(&out);
            (rrc, r_file, crc, c_file)
        });
        assert_eq!(crc, rrc, "{ctx}");
        assert_opt_bytes_eq(&c_file, &r_file, &ctx);
        let expect = reference(&av, &bv, m as usize, k as usize, n as usize);
        assert_bytes_eq(c_file.as_deref().unwrap_or_default(), expect.as_bytes(), &ctx);
    }
}

#[test]
fn err_e26b_driver_errors_rust_first() {
    let (c, r) = both();
    let cases: &[(c_int, c_int, &str, c_int, c_int, &str)] = &[
        (2, 2, "1 2\n", 2, 2, "1 0\n0 1\n"),
        (2, 2, "1 2\n3 4\n", 2, 2, "1\n0\n"),
        (2, 2, "1 2\n3 4\n", 2, 3, "1 0\n0 1\n1 1\n"),
        (-1, 2, "1 2\n3 4\n", 2, 2, "1 0\n0 1\n"),
        (2, -2, "1 2\n3 4\n", 2, 2, "1 0\n0 1\n"),
    ];
    for (i, (wa, ha, at, wb, hb, bt)) in cases.iter().enumerate() {
        let a = CBuf::new(*at);
        let b = CBuf::new(*bt);
        let ctx = format!("E26b #{i}");
        let (rrc, r_err, crc, c_err) = in_temp_cwd(|_dir| {
            let (rrc, r_err) = capture_stderr(|| unsafe {
                (r.driver)(*wa, *ha, a.as_ptr(), *wb, *hb, b.as_ptr())
            });
            let (crc, c_err) = capture_stderr(|| unsafe {
                (c.driver)(*wa, *ha, a.as_ptr(), *wb, *hb, b.as_ptr())
            });
            (rrc, r_err, crc, c_err)
        });
        assert_eq!(crc, rrc, "{ctx}");
        assert_eq!(crc, EXIT_FAILURE, "{ctx}");
        assert_bytes_eq(&c_err, &r_err, &format!("{ctx} stderr"));
    }
}

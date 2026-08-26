//! Phase C — error-path differential tests, one test per `ERRORS.md` row
//! (the `driver` rows E26–E30 live in `tests/driver.rs`, which owns the CWD).
//!
//! Every test builds the exact invalid input, calls BOTH libraries through
//! their exported symbols and asserts the SAME sentinel / errno value **and**
//! the same bytes on `stderr`.

mod common;

use common::*;
use std::ffi::c_int;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::ptr;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `allocate_matrix` on both; asserts identical NULL-ness and stderr.
/// Returns `true` when both returned NULL.
fn diff_allocate(width: c_int, height: c_int, ctx: &str) -> bool {
    let (c, r) = both();
    let (cp, c_err) = capture_stderr(|| unsafe { (c.allocate_matrix)(width, height) });
    let c_null = cp.is_null();
    let c_snap = unsafe { snap_matrix(cp, false) };
    unsafe { (c.free_matrix)(cp) };
    let (rp, r_err) = capture_stderr(|| unsafe { (r.allocate_matrix)(width, height) });
    let r_null = rp.is_null();
    let r_snap = unsafe { snap_matrix(rp, false) };
    unsafe { (r.free_matrix)(rp) };
    assert_eq!(
        c_null, r_null,
        "allocate_matrix({width}, {height}): NULL-ness mismatch [{ctx}]"
    );
    assert_eq!(
        c_snap, r_snap,
        "allocate_matrix({width}, {height}): struct mismatch [{ctx}]"
    );
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("allocate_matrix({width}, {height}): stderr mismatch [{ctx}]"),
    );
    c_null
}

/// `initialize_matrix_from_string` on both; asserts identical result + stderr.
/// Returns `true` when both returned NULL.
fn diff_init(text: &CBuf, width: c_int, height: c_int, ctx: &str) -> bool {
    let (c, r) = both();
    let (cp, rp) = unsafe { init_both(text, width, height, ctx) };
    unsafe {
        (c.free_matrix)(cp);
        (r.free_matrix)(rp);
    }
    cp.is_null()
}

/// `multiply_matrices` on both with the *same* (caller-owned) operands.
fn diff_multiply(
    a: *mut MatrixT,
    b: *mut MatrixT,
    ctx: &str,
) -> bool {
    let (c, r) = both();
    let (cp, c_err) = capture_stderr(|| unsafe { (c.multiply_matrices)(a, b) });
    let (rp, r_err) = capture_stderr(|| unsafe { (r.multiply_matrices)(a, b) });
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "multiply_matrices: NULL-ness mismatch [{ctx}]"
    );
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("multiply_matrices: stderr mismatch [{ctx}]"),
    );
    unsafe {
        (c.free_matrix)(cp);
        (r.free_matrix)(rp);
    }
    cp.is_null()
}

/// `matrix_to_string` on both with the *same* (caller-owned) matrix.
fn diff_to_string(m: *mut MatrixT, ctx: &str) -> Option<Vec<u8>> {
    let (c, r) = both();
    let (cs, c_err) = capture_stderr(|| unsafe { (c.matrix_to_string)(m) });
    let (rs, r_err) = capture_stderr(|| unsafe { (r.matrix_to_string)(m) });
    let cb = unsafe { take_c_string(cs) };
    let rb = unsafe { take_c_string(rs) };
    assert_opt_bytes_eq(&cb, &rb, &format!("matrix_to_string mismatch [{ctx}]"));
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("matrix_to_string stderr mismatch [{ctx}]"),
    );
    cb
}

/// `write_to_file` on both with the *same* file name (error paths never create
/// the file, so the `stderr` messages must be byte-identical too).
fn diff_write(filename: &CBuf, content: &CBuf, ctx: &str) -> c_int {
    let (c, r) = both();
    let (crc, c_err) =
        capture_stderr(|| unsafe { (c.write_to_file)(filename.as_ptr(), content.as_ptr()) });
    let (rrc, r_err) =
        capture_stderr(|| unsafe { (r.write_to_file)(filename.as_ptr(), content.as_ptr()) });
    assert_eq!(crc, rrc, "write_to_file return mismatch [{ctx}]");
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("write_to_file stderr mismatch [{ctx}]"),
    );
    crc
}

/// A `matrix_t` crafted by the test (never allocated by either library) — used
/// to reach the branches that inspect only `width`/`height`.
fn fake_matrix(width: c_int, height: c_int) -> MatrixT {
    MatrixT {
        matrix: ptr::null_mut(),
        width,
        height,
    }
}

// ---------------------------------------------------------------------------
// E2 / E3 / E4 — allocate_matrix failure paths
// ---------------------------------------------------------------------------

#[test]
fn err_e2_allocate_negative_height() {
    for h in [-1, -2, -7, -1000, i32::MIN, i32::MIN + 1, -1_073_741_824] {
        for w in [0, 1, 5] {
            assert!(
                diff_allocate(w, h, "E2"),
                "allocate_matrix({w}, {h}) should fail"
            );
        }
    }
}

#[test]
fn err_e3_allocate_negative_width() {
    for w in [-1, -2, -9, -1000, i32::MIN, i32::MIN + 1] {
        for h in [1, 2, 5] {
            assert!(
                diff_allocate(w, h, "E3"),
                "allocate_matrix({w}, {h}) should fail"
            );
        }
    }
    // width < 0 with height == 0 does NOT fail (the row loop never runs).
    for w in [-1, -1000, i32::MIN] {
        assert!(
            !diff_allocate(w, 0, "E3 h=0"),
            "allocate_matrix({w}, 0) should succeed"
        );
    }
}

#[test]
fn err_e4_allocate_huge_width() {
    // ~8 GiB single row request: whatever malloc() decides, both libraries must
    // decide identically (asserted inside diff_allocate, twice per shape so that
    // an allocator state change cannot hide a divergence).
    let shapes = [(2_000_000_000, 1), (i32::MAX, 1), (1_073_741_824, 2)];
    let mut probes = 0;
    for (w, h) in shapes {
        let first = diff_allocate(w, h, "E4");
        let second = diff_allocate(w, h, "E4 repeat");
        assert_eq!(first, second, "allocate_matrix({w}, {h}) is not deterministic");
        probes += 1;
    }
    assert_eq!(probes, shapes.len(), "not every huge shape was probed");
}

// ---------------------------------------------------------------------------
// E5 — free_matrix(NULL)
// ---------------------------------------------------------------------------

#[test]
fn err_e5_free_null() {
    let (c, r) = both();
    let ((), c_err) = capture_stderr(|| unsafe { (c.free_matrix)(ptr::null_mut()) });
    let ((), r_err) = capture_stderr(|| unsafe { (r.free_matrix)(ptr::null_mut()) });
    assert_bytes_eq(&c_err, &r_err, "free_matrix(NULL) stderr mismatch");
    assert!(c_err.is_empty(), "free_matrix(NULL) must be silent");
    // ... and repeated calls stay harmless.
    for _ in 0..10 {
        unsafe {
            (c.free_matrix)(ptr::null_mut());
            (r.free_matrix)(ptr::null_mut());
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — "Insufficient rows in input string."
// ---------------------------------------------------------------------------

#[test]
fn err_e7_insufficient_rows() {
    let cases: &[(&str, c_int, c_int)] = &[
        ("", 1, 1),
        ("\n", 1, 1),
        ("\n\n\n", 2, 2),
        ("1 2\n", 2, 2),
        ("1 2\n3 4\n", 2, 3),
        ("1\n2\n", 1, 5),
        ("   \n   \n", 1, 1), // rows exist but contain no columns → see E8
        ("", 0, 1),
        ("", 5, 3),
        ("\n\n", 0, 1),
    ];
    for (text, w, h) in cases {
        let ctx = format!("E7 {text:?} {w}x{h}");
        assert!(
            diff_init(&CBuf::new(*text), *w, *h, &ctx),
            "{ctx}: expected NULL"
        );
    }
    // randomised: always at least one row short
    let mut rng = Rng::new(0xE7);
    for _ in 0..100 {
        let w = rng.i32_in(1, 5);
        let rows = rng.i32_in(0, 4);
        let h = rows + rng.i32_in(1, 3);
        let values = random_values(&mut rng, w, rows, -100, 100);
        let text = matrix_text(&values);
        let ctx = format!("E7 rnd {w}x{h} rows={rows}");
        assert!(diff_init(&CBuf::new(text), w, h, &ctx), "{ctx}");
    }
}

// ---------------------------------------------------------------------------
// E8 — "Insufficient columns in row %d."
// ---------------------------------------------------------------------------

#[test]
fn err_e8_insufficient_columns() {
    let cases: &[(&str, c_int, c_int)] = &[
        ("1\n", 2, 1),
        ("1 2\n3\n", 2, 2),   // fails on row 2
        ("1\n2 3\n", 2, 2),   // fails on row 1
        ("1 2 3\n4 5 6\n7\n", 3, 3), // fails on row 3
        ("   \n", 1, 1),      // whitespace-only row has no tokens
        ("1 2\n \n", 1, 2),
    ];
    for (text, w, h) in cases {
        let ctx = format!("E8 {text:?} {w}x{h}");
        assert!(
            diff_init(&CBuf::new(*text), *w, *h, &ctx),
            "{ctx}: expected NULL"
        );
    }
    // randomised: a random row is short, so the 1-based row number in the
    // message differs from case to case.
    let mut rng = Rng::new(0xE8);
    for _ in 0..150 {
        let w = rng.i32_in(2, 6);
        let h = rng.i32_in(1, 6);
        let bad_row = rng.i32_in(0, h - 1);
        let mut text = String::new();
        for i in 0..h {
            let cols = if i == bad_row { rng.i32_in(0, w - 1) } else { w };
            let vals: Vec<String> = (0..cols).map(|_| rng.i32_in(-99, 99).to_string()).collect();
            text.push_str(&vals.join(" "));
            text.push('\n');
        }
        let ctx = format!("E8 rnd {w}x{h} bad_row={bad_row} {text:?}");
        assert!(diff_init(&CBuf::new(text), w, h, &ctx), "{ctx}");
    }
}

// ---------------------------------------------------------------------------
// E9 / E10 — initialize_matrix_from_string with negative dimensions
// ---------------------------------------------------------------------------

#[test]
fn err_e9_init_negative_width() {
    for w in [-1, -3, -1000, i32::MIN] {
        for h in [1, 2, 4] {
            // enough rows: allocate_matrix fails, no column is ever parsed
            let text = "1 2 3\n".repeat(h as usize);
            let ctx = format!("E9 w={w} h={h}");
            assert!(diff_init(&CBuf::new(text), w, h, &ctx), "{ctx}");
            // too few rows: the "Insufficient rows" message comes first
            let ctx = format!("E9 short w={w} h={h}");
            assert!(diff_init(&CBuf::new(""), w, h, &ctx), "{ctx}");
        }
    }
}

#[test]
fn err_e10_init_negative_height() {
    for h in [-1, -5, -1000, i32::MIN] {
        for w in [0, 1, 3, -1] {
            let ctx = format!("E10 w={w} h={h}");
            assert!(diff_init(&CBuf::new("1 2 3\n"), w, h, &ctx), "{ctx}");
        }
    }
}

// ---------------------------------------------------------------------------
// E12 / E13 / E14 — multiply_matrices failure paths
// ---------------------------------------------------------------------------

#[test]
fn err_e12_dim_mismatch() {
    let (c, r) = both();
    // Real matrices from the C library (both libs only read them).
    let cases: &[((c_int, c_int), (c_int, c_int))] = &[
        ((2, 2), (2, 3)), // a.width 2 != b.height 3
        ((1, 1), (1, 2)),
        ((3, 1), (1, 1)),
        ((1, 3), (3, 3)), // a.width 1 != b.height 3
        ((0, 2), (2, 1)), // a.width 0 != b.height 1
        ((2, 1), (1, 0)), // a.width 2 != b.height 0
        ((5, 5), (5, 4)),
    ];
    /// Text that lets `initialize_matrix_from_string(w, h)` succeed.
    fn text_for(w: c_int, h: c_int, v: i32) -> String {
        if h == 0 {
            String::new()
        } else if w == 0 {
            "x\n".repeat(h as usize)
        } else {
            matrix_text(&vec![vec![v; w as usize]; h as usize])
        }
    }

    for ((wa, ha), (wb, hb)) in cases {
        let a_text = text_for(*wa, *ha, 1);
        let b_text = text_for(*wb, *hb, 2);
        let a = unsafe {
            (c.initialize_matrix_from_string)(CBuf::new(a_text).as_ptr(), *wa, *ha)
        };
        let b = unsafe {
            (c.initialize_matrix_from_string)(CBuf::new(b_text).as_ptr(), *wb, *hb)
        };
        assert!(!a.is_null() && !b.is_null(), "setup failed");
        let ctx = format!("E12 {ha}x{wa} * {hb}x{wb}");
        assert!(diff_multiply(a, b, &ctx), "{ctx}: expected NULL");
        unsafe {
            (c.free_matrix)(a);
            (c.free_matrix)(b);
        }
    }
    // ... and with hand-crafted dimensions, including negative ones.
    let mut rng = Rng::new(0xE12);
    for _ in 0..100 {
        let mut a = fake_matrix(rng.i32_in(-4, 4), rng.i32_in(0, 4));
        let mut b = fake_matrix(rng.i32_in(-4, 4), rng.i32_in(-4, 4));
        if a.width == b.height {
            b.height = a.width + 1; // guarantee the mismatch branch
        }
        let ctx = format!("E12 fake a.w={} b.h={}", a.width, b.height);
        assert!(
            diff_multiply(&mut a, &mut b, &ctx),
            "{ctx}: expected NULL"
        );
        let _ = r; // both libraries were exercised inside diff_multiply
    }
}

#[test]
fn err_e13_mul_negative_result_width() {
    // dims agree, but the result width is negative ⇒ inner allocate fails.
    for wb in [-1, -3, i32::MIN] {
        for k in [0, 1, 3] {
            let mut a = fake_matrix(k, 2);
            let mut b = fake_matrix(wb, k);
            let ctx = format!("E13 k={k} b.width={wb}");
            assert!(diff_multiply(&mut a, &mut b, &ctx), "{ctx}");
        }
    }
}

#[test]
fn err_e14_mul_negative_result_height() {
    // dims agree, but the result height (a.height) is negative.
    for ha in [-1, -4, i32::MIN] {
        for k in [0, 1, 3] {
            for wb in [0, 1, 2] {
                let mut a = fake_matrix(k, ha);
                let mut b = fake_matrix(wb, k);
                let ctx = format!("E14 a.height={ha} k={k} b.width={wb}");
                assert!(diff_multiply(&mut a, &mut b, &ctx), "{ctx}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 — matrix_to_string failure paths
// ---------------------------------------------------------------------------

#[test]
fn err_e16_to_string_null() {
    let out = diff_to_string(ptr::null_mut(), "E16");
    assert!(out.is_none(), "matrix_to_string(NULL) must return NULL");
    // repeat: the message must be emitted every single time
    for _ in 0..5 {
        assert!(diff_to_string(ptr::null_mut(), "E16 repeat").is_none());
    }
}

#[test]
fn err_e17_to_string_buffer_overflow() {
    // buffer_size = h*(11*w) + h + 1 wraps to a negative int ⇒ malloc fails.
    // (The row pointers are never touched on this path.)
    // (width, height, buffer_size wraps NEGATIVE ⇒ malloc must fail)
    let cases: &[(c_int, c_int, bool)] = &[
        (1, 200_000_000, true),
        (2, 100_000_000, true),
        (11, 20_000_000, true),
        (1, i32::MAX, true),
        (-1, 1, true),
        (1, -1, true),
        (i32::MIN, 1, true),
        // buffer_size wraps back to a small POSITIVE value: malloc succeeds and,
        // because height <= 0, the C code emits an empty string without ever
        // touching a row pointer.  The Rust port must do exactly the same.
        (-1, -1, false),
        (1, i32::MIN, false),
    ];
    for (w, h, expect_null) in cases {
        let mut m = fake_matrix(*w, *h);
        let ctx = format!("E17 {w}x{h}");
        let out = diff_to_string(&mut m, &ctx);
        if *expect_null {
            assert!(out.is_none(), "{ctx}: expected NULL, got {out:?}");
        } else {
            assert_eq!(out.as_deref(), Some(&b""[..]), "{ctx}");
        }
    }
}

// ---------------------------------------------------------------------------
// E18–E25 — write_to_file failure paths
// ---------------------------------------------------------------------------

#[test]
fn err_e18_write_null_content() {
    let dir = unique_dir("e18");
    let f = dir.join("out.txt");
    assert_eq!(
        diff_write(&path_cbuf(&f), &CBuf::null(), "E18"),
        EINVAL,
        "expected EINVAL"
    );
    assert!(!f.exists(), "no file must be created");
    // NULL content wins over an invalid file name, too.
    assert_eq!(
        diff_write(&CBuf::new("/nonexistent-dir/x"), &CBuf::null(), "E18b"),
        EINVAL
    );
    assert_eq!(diff_write(&CBuf::null(), &CBuf::null(), "E18c"), EINVAL);
}

#[test]
fn err_e19_write_enoent() {
    let dir = unique_dir("e19");
    let missing = dir.join("does/not/exist/out.txt");
    assert_eq!(
        diff_write(&path_cbuf(&missing), &CBuf::new("x"), "E19"),
        ENOENT
    );
    assert_eq!(
        diff_write(&CBuf::new("/no/such/path/at/all"), &CBuf::new(""), "E19b"),
        ENOENT
    );
}

#[test]
fn err_e20_write_eisdir() {
    let dir = unique_dir("e20");
    assert_eq!(
        diff_write(&path_cbuf(&dir), &CBuf::new("data"), "E20"),
        EISDIR
    );
    let rc_tmp = diff_write(&CBuf::new("/tmp"), &CBuf::new("data"), "E20b");
    assert_ne!(rc_tmp, 0, "opening a directory for writing must fail");
    // trailing slash on a regular file ⇒ ENOTDIR (20)
    let f = dir.join("file.txt");
    fs::write(&f, "x").unwrap();
    let with_slash = CBuf::new(format!("{}/", f.display()));
    let rc = diff_write(&with_slash, &CBuf::new("data"), "E20c");
    assert_ne!(rc, 0, "trailing slash on a file must fail");
}

#[test]
fn err_e21_write_eacces() {
    let dir = unique_dir("e21");
    // read-only file
    let ro_file = dir.join("readonly.txt");
    fs::write(&ro_file, "original").unwrap();
    fs::set_permissions(&ro_file, fs::Permissions::from_mode(0o444)).unwrap();
    assert_eq!(
        diff_write(&path_cbuf(&ro_file), &CBuf::new("nope"), "E21 file"),
        EACCES
    );
    assert_eq!(fs::read(&ro_file).unwrap(), b"original");

    // read-only directory
    let ro_dir = dir.join("rodir");
    fs::create_dir(&ro_dir).unwrap();
    fs::set_permissions(&ro_dir, fs::Permissions::from_mode(0o500)).unwrap();
    let target = ro_dir.join("new.txt");
    assert_eq!(
        diff_write(&path_cbuf(&target), &CBuf::new("nope"), "E21 dir"),
        EACCES
    );
    fs::set_permissions(&ro_dir, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn err_e22_write_empty_filename() {
    assert_eq!(diff_write(&CBuf::new(""), &CBuf::new("x"), "E22"), ENOENT);
    assert_eq!(diff_write(&CBuf::new(""), &CBuf::new(""), "E22b"), ENOENT);
}

#[test]
fn err_e23_write_enametoolong() {
    let long = "a".repeat(5000);
    assert_eq!(
        diff_write(&CBuf::new(long), &CBuf::new("x"), "E23"),
        ENAMETOOLONG
    );
    let long_component = format!("/tmp/{}", "b".repeat(300));
    assert_eq!(
        diff_write(&CBuf::new(long_component), &CBuf::new("x"), "E23b"),
        ENAMETOOLONG
    );
}

#[test]
fn err_e24_write_null_filename() {
    // fopen(NULL, "w") — glibc reports EFAULT; both libraries must agree.
    let rc = diff_write(&CBuf::null(), &CBuf::new("content"), "E24");
    assert_eq!(rc, EFAULT, "expected EFAULT from fopen(NULL)");
}

#[test]
fn err_e25_write_enospc_devfull() {
    // Small content: the failure surfaces in fclose(); 1 MiB content: it
    // surfaces already in fprintf(). Both paths must return ENOSPC.
    let dev_full = CBuf::new("/dev/full");
    assert_eq!(
        diff_write(&dev_full, &CBuf::new("small\n"), "E25 small"),
        ENOSPC
    );
    let big: Vec<u8> = (0..1024 * 1024).map(|i| b'a' + (i % 26) as u8).collect();
    assert_eq!(diff_write(&dev_full, &CBuf::new(big), "E25 big"), ENOSPC);
    // empty content still succeeds (nothing is written)
    assert_eq!(diff_write(&dev_full, &CBuf::new(""), "E25 empty"), 0);
}

// ---------------------------------------------------------------------------
// E32 — generic boundary sweep (one step past every documented range)
// ---------------------------------------------------------------------------

#[test]
fn err_e32_boundary_dims() {
    // allocate_matrix over the full interesting int range.
    // NOTE: height == INT_MAX is deliberately excluded: in C it would attempt
    // 2^31 successive row allocations (guaranteed machine OOM), which is a
    // resource limit rather than a behavioural difference; the "huge size_t"
    // path is covered by negative heights and by INT_MAX widths (E2/E4).
    let widths = [i32::MIN, -2, -1, 0, 1, 2, 3];
    let heights = [i32::MIN, -2, -1, 0, 1, 2, 3];
    let mut checked = 0usize;
    for w in widths {
        for h in heights {
            diff_allocate(w, h, "E32 allocate");
            checked += 1;
        }
    }
    assert_eq!(checked, widths.len() * heights.len(), "allocate sweep incomplete");

    // initialize_matrix_from_string over the same range, with a text that has
    // 3 rows of 3 columns.
    let text = CBuf::new("1 2 3\n4 5 6\n7 8 9\n");
    let mut init_checked = 0usize;
    for w in widths {
        for h in heights {
            diff_init(&text, w, h, &format!("E32 init {w}x{h}"));
            init_checked += 1;
        }
    }
    assert_eq!(
        init_checked,
        widths.len() * heights.len(),
        "init sweep incomplete"
    );

    // matrix_to_string on crafted shapes around the boundaries. Only shapes
    // with width <= 0 (or a failing malloc) are safe with a NULL row array;
    // every cell-touching shape is covered by Phase B.
    for (w, h) in [
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 1000),
        (-1, 0),
        (i32::MIN, 0),
        (5, 0),
        (i32::MAX, 0),
    ] {
        let mut m = fake_matrix(w, h);
        diff_to_string(&mut m, &format!("E32 to_string {w}x{h}"));
    }

    // multiply_matrices around the 0/-1 boundary of the dimension check.
    for k in [-1, 0, 1] {
        for ha in [-1, 0, 1] {
            for wb in [-1, 0, 1] {
                // k > 0 && ha > 0 && wb > 0 would make the C accumulation loop
                // dereference the (deliberately NULL) row array of the crafted
                // operands — that combination is covered with REAL matrices in
                // Phase B instead.
                if k > 0 && ha > 0 && wb > 0 {
                    continue;
                }
                let mut a = fake_matrix(k, ha);
                let mut b = fake_matrix(wb, k);
                diff_multiply(
                    &mut a,
                    &mut b,
                    &format!("E32 multiply k={k} ha={ha} wb={wb}"),
                );
            }
        }
    }

    // write_to_file with a zero-length name and a zero-length content.
    diff_write(&CBuf::new(""), &CBuf::new(""), "E32 write");
}

// ---------------------------------------------------------------------------
// Order independence — every differential helper above happens to call the C
// library FIRST.  Since several C error paths return the *global* `errno`, a
// Rust port that (incorrectly) read a stale `errno` would still look correct in
// that order.  These tests therefore run the very same error cases with the
// RUST library FIRST and additionally pin the exact expected error code.
// ---------------------------------------------------------------------------

/// Like `diff_write`, but the Rust library goes first.
fn diff_write_rust_first(filename: &CBuf, content: &CBuf, ctx: &str) -> c_int {
    let (c, r) = both();
    let (rrc, r_err) =
        capture_stderr(|| unsafe { (r.write_to_file)(filename.as_ptr(), content.as_ptr()) });
    let (crc, c_err) =
        capture_stderr(|| unsafe { (c.write_to_file)(filename.as_ptr(), content.as_ptr()) });
    assert_eq!(crc, rrc, "write_to_file return mismatch (rust first) [{ctx}]");
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("write_to_file stderr mismatch (rust first) [{ctx}]"),
    );
    rrc
}

#[test]
fn err_order_write_rust_first() {
    let dir = unique_dir("order");
    let ro_file = dir.join("ro.txt");
    fs::write(&ro_file, "orig").unwrap();
    fs::set_permissions(&ro_file, fs::Permissions::from_mode(0o444)).unwrap();

    assert_eq!(
        diff_write_rust_first(&path_cbuf(&dir.join("out.txt")), &CBuf::null(), "O1"),
        EINVAL
    );
    assert_eq!(
        diff_write_rust_first(&path_cbuf(&dir.join("no/such/dir/f")), &CBuf::new("x"), "O2"),
        ENOENT
    );
    assert_eq!(
        diff_write_rust_first(&path_cbuf(&dir), &CBuf::new("x"), "O3"),
        EISDIR
    );
    assert_eq!(
        diff_write_rust_first(&path_cbuf(&ro_file), &CBuf::new("x"), "O4"),
        EACCES
    );
    assert_eq!(
        diff_write_rust_first(&CBuf::new(""), &CBuf::new("x"), "O5"),
        ENOENT
    );
    assert_eq!(
        diff_write_rust_first(&CBuf::new("a".repeat(5000)), &CBuf::new("x"), "O6"),
        ENAMETOOLONG
    );
    assert_eq!(
        diff_write_rust_first(&CBuf::null(), &CBuf::new("x"), "O7"),
        EFAULT
    );
    assert_eq!(
        diff_write_rust_first(&CBuf::new("/dev/full"), &CBuf::new("data\n"), "O8"),
        ENOSPC
    );
    assert_eq!(
        diff_write_rust_first(&path_cbuf(&dir.join("ok.txt")), &CBuf::new("fine\n"), "O9"),
        0
    );

    // ... and each of them again, interleaved with a *successful* call so that
    // `errno` is guaranteed to hold a different value in between.
    for _ in 0..3 {
        assert_eq!(
            diff_write_rust_first(&path_cbuf(&dir.join("ok.txt")), &CBuf::new("fine\n"), "O10"),
            0
        );
        assert_eq!(
            diff_write_rust_first(&path_cbuf(&dir), &CBuf::new("x"), "O11"),
            EISDIR
        );
        assert_eq!(
            diff_write_rust_first(&CBuf::new(""), &CBuf::new("x"), "O12"),
            ENOENT
        );
    }
}

#[test]
fn err_order_matrix_rust_first() {
    let (c, r) = both();

    // allocate_matrix: the perror() text embeds strerror(errno) (ENOMEM).
    for (w, h) in [(5, -1), (-1, 5), (i32::MIN, 3), (3, i32::MIN)] {
        let (rp, r_err) = capture_stderr(|| unsafe { (r.allocate_matrix)(w, h) });
        let (cp, c_err) = capture_stderr(|| unsafe { (c.allocate_matrix)(w, h) });
        assert!(rp.is_null() && cp.is_null(), "allocate({w},{h}) should fail");
        assert_bytes_eq(
            &c_err,
            &r_err,
            &format!("allocate_matrix({w},{h}) stderr mismatch (rust first)"),
        );
    }

    // matrix_to_string with a negative buffer_size (perror path).
    for (w, h) in [(1, 200_000_000), (-1, 1)] {
        let mut m = fake_matrix(w, h);
        let mp: *mut MatrixT = &mut m;
        let (rs, r_err) = capture_stderr(|| unsafe { (r.matrix_to_string)(mp) });
        let (cs, c_err) = capture_stderr(|| unsafe { (c.matrix_to_string)(mp) });
        assert!(rs.is_null() && cs.is_null(), "to_string({w},{h}) should fail");
        assert_bytes_eq(
            &c_err,
            &r_err,
            &format!("matrix_to_string({w},{h}) stderr mismatch (rust first)"),
        );
    }

    // initialize_matrix_from_string error paths.
    for (text, w, h) in [("", 1, 1), ("1\n", 2, 1), ("1 2\n", 2, 3)] {
        let buf = CBuf::new(text);
        let (rp, r_err) =
            capture_stderr(|| unsafe { (r.initialize_matrix_from_string)(buf.as_ptr(), w, h) });
        let (cp, c_err) =
            capture_stderr(|| unsafe { (c.initialize_matrix_from_string)(buf.as_ptr(), w, h) });
        assert!(rp.is_null() && cp.is_null(), "init({text:?},{w},{h})");
        assert_bytes_eq(
            &c_err,
            &r_err,
            &format!("init({text:?},{w},{h}) stderr mismatch (rust first)"),
        );
    }

    // multiply_matrices dimension mismatch.
    let mut a = fake_matrix(2, 2);
    let mut b = fake_matrix(2, 3);
    let (ap, bp): (*mut MatrixT, *mut MatrixT) = (&mut a, &mut b);
    let (rp, r_err) = capture_stderr(|| unsafe { (r.multiply_matrices)(ap, bp) });
    let (cp, c_err) = capture_stderr(|| unsafe { (c.multiply_matrices)(ap, bp) });
    assert!(rp.is_null() && cp.is_null());
    assert_bytes_eq(&c_err, &r_err, "multiply stderr mismatch (rust first)");

    // free_matrix(NULL) either way round.
    unsafe {
        (r.free_matrix)(ptr::null_mut());
        (c.free_matrix)(ptr::null_mut());
    }
}

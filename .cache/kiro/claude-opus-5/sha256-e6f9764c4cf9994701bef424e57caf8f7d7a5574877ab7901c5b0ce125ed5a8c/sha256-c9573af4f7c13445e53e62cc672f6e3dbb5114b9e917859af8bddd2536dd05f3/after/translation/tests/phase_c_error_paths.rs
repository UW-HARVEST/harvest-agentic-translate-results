//! Phase C — error-path differential tests, one per row of `ERRORS.md`, plus
//! the generic FFI boundary cases.
//!
//! Each test constructs the exact invalid input, calls both shared objects
//! through their exports, and asserts they agree on the *specific* sentinel
//! (`-1`, `-2`, `NULL`) and on the exact stderr bytes — not merely that both
//! failed somehow.

mod common;

use common::*;

/// E1 — `forward_goto_example(x)` with `x < 0` takes `goto error`.
#[test]
fn err_e1_negative_x() {
    let mut rng = Rng::new(101);
    let mut xs = vec![-1i32, -2, -100, -12345, i32::MIN + 1];
    for _ in 0..256 {
        xs.push(rng.in_range_i32(i32::MIN, -1));
    }

    for x in xs {
        // Establish the C's own contract first, then compare Rust against it.
        let (c, r) = both();
        let (cv, cc) = capture(|| (c.forward_goto_example)(x));
        assert_eq!(cv, -1, "E1: C must return the -1 sentinel for x={x}");
        assert_eq!(
            cc.err, b"Error: negative input\n",
            "E1: C stderr text for x={x}"
        );
        assert!(cc.out.is_empty(), "E1: C writes nothing to stdout for x={x}");

        let (rv, rc) = capture(|| (r.forward_goto_example)(x));
        assert_eq!(rv, cv, "E1: Rust sentinel differs for x={x}");
        assert_eq!(rc.err, cc.err, "E1: Rust stderr differs for x={x}");
        assert_eq!(rc.out, cc.out, "E1: Rust stdout differs for x={x}");
    }
}

/// E2 — `INT_MIN`, the extreme boundary of the `x < 0` test.
#[test]
fn err_e2_int_min() {
    assert_eq!(diff_forward(i32::MIN), -1, "E2: INT_MIN -> -1");
    // One step either side of the sign boundary.
    assert_eq!(diff_forward(-1), -1, "E2: -1 -> -1");
    assert_eq!(diff_forward(0), 0, "E2: 0 is not negative -> 2*0");
    assert_eq!(diff_forward(1), 2, "E2: 1 -> 2");
}

/// E3 — `fopen` returns NULL (ENOENT): cleanup runs with `fp == NULL`, so no
/// `fclose`, and the result is the NULL sentinel.
#[test]
fn err_e3_fopen_enoent() {
    for _ in 0..16 {
        let missing = missing_path();
        let name = missing.as_os_str().as_encoded_bytes();

        let (c, r) = both();
        let (cfp, cc) = capture(|| unsafe { (c.open_with_cleanup)(cstr(name).as_ptr()) });
        assert!(cfp.is_null(), "E3: C must return the NULL sentinel");
        let expected = format!(
            "Error: opening or processing file {}\n",
            String::from_utf8_lossy(name)
        );
        assert_eq!(cc.err, expected.as_bytes(), "E3: C stderr text");
        assert!(cc.out.is_empty(), "E3: C writes nothing to stdout");

        let (rfp, rc) = capture(|| unsafe { (r.open_with_cleanup)(cstr(name).as_ptr()) });
        assert!(rfp.is_null(), "E3: Rust must also return NULL");
        assert_eq!(rc.err, cc.err, "E3: stderr differs");
        assert_eq!(rc.out, cc.out, "E3: stdout differs");
    }
}

/// E4 — every other way `fopen` can fail. The C never inspects `errno`, so all
/// of them must collapse to the same NULL + message behaviour.
#[test]
fn err_e4_fopen_other_failures() {
    // Empty filename -> ENOENT.
    assert!(diff_open(Some(b"")).null, "E4: empty filename");

    // Path with a non-directory component -> ENOTDIR.
    let f = TempPath::file(b"i am a file\n");
    let mut not_a_dir = f.bytes();
    not_a_dir.extend_from_slice(b"/child");
    assert!(diff_open(Some(&not_a_dir)).null, "E4: ENOTDIR");

    // Oversized single component -> ENAMETOOLONG.
    let too_long = format!("/tmp/{}", "n".repeat(5000));
    assert!(diff_open(Some(too_long.as_bytes())).null, "E4: ENAMETOOLONG");

    // Unreadable file -> EACCES (skipped when running as root, where the
    // permission bits do not apply).
    let secret = TempPath::file(b"secret\n");
    let mut perms = std::fs::metadata(&secret.path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
    }
    std::fs::set_permissions(&secret.path, perms).unwrap();
    let readable_anyway = std::fs::File::open(&secret.path).is_ok();
    if readable_anyway {
        eprintln!("E4: running with privileges that bypass mode 000; EACCES case skipped");
    } else {
        assert!(diff_open(Some(&secret.bytes())).null, "E4: EACCES");
    }
    // Restore so the TempPath drop can unlink it.
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&secret.path).unwrap().permissions();
        p.set_mode(0o600);
        std::fs::set_permissions(&secret.path, p).unwrap();
    }

    // A path that is a symlink loop -> ELOOP.
    let d = TempPath::dir();
    let loop_a = d.path.join("a");
    let loop_b = d.path.join("b");
    if std::os::unix::fs::symlink(&loop_b, &loop_a).is_ok()
        && std::os::unix::fs::symlink(&loop_a, &loop_b).is_ok()
    {
        assert!(
            diff_open(Some(loop_a.as_os_str().as_encoded_bytes())).null,
            "E4: ELOOP"
        );
    }

    // A dangling symlink -> ENOENT through a different kernel path.
    let dangling = d.path.join("dangling");
    if std::os::unix::fs::symlink(missing_path(), &dangling).is_ok() {
        assert!(
            diff_open(Some(dangling.as_os_str().as_encoded_bytes())).null,
            "E4: dangling symlink"
        );
    }
}

/// E5 — `ferror(fp)` true after the read loop. Reached by `fopen`-ing a
/// directory: the open succeeds on Linux, then `fgets` fails with `EISDIR`.
/// This is the branch where cleanup *does* call `fclose`.
#[test]
fn err_e5_ferror_directory() {
    let d = TempPath::dir();
    let name = d.bytes();

    let (c, r) = both();
    let (cfp, cc) = capture(|| unsafe { (c.open_with_cleanup)(cstr(&name).as_ptr()) });
    assert!(cfp.is_null(), "E5: C must return NULL on the ferror path");
    let expected = format!(
        "Error: opening or processing file {}\n",
        String::from_utf8_lossy(&name)
    );
    assert_eq!(cc.err, expected.as_bytes(), "E5: C stderr text");
    assert!(cc.out.is_empty(), "E5: nothing readable, so no stdout");

    let (rfp, rc) = capture(|| unsafe { (r.open_with_cleanup)(cstr(&name).as_ptr()) });
    assert!(rfp.is_null(), "E5: Rust must also return NULL");
    assert_eq!(rc.err, cc.err, "E5: stderr differs");
    assert_eq!(rc.out, cc.out, "E5: stdout differs");

    // Non-empty directories and `/tmp` itself take the same branch.
    let populated = TempPath::dir();
    std::fs::write(populated.path.join("child"), b"x").unwrap();
    assert!(diff_open(Some(&populated.bytes())).null, "E5: populated dir");
    assert!(diff_open(Some(b"/tmp")).null, "E5: /tmp");
    assert!(diff_open(Some(b"/")).null, "E5: root dir");
}

/// E6 — NULL `filename`: forwarded straight into `fopen` and then into
/// `fprintf`'s `%s`, which glibc renders as `(null)`.
#[test]
fn err_e6_null_filename() {
    let (c, r) = both();

    let (cfp, cc) = capture(|| unsafe { (c.open_with_cleanup)(std::ptr::null()) });
    assert!(cfp.is_null(), "E6: C returns NULL");
    let (rfp, rc) = capture(|| unsafe { (r.open_with_cleanup)(std::ptr::null()) });
    assert!(rfp.is_null(), "E6: Rust returns NULL");
    assert_eq!(cc.err, rc.err, "E6: stderr differs for NULL filename");
    assert_eq!(cc.out, rc.out, "E6: stdout differs for NULL filename");
    assert!(
        cc.err.starts_with(b"Error: opening or processing file "),
        "E6: C still emitted the cleanup message: {:?}",
        String::from_utf8_lossy(&cc.err)
    );

    // Same through the composed entry point, for both signs of `num`.
    assert_eq!(diff_driver(0, None), -2, "E6: driver(0, NULL) -> -2");
    assert_eq!(diff_driver(7, None), -2, "E6: driver(7, NULL) -> -2");
    assert_eq!(
        diff_driver(-7, None),
        -1,
        "E6: driver(-7, NULL) short-circuits to -1 without dereferencing"
    );
    assert_eq!(diff_driver(i32::MAX, None), -2, "E6: driver(INT_MAX, NULL)");
    assert_eq!(diff_driver(i32::MIN, None), -1, "E6: driver(INT_MIN, NULL)");
}

/// E7 — `driver` returns `-1` when `res == -1`, never touching the file.
#[test]
fn err_e7_driver_negative_num() {
    let f = TempPath::file(b"should not be read\n");
    let good = f.bytes();

    let mut rng = Rng::new(107);
    let mut nums = vec![-1i32, -2, -999, i32::MIN, i32::MIN + 1];
    for _ in 0..64 {
        nums.push(rng.in_range_i32(i32::MIN, -1));
    }

    for num in nums {
        let (c, r) = both();
        let (cv, cc) = capture(|| unsafe { (c.driver)(num, cstr(&good).as_ptr()) });
        assert_eq!(cv, -1, "E7: C returns the -1 sentinel for num={num}");
        assert_eq!(
            cc.err, b"Error: negative input\n",
            "E7: only the forward_goto_example message appears"
        );
        assert!(
            cc.out.is_empty(),
            "E7: no `Goto output:` line and no file contents for num={num}"
        );

        let (rv, rc) = capture(|| unsafe { (r.driver)(num, cstr(&good).as_ptr()) });
        assert_eq!(rv, cv, "E7: Rust sentinel differs for num={num}");
        assert_eq!(rc.err, cc.err, "E7: stderr differs for num={num}");
        assert_eq!(rc.out, cc.out, "E7: stdout differs for num={num}");
    }
}

/// E8 — `driver` returns `-2` when `open_with_cleanup` fails, for each distinct
/// underlying failure (E3/E4/E5/E6).
#[test]
fn err_e8_driver_file_failure() {
    let missing = missing_path();
    let d = TempPath::dir();
    let too_long = format!("/tmp/{}", "n".repeat(5000));
    let f = TempPath::file(b"file\n");
    let mut not_a_dir = f.bytes();
    not_a_dir.extend_from_slice(b"/child");

    let cases: Vec<(&str, Option<Vec<u8>>)> = vec![
        ("ENOENT", Some(missing.as_os_str().as_encoded_bytes().to_vec())),
        ("empty name", Some(Vec::new())),
        ("ENAMETOOLONG", Some(too_long.into_bytes())),
        ("ENOTDIR", Some(not_a_dir)),
        ("directory/ferror", Some(d.bytes())),
        ("NULL", None),
    ];

    for (label, name) in cases {
        for num in [0i32, 1, 42, i32::MAX / 2, i32::MAX] {
            let (c, r) = both();
            let owned = name.clone().map(|n| cstr(&n));
            let ptr = owned.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
            let (cv, cc) = capture(|| unsafe { (c.driver)(num, ptr) });
            assert_eq!(cv, -2, "E8/{label}: C returns the -2 sentinel (num={num})");
            assert!(
                cc.out.starts_with(b"Processing: "),
                "E8/{label}: stdout still has the success prefix"
            );

            let owned = name.clone().map(|n| cstr(&n));
            let ptr = owned.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
            let (rv, rc) = capture(|| unsafe { (r.driver)(num, ptr) });
            assert_eq!(rv, cv, "E8/{label}: Rust sentinel differs (num={num})");
            assert_eq!(rc.out, cc.out, "E8/{label}: stdout differs (num={num})");
            assert_eq!(rc.err, cc.err, "E8/{label}: stderr differs (num={num})");
        }
    }
}

/// E9 — the `sizeof(buffer) == 100` bound: chunking must happen at identical
/// offsets, which is only observable through the emitted byte stream.
#[test]
fn err_e9_buffer_boundary() {
    // Exhaustive sweep across two full buffer widths, with and without a
    // trailing newline, and with a newline at every possible offset.
    for len in 0..=205usize {
        let content = vec![b'Q'; len];
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "E9: len={len}");
    }

    for nl in 0..=205usize {
        let mut content = vec![b'R'; 206];
        content[nl] = b'\n';
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "E9: newline at {nl}");
    }
}

/// E10 — embedded NUL bytes make `printf("%s", buffer)` lossy; both must lose
/// exactly the same bytes.
#[test]
fn err_e10_embedded_nul() {
    // NUL at every offset within the first two chunks.
    for pos in 0..=200usize {
        let mut content = vec![b'S'; 205];
        content[pos] = 0;
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "E10: NUL at {pos}");
    }

    // All-NUL, alternating, and NUL-then-newline shapes.
    let shapes: Vec<Vec<u8>> = vec![
        vec![0u8; 250],
        (0..250).map(|i| if i % 2 == 0 { 0 } else { b'T' }).collect(),
        b"\0\n\0\n\0\n".to_vec(),
        b"before\0after\n".to_vec(),
        vec![0u8; 1],
    ];
    for (i, s) in shapes.iter().enumerate() {
        let f = TempPath::file(s);
        assert!(!diff_open(Some(&f.bytes())).null, "E10: shape {i}");
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary sweep (required regardless of the ERRORS.md rows)
// ---------------------------------------------------------------------------

/// Out-of-range integer arguments across the FFI boundary. The API has no
/// `enum` parameter, so the analogous case is the full `int32` domain of `num`
/// including both extremes and the values straddling every branch.
#[test]
fn boundary_int_domain() {
    let f = TempPath::file(b"z\n");
    let good = f.bytes();

    let interesting: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX / 2 - 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        (1 << 30) - 1,
        1 << 30,
        (1 << 30) + 1,
        i32::MAX - 1,
        i32::MAX,
        // Values that only differ from the above in the high bit / sign
        // reinterpretation, i.e. what an out-of-range enum would look like.
        u32::MAX as i32,
        0x8000_0000u32 as i32,
        0xDEAD_BEEFu32 as i32,
        0x7FFF_FFFF,
    ];

    for x in &interesting {
        diff_forward(*x);
        diff_driver(*x, Some(&good));
        diff_driver(*x, None);
    }
}

/// NULL pointer, zero length, and oversized length in one place.
#[test]
fn boundary_pointer_and_length() {
    // NULL.
    assert!(diff_open(None).null, "NULL filename -> NULL");
    assert_eq!(diff_driver(1, None), -2, "driver with NULL filename -> -2");

    // Zero-length name and zero-length file.
    assert!(diff_open(Some(b"")).null, "empty filename -> NULL");
    let empty = TempPath::file(b"");
    assert!(!diff_open(Some(&empty.bytes())).null, "zero-byte file opens");

    // Oversized: names past NAME_MAX and past PATH_MAX.
    let past_name_max = format!("/tmp/{}", "x".repeat(300));
    assert!(diff_open(Some(past_name_max.as_bytes())).null, "past NAME_MAX");
    let past_path_max = format!("/tmp{}", "/yyyyyyyy".repeat(1000));
    assert!(diff_open(Some(past_path_max.as_bytes())).null, "past PATH_MAX");

    // Oversized content: far more than the 100-byte buffer.
    let big = TempPath::file(&vec![b'W'; 1 << 20]);
    assert!(!diff_open(Some(&big.bytes())).null, "1 MiB file");
}

/// Repeated calls on the same failing input: state must not drift between the
/// two objects (e.g. a leaked fd on the `ferror` path would eventually diverge).
#[test]
fn boundary_repeated_failures() {
    let d = TempPath::dir();
    let dir = d.bytes();
    let missing = missing_path();
    let bad = missing.as_os_str().as_encoded_bytes().to_vec();

    for i in 0..300 {
        assert!(diff_open(Some(&dir)).null, "repeat {i}: ferror path");
        assert!(diff_open(Some(&bad)).null, "repeat {i}: fopen path");
    }
}

// ---------------------------------------------------------------------------

fn cstr(bytes: &[u8]) -> std::ffi::CString {
    std::ffi::CString::new(bytes).expect("no interior NUL in a filename")
}

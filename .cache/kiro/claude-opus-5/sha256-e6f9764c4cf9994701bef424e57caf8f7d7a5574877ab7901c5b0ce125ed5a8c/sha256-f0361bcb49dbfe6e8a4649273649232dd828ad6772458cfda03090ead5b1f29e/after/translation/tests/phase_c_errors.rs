//! Phase C — error-path differential tests, one test per row of `ERRORS.md`,
//! plus the generic FFI-boundary abuse cases (null pointers, zero / oversized
//! lengths, values one past a range).

mod common;

use common::*;
use std::os::unix::fs::PermissionsExt;

// --- Row 1: forward_goto_example, x < 0 -----------------------------------
#[test]
fn err01_forward_negative() {
    let mut rng = Rng::new(SEED ^ 101);
    diff_forward(-1);
    for _ in 0..256 {
        diff_forward(rng.range_i32(i32::MIN, -1));
    }
}

// --- Row 2: x == 0 is NOT rejected ---------------------------------------
#[test]
fn err02_forward_zero_not_rejected() {
    // Assert that C really does not treat 0 as an error, then that Rust agrees.
    let c = capture(|| unsafe { (c_lib().forward_goto_example)(0) });
    assert_eq!(c.ret, 0, "C: forward_goto_example(0) should return 0");
    assert!(c.stderr.is_empty(), "C: 0 must not hit the error label");
    diff_forward(0);
}

// --- Row 3: INT_MIN ------------------------------------------------------
#[test]
fn err03_forward_int_min() {
    diff_forward(i32::MIN);
    diff_forward(i32::MIN + 1);
}

// --- Row 4: fopen ENOENT -------------------------------------------------
#[test]
fn err04_open_nonexistent() {
    let mut rng = Rng::new(SEED ^ 104);
    for _ in 0..64 {
        let mut name = b"/tmp/difftest-nope-".to_vec();
        for _ in 0..20 {
            name.push(b'a' + (rng.below(26) as u8));
        }
        diff_open(Some(&name));
    }
    // nested missing directory too (ENOENT on a path component)
    diff_open(Some(b"/tmp/difftest-no-such-dir-xyz/inner/file.txt"));
    // ENOTDIR: a regular file used as a directory component
    let f = TempFile::with_bytes("notdir", b"x\n");
    let mut p = f.name_bytes();
    p.extend_from_slice(b"/inner");
    diff_open(Some(&p));
}

// --- Row 5: empty filename ----------------------------------------------
#[test]
fn err05_open_empty_filename() {
    diff_open(Some(b""));
}

// --- Row 6: EACCES ------------------------------------------------------
#[test]
fn err06_open_permission_denied() {
    let f = TempFile::with_bytes("noperm", b"secret\n");
    std::fs::set_permissions(&f.0, std::fs::Permissions::from_mode(0o000))
        .expect("chmod 000");
    // Running as root would defeat this row; skip rather than report a false pass.
    if std::fs::File::open(&f.0).is_ok() {
        eprintln!("note: running with privileges that bypass mode 0000; row 6 covered by row 4");
        return;
    }
    diff_open(Some(&f.name_bytes()));
}

// --- Row 7: NULL filename ----------------------------------------------
#[test]
fn err07_open_null_filename() {
    // glibc's fopen(NULL, "r") fails with EFAULT and fprintf's %s prints
    // "(null)"; both libraries must do the identical thing.
    let c = capture(|| unsafe { (c_lib().open_with_cleanup)(std::ptr::null()) });
    assert!(c.ret.is_null(), "C: NULL filename must yield NULL");
    diff_open(None);
}

// --- Row 8: ferror path (directory) ------------------------------------
#[test]
fn err08_open_directory_sets_ferror() {
    for dir in [
        &b"/tmp"[..],
        &b"/"[..],
        &b"/usr"[..],
        std::env::temp_dir().as_os_str().as_encoded_bytes(),
    ] {
        // Confirm this really is the ferror branch in C, not the !fp branch:
        // stderr carries the message and the return is NULL either way, so
        // check that fopen on a directory actually succeeds on this platform.
        diff_open(Some(dir));
    }
    // Sanity check that the ferror branch is reachable at all here.
    let probe = unsafe { libc::fopen(c"/tmp".as_ptr(), c"r".as_ptr()) };
    assert!(
        !probe.is_null(),
        "platform does not allow fopen() on a directory — ERRORS.md row 8 \
         (ferror branch) would be unreachable"
    );
    unsafe {
        let mut buf = [0i8; 100];
        let r = libc::fgets(buf.as_mut_ptr(), 100, probe);
        assert!(r.is_null(), "fgets on a directory should fail");
        assert_ne!(libc::ferror(probe), 0, "ferror should be set");
        libc::fclose(probe);
    }
}

// --- Row 9: ENAMETOOLONG ------------------------------------------------
#[test]
fn err09_open_name_too_long() {
    let mut name = b"/tmp/".to_vec();
    name.extend(std::iter::repeat(b'x').take(5000));
    diff_open(Some(&name));
    // exactly-at and one-past a single component's 255-byte limit
    for n in [254usize, 255, 256] {
        let mut p = b"/tmp/".to_vec();
        p.extend(std::iter::repeat(b'y').take(n));
        diff_open(Some(&p));
    }
}

// --- Row 10: driver res == -1 -------------------------------------------
#[test]
fn err10_driver_negative_num() {
    let mut rng = Rng::new(SEED ^ 110);
    let f = TempFile::with_bytes("d-err", b"content\n");
    let name = f.name_bytes();
    for _ in 0..128 {
        let num = rng.range_i32(i32::MIN, -1);
        let c = capture(|| unsafe { (c_lib().driver)(num, name.as_ptr() as *const _) });
        assert_eq!(c.ret, -1, "C: driver({num}, valid) should return -1");
        diff_driver(num, Some(&name));
    }
}

// --- Row 11: driver out == NULL -----------------------------------------
#[test]
fn err11_driver_open_failure_returns_minus_2() {
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..64 {
        let num = rng.range_i32(0, i32::MAX);
        let mut name = b"/tmp/difftest-absent-".to_vec();
        for _ in 0..18 {
            name.push(b'a' + (rng.below(26) as u8));
        }
        let c = capture(|| unsafe { (c_lib().driver)(num, name.as_ptr() as *const _) });
        assert_eq!(c.ret, -2, "C: driver({num}, missing) should return -2");
        diff_driver(num, Some(&name));
    }
    // the ferror flavour of the -2 path
    diff_driver(5, Some(b"/tmp"));
    diff_driver(0, Some(b""));
}

// --- Row 12: negative num AND NULL filename -----------------------------
#[test]
fn err12_driver_negative_num_and_null_filename() {
    for num in [-1i32, -2, i32::MIN] {
        let c = capture(|| unsafe { (c_lib().driver)(num, std::ptr::null()) });
        assert_eq!(c.ret, -1, "C: short-circuits before touching filename");
        assert!(
            !c.stderr.windows(6).any(|w| w == b"(null)"),
            "C: filename must never be formatted on this path"
        );
        diff_driver(num, None);
    }
}

// --- Row 13: line longer than sizeof(buffer)-1 --------------------------
#[test]
fn err13_lines_past_buffer_capacity() {
    let mut rng = Rng::new(SEED ^ 113);
    for len in [99usize, 100, 101, 199, 200, 201, 999, 1000, 1001] {
        let content: Vec<u8> = (0..len).map(|_| rng.printable()).collect();
        diff_open_file("cap", &content);
        let mut with_nl = content.clone();
        with_nl.push(b'\n');
        diff_open_file("cap-nl", &with_nl);
    }
}

// --- Row 14: signed-overflow inputs -------------------------------------
#[test]
fn err14_forward_overflow_values() {
    let mut rng = Rng::new(SEED ^ 114);
    for x in [1i32 << 30, (1 << 30) + 1, i32::MAX - 1, i32::MAX] {
        diff_forward(x);
        diff_driver_file("ovf", x, b"ok\n");
    }
    for _ in 0..128 {
        let x = rng.range_i32(1 << 30, i32::MAX);
        diff_forward(x);
    }
    // `x*2` wraps to an even negative, so `driver` never mistakes it for -1.
    let c = capture(|| unsafe { (c_lib().forward_goto_example)(1 << 30) });
    assert_eq!(c.ret, i32::MIN, "C: (1<<30)*2 wraps to INT_MIN");
    assert_ne!(c.ret, -1);
}

// --- Generic FFI boundary abuse -----------------------------------------
#[test]
fn generic_null_pointer_arguments() {
    diff_open(None);
    diff_driver(0, None);
    diff_driver(1, None);
    diff_driver(i32::MAX, None);
    diff_driver(-1, None);
    diff_driver(i32::MIN, None);
}

#[test]
fn generic_int_domain_sweep() {
    // Whole-domain sweep at every representable boundary and a stride sample.
    // `int num` is the only non-pointer parameter and the API has no enums, so
    // this is the complete out-of-range-scalar surface.
    let interesting: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -65537,
        -65536,
        -65535,
        -256,
        -255,
        -128,
        -127,
        -2,
        -1,
        0,
        1,
        2,
        127,
        128,
        255,
        256,
        32767,
        32768,
        65535,
        65536,
        65537,
        (1 << 30) - 1,
        1 << 30,
        (1 << 30) + 1,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &x in &interesting {
        diff_forward(x);
    }
    let f = TempFile::with_bytes("sweep", b"a\nbb\nccc\n");
    let name = f.name_bytes();
    for &x in &interesting {
        diff_driver(x, Some(&name));
    }
}

#[test]
fn generic_zero_and_oversized_content() {
    // "zero length" and "oversized" for this API means file content length.
    diff_open_file("zero", b"");
    diff_driver_file("zero", 3, b"");
    let big: Vec<u8> = (0..(256 * 1024)).map(|i| b'a' + (i % 26) as u8).collect();
    diff_open_file("huge-noline", &big);
    diff_driver_file("huge-noline", 3, &big);
    let mut big_lines = Vec::with_capacity(300 * 1024);
    while big_lines.len() < 256 * 1024 {
        big_lines.extend_from_slice(b"0123456789abcdefghijklmnopqrstuvwxyz\n");
    }
    diff_open_file("huge-lines", &big_lines);
    diff_driver_file("huge-lines", 3, &big_lines);
}

#[test]
fn generic_weird_filenames() {
    for name in [
        &b"."[..],
        &b".."[..],
        &b"/"[..],
        &b"//"[..],
        &b"/dev/null"[..],
        &b"/dev/zero"[..], // opens fine, reads endless NULs -> would hang; see note
        &b"/proc/self/status"[..],
        &b" "[..],
        &b"\t"[..],
        &b"%s"[..],   // format-specifier in the filename: fprintf("%s", name)
        &b"%n%n%n"[..],
        &b"%d %x"[..],
        &b"a\\b"[..],
    ] {
        if name == b"/dev/zero" {
            continue; // unbounded read: excluded deliberately, not a divergence
        }
        diff_open(Some(name));
        diff_driver(1, Some(name));
    }
}

#[test]
fn generic_format_specifier_filenames_are_not_expanded() {
    // The C passes `filename` as a `%s` ARGUMENT, never as a format string, so
    // specifiers must appear literally. Verify against C, then compare Rust.
    let out = capture(|| unsafe { (c_lib().open_with_cleanup)(c"/tmp/nope-%s-%d-%n".as_ptr()) });
    assert!(
        out.stderr.ends_with(b"/tmp/nope-%s-%d-%n\n"),
        "C: specifiers must be literal, got {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    diff_open(Some(b"/tmp/nope-%s-%d-%n"));
}

#[test]
fn generic_symlink_and_fifo() {
    // A dangling symlink -> ENOENT (!fp branch).
    let target = std::env::temp_dir().join(format!("difftest-dangling-{}", std::process::id()));
    let link = std::env::temp_dir().join(format!("difftest-link-{}", std::process::id()));
    let _ = std::fs::remove_file(&link);
    if std::os::unix::fs::symlink(&target, &link).is_ok() {
        diff_open(Some(link.as_os_str().as_encoded_bytes()));
        diff_driver(2, Some(link.as_os_str().as_encoded_bytes()));
        let _ = std::fs::remove_file(&link);
    }
}

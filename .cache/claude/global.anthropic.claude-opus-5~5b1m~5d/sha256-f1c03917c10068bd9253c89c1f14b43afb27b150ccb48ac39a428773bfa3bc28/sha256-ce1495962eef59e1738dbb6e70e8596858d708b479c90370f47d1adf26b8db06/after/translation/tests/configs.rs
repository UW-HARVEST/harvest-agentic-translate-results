// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row drives BOTH `.so`s through their
// exported C symbols and compares return value + stream state + the exact
// stdout/stderr bytes. Rows use many randomized inputs from a fixed seed.
//
// Test names are prefixed `cfg_NN_` matching the CONFIGS.md row numbers.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1-4 — forward_goto_example (lowest-level entry point, called directly)
// ---------------------------------------------------------------------------

/// Row 1: x < 0 (the `goto error` branch), randomized over INT_MIN..0.
#[test]
fn cfg_01_fge_negative() {
    let mut rng = Rng::new(1);
    diff_forward(-1, "row1/-1");
    diff_forward(i32::MIN, "row1/INT_MIN");
    for i in 0..400 {
        let x = rng.range_i64(i32::MIN as i64, -1) as i32;
        diff_forward(x, &format!("row1/rand#{i}"));
    }
}

/// Row 2: x == 0 — the boundary between the two branches.
#[test]
fn cfg_02_fge_zero() {
    diff_forward(0, "row2/zero");
}

/// Row 3: 0 < x < 2^30 — happy path, `x * 2` does not wrap.
#[test]
fn cfg_03_fge_positive() {
    let mut rng = Rng::new(3);
    for x in [1i32, 2, 3, 7, 99, 1000, 65535, (1 << 29) - 1] {
        diff_forward(x, "row3/fixed");
    }
    for i in 0..400 {
        let x = rng.range_i64(1, (1i64 << 30) - 1) as i32;
        diff_forward(x, &format!("row3/rand#{i}"));
    }
}

/// Row 4: x >= 2^30 — `x * 2` wraps to a negative value (gcc -O0 emits
/// `add %eax,%eax`, i.e. plain two's-complement wrap-around).
#[test]
fn cfg_04_overflow() {
    let mut rng = Rng::new(4);
    for x in [(1i32 << 30) - 1, 1i32 << 30, i32::MAX, i32::MAX - 1] {
        diff_forward(x, "row4/boundary");
    }
    for i in 0..400 {
        let x = rng.range_i64(1i64 << 30, i32::MAX as i64) as i32;
        diff_forward(x, &format!("row4/rand#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 5-17 — open_with_cleanup (called directly, not via driver)
// ---------------------------------------------------------------------------

/// Row 5: 0-byte regular file — the fgets loop body never executes.
#[test]
fn cfg_05_empty_file() {
    let p = put_file("cfg05-empty", b"");
    diff_open_path(&p, "row5/empty");
}

/// Row 6: one line shorter than the 99-byte fgets chunk, WITH trailing newline.
#[test]
fn cfg_06_one_line_nl() {
    let mut rng = Rng::new(6);
    for i in 0..200 {
        let n = rng.range(0, 98);
        let mut body = rng.text(n);
        body.push(b'\n');
        let p = put_file("cfg06-oneline-nl", &body);
        diff_open_path(&p, &format!("row6/len={n}#{i}"));
    }
}

/// Row 7: one line shorter than the chunk, WITHOUT trailing newline.
#[test]
fn cfg_07_one_line_no_nl() {
    let mut rng = Rng::new(7);
    for i in 0..200 {
        let n = rng.range(1, 98);
        let body = rng.text(n);
        let p = put_file("cfg07-oneline-nonl", &body);
        diff_open_path(&p, &format!("row7/len={n}#{i}"));
    }
}

/// Row 8: many lines, randomized counts and lengths (loop runs many times).
#[test]
fn cfg_08_multi_line() {
    let mut rng = Rng::new(8);
    for i in 0..200 {
        let lines = rng.range(2, 40);
        let maxlen = rng.range(0, 250);
        let trailing = rng.next_u64() & 1 == 0;
        let body = random_lines(&mut rng, lines, maxlen, trailing);
        let p = put_file("cfg08-multi", &body);
        diff_open_path(&p, &format!("row8/lines={lines} maxlen={maxlen}#{i}"));
    }
}

/// Row 9: line lengths straddling `sizeof(buffer) - 1 == 99`, the exact number
/// of data bytes one `fgets` call can return.
#[test]
fn cfg_09_chunk_boundary() {
    for len in [97usize, 98, 99, 100, 101, 199, 200, 201] {
        for &nl in &[false, true] {
            let mut body = vec![b'A'; len];
            if nl {
                body.push(b'\n');
            }
            let p = put_file("cfg09-chunk", &body);
            diff_open_path(&p, &format!("row9/len={len} nl={nl}"));
        }
    }
}

/// Row 10: a single 64 KiB line with no newline at all — hundreds of chunked
/// `fgets` trips, each printed with `printf("%s", ...)`.
#[test]
fn cfg_10_huge_single_line() {
    let mut rng = Rng::new(10);
    for i in 0..3 {
        let n = 64 * 1024 + i;
        let body = rng.text(n);
        let p = put_file("cfg10-huge", &body);
        diff_open_path(&p, &format!("row10/len={n}"));
    }
}

/// Row 11: embedded NUL bytes — `printf("%s", buffer)` stops at the first NUL,
/// so the emitted bytes deliberately differ from the file contents.
#[test]
fn cfg_11_embedded_nul() {
    let mut rng = Rng::new(11);
    // Hand-built shapes that put the NUL at interesting offsets.
    let fixed: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"\0\n".to_vec(),
        b"abc\0def\n".to_vec(),
        b"\0abc\n".to_vec(),
        b"abc\0".to_vec(),
        [vec![b'x'; 98], vec![0u8], b"tail\n".to_vec()].concat(),
        [vec![b'x'; 99], vec![0u8], b"tail\n".to_vec()].concat(),
        [vec![b'x'; 100], vec![0u8], b"tail\n".to_vec()].concat(),
        [vec![0u8; 300], b"\n".to_vec()].concat(),
    ];
    for (i, body) in fixed.iter().enumerate() {
        let p = put_file("cfg11-nul-fixed", body);
        diff_open_path(&p, &format!("row11/fixed#{i}"));
    }
    for i in 0..200 {
        let n = rng.range(1, 400);
        let mut body = rng.text(n);
        // Sprinkle NULs and newlines.
        let holes = rng.range(1, 6);
        for _ in 0..holes {
            let at = rng.range(0, n - 1);
            body[at] = 0;
        }
        let nls = rng.range(0, 6);
        for _ in 0..nls {
            let at = rng.range(0, n - 1);
            body[at] = b'\n';
        }
        let p = put_file("cfg11-nul", &body);
        diff_open_path(&p, &format!("row11/rand#{i} len={n}"));
    }
}

/// Row 12: fully random binary content (0x00..=0xFF): `%` conversion chars,
/// CR, high-bit bytes, invalid UTF-8, NULs, newlines.
#[test]
fn cfg_12_random_binary() {
    let mut rng = Rng::new(12);
    for i in 0..200 {
        let n = rng.range(1, 700);
        let body = rng.bytes(n);
        let p = put_file("cfg12-bin", &body);
        diff_open_path(&p, &format!("row12/rand#{i} len={n}"));
    }
    // Explicitly include format-specifier-looking payloads.
    for body in [
        &b"%s%s%s%n\n"[..],
        &b"%d %p %x\n"[..],
        &b"100%\n"[..],
        &b"\r\n\r\n"[..],
        &b"\xff\xfe\xfd\n"[..],
    ] {
        let p = put_file("cfg12-fmt", body);
        diff_open_path(&p, "row12/fmt");
    }
}

/// Row 13: only newlines — many single-byte loop trips.
#[test]
fn cfg_13_blank_lines() {
    let mut rng = Rng::new(13);
    for i in 0..100 {
        let n = rng.range(1, 64);
        let body = vec![b'\n'; n];
        let p = put_file("cfg13-blank", &body);
        diff_open_path(&p, &format!("row13/n={n}#{i}"));
    }
}

/// Row 14: 1 MiB file — far more than one stdio buffer, thousands of trips.
#[test]
fn cfg_14_large_file() {
    let mut rng = Rng::new(14);
    for i in 0..2 {
        let mut body = Vec::with_capacity(1 << 20);
        while body.len() < (1 << 20) {
            let n = rng.range(0, 300);
            body.extend_from_slice(&rng.text(n));
            body.push(b'\n');
        }
        let p = put_file("cfg14-large", &body);
        diff_open_path(&p, &format!("row14/#{i} len={}", body.len()));
    }
}

/// Row 15: `/dev/null` — opens successfully, zero loop trips, no error.
#[test]
fn cfg_15_dev_null() {
    diff_open(Some(b"/dev/null"), "row15/dev-null");
}

/// Row 16: symlink to a randomized regular file.
#[test]
fn cfg_16_symlink() {
    let mut rng = Rng::new(16);
    let link = scratch_dir().join("cfg16-link");
    for i in 0..50 {
        let lines = rng.range(0, 10);
        let body = random_lines(&mut rng, lines, 120, true);
        let target = put_file("cfg16-target", &body);
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        diff_open_path(&link, &format!("row16/#{i}"));
    }
}

/// Row 17: on success the function hands the caller an OPEN `FILE*`; its
/// position / EOF / error state is part of the observable output.
#[test]
fn cfg_17_returned_stream_state() {
    let mut rng = Rng::new(17);
    for i in 0..200 {
        let n = rng.range(0, 5000);
        let body = rng.text(n);
        let p = put_file("cfg17-state", &body);
        diff_open_path(&p, &format!("row17/size={n}#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 18-22 — driver (the composed pipeline)
// ---------------------------------------------------------------------------

/// Row 18: num < 0 — `driver` returns -1 before ever looking at the file, so
/// the filename (valid / missing / NULL) must make no difference.
#[test]
fn cfg_18_driver_negative() {
    let mut rng = Rng::new(18);
    let good = put_file("cfg18-good", b"line one\nline two\n");
    let gone = missing_path("cfg18");
    for i in 0..100 {
        let num = rng.range_i64(i32::MIN as i64, -1) as i32;
        diff_driver_path(num, &good, &format!("row18/good#{i}"));
        diff_driver_path(num, &gone, &format!("row18/missing#{i}"));
        diff_driver(num, None, &format!("row18/null#{i}"));
    }
}

/// Row 19: num >= 0 (no wrap) with a valid file — full success path,
/// `fclose(out)` taken, returns 0.
#[test]
fn cfg_19_driver_success() {
    let mut rng = Rng::new(19);
    for i in 0..300 {
        let num = rng.range_i64(0, (1i64 << 30) - 1) as i32;
        let lines = rng.range(0, 12);
        let trailing = rng.next_u64() & 1 == 0;
        let body = random_lines(&mut rng, lines, 150, trailing);
        let p = put_file("cfg19-body", &body);
        diff_driver_path(num, &p, &format!("row19/num={num}#{i}"));
    }
}

/// Row 20: num >= 2^30 — `x * 2` wraps negative, yet `driver` only compares
/// against -1, so it still prints the wrapped value and continues to the file.
#[test]
fn cfg_20_driver_overflow_success() {
    let mut rng = Rng::new(20);
    let p = put_file("cfg20-body", b"alpha\nbeta\n");
    for num in [1i32 << 30, (1i32 << 30) + 1, i32::MAX - 1, i32::MAX] {
        diff_driver_path(num, &p, "row20/boundary");
    }
    for i in 0..200 {
        let num = rng.range_i64(1i64 << 30, i32::MAX as i64) as i32;
        diff_driver_path(num, &p, &format!("row20/rand#{i}"));
    }
}

/// Row 21: num == 0 against several file shapes.
#[test]
fn cfg_21_driver_zero() {
    let empty = put_file("cfg21-empty", b"");
    let multi = put_file("cfg21-multi", b"a\nbb\nccc\n");
    diff_driver_path(0, &empty, "row21/empty");
    diff_driver(0, Some(b"/dev/null"), "row21/dev-null");
    diff_driver_path(0, &multi, "row21/multi");
}

/// Row 22: full randomized cross-product — `num` from the entire int domain
/// crossed with every file shape the C code distinguishes.
#[test]
fn cfg_22_driver_cross_product() {
    let mut rng = Rng::new(22);

    let dir = make_dir("cfg22");
    let gone = missing_path("cfg22");
    let empty = put_file("cfg22-empty", b"");
    let one = put_file("cfg22-one", b"single line\n");
    let multi = put_file("cfg22-multi", b"a\nbb\nccc\ndddd\n");
    let chunk = put_file("cfg22-chunk", &vec![b'Z'; 99]);
    let nul = put_file("cfg22-nul", b"aa\0bb\ncc\0\n");
    let bin = put_file("cfg22-bin", &rng.bytes(512));

    // Shapes: Some(path bytes) or None for a NULL pointer.
    let shapes: Vec<(&str, Option<Vec<u8>>)> = vec![
        ("dir", Some(dir.to_str().unwrap().as_bytes().to_vec())),
        ("missing", Some(gone.to_str().unwrap().as_bytes().to_vec())),
        ("empty", Some(empty.to_str().unwrap().as_bytes().to_vec())),
        ("one-line", Some(one.to_str().unwrap().as_bytes().to_vec())),
        ("multi-line", Some(multi.to_str().unwrap().as_bytes().to_vec())),
        ("chunk99", Some(chunk.to_str().unwrap().as_bytes().to_vec())),
        ("embedded-nul", Some(nul.to_str().unwrap().as_bytes().to_vec())),
        ("binary", Some(bin.to_str().unwrap().as_bytes().to_vec())),
        ("dev-null", Some(b"/dev/null".to_vec())),
        ("empty-name", Some(Vec::new())),
        ("NULL", None),
    ];

    // Deterministic sweep over every shape with representative `num` classes ...
    let nums: Vec<i32> = vec![i32::MIN, -1, 0, 1, 1 << 29, 1 << 30, i32::MAX];
    for (tag, shape) in &shapes {
        for &num in &nums {
            diff_driver(num, shape.as_deref(), &format!("row22/{tag}/num={num}"));
        }
    }

    // ... plus randomized draws from the FULL int domain.
    for i in 0..600 {
        let num = rng.i32_any();
        let (tag, shape) = &shapes[rng.range(0, shapes.len() - 1)];
        diff_driver(num, shape.as_deref(), &format!("row22/rand#{i}/{tag}/num={num}"));
    }
}

/// Row 23: hand-composed pipeline — call the two low-level functions in the
/// same order `driver` does, so the *interleaving* of the two output streams in
/// one capture window is compared, not just the per-function output.
#[test]
fn cfg_23_manual_pipeline() {
    let mut rng = Rng::new(23);
    let files: Vec<std::path::PathBuf> = vec![
        put_file("cfg23-a", b"one\ntwo\nthree\n"),
        put_file("cfg23-b", b""),
        put_file("cfg23-c", &vec![b'q'; 250]),
        make_dir("cfg23").clone(),
        missing_path("cfg23"),
    ];

    for i in 0..200 {
        let num = rng.i32_any();
        let path = &files[rng.range(0, files.len() - 1)];
        let name = path.to_str().unwrap().as_bytes();

        // Composite observation: run BOTH low-level calls inside ONE capture.
        let run = |fwd: FnForward, opn: FnOpen| -> (i32, i64, Vec<u8>, Vec<u8>) {
            let cname = std::ffi::CString::new(name).unwrap();
            let ((res, opened), out, err) = capture(|| unsafe {
                let res = fwd(num);
                if res == -1 {
                    (res, 0i64)
                } else {
                    let fp = opn(cname.as_ptr());
                    if fp.is_null() {
                        (res, 0i64)
                    } else {
                        // driver() does exactly this with the stream it gets.
                        close_stream(fp);
                        (res, 1i64)
                    }
                }
            });
            (res, opened, out, err)
        };

        let cfd = fd_count();
        let (cr, co, cout, cerr) = run(c_forward(), c_open());
        let cfd = fd_count() - cfd;
        let rfd = fd_count();
        let (rr, ro, rout, rerr) = run(r_forward(), r_open());
        let rfd = fd_count() - rfd;
        compare(
            &format!("row23 composed pipeline #{i} num={num} file={path:?}"),
            &Obs {
                ret: cr as i64,
                stream: Some((co, false, false, false)),
                stdout: cout,
                stderr: cerr,
                fd_delta: cfd,
            },
            &Obs {
                ret: rr as i64,
                stream: Some((ro, false, false, false)),
                stdout: rout,
                stderr: rerr,
                fd_delta: rfd,
            },
        );
    }
}

/// Row 24: many repeated calls on the same path within one process — catches
/// fd/stream leaks and buffering-state divergence that a single call misses.
#[test]
fn cfg_24_repeat_calls() {
    let p = put_file("cfg24-body", b"repeat me\nagain\n");
    for i in 0..200 {
        diff_open_path(&p, &format!("row24/#{i}"));
    }
    // And the same for the composed entry point.
    for i in 0..200 {
        diff_driver_path(7, &p, &format!("row24/driver#{i}"));
    }
}

/// Row 25: file permission bits.
///
/// `open_with_cleanup` opens with mode `"r"` — read-only. A file that is
/// readable but NOT writable (0o444 / 0o400) must therefore open successfully;
/// this is what pins the mode string down (`"r+"` would need write access and
/// fail with EACCES, while behaving identically on the 0o644 files used by the
/// other rows).
#[test]
fn cfg_25_permission_modes() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        eprintln!("skipping cfg_25: root bypasses permission bits");
        return;
    }
    let body: &[u8] = b"readable but not writable\nsecond line\n";
    for mode in [0o444u32, 0o400, 0o644, 0o604, 0o440, 0o666, 0o200, 0o000] {
        let p = put_file("cfg25-mode", body);
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
        diff_open_path(&p, &format!("row25/mode={mode:04o}"));
        diff_driver_path(3, &p, &format!("row25/driver mode={mode:04o}"));
        // Restore so put_file can overwrite it on the next iteration.
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

/// Row 26: descriptor accounting.
///
/// Neither the return value nor the printed bytes reveal whether the cleanup
/// block's `fclose(fp)` ran, so assert it directly: over many iterations of the
/// `ferror` path (a directory — `fopen` succeeds, `fgets` fails) the number of
/// open descriptors must not grow, and must grow identically for C and Rust.
#[test]
fn cfg_26_no_fd_leak() {
    let dir = make_dir("cfg26");
    let file = put_file("cfg26-body", b"data\ndata\n");
    let gone = missing_path("cfg26");

    // `diff_*` already compares the per-call fd delta; also check the absolute
    // growth over a long run, which is where a 1-fd-per-call leak becomes loud.
    let baseline = fd_count();
    for i in 0..150 {
        diff_open_path(&dir, &format!("row26/ferror#{i}"));
        diff_open_path(&file, &format!("row26/ok#{i}"));
        diff_open_path(&gone, &format!("row26/missing#{i}"));
        diff_driver_path(1, &dir, &format!("row26/driver-ferror#{i}"));
        diff_driver_path(1, &file, &format!("row26/driver-ok#{i}"));
    }
    let grown = fd_count() - baseline;
    assert_eq!(
        grown, 0,
        "open descriptors grew by {grown} over 750 calls — a stream is not being \
         fclose()d (C baseline={baseline}, now={})",
        fd_count()
    );
}

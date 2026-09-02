//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every test drives both shared objects through `libloading` only, starting at
//! the lowest-level entry point (`forward_goto_example`), then the low-level
//! `open_with_cleanup`, then the composed `driver` pipeline. Rows marked
//! *randomized* in `CONFIGS.md` use the fixed-seed PRNG in `common`.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// C1–C4: forward_goto_example (lowest-level entry point)
// ---------------------------------------------------------------------------

/// C1 — non-negative `x`, success path, no overflow.
#[test]
fn cfg_c1_fwd_nonneg() {
    for x in 0..=1000 {
        let v = diff_forward(x);
        assert_eq!(v, x * 2, "C1: expected the C's 2x for x={x}");
    }

    let mut rng = Rng::new(1);
    for _ in 0..512 {
        let x = rng.in_range_i32(0, i32::MAX / 2);
        let v = diff_forward(x);
        assert_eq!(v, x * 2, "C1: expected 2x for x={x}");
    }
}

/// C2 — negative `x`, error path taken via `goto error`.
#[test]
fn cfg_c2_fwd_negative() {
    let mut rng = Rng::new(2);
    for _ in 0..512 {
        let x = rng.in_range_i32(i32::MIN, -1);
        let v = diff_forward(x);
        assert_eq!(v, -1, "C2: negative x={x} must yield -1");
    }
}

/// C3 — the signed-overflow boundary of `x * 2`.
#[test]
fn cfg_c3_fwd_overflow() {
    let fixed = [
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX - 1,
        i32::MAX,
        1 << 30,
        (1 << 30) + 1,
        0x7FFF_FFFE,
    ];
    for x in fixed {
        let v = diff_forward(x);
        assert_eq!(v, x.wrapping_mul(2), "C3: wrapping 2x for x={x}");
    }

    let mut rng = Rng::new(3);
    for _ in 0..256 {
        let x = rng.in_range_i32(i32::MAX / 2, i32::MAX);
        let v = diff_forward(x);
        assert_eq!(v, x.wrapping_mul(2), "C3: wrapping 2x for x={x}");
    }
}

/// C4 — unconstrained `int32` sweep, mixing both paths in one stream.
#[test]
fn cfg_c4_fwd_full_sweep() {
    let mut rng = Rng::new(4);
    for _ in 0..2048 {
        let x = rng.next_u32() as i32;
        let v = diff_forward(x);
        let expected = if x < 0 { -1 } else { x.wrapping_mul(2) };
        assert_eq!(v, expected, "C4: x={x}");
    }
}

// ---------------------------------------------------------------------------
// C5–C15: open_with_cleanup (low-level entry point, driven directly)
// ---------------------------------------------------------------------------

/// C5 — zero-byte file: the `fgets` loop body never executes.
#[test]
fn cfg_c5_empty_file() {
    let f = TempPath::file(b"");
    let st = diff_open(Some(&f.bytes()));
    assert!(!st.null, "C5: empty file opens successfully");
    assert_eq!(st.tell, 0, "C5: nothing was consumed");
}

/// C6 — one short line terminated by a newline.
#[test]
fn cfg_c6_single_line_nl() {
    let f = TempPath::file(b"hello world\n");
    let st = diff_open(Some(&f.bytes()));
    assert!(!st.null, "C6: file opens successfully");
    assert_eq!(st.tell, 12, "C6: whole file consumed");
}

/// C7 — one short line with **no** trailing newline.
#[test]
fn cfg_c7_single_line_no_nl() {
    for content in [&b"no trailing newline"[..], b"x", b"\n", b"a\nb"] {
        let f = TempPath::file(content);
        let st = diff_open(Some(&f.bytes()));
        assert!(!st.null, "C7: {content:?} opens successfully");
    }
}

/// C8 — many lines, randomized count and per-line length (including empties).
#[test]
fn cfg_c8_many_lines() {
    let mut rng = Rng::new(8);
    for case in 0..64 {
        let lines = 2 + rng.below(49) as usize;
        let mut content = Vec::new();
        for _ in 0..lines {
            let len = rng.below(251) as usize;
            for _ in 0..len {
                // printable ASCII except '\n' so line structure stays controlled
                content.push(0x20 + (rng.below(0x5F) as u8));
            }
            content.push(b'\n');
        }
        // Half the cases drop the final newline.
        if case % 2 == 1 {
            content.pop();
        }
        let f = TempPath::file(&content);
        let st = diff_open(Some(&f.bytes()));
        assert!(!st.null, "C8 case {case}: opens successfully");
    }
}

/// C9 — line lengths straddling the 100-byte `fgets` buffer.
#[test]
fn cfg_c9_buffer_boundaries() {
    for len in [0usize, 1, 97, 98, 99, 100, 101, 102, 197, 198, 199, 200, 201, 299, 300] {
        for trailing_nl in [true, false] {
            let mut content = vec![b'A'; len];
            if trailing_nl {
                content.push(b'\n');
            }
            let f = TempPath::file(&content);
            let st = diff_open(Some(&f.bytes()));
            assert!(!st.null, "C9: len={len} nl={trailing_nl} opens successfully");
        }
    }

    // Newlines placed exactly at the chunk seams.
    for pos in [98usize, 99, 100, 101, 198, 199, 200] {
        let mut content = vec![b'B'; 300];
        content[pos] = b'\n';
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "C9: seam newline at {pos}");
    }
}

/// C10 — a single line far longer than the buffer: hundreds of `fgets` calls.
#[test]
fn cfg_c10_huge_line() {
    let mut rng = Rng::new(10);
    for _ in 0..8 {
        let len = 1024 + rng.below(64 * 1024) as usize;
        let content: Vec<u8> = (0..len).map(|_| 0x21 + (rng.below(0x5E) as u8)).collect();
        let f = TempPath::file(&content);
        let st = diff_open(Some(&f.bytes()));
        assert!(!st.null, "C10: len={len} opens successfully");
        assert_eq!(st.tell, len as i64, "C10: whole file consumed");
    }
}

/// C11 — randomized binary content with no NUL bytes (`%s` is lossless).
#[test]
fn cfg_c11_binary_no_nul() {
    let mut rng = Rng::new(11);
    for _ in 0..48 {
        let len = rng.below(8 * 1024) as usize;
        let content: Vec<u8> = (0..len)
            .map(|_| {
                let mut b = rng.byte();
                if b == 0 {
                    b = 1;
                }
                b
            })
            .collect();
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "C11: len={len}");
    }
}

/// C12 — randomized binary content **including** NUL bytes, so `printf("%s")`
/// truncates each 99-byte chunk at its first NUL. Both must truncate the same.
#[test]
fn cfg_c12_binary_with_nul() {
    let mut rng = Rng::new(12);
    for _ in 0..48 {
        let len = 1 + rng.below(4 * 1024) as usize;
        let content: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "C12: len={len}");
    }

    // NULs placed deliberately at and around the chunk seams.
    for pos in [0usize, 1, 98, 99, 100, 197, 198, 199] {
        let mut content = vec![b'C'; 400];
        content[pos] = 0;
        let f = TempPath::file(&content);
        assert!(!diff_open(Some(&f.bytes())).null, "C12: NUL at {pos}");
    }
}

/// C13 — content that looks like `printf` conversions must be emitted literally.
#[test]
fn cfg_c13_format_like_content() {
    let payloads: [&[u8]; 6] = [
        b"%s %d %n %%\n",
        b"%s%s%s%s%s%s%s%s%s%s\n",
        b"100%% sure\n%p %x %hn\n",
        b"%n",
        b"%",
        b"%*d %.*f %099d\n",
    ];
    for p in payloads {
        let f = TempPath::file(p);
        assert!(
            !diff_open(Some(&f.bytes())).null,
            "C13: {:?}",
            String::from_utf8_lossy(p)
        );
    }
}

/// C14 — the success path returns a handle that is still open at EOF; compare
/// its observable state between C and Rust.
#[test]
fn cfg_c14_returned_handle_state() {
    let mut rng = Rng::new(14);
    for _ in 0..24 {
        let len = rng.below(2000) as usize;
        let content: Vec<u8> = (0..len).map(|_| b'a' + (rng.below(26) as u8)).collect();
        let f = TempPath::file(&content);
        // diff_open already asserts the full HandleState matches; additionally
        // pin down what that state must be, so a "both return NULL" false pass
        // cannot hide here.
        let st = diff_open(Some(&f.bytes()));
        assert!(!st.null, "C14: handle is non-NULL on success");
        assert!(st.fd_valid, "C14: handle still open (fileno valid)");
        assert!(st.eof_set, "C14: file was read to EOF");
        assert!(!st.error_set, "C14: no error flag on the success path");
        assert_eq!(st.tell, len as i64, "C14: positioned at end of file");
    }
}

/// C15 — filename shapes that are valid but unusual.
#[test]
fn cfg_c15_filename_shapes() {
    for tag in ["plain", "with space", "pct%s%d", "dash-and_under", "dot.dot.dot"] {
        let f = TempPath::file_named(tag, b"content\n");
        assert!(!diff_open(Some(&f.bytes())).null, "C15: tag={tag}");
    }

    // A long-but-valid component (well under NAME_MAX = 255).
    let long_tag = "l".repeat(180);
    let f = TempPath::file_named(&long_tag, b"long name\n");
    assert!(!diff_open(Some(&f.bytes())).null, "C15: 180-char name component");

    // Nested directories in the path.
    let d = TempPath::dir();
    let nested = d.path.join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("deep.txt");
    std::fs::write(&file, b"deep\n").unwrap();
    assert!(
        !diff_open(Some(file.as_os_str().as_encoded_bytes())).null,
        "C15: nested path"
    );
}

// ---------------------------------------------------------------------------
// C16–C20: driver (the composed pipeline over both lower-level functions)
// ---------------------------------------------------------------------------

/// C16 — full success pipeline, randomized over `num` × content shape.
#[test]
fn cfg_c16_driver_success() {
    let mut rng = Rng::new(16);
    for case in 0..96 {
        let num = rng.in_range_i32(0, i32::MAX / 2);
        let content: Vec<u8> = match case % 6 {
            0 => Vec::new(),
            1 => b"one line\n".to_vec(),
            2 => b"no newline".to_vec(),
            3 => vec![b'X'; 99 + (rng.below(5) as usize)],
            4 => (0..rng.below(3000)).map(|_| b'a' + (rng.below(26) as u8)).collect(),
            _ => (0..rng.below(500)).map(|_| rng.byte()).collect(),
        };
        let f = TempPath::file(&content);
        let rc = diff_driver(num, Some(&f.bytes()));
        assert_eq!(rc, 0, "C16 case {case}: num={num} must succeed");
    }
}

/// C17 — `num` whose doubled value overflows: `Goto output:` prints a negative.
#[test]
fn cfg_c17_driver_overflow_num() {
    let f = TempPath::file(b"payload\n");
    let name = f.bytes();
    for num in [i32::MAX / 2 + 1, 1 << 30, i32::MAX - 1, i32::MAX] {
        let rc = diff_driver(num, Some(&name));
        assert_eq!(rc, 0, "C17: num={num} still succeeds (result is not -1)");
    }

    let mut rng = Rng::new(17);
    for _ in 0..64 {
        let num = rng.in_range_i32(i32::MAX / 2 + 1, i32::MAX);
        assert_eq!(diff_driver(num, Some(&name)), 0, "C17: num={num}");
    }
}

/// C18 — `num < 0` short-circuits before `filename` is ever used.
#[test]
fn cfg_c18_driver_short_circuit() {
    let f = TempPath::file(b"never read\n");
    let good = f.bytes();
    let mut rng = Rng::new(18);
    for _ in 0..128 {
        let num = rng.in_range_i32(i32::MIN, -1);
        // Valid file: must still be ignored.
        assert_eq!(diff_driver(num, Some(&good)), -1, "C18: num={num} valid file");
        // Missing file: same result, proving the file is never touched.
        let missing = missing_path();
        assert_eq!(
            diff_driver(num, Some(missing.as_os_str().as_encoded_bytes())),
            -1,
            "C18: num={num} missing file"
        );
    }
}

/// C19 — valid `num`, unopenable file: the error leaf inside a valid pipeline.
#[test]
fn cfg_c19_driver_file_missing() {
    let mut rng = Rng::new(19);
    for _ in 0..32 {
        let num = rng.in_range_i32(0, i32::MAX);
        let missing = missing_path();
        let rc = diff_driver(num, Some(missing.as_os_str().as_encoded_bytes()));
        assert_eq!(rc, -2, "C19: num={num} missing file must give -2");
    }
}

/// C20 — valid `num`, path is a directory: the `ferror` leaf inside the pipeline.
#[test]
fn cfg_c20_driver_directory() {
    let d = TempPath::dir();
    let name = d.bytes();
    for num in [0, 1, 7, 12345, i32::MAX] {
        assert_eq!(diff_driver(num, Some(&name)), -2, "C20: num={num} directory");
    }
}

// ---------------------------------------------------------------------------
// C21–C22: cross-function stream ordering and state
// ---------------------------------------------------------------------------

/// C23 — special files: sources whose `stat` size is 0 but which yield data,
/// and sources where `ftell` behaves unusually. These reach the `fgets` loop
/// through a different kernel read path than a regular file.
#[test]
fn cfg_c23_special_files() {
    // Always present, always empty.
    assert!(!diff_open(Some(b"/dev/null")).null, "C23: /dev/null");

    // procfs: reports st_size == 0 yet returns bytes, and its content is stable
    // for the lifetime of the machine/process.
    for p in ["/proc/version", "/proc/self/cmdline", "/proc/uptime"] {
        if std::path::Path::new(p).exists() {
            // /proc/uptime changes between reads, so only assert non-NULL there;
            // diff_open would compare two different snapshots.
            if p == "/proc/uptime" {
                continue;
            }
            assert!(!diff_open(Some(p.as_bytes())).null, "C23: {p}");
        }
    }

    for p in ["/etc/os-release", "/etc/hostname"] {
        if std::path::Path::new(p).exists() {
            assert!(!diff_open(Some(p.as_bytes())).null, "C23: {p}");
        }
    }

    // A symlink pointing at a regular file.
    let d = TempPath::dir();
    let target = d.path.join("target.txt");
    std::fs::write(&target, b"through a symlink\n").unwrap();
    let link = d.path.join("link");
    if std::os::unix::fs::symlink(&target, &link).is_ok() {
        assert!(
            !diff_open(Some(link.as_os_str().as_encoded_bytes())).null,
            "C23: symlink to regular file"
        );
    }

    // Through the composed pipeline too.
    assert_eq!(diff_driver(9, Some(b"/dev/null")), 0, "C23: driver on /dev/null");
    if std::path::Path::new("/proc/version").exists() {
        assert_eq!(
            diff_driver(9, Some(b"/proc/version")),
            0,
            "C23: driver on /proc/version"
        );
    }
}

/// line-buffered stdout against unbuffered stderr is compared too.
#[test]
fn cfg_c21_interleaved_streams() {
    let f = TempPath::file(b"line A\nline B\n");
    let good = f.bytes();
    let missing = missing_path();
    let bad = missing.as_os_str().as_encoded_bytes().to_vec();
    let d = TempPath::dir();
    let dir = d.bytes();

    let script = |api: &Api| {
        let good = std::ffi::CString::new(good.clone()).unwrap();
        let bad = std::ffi::CString::new(bad.clone()).unwrap();
        let dir = std::ffi::CString::new(dir.clone()).unwrap();
        let mut codes = Vec::new();
        unsafe {
            codes.push((api.forward_goto_example)(5));
            codes.push((api.forward_goto_example)(-5));
            codes.push((api.driver)(3, good.as_ptr()));
            codes.push((api.driver)(-3, good.as_ptr()));
            codes.push((api.driver)(4, bad.as_ptr()));
            codes.push((api.driver)(4, dir.as_ptr()));
            let fp = (api.open_with_cleanup)(good.as_ptr());
            codes.push(i32::from(!fp.is_null()));
            if !fp.is_null() {
                // Close through the same libc both objects share.
                unsafe extern "C" {
                    fn fclose(s: *mut std::ffi::c_void) -> i32;
                }
                fclose(fp);
            }
            codes.push((api.open_with_cleanup)(bad.as_ptr()).is_null() as i32);
        }
        codes
    };

    let (c, r) = both();
    let (c_codes, c_cap) = capture_merged(|| script(c));
    let (r_codes, r_cap) = capture_merged(|| script(r));
    assert_eq!(c_codes, r_codes, "C21: return codes differ");
    assert_eq!(
        c_cap.out, r_cap.out,
        "C21: merged stdout+stderr byte stream differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_cap.out),
        String::from_utf8_lossy(&r_cap.out)
    );
    assert!(!c_cap.out.is_empty(), "C21: the script must actually produce output");
}

/// C22 — a long randomized script of interleaved calls to all three entry
/// points, compared as one accumulated output stream.
#[test]
fn cfg_c22_mixed_call_sequence() {
    // Build the input corpus once so both runs see identical files.
    let files: Vec<TempPath> = vec![
        TempPath::file(b""),
        TempPath::file(b"a\n"),
        TempPath::file(b"no nl"),
        TempPath::file(&vec![b'Z'; 250]),
        TempPath::file(b"\0hidden\n"),
        TempPath::dir(),
    ];
    let missing = missing_path();

    let mut names: Vec<Vec<u8>> = files.iter().map(|f| f.bytes()).collect();
    names.push(missing.as_os_str().as_encoded_bytes().to_vec());

    // Pre-generate the call script so both objects replay exactly the same one.
    #[derive(Clone, Copy)]
    enum Call {
        Fwd(i32),
        Open(usize),
        Drv(i32, usize),
        DrvNull(i32),
        OpenNull,
    }
    let mut rng = Rng::new(22);
    let script: Vec<Call> = (0..600)
        .map(|_| match rng.below(5) {
            0 => Call::Fwd(rng.next_u32() as i32),
            1 => Call::Open(rng.below(names.len() as u64) as usize),
            2 => Call::Drv(rng.next_u32() as i32, rng.below(names.len() as u64) as usize),
            3 => Call::DrvNull(rng.next_u32() as i32),
            _ => Call::OpenNull,
        })
        .collect();

    let cnames: Vec<std::ffi::CString> = names
        .iter()
        .map(|n| std::ffi::CString::new(n.clone()).unwrap())
        .collect();

    let run = |api: &Api| {
        let mut codes: Vec<i64> = Vec::with_capacity(script.len());
        unsafe extern "C" {
            fn fclose(s: *mut std::ffi::c_void) -> i32;
        }
        for call in &script {
            unsafe {
                match *call {
                    Call::Fwd(x) => codes.push((api.forward_goto_example)(x) as i64),
                    Call::Open(i) => {
                        let fp = (api.open_with_cleanup)(cnames[i].as_ptr());
                        codes.push(fp.is_null() as i64);
                        if !fp.is_null() {
                            fclose(fp);
                        }
                    }
                    Call::OpenNull => {
                        let fp = (api.open_with_cleanup)(std::ptr::null());
                        codes.push(fp.is_null() as i64);
                        if !fp.is_null() {
                            fclose(fp);
                        }
                    }
                    Call::Drv(n, i) => codes.push((api.driver)(n, cnames[i].as_ptr()) as i64),
                    Call::DrvNull(n) => codes.push((api.driver)(n, std::ptr::null()) as i64),
                }
            }
        }
        codes
    };

    let (c, r) = both();
    let (cc, ccap) = capture(|| run(c));
    let (rc, rcap) = capture(|| run(r));
    assert_eq!(cc, rc, "C22: return-code sequence differs");
    assert_eq!(
        ccap.out, rcap.out,
        "C22: accumulated stdout differs ({} vs {} bytes)",
        ccap.out.len(),
        rcap.out.len()
    );
    assert_eq!(
        ccap.err, rcap.err,
        "C22: accumulated stderr differs ({} vs {} bytes)",
        ccap.err.len(),
        rcap.err.len()
    );
    assert!(!ccap.out.is_empty() && !ccap.err.is_empty(), "C22: script must exercise both streams");
}

//! Phase B (valid paths) + Phase C (error paths) differential tests driven
//! through the process interface, which is how an external caller uses this
//! program: `c_src/CMakeLists.txt` builds an executable whose entire API is
//! `main` reading stdin and writing stdout.
//!
//! Every test runs the **C build** and the **Rust build** under identical
//! conditions and requires identical (exit status, stdout bytes, stderr bytes).
//! Row numbers refer to `CONFIGS.md` (cfg_*) and `ERRORS.md` (err_*).

mod common;

use common::*;
use std::path::Path;

// ===========================================================================
// Phase B -- CONFIGS.md
// ===========================================================================

/// Row 1: empty input, the loop never runs.
#[test]
fn cfg_01_empty() {
    assert_same("cfg_01_empty", b"");
}

/// Row 2: a lone newline, the shortest successful chunk.
#[test]
fn cfg_02_single_newline() {
    assert_same("cfg_02_single_newline", b"\n");
    assert_same("cfg_02_two_newlines", b"\n\n");
    assert_same("cfg_02_many_newlines", &vec![b'\n'; 300]);
}

/// Row 3: one newline-terminated line.
#[test]
fn cfg_03_one_line_nl() {
    assert_same("cfg_03", b"hello\n");
    assert_same("cfg_03_space", b" \n");
    assert_same("cfg_03_long_word", b"abcdefghijklmnopqrstuvwxyz\n");
}

/// Row 4: one line with no trailing newline (chunk ends at EOF).
#[test]
fn cfg_04_one_line_no_nl() {
    assert_same("cfg_04", b"hello");
    assert_same("cfg_04_one_byte", b"x");
}

/// Row 5: many short newline-terminated lines, total below the 4096 buffer.
#[test]
fn cfg_05_many_short_lines() {
    let mut v = Vec::new();
    for i in 0..200 {
        v.extend_from_slice(format!("line {}\n", i).as_bytes());
    }
    assert_same("cfg_05_many_short_lines", &v);
}

/// Row 6: randomized line lengths (0..300) and a randomized trailing newline.
#[test]
fn cfg_06_random_line_lengths() {
    let mut rng = Rng::new(SEED ^ 6);
    for case in 0..120 {
        let lines = rng.below(12) as usize;
        let mut v = Vec::new();
        for _ in 0..lines {
            let len = rng.below(301) as usize;
            for _ in 0..len {
                // printable, so a failure is easy to read
                v.push(b'a' + (rng.below(26) as u8));
            }
            if rng.bool_pct(85) {
                v.push(b'\n');
            }
        }
        assert_same(&format!("cfg_06_random_line_lengths[case={}]", case), &v);
    }
}

/// Row 7: 126 bytes + newline == exactly 127, one full chunk.
#[test]
fn cfg_07_len126_plus_nl() {
    let mut v = vec![b'a'; 126];
    v.push(b'\n');
    assert_same("cfg_07_len126_plus_nl", &v);
}

/// Row 8: 127 bytes + newline -> chunk 1 stops at the 127 limit, chunk 2 is "\n".
#[test]
fn cfg_08_len127_plus_nl() {
    let mut v = vec![b'a'; 127];
    v.push(b'\n');
    assert_same("cfg_08_len127_plus_nl", &v);
}

/// Row 9: exactly 127 bytes, no newline, then EOF.
#[test]
fn cfg_09_len127_no_nl() {
    assert_same("cfg_09_len127_no_nl", &vec![b'a'; 127]);
}

/// Row 10: exactly 128 bytes, no newline -> 127 + 1.
#[test]
fn cfg_10_len128_no_nl() {
    assert_same("cfg_10_len128_no_nl", &vec![b'a'; 128]);
}

/// Row 11: the whole neighbourhood of the 127-byte chunk boundary.
#[test]
fn cfg_11_boundary_sweep() {
    for len in (120..=136).chain(250..=260).chain([1, 2, 3, 253, 254, 255, 381, 382]) {
        for nl in [false, true] {
            let mut v = vec![b'q'; len];
            if nl {
                v.push(b'\n');
            }
            assert_same(
                &format!("cfg_11_boundary_sweep[len={},nl={}]", len, nl),
                &v,
            );
        }
    }
}

/// Row 12: a NUL in the middle of a chunk truncates that chunk (`fputs`).
#[test]
fn cfg_12_nul_middle() {
    assert_same("cfg_12_nul_middle", b"ab\x00cd\nxy\n");
    assert_same("cfg_12_nul_middle_no_nl", b"ab\x00cd");
    assert_same("cfg_12_nul_two", b"a\x00b\x00c\nnext\n");
}

/// Row 13: a NUL as the first byte of a chunk emits nothing for that chunk.
#[test]
fn cfg_13_nul_leading() {
    assert_same("cfg_13_nul_leading", b"\x00abc\ndef\n");
    assert_same("cfg_13_nul_leading_only", b"\x00\n");
}

/// Row 14: a NUL as the last byte before the newline.
#[test]
fn cfg_14_nul_before_newline() {
    assert_same("cfg_14_nul_before_newline", b"abc\x00\ndef\n");
}

/// Row 15: NUL exactly at / next to the 127-byte chunk boundary.
#[test]
fn cfg_15_nul_at_boundary() {
    for nul_at in 120..=134 {
        let mut v = vec![b'a'; nul_at];
        v.push(0);
        v.extend_from_slice(b"tail\n");
        v.extend_from_slice(b"second line\n");
        assert_same(&format!("cfg_15_nul_at_boundary[nul_at={}]", nul_at), &v);
    }
}

/// Row 16: input made only of NUL bytes -> non-empty input, empty output.
#[test]
fn cfg_16_all_nuls() {
    for n in [1usize, 2, 126, 127, 128, 129, 300] {
        assert_same(&format!("cfg_16_all_nuls[n={}]", n), &vec![0u8; n]);
    }
}

/// Row 17: randomized inputs with NULs at random positions and densities.
#[test]
fn cfg_17_random_with_nuls() {
    let mut rng = Rng::new(SEED ^ 17);
    for case in 0..200 {
        let len = rng.below(600) as usize;
        let nul_pct = rng.below(40) + 1;
        let nl_pct = rng.below(20) + 1;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            if rng.bool_pct(nul_pct) {
                v.push(0);
            } else if rng.bool_pct(nl_pct) {
                v.push(b'\n');
            } else {
                v.push(b'A' + (rng.below(26) as u8));
            }
        }
        assert_same(&format!("cfg_17_random_with_nuls[case={}]", case), &v);
    }
}

/// Row 18: CRLF line endings pass through untouched.
#[test]
fn cfg_18_crlf() {
    assert_same("cfg_18_crlf", b"line1\r\nline2\r\n");
    assert_same("cfg_18_cr_only", b"line1\rline2\r");
    assert_same("cfg_18_lone_cr_at_boundary", &{
        let mut v = vec![b'z'; 126];
        v.extend_from_slice(b"\r\n");
        v
    });
}

/// Row 19: all 256 byte values in order.
#[test]
fn cfg_19_all_byte_values() {
    let all: Vec<u8> = (0..=255u8).collect();
    assert_same("cfg_19_all_byte_values", &all);
    // and again, repeated so it crosses several chunk boundaries
    let mut rep = Vec::new();
    for _ in 0..10 {
        rep.extend_from_slice(&all);
    }
    assert_same("cfg_19_all_byte_values_x10", &rep);
}

/// Row 20: invalid UTF-8 must survive byte for byte.
#[test]
fn cfg_20_invalid_utf8() {
    assert_same("cfg_20_invalid_utf8", b"\xff\xfe\xc3\x28\n\x80\x81\n");
    assert_same("cfg_20_lone_continuation", b"\x80\x80\x80\n");
    assert_same("cfg_20_truncated_seq", b"\xe2\x82\n");
    assert_same("cfg_20_valid_utf8", "héllo wörld ✓\n".as_bytes());
}

/// Row 21: uniformly random binary input.
#[test]
fn cfg_21_random_binary() {
    let mut rng = Rng::new(SEED ^ 21);
    for case in 0..150 {
        let len = rng.below(900) as usize;
        let v: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_same(&format!("cfg_21_random_binary[case={}]", case), &v);
    }
}

/// Row 22: input length right at the stdout block-buffer boundary.
#[test]
fn cfg_22_stdio_buffer_boundary() {
    for len in [4094usize, 4095, 4096, 4097, 4098, 8191, 8192, 8193] {
        // newline-terminated lines so chunking is exercised too
        let mut v = Vec::with_capacity(len);
        while v.len() < len {
            let remaining = len - v.len();
            let piece = remaining.min(50);
            v.extend(std::iter::repeat(b'k').take(piece - 1));
            if v.len() < len {
                v.push(b'\n');
            }
        }
        v.truncate(len);
        assert_same(&format!("cfg_22_stdio_buffer_boundary[len={}]", len), &v);
    }
}

/// Row 23: input spanning many buffer blocks (~1 MB).
#[test]
fn cfg_23_multi_block_large() {
    let mut v = Vec::with_capacity(1_000_000);
    let mut rng = Rng::new(SEED ^ 23);
    while v.len() < 1_000_000 {
        let len = rng.below(200) as usize;
        for _ in 0..len {
            v.push(b'a' + (rng.below(26) as u8));
        }
        v.push(b'\n');
    }
    assert_same("cfg_23_multi_block_large", &v);

    // and a 1 MB single "line" with no newline at all (pure 127-byte chunking)
    assert_same("cfg_23_no_newline_1mb", &vec![b'w'; 1_000_000]);
}

/// Row 24: stdout is a pipe -> glibc block buffers, so *nothing* may be visible
/// before 4096 bytes have accumulated. This is a timing-observable property of
/// the stream's buffering mode, not just of the final byte stream.
#[test]
fn cfg_24_stdout_pipe_block_buffered() {
    // one line only: far below 4096 -> both must show nothing before EOF
    let c = visible_before_eof(&c_exe(), b"hello\n", 700, false);
    let r = visible_before_eof(&rust_exe(), b"hello\n", 700, false);
    assert_eq!(
        c, r,
        "stdout=pipe, one short line: bytes visible before EOF differ (C={}, Rust={})",
        c, r
    );
    assert_eq!(c, 0, "sanity: C must withhold a short line on a pipe");

    // just over one block -> both must show exactly one 4096-byte block
    let input: Vec<u8> = std::iter::repeat(b"y".repeat(99))
        .take(45)
        .flat_map(|mut l| {
            l.push(b'\n');
            l
        })
        .collect();
    let c = visible_before_eof(&c_exe(), &input, 700, false);
    let r = visible_before_eof(&rust_exe(), &input, 700, false);
    assert_eq!(
        c, r,
        "stdout=pipe, {} bytes fed: bytes visible before EOF differ (C={}, Rust={})",
        input.len(),
        c,
        r
    );
    assert_eq!(c, 4096, "sanity: C flushes one 4096-byte block");
}

/// Row 25 (+ row 27): stdout is a regular file, stdin is a regular file.
#[test]
fn cfg_25_stdout_regular_file() {
    // run_file_io already uses regular files for both ends; assert on content
    let mut rng = Rng::new(SEED ^ 25);
    for case in 0..40 {
        let len = rng.below(9000) as usize;
        let v: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let c = run_file_io(&c_exe(), &v, &[]);
        let r = run_file_io(&rust_exe(), &v, &[]);
        assert_eq!(c, r, "cfg_25 case {} diverged", case);
        // cross-check the C behaviour against the model of fgets+fputs
        assert_eq!(
            c.stdout,
            model(&v),
            "cfg_25 case {}: C output does not match the fgets/fputs model",
            case
        );
    }
}

/// Row 26: stdout is a tty -> glibc line buffers, so each line appears at once.
#[test]
fn cfg_26_stdout_tty_line_buffered() {
    let c = visible_before_eof(&c_exe(), b"hello\n", 700, true);
    let r = visible_before_eof(&rust_exe(), b"hello\n", 700, true);
    assert_eq!(
        c, r,
        "stdout=tty, one short line: bytes visible before EOF differ (C={}, Rust={})",
        c, r
    );
    assert!(
        c >= 6,
        "sanity: C must line-buffer to a tty and emit the line immediately (got {})",
        c
    );
}

/// Row 28: stdin is a pipe fed incrementally; the writer closes late.
#[test]
fn cfg_28_stdin_pipe_incremental() {
    let pieces: Vec<&[u8]> = vec![
        b"first",
        b" part\n",
        b"second\n",
        &[b'x'; 200],
        b"\n",
        b"\x00hidden\n",
        b"tail-no-newline",
    ];
    let c = run_stdin_pipe_incremental(&c_exe(), &pieces);
    let r = run_stdin_pipe_incremental(&rust_exe(), &pieces);
    assert_eq!(c, r, "cfg_28_stdin_pipe_incremental diverged");
}

/// Row 29: stdin is a tty (the pty master closes to signal EOF).
#[test]
fn cfg_29_stdin_tty() {
    let c = run_with_tty_stdin(&c_exe(), b"abc\ndef\n");
    let r = run_with_tty_stdin(&rust_exe(), b"abc\ndef\n");
    assert_eq!(c, r, "cfg_29_stdin_tty diverged");
}

/// Row 30: stdin and stdout are the same tty -- the "interactive echo" mode
/// described by the C comment: type a line, see it echoed straight away.
#[test]
fn cfg_30_interactive_tty_both() {
    let c = interactive_tty_roundtrip(&c_exe());
    let r = interactive_tty_roundtrip(&rust_exe());
    assert_eq!(
        c, r,
        "cfg_30 interactive tty echo diverged (C={:?}, Rust={:?})",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert!(
        !c.is_empty(),
        "sanity: the C build must echo interactively on a tty"
    );
}

/// Row 31: command-line arguments are ignored (`int main()` takes none).
#[test]
fn cfg_31_args_ignored() {
    let input = b"alpha\nbeta\n";
    for args in [
        vec![],
        vec!["-h"],
        vec!["--help"],
        vec!["/nonexistent/file"],
        vec!["a", "b", "c"],
        vec![""],
        vec!["-"],
    ] {
        assert_same_args(
            &format!("cfg_31_args_ignored[{:?}]", args),
            input,
            &args,
        );
    }
}

// ===========================================================================
// Phase C -- ERRORS.md
// ===========================================================================

/// Error row 1: stdin already at EOF on the first `fgets`.
#[test]
fn err_01_empty_stdin_immediate_eof() {
    let c = run_file_io(&c_exe(), b"", &[]);
    let r = run_file_io(&rust_exe(), b"", &[]);
    assert_eq!(c, r, "err_01 diverged");
    assert_eq!(c.status, Ok(0), "C must exit 0");
    assert!(c.stdout.is_empty() && c.stderr.is_empty());
}

/// Error row 2: fd 0 closed -> `read` fails EBADF -> `fgets` returns NULL.
#[test]
fn err_02_stdin_closed_ebadf() {
    let c = run_with_closed_fd(&c_exe(), b"ignored\n", 0);
    let r = run_with_closed_fd(&rust_exe(), b"ignored\n", 0);
    assert_eq!(c, r, "err_02 diverged (closed stdin)");
    assert_eq!(c.status, Ok(0), "C exits 0 even with stdin closed");
    assert!(c.stdout.is_empty(), "no output with stdin closed");
}

/// Error row 3: stdin is a directory -> `read` fails EISDIR.
#[test]
fn err_03_stdin_is_directory_eisdir() {
    let dir = crate_root();
    let c = run_stdin_from_path(&c_exe(), &dir);
    let r = run_stdin_from_path(&rust_exe(), &dir);
    assert_eq!(c, r, "err_03 diverged (stdin is a directory)");
    assert_eq!(c.status, Ok(0));
    assert!(c.stdout.is_empty());
}

/// Error row 4: stdin is /dev/null.
#[test]
fn err_04_stdin_dev_null() {
    let p = Path::new("/dev/null");
    let c = run_stdin_from_path(&c_exe(), p);
    let r = run_stdin_from_path(&rust_exe(), p);
    assert_eq!(c, r, "err_04 diverged (/dev/null stdin)");
    assert_eq!(c.status, Ok(0));
    assert!(c.stdout.is_empty());
}

/// Error row 5: EOF after >= 1 byte with no trailing newline -> partial chunk is
/// returned by `fgets`, not rejected, and no newline is invented.
#[test]
fn err_05_eof_without_trailing_newline() {
    for input in [
        &b"x"[..],
        &b"abc"[..],
        &b"line1\nline2"[..],
        &vec![b'p'; 127][..],
        &vec![b'p'; 128][..],
    ] {
        let c = run_file_io(&c_exe(), input, &[]);
        let r = run_file_io(&rust_exe(), input, &[]);
        assert_eq!(c, r, "err_05 diverged for {:?}", Preview(input));
        assert_eq!(&c.stdout, input, "C echoes the partial last line verbatim");
    }
}

/// Error row 6: a line longer than the 128-byte buffer is split, never truncated.
#[test]
fn err_06_line_exceeds_buffer_127_split() {
    for len in [128usize, 200, 254, 255, 1000, 5000] {
        let mut v = vec![b'L'; len];
        v.push(b'\n');
        let c = run_file_io(&c_exe(), &v, &[]);
        let r = run_file_io(&rust_exe(), &v, &[]);
        assert_eq!(c, r, "err_06 diverged for len={}", len);
        assert_eq!(c.stdout, v, "an over-long line must be echoed in full");
    }
}

/// Error row 7: an embedded NUL truncates the rest of its chunk.
#[test]
fn err_07_embedded_nul_truncates_chunk() {
    let input = b"ab\x00cd\nxy\n";
    let c = run_file_io(&c_exe(), input, &[]);
    let r = run_file_io(&rust_exe(), input, &[]);
    assert_eq!(c, r, "err_07 diverged");
    assert_eq!(
        c.stdout, b"abxy\n",
        "C drops everything from the NUL to the end of the chunk"
    );
}

/// Error row 8: a chunk that starts with NUL writes nothing but keeps looping.
#[test]
fn err_08_leading_nul_writes_nothing() {
    let input = b"\x00skipped\nkept\n";
    let c = run_file_io(&c_exe(), input, &[]);
    let r = run_file_io(&rust_exe(), input, &[]);
    assert_eq!(c, r, "err_08 diverged");
    assert_eq!(c.stdout, b"kept\n", "the NUL-leading chunk emits nothing");
}

/// Error row 9: an all-NUL input succeeds in `fgets` yet produces no output.
#[test]
fn err_09_all_nul_input_no_output() {
    let input = &[0u8, 0, 0][..];
    let c = run_file_io(&c_exe(), input, &[]);
    let r = run_file_io(&rust_exe(), input, &[]);
    assert_eq!(c, r, "err_09 diverged");
    assert_eq!(c.status, Ok(0));
    assert!(
        c.stdout.is_empty(),
        "non-empty input, but C writes nothing: {:?}",
        Preview(&c.stdout)
    );
}

/// Error row 10: NUL right at the chunk boundary.
#[test]
fn err_10_nul_at_chunk_boundary() {
    // NUL at offset 126 -> first chunk emits 126 bytes, then "bbbbbbbbbb\n"
    let mut a = vec![b'a'; 126];
    a.push(0);
    a.extend_from_slice(&[b'b'; 10]);
    a.push(b'\n');
    let c = run_file_io(&c_exe(), &a, &[]);
    let r = run_file_io(&rust_exe(), &a, &[]);
    assert_eq!(c, r, "err_10 diverged (NUL at 126)");
    assert_eq!(c.stdout.len(), 126 + 11);

    // NUL at offset 127 -> it *starts* the second chunk, which emits nothing
    let mut b = vec![b'a'; 127];
    b.push(0);
    b.extend_from_slice(&[b'b'; 10]);
    b.push(b'\n');
    let c = run_file_io(&c_exe(), &b, &[]);
    let r = run_file_io(&rust_exe(), &b, &[]);
    assert_eq!(c, r, "err_10 diverged (NUL at 127)");
    assert_eq!(c.stdout.len(), 127, "second chunk begins with NUL -> dropped");
}

/// Error row 11: fd 1 closed -> every `fputs` fails, but the result is ignored,
/// so the program still exits 0 after draining stdin.
#[test]
fn err_11_stdout_closed_ebadf_still_drains() {
    // Big enough that a program which stops early would leave stdin unread.
    let input: Vec<u8> = (0..20000u32)
        .flat_map(|i| format!("line {}\n", i).into_bytes())
        .collect();
    let c = run_with_closed_fd(&c_exe(), &input, 1);
    let r = run_with_closed_fd(&rust_exe(), &input, 1);
    assert_eq!(c, r, "err_11 diverged (closed stdout)");
    assert_eq!(c.status, Ok(0), "a failing fputs must not change the exit code");
    assert!(c.stderr.is_empty(), "the C program never writes diagnostics");
}

/// Error row 12: stdout's reader goes away -> SIGPIPE kills the process.
#[test]
fn err_12_broken_pipe_dies_with_sigpipe() {
    let input: Vec<u8> = std::iter::repeat(b"x".repeat(100))
        .take(20000)
        .flat_map(|mut l| {
            l.push(b'\n');
            l
        })
        .collect();
    let c = run_broken_pipe(&c_exe(), &input);
    let r = run_broken_pipe(&rust_exe(), &input);
    assert_eq!(
        c, r,
        "err_12 diverged: C={:?}, Rust={:?} (expected both killed by SIGPIPE=13)",
        c, r
    );
    assert_eq!(
        c,
        Err(13),
        "sanity: the C build is killed by SIGPIPE on a broken stdout"
    );
}

/// Error row 13: stdout is /dev/full -> ENOSPC on flush, ignored.
#[test]
fn err_13_stdout_dev_full_enospc() {
    let p = Path::new("/dev/full");
    let input: Vec<u8> = (0..5000u32)
        .flat_map(|i| format!("l{}\n", i).into_bytes())
        .collect();
    match (
        run_stdout_to_path(&c_exe(), &input, p),
        run_stdout_to_path(&rust_exe(), &input, p),
    ) {
        (Some(c), Some(r)) => {
            assert_eq!(c, r, "err_13 diverged (/dev/full)");
            assert_eq!(c.status, Ok(0), "ENOSPC on stdout must still exit 0");
        }
        _ => {
            eprintln!("err_13: /dev/full not writable in this environment; skipped");
        }
    }
}

/// Error row 14: arguments cannot be rejected, they are simply ignored.
#[test]
fn err_14_arguments_ignored() {
    let input = b"data\n";
    let base = run_file_io(&c_exe(), input, &[]);
    for args in [
        vec!["--bogus-flag"],
        vec!["-x", "-y"],
        vec!["\u{1F600}"],
        vec!["a".repeat(4096).as_str()],
    ] {
        let c = run_file_io(&c_exe(), input, &args);
        let r = run_file_io(&rust_exe(), input, &args);
        assert_eq!(c, r, "err_14 diverged for args {:?}", args);
        assert_eq!(
            c.stdout, base.stdout,
            "arguments must not change the output"
        );
        assert_eq!(c.status, Ok(0));
    }
}

// ===========================================================================
// tty helpers used by rows 29 and 30
// ===========================================================================

fn run_with_tty_stdin(exe: &Path, input: &[u8]) -> Outcome {
    use std::fs;
    use std::io::Write;
    use std::process::{Command, Stdio};

    let _g = fd_lock().lock().unwrap();
    let (master, slave) = open_pty();
    let out_path = temp_path("out");
    let fout = fs::File::create(&out_path).expect("create stdout temp");

    let child = Command::new(exe)
        .stdin(unsafe { stdio_from(slave) })
        .stdout(Stdio::from(fout))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    close_extra();

    {
        let mut m = unsafe { file_from(master) };
        m.write_all(input).expect("write pty");
        m.flush().ok();
        // Ctrl-D twice: EOF for a tty in canonical mode
        m.write_all(&[4u8]).ok();
        m.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(60));
        m.write_all(&[4u8]).ok();
        m.flush().ok();
    }
    // Closing the master makes the slave read return 0 / EIO -> loop ends.
    unsafe { libc::close(master) };

    let out = child.wait_with_output().expect("wait");
    let stdout = read_or_empty(&out_path);
    let _ = fs::remove_file(&out_path);
    Outcome {
        status: match out.status.code() {
            Some(c) => Ok(c),
            None => Err(std::os::unix::process::ExitStatusExt::signal(&out.status).unwrap_or(-1)),
        },
        stdout,
        stderr: out.stderr,
    }
}

/// stdin and stdout on the *same* pty: write a line, read the echo back.
fn interactive_tty_roundtrip(exe: &Path) -> Vec<u8> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let _g = fd_lock().lock().unwrap();
    let (master, slave) = open_pty();
    let mut child = Command::new(exe)
        .stdin(unsafe { stdio_from(slave) })
        .stdout(unsafe { stdio_from(slave) })
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    close_extra();

    {
        let mut m = unsafe { file_from(master) };
        m.write_all(b"typed line\n").expect("write pty");
        m.flush().ok();
    }
    // The pty echoes our own input back too, so look for the *second* copy.
    let seen = poll_read_bytes(master, 800, 4096);
    let _ = child.kill();
    let _ = child.wait();
    unsafe { libc::close(master) };

    // Count how many times the line appears: 1 = only the tty's own echo,
    // 2 = the program echoed it as well (what the C build does).
    let needle = b"typed line";
    let count = seen
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count();
    format!("occurrences={}", count).into_bytes()
}

unsafe fn stdio_from(fd: i32) -> std::process::Stdio {
    use std::os::fd::FromRawFd;
    let d = libc::dup(fd);
    assert!(d >= 0, "dup");
    set_cloexec(d);
    std::process::Stdio::from_raw_fd(d)
}

unsafe fn file_from(fd: i32) -> std::mem::ManuallyDrop<std::fs::File> {
    use std::os::fd::FromRawFd;
    std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd))
}

// ===========================================================================
// Additional configuration rows found while probing observable side effects
// ===========================================================================

/// Row 34: stdin **and** stdout are the same file, opened separately.
///
/// The input contains NUL bytes, so `fputs` truncation makes the output shorter
/// than the input: the write offset lags the read offset and the file is being
/// rewritten while it is still being read. The resulting bytes therefore depend
/// on the exact interleaving of the stream's 4096-byte reads and writes, which
/// makes this the sharpest available test that the Rust side reproduces glibc's
/// buffering discipline and not merely the final byte sequence.
#[test]
fn cfg_34_same_file_stdin_stdout() {
    use std::fs;
    use std::process::{Command, Stdio};

    fn run(exe: &Path, seed_bytes: &[u8]) -> Vec<u8> {
        let path = write_temp("samefile", seed_bytes);
        let fin = fs::File::open(&path).expect("open ro");
        // deliberately no truncate: the same file, a second open, offset 0
        let fout = fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open rw");
        let st = Command::new(exe)
            .stdin(Stdio::from(fin))
            .stdout(Stdio::from(fout))
            .stderr(Stdio::null())
            .status()
            .expect("spawn");
        assert!(st.success(), "{} failed: {:?}", exe.display(), st);
        let out = fs::read(&path).expect("read back");
        let _ = fs::remove_file(&path);
        out
    }

    let mut seed = Vec::new();
    for i in 0..4000u32 {
        seed.extend_from_slice(format!("rec{:05}", i).as_bytes());
        seed.push(0);
        seed.extend_from_slice(b"HIDDENHIDDENHIDDEN");
        seed.push(b'\n');
    }

    let c = run(&c_exe(), &seed);
    let r = run(&rust_exe(), &seed);
    assert_eq!(
        c.len(),
        r.len(),
        "same-file echo produced different file lengths (C={}, Rust={})",
        c.len(),
        r.len()
    );
    assert!(
        c == r,
        "same-file echo diverged at byte {:?} -- the read/write interleaving differs",
        first_diff(&c, &r)
    );
}

/// Row 35: how much of a *shared* stdin does the program consume?
///
/// The child gets a dup of our descriptor, so it shares the file offset. After
/// the child exits, whatever is left is what the next reader of that descriptor
/// would see - an externally visible consequence of the input buffering and of
/// `main` draining stdin even when writes fail.
#[test]
fn cfg_35_shared_stdin_leftover() {
    use std::fs;
    use std::io::Read;
    use std::process::{Command, Stdio};

    fn leftover(exe: &Path, input: &[u8]) -> usize {
        let path = write_temp("shared", input);
        let mut fin = fs::File::open(&path).expect("open");
        let dup = fin.try_clone().expect("dup"); // dup(2): shares the offset
        let st = Command::new(exe)
            .stdin(Stdio::from(dup))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("spawn");
        assert!(st.success());
        let mut rest = Vec::new();
        fin.read_to_end(&mut rest).expect("read rest");
        let _ = fs::remove_file(&path);
        rest.len()
    }

    let input: Vec<u8> = (0..4000u32)
        .flat_map(|i| format!("x{:04}\n", i).into_bytes())
        .collect();
    let c = leftover(&c_exe(), &input);
    let r = leftover(&rust_exe(), &input);
    assert_eq!(
        c, r,
        "leftover bytes on a shared stdin differ (C={}, Rust={})",
        c, r
    );
    assert_eq!(c, 0, "sanity: the C program drains stdin to EOF");
}

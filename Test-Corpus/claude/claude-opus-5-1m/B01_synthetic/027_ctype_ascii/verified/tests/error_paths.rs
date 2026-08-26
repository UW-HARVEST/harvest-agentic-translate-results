//! Phase C -- error-path differential tests, one per row of ERRORS.md.
//!
//! Both implementations are driven through their exported `driver` / `main`
//! symbols and must reject / degrade identically (same bytes, same return
//! value, same exit status / signal).

mod common;

use common::*;

/// The 14 lines the C code prints for `char c`, obtained from the C `.so`
/// itself (used as the reference in the "identical to row 1" rows).
fn c_reference(c: i8) -> Vec<u8> {
    let l = libs();
    let run = run_ops(&l.c, &[Op::Driver(c)], &Cfg::default());
    assert_eq!(run.exit_code, Some(0));
    assert!(!run.stdout.is_empty());
    run.stdout
}

// --- row 1 -----------------------------------------------------------------

#[test]
fn err_01_main_stdin_empty_eof() {
    // getchar() == EOF (-1) -> (char)-1; the C code performs no EOF check.
    let run = assert_same("err_01", &[Op::Main], &Cfg::stdin_file(&[]));
    assert_eq!(run.rets, vec![0], "main() must return 0");
    assert_eq!(
        run.stdout,
        c_reference(-1),
        "EOF must produce the output of char -1"
    );
    // Also through a pipe (non-seekable, read() returns 0).
    let run = assert_same("err_01/pipe", &[Op::Main], &Cfg::stdin_pipe(&[]));
    assert_eq!(run.stdout, c_reference(-1));
}

// --- row 2 -----------------------------------------------------------------

#[test]
fn err_02_main_stdin_devnull() {
    let cfg = Cfg {
        stdin: StdinSpec::DevNull,
        ..Cfg::default()
    };
    let run = assert_same("err_02", &[Op::Main], &cfg);
    assert_eq!(run.rets, vec![0]);
    assert_eq!(run.stdout, c_reference(-1));
}

// --- row 3 -----------------------------------------------------------------

#[test]
fn err_03_main_stdin_closed() {
    // fd 0 closed -> read() fails with EBADF -> getc returns EOF.
    let cfg = Cfg {
        stdin: StdinSpec::Closed,
        ..Cfg::default()
    };
    let run = assert_same("err_03", &[Op::Main], &cfg);
    assert_eq!(run.rets, vec![0]);
    assert_eq!(run.stdout, c_reference(-1));
    // Repeated calls with a broken stdin keep returning EOF.
    let run = assert_same("err_03/repeated", &[Op::Main, Op::Main, Op::Main], &cfg);
    assert_eq!(run.rets, vec![0, 0, 0]);
}

// --- row 4 -----------------------------------------------------------------

#[test]
fn err_04_main_stdin_write_only() {
    // fd 0 is an O_WRONLY file -> read() fails with EBADF.
    let cfg = Cfg {
        stdin: StdinSpec::WriteOnly,
        ..Cfg::default()
    };
    let run = assert_same("err_04", &[Op::Main], &cfg);
    assert_eq!(run.rets, vec![0]);
    assert_eq!(run.stdout, c_reference(-1));
}

// --- row 5 -----------------------------------------------------------------

#[test]
fn err_05_main_stdin_directory() {
    // fd 0 is a directory -> read() fails with EISDIR.
    let cfg = Cfg {
        stdin: StdinSpec::Directory,
        ..Cfg::default()
    };
    let run = assert_same("err_05", &[Op::Main], &cfg);
    assert_eq!(run.rets, vec![0]);
    assert_eq!(run.stdout, c_reference(-1));
}

// --- row 6 -----------------------------------------------------------------

#[test]
fn err_06_main_byte_ff_aliases_eof() {
    // Byte 0xFF and EOF are indistinguishable, because `char` is signed and no
    // EOF check exists.
    let byte_ff = assert_same("err_06/0xFF", &[Op::Main], &Cfg::stdin_file(&[0xFF]));
    let eof = assert_same("err_06/eof", &[Op::Main], &Cfg::stdin_file(&[]));
    assert_eq!(byte_ff.stdout, eof.stdout);
    assert_eq!(byte_ff.stdout, c_reference(-1));
    assert_eq!(byte_ff.rets, vec![0]);
}

// --- row 7 -----------------------------------------------------------------

#[test]
fn err_07_main_byte_nul() {
    // An embedded NUL byte: `printf("%c")` writes it into the stream.
    let run = assert_same("err_07", &[Op::Main], &Cfg::stdin_file(&[0x00, b'A']));
    assert_eq!(run.stdout, c_reference(0));
    assert_eq!(run.stdout.iter().filter(|&&b| b == 0).count(), 2);
    assert!(run.stdout.starts_with(b"alphanumeric: 0\n"));
    assert!(run.stdout.windows(11).any(|w| w == b"control: 2\n"));
}

// --- row 8 -----------------------------------------------------------------

#[test]
fn err_08_main_stdout_closed() {
    // fd 1 closed: every printf fails, the return value is never checked.
    let cfg = Cfg::stdin_file(&[b'Q']).with_stdout(StdoutSpec::Closed);
    let run = assert_same("err_08", &[Op::Main], &cfg);
    assert!(run.stdout.is_empty(), "no output is observable");
    assert_eq!(run.rets, vec![0], "main() still returns 0");
    assert_eq!(run.exit_code, Some(0));
}

// --- row 9 -----------------------------------------------------------------

#[test]
fn err_09_main_stdout_broken_pipe_sigpipe_ignored() {
    // Writes fail with EPIPE; unchecked, so the call still completes.
    let cfg = Cfg::stdin_file(&[b'Z'])
        .with_stdout(StdoutSpec::BrokenPipe)
        .ignoring_sigpipe();
    let run = assert_same("err_09", &[Op::Main], &cfg);
    assert!(run.stdout.is_empty());
    assert_eq!(run.rets, vec![0]);
    assert_eq!(run.exit_code, Some(0));

    let cfg = Cfg::default()
        .with_stdout(StdoutSpec::BrokenPipe)
        .ignoring_sigpipe();
    let run = assert_same("err_09/driver", &[Op::Driver(b'z' as i8)], &cfg);
    assert!(run.stdout.is_empty());
}

// --- row 10 (shared-object half; the executable half lives in binaries_e2e) --

#[test]
fn err_10_broken_pipe_default_sigpipe_kills_both() {
    // With the default SIGPIPE disposition the write kills the process.
    let cfg = Cfg::stdin_file(&[b'Z'])
        .with_stdout(StdoutSpec::BrokenPipe)
        .with_default_sigpipe();
    let (c, r) = assert_same_allow_abnormal("err_10", &[Op::Main], &cfg);
    assert_eq!(c.signal, Some(13), "C must be killed by SIGPIPE: {c:?}");
    assert_eq!(r.signal, Some(13), "Rust must be killed by SIGPIPE: {r:?}");
    assert!(c.stdout.is_empty() && r.stdout.is_empty());

    let cfg = Cfg::default()
        .with_stdout(StdoutSpec::BrokenPipe)
        .with_default_sigpipe();
    let (c, r) = assert_same_allow_abnormal("err_10/driver", &[Op::Driver(1)], &cfg);
    assert_eq!(c.signal, Some(13));
    assert_eq!(r.signal, Some(13));
}

// --- row 11 ----------------------------------------------------------------

#[test]
fn err_11_driver_minus_one() {
    // Negative table index (`__ctype_b_loc()[-1]`).
    let run = assert_driver_same("err_11", -1);
    assert_eq!(run.exit_code, Some(0));
    assert!(run.stdout.ends_with(&[b' ', 0xFF, b'\n']), "{run:?}");
    for line in run.stdout.split(|&b| b == b'\n') {
        if let Some(pos) = line.iter().position(|&b| b == b':') {
            let (name, value) = (&line[..pos], &line[pos + 2..]);
            if name != b"to lower" && name != b"to upper" {
                assert_eq!(value, b"0", "class {} must be 0", escape(name));
            }
        }
    }
}

// --- row 12 ----------------------------------------------------------------

#[test]
fn err_12_driver_minus_128() {
    let run = assert_driver_same("err_12", -128);
    assert_eq!(run.exit_code, Some(0));
    assert!(run.stdout.ends_with(&[b' ', 0x80, b'\n']), "{run:?}");
}

// --- row 13 ----------------------------------------------------------------

#[test]
fn err_13_driver_del_127() {
    let run = assert_driver_same("err_13", 127);
    assert!(run.stdout.windows(11).any(|w| w == b"control: 2\n"), "{run:?}");
    assert!(run.stdout.ends_with(&[b' ', 0x7F, b'\n']), "{run:?}");
}

// --- row 14 ----------------------------------------------------------------

#[test]
fn err_14_driver_nul() {
    let run = assert_driver_same("err_14", 0);
    assert_eq!(run.stdout.iter().filter(|&&b| b == 0).count(), 2, "{run:?}");
    assert!(run.stdout.windows(11).any(|w| w == b"control: 2\n"));
}

// --- row 15 ----------------------------------------------------------------

#[test]
fn err_15_driver_all_negative_chars() {
    // Every out-of-`unsigned char`-range value a `char` can hold.
    for b in 0x80u8..=0xFF {
        let run = assert_driver_same(&format!("err_15/0x{b:02x}"), b as i8);
        assert!(
            run.stdout.ends_with(&[b' ', b, b'\n']),
            "toupper must be the identity for 0x{b:02x}: {run:?}"
        );
        assert!(run.stdout.starts_with(b"alphanumeric: 0\n"), "{run:?}");
    }
    // ... and all of them in one process.
    let ops: Vec<Op> = (0x80u8..=0xFF).map(|b| Op::Driver(b as i8)).collect();
    for chunk in ops.chunks(32) {
        assert_same("err_15/one-process", chunk, &Cfg::default());
    }
}

// --- row 16 ----------------------------------------------------------------

#[test]
fn err_16_driver_int_with_garbage_high_bits() {
    // The `char` parameter reached through a `void driver(int)` prototype: the
    // out-of-range analogue of an invalid C enum value.
    let mut rng = Rng::new(SEED ^ 0x1616);
    let mut values: Vec<i32> = vec![
        i32::MIN,
        i32::MAX,
        -1,
        0,
        256,
        257,
        -256,
        -257,
        0x0000_FF00,
        0x7FFF_FFFF,
        0xDEAD_BEEFu32 as i32,
        0x0100_0000,
        1 << 31,
    ];
    for _ in 0..512 {
        values.push(rng.next_i32());
    }
    for v in values {
        let label = format!("err_16/0x{:08x}", v as u32);
        let wide = assert_same(&label, &[Op::DriverInt(v)], &Cfg::default());
        let narrow = c_reference((v & 0xFF) as u8 as i8);
        assert_eq!(
            wide.stdout, narrow,
            "0x{:08x} must be treated as char 0x{:02x}",
            v as u32,
            (v & 0xFF) as u8
        );
    }
}

// --- row 17 ----------------------------------------------------------------

#[test]
fn err_17_driver_ignores_host_locale() {
    // setlocale(LC_ALL, "C") return value is unchecked; the caller's locale
    // must not influence the result even when it classifies 0x80..0xFF.
    for loc in ["en_US.iso88591", "de_DE.iso88591", "en_US.utf8", "C.utf8"] {
        if !locale_available(loc) {
            continue;
        }
        let cfg = Cfg::default().with_locale(loc);
        for b in (0x00u8..=0xFF).step_by(5) {
            let run = assert_same(
                &format!("err_17/{loc}/0x{b:02x}"),
                &[Op::Driver(b as i8)],
                &cfg,
            );
            assert_eq!(
                run.stdout,
                c_reference(b as i8),
                "locale {loc} leaked into the output for 0x{b:02x}"
            );
        }
    }
    // A locale name that does not exist: setlocale() returns NULL, unchecked.
    let cfg = Cfg::default().with_locale("xx_YY.nonexistent");
    let run = assert_same("err_17/bogus-locale", &[Op::Driver(b'A' as i8)], &cfg);
    assert_eq!(run.stdout, c_reference(b'A' as i8));
}

// --- row 18 ----------------------------------------------------------------

#[test]
fn err_18_driver_stdout_closed() {
    let cfg = Cfg::default().with_stdout(StdoutSpec::Closed);
    let mut rng = Rng::new(SEED ^ 0x1818);
    for _ in 0..16 {
        let b = rng.byte();
        let run = assert_same(
            &format!("err_18/0x{b:02x}"),
            &[Op::Driver(b as i8)],
            &cfg,
        );
        assert!(run.stdout.is_empty());
        assert_eq!(run.exit_code, Some(0));
    }
    // Many failing calls in a row must still not surface an error.
    let ops: Vec<Op> = (0..32).map(|_| Op::Driver(rng.byte() as i8)).collect();
    let run = assert_same("err_18/batched", &ops, &cfg);
    assert!(run.stdout.is_empty());
}

// --- row 19 ----------------------------------------------------------------

#[test]
fn err_19_driver_repeated_calls_no_state_drift() {
    // The same value many times must produce exactly N copies of one block.
    let single = c_reference(b'k' as i8);
    let ops = vec![Op::Driver(b'k' as i8); 40];
    let run = assert_same("err_19", &ops, &Cfg::default());
    let mut expect = Vec::new();
    for _ in 0..40 {
        expect.extend_from_slice(&single);
    }
    assert_eq!(run.stdout, expect);
}

// --- row 20 ----------------------------------------------------------------

#[test]
fn err_20_main_repeated_calls_consume_successive_bytes() {
    // Successive calls consume successive bytes, then EOF forever.
    let data = [b'a', b'B', 0x00, 0xFF, b'7'];
    let ops = vec![Op::Main; 7];
    let run = assert_same("err_20/file", &ops, &Cfg::stdin_file(&data));
    let mut expect = Vec::new();
    for b in data {
        expect.extend_from_slice(&c_reference(b as i8));
    }
    // Calls 6 and 7 hit EOF -> the char -1 output (identical to byte 0xFF).
    expect.extend_from_slice(&c_reference(-1));
    expect.extend_from_slice(&c_reference(-1));
    assert_eq!(run.stdout, expect);
    assert_eq!(run.rets, vec![0; 7]);

    let run = assert_same("err_20/pipe", &ops, &Cfg::stdin_pipe(&data));
    assert_eq!(run.stdout, expect);
}

// --- generic FFI boundaries that the API does not expose -------------------

#[test]
fn err_generic_extra_arguments_across_the_ffi_boundary() {
    // `int main()` is *unprototyped* in C, so a caller may legitimately pass
    // argc/argv/envp; and a caller may hand `driver` extra register arguments.
    // Both must be ignored identically.
    let mut rng = Rng::new(SEED ^ 0xEEEE);
    for i in 0..64 {
        let argc = rng.next_i32();
        let argv = rng.next_u64() as usize;
        let envp = rng.next_u64() as usize;
        let b = rng.byte();
        let run = assert_same(
            &format!("err_generic/main(argc,argv,envp) @{i}"),
            &[Op::MainArgs(argc, argv, envp)],
            &Cfg::stdin_file(&[b]),
        );
        assert_eq!(run.rets, vec![0], "main must still return 0");
        assert_eq!(run.stdout, c_reference(b as i8));

        let run = assert_same(
            &format!("err_generic/driver(c,junk,junk) @{i}"),
            &[Op::DriverExtra(b as i8, argc, envp)],
            &Cfg::default(),
        );
        assert_eq!(run.stdout, c_reference(b as i8));
    }
}

#[test]
fn err_generic_no_pointer_or_length_parameters() {
    // `driver(char)` and `main(void)` take no pointer, length or enum
    // parameter, so the generic null / zero-length / oversized-length /
    // invalid-enum cases reduce to row 16 (any register bit pattern) and are
    // covered there. This test documents the surface and asserts the exported
    // symbol set is exactly the two functions.
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(c_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut names: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .collect();
    names.sort_unstable();
    assert_eq!(names, vec!["driver", "main"], "unexpected C export surface");
}

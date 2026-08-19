//! Phase B — rows C25/C26 of CONFIGS.md: the real programs, compared end to
//! end.  `c_src/build/driver` (CMake's `add_executable`) vs the cargo-built
//! `driver`: same stdin bytes, compare stdout, exit code and terminating signal.

mod common;

use common::*;

/// A representative sample of every input class from CONFIGS.md C10–C22.
fn input_classes() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"\n".to_vec(),
        b"\t\t\t".to_vec(),
        b"\x0b\x0c\r ".to_vec(),
        b"0".to_vec(),
        b"0\n".to_vec(),
        b"7".to_vec(),
        b"42\n".to_vec(),
        b"-42\n".to_vec(),
        b"+42\n".to_vec(),
        b"  \t\n  -42\r\n".to_vec(),
        b"000000000000042".to_vec(),
        b"-000000000000042".to_vec(),
        b"2147483647".to_vec(),
        b"2147483648".to_vec(),
        b"-2147483648".to_vec(),
        b"-2147483649".to_vec(),
        b"4294967295".to_vec(),
        b"4294967296".to_vec(),
        b"9223372036854775807".to_vec(),
        b"9223372036854775808".to_vec(),
        b"-9223372036854775808".to_vec(),
        b"-9223372036854775809".to_vec(),
        b"18446744073709551616".to_vec(),
        b"-18446744073709551616".to_vec(),
        b"340282366920938463463374607431768211456".to_vec(),
        b"abc".to_vec(),
        b".5".to_vec(),
        b"-".to_vec(),
        b"+".to_vec(),
        b"-a".to_vec(),
        b"+ 5".to_vec(),
        b"--5".to_vec(),
        b"0x10".to_vec(),
        b"12abc".to_vec(),
        b"1 2".to_vec(),
        b"12\n34".to_vec(),
        b"\0".to_vec(),
        b"12\0".to_vec(),
        b"\xff".to_vec(),
        b"\xc3\xa912".to_vec(),
    ];
    // Buffer-boundary shapes.
    for n in [4_095usize, 4_096, 4_097, 8_191, 8_192, 8_193] {
        let mut spaces = vec![b' '; n];
        spaces.extend_from_slice(b"-1234\n");
        v.push(spaces);
        v.push(format!("{}\n", "9".repeat(n)).into_bytes());
        v.push(format!("{}7\n", "0".repeat(n)).into_bytes());
    }
    // Randomised values across the whole ladder.
    let mut rng = Rng::new(0xE1E1);
    for _ in 0..60 {
        let sign = rng.pick(&["", "-", "+"]);
        let len = 1 + rng.below(24) as usize;
        v.push(format!("{sign}{}\n", rng.digits(len)).into_bytes());
    }
    for _ in 0..40 {
        let len = 1 + rng.below(10) as usize;
        v.push((0..len).map(|_| rng.next_u64() as u8).collect());
    }
    v
}

/// C25 — stdin is a regular file.
#[test]
fn c25_executables_stdin_file() {
    for input in input_classes() {
        diff_exe_file_stdin(&input);
    }
}

/// C26 — stdin and stdout are pipes.
#[test]
fn c26_executables_stdin_pipe() {
    for input in input_classes() {
        diff_exe_pipe_stdin(&input);
    }
}

/// C29 — the environment: the C program never calls `setlocale`, so it stays in
/// the "C" locale whatever `LC_*`/`LANG` say.  The translation must be just as
/// insensitive (a locale-aware digit parser or `%02x` formatter would show up
/// here).
#[test]
fn c29_executables_ignore_the_locale_environment() {
    use std::process::{Command, Stdio};

    let envs: [&[(&str, &str)]; 6] = [
        &[("LC_ALL", "C")],
        &[("LC_ALL", "POSIX")],
        &[("LC_ALL", "en_US.UTF-8")],
        &[("LC_ALL", "tr_TR.UTF-8")],
        &[("LC_NUMERIC", "de_DE.UTF-8"), ("LANG", "de_DE.UTF-8")],
        &[("LANG", "fr_FR.UTF-8"), ("LC_ALL", "")],
    ];
    let inputs: [&[u8]; 8] = [
        b"1234567",
        b"-1234567",
        b"1.234",
        b"1,234",
        b"2147483648",
        b"9223372036854775808",
        b"",
        b"abc",
    ];

    for env in envs {
        for input in inputs {
            let mut outs = Vec::new();
            for exe in [c_exe_path(), rust_exe_path()] {
                let path = scratch_file("locale-stdin");
                std::fs::write(&path, input).unwrap();
                let f = std::fs::File::open(&path).unwrap();
                let mut cmd = Command::new(exe);
                cmd.stdin(Stdio::from(f)).stdout(Stdio::piped());
                for (k, v) in env {
                    cmd.env(k, v);
                }
                let out = cmd.output().unwrap();
                let _ = std::fs::remove_file(&path);
                outs.push((as_text(&out.stdout), out.status.code()));
            }
            assert_eq!(
                outs[0],
                outs[1],
                "locale {env:?} changed the result for {}",
                preview(input)
            );
        }
    }
}

/// Sanity anchor: the C program's output for a known input, so a silent
/// regression in *both* implementations cannot pass unnoticed.
#[test]
fn known_good_outputs() {
    for (input, expect) in [
        (&b"42\n"[..], "2a000000\n"),
        (b"-42", "d6ffffff\n"),
        (b"", "00000000\n"),
        (b"abc", "00000000\n"),
        (b"2147483648", "00000080\n"),
        (b"9223372036854775808", "ffffffff\n"),
        (b"-9223372036854775809", "00000000\n"),
    ] {
        let c = run_exe_with_file_stdin(c_exe_path(), input);
        let r = run_exe_with_file_stdin(rust_exe_path(), input);
        assert_eq!(as_text(&c.out), expect, "C output for {}", preview(input));
        assert_eq!(as_text(&r.out), expect, "Rust output for {}", preview(input));
        assert_eq!(c.status, Some(0));
        assert_eq!(r.status, Some(0));
    }
}

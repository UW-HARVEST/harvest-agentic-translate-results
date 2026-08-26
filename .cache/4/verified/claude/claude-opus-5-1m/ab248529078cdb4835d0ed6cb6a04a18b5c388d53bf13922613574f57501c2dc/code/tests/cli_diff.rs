//! Phase B — differential tests of the complete programs (CONFIGS.md entry
//! point `EP3`): the CMake-built C `driver` and the cargo-built Rust `driver`
//! are spawned with identical arguments and their stdout, stderr and exit
//! status must match byte for byte.

mod common;

use common::{assert_exe_same, c_exe, run_exe, run_exe_full, rust_exe, Rng, SEED};

const TAG: &str = "cli_diff";

fn same(arg: &[u8]) {
    assert_exe_same(TAG, &[arg.to_vec()]);
}

// ------------------------------------------------------------------ row 10 ---

#[test]
fn cfg_small_strides() {
    for v in -50i32..=50 {
        same(v.to_string().as_bytes());
    }
    for arg in [&b"0"[..], b"1", b"2", b"7", b"-1", b"-3"] {
        same(arg);
    }
}

// ------------------------------------------------------------------ row 11 ---

#[test]
fn cfg_whitespace_sign_zeros() {
    for arg in [
        &b"+2"[..],
        b"-0",
        b"+0",
        b"0000",
        b" 7",
        b"\t-4",
        b"\n\x0b\x0c\r 12",
        b"   +000123",
        b"\r\n-000000000000000005",
        b"          8",
        b"+00000000000000000009",
        b"-000",
        b" \t 42 ",
    ] {
        same(arg);
    }

    let mut rng = Rng::new(SEED ^ 0xB1);
    let spaces: [&[u8]; 7] = [b"", b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r"];
    let signs: [&[u8]; 3] = [b"", b"+", b"-"];
    for _ in 0..150 {
        let mut s = Vec::new();
        for _ in 0..rng.below(4) {
            s.extend_from_slice(*rng.pick(&spaces));
        }
        s.extend_from_slice(*rng.pick(&signs));
        for _ in 0..rng.below(4) {
            s.push(b'0');
        }
        s.extend_from_slice(rng.below(1_000_000).to_string().as_bytes());
        same(&s);
    }
}

// ------------------------------------------------------------------ row 12 ---

#[test]
fn cfg_trailing_garbage() {
    for arg in [
        &b"5abc"[..], b"0x10", b"0X10", b"3 4", b"7\n", b"12.", b"9,9", b"1e5", b"6-", b"2+3",
        b"42/", b"8:", b"-7xyz", b"+9 ", b"0b101", b"1_000", b"3.14",
    ] {
        same(arg);
    }

    let mut rng = Rng::new(SEED ^ 0xB2);
    let suffix: &[u8] = b"abcxyzXYZ .,;:/-+*_'\"\\|()[]{}\t\n\r\x0b\x0c#$%&!?@^~`=<>";
    for _ in 0..150 {
        let mut s = Vec::new();
        if rng.below(2) == 0 {
            s.push(if rng.below(2) == 0 { b'+' } else { b'-' });
        }
        s.extend_from_slice(rng.below(100_000).to_string().as_bytes());
        for _ in 0..1 + rng.below(4) {
            s.push(suffix[rng.below(suffix.len() as u64) as usize]);
        }
        same(&s);
    }
}

// ------------------------------------------------------------------ row 13 ---

#[test]
fn cfg_digit_count_sweep() {
    let mut rng = Rng::new(SEED ^ 0xB3);
    for digits in 1..=19u32 {
        for sign in ["", "+", "-"] {
            let lo = if digits == 1 { 0u128 } else { 10u128.pow(digits - 1) };
            let hi = 10u128.pow(digits) - 1;
            for v in [lo, hi] {
                same(format!("{sign}{v}").as_bytes());
            }
            for _ in 0..4 {
                let span = (hi - lo + 1) as u64 as u128;
                let v = lo + (rng.next_u64() as u128) % span.max(1);
                same(format!("{sign}{v}").as_bytes());
            }
        }
    }
}

// ------------------------------------------------------------------ row 14 ---

#[test]
fn cfg_long_digit_runs() {
    let mut rng = Rng::new(SEED ^ 0xB4);
    for len in [20usize, 21, 25, 39, 40, 63, 64, 100, 199, 200] {
        for sign in ["", "+", "-"] {
            for body in ["9".repeat(len), "1".repeat(len), format!("{}7", "0".repeat(len - 1))] {
                same(format!("{sign}{body}").as_bytes());
            }
            let random: String = (0..len)
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            same(format!("{sign}{random}").as_bytes());
        }
    }
}

// ------------------------------------------------------------------ row 15 ---

#[test]
fn cfg_random_i32_strides() {
    let mut rng = Rng::new(SEED ^ 0xB5);
    for _ in 0..400 {
        same(rng.next_i32().to_string().as_bytes());
    }
    // powers of two and their neighbours: the values that make `i * stride`
    // and `sum += update` overflow in the most interesting ways.
    for k in 0..32u32 {
        let p = 1i32.wrapping_shl(k);
        for v in [p, p.wrapping_neg(), p.wrapping_add(1), p.wrapping_sub(1)] {
            same(v.to_string().as_bytes());
        }
    }
}

// -------------------------------------------------------------- rows 16, 17 --

#[test]
fn cfg_int_boundaries() {
    for arg in [
        "2147483645",
        "2147483646",
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967295",
        "-4294967296",
        "8589934592",
        "-8589934592",
        "6442450944",
        "12884901888",
    ] {
        same(arg.as_bytes());
    }
}

// -------------------------------------------------------------- rows 18–21 --

#[test]
fn cfg_long_boundaries() {
    for arg in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "9223372036854775810",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775806",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "-18446744073709551616",
        "+9223372036854775807",
        "+9223372036854775808",
        "00000000009223372036854775808",
    ] {
        same(arg.as_bytes());
    }
}

// ------------------------------------------------------------------ row 22 ---

#[test]
fn cfg_random_i64_strings() {
    let mut rng = Rng::new(SEED ^ 0xB6);
    for _ in 0..250 {
        same((rng.next_u64() as i64).to_string().as_bytes());
        let shift = rng.below(64) as u32;
        same(((rng.next_u64() >> shift) as i64).to_string().as_bytes());
    }
}

// ------------------------------------------------------------------ row 23 ---

#[test]
fn cfg_random_byte_soup() {
    let mut rng = Rng::new(SEED ^ 0xB7);
    for _ in 0..400 {
        let len = rng.below(12) as usize;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            // No NUL: execve() cannot carry it in an argument.
            let b = match rng.below(10) {
                0..=4 => b'0' + rng.below(10) as u8,
                5 => *rng.pick(&[b'+', b'-']),
                6 => *rng.pick(&[b' ', b'\t', b'\n', b'\x0b', b'\x0c', b'\r']),
                7 => b'a' + rng.below(26) as u8,
                8 => 0x80 + rng.below(0x80) as u8,
                _ => 0x21 + rng.below(0x5e) as u8,
            };
            s.push(b);
        }
        same(&s);
    }
}

// ------------------------------------------------------------------ row 29 ---

#[test]
fn cfg_stdout_file_vs_pipe() {
    use std::process::Stdio;

    for arg in ["1", "-7", "1000000000", "abc", ""] {
        // stdout = pipe (what run_exe uses)
        let piped_c = run_exe(&c_exe(TAG), &[arg.as_bytes().to_vec()]);
        let piped_r = run_exe(&rust_exe(), &[arg.as_bytes().to_vec()]);
        common::assert_same(&piped_c, &piped_r, &[arg.as_bytes().to_vec()]);

        // stdout = regular file (fully buffered in C, flushed at exit)
        let dir = common::work_dir(TAG);
        let mut bytes = Vec::new();
        for (i, exe) in [c_exe(TAG), rust_exe()].iter().enumerate() {
            let path = dir.join(format!("stdout_{i}_{}.txt", arg.len()));
            let file = std::fs::File::create(&path).unwrap();
            let status = std::process::Command::new(exe)
                .arg(arg)
                .stdout(Stdio::from(file))
                .status()
                .unwrap();
            bytes.push((std::fs::read(&path).unwrap(), status.code()));
        }
        assert_eq!(
            bytes[0], bytes[1],
            "stdout-to-file differs for {arg:?}"
        );
        assert_eq!(
            bytes[0].0, piped_c.stdout,
            "C: stdout-to-file differs from stdout-to-pipe for {arg:?}"
        );
        assert_eq!(
            bytes[1].0, piped_r.stdout,
            "Rust: stdout-to-file differs from stdout-to-pipe for {arg:?}"
        );
    }
}

// ------------------------------------------------------------------ row 31 ---

#[test]
fn cfg_argv0_variation() {
    for argv0 in [
        &b"driver"[..],
        b"./driver",
        b"",
        b"a-very-long-program-name-that-should-not-matter-at-all-0123456789",
        b"\xff\xfe",
        b"/usr/bin/whatever",
    ] {
        for arg in [&b"3"[..], b"abc", b""] {
            let args = vec![arg.to_vec()];
            let c = run_exe_full(&c_exe(TAG), &args, Some(argv0), None);
            let r = run_exe_full(&rust_exe(), &args, Some(argv0), None);
            common::assert_same(&c, &r, &args);
        }
    }
}

// ------------------------------------------------------------------ row 33 ---

/// Oversized arguments (well past any fixed-size buffer, up to just below
/// `MAX_ARG_STRLEN`).
#[test]
fn cfg_oversized_arguments() {
    let mut rng = Rng::new(SEED ^ 0xB8);
    for len in [1_000usize, 10_000, 100_000] {
        same(&vec![b'7'; len]);
        same(&vec![b'0'; len]);
        same(&vec![b'z'; len]);
        let mut leading_spaces = vec![b' '; len];
        leading_spaces.push(b'5');
        same(&leading_spaces);
        let mut digits_then_junk: Vec<u8> = b"-12345".to_vec();
        digits_then_junk.extend(std::iter::repeat(b'x').take(len));
        same(&digits_then_junk);
        let random_digits: Vec<u8> = (0..len).map(|_| b'0' + rng.below(10) as u8).collect();
        same(&random_digits);
    }
}

// ------------------------------------------------------------------ row 34 ---

/// fd 1 closed before `exec`: `printf` fails with `EBADF`, which the C code
/// ignores, so the exit status must still be 0 (resp. 1 for a bad argument).
#[test]
fn cfg_stdout_closed() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    for arg in ["1", "abc", ""] {
        let mut results = Vec::new();
        for exe in [c_exe(TAG), rust_exe()] {
            let mut cmd = Command::new(&exe);
            cmd.arg(arg);
            unsafe {
                cmd.pre_exec(|| {
                    libc::close(1);
                    Ok(())
                });
            }
            let out = cmd.output().expect("spawn");
            results.push((out.status.code(), out.stderr));
        }
        assert_eq!(
            results[0], results[1],
            "closed stdout: C vs Rust differ for arg {arg:?}"
        );
    }
}

// ------------------------------------------------------------------ row 32 ---

#[test]
fn cfg_locale_env() {
    // The program never calls setlocale(), so `strtol` always works in the "C"
    // locale; the environment must not change anything.
    let envs: [&[(&str, &str)]; 6] = [
        &[],
        &[("LC_ALL", "C")],
        &[("LC_ALL", "en_US.UTF-8")],
        &[("LC_ALL", "de_DE.UTF-8"), ("LANG", "de_DE.UTF-8")],
        &[("LC_NUMERIC", "de_DE.UTF-8")],
        &[("LANG", "C.UTF-8"), ("LC_NUMERIC", "fr_FR.UTF-8")],
    ];
    for env in envs {
        for arg in [
            &b"1234"[..],
            b"1.234",
            b"1,234",
            b"-9",
            b"\xa0 1",
            b"",
            b"  12",
        ] {
            let args = vec![arg.to_vec()];
            let c = run_exe_full(&c_exe(TAG), &args, None, Some(env));
            let r = run_exe_full(&rust_exe(), &args, None, Some(env));
            common::assert_same(&c, &r, &args);
        }
    }
}

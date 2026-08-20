//! Phase B — valid-path differential tests for the `main` entry point
//! (CONFIGS.md rows C13..C26).
//!
//! The lower level entry points (`driver`, `print_foo`, rows C1..C12 and C27)
//! are covered by `tests/ffi_inproc.rs`.
//!
//! Every test drives BOTH implementations through their built artifacts (the
//! two shared objects via `libloading`, and the two executables) and compares
//! the results byte for byte.

mod common;

use common::*;

// ===========================================================================
// main — stdin driven
// ===========================================================================

fn tok_sep(rng: &mut Rng) -> &'static str {
    *rng.pick(&[
        " ", "\n", "\t", "\r", "\u{b}", "\u{c}", "  ", " \n ", "\n\n", "\t\t ", " \r\n\t",
    ])
}

fn magnitude(rng: &mut Rng, class: u32) -> String {
    match class {
        0 => format!("{}", rng.below(10)),
        1 => format!("{}", rng.next_u32() % (1u32 << 31)),
        2 => format!("{}", (1u64 << 31) + rng.below(1u64 << 31)),
        3 => format!("{}", (1u64 << 32) + rng.below(u64::MAX - (1u64 << 32))),
        4 => format!("{}", (1u128 << 63) + u128::from(rng.next_u64() >> 1)),
        5 => format!("{}", u128::from(u64::MAX) + 1 + u128::from(rng.next_u64())),
        6 => {
            // far beyond ULONG_MAX: 25..60 digits
            let n = 25 + rng.below(36) as usize;
            let mut s = String::new();
            for i in 0..n {
                let d = rng.below(10) as u8;
                let d = if i == 0 && d == 0 { 1 } else { d };
                s.push((b'0' + d) as char);
            }
            s
        }
        _ => format!("{}", u32::MAX),
    }
}

fn sign(rng: &mut Rng) -> &'static str {
    *rng.pick(&["", "+", "-"])
}

/// C13: `.so` `main`, four well formed tokens, single spaces.
#[test]
fn cfg_c13_so_main_simple() {
    for input in [
        &b"0 0 0 0\n"[..],
        b"1 2 3 4\n",
        b"3 7 1 -5\n",
        b"4 8 2 100\n",
        b"5 6 7 8",
        b"4294967295 4294967295 1 -2147483648\n",
    ] {
        assert_so_main_same(input, "C13 .so main simple");
    }
}

/// C14: `.so` `main`, randomized tokens and separators.
#[test]
fn cfg_c14_so_main_random() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..150 {
        let mut s = String::new();
        for _ in 0..4 {
            let cls = rng.below(8) as u32;
            s.push_str(sign(&mut rng));
            s.push_str(&magnitude(&mut rng, cls));
            s.push_str(tok_sep(&mut rng));
        }
        assert_so_main_same(s.as_bytes(), "C14 .so main random");
    }
}

/// C15: 0..8 tokens, with and without a trailing newline.
#[test]
fn cfg_c15_exe_token_counts() {
    let mut rng = Rng::new(SEED ^ 15);
    for n in [0usize, 1, 2, 3, 4, 5, 8] {
        for trailing in ["", "\n"] {
            for _ in 0..20 {
                let mut toks = Vec::new();
                for _ in 0..n {
                    let cls = rng.below(3) as u32;
                    let sg = sign(&mut rng);
                    toks.push(format!("{}{}", sg, magnitude(&mut rng, cls)));
                }
                let s = format!("{}{}", toks.join(" "), trailing);
                assert_exe_same(s.as_bytes(), "C15 token counts");
            }
        }
    }
}

/// C16: every C white-space byte as a separator, singly and in runs.
#[test]
fn cfg_c16_exe_separators() {
    let ws = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    // single separator kinds
    for &w in &ws {
        let mut v = Vec::new();
        for (i, t) in ["12", "34", "56", "78"].iter().enumerate() {
            if i > 0 {
                v.push(w);
            }
            v.extend_from_slice(t.as_bytes());
        }
        assert_exe_same(&v, "C16 single separator");
    }
    // runs of mixed white space, also leading and trailing
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..200 {
        let mut v = Vec::new();
        for _ in 0..rng.below(5) {
            v.push(*rng.pick(&ws));
        }
        for i in 0..4 {
            if i > 0 {
                for _ in 0..=rng.below(4) {
                    v.push(*rng.pick(&ws));
                }
            }
            let cls = rng.below(3) as u32;
            v.extend_from_slice(magnitude(&mut rng, cls).as_bytes());
        }
        for _ in 0..rng.below(4) {
            v.push(*rng.pick(&ws));
        }
        assert_exe_same(&v, "C16 separator runs");
    }
    // all six, in every order, as the only separator between two tokens
    for &a in &ws {
        for &b in &ws {
            let v = vec![b'1', a, b, b'2', a, b'3', b, b'4'];
            assert_exe_same(&v, "C16 separator pairs");
        }
    }
}

/// C17: all 81 sign-form combinations.
#[test]
fn cfg_c17_exe_sign_forms() {
    let signs = ["", "+", "-"];
    let mut rng = Rng::new(SEED ^ 17);
    for &s0 in &signs {
        for &s1 in &signs {
            for &s2 in &signs {
                for &s3 in &signs {
                    let s = format!(
                        "{}{} {}{} {}{} {}{}\n",
                        s0,
                        magnitude(&mut rng, 1),
                        s1,
                        magnitude(&mut rng, 1),
                        s2,
                        magnitude(&mut rng, 0),
                        s3,
                        magnitude(&mut rng, 1)
                    );
                    assert_exe_same(s.as_bytes(), "C17 sign forms");
                }
            }
        }
    }
}

/// C18: cross product of magnitude classes over the four conversions.
#[test]
fn cfg_c18_exe_magnitude_classes() {
    let mut rng = Rng::new(SEED ^ 18);
    // 7 classes on 4 positions is 2401 combinations; sample the full cross
    // product of the first two positions exhaustively and randomize the rest.
    for c0 in 0..7u32 {
        for c1 in 0..7u32 {
            for _ in 0..3 {
                let c2 = rng.below(7) as u32;
                let c3 = rng.below(7) as u32;
                let s = format!(
                    "{}{} {}{} {}{} {}{}\n",
                    sign(&mut rng),
                    magnitude(&mut rng, c0),
                    sign(&mut rng),
                    magnitude(&mut rng, c1),
                    sign(&mut rng),
                    magnitude(&mut rng, c2),
                    sign(&mut rng),
                    magnitude(&mut rng, c3)
                );
                assert_exe_same(s.as_bytes(), "C18 magnitude classes");
            }
        }
    }
}

/// C19: leading zeros.
#[test]
fn cfg_c19_exe_leading_zeros() {
    let mut rng = Rng::new(SEED ^ 19);
    for zeros in 1..=30usize {
        let z = "0".repeat(zeros);
        let s = format!(
            "{z}{} {z}{} {z}{} -{z}{}\n",
            rng.below(1000),
            rng.below(1000),
            rng.below(2),
            rng.below(100000)
        );
        assert_exe_same(s.as_bytes(), "C19 leading zeros");
        // all-zero tokens too
        let s2 = format!("{z} {z} {z} {z}\n");
        assert_exe_same(s2.as_bytes(), "C19 all-zero tokens");
    }
}

/// C20: the full 4×8×2 bit-field outcome matrix, driven from stdin.
#[test]
fn cfg_c20_exe_bitfield_matrix() {
    let mut rng = Rng::new(SEED ^ 20);
    for xr in 0..4u32 {
        for yr in 0..8u32 {
            for bnz in 0..2u32 {
                // pick values with the wanted residues but random multiples
                let x = xr + 4 * (rng.next_u32() % 1000);
                let y = yr + 8 * (rng.next_u32() % 1000);
                let b = if bnz == 0 {
                    0i64
                } else {
                    1 + (rng.below(1000) as i64) * if rng.below(2) == 0 { 1 } else { -1 }
                };
                let z = rng.next_u32() as i32;
                let s = format!("{x} {y} {b} {z}\n");
                assert_exe_same(s.as_bytes(), "C20 bit-field matrix");
            }
        }
    }
}

/// C21: tokens straddling the 4096/8192 byte reader buffer boundary.
#[test]
fn cfg_c21_exe_buffer_boundary() {
    for off in [
        1usize, 2, 4090, 4094, 4095, 4096, 4097, 4098, 4100, 8190, 8191, 8192, 8193, 12287, 12288,
    ] {
        // leading white space run ending near the boundary
        let mut v = vec![b' '; off];
        v.extend_from_slice(b"12345 67 1 -42\n");
        assert_exe_same(&v, "C21 leading ws straddle");

        // a long digit run straddling the boundary
        let mut v = Vec::new();
        v.extend_from_slice(b"1 ");
        v.extend(std::iter::repeat(b'0').take(off));
        v.extend_from_slice(b"5 3 1 2\n");
        assert_exe_same(&v, "C21 digit run straddle");

        // a huge token whose digits cross the boundary (overflow path)
        let mut v = Vec::new();
        v.extend(std::iter::repeat(b'9').take(off));
        v.extend_from_slice(b" 1 2 3\n");
        assert_exe_same(&v, "C21 nines straddle");

        // pushback exactly at the boundary
        let mut v = vec![b'7'; off];
        v.extend_from_slice(b"x 1 2 3\n");
        assert_exe_same(&v, "C21 pushback straddle");
    }
}

/// C22: the different kinds of stdin.
#[test]
fn cfg_c22_exe_stdin_kinds() {
    use std::process::{Command, Stdio};
    let payload = b"5 6 7 8\n";

    // regular file
    let dir = std::env::temp_dir().join(format!("driver-c22-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("in.txt");
    std::fs::write(&f, payload).unwrap();
    for (label, mk) in [
        ("file", 0),
        ("devnull", 1),
        ("empty-file", 2),
        ("directory", 3),
    ] {
        let run = |prog: &std::path::Path| {
            let stdin = match mk {
                0 => Stdio::from(std::fs::File::open(&f).unwrap()),
                1 => Stdio::from(std::fs::File::open("/dev/null").unwrap()),
                2 => {
                    let e = dir.join("empty");
                    std::fs::write(&e, b"").unwrap();
                    Stdio::from(std::fs::File::open(&e).unwrap())
                }
                _ => Stdio::from(std::fs::File::open(&dir).unwrap()),
            };
            Command::new(prog)
                .stdin(stdin)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .unwrap()
        };
        let c = run(&c_exe());
        let r = run(&rust_exe());
        assert_eq!(
            (c.status.code(), &c.stdout),
            (r.status.code(), &r.stdout),
            "C22 stdin kind {label}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);

    // pipe (the default in every other test)
    assert_exe_same(payload, "C22 pipe");
}

/// C23: input arriving in several separate writes on a pipe.
#[test]
fn cfg_c23_exe_partial_reads() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let chunks: Vec<Vec<Vec<u8>>> = vec![
        vec![b"1".to_vec(), b"2 3".to_vec(), b" 1 9\n".to_vec()],
        vec![b"12".to_vec(), b"34".to_vec(), b"5 6 7 8\n".to_vec()],
        vec![b" ".to_vec(), b" ".to_vec(), b"7".to_vec(), b" 7 7 7".to_vec()],
        vec![b"429496729".to_vec(), b"6 1 1 1\n".to_vec()],
    ];
    for (i, chunk) in chunks.iter().enumerate() {
        let run = |prog: &std::path::Path| {
            let mut child = Command::new(prog)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            {
                let si = child.stdin.as_mut().unwrap();
                for part in chunk {
                    let _ = si.write_all(part);
                    let _ = si.flush();
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }
            }
            child.wait_with_output().unwrap()
        };
        let c = run(&c_exe());
        let r = run(&rust_exe());
        assert_eq!(
            (c.status.code(), &c.stdout),
            (r.status.code(), &r.stdout),
            "C23 partial reads #{i}"
        );
    }
}

/// C24: randomized soup over the alphabet the scanner actually branches on.
#[test]
fn cfg_c24_exe_random_soup() {
    let alphabet: &[u8] = b"0123456789+- \t\n\x0b\x0c\r.xeaA";
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..4000 {
        let n = rng.below(40) as usize;
        let v: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        assert_exe_same(&v, "C24 random soup");
    }
}

/// C25: fully arbitrary bytes (NUL and >0x7f included).
#[test]
fn cfg_c25_exe_random_bytes() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..2000 {
        let n = rng.below(64) as usize;
        let v: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        assert_exe_same(&v, "C25 random bytes");
    }
}

/// C26: the executable and the `dlopen`ed `main` agree, for both languages.
#[test]
fn cfg_c26_exe_matches_so() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut inputs: Vec<Vec<u8>> = vec![
        b"1 2 3 4\n".to_vec(),
        b"".to_vec(),
        b"   ".to_vec(),
        b"abc".to_vec(),
        b"-1 -2 -3 -4".to_vec(),
        b"18446744073709551616 1 1 1".to_vec(),
    ];
    for _ in 0..60 {
        let n = rng.below(30) as usize;
        inputs.push((0..n).map(|_| *rng.pick(b"0123456789+- \n")).collect());
    }
    for input in &inputs {
        // C .so main vs Rust .so main
        assert_so_main_same(input, "C26 .so main");
        // and each .so against its own executable
        let cso = c_shared_lib();
        let rso = rust_shared_lib();
        let runner = so_runner();
        let c_exe_run = run_with_stdin(&c_exe(), &[], input);
        let c_so_run = run_with_stdin(&runner, &[cso.to_str().unwrap(), "main"], input);
        assert_eq!(
            c_exe_run.stdout, c_so_run.stdout,
            "C26 C exe vs C .so for {:?}",
            Preview(input)
        );
        let r_exe_run = run_with_stdin(&rust_exe(), &[], input);
        let r_so_run = run_with_stdin(&runner, &[rso.to_str().unwrap(), "main"], input);
        assert_eq!(
            r_exe_run.stdout, r_so_run.stdout,
            "C26 Rust exe vs Rust .so for {:?}",
            Preview(input)
        );
        assert_eq!(
            c_exe_run.stdout, r_exe_run.stdout,
            "C26 C exe vs Rust exe for {:?}",
            Preview(input)
        );
    }
}

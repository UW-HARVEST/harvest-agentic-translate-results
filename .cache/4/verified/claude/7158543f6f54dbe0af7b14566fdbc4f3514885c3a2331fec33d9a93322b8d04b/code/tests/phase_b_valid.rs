//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives **both** implementations through their public boundary
//! (the `.so`'s exported symbols via `libloading`, or the executable's process
//! boundary) and compares the results byte-for-byte. The Rust code is never
//! called directly.
//!
//! All randomised rows use the fixed-seed xorshift64\* in `common::Rng`, so a
//! failure is exactly reproducible.

mod common;

use common::*;

/// Samples per randomised row (kept high enough to catch value-dependent bugs,
/// low enough that the whole suite finishes in seconds).
const N: usize = 256;

// ===========================================================================
// Rows 8–21: the composed pipeline `scanf -> driver -> printf`, driven through
// the executables' process boundary.
// ===========================================================================

/// Row 8 — empty input.
#[test]
fn row08_exe_empty_input() {
    assert_same_exe_all("row08", &[Vec::new()]);
}

/// Row 9 — a bare unsigned decimal integer, randomised across the `int` range.
#[test]
fn row09_exe_plain_integer() {
    let mut rng = Rng::new(0x9);
    let mut payloads = Vec::new();
    for _ in 0..N {
        let v = rng.next_u32() as u64 % 2_147_483_648;
        payloads.push(format!("{v}").into_bytes());
    }
    // plus the extremes of the unsigned range
    for v in ["0", "1", "2147483647", "2147483646"] {
        payloads.push(v.as_bytes().to_vec());
    }
    assert_same_exe_all("row09", &payloads);
}

/// Row 10 — explicit `+` / `-` sign.
#[test]
fn row10_exe_signed_integer() {
    let mut rng = Rng::new(0x10);
    let mut payloads = Vec::new();
    for _ in 0..N {
        let v = rng.next_u32() as u64 % 2_147_483_649;
        let sign = if rng.next_u64() & 1 == 0 { "-" } else { "+" };
        payloads.push(format!("{sign}{v}").into_bytes());
    }
    for v in ["-0", "+0", "-1", "+1", "-2147483648", "+2147483647"] {
        payloads.push(v.as_bytes().to_vec());
    }
    assert_same_exe_all("row10", &payloads);
}

/// Row 11 — every `isspace` byte as a leading-whitespace skip, singly and mixed
/// (including newlines, so `%d` has to cross lines).
#[test]
fn row11_exe_leading_whitespace_kinds() {
    const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    let mut payloads = Vec::new();
    for w in WS {
        for reps in [1usize, 2, 5] {
            let mut p = vec![w; reps];
            p.extend_from_slice(b"42");
            payloads.push(p);
        }
    }
    let mut rng = Rng::new(0x11);
    for _ in 0..N {
        let n = 1 + rng.below(20) as usize;
        let mut p: Vec<u8> = (0..n).map(|_| *rng.pick(&WS)).collect();
        let v = rng.next_u32() as i32;
        p.extend_from_slice(format!("{v}").as_bytes());
        // random trailing whitespace too
        let m = rng.below(4) as usize;
        for _ in 0..m {
            p.push(*rng.pick(&WS));
        }
        payloads.push(p);
    }
    assert_same_exe_all("row11", &payloads);
}

/// Row 12 — whitespace runs that cross glibc's `stdin` block size and Rust's
/// `BufReader` capacity.
#[test]
fn row12_exe_long_whitespace_run() {
    const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    let mut rng = Rng::new(0x12);
    let mut payloads = Vec::new();
    for len in [1usize, 4095, 4096, 4097, 8191, 8192, 8193, 8200] {
        let mut p = vec![b' '; len];
        p.extend_from_slice(b"-12345");
        payloads.push(p);
    }
    for _ in 0..16 {
        let len = 1 + rng.below(8200) as usize;
        let mut p: Vec<u8> = (0..len).map(|_| *rng.pick(&WS)).collect();
        p.extend_from_slice(format!("{}", rng.next_i32()).as_bytes());
        payloads.push(p);
    }
    assert_same_exe_all("row12", &payloads);
}

/// Row 13 — leading zeros (glibc's leading-`0` base-prefix probe).
#[test]
fn row13_exe_leading_zeros() {
    let mut rng = Rng::new(0x13);
    let mut payloads = Vec::new();
    for zeros in [1usize, 2, 3, 19, 20, 21, 100, 300] {
        for v in ["0", "5", "42", "2147483647", "9223372036854775808"] {
            let mut p = vec![b'0'; zeros];
            p.extend_from_slice(v.as_bytes());
            payloads.push(p.clone());
            let mut q = vec![b'-'];
            q.extend_from_slice(&p);
            payloads.push(q);
        }
    }
    for _ in 0..N {
        let zeros = 1 + rng.below(40) as usize;
        let mut p = vec![b'0'; zeros];
        p.extend_from_slice(format!("{}", rng.next_u32()).as_bytes());
        payloads.push(p);
    }
    assert_same_exe_all("row13", &payloads);
}

/// Row 14 — exact digit-count sweep from 1 to 25 digits.
#[test]
fn row14_exe_digit_count_sweep() {
    let mut rng = Rng::new(0x14);
    let mut payloads = Vec::new();
    for digits in 1..=25usize {
        for _ in 0..8 {
            let mut s = String::new();
            // first digit 1..9 so the length is exactly `digits`
            s.push((b'1' + rng.below(9) as u8) as char);
            for _ in 1..digits {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
            payloads.push(s.clone().into_bytes());
            payloads.push(format!("-{s}").into_bytes());
        }
    }
    assert_same_exe_all("row14", &payloads);
}

/// Row 15 — magnitudes above `int` but inside `long`: the accumulator is a
/// `long`, and only the low 32 bits reach `x`.
#[test]
fn row15_exe_long_range_truncation() {
    let mut rng = Rng::new(0x15);
    let mut payloads = Vec::new();
    for v in [
        2_147_483_648i64,
        2_147_483_649,
        4_294_967_295,
        4_294_967_296,
        4_294_967_297,
        8_589_934_592,
        i64::MAX,
        i64::MAX - 1,
    ] {
        payloads.push(format!("{v}").into_bytes());
        payloads.push(format!("-{v}").into_bytes());
    }
    for _ in 0..N {
        let v = rng.range(2_147_483_648, i64::MAX);
        payloads.push(format!("{v}").into_bytes());
        payloads.push(format!("-{v}").into_bytes());
    }
    assert_same_exe_all("row15", &payloads);
}

/// Row 16 — magnitudes past `LONG_MAX`, where `strtol` clamps.
#[test]
fn row16_exe_strtol_clamp() {
    let mut rng = Rng::new(0x16);
    let mut payloads = Vec::new();
    for _ in 0..N {
        let digits = 20 + rng.below(21) as usize;
        let mut s = String::new();
        s.push((b'1' + rng.below(9) as u8) as char);
        for _ in 1..digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        payloads.push(s.clone().into_bytes());
        payloads.push(format!("-{s}").into_bytes());
    }
    // A number far larger than any internal buffer.
    payloads.push(vec![b'9'; 1_000_000]);
    let mut huge = vec![b'-'];
    huge.extend_from_slice(&vec![b'7'; 1_000_000]);
    payloads.push(huge);
    assert_same_exe_all("row16", &payloads);
}

/// Row 17 — the exact `int` / `unsigned` / `long` boundary literals.
#[test]
fn row17_exe_boundary_literals() {
    let lits = [
        "2147483646",
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "4294967294",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967295",
        "-4294967296",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "18446744073709551615",
        "18446744073709551616",
    ];
    let payloads: Vec<Vec<u8>> = lits.iter().map(|s| s.as_bytes().to_vec()).collect();
    assert_same_exe_all("row17", &payloads);
}

/// Row 18 — what terminates the digit loop.
#[test]
fn row18_exe_terminator_classes() {
    let mut payloads = Vec::new();
    let terms: [&str; 14] = [
        "", "\n", " ", "\t", "\r", "a", "Z", ".", ",", "-", "+", "e", "x", "\0",
    ];
    for t in terms {
        for v in ["0", "7", "42", "-42", "+42", "2147483648", "0042"] {
            payloads.push(format!("{v}{t}").into_bytes());
            payloads.push(format!("{v}{t}rest 99").into_bytes());
        }
    }
    assert_same_exe_all("row18", &payloads);
}

/// Row 19 — several numbers on `stdin`; only the first must be consumed.
#[test]
fn row19_exe_multiple_numbers() {
    let mut rng = Rng::new(0x19);
    let mut payloads = Vec::new();
    for _ in 0..N {
        let count = 2 + rng.below(4) as usize;
        let sep = *rng.pick(&[" ", "\n", "\t", "  \n ", "\r\n"]);
        let nums: Vec<String> = (0..count).map(|_| format!("{}", rng.next_i32())).collect();
        payloads.push(nums.join(sep).into_bytes());
    }
    assert_same_exe_all("row19", &payloads);
}

/// Row 20 — completely unbiased random byte soup.
#[test]
fn row20_exe_random_byte_soup() {
    let mut rng = Rng::new(0x20);
    let mut payloads = Vec::new();
    for _ in 0..1024 {
        let len = rng.below(65) as usize;
        payloads.push((0..len).map(|_| rng.byte()).collect::<Vec<u8>>());
    }
    assert_same_exe_all("row20", &payloads);
}

/// Row 21 — randomised "almost a number" strings built from the tokens the
/// conversion has to reject or stop at.
#[test]
fn row21_exe_almost_numbers() {
    const TOKENS: [&str; 24] = [
        "-", "+", "--", "++", "+-", "-+", " ", "\t", "\n", "0", "0x", "0X", "0b", "1", "9", "42",
        ".", ",", "e", "E", "'", "٣", "０", "\u{a0}",
    ];
    let mut rng = Rng::new(0x21);
    let mut payloads = Vec::new();
    for _ in 0..N * 2 {
        let n = 1 + rng.below(6) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(*rng.pick(&TOKENS));
        }
        payloads.push(s.into_bytes());
    }
    // Hand-picked shapes that exercise a specific glibc branch.
    for s in [
        "0x10", "0X10", "0x", "08", "09", "0 8", "1,000", "1.5", "1e5", "- 42", "+ 42", "--5",
        "++5", "1'000", "٣", "０", "\u{a0}42", "\u{feff}42",
    ] {
        payloads.push(s.as_bytes().to_vec());
    }
    assert_same_exe_all("row21", &payloads);
}

// ===========================================================================
// Rows 22–23: the same payloads through different kernel plumbing.
// ===========================================================================

/// Row 22 — `stdin` as a pipe vs a regular file vs `/dev/null`.
#[test]
fn row22_exe_stdin_pipe_vs_file() {
    let c = c_exe();
    let r = rust_exe_release();
    let mut rng = Rng::new(0x22);
    let mut payloads: Vec<Vec<u8>> = vec![Vec::new(), b"42".to_vec(), b"  \n -7 x".to_vec()];
    for _ in 0..N {
        let len = rng.below(48) as usize;
        payloads.push((0..len).map(|_| rng.byte()).collect());
    }
    for _ in 0..32 {
        payloads.push(format!("{}", rng.next_i32()).into_bytes());
    }
    for p in &payloads {
        for sk in [StdinKind::File, StdinKind::Pipe, StdinKind::DevNull] {
            let co = run_cfg(&c, &[], p, sk, StdoutKind::Pipe, &[]);
            let ro = run_cfg(&r, &[], p, sk, StdoutKind::Pipe, &[]);
            assert_same_outcome(
                "row22",
                &format!("stdin={sk:?} payload={}", show(p)),
                &co,
                &ro,
            );
        }
    }
}

/// Row 23 — `stdout` to a pipe vs a regular file (glibc full buffering vs
/// Rust's `LineWriter`): the produced bytes must be identical either way.
#[test]
fn row23_exe_stdout_file_vs_pipe() {
    let c = c_exe();
    let r = rust_exe_release();
    let mut rng = Rng::new(0x23);
    let mut payloads: Vec<Vec<u8>> = vec![Vec::new(), b"1073741824".to_vec()];
    for _ in 0..N {
        payloads.push(format!("{}", rng.next_i32()).into_bytes());
    }
    for p in &payloads {
        let mut seen: Option<Vec<u8>> = None;
        for ok in [StdoutKind::Pipe, StdoutKind::File] {
            let co = run_cfg(&c, &[], p, StdinKind::File, ok, &[]);
            let ro = run_cfg(&r, &[], p, StdinKind::File, ok, &[]);
            assert_same_outcome(
                "row23",
                &format!("stdout={ok:?} payload={}", show(p)),
                &co,
                &ro,
            );
            // The bytes must not depend on the destination kind either.
            match &seen {
                None => seen = Some(co.stdout.clone()),
                Some(prev) => assert_eq!(
                    prev,
                    &co.stdout,
                    "[row23] C output differs between stdout kinds for {}",
                    show(p)
                ),
            }
        }
    }
}

// ===========================================================================
// Row 24: the shared object's exported `main`, called by an external loader.
// ===========================================================================

/// Row 29 — how much of `stdin` the program consumes, as seen by a *second*
/// reader sharing file descriptor 0 (`{ ./driver; cat; } < file`).
///
/// glibc rewinds a seekable `stdin` to the first byte the conversion did not
/// consume when it cleans the stream up at exit, and pushes the terminating
/// character back with `ungetc`; on a pipe it cannot rewind, so exactly the
/// bytes of one `st_blksize` read are swallowed. Both must match.
#[test]
fn row29_exe_shared_stdin_leftovers() {
    let c = c_exe();
    let r = rust_exe_release();
    let mut rng = Rng::new(0x29);

    let mut payloads: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"42".to_vec(),
        b"42rest".to_vec(),
        b"  42XY".to_vec(),
        b"abcdef".to_vec(),
        b"-abc".to_vec(),
        b"-42Z".to_vec(),
        b"+42Q".to_vec(),
        b"0042Q".to_vec(),
        b"0x10Y".to_vec(),
        b"1,000x".to_vec(),
        b"   ".to_vec(),
        b"\n\n42\n\n99\n".to_vec(),
        b"999999999999999999999Z".to_vec(),
        b"\0 42".to_vec(),
    ];
    // Sizes around glibc's block size and Rust's old BufReader capacity.
    for len in [1usize, 4095, 4096, 4097, 8191, 8192, 8193, 20_000, 50_000] {
        let mut p = b"42 ".to_vec();
        p.extend(std::iter::repeat(b'A').take(len));
        payloads.push(p);
    }
    for _ in 0..64 {
        let len = rng.below(6000) as usize;
        let mut p = format!("{}", rng.next_i32()).into_bytes();
        p.extend((0..len).map(|_| rng.byte()));
        payloads.push(p);
    }

    for p in &payloads {
        let (cout, cleft) = leftover_via_file(&c, p);
        let (rout, rleft) = leftover_via_file(&r, p);
        assert_eq!(
            show(&cout),
            show(&rout),
            "[row29/file] stdout differs for {}",
            show(p)
        );
        assert_eq!(
            cleft.len(),
            rleft.len(),
            "[row29/file] leftover length differs for {} (C kept {} bytes, Rust {})",
            show(p),
            cleft.len(),
            rleft.len()
        );
        assert_eq!(
            show(&cleft),
            show(&rleft),
            "[row29/file] leftover bytes differ for {}",
            show(p)
        );

        if p.len() < 60_000 {
            let (cout, cleft) = leftover_via_pipe(&c, p);
            let (rout, rleft) = leftover_via_pipe(&r, p);
            assert_eq!(
                show(&cout),
                show(&rout),
                "[row29/pipe] stdout differs for {}",
                show(p)
            );
            assert_eq!(
                cleft.len(),
                rleft.len(),
                "[row29/pipe] leftover length differs for {} (C kept {}, Rust {})",
                show(p),
                cleft.len(),
                rleft.len()
            );
            assert_eq!(
                show(&cleft),
                show(&rleft),
                "[row29/pipe] leftover bytes differ for {}",
                show(p)
            );
        }
    }
}

/// Row 30 — consecutive runs sharing one `stdin` offset, a non-zero starting
/// offset, and character-device inputs.
///
/// `{ ./driver; ./driver; ./driver; } < "42 99 7"` must print `384 498 314`:
/// each run may consume only its own number. This is the sharpest available
/// check on the read granularity and the exit-time seek.
#[test]
fn row30_exe_sequential_runs_and_offsets() {
    let c = c_exe();
    let r = rust_exe_release();
    let mut rng = Rng::new(0x30);

    let mut cases: Vec<(Vec<u8>, usize, u64)> = vec![
        (b"42 99 7\n".to_vec(), 3, 0),
        (b"1 2 3 4 5".to_vec(), 5, 0),
        (b"  -1\n-2\t-3  ".to_vec(), 3, 0),
        (b"XXXXX77tail".to_vec(), 1, 5),
        (b"XXXXX77 88tail".to_vec(), 2, 5),
        (b"abc 42".to_vec(), 2, 0),
        (b"".to_vec(), 2, 0),
        (b"7".to_vec(), 3, 0),
        (b"0x10 5".to_vec(), 2, 0),
        (b"1,2,3".to_vec(), 3, 0),
    ];
    for _ in 0..32 {
        let count = 1 + rng.below(4) as usize;
        let nums: Vec<String> = (0..count + 1)
            .map(|_| format!("{}", rng.range(-1_000_000, 1_000_000)))
            .collect();
        let sep = *rng.pick(&[" ", "\n", "\t", "  ", "\r\n"]);
        cases.push((nums.join(sep).into_bytes(), count, 0));
    }
    // A payload longer than one glibc block, so a naive reader would swallow
    // every later number.
    let mut long = String::new();
    for i in 0..3000 {
        long.push_str(&format!("{} ", i % 1000));
    }
    cases.push((long.into_bytes(), 6, 0));

    for (payload, runs, offset) in &cases {
        let co = shared_runs(&c, payload, *runs, *offset);
        let ro = shared_runs(&r, payload, *runs, *offset);
        let cs: Vec<String> = co.iter().map(|v| show(v)).collect();
        let rs: Vec<String> = ro.iter().map(|v| show(v)).collect();
        assert_eq!(
            cs,
            rs,
            "[row30] {runs} shared run(s) from offset {offset} of {} diverged",
            show(payload)
        );
    }

    // The canonical expectation, pinned to the C's actual output.
    let seq = shared_runs(&c, b"42 99 7\n", 3, 0);
    assert_eq!(
        seq.iter().take(3).map(|v| show(v)).collect::<Vec<_>>(),
        vec!["384\\n", "498\\n", "314\\n"],
        "[row30] C reference behaviour for sequential runs changed"
    );

    // Character devices: no payload can emulate these.
    for dev in ["/dev/zero", "/dev/null"] {
        let co = run_stdin_path(&c, dev);
        let ro = run_stdin_path(&r, dev);
        assert_same_outcome("row30", &format!("stdin={dev}"), &co, &ro);
    }

    // Command-line arguments are ignored by `int main()`.
    let co = run_args(&c, &["a", "b", "-1"], b"5");
    let ro = run_args(&r, &["a", "b", "-1"], b"5");
    assert_same_outcome("row30", "argv ignored", &co, &ro);
}

/// Row 24 — `dlsym(handle, "main")` from a separate C consumer process, so the
/// exported `main` of both `.so`s is exercised with a pristine `stdin`.
#[test]
fn row24_so_main_via_external_loader() {
    let loader = loader_exe();
    let cso = c_so().to_string_lossy().to_string();
    let rso = rust_so().to_string_lossy().to_string();
    let mut rng = Rng::new(0x24);

    let mut payloads: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"0".to_vec(),
        b"42".to_vec(),
        b"-42".to_vec(),
        b"+42".to_vec(),
        b"  \n\t 42".to_vec(),
        b"abc".to_vec(),
        b"-".to_vec(),
        b"0x10".to_vec(),
        b"1,000".to_vec(),
        b"2147483648".to_vec(),
        b"9223372036854775808".to_vec(),
        b"-9223372036854775809".to_vec(),
        b"1073741824".to_vec(),
        b"42abc".to_vec(),
        b"\0 42".to_vec(),
    ];
    for _ in 0..N {
        let len = rng.below(40) as usize;
        payloads.push((0..len).map(|_| rng.byte()).collect());
    }
    for _ in 0..N {
        payloads.push(format!("{}", rng.next_i32()).into_bytes());
    }

    for p in &payloads {
        let co = run_args(&loader, &[&cso, "main"], p);
        let ro = run_args(&loader, &[&rso, "main"], p);
        assert_same_outcome("row24", &format!("so main payload={}", show(p)), &co, &ro);
    }
}

/// Row 24b — the same loader used for the `driver` export, as an independent
/// cross-check of the in-process `libloading` results.
#[test]
fn row24b_so_driver_via_external_loader() {
    let loader = loader_exe();
    let cso = c_so().to_string_lossy().to_string();
    let rso = rust_so().to_string_lossy().to_string();
    let mut rng = Rng::new(0x24b);
    let mut xs: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, 1_073_741_824, -1_073_741_824];
    for _ in 0..64 {
        xs.push(rng.next_i32());
    }
    for x in xs {
        let arg = x.to_string();
        let co = run_args(&loader, &[&cso, "driver", &arg], b"");
        let ro = run_args(&loader, &[&rso, "driver", &arg], b"");
        assert_same_outcome("row24b", &format!("so driver({x})"), &co, &ro);
    }
}

// ===========================================================================
// Rows 25–28: build configurations.
// ===========================================================================

/// Row 25 — the Rust `dev` profile (overflow checks **on**) must behave exactly
/// like the `release` profile and like C. This is where a non-wrapping
/// arithmetic translation would panic instead of wrapping.
#[test]
fn row25_rust_dev_profile_matches() {
    let c = c_exe();
    let dev = rust_exe_dev();
    let rel = rust_exe_release();
    let mut rng = Rng::new(0x25);
    let mut payloads: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"1073741824".to_vec(),
        b"1073741823".to_vec(),
        b"-1073741824".to_vec(),
        b"2147483647".to_vec(),
        b"-2147483648".to_vec(),
        b"9223372036854775808".to_vec(),
        b"-9223372036854775809".to_vec(),
        b"99999999999999999999".to_vec(),
        b"abc".to_vec(),
    ];
    for _ in 0..N {
        payloads.push(format!("{}", rng.next_i32()).into_bytes());
    }
    for _ in 0..64 {
        let digits = 1 + rng.below(30) as usize;
        let mut s = String::new();
        for _ in 0..digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        payloads.push(s.into_bytes());
    }
    for p in &payloads {
        let co = run(&c, p);
        let dv = run(&dev, p);
        let rl = run(&rel, p);
        assert_same_outcome("row25", &format!("dev payload={}", show(p)), &co, &dv);
        assert_same_outcome("row25", &format!("release payload={}", show(p)), &co, &rl);
    }
}

/// Row 26 — every `CMAKE_BUILD_TYPE` flavour of the C program must agree with
/// the Rust program (the `2*x` signed overflow is UB, so this pins it down).
#[test]
fn row26_c_build_types_match() {
    let r = rust_exe_release();
    let mut rng = Rng::new(0x26);
    let mut payloads: Vec<Vec<u8>> = vec![
        b"1073741824".to_vec(),
        b"1073741823".to_vec(),
        b"-1073741824".to_vec(),
        b"-1073741823".to_vec(),
        b"2147483647".to_vec(),
        b"-2147483648".to_vec(),
        b"1073741674".to_vec(),
        b"1073741673".to_vec(),
    ];
    for _ in 0..64 {
        payloads.push(format!("{}", rng.next_i32()).into_bytes());
    }
    for (flavour, _) in C_BUILD_TYPES {
        let c = c_exe_flavour(flavour);
        for p in &payloads {
            let co = run(&c, p);
            let ro = run(&r, p);
            assert_same_outcome(
                "row26",
                &format!("CMAKE_BUILD_TYPE={flavour} payload={}", show(p)),
                &co,
                &ro,
            );
        }
    }
}

/// Row 27 — the C never calls `setlocale`, so locale environment variables must
/// change nothing on either side (in particular `isspace` and digit grouping).
#[test]
fn row27_locale_env_is_irrelevant() {
    let c = c_exe();
    let r = rust_exe_release();
    let locales: [&[(&str, Option<&str>)]; 5] = [
        &[("LC_ALL", None), ("LC_NUMERIC", None), ("LANG", None)],
        &[("LC_ALL", Some("C"))],
        &[("LC_ALL", Some("en_US.UTF-8"))],
        &[("LC_NUMERIC", Some("de_DE.UTF-8")), ("LC_ALL", None)],
        &[("LC_ALL", Some("POSIX")), ("LANG", Some("C.UTF-8"))],
    ];
    let payloads: Vec<Vec<u8>> = [
        "42", "1,000", "1.000", "1 000", "-7", "  \u{a0}42", "٣", "2147483648", "",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();

    for env in locales {
        for p in &payloads {
            let co = run_cfg(&c, &[], p, StdinKind::File, StdoutKind::Pipe, env);
            let ro = run_cfg(&r, &[], p, StdinKind::File, StdoutKind::Pipe, env);
            assert_same_outcome(
                "row27",
                &format!("env={env:?} payload={}", show(p)),
                &co,
                &ro,
            );
        }
    }
}

/// Row 28 — the crate declares no Cargo features, so the single valid feature
/// combination is the empty one. This test documents/asserts that fact so the
/// claim cannot silently rot when a feature is added later.
#[test]
fn row28_single_feature_combination() {
    let manifest = std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("read manifest");
    let mut in_features = false;
    let mut declared = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_features = t == "[features]";
            continue;
        }
        if in_features && !t.is_empty() && !t.starts_with('#') {
            declared.push(t.to_string());
        }
    }
    assert!(
        declared.is_empty(),
        "Cargo features were added ({declared:?}); Phases B and C must now be \
         run for every combination — see CONFIGS.md row 28"
    );
}

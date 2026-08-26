//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row calls BOTH the C `.so` and the
//! Rust `.so` through `dlsym` and compares the returned line-pointer arrays
//! byte-for-byte, over many randomized inputs with a fixed seed.

mod common;

use common::{count_lines, diff, diff_read, model, Outcome, Rng};

/// Build a buffer from a list of line lengths.
///
/// Each line is `len` non-NUL bytes; every line is followed by a NUL except
/// optionally the last one when `terminate_last == false`.
fn build(rng: &mut Rng, lens: &[usize], terminate_last: bool, high_bit: bool) -> Vec<u8> {
    let mut v = Vec::new();
    for (i, &len) in lens.iter().enumerate() {
        for _ in 0..len {
            // any byte except NUL
            let b = if high_bit {
                0x80u8 | (rng.byte() & 0x7f)
            } else {
                let mut b = rng.byte();
                if b == 0 {
                    b = 1;
                }
                b
            };
            v.push(b);
        }
        let last = i + 1 == lens.len();
        if !last || terminate_last {
            v.push(0);
        }
    }
    v
}

fn check(buf: &mut [u8], num_lines: usize, buffer_size: usize, label: &str) -> Outcome {
    let expect = model(buf, num_lines, buffer_size);
    let got = diff(buf, num_lines, buffer_size, label);
    assert_eq!(
        got, expect,
        "{label}: both impls agreed but disagree with the C algorithm model \
         (numLines={num_lines}, bufferSize={buffer_size})"
    );
    got
}

fn assert_ok(o: &Outcome, label: &str) {
    assert!(
        matches!(o, Outcome::Ok(_)),
        "{label}: expected the success path, got {o:?} — this row would be vacuous"
    );
}

// ---------------------------------------------------------------- row 1
#[test]
fn cfg_01_zero_lines_zero_size_real_buffer() {
    let mut rng = Rng::new(0x1001);
    for _ in 0..200 {
        let n = 1 + rng.below(64);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let o = check(&mut buf, 0, 0, "cfg01");
        assert_eq!(o, Outcome::Ok(vec![]), "cfg01: malloc(0) must still succeed");
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn cfg_02_zero_lines_nonempty_buffer() {
    let mut rng = Rng::new(0x1002);
    for _ in 0..200 {
        let n = 1 + rng.below(128);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let size = 1 + rng.below(n);
        let o = check(&mut buf, 0, size, "cfg02");
        assert_eq!(o, Outcome::Ok(vec![]));
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn cfg_03_one_empty_line_size_one() {
    let mut buf = [0u8; 1];
    let o = check(&mut buf, 1, 1, "cfg03");
    assert_eq!(o, Outcome::Ok(vec![0]));
}

// ---------------------------------------------------------------- rows 4 + 5
#[test]
fn cfg_04_05_one_unterminated_byte_all_values() {
    // Every non-NUL byte value 1..=255, incl. the high-bit half (row 5): the C
    // compares a *signed* char against '\0', so 0x80..0xFF must not terminate.
    for b in 1u16..=255 {
        let mut buf = [b as u8; 1];
        let o = check(&mut buf, 1, 1, "cfg04/05");
        assert_eq!(o, Outcome::Ok(vec![0]), "byte 0x{b:02x}");
    }
    // and the NUL case for contrast
    let mut z = [0u8; 1];
    assert_eq!(check(&mut z, 1, 1, "cfg04/05-nul"), Outcome::Ok(vec![0]));
}

// ---------------------------------------------------------------- row 6
#[test]
fn cfg_06_one_terminated_line_exact_fit() {
    let mut rng = Rng::new(0x1006);
    for _ in 0..400 {
        let len = rng.below(40);
        let mut buf = build(&mut rng, &[len], true, false);
        let size = buf.len();
        let o = check(&mut buf, 1, size, "cfg06");
        assert_eq!(o, Outcome::Ok(vec![0]));
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn cfg_07_one_line_truncated_no_terminator() {
    let mut rng = Rng::new(0x1007);
    for _ in 0..400 {
        let len = 1 + rng.below(40);
        let mut buf = build(&mut rng, &[len], false, false);
        let size = buf.len();
        assert_eq!(size, len);
        let o = check(&mut buf, 1, size, "cfg07");
        assert_eq!(o, Outcome::Ok(vec![0]));
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn cfg_08_one_line_requested_with_slack() {
    let mut rng = Rng::new(0x1008);
    for _ in 0..400 {
        let lens: Vec<usize> = (0..1 + rng.below(5)).map(|_| rng.below(8)).collect();
        let mut buf = build(&mut rng, &lens, true, false);
        let size = buf.len();
        let o = check(&mut buf, 1, size, "cfg08");
        assert_ok(&o, "cfg08");
        assert_eq!(o, Outcome::Ok(vec![0]));
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn cfg_09_two_terminated_lines_exact_fit() {
    let mut rng = Rng::new(0x1009);
    for _ in 0..400 {
        let a = rng.below(16);
        let b = rng.below(16);
        let mut buf = build(&mut rng, &[a, b], true, false);
        let size = buf.len();
        let o = check(&mut buf, 2, size, "cfg09");
        assert_eq!(o, Outcome::Ok(vec![0, a as isize + 1]));
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn cfg_10_two_lines_last_unterminated() {
    let mut rng = Rng::new(0x1010);
    for _ in 0..400 {
        let a = rng.below(16);
        let b = 1 + rng.below(16);
        let mut buf = build(&mut rng, &[a, b], false, false);
        let size = buf.len();
        let o = check(&mut buf, 2, size, "cfg10");
        assert_eq!(o, Outcome::Ok(vec![0, a as isize + 1]));
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn cfg_11_many_terminated_lines_exact_fit() {
    let mut rng = Rng::new(0x1011);
    for _ in 0..600 {
        let n = 3 + rng.below(62);
        let lens: Vec<usize> = (0..n).map(|_| rng.below(12)).collect();
        let mut buf = build(&mut rng, &lens, true, false);
        let size = buf.len();
        let o = check(&mut buf, n, size, "cfg11");
        assert_ok(&o, "cfg11");
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn cfg_12_many_lines_last_unterminated() {
    let mut rng = Rng::new(0x1012);
    for _ in 0..600 {
        let n = 3 + rng.below(62);
        let mut lens: Vec<usize> = (0..n).map(|_| rng.below(12)).collect();
        // last line must be non-empty for "unterminated" to be meaningful
        let last = lens.len() - 1;
        lens[last] = 1 + rng.below(12);
        let mut buf = build(&mut rng, &lens, false, false);
        let size = buf.len();
        let o = check(&mut buf, n, size, "cfg12");
        assert_ok(&o, "cfg12");
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn cfg_13_leading_empty_line() {
    let mut rng = Rng::new(0x1013);
    // "\0abc\0" style: first line zero-length
    for _ in 0..400 {
        let n = 2 + rng.below(8);
        let mut lens: Vec<usize> = (0..n).map(|_| 1 + rng.below(6)).collect();
        lens[0] = 0;
        let mut buf = build(&mut rng, &lens, true, false);
        let size = buf.len();
        let o = check(&mut buf, n, size, "cfg13");
        match &o {
            Outcome::Ok(v) => assert_eq!(v[0], 0),
            _ => panic!("cfg13 expected success"),
        }
    }
    let mut fixed = b"\0abc\0".to_vec();
    assert_eq!(check(&mut fixed, 2, 5, "cfg13-fixed"), Outcome::Ok(vec![0, 1]));
}

// ---------------------------------------------------------------- row 14
#[test]
fn cfg_14_interior_consecutive_nuls() {
    let mut rng = Rng::new(0x1014);
    for _ in 0..600 {
        let n = 3 + rng.below(16);
        let mut lens: Vec<usize> = (0..n).map(|_| 1 + rng.below(6)).collect();
        // punch 1..3 zero-length lines somewhere in the interior
        let holes = 1 + rng.below(3);
        for _ in 0..holes {
            let idx = 1 + rng.below(n - 2);
            lens[idx] = 0;
        }
        let mut buf = build(&mut rng, &lens, true, false);
        let size = buf.len();
        assert_ok(&check(&mut buf, n, size, "cfg14"), "cfg14");
    }
    let mut fixed = b"a\0\0b\0".to_vec();
    assert_eq!(
        check(&mut fixed, 3, 5, "cfg14-fixed"),
        Outcome::Ok(vec![0, 2, 3])
    );
}

// ---------------------------------------------------------------- row 15
#[test]
fn cfg_15_trailing_extra_nul() {
    // "a\0\0": lines are "a" and "" -> exactly 2 lines in 3 bytes
    let mut fixed = b"a\0\0".to_vec();
    assert_eq!(check(&mut fixed, 1, 3, "cfg15-n1"), Outcome::Ok(vec![0]));
    assert_eq!(check(&mut fixed, 2, 3, "cfg15-n2"), Outcome::Ok(vec![0, 2]));

    let mut rng = Rng::new(0x1015);
    for _ in 0..400 {
        let n = 1 + rng.below(12);
        let mut lens: Vec<usize> = (0..n).map(|_| 1 + rng.below(6)).collect();
        lens.push(0); // trailing zero-length line
        let mut buf = build(&mut rng, &lens, true, false);
        let size = buf.len();
        assert_ok(&check(&mut buf, n + 1, size, "cfg15"), "cfg15");
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn cfg_16_all_nul_max_line_count() {
    for size in 1..=64usize {
        let mut buf = vec![0u8; size];
        let o = check(&mut buf, size, size, "cfg16");
        let expect: Vec<isize> = (0..size as isize).collect();
        assert_eq!(o, Outcome::Ok(expect), "size={size}");
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn cfg_17_all_nul_fewer_lines_than_size() {
    let mut rng = Rng::new(0x1017);
    for _ in 0..400 {
        let size = 2 + rng.below(64);
        let n = rng.below(size); // strictly fewer than the max
        let mut buf = vec![0u8; size];
        let o = check(&mut buf, n, size, "cfg17");
        let expect: Vec<isize> = (0..n as isize).collect();
        assert_eq!(o, Outcome::Ok(expect));
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn cfg_18_more_lines_present_than_requested() {
    let mut rng = Rng::new(0x1018);
    for _ in 0..600 {
        let present = 4 + rng.below(32);
        let lens: Vec<usize> = (0..present).map(|_| rng.below(10)).collect();
        let mut buf = build(&mut rng, &lens, true, false);
        let size = buf.len();
        let want = 1 + rng.below(present - 1); // strictly fewer than present
        assert_ok(&check(&mut buf, want, size, "cfg18"), "cfg18");
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn cfg_19_single_very_long_line() {
    // unterminated 8193-byte line -> inner loop exits on pos+len == bufferSize
    let mut buf = vec![0x41u8; 8193];
    assert_eq!(check(&mut buf, 1, 8193, "cfg19-unterm"), Outcome::Ok(vec![0]));
    // 8192 bytes + NUL -> inner loop exits on the terminator
    let mut buf2 = vec![0x41u8; 8193];
    buf2[8192] = 0;
    assert_eq!(check(&mut buf2, 1, 8193, "cfg19-term"), Outcome::Ok(vec![0]));
    // two long lines
    let mut buf3 = vec![0x41u8; 8193];
    buf3[4096] = 0;
    assert_eq!(
        check(&mut buf3, 2, 8193, "cfg19-two"),
        Outcome::Ok(vec![0, 4097])
    );
}

// ---------------------------------------------------------------- row 20
#[test]
fn cfg_20_high_bit_payloads() {
    let mut rng = Rng::new(0x1020);
    for _ in 0..600 {
        let n = 3 + rng.below(32);
        let lens: Vec<usize> = (0..n).map(|_| 1 + rng.below(10)).collect();
        let mut buf = build(&mut rng, &lens, true, /* high_bit */ true);
        // every payload byte must have the high bit set
        assert!(buf.iter().all(|&b| b == 0 || b >= 0x80));
        let size = buf.len();
        assert_ok(&check(&mut buf, n, size, "cfg20"), "cfg20");
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn cfg_21_property_random_bytes_random_nul_density() {
    let mut rng = Rng::new(0xDEAD_BEEF_0021);
    let mut successes = 0usize;
    for case in 0..4000 {
        let cap = 1 + rng.below(96);
        // NUL density from very dense (1) to very sparse (32)
        let density = 1 + rng.below(32);
        let mut buf: Vec<u8> = (0..cap)
            .map(|_| {
                if rng.below(density) == 0 {
                    0
                } else {
                    let b = rng.byte();
                    if b == 0 {
                        1
                    } else {
                        b
                    }
                }
            })
            .collect();
        let size = 1 + rng.below(cap); // may truncate mid-line
        let present = count_lines(&buf, size);
        let n = rng.below(present + 1); // always satisfiable
        let o = check(&mut buf, n, size, &format!("cfg21#{case}"));
        assert_ok(&o, "cfg21");
        successes += 1;
    }
    assert_eq!(successes, 4000);
}

// ---------------------------------------------------------------- row 22
#[test]
fn cfg_22_property_random_line_length_vectors() {
    let mut rng = Rng::new(0xC0FF_EE00_0022);
    for case in 0..4000 {
        let n = rng.below(40);
        let lens: Vec<usize> = (0..n).map(|_| rng.below(9)).collect(); // incl. 0-length
        let terminate_last = rng.below(2) == 0;
        let high = rng.below(4) == 0;
        let mut buf = build(&mut rng, &lens, terminate_last, high);
        if buf.is_empty() {
            buf.push(0);
        }
        // s= (exact) or s+ (slack) or sT (truncated)
        let size = match rng.below(3) {
            0 => buf.len(),
            1 => {
                let extra = 1 + rng.below(8);
                buf.extend(std::iter::repeat(0x41).take(extra));
                buf.len()
            }
            _ => 1 + rng.below(buf.len()),
        };
        let present = count_lines(&buf, size);
        let want = rng.below(present + 1);
        assert_ok(
            &check(&mut buf, want, size, &format!("cfg22#{case}")),
            "cfg22",
        );
    }
}

// ---------------------------------------------------------------- row 23
#[test]
fn cfg_23_one_hundred_thousand_lines() {
    let n = 100_000usize;
    let mut buf = vec![0u8; n * 2];
    for i in 0..n {
        buf[i * 2] = b'x';
        buf[i * 2 + 1] = 0;
    }
    let size = buf.len();
    assert_eq!(count_lines(&buf, size), n);
    let o = diff(&mut buf, n, size, "cfg23");
    match &o {
        Outcome::Ok(v) => {
            assert_eq!(v.len(), n);
            for (i, &off) in v.iter().enumerate() {
                assert_eq!(off, (i * 2) as isize, "slot {i}");
            }
        }
        Outcome::Null => panic!("cfg23: expected success"),
    }
    // partial request over the same big buffer
    assert_ok(&diff_read(&mut buf, 12_345, size, 12_345, "cfg23-partial"), "cfg23-partial");
}

// ---------------------------------------------------------------- row 24
#[test]
fn cfg_24_full_two_dimensional_boundary_sweep() {
    for size in 1..=64usize {
        for content in [0x00u8, 0x41u8, 0xFFu8] {
            let mut buf = vec![content; size];
            let present = count_lines(&buf, size);
            for n in 0..=present {
                let o = check(&mut buf, n, size, "cfg24");
                assert_ok(&o, "cfg24");
            }
        }
    }
    // mixed pattern sweep: NUL every k-th byte
    for size in 1..=48usize {
        for k in 1..=6usize {
            let mut buf: Vec<u8> = (0..size)
                .map(|i| if i % k == k - 1 { 0 } else { 0x42 })
                .collect();
            let present = count_lines(&buf, size);
            for n in 0..=present {
                assert_ok(&check(&mut buf, n, size, "cfg24-mixed"), "cfg24-mixed");
            }
        }
    }
}

// ---------------------------------------------------------------- row 25
/// Allocation-size parity. The returned block is owned by the caller (the C
/// header pulls in <stdlib.h> and the C code `malloc`s it), so the number of
/// bytes requested is part of the observable contract: it fixes
/// `sizeof(const char**) == 8` and the `numLines * sizeof(...)` arithmetic.
#[test]
fn cfg_25_allocation_size_parity() {
    use common::diff_alloc_size;
    // NOTE: only sizes below glibc's 128 KiB mmap threshold are checked.
    // At/above it (numLines >= 16382 => 131056 bytes) `malloc_usable_size`
    // becomes a function of *allocator history* (glibc's dynamic mmap
    // threshold) rather than of the requested size, so the two calls legitimately
    // report different usable sizes for the identical request. Measured: the
    // first divergence is exactly numLines == 16382. Above that this is not a
    // valid differential observable, and the line-pointer comparison in the
    // other rows (which reads slot i at byte offset 8*i) already pins the
    // element size anyway.
    let mut counts: Vec<usize> = (0..80).collect();
    counts.extend([100usize, 255, 256, 257, 1000, 4096, 8192, 16_000, 16_381]);
    for n in counts {
        let size = n.max(1);
        let mut buf = vec![0u8; size]; // all-NUL -> exactly `size` lines available
        let r = diff_alloc_size(&mut buf, n, size, "cfg25");
        let (a, _) = r.unwrap_or_else(|| panic!("cfg25: expected success for numLines={n}"));
        assert!(
            a >= n * 8,
            "cfg25: usable size {a} < numLines*8 ({}) for numLines={n} — \
             element size looks wrong",
            n * 8
        );
        // glibc never over-allocates by a whole extra element for these sizes,
        // so this also rules out an element size of 16.
        assert!(
            a < (n + 2) * 8 + 32,
            "cfg25: usable size {a} far exceeds numLines*8 ({}) for numLines={n}",
            n * 8
        );
    }
}

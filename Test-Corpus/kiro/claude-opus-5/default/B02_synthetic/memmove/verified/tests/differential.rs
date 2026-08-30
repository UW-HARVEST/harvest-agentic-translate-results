//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses over the same stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! The input classes below were enumerated by reading `c_src/src/main.c` and
//! `c_src/src/lib.c` and taking every `if`, every early `return` and every
//! length/threshold comparison as its own class.

mod common;

use common::{assert_same, c_overflows, case, input};

// ---------------------------------------------------------------------------
// Phase A sanity: both executables exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_programs_are_runnable() {
    let c = common::run(common::c_bin(), b"0 0 0 1 42\n");
    let r = common::run(&common::rust_bin(), b"0 0 0 1 42\n");
    assert_eq!(c.status, Ok(0), "C program did not exit 0 on a trivial input");
    assert_eq!(r.status, Ok(0), "Rust program did not exit 0 on a trivial input");
    assert_eq!(c.stdout, b"1 42\n");
    assert_eq!(c.stdout, r.stdout);
}

// ---------------------------------------------------------------------------
// main.c: the four `scanf` guards and the length bound.
// ---------------------------------------------------------------------------

#[test]
fn error_reading_flags() {
    for (label, inp) in [
        ("empty stdin", &b""[..]),
        ("single newline", b"\n"),
        ("blank line", b"   \n"),
        ("all whitespace kinds", b" \t\n\x0b\x0c\r "),
        ("non numeric", b"abc\n"),
        ("lone plus", b"+"),
        ("lone minus", b"-"),
        ("plus then letters", b"+abc\n"),
        ("double sign", b"++1 2 3 0\n"),
        ("mixed sign", b"+-1 2 3 0\n"),
        ("nul byte", b"\x00"),
        ("nul then digits", b"\x00 1 2 3\n"),
        ("high byte", b"\xff 1 2 0\n"),
        ("dot first", b".5 1 2 0\n"),
    ] {
        assert_same(label, inp);
    }
}

#[test]
fn error_reading_param1() {
    for (label, inp) in [
        ("flags only", &b"1"[..]),
        ("flags then newline", b"1\n"),
        ("flags then junk", b"1 x 2 3\n"),
        ("flags then sign only", b"1 - 2 3\n"),
        ("flags then dot", b"1 . 2 3\n"),
        ("flags then trailing space", b"7   "),
        // `%u` stops before `x`, so `0` is accepted for flags and `x10` is what
        // the next conversion sees.
        ("hex literal for flags", b"0x10 1 2 0\n"),
    ] {
        assert_same(label, inp);
    }
}

#[test]
fn error_reading_param2() {
    for (label, inp) in [
        ("two fields", &b"1 2"[..]),
        ("two fields newline", b"1 2\n"),
        ("junk at param2", b"1 2 x 3\n"),
        ("sign only at param2", b"1 2 + 3\n"),
        // `1.5` parses as `1`, leaving `.5` for param2, which fails.
        ("float for param1", b"1 1.5 3 0\n"),
        ("exponent for param1", b"1 1e5 3 0\n"),
    ] {
        assert_same(label, inp);
    }
}

#[test]
fn error_reading_length() {
    for (label, inp) in [
        ("three fields", &b"1 2 3"[..]),
        ("three fields newline", b"1 2 3\n"),
        ("junk at length", b"1 2 3 x\n"),
        ("sign only at length", b"1 2 3 -\n"),
        ("float for param2", b"1 2 3.5 4\n"),
    ] {
        assert_same(label, inp);
    }
}

#[test]
fn length_exceeds_maximum() {
    for (label, inp) in [
        ("257", &b"0 0 0 257\n"[..]),
        ("258", b"0 0 0 258\n"),
        ("1000", b"0 0 0 1000\n"),
        ("one million", b"0 0 0 1000000\n"),
        // `%zu` goes through strtoul, so a negative length wraps to ULONG_MAX.
        ("negative one", b"0 0 0 -1\n"),
        ("negative 256", b"0 0 0 -256\n"),
        ("ULONG_MAX", b"0 0 0 18446744073709551615\n"),
        ("ULONG_MAX plus one clamps", b"0 0 0 18446744073709551616\n"),
        ("far past ULONG_MAX", b"0 0 0 99999999999999999999999999999999\n"),
        ("explicit plus", b"0 0 0 +257\n"),
        // Bytes after an over-long length are never read.
        ("over long with data", b"0 0 0 300 1 2 3\n"),
    ] {
        assert_same(label, inp);
    }
}

#[test]
fn error_reading_byte() {
    for (label, inp) in [
        ("no bytes at all", &b"0 0 0 1\n"[..]),
        ("fails at index 0", b"0 0 0 5 x\n"),
        ("fails at index 1", b"0 0 0 5 1\n"),
        ("fails at index 3", b"0 0 0 5 1 2 3 y 5\n"),
        ("fails at index 4", b"0 0 0 5 1 2 3 4\n"),
        ("fails at index 254", b"0 0 0 256 1 2 3\n"),
        ("sign only mid buffer", b"0 0 0 3 1 - 3\n"),
    ] {
        assert_same(label, inp);
    }
    // Truncated maximum-length buffer: fails at index 255.
    let mut inp = String::from("0 0 0 256");
    for i in 0..255 {
        inp.push_str(&format!(" {}", i % 256));
    }
    inp.push('\n');
    assert_same("fails at index 255", inp.as_bytes());
}

// ---------------------------------------------------------------------------
// main.c: numeric conversion behaviour (glibc strtoul/strtol semantics).
// ---------------------------------------------------------------------------

#[test]
fn integer_conversion_edges() {
    for (label, inp) in [
        ("flags UINT_MAX", &b"4294967295 1 2 2 1 2\n"[..]),
        ("flags UINT_MAX+1 truncates", b"4294967296 1 2 2 1 2\n"),
        ("flags UINT_MAX+2 truncates", b"4294967297 1 2 2 1 2\n"),
        ("flags ULONG_MAX", b"18446744073709551615 1 2 2 1 2\n"),
        ("flags ULONG_MAX+1 clamps", b"18446744073709551616 1 2 2 1 2\n"),
        ("flags absurdly large", b"999999999999999999999999999 1 2 2 1 2\n"),
        ("flags negative wraps", b"-1 1 2 2 1 2\n"),
        ("flags negative two", b"-2 1 2 2 1 2\n"),
        ("param1 INT_MAX", b"31 2147483647 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 INT_MAX+1", b"31 2147483648 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 INT_MIN", b"31 -2147483648 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 INT_MIN-1", b"31 -2147483649 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 LONG_MAX", b"31 9223372036854775807 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 LONG_MAX+1 clamps", b"31 9223372036854775808 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 LONG_MIN", b"31 -9223372036854775808 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 LONG_MIN-1 clamps", b"31 -9223372036854775809 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 absurd positive", b"31 99999999999999999999999999 0 8 1 2 3 4 5 6 7 8\n"),
        ("param1 absurd negative", b"31 -99999999999999999999999999 0 8 1 2 3 4 5 6 7 8\n"),
        ("param2 UINT_MAX+1", b"7 4 4294967296 8 1 1 2 2 3 3 4 4\n"),
        ("param2 UINT_MAX+1 plus one", b"7 4 4294967297 8 1 1 2 2 3 3 4 4\n"),
        ("param2 INT_MAX+1", b"7 4 2147483648 8 1 1 2 2 3 3 4 4\n"),
        ("param2 negative", b"7 4 -1 8 1 1 2 2 3 3 4 4\n"),
        ("bytes out of range", b"0 0 0 4 256 257 -1 4294967296\n"),
        ("bytes wild", b"0 0 0 4 511 -256 99999999999999999999 -0\n"),
        ("bytes clamped huge", b"0 0 0 3 18446744073709551615 18446744073709551616 -1\n"),
        ("leading zeros everywhere", b"00000003 000001 0000 0004 01 02 03 04\n"),
        ("padded byte values", b"0 0 0 2 0000000000000005 000000000000009\n"),
        ("explicit plus signs", b"+3 +1 +0 +4 +1 +2 +3 +4\n"),
        ("negative zero", b"-0 -0 -0 -0\n"),
        ("octal-looking length", b"0 0 0 010 1 2 3 4 5 6 7 8\n"),
    ] {
        assert_same(label, inp);
    }
}

#[test]
fn whitespace_and_stream_shape() {
    for (label, inp) in [
        // scanf skips newlines, so a vertical input is identical to a flat one.
        ("newline separated", &b"3\n1\n0\n4\n1\n2\n3\n4\n"[..]),
        ("crlf separated", b"3\r\n1\r\n0\r\n4\r\n1\r\n2\r\n3\r\n4\r\n"),
        ("mixed whitespace", b"  \n\n\t 3 \r\n 1 \x0b 0 \x0c 4 \n 1 \t 2 \r 3 \n 4 \n"),
        ("no trailing newline", b"3 1 0 4 1 2 3 4"),
        ("trailing whitespace", b"3 1 0 4 1 2 3 4   \n\n\n"),
        ("extra tokens ignored", b"3 1 0 4 1 2 3 4 5 6 7\n"),
        ("trailing junk ignored", b"3 1 0 4 1 2 3 4 garbage\n"),
        ("huge leading whitespace", b"                                  0 0 0 0\n"),
    ] {
        assert_same(label, inp);
    }
}

// ---------------------------------------------------------------------------
// process_buffer: the guards at the top and the flag dispatch.
// ---------------------------------------------------------------------------

#[test]
fn zero_length_returns_zero_for_every_flag_combination() {
    for flags in 0u32..32 {
        for param1 in [-2147483648i64, -1, 0, 1, 3, 4, 255, 2147483647] {
            for param2 in [0i64, 1] {
                assert_same(
                    &format!("len0 flags={flags} p1={param1} p2={param2}"),
                    &case(flags, param1, param2, &[]),
                );
            }
        }
    }
}

#[test]
fn no_flags_is_passthrough() {
    for data in [
        vec![],
        vec![7],
        vec![7, 8],
        vec![0, 255, 128, 1],
        (0..=255u8).collect::<Vec<u8>>(),
    ] {
        assert_same(&format!("flags0 len{}", data.len()), &case(0, 0, 0, &data));
    }
}

#[test]
fn single_element_buffer() {
    for flags in 0u32..32 {
        for param1 in [-2i64, -1, 0, 1, 2, 3, 4, 255] {
            for param2 in [0i64, 1] {
                assert_same(
                    &format!("len1 flags={flags} p1={param1} p2={param2}"),
                    &case(flags, param1, param2, &[42]),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rotate_buffer (flag 0x01)
// ---------------------------------------------------------------------------

#[test]
fn rotate_all_offsets() {
    let data: Vec<u8> = (0..10u8).collect();
    // Covers offset == 0 (skipped), the `offset < len / 2` prefix path, the
    // `else` suffix path, and the negative-offset normalisation.
    for param1 in -25i64..=25 {
        assert_same(&format!("rotate p1={param1}"), &case(0x01, param1, 0, &data));
    }
    // len == 1 hits the `len <= 1` early return; len == 2 makes `len / 2 == 1`
    // so offset 1 takes the suffix path.
    for len in [1usize, 2, 3, 4, 5] {
        let d: Vec<u8> = (0..len as u8).collect();
        for param1 in -6i64..=6 {
            assert_same(
                &format!("rotate len={len} p1={param1}"),
                &case(0x01, param1, 0, &d),
            );
        }
    }
}

#[test]
fn rotate_extremes() {
    let full: Vec<u8> = (0..=255u8).collect();
    for param1 in [
        -2147483648i64,
        -2147483647,
        -100000,
        -257,
        -256,
        -255,
        -128,
        -127,
        -1,
        1,
        127,
        128,
        129,
        255,
        256,
        257,
        100000,
        2147483647,
    ] {
        assert_same(
            &format!("rotate256 p1={param1}"),
            &case(0x01, param1, 0, &full),
        );
    }
    // Odd length so that len / 2 truncates.
    let odd: Vec<u8> = (0..255u8).collect();
    for param1 in [1i64, 126, 127, 128, 129, 254, -127, -128] {
        assert_same(
            &format!("rotate255 p1={param1}"),
            &case(0x01, param1, 0, &odd),
        );
    }
}

// ---------------------------------------------------------------------------
// compact_runs (flag 0x02)
// ---------------------------------------------------------------------------

#[test]
fn compact_runs_thresholds() {
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        ("single", vec![5]),
        ("pair", vec![5, 5]),
        ("triple", vec![5, 5, 5]),
        ("quad", vec![5, 5, 5, 5]),
        ("no runs", vec![1, 2, 3, 4, 5, 6, 7, 8]),
        ("mixed runs", vec![1, 1, 1, 2, 2, 3, 3, 3, 3, 4]),
        ("run at end", vec![1, 2, 3, 4, 4, 4, 4, 4]),
        ("run at start", vec![9, 9, 9, 9, 1, 2, 3, 4]),
        ("alternating", vec![1, 2, 1, 2, 1, 2, 1, 2]),
        ("two long runs", {
            let mut v = vec![1u8; 10];
            v.extend(std::iter::repeat(2u8).take(10));
            v
        }),
    ];
    // param1 <= 0 and param1 > 255 fall back to the default threshold of 3.
    for param1 in [-2147483648i64, -1, 0, 1, 2, 3, 4, 5, 10, 255, 256, 2147483647] {
        for (name, data) in &patterns {
            if c_overflows(0x02, param1, data.len()) {
                continue;
            }
            assert_same(
                &format!("compact {name} p1={param1}"),
                &case(0x02, param1, 0, data),
            );
        }
    }
}

#[test]
fn compact_runs_caps_at_255() {
    // A single run of 256 identical bytes is the only way to reach the
    // `run_len > 255` cap; the C code then advances `read` by the capped 255,
    // leaving one stray byte behind.
    let all_same = vec![7u8; 256];
    for param1 in [-1i64, 0, 1, 2, 3, 100, 254, 255, 256] {
        if c_overflows(0x02, param1, all_same.len()) {
            continue;
        }
        assert_same(
            &format!("cap255 p1={param1}"),
            &case(0x02, param1, 0, &all_same),
        );
    }
    // 255 identical bytes sits exactly on the cap without exceeding it.
    let just_under = vec![7u8; 255];
    for param1 in [-1i64, 2, 3, 254, 255] {
        assert_same(
            &format!("cap254 p1={param1}"),
            &case(0x02, param1, 0, &just_under),
        );
    }
    // A 256-long run followed by nothing vs. a 255-long run plus a different
    // tail byte, which exercises the `read + run_len < len` memmove.
    let mut run_plus_tail = vec![7u8; 255];
    run_plus_tail.push(8);
    for param1 in [-1i64, 3, 255] {
        assert_same(
            &format!("cap tail p1={param1}"),
            &case(0x02, param1, 0, &run_plus_tail),
        );
    }
}

#[test]
fn compact_runs_growth_path() {
    // Threshold 1 rewrites every single-element run as two bytes, so the
    // logical length grows past the input length. 128 input bytes is the
    // largest size for which the C program still stays inside `buffer[256]`.
    for len in [1usize, 2, 3, 4, 5, 8, 16, 64, 127, 128] {
        let distinct: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
        assert_same(
            &format!("grow distinct len={len}"),
            &case(0x02, 1, 0, &distinct),
        );
        let alternating: Vec<u8> = (0..len).map(|i| (i % 2) as u8).collect();
        assert_same(
            &format!("grow alternating len={len}"),
            &case(0x02, 1, 0, &alternating),
        );
    }
    // Growth interleaved with shrinking runs, which makes the logical length
    // move up and then back down inside one call.
    let mixed = vec![1, 2, 3, 3, 3, 3, 3, 3];
    assert_same("grow then shrink", &case(0x02, 1, 0, &mixed));
    // Growth feeding the later flags.
    for flags in [0x02u32 | 0x04, 0x02 | 0x08, 0x02 | 0x10, 0x1e, 0x1f] {
        for param2 in [0i64, 1] {
            assert_same(
                &format!("grow chained flags={flags} p2={param2}"),
                &case(flags, 1, param2, &[0, 1, 2, 3, 4, 5, 6, 7]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// remove_duplicates (flag 0x04)
// ---------------------------------------------------------------------------

#[test]
fn remove_duplicates_both_paths() {
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        ("empty-ish single", vec![9]),
        ("two same", vec![9, 9]),
        ("two different", vec![9, 8]),
        ("all same", vec![4, 4, 4, 4, 4, 4]),
        ("all distinct", vec![1, 2, 3, 4, 5, 6]),
        ("dupes late", vec![1, 2, 3, 1, 2, 3]),
        ("dupes early", vec![1, 1, 2, 2, 3, 3]),
        ("zeros mixed", vec![0, 5, 0, 5, 0, 5]),
        ("full byte range", (0..=255u8).collect()),
        ("every value twice", (0..=255u8).map(|b| b / 2).collect()),
    ];
    // param2 == 0 takes the swap-to-front path, anything else preserves order.
    for param2 in [0i64, 1, -1, 2, 2147483647, -2147483648] {
        for (name, data) in &patterns {
            assert_same(
                &format!("dedup {name} p2={param2}"),
                &case(0x04, 0, param2, data),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// interleave_halves (flag 0x08)
// ---------------------------------------------------------------------------

#[test]
fn interleave_length_guard_and_parities() {
    // new_len < 2 skips the call entirely.
    assert_same("interleave len1", &case(0x08, 0, 0, &[7]));
    for len in [2usize, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 127, 128, 254, 255, 256] {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        assert_same(&format!("interleave len={len}"), &case(0x08, 0, 0, &data));
    }
    // Compaction first, so interleave runs on a length that differs from the
    // input length, including odd results that hit the `buf[len - 1]` fixup.
    for param1 in [-1i64, 1, 2, 3] {
        for data in [
            vec![7u8; 256],
            vec![1, 1, 1, 2, 2, 2, 3, 3, 3],
            vec![1, 2, 3, 4, 5],
        ] {
            if c_overflows(0x0a, param1, data.len()) {
                continue;
            }
            assert_same(
                &format!("compact+interleave p1={param1} len={}", data.len()),
                &case(0x0a, param1, 0, &data),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// reverse_segments (flag 0x10)
// ---------------------------------------------------------------------------

#[test]
fn reverse_segments_sizes() {
    // new_len < 4 skips the call.
    for len in [1usize, 2, 3] {
        let d: Vec<u8> = (0..len as u8).collect();
        for param1 in [-1i64, 1, 2, 3, 4] {
            assert_same(
                &format!("revseg short len={len} p1={param1}"),
                &case(0x10, param1, 0, &d),
            );
        }
    }
    let data: Vec<u8> = (0..12u8).collect();
    // param1 <= 0 defaults to 4; seg_size 1 returns early; seg_size > new_len
    // is skipped by the caller; seg_size == new_len reverses the whole buffer.
    for param1 in [-2147483648i64, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 255, 2147483647] {
        assert_same(
            &format!("revseg len12 p1={param1}"),
            &case(0x10, param1, 0, &data),
        );
    }
    // Remainders of 0, 1 and >1 relative to the segment size.
    for len in [4usize, 5, 6, 7, 9, 10, 11, 255, 256] {
        let d: Vec<u8> = (0..len).map(|i| (i % 253) as u8).collect();
        for param1 in [2i64, 3, 4, 5, 7, 128, 255, 256] {
            assert_same(
                &format!("revseg len={len} p1={param1}"),
                &case(0x10, param1, 0, &d),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Flag combinations and the maximum buffer the program accepts.
// ---------------------------------------------------------------------------

#[test]
fn every_flag_combination_on_representative_buffers() {
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        ("two", vec![3, 9]),
        ("four", vec![4, 4, 1, 2]),
        ("runs and dupes", vec![5, 5, 5, 1, 2, 2, 7, 7, 7, 7, 3, 4]),
        ("all same 16", vec![6u8; 16]),
        ("distinct 16", (0..16u8).collect()),
        ("max distinct", (0..=255u8).collect()),
        ("max all same", vec![7u8; 256]),
        ("max alternating", (0..256).map(|i| (i % 2) as u8).collect()),
        ("max blocks", (0..256).map(|i| (i / 4) as u8).collect()),
    ];
    for flags in 0u32..32 {
        for param1 in [-2147483648i64, -1, 0, 1, 2, 3, 4, 5, 128, 255, 256, 2147483647] {
            for param2 in [0i64, 1] {
                for (name, data) in &patterns {
                    if c_overflows(flags, param1, data.len()) {
                        continue;
                    }
                    assert_same(
                        &format!("combo flags={flags} p1={param1} p2={param2} {name}"),
                        &case(flags, param1, param2, data),
                    );
                }
            }
        }
    }
}

#[test]
fn maximum_length_boundary() {
    let full: Vec<u8> = (0..=255u8).collect();
    // 256 is accepted, 257 is rejected.
    assert_same("length 256 accepted", &case(0x1f, 3, 1, &full));
    assert_same("length 255 accepted", &case(0x1f, 3, 1, &full[..255]));
    assert_same("length 257 rejected", &input("31", "3", "1", 257, &full));
    // The declared length wins over how many bytes follow.
    assert_same("declared 256 exact", &input("31", "3", "1", 256, &full));
    assert_same("declared 3 with 256 bytes", &input("31", "3", "1", 3, &full));
}

// ---------------------------------------------------------------------------
// Deterministic randomised sweep over the defined input domain.
// ---------------------------------------------------------------------------

/// Small xorshift so the sweep is reproducible without a dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

#[test]
fn randomised_sweep() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let lengths = [0usize, 1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 63, 64, 127, 128, 200, 255, 256];
    let params: [i64; 22] = [
        -2147483648, -100000, -257, -256, -255, -128, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 8,
        128, 255, 256, 2147483647,
    ];
    let alphabets = [1u16, 2, 3, 5, 17, 256];

    for i in 0..3000 {
        let len = rng.pick(&lengths);
        let alphabet = rng.pick(&alphabets);
        let data: Vec<u8> = (0..len)
            .map(|_| (rng.below(alphabet as u64)) as u8)
            .collect();
        let flags = rng.below(32) as u32;
        let param1 = rng.pick(&params);
        let param2 = rng.pick(&[0i64, 1, -1, 2147483647]);
        if c_overflows(flags, param1, data.len()) {
            continue;
        }
        assert_same(
            &format!("sweep #{i} flags={flags} p1={param1} p2={param2} len={len}"),
            &case(flags, param1, param2, &data),
        );
    }
}

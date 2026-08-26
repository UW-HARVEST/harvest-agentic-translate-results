//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are driven exclusively through their `.so` exports.
//! Every row uses many randomized inputs from a fixed seed.

mod common;

use common::{diff_call_fma, diff_driver, diff_fma_array, both, Rng, INT_BOUNDARY};
use std::ffi::c_int;

const ITERS: usize = 400;

// ===========================================================================
// fma_array — lowest-level entry point
// ===========================================================================

/// C1: `len == 0`, disjoint buffers — must leave `out` completely untouched.
#[test]
fn c1_fma_array_len0() {
    let mut rng = Rng::for_test("c1");
    for _ in 0..ITERS {
        let n = rng.range(1, 8);
        let out: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let got = diff_fma_array(&out, &m1, &m2, &ad, 0, "c1");
        assert_eq!(got, out, "len=0 must not modify out");
    }
}

/// C2: `len == 1`, small values (no overflow).
#[test]
fn c2_fma_array_len1_small() {
    let mut rng = Rng::for_test("c2");
    for _ in 0..ITERS {
        let m1 = [rng.small_i32()];
        let m2 = [rng.small_i32()];
        let ad = [rng.small_i32()];
        let got = diff_fma_array(&[0], &m1, &m2, &ad, 1, "c2");
        assert_eq!(got[0], m1[0] * m2[0] + ad[0]);
    }
}

/// C3: `len == 1`, full `int` range (signed overflow expected).
#[test]
fn c3_fma_array_len1_fullrange() {
    let mut rng = Rng::for_test("c3");
    for _ in 0..ITERS {
        let m1 = [rng.next_i32()];
        let m2 = [rng.next_i32()];
        let ad = [rng.next_i32()];
        let got = diff_fma_array(&[rng.next_i32()], &m1, &m2, &ad, 1, "c3");
        assert_eq!(got[0], m1[0].wrapping_mul(m2[0]).wrapping_add(ad[0]));
    }
}

/// C4: small `len`, small values.
#[test]
fn c4_fma_array_small_len_small_vals() {
    let mut rng = Rng::for_test("c4");
    for _ in 0..ITERS {
        let n = rng.range(2, 16);
        let out: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.small_i32()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.small_i32()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.small_i32()).collect();
        let got = diff_fma_array(&out, &m1, &m2, &ad, n as c_int, "c4");
        for i in 0..n {
            assert_eq!(got[i], m1[i] * m2[i] + ad[i]);
        }
    }
}

/// C5: small `len`, full-range values — signed overflow on nearly every element.
#[test]
fn c5_fma_array_small_len_fullrange() {
    let mut rng = Rng::for_test("c5");
    for _ in 0..ITERS {
        let n = rng.range(2, 16);
        let out: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let got = diff_fma_array(&out, &m1, &m2, &ad, n as c_int, "c5");
        for i in 0..n {
            assert_eq!(got[i], m1[i].wrapping_mul(m2[i]).wrapping_add(ad[i]));
        }
    }
}

/// C6: operands drawn only from the `int` boundary set.
#[test]
fn c6_fma_array_boundary_values() {
    let mut rng = Rng::for_test("c6");
    for _ in 0..ITERS {
        let n = rng.range(2, 16);
        let out: Vec<i32> = (0..n).map(|_| *rng.pick(&INT_BOUNDARY)).collect();
        let m1: Vec<i32> = (0..n).map(|_| *rng.pick(&INT_BOUNDARY)).collect();
        let m2: Vec<i32> = (0..n).map(|_| *rng.pick(&INT_BOUNDARY)).collect();
        let ad: Vec<i32> = (0..n).map(|_| *rng.pick(&INT_BOUNDARY)).collect();
        diff_fma_array(&out, &m1, &m2, &ad, n as c_int, "c6");
    }
    // Also the exhaustive cross product of the boundary set at len == 1.
    for &a in INT_BOUNDARY.iter() {
        for &b in INT_BOUNDARY.iter() {
            for &c in INT_BOUNDARY.iter() {
                diff_fma_array(&[0], &[a], &[b], &[c], 1, "c6-exhaustive");
            }
        }
    }
}

/// C7: large `len`.
#[test]
fn c7_fma_array_large_len() {
    let mut rng = Rng::for_test("c7");
    for _ in 0..24 {
        let n = rng.range(1024, 4096);
        let out: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let got = diff_fma_array(&out, &m1, &m2, &ad, n as c_int, "c7");
        for i in 0..n {
            assert_eq!(got[i], m1[i].wrapping_mul(m2[i]).wrapping_add(ad[i]));
        }
    }
}

/// C8: negative `len` — the `for` loop must run zero times in both.
#[test]
fn c8_fma_array_negative_len_noop() {
    let mut rng = Rng::for_test("c8");
    let mut lens: Vec<c_int> = vec![-1, -2, -16, -1000, i32::MIN, i32::MIN + 1];
    for _ in 0..64 {
        lens.push(-((rng.range(1, 1 << 30)) as c_int));
    }
    for len in lens {
        let n = rng.range(1, 8);
        let out: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m1: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let got = diff_fma_array(&out, &m1, &m2, &ad, len, "c8");
        assert_eq!(got, out, "negative len={len} must not modify out");
    }
}

// --- aliasing rows (the C declares `int *restrict out`) --------------------

fn alias_case(tag: &str, which: usize) {
    let (c, r) = both();
    let mut rng = Rng::for_test(tag);
    for _ in 0..ITERS {
        let n = rng.range(1, 24);
        let base: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let other1: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let other2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();

        let run = |f: common::FmaArrayFn| -> Vec<i32> {
            let mut buf = base.clone();
            let mut o1 = other1.clone();
            let mut o2 = other2.clone();
            unsafe {
                let p = buf.as_mut_ptr();
                match which {
                    // out == mul1
                    0 => f(p, p, o1.as_ptr(), o2.as_ptr(), n as c_int),
                    // out == mul2
                    1 => f(p, o1.as_ptr(), p, o2.as_ptr(), n as c_int),
                    // out == add
                    2 => f(p, o1.as_ptr(), o2.as_ptr(), p, n as c_int),
                    // all four identical
                    3 => f(p, p, p, p, n as c_int),
                    _ => unreachable!(),
                }
            }
            let _ = (&mut o1, &mut o2);
            buf
        };
        let vc = run(c.fma_array);
        let vr = run(r.fma_array);
        assert_eq!(vc, vr, "aliasing mismatch [{tag}] n={n} base={base:?}");
    }
}

/// C9: `out == mul1`.
#[test]
fn c9_fma_array_alias_out_mul1() {
    alias_case("c9", 0);
}

/// C10: `out == mul2`.
#[test]
fn c10_fma_array_alias_out_mul2() {
    alias_case("c10", 1);
}

/// C11: `out == add`.
#[test]
fn c11_fma_array_alias_out_add() {
    alias_case("c11", 2);
}

/// C12: all four pointers identical.
#[test]
fn c12_fma_array_alias_all_same() {
    alias_case("c12", 3);
}

/// C13: `out` and `mul1` overlap at an offset — write order matters.
#[test]
fn c13_fma_array_alias_offset_overlap() {
    let (c, r) = both();
    let mut rng = Rng::for_test("c13");
    for _ in 0..ITERS {
        let n = rng.range(1, 24);
        let base: Vec<i32> = (0..n + 4).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let off = rng.range(1, 4);
        let run = |f: common::FmaArrayFn| -> Vec<i32> {
            let mut buf = base.clone();
            unsafe {
                let p = buf.as_mut_ptr();
                f(p, p.add(off) as *const i32, m2.as_ptr(), ad.as_ptr(), n as c_int);
            }
            buf
        };
        assert_eq!(
            run(c.fma_array),
            run(r.fma_array),
            "offset-overlap mismatch n={n} off={off}"
        );
    }
}

// ===========================================================================
// call_fma — mid-level entry point
// ===========================================================================

/// C14: `len == 0` (the one explicit guard in the C).
#[test]
fn c14_call_fma_len0() {
    let mut rng = Rng::for_test("c14");
    for _ in 0..ITERS {
        let n = rng.range(1, 8);
        let data: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        assert_eq!(diff_call_fma(&data, 0, "c14"), 0);
    }
}

/// C15: `len == 1` — `out[len-1]` is `out[0]`.
#[test]
fn c15_call_fma_len1() {
    let mut rng = Rng::for_test("c15");
    for _ in 0..ITERS {
        let data = [rng.next_i32()];
        assert_eq!(diff_call_fma(&data, 1, "c15"), data[0]);
    }
}

/// C16: small `len`, small values.
#[test]
fn c16_call_fma_small_len_small_vals() {
    let mut rng = Rng::for_test("c16");
    for _ in 0..ITERS {
        let n = rng.range(2, 64);
        let data: Vec<i32> = (0..n).map(|_| rng.small_i32()).collect();
        assert_eq!(diff_call_fma(&data, n as c_int, "c16"), data[n - 1]);
    }
}

/// C17: small `len`, full `int` range.
#[test]
fn c17_call_fma_small_len_fullrange() {
    let mut rng = Rng::for_test("c17");
    for _ in 0..ITERS {
        let n = rng.range(2, 64);
        let data: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        assert_eq!(diff_call_fma(&data, n as c_int, "c17"), data[n - 1]);
    }
}

/// C18: values only from the boundary set.
#[test]
fn c18_call_fma_boundary_values() {
    let mut rng = Rng::for_test("c18");
    for _ in 0..ITERS {
        let n = rng.range(2, 64);
        let data: Vec<i32> = (0..n).map(|_| *rng.pick(&INT_BOUNDARY)).collect();
        assert_eq!(diff_call_fma(&data, n as c_int, "c18"), data[n - 1]);
    }
    // Every boundary value in the decisive last position.
    for &v in INT_BOUNDARY.iter() {
        let data = vec![v; 3];
        assert_eq!(diff_call_fma(&data, 3, "c18-last"), v);
    }
}

/// C19: large `len` that still fits the C VLA stack budget (3 * len * 4 bytes).
#[test]
fn c19_call_fma_large_len() {
    let mut rng = Rng::for_test("c19");
    for _ in 0..16 {
        let n = rng.range(1024, 32768);
        let data: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        assert_eq!(diff_call_fma(&data, n as c_int, "c19"), data[n - 1]);
    }
}

/// C20: `len == 100` — the exact shape `driver` produces at its loop bound.
#[test]
fn c20_call_fma_len_100() {
    let mut rng = Rng::for_test("c20");
    for _ in 0..ITERS {
        let data: Vec<i32> = (0..100).map(|_| rng.next_i32()).collect();
        assert_eq!(diff_call_fma(&data, 100, "c20"), data[99]);
    }
}

/// C21: `data` pointing into the middle of a larger buffer.
#[test]
fn c21_call_fma_offset_data_ptr() {
    let (c, r) = both();
    let mut rng = Rng::for_test("c21");
    for _ in 0..ITERS {
        let total = rng.range(8, 128);
        let off = rng.range(1, total - 1);
        let n = rng.range(1, total - off);
        let buf: Vec<i32> = (0..total).map(|_| rng.next_i32()).collect();
        let p = unsafe { buf.as_ptr().add(off) };
        let vc = unsafe { (c.call_fma)(p, n as c_int) };
        let vr = unsafe { (r.call_fma)(p, n as c_int) };
        assert_eq!(vc, vr, "offset data ptr mismatch off={off} n={n}");
        assert_eq!(vc, buf[off + n - 1]);
    }
}

// ===========================================================================
// driver — top-level entry point (stdout bytes compared)
// ===========================================================================

const WS: [u8; 6] = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];

/// Accumulates `(input, expected_printed_value)` pairs and checks the whole
/// batch with a single differential run. `None` means "no independent oracle —
/// only require C and Rust to agree".
struct DriverBatch {
    ctx: &'static str,
    inputs: Vec<Vec<u8>>,
    expect: Vec<Option<i32>>,
}

impl DriverBatch {
    fn new(ctx: &'static str) -> Self {
        DriverBatch {
            ctx,
            inputs: Vec::new(),
            expect: Vec::new(),
        }
    }
    fn push(&mut self, input: Vec<u8>, expect: Option<i32>) {
        self.inputs.push(input);
        self.expect.push(expect);
    }
    fn push_str(&mut self, input: &str, expect: Option<i32>) {
        self.push(input.as_bytes().to_vec(), expect);
    }
    /// Runs the differential comparison and the oracle assertions.
    fn run(self) {
        assert!(!self.inputs.is_empty(), "[{}] empty batch", self.ctx);
        let lines = common::diff_driver_lines(&self.inputs, self.ctx);
        for (i, want) in self.expect.iter().enumerate() {
            if let Some(v) = want {
                let expected = format!("{v}\n").into_bytes();
                assert_eq!(
                    lines[i],
                    expected,
                    "[{}] input #{i} {:?}: expected {:?}, both libraries printed {:?}",
                    self.ctx,
                    String::from_utf8_lossy(&self.inputs[i]),
                    String::from_utf8_lossy(&expected),
                    String::from_utf8_lossy(&lines[i]),
                );
            } else {
                // Still require a well-formed single decimal line.
                let s = String::from_utf8_lossy(&lines[i]).to_string();
                assert!(
                    s.ends_with('\n') && s.trim().parse::<i32>().is_ok(),
                    "[{}] input #{i} {:?}: malformed output {:?}",
                    self.ctx,
                    String::from_utf8_lossy(&self.inputs[i]),
                    s
                );
            }
        }
    }
}

fn expect_line(out: &[u8], value: i32, ctx: &str) {
    let want = format!("{value}\n").into_bytes();
    assert_eq!(
        out,
        want.as_slice(),
        "[{ctx}] expected {:?}, got {:?}",
        String::from_utf8_lossy(&want),
        String::from_utf8_lossy(out)
    );
}

fn join_vals(vals: &[i32], sep: &str) -> String {
    vals.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(sep)
}

/// C22: exactly one token, no surrounding whitespace.
#[test]
fn c22_driver_one_token() {
    let mut rng = Rng::for_test("c22");
    let mut b = DriverBatch::new("c22");
    for _ in 0..ITERS {
        let v = rng.next_i32();
        b.push(v.to_string().into_bytes(), Some(v));
    }
    // Every boundary value on its own.
    for &v in INT_BOUNDARY.iter() {
        b.push(v.to_string().into_bytes(), Some(v));
    }
    b.run();
}

/// C23: 2..=20 space-separated tokens.
#[test]
fn c23_driver_few_tokens_space() {
    let mut rng = Rng::for_test("c23");
    let mut b = DriverBatch::new("c23");
    for _ in 0..ITERS {
        let n = rng.range(2, 20);
        let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        b.push(join_vals(&vals, " ").into_bytes(), Some(vals[n - 1]));
    }
    b.run();
}

/// C24: randomized whitespace runs between and around tokens.
#[test]
fn c24_driver_random_whitespace_mix() {
    let mut rng = Rng::for_test("c24");
    let mut b = DriverBatch::new("c24");
    for _ in 0..ITERS {
        let n = rng.range(2, 20);
        let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..rng.range(0, 5) {
            s.push(*rng.pick(&WS));
        }
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                for _ in 0..rng.range(1, 5) {
                    s.push(*rng.pick(&WS));
                }
            }
            s.extend_from_slice(v.to_string().as_bytes());
        }
        for _ in 0..rng.range(0, 5) {
            s.push(*rng.pick(&WS));
        }
        b.push(s, Some(vals[n - 1]));
    }
    b.run();
}

/// C25: randomized sign form and leading-zero padding.
#[test]
fn c25_driver_sign_and_leading_zeros() {
    let mut rng = Rng::for_test("c25");
    let mut b = DriverBatch::new("c25");
    for _ in 0..ITERS {
        let n = rng.range(2, 20);
        let mut last = 0i32;
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            let mag = (rng.next_u32() % 100_000) as i32;
            let neg = rng.bool();
            last = if neg { -mag } else { mag };
            let zeros = "0".repeat(rng.range(0, 4));
            let sign = if neg {
                "-"
            } else if rng.bool() {
                "+"
            } else {
                ""
            };
            s.push_str(&format!("{sign}{zeros}{mag}"));
        }
        b.push(s.into_bytes(), Some(last));
    }
    for t in ["-0", "+0", "-000", "+000", "0 -0", "-0 0", "00000000000000000"] {
        b.push_str(t, Some(0));
    }
    b.run();
}

/// C26: tokens drawn from the numeric boundary set.
#[test]
fn c26_driver_boundary_numbers() {
    let mut rng = Rng::for_test("c26");
    let mut b = DriverBatch::new("c26");
    for _ in 0..ITERS {
        let n = rng.range(1, 20);
        let vals: Vec<i32> = (0..n).map(|_| *rng.pick(&INT_BOUNDARY)).collect();
        b.push(join_vals(&vals, " ").into_bytes(), Some(vals[n - 1]));
    }
    // Exhaustive: every boundary value in the last (decisive) position.
    for &a in INT_BOUNDARY.iter() {
        for &z in INT_BOUNDARY.iter() {
            b.push(format!("{a} {z}").into_bytes(), Some(z));
        }
    }
    b.run();
}

/// C27: tokens outside the `int` range (and outside `long`).
#[test]
fn c27_driver_out_of_range_numbers() {
    let mut rng = Rng::for_test("c27");
    let mut b = DriverBatch::new("c27");
    for _ in 0..ITERS {
        let n = rng.range(1, 8);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            if rng.bool() {
                s.push('-');
            }
            let digits = rng.range(10, 30);
            for d in 0..digits {
                let c = if d == 0 {
                    b'1' + (rng.next_u32() % 9) as u8
                } else {
                    b'0' + (rng.next_u32() % 10) as u8
                };
                s.push(c as char);
            }
        }
        // No independent oracle (glibc saturate-then-truncate); require agreement.
        b.push(s.into_bytes(), None);
    }
    // Pinned values, verified against the C .so.
    for (input, want) in [
        ("2147483648", -2147483648i32),
        ("-2147483649", 2147483647),
        ("4294967296", 0),
        ("4294967297", 1),
        ("99999999999999999999", -1),
        ("-99999999999999999999", 0),
        ("9223372036854775807", -1),
        ("9223372036854775808", -1),
        ("-9223372036854775808", 0),
        ("-9223372036854775809", 0),
    ] {
        b.push_str(input, Some(want));
    }
    b.run();
}

/// C28: 99 / 100 / 101 tokens — the loop bound and one step either side.
#[test]
fn c28_driver_token_count_boundary() {
    let mut rng = Rng::for_test("c28");
    let mut b = DriverBatch::new("c28");
    for _ in 0..60 {
        for n in [99usize, 100, 101] {
            let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            b.push(
                join_vals(&vals, " ").into_bytes(),
                Some(vals[n.min(100) - 1]),
            );
        }
    }
    b.run();
}

/// C29: far past the 100-token bound.
#[test]
fn c29_driver_many_tokens() {
    let mut rng = Rng::for_test("c29");
    let mut b = DriverBatch::new("c29");
    for _ in 0..80 {
        let n = rng.range(101, 300);
        let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        b.push(join_vals(&vals, " ").into_bytes(), Some(vals[99]));
    }
    b.run();
}

/// C30: a valid prefix then a non-numeric token at a random index.
#[test]
fn c30_driver_random_early_exit() {
    const JUNK: [&str; 8] = ["x", "abc", ",", ";", "/", "@", "q1", "zz9"];
    let mut rng = Rng::for_test("c30");
    let mut b = DriverBatch::new("c30");
    for _ in 0..ITERS {
        let k = rng.range(0, 20);
        let vals: Vec<i32> = (0..k).map(|_| rng.next_i32()).collect();
        let mut s = String::new();
        for v in &vals {
            s.push_str(&v.to_string());
            s.push(' ');
        }
        s.push_str(*rng.pick(&JUNK));
        s.push(' ');
        for _ in 0..rng.range(0, 5) {
            s.push_str(&rng.next_i32().to_string());
            s.push(' ');
        }
        let want = if k == 0 { 0 } else { vals[k - 1] };
        b.push(s.into_bytes(), Some(want));
    }
    b.run();
}

/// C31: random printable-ASCII + whitespace soup.
#[test]
fn c31_driver_random_ascii_fuzz() {
    let mut rng = Rng::for_test("c31");
    let mut b = DriverBatch::new("c31");
    for _ in 0..3000 {
        let n = rng.range(0, 256);
        let mut s: Vec<u8> = Vec::with_capacity(n);
        for _ in 0..n {
            let byte = match rng.range(0, 3) {
                0 => b'0' + (rng.next_u32() % 10) as u8,
                1 => *rng.pick(&WS),
                2 => *rng.pick(&[b'-', b'+', b'.', b',', b'x', b'e']),
                _ => 0x20 + (rng.next_u32() % 95) as u8,
            };
            s.push(byte);
        }
        b.push(s, None);
    }
    b.run();
}

/// C32: random arbitrary (non-NUL) bytes, including non-ASCII.
#[test]
fn c32_driver_random_byte_fuzz() {
    let mut rng = Rng::for_test("c32");
    let mut b = DriverBatch::new("c32");
    for _ in 0..3000 {
        let n = rng.range(0, 256);
        let s: Vec<u8> = (0..n).map(|_| 1 + (rng.next_u32() % 255) as u8).collect();
        b.push(s, None);
    }
    b.run();
}

/// C33: alternating digit runs and separators — stresses the `%zn` cursor.
#[test]
fn c33_driver_digit_run_fuzz() {
    let mut rng = Rng::for_test("c33");
    let mut b = DriverBatch::new("c33");
    for _ in 0..2000 {
        let groups = rng.range(0, 40);
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..groups {
            for _ in 0..rng.range(0, 3) {
                s.push(*rng.pick(&WS));
            }
            if rng.range(0, 4) == 0 {
                s.push(*rng.pick(&[b'-', b'+']));
            }
            for _ in 0..rng.range(1, 12) {
                s.push(b'0' + (rng.next_u32() % 10) as u8);
            }
            if rng.range(0, 6) == 0 {
                s.push(*rng.pick(&[b'a', b'x', b'.', b',', b'E']));
            }
        }
        b.push(s, None);
    }
    b.run();
}

/// C34: long inputs (4 KiB .. 64 KiB).
#[test]
fn c34_driver_long_input() {
    let mut rng = Rng::for_test("c34");
    let mut b = DriverBatch::new("c34");
    for _ in 0..12 {
        let target = rng.range(4096, 65536);
        let mut s: Vec<u8> = Vec::with_capacity(target + 16);
        let mut vals: Vec<i32> = Vec::new();
        while s.len() < target {
            let v = rng.next_i32();
            vals.push(v);
            s.extend_from_slice(v.to_string().as_bytes());
            for _ in 0..rng.range(1, 3) {
                s.push(*rng.pick(&WS));
            }
        }
        let want = vals[vals.len().min(100) - 1];
        b.push(s, Some(want));
    }
    b.run();
}

/// C35: token with an alpha suffix at a random token index.
#[test]
fn c35_driver_alpha_suffix() {
    let mut rng = Rng::for_test("c35");
    let mut b = DriverBatch::new("c35");
    for _ in 0..ITERS {
        let n = rng.range(1, 12);
        let idx = rng.range(0, n - 1);
        let vals: Vec<i32> = (0..n).map(|_| (rng.next_u32() % 100_000) as i32).collect();
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&vals[i].to_string());
            if i == idx {
                s.push_str(*rng.pick(&["abc", "x", "Z", "e", "q"]));
            }
        }
        // The scan stops right after the suffixed token.
        b.push(s.into_bytes(), Some(vals[idx]));
    }
    b.run();
}

// ===========================================================================
// Composed pipeline
// ===========================================================================

/// C36: `driver` -> `call_fma` -> `fma_array` end to end. The printed value must
/// equal what `call_fma` returns for the same token vector, in *both* libraries.
#[test]
fn c36_pipeline_consistency() {
    let (c, r) = both();
    let mut rng = Rng::for_test("c36");
    for _ in 0..300 {
        let n = rng.range(1, 150);
        let vals: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let s = vals
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let out = diff_driver(s.as_bytes(), "c36");

        let used = n.min(100);
        let tokens: Vec<i32> = vals[..used].to_vec();
        let via_c = unsafe { (c.call_fma)(tokens.as_ptr(), used as c_int) };
        let via_r = unsafe { (r.call_fma)(tokens.as_ptr(), used as c_int) };
        assert_eq!(via_c, via_r, "call_fma leg disagrees");
        expect_line(&out, via_c, "c36");
    }
}

/// C37: `call_fma(data, len)` must equal the explicit
/// `fma_array(out, ones, data, zeros, len)` sequence in both libraries.
#[test]
fn c37_call_fma_matches_manual_fma_array() {
    let (c, r) = both();
    let mut rng = Rng::for_test("c37");
    for _ in 0..ITERS {
        let n = rng.range(1, 128);
        let data: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let ones = vec![1i32; n];
        let zeros = vec![0i32; n];

        for imp in [c, r] {
            let mut out = vec![0i32; n];
            unsafe {
                (imp.fma_array)(
                    out.as_mut_ptr(),
                    ones.as_ptr(),
                    data.as_ptr(),
                    zeros.as_ptr(),
                    n as c_int,
                );
            }
            let via_call = unsafe { (imp.call_fma)(data.as_ptr(), n as c_int) };
            assert_eq!(
                out[n - 1],
                via_call,
                "[{}] call_fma != manual fma_array pipeline (n={n})",
                imp.name
            );
            assert_eq!(out, data, "[{}] ones*data+zeros must equal data", imp.name);
        }
        diff_call_fma(&data, n as c_int, "c37");
    }
}

/// C38: reusing one `out` buffer with shrinking `len` — the tail must keep its
/// previous contents in both implementations.
#[test]
fn c38_fma_array_stale_tail() {
    let (c, r) = both();
    let mut rng = Rng::for_test("c38");
    for _ in 0..200 {
        let cap = rng.range(4, 64);
        let m1: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
        let m2: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
        let ad: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
        let init: Vec<i32> = (0..cap).map(|_| rng.next_i32()).collect();
        let lens: Vec<c_int> = {
            let mut l: Vec<c_int> = (0..4).map(|_| rng.range(0, cap) as c_int).collect();
            l.sort_unstable_by(|a, b| b.cmp(a));
            l
        };
        let run = |f: common::FmaArrayFn| -> Vec<i32> {
            let mut out = init.clone();
            for &len in &lens {
                unsafe {
                    f(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), len);
                }
            }
            out
        };
        assert_eq!(run(c.fma_array), run(r.fma_array), "stale-tail mismatch");
    }
}

/// C39: the C `.so` imports `__isoc99_sscanf` while the Rust `.so` imports the
/// legacy `sscanf`. Prove the two glibc entry points are indistinguishable for
/// the only format string the library uses, `"%d%zn"`.
#[test]
fn sscanf_entrypoint_equivalence_d_zn() {
    extern "C" {
        fn sscanf(s: *const std::ffi::c_char, fmt: *const std::ffi::c_char, ...) -> c_int;
        fn __isoc99_sscanf(s: *const std::ffi::c_char, fmt: *const std::ffi::c_char, ...)
            -> c_int;
    }
    const FMT: &[u8; 6] = b"%d%zn\0";
    let mut rng = Rng::for_test("c39");
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"abc".to_vec(),
        b"42".to_vec(),
        b"  -7  ".to_vec(),
        b"+9".to_vec(),
        b"0x10".to_vec(),
        b"2147483648".to_vec(),
        b"-99999999999999999999".to_vec(),
        b"-".to_vec(),
        b"3.14".to_vec(),
    ];
    for _ in 0..3000 {
        let n = rng.range(0, 40);
        inputs.push(
            (0..n)
                .map(|_| match rng.range(0, 3) {
                    0 => b'0' + (rng.next_u32() % 10) as u8,
                    1 => *rng.pick(&WS),
                    2 => *rng.pick(&[b'-', b'+', b'.', b'x', b'e', b',']),
                    _ => 0x21 + (rng.next_u32() % 94) as u8,
                })
                .collect(),
        );
    }
    for inp in inputs {
        let mut buf = inp.clone();
        buf.push(0);
        let p = buf.as_ptr() as *const std::ffi::c_char;
        let f = FMT.as_ptr() as *const std::ffi::c_char;

        let (mut v1, mut nb1) = (0i32, usize::MAX);
        let (mut v2, mut nb2) = (0i32, usize::MAX);
        let r1 = unsafe { sscanf(p, f, &mut v1 as *mut i32, &mut nb1 as *mut usize) };
        let r2 = unsafe { __isoc99_sscanf(p, f, &mut v2 as *mut i32, &mut nb2 as *mut usize) };
        assert_eq!(
            (r1, v1, nb1),
            (r2, v2, nb2),
            "sscanf vs __isoc99_sscanf diverge on {:?}",
            String::from_utf8_lossy(&inp)
        );
    }
}

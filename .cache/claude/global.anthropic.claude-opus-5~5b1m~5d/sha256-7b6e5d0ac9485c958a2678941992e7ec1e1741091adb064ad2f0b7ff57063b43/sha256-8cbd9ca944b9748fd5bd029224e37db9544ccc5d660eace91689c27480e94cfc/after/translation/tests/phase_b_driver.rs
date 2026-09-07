// Phase B -- top-level convenience entry point: `driver`.
// CONFIGS.md rows C16..C32.
//
// `driver`'s only observable effect is its `printf`, so every test captures
// file descriptor 1 around the call and compares the captured bytes.

mod common;
use common::*;

fn join(nums: &[i32], sep: &str) -> Vec<u8> {
    nums.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(sep)
        .into_bytes()
}

#[test]
fn c16_single_integer_no_whitespace() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..500 {
        let v = rng.i32_any();
        assert_driver_same(v.to_string().as_bytes(), "C16");
    }
    for &v in EXTREMES {
        assert_driver_same(v.to_string().as_bytes(), "C16/extremes");
    }
}

#[test]
fn c17_explicit_plus_sign_and_leading_zeros() {
    let mut rng = Rng::new(SEED ^ 17);
    let mut cases: Vec<Vec<u8>> = vec![
        b"007".to_vec(),
        b"+000042".to_vec(),
        b"-000042".to_vec(),
        b"+0".to_vec(),
        b"-0".to_vec(),
        b"0".to_vec(),
        b"00000000000000000000000000005".to_vec(),
        b"+2147483647".to_vec(),
        b"+0000002147483647".to_vec(),
        b"1 +2 -3 +04 -005".to_vec(),
    ];
    for _ in 0..200 {
        let v = rng.range(0, 100_000) as i32;
        cases.push(format!("+{v}").into_bytes());
        cases.push(format!("{:08}", v).into_bytes());
        cases.push(format!("-{:06}", v).into_bytes());
    }
    for c in &cases {
        assert_driver_same(c, "C17");
    }
}

#[test]
fn c18_two_integers_one_space() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..600 {
        let a = rng.i32_any();
        let b = rng.i32_any();
        assert_driver_same(&join(&[a, b], " "), "C18");
    }
}

#[test]
fn c19_n_integers_single_space() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..400 {
        let n = rng.range(2, 20);
        let nums = rng.vec_any(n);
        assert_driver_same(&join(&nums, " "), "C19");
    }
}

#[test]
fn c20_each_whitespace_separator_class() {
    let mut rng = Rng::new(SEED ^ 20);
    for &ws in WS {
        let sep = String::from_utf8(vec![ws]).unwrap();
        for _ in 0..80 {
            let n = rng.range(2, 12);
            let nums = rng.vec_any(n);
            assert_driver_same(&join(&nums, &sep), "C20");
        }
    }
}

#[test]
fn c21_mixed_whitespace_runs() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..400 {
        let n = rng.range(2, 15);
        let nums = rng.vec_any(n);
        let mut s: Vec<u8> = Vec::new();
        for (i, v) in nums.iter().enumerate() {
            if i > 0 {
                let runlen = rng.range(1, 6);
                for _ in 0..runlen {
                    s.push(rng.pick(WS));
                }
            }
            s.extend_from_slice(v.to_string().as_bytes());
        }
        assert_driver_same(&s, "C21");
    }
}

#[test]
fn c22_leading_and_trailing_whitespace() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..400 {
        let n = rng.range(1, 10);
        let nums = rng.vec_any(n);
        let body = join(&nums, " ");
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..rng.range(1, 5) {
            s.push(rng.pick(WS));
        }
        s.extend_from_slice(&body);
        for _ in 0..rng.range(1, 5) {
            s.push(rng.pick(WS));
        }
        assert_driver_same(&s, "C22");
    }
    assert_driver_same(b"   ", "C22/only-ws");
    assert_driver_same(b"\n\n\t ", "C22/only-ws2");
    assert_driver_same(b"  42", "C22/lead");
    assert_driver_same(b"42  ", "C22/trail");
    assert_driver_same(b"\t\n 42 \r\n", "C22/both");
}

#[test]
fn c23_no_separator_before_minus_sign() {
    // `%d` treats `-` as the start of a new token, so "5-3-2" is three tokens.
    let fixed: &[&[u8]] = &[
        b"5-3-2",
        b"1-1",
        b"-1-2-3",
        b"10-20+30",
        b"7+8",
        b"0-0",
        b"2147483647-2147483648",
    ];
    for c in fixed {
        assert_driver_same(c, "C23/fixed");
    }
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..400 {
        let n = rng.range(2, 12);
        let mut s: Vec<u8> = Vec::new();
        for i in 0..n {
            let v = rng.range(0, 100_000) as i32;
            if i == 0 {
                s.extend_from_slice(v.to_string().as_bytes());
            } else {
                s.push(if rng.next_u64() & 1 == 0 { b'-' } else { b'+' });
                s.extend_from_slice(v.to_string().as_bytes());
            }
        }
        assert_driver_same(&s, "C23/random");
    }
}

#[test]
fn c24_exactly_99_integers() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..60 {
        let nums = rng.vec_any(99);
        assert_driver_same(&join(&nums, " "), "C24");
    }
}

#[test]
fn c25_exactly_100_integers() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..60 {
        let nums = rng.vec_any(100);
        assert_driver_same(&join(&nums, " "), "C25");
    }
}

#[test]
fn c26_exactly_101_integers_cap_boundary() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..60 {
        let nums = rng.vec_any(101);
        let input = join(&nums, " ");
        // Both must print the 100th value, not the 101st.
        let (c_out, r_out) = run_driver_both(&input);
        assert_eq!(c_out, r_out, "[C26] diverged on 101 integers");
        assert_eq!(
            String::from_utf8_lossy(&c_out).trim_end(),
            nums[99].to_string(),
            "[C26] C should print the 100th integer (cap at 100)"
        );
    }
}

#[test]
fn c27_far_past_the_cap() {
    let mut rng = Rng::new(SEED ^ 27);
    for n in [150usize, 300] {
        for _ in 0..30 {
            let nums = rng.vec_any(n);
            let input = join(&nums, " ");
            let (c_out, r_out) = run_driver_both(&input);
            assert_eq!(c_out, r_out, "[C27] diverged on {n} integers");
            assert_eq!(
                String::from_utf8_lossy(&c_out).trim_end(),
                nums[99].to_string()
            );
        }
    }
}

#[test]
fn c28_in_range_int_boundaries() {
    let cases: &[&[u8]] = &[
        b"-2147483648",
        b"2147483647",
        b"-2147483648 2147483647",
        b"2147483647 -2147483648",
        b"2147483647 2147483647 2147483647",
        b"-2147483648 -2147483648",
        b"0 -2147483648",
        b"-2147483647 2147483646",
    ];
    for c in cases {
        assert_driver_same(c, "C28");
    }
    // ... and a long run of them, so the cap and the boundary interact.
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..40 {
        let n = rng.range(95, 105);
        let nums: Vec<i32> = (0..n)
            .map(|_| if rng.next_u64() & 1 == 0 { i32::MIN } else { i32::MAX })
            .collect();
        assert_driver_same(&join(&nums, " "), "C28/long");
    }
}

#[test]
fn c29_clamping_tokens_mixed_with_valid() {
    let cases: &[&[u8]] = &[
        b"2147483648",
        b"-2147483649",
        b"99999999999999999999",
        b"-99999999999999999999",
        b"1 2147483648",
        b"2147483648 7",
        b"5 99999999999999999999 6",
        b"340282366920938463463374607431768211456",
        b"9223372036854775808",
        b"-9223372036854775809",
        b"00000000000000002147483648",
    ];
    for c in cases {
        assert_driver_same(c, "C29");
    }
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..300 {
        let n = rng.range(1, 8);
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..n {
            if rng.next_u64() % 3 == 0 {
                // huge digit string
                let digits = rng.range(11, 30);
                let mut s = String::new();
                if rng.next_u64() & 1 == 0 {
                    s.push('-');
                }
                for _ in 0..digits {
                    s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
                }
                parts.push(s);
            } else {
                parts.push(rng.i32_any().to_string());
            }
        }
        assert_driver_same(parts.join(" ").as_bytes(), "C29/random");
    }
}

#[test]
fn c30_valid_prefix_then_garbage() {
    let cases: &[&[u8]] = &[
        b"1 2 3 abc 9",
        b"7 0x10",
        b"4 5 +",
        b"4 5 -",
        b"1 2 3 -",
        b"10 20 30 ! 40",
        b"1,2,3",
        b"1;2",
        b"3.14",
        b"1 2 3.5 4",
        b"1e5",
        b"1 2 e 3",
        b"0x1f",
        b"12abc34",
        b"1 2 \x01 3",
        b"5 #6",
        b"9 (10)",
    ];
    for c in cases {
        assert_driver_same(c, "C30");
    }
}

#[test]
fn c31_fuzz_random_text() {
    let mut rng = Rng::new(SEED ^ 31);
    let garbage: &[u8] = b"abcxyzXYZ.,;:!?()[]{}/\\*&^%$#@~'\"|<>=_e";
    for _ in 0..600 {
        let toks = rng.range(0, 25);
        let mut s: Vec<u8> = Vec::new();
        for _ in 0..toks {
            match rng.next_u64() % 10 {
                0..=5 => s.extend_from_slice(rng.i32_any().to_string().as_bytes()),
                6 => {
                    let d = rng.range(1, 25);
                    for _ in 0..d {
                        s.push(b'0' + (rng.next_u64() % 10) as u8);
                    }
                }
                7 => s.push(rng.pick(garbage)),
                8 => s.push(rng.pick(b"+-")),
                _ => {
                    for _ in 0..rng.range(1, 4) {
                        s.push(rng.pick(WS));
                    }
                }
            }
            if rng.next_u64() & 1 == 0 {
                s.push(rng.pick(WS));
            }
        }
        assert_driver_same(&s, "C31");
    }
}

#[test]
fn c32_repeated_invocations_in_one_process() {
    // No residual global state: interleave C and Rust calls and compare the
    // whole concatenated transcript.
    let mut rng = Rng::new(SEED ^ 32);
    let l = libs();
    let c_fn = l.c.driver();
    let r_fn = l.rust.driver();

    let inputs: Vec<Vec<u8>> = (0..10)
        .map(|_| {
            let n = rng.range(0, 120);
            let nums = rng.vec_any(n);
            let mut b = join(&nums, " ");
            b.push(0);
            b
        })
        .collect();

    let c_all = capture_stdout(|| {
        for i in &inputs {
            unsafe { c_fn(i.as_ptr() as *const std::ffi::c_char) };
        }
    });
    let r_all = capture_stdout(|| {
        for i in &inputs {
            unsafe { r_fn(i.as_ptr() as *const std::ffi::c_char) };
        }
    });
    assert_eq!(c_all, r_all, "[C32] transcript of 10 sequential calls diverged");
    assert_eq!(c_all.iter().filter(|b| **b == b'\n').count(), 10);
}

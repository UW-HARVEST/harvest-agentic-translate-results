//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driven with many randomized inputs
//! from a fixed seed. Both implementations are reached only through their
//! `.so` exports.

mod common;

use common::{
    c, diff_call_fma, diff_driver, diff_fma_array, rs, run_driver, Rng, EXTREMES, SEED,
};

const REPS: usize = 200;

fn rand_vec(rng: &mut Rng, n: usize, full_range: bool) -> Vec<i32> {
    (0..n)
        .map(|_| {
            if full_range {
                rng.next_i32()
            } else {
                rng.range(-1000, 1000) as i32
            }
        })
        .collect()
}

fn rand_extreme_vec(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| *rng.pick(&EXTREMES)).collect()
}

// ===========================================================================
// fma_array — rows 1..12
// ===========================================================================

fn fma_row(seed_tag: u64, ctx: &str, lens: &[usize], full_range: bool, slack: usize) {
    let mut rng = Rng::new(SEED ^ seed_tag);
    for rep in 0..REPS {
        let n = *rng.pick(lens);
        let mul1 = rand_vec(&mut rng, n.max(1), full_range);
        let mul2 = rand_vec(&mut rng, n.max(1), full_range);
        let add = rand_vec(&mut rng, n.max(1), full_range);
        diff_fma_array(
            &mul1,
            &mul2,
            &add,
            n as i32,
            n + slack,
            &format!("{ctx} rep={rep} len={n}"),
        );
    }
}

#[test]
fn cfg01_fma_len1() {
    fma_row(1, "cfg01 len=1 small values", &[1], false, 3);
}

#[test]
fn cfg02_fma_len2() {
    fma_row(2, "cfg02 len=2 small values", &[2], false, 3);
}

#[test]
fn cfg03_fma_len3to8_fullrange() {
    fma_row(3, "cfg03 len 3..=8 full range", &[3, 4, 5, 6, 7, 8], true, 4);
}

#[test]
fn cfg04_fma_len99() {
    fma_row(4, "cfg04 len=99", &[99], true, 5);
}

#[test]
fn cfg05_fma_len100() {
    fma_row(5, "cfg05 len=100", &[100], true, 5);
}

#[test]
fn cfg06_fma_len101() {
    fma_row(6, "cfg06 len=101", &[101], true, 5);
}

#[test]
fn cfg07_fma_large() {
    let mut rng = Rng::new(SEED ^ 7);
    for rep in 0..40 {
        for &n in &[1000usize, 4096] {
            let mul1 = rand_vec(&mut rng, n, true);
            let mul2 = rand_vec(&mut rng, n, true);
            let add = rand_vec(&mut rng, n, true);
            diff_fma_array(
                &mul1,
                &mul2,
                &add,
                n as i32,
                n + 8,
                &format!("cfg07 large rep={rep} len={n}"),
            );
        }
    }
}

#[test]
fn cfg08_fma_zero_mul1() {
    let mut rng = Rng::new(SEED ^ 8);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 64);
        let mul1 = vec![0i32; n];
        let mul2 = rand_vec(&mut rng, n, true);
        let add = rand_vec(&mut rng, n, true);
        diff_fma_array(&mul1, &mul2, &add, n as i32, n + 4, &format!("cfg08 rep={rep}"));
    }
}

#[test]
fn cfg09_fma_ones_mul1() {
    let mut rng = Rng::new(SEED ^ 9);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 64);
        let mul1 = vec![1i32; n];
        let mul2 = rand_vec(&mut rng, n, true);
        let add = rand_vec(&mut rng, n, true);
        diff_fma_array(&mul1, &mul2, &add, n as i32, n + 4, &format!("cfg09 rep={rep}"));
    }
}

#[test]
fn cfg10_fma_zero_add() {
    let mut rng = Rng::new(SEED ^ 10);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 64);
        let mul1 = rand_vec(&mut rng, n, true);
        let mul2 = rand_vec(&mut rng, n, true);
        let add = vec![0i32; n];
        diff_fma_array(&mul1, &mul2, &add, n as i32, n + 4, &format!("cfg10 rep={rep}"));
    }
}

#[test]
fn cfg11_fma_extreme_overflow() {
    let mut rng = Rng::new(SEED ^ 11);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 32);
        let mul1 = rand_extreme_vec(&mut rng, n);
        let mul2 = rand_extreme_vec(&mut rng, n);
        let add = rand_extreme_vec(&mut rng, n);
        diff_fma_array(
            &mul1,
            &mul2,
            &add,
            n as i32,
            n + 4,
            &format!("cfg11 extremes rep={rep} n={n}"),
        );
    }
    // Every ordered pair of extremes, exhaustively, plus every extreme addend.
    for &a in EXTREMES.iter() {
        for &b in EXTREMES.iter() {
            for &k in EXTREMES.iter() {
                diff_fma_array(&[a], &[b], &[k], 1, 4, &format!("cfg11 exhaustive {a}*{b}+{k}"));
            }
        }
    }
}

#[test]
fn cfg12_fma_no_write_past_len() {
    // out buffer far bigger than len: the sentinel tail must survive in both.
    let mut rng = Rng::new(SEED ^ 12);
    for rep in 0..REPS {
        let n = rng.usize_range(0, 32);
        let cap = n + rng.usize_range(1, 32);
        let mul1 = rand_vec(&mut rng, n.max(1), true);
        let mul2 = rand_vec(&mut rng, n.max(1), true);
        let add = rand_vec(&mut rng, n.max(1), true);
        let out = diff_fma_array(
            &mul1,
            &mul2,
            &add,
            n as i32,
            cap,
            &format!("cfg12 rep={rep} n={n} cap={cap}"),
        );
        for i in n..cap {
            assert_eq!(
                out[i],
                0x5A5A_0000u32 as i32 ^ i as i32,
                "cfg12: element {i} past len={n} was overwritten"
            );
        }
    }
}

// ===========================================================================
// call_fma — rows 13..19
// ===========================================================================

#[test]
fn cfg13_call_fma_len1() {
    let mut rng = Rng::new(SEED ^ 13);
    for rep in 0..REPS {
        let data = rand_vec(&mut rng, 1, true);
        let v = diff_call_fma(&data, 1, &format!("cfg13 rep={rep}"));
        assert_eq!(v, data[0], "cfg13: call_fma(len=1) must return data[0]");
    }
}

#[test]
fn cfg14_call_fma_len2() {
    let mut rng = Rng::new(SEED ^ 14);
    for rep in 0..REPS {
        let data = rand_vec(&mut rng, 2, true);
        diff_call_fma(&data, 2, &format!("cfg14 rep={rep}"));
    }
}

#[test]
fn cfg15_call_fma_len3to8() {
    let mut rng = Rng::new(SEED ^ 15);
    for rep in 0..REPS {
        let n = rng.usize_range(3, 8);
        let data = rand_vec(&mut rng, n, true);
        diff_call_fma(&data, n as i32, &format!("cfg15 rep={rep} n={n}"));
    }
}

#[test]
fn cfg16_call_fma_capacity_boundary() {
    let mut rng = Rng::new(SEED ^ 16);
    for rep in 0..REPS {
        for &n in &[99usize, 100, 101] {
            let data = rand_vec(&mut rng, n, true);
            diff_call_fma(&data, n as i32, &format!("cfg16 rep={rep} n={n}"));
        }
    }
}

#[test]
fn cfg17_call_fma_large() {
    let mut rng = Rng::new(SEED ^ 17);
    for rep in 0..60 {
        let n = 1000;
        let data = rand_vec(&mut rng, n, true);
        diff_call_fma(&data, n as i32, &format!("cfg17 rep={rep}"));
    }
}

#[test]
fn cfg18_call_fma_extremes() {
    let mut rng = Rng::new(SEED ^ 18);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 64);
        let data = rand_extreme_vec(&mut rng, n);
        diff_call_fma(&data, n as i32, &format!("cfg18 rep={rep} n={n}"));
    }
    for &v in EXTREMES.iter() {
        diff_call_fma(&[v], 1, &format!("cfg18 single {v}"));
    }
}

#[test]
fn cfg19_call_fma_len_shorter_than_buffer() {
    let mut rng = Rng::new(SEED ^ 19);
    for rep in 0..REPS {
        let cap = rng.usize_range(2, 64);
        let len = rng.usize_range(1, cap);
        let data = rand_vec(&mut rng, cap, true);
        let v = diff_call_fma(&data, len as i32, &format!("cfg19 rep={rep} len={len} cap={cap}"));
        assert_eq!(
            v, data[len - 1],
            "cfg19: result must be data[len-1], tail must be ignored"
        );
    }
}

// ===========================================================================
// driver — rows 20..36
// ===========================================================================

const WS: [&str; 6] = [" ", "\t", "\n", "\r", "\u{b}", "\u{c}"];

fn join_single_space(vals: &[i32]) -> String {
    vals.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn cfg20_driver_one_int() {
    let mut rng = Rng::new(SEED ^ 20);
    for rep in 0..REPS {
        let v = rng.next_i32();
        let out = diff_driver(&v.to_string(), &format!("cfg20 rep={rep} v={v}"));
        assert_eq!(out, format!("{v}\n").into_bytes(), "cfg20 expected value echo");
    }
}

#[test]
fn cfg21_driver_two_ints() {
    let mut rng = Rng::new(SEED ^ 21);
    for rep in 0..REPS {
        let a = rng.next_i32();
        let b = rng.next_i32();
        diff_driver(&format!("{a} {b}"), &format!("cfg21 rep={rep}"));
    }
}

#[test]
fn cfg22_driver_3to20_single_space() {
    let mut rng = Rng::new(SEED ^ 22);
    for rep in 0..REPS {
        let n = rng.usize_range(3, 20);
        let vals = rand_vec(&mut rng, n, true);
        diff_driver(&join_single_space(&vals), &format!("cfg22 rep={rep} n={n}"));
    }
}

#[test]
fn cfg23_driver_mixed_whitespace_separators() {
    let mut rng = Rng::new(SEED ^ 23);
    for rep in 0..REPS {
        let n = rng.usize_range(3, 20);
        let vals = rand_vec(&mut rng, n, true);
        let mut s = String::new();
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                let reps = rng.usize_range(1, 4);
                for _ in 0..reps {
                    s.push_str(rng.pick(&WS));
                }
            }
            s.push_str(&v.to_string());
        }
        diff_driver(&s, &format!("cfg23 rep={rep} n={n}"));
    }
}

#[test]
fn cfg24_driver_leading_whitespace() {
    let mut rng = Rng::new(SEED ^ 24);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 10);
        let vals = rand_vec(&mut rng, n, true);
        let mut s = String::new();
        for _ in 0..rng.usize_range(1, 8) {
            s.push_str(rng.pick(&WS));
        }
        s.push_str(&join_single_space(&vals));
        diff_driver(&s, &format!("cfg24 rep={rep}"));
    }
}

#[test]
fn cfg25_driver_trailing_whitespace() {
    let mut rng = Rng::new(SEED ^ 25);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 10);
        let vals = rand_vec(&mut rng, n, true);
        let mut s = join_single_space(&vals);
        for _ in 0..rng.usize_range(1, 8) {
            s.push_str(rng.pick(&WS));
        }
        diff_driver(&s, &format!("cfg25 rep={rep}"));
    }
}

#[test]
fn cfg26_driver_explicit_plus_signs() {
    let mut rng = Rng::new(SEED ^ 26);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 12);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            let v = rng.range(0, 100_000) as i32;
            if rng.bool() {
                s.push('+');
            }
            s.push_str(&v.to_string());
        }
        diff_driver(&s, &format!("cfg26 rep={rep}"));
    }
}

#[test]
fn cfg27_driver_mixed_signs() {
    let mut rng = Rng::new(SEED ^ 27);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 12);
        let vals: Vec<i32> = (0..n).map(|_| rng.range(-100_000, 100_000) as i32).collect();
        diff_driver(&join_single_space(&vals), &format!("cfg27 rep={rep}"));
    }
}

#[test]
fn cfg28_driver_leading_zeros() {
    let mut rng = Rng::new(SEED ^ 28);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 12);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            let neg = rng.bool();
            let v = rng.range(0, 99_999);
            let pad = rng.usize_range(0, 6);
            if neg {
                s.push('-');
            }
            for _ in 0..pad {
                s.push('0');
            }
            s.push_str(&v.to_string());
        }
        diff_driver(&s, &format!("cfg28 rep={rep}"));
    }
    for lit in ["007", "-00042", "0000", "+000", "0", "-0"] {
        diff_driver(lit, &format!("cfg28 literal {lit}"));
    }
}

#[test]
fn cfg29_driver_signs_as_separators() {
    // "1-2+3" parses as 1, -2, 3 with no whitespace at all.
    let mut rng = Rng::new(SEED ^ 29);
    for rep in 0..REPS {
        let n = rng.usize_range(2, 15);
        let mut s = String::new();
        s.push_str(&rng.range(0, 9999).to_string());
        for _ in 1..n {
            s.push(if rng.bool() { '-' } else { '+' });
            s.push_str(&rng.range(0, 9999).to_string());
        }
        diff_driver(&s, &format!("cfg29 rep={rep} n={n}"));
    }
    for lit in ["1-2+3-4", "5+5", "-1-1-1", "+1+2+3"] {
        diff_driver(lit, &format!("cfg29 literal {lit}"));
    }
}

#[test]
fn cfg30_driver_int_boundaries() {
    let mut rng = Rng::new(SEED ^ 30);
    for rep in 0..REPS {
        let n = rng.usize_range(1, 12);
        let vals = rand_extreme_vec(&mut rng, n);
        diff_driver(&join_single_space(&vals), &format!("cfg30 rep={rep}"));
    }
    for &v in EXTREMES.iter() {
        diff_driver(&v.to_string(), &format!("cfg30 single {v}"));
    }
}

#[test]
fn cfg31_driver_exactly_99() {
    let mut rng = Rng::new(SEED ^ 31);
    for rep in 0..30 {
        let vals = rand_vec(&mut rng, 99, true);
        let out = diff_driver(&join_single_space(&vals), &format!("cfg31 rep={rep}"));
        assert_eq!(out, format!("{}\n", vals[98]).into_bytes());
    }
}

#[test]
fn cfg32_driver_exactly_100() {
    let mut rng = Rng::new(SEED ^ 32);
    for rep in 0..30 {
        let vals = rand_vec(&mut rng, 100, true);
        let out = diff_driver(&join_single_space(&vals), &format!("cfg32 rep={rep}"));
        assert_eq!(out, format!("{}\n", vals[99]).into_bytes());
    }
}

#[test]
fn cfg33_driver_more_than_100() {
    let mut rng = Rng::new(SEED ^ 33);
    for rep in 0..40 {
        let n = rng.usize_range(101, 300);
        let vals = rand_vec(&mut rng, n, true);
        let out = diff_driver(&join_single_space(&vals), &format!("cfg33 rep={rep} n={n}"));
        assert_eq!(
            out,
            format!("{}\n", vals[99]).into_bytes(),
            "cfg33: only the first 100 integers are read"
        );
    }
}

#[test]
fn cfg34_driver_long_input() {
    let mut rng = Rng::new(SEED ^ 34);
    for rep in 0..15 {
        let n = rng.usize_range(500, 1200);
        let vals = rand_vec(&mut rng, n, true);
        let mut s = String::new();
        for (i, v) in vals.iter().enumerate() {
            if i > 0 {
                for _ in 0..rng.usize_range(1, 3) {
                    s.push_str(rng.pick(&WS));
                }
            }
            s.push_str(&v.to_string());
        }
        diff_driver(&s, &format!("cfg34 rep={rep} n={n} bytes={}", s.len()));
    }
}

#[test]
fn cfg35_driver_random_ascii_soup() {
    let mut rng = Rng::new(SEED ^ 35);
    // Bias the alphabet towards things %d cares about.
    let alphabet: Vec<u8> = b"0123456789 \t\n\r+-abxX.,;eE_/*0123456789 "
        .iter()
        .copied()
        .collect();
    for rep in 0..600 {
        let n = rng.usize_range(0, 80);
        let s: String = (0..n)
            .map(|_| *rng.pick(&alphabet) as char)
            .collect();
        diff_driver(&s, &format!("cfg35 rep={rep}"));
    }
}

#[test]
fn cfg36_driver_valid_prefix_then_garbage() {
    let mut rng = Rng::new(SEED ^ 36);
    let garbage: Vec<u8> = b"abxX.,;/*=%!()[]{}'\"~`?#$&|<>^:@".iter().copied().collect();
    for rep in 0..REPS {
        let n = rng.usize_range(0, 12);
        let vals = rand_vec(&mut rng, n, true);
        let mut s = join_single_space(&vals);
        if n > 0 {
            s.push(' ');
        }
        for _ in 0..rng.usize_range(1, 10) {
            s.push(*rng.pick(&garbage) as char);
        }
        if rng.bool() {
            s.push_str(&format!(" {}", rng.next_i32()));
        }
        let out = diff_driver(&s, &format!("cfg36 rep={rep}"));
        let expect = if n == 0 { 0 } else { vals[n - 1] };
        assert_eq!(out, format!("{expect}\n").into_bytes(), "cfg36 input={s:?}");
    }
}

#[test]
fn cfg37_driver_matches_composed_low_level_pipeline() {
    // Row 37: the same data driven through all three entry points, cross-checked
    // between the two libraries *and* between the levels.
    let mut rng = Rng::new(SEED ^ 37);
    for rep in 0..REPS {
        let n = rng.usize_range(0, 150);
        let vals = rand_vec(&mut rng, n, true);
        let text = join_single_space(&vals);

        let driver_out = diff_driver(&text, &format!("cfg37 driver rep={rep} n={n}"));

        // What driver() should have computed: first min(n,100) values, then
        // call_fma over them.
        let seen = vals[..n.min(100)].to_vec();
        let len = seen.len() as i32;
        let via_call = diff_call_fma(
            &seen,
            len,
            &format!("cfg37 call_fma rep={rep} len={len}"),
        );
        assert_eq!(
            driver_out,
            format!("{via_call}\n").into_bytes(),
            "cfg37: driver stdout must equal call_fma over the parsed prefix"
        );

        // And the same again one level lower: fma_array(ones, seen, zeros).
        if !seen.is_empty() {
            let ones = vec![1i32; seen.len()];
            let zeros = vec![0i32; seen.len()];
            let out = diff_fma_array(
                &ones,
                &seen,
                &zeros,
                len,
                seen.len() + 4,
                &format!("cfg37 fma_array rep={rep}"),
            );
            assert_eq!(
                out[seen.len() - 1],
                via_call,
                "cfg37: fma_array tail must equal call_fma result"
            );
        }
    }
}

// ===========================================================================
// Sanity: the two .so files really are two distinct objects.
// ===========================================================================

#[test]
fn cfg00_distinct_shared_objects() {
    let cp = c().path.canonicalize().unwrap();
    let rp = rs().path.canonicalize().unwrap();
    assert_ne!(cp, rp, "the harness must load two different .so files");
    assert!(
        cp.to_string_lossy().contains("c_src"),
        "C library should come from c_src/build, got {}",
        cp.display()
    );
    // Both must actually respond through their exports.
    assert_eq!(run_driver(c(), "42"), b"42\n");
    assert_eq!(run_driver(rs(), "42"), b"42\n");
}

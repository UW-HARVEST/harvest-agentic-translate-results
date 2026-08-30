// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md.  Every test drives BOTH the C `.so` and the
// Rust `.so` through their exported symbols (loaded with `libloading`) and
// requires byte-identical results.  Inputs are randomized from a fixed seed.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Random arrays of `n` elements, values produced by `gen`.
fn arrays(rng: &mut Rng, n: usize, mut g: impl FnMut(&mut Rng) -> i32) -> [Vec<i32>; 4] {
    let mk = |rng: &mut Rng, g: &mut dyn FnMut(&mut Rng) -> i32| {
        (0..n).map(|_| g(rng)).collect::<Vec<i32>>()
    };
    let a = mk(rng, &mut g);
    let b = mk(rng, &mut g);
    let c = mk(rng, &mut g);
    let d = mk(rng, &mut g);
    [a, b, c, d]
}

fn fma_case(rng: &mut Rng, alias: Alias, n: usize, len: i32, g: impl FnMut(&mut Rng) -> i32) {
    let [o, m1, m2, ad] = arrays(rng, n, g);
    assert_fma_array_eq(alias, len, &o, &m1, &m2, &ad);
}

const WS: [&str; 6] = [" ", "\t", "\n", "\r", "\x0b", "\x0c"];

fn ws_run(rng: &mut Rng) -> String {
    let k = 1 + rng.below(3);
    (0..k).map(|_| rng.pick(&WS)).collect::<Vec<&str>>().concat()
}

/// Render `vals` as a whitespace separated decimal list.
fn join_space(vals: &[i32]) -> String {
    vals.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// rows 1-15 : fma_array (lowest-level entry point, driven directly)
// ---------------------------------------------------------------------------

#[test]
fn cfg01_fma_len0_distinct() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..500 {
        // buffers are non-empty so that a spurious store would be visible
        fma_case(&mut rng, Alias::Distinct, 8, 0, |r| r.i32_corner());
    }
}

#[test]
fn cfg02_fma_len1_small() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..2000 {
        fma_case(&mut rng, Alias::Distinct, 1, 1, |r| r.range_i32(-100, 100));
    }
}

#[test]
fn cfg03_fma_len2_small() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        fma_case(&mut rng, Alias::Distinct, 2, 2, |r| r.range_i32(-100, 100));
    }
}

#[test]
fn cfg04_fma_small_lens_small_vals() {
    let mut rng = Rng::new(SEED ^ 4);
    for &n in &[3usize, 5, 7, 8, 16] {
        for _ in 0..400 {
            fma_case(&mut rng, Alias::Distinct, n, n as i32, |r| {
                r.range_i32(-1000, 1000)
            });
            // also a shorter len than the buffers, to check the tail is untouched
            let shorter = 1 + rng.below(n) as i32;
            fma_case(&mut rng, Alias::Distinct, n, shorter, |r| r.i32_corner());
        }
    }
}

#[test]
fn cfg05_fma_len64_full_range() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..500 {
        fma_case(&mut rng, Alias::Distinct, 64, 64, |r| r.i32_full());
    }
}

#[test]
fn cfg06_fma_corners() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..300 {
        fma_case(&mut rng, Alias::Distinct, 100, 100, |r| r.i32_corner());
    }
    // exhaustive corner cross-product on a single element
    const CORNERS: [i32; 9] = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &CORNERS {
        for &b in &CORNERS {
            for &c in &CORNERS {
                assert_fma_array_eq(Alias::Distinct, 1, &[0], &[a], &[b], &[c]);
            }
        }
    }
}

#[test]
fn cfg07_fma_len1000_full_range() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..60 {
        fma_case(&mut rng, Alias::Distinct, 1000, 1000, |r| r.i32_full());
    }
}

fn alias_row(seed_mix: u64, alias: Alias) {
    let mut rng = Rng::new(SEED ^ seed_mix);
    for _ in 0..1500 {
        let n = 1 + rng.below(32);
        let len = (1 + rng.below(n)) as i32;
        fma_case(&mut rng, alias, n, len, |r| r.i32_corner());
    }
}

#[test]
fn cfg08_fma_alias_out_eq_mul1() {
    alias_row(8, Alias::OutEqMul1);
}

#[test]
fn cfg09_fma_alias_out_eq_mul2() {
    alias_row(9, Alias::OutEqMul2);
}

#[test]
fn cfg10_fma_alias_out_eq_add() {
    alias_row(10, Alias::OutEqAdd);
}

#[test]
fn cfg11_fma_alias_mul1_eq_mul2() {
    alias_row(11, Alias::Mul1EqMul2);
    alias_row(111, Alias::Mul1EqAdd);
}

#[test]
fn cfg12_fma_alias_all_inputs_same() {
    alias_row(12, Alias::AllInputsSame);
}

#[test]
fn cfg13_fma_alias_everything_same() {
    alias_row(13, Alias::EverythingSame);
}

#[test]
fn cfg14_fma_partial_overlap_forward() {
    alias_row(14, Alias::PartialOverlapForward);
}

#[test]
fn cfg15_fma_partial_overlap_backward() {
    alias_row(15, Alias::PartialOverlapBackward);
}

// ---------------------------------------------------------------------------
// rows 16-22 : call_fma (mid-level entry point, driven directly)
// ---------------------------------------------------------------------------

#[test]
fn cfg16_call_fma_len0() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..1000 {
        let data: Vec<i32> = (0..8).map(|_| rng.i32_corner()).collect();
        assert_call_fma_eq(&data, 0);
    }
}

#[test]
fn cfg17_call_fma_len1() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..3000 {
        let data = vec![rng.i32_corner()];
        assert_call_fma_eq(&data, 1);
    }
}

#[test]
fn cfg18_call_fma_tiny_lens() {
    let mut rng = Rng::new(SEED ^ 18);
    for len in 2i32..=5 {
        for _ in 0..1000 {
            let data: Vec<i32> = (0..len as usize).map(|_| rng.i32_full()).collect();
            assert_call_fma_eq(&data, len);
        }
    }
}

#[test]
fn cfg19_call_fma_random_lens_small() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..3000 {
        let len = 1 + rng.below(100);
        let data: Vec<i32> = (0..len).map(|_| rng.range_i32(-500, 500)).collect();
        assert_call_fma_eq(&data, len as i32);
        // len shorter than the buffer: only data[len-1] must be returned
        let shorter = 1 + rng.below(len);
        assert_call_fma_eq(&data, shorter as i32);
    }
}

#[test]
fn cfg20_call_fma_corners() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..3000 {
        let len = 1 + rng.below(100);
        let data: Vec<i32> = (0..len).map(|_| rng.i32_corner()).collect();
        assert_call_fma_eq(&data, len as i32);
    }
}

#[test]
fn cfg21_call_fma_len100() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..1000 {
        let data: Vec<i32> = (0..100).map(|_| rng.i32_full()).collect();
        assert_call_fma_eq(&data, 100);
    }
}

#[test]
fn cfg22_call_fma_large_lens() {
    // The C `call_fma` places three `int[len]` VLAs on the CALLER's stack
    // (~12 * len bytes), so the big lengths must run on a thread with a large
    // stack -- otherwise we would be measuring libtest's 2 MiB thread stack
    // rather than the translation.
    on_big_stack(|| {
        let mut rng = Rng::new(SEED ^ 22);
        for &len in &[1000usize, 4096, 100_000, 600_000, 1_500_000] {
            for _ in 0..3 {
                let data: Vec<i32> = (0..len).map(|_| rng.i32_full()).collect();
                assert_call_fma_eq(&data, len as i32);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// rows 23-33 : driver (top-level entry point, stdout compared byte-for-byte)
// ---------------------------------------------------------------------------

#[test]
fn cfg23_driver_single_token() {
    let mut rng = Rng::new(SEED ^ 23);
    let mut inputs: Vec<String> = [0i32, 1, 7, 42, -1, -7, i32::MAX, i32::MIN]
        .iter()
        .map(|v| v.to_string())
        .collect();
    for _ in 0..400 {
        inputs.push(rng.i32_corner().to_string());
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg24_driver_space_separated() {
    let mut rng = Rng::new(SEED ^ 24);
    let mut inputs: Vec<String> = Vec::new();
    for n in 2usize..=8 {
        for _ in 0..80 {
            let vals: Vec<i32> = (0..n).map(|_| rng.range_i32(-9999, 9999)).collect();
            inputs.push(join_space(&vals));
        }
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg25_driver_mixed_whitespace() {
    let mut rng = Rng::new(SEED ^ 25);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..600 {
        let n = 1 + rng.below(20);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push_str(&ws_run(&mut rng));
            }
            s.push_str(&rng.i32_corner().to_string());
        }
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg26_driver_leading_trailing_ws() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..600 {
        let n = 1 + rng.below(20);
        let mut s = ws_run(&mut rng);
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&rng.i32_full().to_string());
        }
        s.push_str(&ws_run(&mut rng));
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg27_driver_sign_and_leading_zeros() {
    let mut rng = Rng::new(SEED ^ 27);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..800 {
        let n = 1 + rng.below(20);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push_str(&ws_run(&mut rng));
            }
            let v = rng.range_i32(-99999, 99999);
            let zeros = "0".repeat(rng.below(4));
            match rng.below(3) {
                0 if v >= 0 => s.push_str(&format!("+{zeros}{v}")),
                1 if v >= 0 => s.push_str(&format!("{zeros}{v}")),
                _ if v < 0 => s.push_str(&format!("-{zeros}{}", v.unsigned_abs())),
                _ => s.push_str(&format!("{zeros}{v}")),
            }
        }
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg28_driver_full_range_values() {
    let mut rng = Rng::new(SEED ^ 28);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..800 {
        let n = 1 + rng.below(20);
        let vals: Vec<i32> = (0..n)
            .map(|_| match rng.below(4) {
                0 => i32::MIN,
                1 => i32::MAX,
                _ => rng.i32_full(),
            })
            .collect();
        inputs.push(join_space(&vals));
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg29_driver_count_cap_sweep() {
    let mut rng = Rng::new(SEED ^ 29);
    let mut inputs: Vec<String> = Vec::new();
    for &n in &[1usize, 2, 98, 99, 100, 101, 102, 150, 250] {
        for _ in 0..20 {
            let vals: Vec<i32> = (0..n).map(|_| rng.i32_full()).collect();
            inputs.push(join_space(&vals));
        }
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg30_driver_nonws_separator() {
    let mut rng = Rng::new(SEED ^ 30);
    const SEPS: [&str; 8] = [",", ";", ".", "/", ":", "|", "=", "_"];
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..1000 {
        let n = 2 + rng.below(11);
        let cut = rng.below(n); // token index after which the odd separator sits
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                if i == cut + 1 {
                    s.push_str(rng.pick(&SEPS));
                } else {
                    s.push_str(&ws_run(&mut rng));
                }
            }
            s.push_str(&rng.range_i32(-99999, 99999).to_string());
        }
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg31_driver_trailing_garbage() {
    let mut rng = Rng::new(SEED ^ 31);
    const TAIL: [&str; 12] = [
        "abc", "x10", "e5", "+", "-", "0x10", "..", "e", "E+", "#", "\u{7f}", "%d",
    ];
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..1000 {
        let n = 1 + rng.below(12);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push_str(&ws_run(&mut rng));
            }
            s.push_str(&rng.range_i32(-99999, 99999).to_string());
        }
        if rng.below(2) == 0 {
            s.push_str(&ws_run(&mut rng));
        }
        s.push_str(rng.pick(&TAIL));
        inputs.push(s);
    }
    // digits glued directly onto a token
    for base in ["12abc", "0x10", "007x", "-0b1", "5e3", "1.5", "9,9"] {
        inputs.push(base.to_string());
        inputs.push(format!("3 {base} 8"));
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg32_driver_oversized_input() {
    let mut rng = Rng::new(SEED ^ 32);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..10 {
        let mut s = String::with_capacity(110_000);
        while s.len() < 100_000 {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&rng.i32_full().to_string());
        }
        inputs.push(s);
    }
    assert_driver_batch_bytes(&inputs);
}

#[test]
fn cfg33_driver_fuzz() {
    let mut rng = Rng::new(SEED ^ 33);
    const JUNK: [&str; 14] = [
        "", "a", "z", "+", "-", "*", ",", ".", "x", "0x", "e", "\t", "\n", "  ",
    ];
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..2000 {
        let n = rng.below(141);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push_str(&ws_run(&mut rng));
            }
            match rng.below(8) {
                0 => s.push_str(&rng.i32_corner().to_string()),
                1 => s.push_str(&format!("{:+}", rng.range_i32(-1000, 1000))),
                2 => s.push_str(&format!("{}{}", "0".repeat(rng.below(5)), rng.below(1000))),
                3 => s.push_str(rng.pick(&JUNK)),
                4 => s.push_str(&format!("{}", u64::from(rng.next_u32()) + 2_147_483_648)),
                5 => s.push_str(&format!("-{}", u64::from(rng.next_u32()) + 2_147_483_649)),
                6 => s.push_str(&"9".repeat(1 + rng.below(25))),
                _ => s.push_str(&rng.i32_full().to_string()),
            }
        }
        if rng.below(4) == 0 {
            s.push_str(rng.pick(&JUNK));
        }
        // interior NULs cannot be expressed through `const char *`
        inputs.push(s.bytes().filter(|&b| b != 0).collect());
    }
    assert_driver_batch_bytes(&inputs);
}

// ---------------------------------------------------------------------------
// row 34 : the composed pipeline, all three exports together
// ---------------------------------------------------------------------------

#[test]
fn cfg34_pipeline_cross_check() {
    let mut rng = Rng::new(SEED ^ 34);
    let mut datasets: Vec<Vec<i32>> = Vec::new();
    for _ in 0..800 {
        let n = rng.below(130);
        datasets.push((0..n).map(|_| rng.i32_full()).collect());
    }

    // top level: one batched differential run over every dataset
    let texts: Vec<String> = datasets.iter().map(|v| join_space(v)).collect();
    let printed = assert_driver_batch_bytes(&texts);

    for (k, vals) in datasets.iter().enumerate() {
        // `driver` only ever consumes the first 100 tokens
        let capped = &vals[..vals.len().min(100)];

        // mid level, exactly as `driver` calls it
        assert_call_fma_eq(
            if capped.is_empty() { &[0] } else { capped },
            capped.len() as i32,
        );

        // low level: reproduce call_fma's own internals (ones / zeros) by hand
        if !capped.is_empty() {
            let ones = vec![1i32; capped.len()];
            let zeros = vec![0i32; capped.len()];
            assert_fma_array_eq(
                Alias::Distinct,
                capped.len() as i32,
                &vec![0i32; capped.len()],
                &ones,
                capped,
                &zeros,
            );
        }

        // the whole chain must agree on the printed value: driver prints
        // call_fma(data, min(n,100)) == data[min(n,100)-1], or 0 when empty
        let expect = if capped.is_empty() {
            "0\n".to_string()
        } else {
            format!("{}\n", capped[capped.len() - 1])
        };
        assert_eq!(
            String::from_utf8_lossy(&printed[k]),
            expect,
            "C pipeline value unexpected for n={}",
            vals.len()
        );
    }
}

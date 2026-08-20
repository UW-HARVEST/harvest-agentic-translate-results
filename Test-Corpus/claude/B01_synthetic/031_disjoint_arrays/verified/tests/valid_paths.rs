//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both implementations are loaded from
//! their `.so` via `libloading` and called through their C-ABI exports; the
//! Rust functions are never called directly. Every row is run against BOTH C
//! build variants (`-O0` and `-O2`, row C41) and uses many randomized inputs
//! from a fixed-seed PRNG so failures reproduce exactly.

mod common;

use common::*;
use std::os::raw::c_int;

const CANARY: i32 = 0x5A5A_5A5Au32 as i32;

/// Builds an `out` template of `n` canary words.
fn canary(n: usize) -> Vec<i32> {
    vec![CANARY; n]
}

// ===========================================================================
// C1 — fma_array, len == 0
// ===========================================================================
#[test]
fn cfg_c1_fma_array_len_zero() {
    let mut rng = Rng::new(0xC001);
    for (c, r) in pairs() {
        for n in [1usize, 2, 8, 64] {
            for _ in 0..200 {
                let m1 = rng.vec_i32(n);
                let m2 = rng.vec_i32(n);
                let ad = rng.vec_i32(n);
                let tmpl = canary(n);
                let (cv, rv) =
                    run_fma_array(&c, &r, &tmpl, &m1, &m2, &ad, 0, Alias::None);
                assert_eq!(cv, rv, "C1 {} vs {}", c.name, r.name);
                assert_eq!(cv, tmpl, "C1: len=0 must not write anything ({})", c.name);
            }
        }
    }
}

// ===========================================================================
// C2 — fma_array, len == 1, random values
// ===========================================================================
#[test]
fn cfg_c2_fma_array_len_one_random() {
    let mut rng = Rng::new(0xC002);
    for (c, r) in pairs() {
        for _ in 0..2000 {
            let m1 = rng.vec_i32(1);
            let m2 = rng.vec_i32(1);
            let ad = rng.vec_i32(1);
            assert_fma_array_eq(&c, &r, "C2", &canary(1), &m1, &m2, &ad, 1, Alias::None);
        }
    }
}

// ===========================================================================
// C3 — fma_array, len 2..=8
// ===========================================================================
#[test]
fn cfg_c3_fma_array_small_lens_random() {
    let mut rng = Rng::new(0xC003);
    for (c, r) in pairs() {
        for len in 2..=8i32 {
            let n = len as usize;
            for _ in 0..500 {
                let m1 = rng.vec_i32(n);
                let m2 = rng.vec_i32(n);
                let ad = rng.vec_i32(n);
                assert_fma_array_eq(
                    &c,
                    &r,
                    "C3",
                    &canary(n),
                    &m1,
                    &m2,
                    &ad,
                    len,
                    Alias::None,
                );
            }
        }
    }
}

// ===========================================================================
// C4 — fma_array, SIMD width boundaries
// ===========================================================================
#[test]
fn cfg_c4_fma_array_simd_boundary_lens() {
    let mut rng = Rng::new(0xC004);
    for (c, r) in pairs() {
        for len in [15i32, 16, 17, 31, 32, 33, 63, 64, 65] {
            let n = len as usize;
            for _ in 0..200 {
                let m1 = rng.vec_i32(n);
                let m2 = rng.vec_i32(n);
                let ad = rng.vec_i32(n);
                assert_fma_array_eq(
                    &c,
                    &r,
                    "C4",
                    &canary(n),
                    &m1,
                    &m2,
                    &ad,
                    len,
                    Alias::None,
                );
            }
        }
    }
}

// ===========================================================================
// C5 — fma_array, len == 100 (the `main` cap)
// ===========================================================================
#[test]
fn cfg_c5_fma_array_len_100_random() {
    let mut rng = Rng::new(0xC005);
    for (c, r) in pairs() {
        for _ in 0..300 {
            let m1 = rng.vec_i32(100);
            let m2 = rng.vec_i32(100);
            let ad = rng.vec_i32(100);
            assert_fma_array_eq(
                &c,
                &r,
                "C5",
                &canary(100),
                &m1,
                &m2,
                &ad,
                100,
                Alias::None,
            );
        }
    }
}

// ===========================================================================
// C6 — fma_array, large lengths
// ===========================================================================
#[test]
fn cfg_c6_fma_array_large_lens() {
    let mut rng = Rng::new(0xC006);
    for (c, r) in pairs() {
        for len in [1000i32, 65536] {
            let n = len as usize;
            for _ in 0..20 {
                let m1 = rng.vec_i32(n);
                let m2 = rng.vec_i32(n);
                let ad = rng.vec_i32(n);
                assert_fma_array_eq(
                    &c,
                    &r,
                    "C6",
                    &canary(n),
                    &m1,
                    &m2,
                    &ad,
                    len,
                    Alias::None,
                );
            }
        }
    }
}

// ===========================================================================
// C7 — fma_array, all zeros
// ===========================================================================
#[test]
fn cfg_c7_fma_array_all_zeros() {
    for (c, r) in pairs() {
        for len in 1..=64i32 {
            let n = len as usize;
            let z = vec![0i32; n];
            assert_fma_array_eq(&c, &r, "C7", &canary(n), &z, &z, &z, len, Alias::None);
        }
    }
}

// ===========================================================================
// C8 — fma_array, all ones
// ===========================================================================
#[test]
fn cfg_c8_fma_array_all_ones() {
    for (c, r) in pairs() {
        for len in 1..=64i32 {
            let n = len as usize;
            let o = vec![1i32; n];
            assert_fma_array_eq(&c, &r, "C8", &canary(n), &o, &o, &o, len, Alias::None);
        }
    }
}

// ===========================================================================
// C9 — fma_array, extreme values (signed multiply + add wraparound)
// ===========================================================================
#[test]
fn cfg_c9_fma_array_extreme_values() {
    let mut rng = Rng::new(0xC009);
    for (c, r) in pairs() {
        for _ in 0..2000 {
            let n = 1 + rng.below(17);
            let m1 = rng.vec_extreme_i32(n);
            let m2 = rng.vec_extreme_i32(n);
            let ad = rng.vec_extreme_i32(n);
            assert_fma_array_eq(
                &c,
                &r,
                "C9",
                &canary(n),
                &m1,
                &m2,
                &ad,
                n as c_int,
                Alias::None,
            );
        }
    }
}

// ===========================================================================
// C10 — fma_array, products chosen to overflow int
// ===========================================================================
#[test]
fn cfg_c10_fma_array_multiply_overflow() {
    let mut rng = Rng::new(0xC010);
    for (c, r) in pairs() {
        for _ in 0..2000 {
            let n = 1 + rng.below(9);
            // Magnitudes >= 2^16 so nearly every product overflows.
            let mk = |rng: &mut Rng, n: usize| -> Vec<i32> {
                (0..n)
                    .map(|_| {
                        let m = (rng.range(1 << 16, i32::MAX as i64)) as i32;
                        if rng.next_u64() & 1 == 0 {
                            m
                        } else {
                            m.wrapping_neg()
                        }
                    })
                    .collect()
            };
            let m1 = mk(&mut rng, n);
            let m2 = mk(&mut rng, n);
            let ad = rng.vec_i32(n);
            assert_fma_array_eq(
                &c,
                &r,
                "C10",
                &canary(n),
                &m1,
                &m2,
                &ad,
                n as c_int,
                Alias::None,
            );
        }
    }
}

// ===========================================================================
// C11 — fma_array, add-side overflow
// ===========================================================================
#[test]
fn cfg_c11_fma_array_add_overflow() {
    let mut rng = Rng::new(0xC011);
    for (c, r) in pairs() {
        for _ in 0..2000 {
            let n = 1 + rng.below(9);
            // Small products, extreme addends -> the `+ add[i]` overflows.
            let m1: Vec<i32> = (0..n).map(|_| rng.range(-1000, 1000) as i32).collect();
            let m2: Vec<i32> = (0..n).map(|_| rng.range(-1000, 1000) as i32).collect();
            let ad: Vec<i32> = (0..n)
                .map(|_| {
                    if rng.next_u64() & 1 == 0 {
                        i32::MAX - (rng.below(4) as i32)
                    } else {
                        i32::MIN + (rng.below(4) as i32)
                    }
                })
                .collect();
            assert_fma_array_eq(
                &c,
                &r,
                "C11",
                &canary(n),
                &m1,
                &m2,
                &ad,
                n as c_int,
                Alias::None,
            );
        }
    }
}

// ===========================================================================
// C12/C13/C14 — legal aliasing among the read-only pointers
// ===========================================================================
fn alias_row(seed: u64, label: &str, alias: Alias) {
    let mut rng = Rng::new(seed);
    for (c, r) in pairs() {
        for _ in 0..1000 {
            let n = 1 + rng.below(33);
            let m1 = rng.vec_i32(n);
            let m2 = rng.vec_i32(n);
            let ad = rng.vec_i32(n);
            assert_fma_array_eq(
                &c,
                &r,
                label,
                &canary(n),
                &m1,
                &m2,
                &ad,
                n as c_int,
                alias,
            );
        }
    }
}

#[test]
fn cfg_c12_fma_array_alias_mul1_mul2() {
    alias_row(0xC012, "C12", Alias::Mul1EqMul2);
}

#[test]
fn cfg_c13_fma_array_alias_mul2_add() {
    alias_row(0xC013, "C13", Alias::Mul2EqAdd);
}

#[test]
fn cfg_c14_fma_array_alias_all_inputs() {
    alias_row(0xC014, "C14", Alias::AllInputs);
}

// ===========================================================================
// C15 — fma_array, padded out buffer: nothing past len-1 may be written
// ===========================================================================
#[test]
fn cfg_c15_fma_array_out_padding_untouched() {
    let mut rng = Rng::new(0xC015);
    for (c, r) in pairs() {
        for _ in 0..1000 {
            let len = 1 + rng.below(32);
            let pad = 1 + rng.below(8);
            let n = len + pad;
            let m1 = rng.vec_i32(n);
            let m2 = rng.vec_i32(n);
            let ad = rng.vec_i32(n);
            let tmpl = canary(n);
            let (cv, rv) =
                run_fma_array(&c, &r, &tmpl, &m1, &m2, &ad, len as c_int, Alias::None);
            assert_eq!(cv, rv, "C15 {} vs {}: len={len} pad={pad}", c.name, r.name);
            assert!(
                cv[len..].iter().all(|&x| x == CANARY),
                "C15: {} wrote past len ({:?})",
                c.name,
                &cv[len..]
            );
            assert!(
                rv[len..].iter().all(|&x| x == CANARY),
                "C15: Rust wrote past len ({:?})",
                &rv[len..]
            );
        }
    }
}

// ===========================================================================
// C16 — fma_array, negative len: zero iterations, no writes
// ===========================================================================
#[test]
fn cfg_c16_fma_array_negative_len_no_writes() {
    let mut rng = Rng::new(0xC016);
    for (c, r) in pairs() {
        for len in [-1i32, -2, -7, -100, -65536, i32::MIN, i32::MIN + 1] {
            for _ in 0..50 {
                let n = 1 + rng.below(16);
                let m1 = rng.vec_i32(n);
                let m2 = rng.vec_i32(n);
                let ad = rng.vec_i32(n);
                let tmpl = canary(n);
                let (cv, rv) =
                    run_fma_array(&c, &r, &tmpl, &m1, &m2, &ad, len, Alias::None);
                assert_eq!(cv, rv, "C16 {} vs {}: len={len}", c.name, r.name);
                assert_eq!(cv, tmpl, "C16: negative len must not write ({})", c.name);
            }
        }
    }
}

// ===========================================================================
// C17 — fma_array, repeated calls into the same out buffer (statelessness)
// ===========================================================================
#[test]
fn cfg_c17_fma_array_repeated_calls() {
    let mut rng = Rng::new(0xC017);
    for (c, r) in pairs() {
        for _ in 0..500 {
            let n = 1 + rng.below(40);
            let m1 = rng.vec_i32(n);
            let m2 = rng.vec_i32(n);
            let ad = rng.vec_i32(n);
            let mut c_out = canary(n);
            let mut r_out = canary(n);
            // Increasing prefixes, all into the same buffer.
            for len in 1..=n {
                unsafe {
                    (c.fma_array)(
                        c_out.as_mut_ptr(),
                        m1.as_ptr(),
                        m2.as_ptr(),
                        ad.as_ptr(),
                        len as c_int,
                    );
                    (r.fma_array)(
                        r_out.as_mut_ptr(),
                        m1.as_ptr(),
                        m2.as_ptr(),
                        ad.as_ptr(),
                        len as c_int,
                    );
                }
                assert_eq!(
                    c_out, r_out,
                    "C17 {} vs {}: n={n} len={len}",
                    c.name, r.name
                );
            }
        }
    }
}

// ===========================================================================
// C18 — call_fma, len == 0
// ===========================================================================
#[test]
fn cfg_c18_call_fma_len_zero() {
    let mut rng = Rng::new(0xC018);
    for (c, r) in pairs() {
        for _ in 0..500 {
            let n = 1 + rng.below(16);
            let data = rng.vec_i32(n);
            assert_call_fma_eq(&c, &r, "C18", &data, 0);
            let v = unsafe { (r.call_fma)(data.as_ptr(), 0) };
            assert_eq!(v, 0, "C18: call_fma(_, 0) must be 0");
        }
    }
}

// ===========================================================================
// C19 — call_fma, len == 1
// ===========================================================================
#[test]
fn cfg_c19_call_fma_len_one_random() {
    let mut rng = Rng::new(0xC019);
    for (c, r) in pairs() {
        for _ in 0..2000 {
            let data = rng.vec_i32(1);
            assert_call_fma_eq(&c, &r, "C19", &data, 1);
        }
    }
}

// ===========================================================================
// C20 — call_fma, len 2..=8
// ===========================================================================
#[test]
fn cfg_c20_call_fma_small_lens_random() {
    let mut rng = Rng::new(0xC020);
    for (c, r) in pairs() {
        for len in 2..=8i32 {
            for _ in 0..500 {
                let data = rng.vec_i32(len as usize);
                assert_call_fma_eq(&c, &r, "C20", &data, len);
            }
        }
    }
}

// ===========================================================================
// C21 — call_fma, boundary lengths
// ===========================================================================
#[test]
fn cfg_c21_call_fma_boundary_lens() {
    let mut rng = Rng::new(0xC021);
    for (c, r) in pairs() {
        for len in [15i32, 16, 17, 31, 32, 33, 63, 64, 65, 100] {
            for _ in 0..200 {
                let data = rng.vec_i32(len as usize);
                assert_call_fma_eq(&c, &r, "C21", &data, len);
            }
        }
    }
}

// ===========================================================================
// C22 — call_fma, large lengths that still fit the C VLAs on the stack
// ===========================================================================
#[test]
fn cfg_c22_call_fma_large_lens() {
    // 12*len bytes of C VLAs land on the caller's stack, which libtest sizes at
    // only 2 MiB, so this row needs a bigger one.
    with_big_stack(|| {
        let mut rng = Rng::new(0xC022);
        for (c, r) in pairs() {
            for len in [1000i32, 65536, 200_000] {
                for _ in 0..10 {
                    let data = rng.vec_i32(len as usize);
                    assert_call_fma_eq(&c, &r, "C22", &data, len);
                }
            }
        }
    });
}

// ===========================================================================
// C23 — call_fma, data buffer longer than len
// ===========================================================================
#[test]
fn cfg_c23_call_fma_data_longer_than_len() {
    let mut rng = Rng::new(0xC023);
    for (c, r) in pairs() {
        for _ in 0..1000 {
            let len = 1 + rng.below(64);
            let extra = 1 + rng.below(64);
            let data = rng.vec_i32(len + extra);
            assert_call_fma_eq(&c, &r, "C23", &data, len as c_int);
        }
    }
}

// ===========================================================================
// C24 — call_fma, extreme-value data
// ===========================================================================
#[test]
fn cfg_c24_call_fma_extreme_values() {
    let mut rng = Rng::new(0xC024);
    for (c, r) in pairs() {
        for _ in 0..2000 {
            let len = 1 + rng.below(20);
            let data = rng.vec_extreme_i32(len);
            assert_call_fma_eq(&c, &r, "C24", &data, len as c_int);
        }
    }
}

// ===========================================================================
// C25 — call_fma, same buffer reused across calls with varying len
// ===========================================================================
#[test]
fn cfg_c25_call_fma_repeated_calls() {
    let mut rng = Rng::new(0xC025);
    for (c, r) in pairs() {
        for _ in 0..500 {
            let n = 1 + rng.below(48);
            let data = rng.vec_i32(n);
            for len in 0..=n {
                assert_call_fma_eq(&c, &r, "C25", &data, len as c_int);
            }
        }
    }
}

// ===========================================================================
// main() rows: C26..C38, driven through the exported `main` of each .so
// ===========================================================================

fn for_each_c_so(f: impl Fn(&std::path::Path, &str)) {
    for (tag, p) in c_so_variants() {
        f(&p, tag);
    }
}

// C26 — empty stdin
#[test]
fn cfg_c26_main_empty() {
    for_each_c_so(|so, tag| assert_main_eq(so, tag, b"", false));
}

// C27 — one integer, no trailing newline
#[test]
fn cfg_c27_main_single_random() {
    let mut rng = Rng::new(0xC027);
    let inputs: Vec<Vec<u8>> = (0..300)
        .map(|_| format!("{}", rng.next_i32()).into_bytes())
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C28 — one integer, with trailing newline
#[test]
fn cfg_c28_main_single_random_trailing_nl() {
    let mut rng = Rng::new(0xC028);
    let inputs: Vec<Vec<u8>> = (0..300)
        .map(|_| {
            let mut s = format!("{}", rng.next_i32()).into_bytes();
            s.extend(random_ws(&mut rng));
            s
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C29 — 2..10 integers, single spaces
#[test]
fn cfg_c29_main_multi_space() {
    let mut rng = Rng::new(0xC029);
    let inputs: Vec<Vec<u8>> = (0..200)
        .map(|_| {
            let n = 2 + rng.below(9);
            let toks: Vec<String> = (0..n).map(|_| format!("{}", rng.next_i32())).collect();
            toks.join(" ").into_bytes()
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C30 — 2..10 integers, random whitespace runs
#[test]
fn cfg_c30_main_multi_random_whitespace() {
    let mut rng = Rng::new(0xC030);
    let inputs: Vec<Vec<u8>> = (0..200)
        .map(|_| {
            let n = 2 + rng.below(9);
            let toks: Vec<String> = (0..n).map(|_| format!("{}", rng.next_i32())).collect();
            join_tokens(&mut rng, &toks, false, true)
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C31 — leading whitespace before the first integer
#[test]
fn cfg_c31_main_leading_whitespace() {
    let mut rng = Rng::new(0xC031);
    let inputs: Vec<Vec<u8>> = (0..200)
        .map(|_| {
            let n = 1 + rng.below(5);
            let toks: Vec<String> = (0..n).map(|_| format!("{}", rng.next_i32())).collect();
            join_tokens(&mut rng, &toks, true, true)
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C32 — 99 / 100 / 101 / 250 integers (the i < 100 cap boundary)
#[test]
fn cfg_c32_main_count_boundaries() {
    let mut rng = Rng::new(0xC032);
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for n in [99usize, 100, 101, 250] {
        for _ in 0..40 {
            let toks: Vec<String> = (0..n).map(|_| format!("{}", rng.next_i32())).collect();
            inputs.push(join_tokens(&mut rng, &toks, false, true));
        }
    }
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C33 — explicit signs, -0, leading zeros
#[test]
fn cfg_c33_main_sign_and_leading_zeros() {
    let mut rng = Rng::new(0xC033);
    let inputs: Vec<Vec<u8>> = (0..300)
        .map(|_| {
            let n = 1 + rng.below(6);
            let toks: Vec<String> = (0..n)
                .map(|_| {
                    let mag = rng.range(0, 2_147_483_647);
                    let zeros = "0".repeat(rng.below(6));
                    match rng.below(4) {
                        0 => format!("+{zeros}{mag}"),
                        1 => format!("-{zeros}{mag}"),
                        2 => format!("{zeros}{mag}"),
                        _ => "-0".to_string(),
                    }
                })
                .collect();
            join_tokens(&mut rng, &toks, false, true)
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C34 — magnitude boundaries in random positions
#[test]
fn cfg_c34_main_magnitude_boundaries() {
    let mut rng = Rng::new(0xC034);
    let inputs: Vec<Vec<u8>> = (0..300)
        .map(|_| {
            let n = 1 + rng.below(6);
            let toks: Vec<String> = (0..n)
                .map(|_| rng.pick(&BOUNDARY_TOKENS).to_string())
                .collect();
            join_tokens(&mut rng, &toks, false, true)
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C35 — very long digit runs (glibc saturation path)
#[test]
fn cfg_c35_main_long_digit_runs() {
    let mut rng = Rng::new(0xC035);
    let inputs: Vec<Vec<u8>> = (0..200)
        .map(|_| {
            let n = 1 + rng.below(3);
            let toks: Vec<String> = (0..n)
                .map(|_| {
                    let digits = *rng.pick(&[19usize, 20, 29, 40, 400]);
                    let mut s = String::new();
                    match rng.below(3) {
                        0 => s.push('-'),
                        1 => s.push('+'),
                        _ => {}
                    }
                    // Optional long leading-zero run.
                    if rng.next_u64() & 1 == 0 {
                        s.push_str(&"0".repeat(rng.below(30)));
                    }
                    s.push(char::from(b'1' + (rng.below(9) as u8)));
                    for _ in 1..digits {
                        s.push(char::from(b'0' + (rng.below(10) as u8)));
                    }
                    s
                })
                .collect();
            join_tokens(&mut rng, &toks, false, true)
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C36 — random byte soup over [0-9+- \t\n]
#[test]
fn cfg_c36_main_random_token_soup() {
    let mut rng = Rng::new(0xC036);
    const ALPHABET: &[u8] = b"0123456789+- \t\n0123456789";
    let inputs: Vec<Vec<u8>> = (0..600)
        .map(|_| {
            let n = rng.below(60);
            (0..n).map(|_| *rng.pick(ALPHABET)).collect()
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// C37 — stdin delivered one byte per write
#[test]
fn cfg_c37_main_byte_at_a_time_stdin() {
    let mut rng = Rng::new(0xC037);
    let inputs: Vec<Vec<u8>> = (0..100)
        .map(|_| {
            let n = 1 + rng.below(6);
            let toks: Vec<String> = (0..n)
                .map(|_| {
                    if rng.next_u64() & 3 == 0 {
                        rng.pick(&BOUNDARY_TOKENS).to_string()
                    } else {
                        format!("{}", rng.next_i32())
                    }
                })
                .collect();
            join_tokens(&mut rng, &toks, true, true)
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, true);
        }
    });
}

// C38 — stdin much larger than the 4096-byte read buffer
#[test]
fn cfg_c38_main_large_stdin() {
    let mut rng = Rng::new(0xC038);
    let inputs: Vec<Vec<u8>> = (0..40)
        .map(|_| {
            // ~2000 tokens of ~11 bytes each => well past 4096 bytes and past
            // the 100-integer cap, so the reader must stop mid-buffer.
            let toks: Vec<String> = (0..2000).map(|_| format!("{}", rng.next_i32())).collect();
            toks.join("\n").into_bytes()
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// ===========================================================================
// C39 — the two standalone driver executables, end to end
// ===========================================================================
#[test]
fn cfg_c39_driver_executables_end_to_end() {
    let mut rng = Rng::new(0xC039);
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" \t\n\r\x0b\x0c".to_vec(),
        b"42".to_vec(),
        b"1 2 3\n".to_vec(),
        b"7 abc 9".to_vec(),
        b"abc".to_vec(),
        b"0x1f".to_vec(),
        b"3.9".to_vec(),
        b"-".to_vec(),
        b"+".to_vec(),
    ];
    for t in BOUNDARY_TOKENS {
        inputs.push(t.as_bytes().to_vec());
    }
    for t in BAD_TOKENS {
        inputs.push(t.as_bytes().to_vec());
        inputs.push(format!("5 6 {t} 7").into_bytes());
    }
    for _ in 0..400 {
        let n = rng.below(12);
        let toks: Vec<String> = (0..n)
            .map(|_| match rng.below(4) {
                0 => rng.pick(&BOUNDARY_TOKENS).to_string(),
                1 => rng.pick(&BAD_TOKENS).to_string(),
                _ => format!("{}", rng.next_i32()),
            })
            .collect();
        inputs.push(join_tokens(&mut rng, &toks, true, true));
    }
    // A few counts around the 100 cap.
    for n in [99usize, 100, 101, 137] {
        let toks: Vec<String> = (0..n).map(|_| format!("{}", rng.next_i32())).collect();
        inputs.push(toks.join(" ").into_bytes());
    }
    for i in &inputs {
        assert_driver_eq(i, false);
    }
    // And a subset byte-at-a-time.
    for i in inputs.iter().take(40) {
        assert_driver_eq(i, true);
    }
}

// ===========================================================================
// C40 — interleaved calls to both exports on one loaded instance
// ===========================================================================
#[test]
fn cfg_c40_interleaved_exports() {
    let mut rng = Rng::new(0xC040);
    for (c, r) in pairs() {
        for _ in 0..500 {
            let n = 1 + rng.below(24);
            let m1 = rng.vec_i32(n);
            let m2 = rng.vec_i32(n);
            let ad = rng.vec_i32(n);
            let data = rng.vec_i32(n);

            // fma_array, then call_fma, then fma_array again -- the results
            // must not depend on ordering (no shared state in either build).
            assert_fma_array_eq(
                &c,
                &r,
                "C40a",
                &canary(n),
                &m1,
                &m2,
                &ad,
                n as c_int,
                Alias::None,
            );
            assert_call_fma_eq(&c, &r, "C40b", &data, n as c_int);
            assert_fma_array_eq(
                &c,
                &r,
                "C40c",
                &canary(n),
                &m1,
                &m2,
                &ad,
                n as c_int,
                Alias::None,
            );
            assert_call_fma_eq(&c, &r, "C40d", &data, 0);
            assert_call_fma_eq(&c, &r, "C40e", &data, n as c_int);
        }
    }
}

// ===========================================================================
// C43 — every one of the 256 possible bytes, used as a leading byte, as a
// separator between two integers, and as a trailing byte. This pins glibc's
// C-locale `isspace` set exhaustively instead of sampling it.
// ===========================================================================
#[test]
fn cfg_c43_main_every_byte_as_separator() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for b in 0u8..=255 {
        inputs.push(vec![b, b'7']); // leading
        inputs.push(vec![b'1', b, b'2']); // separator
        inputs.push(vec![b'9', b]); // trailing
        inputs.push(vec![b'-', b, b'5']); // right after a sign
    }
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// ===========================================================================
// C44 — unrestricted random byte fuzz (all 256 byte values), so nothing about
// the input alphabet is assumed.
// ===========================================================================
#[test]
fn cfg_c44_main_full_byte_fuzz() {
    let mut rng = Rng::new(0xC044);
    let inputs: Vec<Vec<u8>> = (0..400)
        .map(|_| {
            let n = rng.below(40);
            (0..n)
                .map(|_| {
                    // Bias towards digits/signs/whitespace so tokens actually
                    // form, while still emitting every possible byte.
                    if rng.next_u64().is_multiple_of(3) {
                        rng.next_u32() as u8
                    } else {
                        *rng.pick(b"0123456789+- \t\n\r\x0b\x0c")
                    }
                })
                .collect()
        })
        .collect();
    for_each_c_so(|so, tag| {
        for i in &inputs {
            assert_main_eq(so, tag, i, false);
        }
    });
}

// ===========================================================================
// C45 — stdin flavours other than a pipe: /dev/null, a seekable regular file
// (glibc buffers those differently from pipes), an empty regular file, and a
// write-only descriptor on fd 0 so every read fails with EBADF.
// ===========================================================================
#[test]
fn cfg_c45_main_unusual_stdin_kinds() {
    use std::process::Stdio;

    let tmp = std::env::temp_dir().join("fma_array_c45");
    std::fs::create_dir_all(&tmp).expect("create temp dir");

    // Regular seekable files with content, and one that is empty.
    let with_content = tmp.join("input.txt");
    std::fs::write(&with_content, b"11 22 33\n44 abc 55\n").expect("write input");
    let empty = tmp.join("empty.txt");
    std::fs::write(&empty, b"").expect("write empty");
    let wo = tmp.join("write_only.txt");

    let kinds: Vec<(&str, StdinFactory)> = vec![
        ("null", Box::new(Stdio::null)),
        (
            "regular-file",
            Box::new({
                let p = with_content.clone();
                move || Stdio::from(std::fs::File::open(&p).expect("open input"))
            }),
        ),
        (
            "empty-file",
            Box::new({
                let p = empty.clone();
                move || Stdio::from(std::fs::File::open(&p).expect("open empty"))
            }),
        ),
        (
            "write-only-fd",
            Box::new({
                let p = wo.clone();
                move || write_only_stdin(&p)
            }),
        ),
    ];

    for (kind, mk) in &kinds {
        // The two standalone driver executables.
        let c = run_with_stdio(&c_driver_exe(), &[], mk());
        let r = run_with_stdio(&rust_driver_exe(), &[], mk());
        assert_eq!(
            (c.stdout.clone(), c.code, c.signal),
            (r.stdout.clone(), r.code, r.signal),
            "C45 driver mismatch for stdin kind {kind}\n  C   : {}\n  Rust: {}",
            c.describe(),
            r.describe()
        );

        // And the exported `main` of each .so.
        let rp = run_with_stdio(
            &soprobe(),
            &[rust_so().display().to_string(), "main".to_string()],
            mk(),
        );
        for (tag, so) in c_so_variants() {
            let cp = run_with_stdio(
                &soprobe(),
                &[so.display().to_string(), "main".to_string()],
                mk(),
            );
            assert_eq!(
                (cp.stdout.clone(), cp.code, cp.signal),
                (rp.stdout.clone(), rp.code, rp.signal),
                "C45 main() mismatch for stdin kind {kind} (C[{tag}])\n  C   : {}\n  Rust: {}",
                cp.describe(),
                rp.describe()
            );
        }
    }
}

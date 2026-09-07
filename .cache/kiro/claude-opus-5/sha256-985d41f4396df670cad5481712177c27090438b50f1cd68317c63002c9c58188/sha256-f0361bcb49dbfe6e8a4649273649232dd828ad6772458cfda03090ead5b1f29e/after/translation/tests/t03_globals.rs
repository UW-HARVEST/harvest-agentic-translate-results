//! Phase B — the writable exported globals as a configuration axis.
//! Covers CONFIGS.md rows C32..C34.
//!
//! `cp_fixed_table`, `cp_len_base`, `cp_len_extra_bits`, `cp_dist_base`,
//! `cp_dist_extra_bits` and `cp_permutation_order` are all non-`const` `D`
//! section symbols, so an external caller can rewrite them and change how the
//! library decodes. Each mutation is applied inside the forked child, to that
//! library's own copy of the global, immediately before the call.

mod harness;

use harness::deflate::*;
use harness::{diff, pair, run, Lib, Outcome, Pair, Rng, OFF_IN, OFF_OUT};

/// Apply `mutate` to `lib`'s globals, then run `cp_inflate`, in one child.
#[track_caller]
fn diff_with_mutation<M>(
    p: &Pair,
    tag: &str,
    input: &[u8],
    out_bytes: i32,
    snap_len: usize,
    mutate: M,
) -> Vec<Outcome>
where
    M: Fn(&Lib),
{
    let sh = &p.shared;
    let mut outs = Vec::new();
    for lib in [&p.c, &p.rust] {
        sh.fill_pattern();
        sh.write(OFF_IN, input);
        outs.push(run(sh, lib, (OFF_OUT, snap_len), |l| unsafe {
            mutate(l);
            (l.cp_inflate)(
                sh.in_ptr(0) as *mut _,
                input.len() as i32,
                sh.out_ptr(0) as *mut _,
                out_bytes,
            )
        }));
    }
    diff(tag, &outs[0], &outs[1]);
    outs
}

// ---------------------------------------------------------------------------
// C32 — mutated cp_fixed_table.
// ---------------------------------------------------------------------------

/// A *complete* replacement fixed table: literals 0..=255 get 9-bit codes
/// (256 * 2^-9 = 1/2) and 256..=287 get 6-bit codes (32 * 2^-6 = 1/2), summing
/// to exactly 1. Distances keep 32 x 5-bit codes.
fn alt_fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![9u8; 288];
    for i in 256..288 {
        v[i] = 6;
    }
    v
}

#[test]
fn c32_mutated_fixed_table_still_decodes() {
    let p = pair();
    let mut rng = Rng::new(0xF1_7AB1E_0000_0032);
    let lit = alt_fixed_lit_lens();
    let dist = vec![5u8; 32];
    assert!(is_complete(&lit) && is_complete(&dist));

    for rep in 0..30 {
        let toks = random_toks(&mut rng, 8 + rep, rep % 2 == 0, 32);
        let expect = expand(&toks);
        let mut e = Enc::new();
        e.fixed_block_with(true, &lit, &dist, &toks);
        let input = e.finish();

        let lit_c = lit.clone();
        let dist_c = dist.clone();
        let outs = diff_with_mutation(
            &p,
            &format!("C32/complete-swap rep={rep}"),
            &input,
            expect.len() as i32,
            expect.len() + 64,
            move |l| unsafe {
                for i in 0..288 {
                    *l.cp_fixed_table.add(i) = lit_c[i];
                }
                for i in 0..32 {
                    *l.cp_fixed_table.add(288 + i) = dist_c[i];
                }
            },
        );
        // Both must have *succeeded* with the replacement table, otherwise this
        // row is not actually exercising the mutated-table decode path.
        assert!(outs[0].completed && outs[0].ret == 1, "{:?}", outs[0]);
        assert_eq!(&outs[0].snap[..expect.len()], &expect[..]);
    }
}

#[test]
fn c32_mutated_fixed_table_arbitrary() {
    let p = pair();
    let mut rng = Rng::new(0xF1_7AB1E_0000_0033);
    // A normally-encoded fixed block decoded against a scrambled table: the two
    // libraries must agree on whatever happens (usually the cp_decode assert).
    let toks = random_toks(&mut rng, 20, true, 16);
    let mut e = Enc::new();
    e.fixed_block(true, &toks);
    let input = e.finish();

    let mutations: Vec<(&str, Box<dyn Fn(&Lib) + Send + Sync>)> = vec![
        (
            "table[0]=9",
            Box::new(|l: &Lib| unsafe { *l.cp_fixed_table.add(0) = 9 }),
        ),
        (
            "table[0]=15",
            Box::new(|l: &Lib| unsafe { *l.cp_fixed_table.add(0) = 15 }),
        ),
        (
            "table[0]=0",
            Box::new(|l: &Lib| unsafe { *l.cp_fixed_table.add(0) = 0 }),
        ),
        (
            "table[256]=0 (drop EOB)",
            Box::new(|l: &Lib| unsafe { *l.cp_fixed_table.add(256) = 0 }),
        ),
        (
            "all lit lens = 9",
            Box::new(|l: &Lib| unsafe {
                for i in 0..288 {
                    *l.cp_fixed_table.add(i) = 9;
                }
            }),
        ),
        (
            "all lit lens = 0",
            Box::new(|l: &Lib| unsafe {
                for i in 0..288 {
                    *l.cp_fixed_table.add(i) = 0;
                }
            }),
        ),
        (
            "all dist lens = 0",
            Box::new(|l: &Lib| unsafe {
                for i in 288..320 {
                    *l.cp_fixed_table.add(i) = 0;
                }
            }),
        ),
        (
            "rotate whole table by 1",
            Box::new(|l: &Lib| unsafe {
                let first = *l.cp_fixed_table.add(0);
                for i in 0..319 {
                    *l.cp_fixed_table.add(i) = *l.cp_fixed_table.add(i + 1);
                }
                *l.cp_fixed_table.add(319) = first;
            }),
        ),
    ];

    let sh = &p.shared;
    for (name, m) in &mutations {
        let mut outs = Vec::new();
        for lib in [&p.c, &p.rust] {
            sh.fill_pattern();
            sh.write(OFF_IN, &input);
            outs.push(run(sh, lib, (OFF_OUT, 512), |l| unsafe {
                m(l);
                (l.cp_inflate)(
                    sh.in_ptr(0) as *mut _,
                    input.len() as i32,
                    sh.out_ptr(0) as *mut _,
                    256,
                )
            }));
        }
        diff(&format!("C32/{name}"), &outs[0], &outs[1]);
    }
}

/// ERRORS.md E9 / E39: `assert(len < 16)` in `cp_build` is reachable only by
/// mutating `cp_fixed_table`. 15 must be accepted, 16 must abort.
#[test]
fn e9_e39_cp_build_len_boundary() {
    let p = pair();
    let mut rng = Rng::new(0xB0_0000_0000_0009);
    let toks = random_toks(&mut rng, 10, false, 1);
    let mut e = Enc::new();
    e.fixed_block(true, &toks);
    let input = e.finish();
    let sh = &p.shared;

    for &v in &[15u8, 16u8, 17, 200, 255] {
        let mut outs = Vec::new();
        for lib in [&p.c, &p.rust] {
            sh.fill_pattern();
            sh.write(OFF_IN, &input);
            outs.push(run(sh, lib, (OFF_OUT, 512), |l| unsafe {
                *l.cp_fixed_table.add(3) = v;
                (l.cp_inflate)(
                    sh.in_ptr(0) as *mut _,
                    input.len() as i32,
                    sh.out_ptr(0) as *mut _,
                    256,
                )
            }));
        }
        diff(&format!("E39/cp_fixed_table[3]={v}"), &outs[0], &outs[1]);
        if v >= 16 {
            assert!(
                !outs[0].completed && outs[0].signal == libc::SIGABRT,
                "cp_fixed_table[3]={v} should trip assert(len < 16): {:?}",
                outs[0]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C33 — mutated length/distance base and extra-bit tables.
// ---------------------------------------------------------------------------

#[test]
fn c33_mutated_len_dist_tables() {
    let p = pair();
    let mut rng = Rng::new(0xBA5E_0000_0000_0033);
    // fixed block containing matches over several length/distance symbols
    let mut toks: Vec<Tok> = (0..64).map(|_| Tok::Lit(rng.u8())).collect();
    for ls in [0usize, 1, 8, 12, 28] {
        toks.push(Tok::RawMatch {
            len_sym: ls,
            len_extra: 0,
            dist_sym: 3,
            dist_extra: 0,
        });
    }
    let mut e = Enc::new();
    e.fixed_block(true, &toks);
    let input = e.finish();
    let sh = &p.shared;

    let mutations: Vec<(&str, Box<dyn Fn(&Lib)>)> = vec![
        (
            "cp_len_base[0]=100",
            Box::new(|l: &Lib| unsafe { *l.cp_len_base.add(0) = 100 }),
        ),
        (
            "cp_len_base[0]=0",
            Box::new(|l: &Lib| unsafe { *l.cp_len_base.add(0) = 0 }),
        ),
        (
            "cp_len_base[*] = 3",
            Box::new(|l: &Lib| unsafe {
                for i in 0..31 {
                    *l.cp_len_base.add(i) = 3;
                }
            }),
        ),
        (
            "cp_len_extra_bits[0]=3",
            Box::new(|l: &Lib| unsafe { *l.cp_len_extra_bits.add(0) = 3 }),
        ),
        (
            "cp_len_extra_bits[*] = 0",
            Box::new(|l: &Lib| unsafe {
                for i in 0..31 {
                    *l.cp_len_extra_bits.add(i) = 0;
                }
            }),
        ),
        (
            "cp_dist_base[3]=1",
            Box::new(|l: &Lib| unsafe { *l.cp_dist_base.add(3) = 1 }),
        ),
        (
            "cp_dist_base[3]=100000",
            Box::new(|l: &Lib| unsafe { *l.cp_dist_base.add(3) = 100_000 }),
        ),
        (
            "cp_dist_extra_bits[3]=5",
            Box::new(|l: &Lib| unsafe { *l.cp_dist_extra_bits.add(3) = 5 }),
        ),
        (
            "cp_dist_extra_bits[*] = 0",
            Box::new(|l: &Lib| unsafe {
                for i in 0..32 {
                    *l.cp_dist_extra_bits.add(i) = 0;
                }
            }),
        ),
    ];

    for (name, m) in &mutations {
        for out_bytes in [256i32, 4096] {
            let mut outs = Vec::new();
            for lib in [&p.c, &p.rust] {
                sh.fill_pattern();
                sh.write(OFF_IN, &input);
                outs.push(run(sh, lib, (OFF_OUT, 8192), |l| unsafe {
                    m(l);
                    (l.cp_inflate)(
                        sh.in_ptr(0) as *mut _,
                        input.len() as i32,
                        sh.out_ptr(0) as *mut _,
                        out_bytes,
                    )
                }));
            }
            diff(&format!("C33/{name} out={out_bytes}"), &outs[0], &outs[1]);
        }
    }
}

// ---------------------------------------------------------------------------
// C34 — mutated cp_permutation_order (affects cp_dynamic only).
// ---------------------------------------------------------------------------

#[test]
fn c34_mutated_permutation_order() {
    let p = pair();
    let mut rng = Rng::new(0x9E64_0000_0000_0034);
    let toks = random_toks(&mut rng, 30, true, 32);
    let (l, d) = alphabets_for(&toks, 288, 32);
    let mut e = Enc::new();
    e.dynamic_block(true, &l, &d, &toks, RleMode::All);
    let input = e.finish();
    let sh = &p.shared;

    let mutations: Vec<(&str, Box<dyn Fn(&Lib)>)> = vec![
        (
            "identity permutation",
            Box::new(|l: &Lib| unsafe {
                for i in 0..19 {
                    *l.cp_permutation_order.add(i) = i as u8;
                }
            }),
        ),
        (
            "rotate by 1",
            Box::new(|l: &Lib| unsafe {
                let f = *l.cp_permutation_order.add(0);
                for i in 0..18 {
                    *l.cp_permutation_order.add(i) = *l.cp_permutation_order.add(i + 1);
                }
                *l.cp_permutation_order.add(18) = f;
            }),
        ),
        (
            "all zeros",
            Box::new(|l: &Lib| unsafe {
                for i in 0..19 {
                    *l.cp_permutation_order.add(i) = 0;
                }
            }),
        ),
        (
            "all 18",
            Box::new(|l: &Lib| unsafe {
                for i in 0..19 {
                    *l.cp_permutation_order.add(i) = 18;
                }
            }),
        ),
    ];

    for (name, m) in &mutations {
        let mut outs = Vec::new();
        for lib in [&p.c, &p.rust] {
            sh.fill_pattern();
            sh.write(OFF_IN, &input);
            outs.push(run(sh, lib, (OFF_OUT, 4096), |l| unsafe {
                m(l);
                (l.cp_inflate)(
                    sh.in_ptr(0) as *mut _,
                    input.len() as i32,
                    sh.out_ptr(0) as *mut _,
                    2048,
                )
            }));
        }
        diff(&format!("C34/{name}"), &outs[0], &outs[1]);
    }
}

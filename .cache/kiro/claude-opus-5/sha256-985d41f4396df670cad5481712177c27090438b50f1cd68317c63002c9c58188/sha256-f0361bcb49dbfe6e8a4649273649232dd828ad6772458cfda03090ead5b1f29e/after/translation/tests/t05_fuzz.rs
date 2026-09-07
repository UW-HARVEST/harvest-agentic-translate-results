//! Phase B row C35 — structured-random fuzzing of both entry points.
//!
//! Compares return value, `cp_error_reason`, the entire output buffer, and the
//! termination signal. A one-sided watchdog kill is re-checked with a long budget
//! before being reported, so timing cannot produce a false divergence.

mod harness;

use core::ffi::c_void;
use harness::deflate::*;
use harness::{
    diff_or_retry, fnv, pair, run, Lib, Outcome, Pair, Rng, OFF_IN, OFF_OUT, OUT_CAP,
};

fn run_inflate(p: &Pair, lib: &Lib, input: &[u8], align: usize, in_bytes: i32, out_bytes: i32, snap_len: usize) -> Outcome {
    let sh = &p.shared;
    sh.fill_pattern();
    sh.write(OFF_IN + align, input);
    run(sh, lib, (OFF_OUT, snap_len), |l| unsafe {
        (l.cp_inflate)(
            sh.in_ptr(align) as *mut c_void,
            in_bytes,
            sh.out_ptr(0) as *mut c_void,
            out_bytes,
        )
    })
}

/// C35a — completely random byte strings.
#[test]
fn c35a_random_bytes() {
    let p = pair();
    let mut rng = Rng::new(0xF011_0000_0000_35A0);
    for i in 0..8000 {
        let n = 1 + rng.below(48);
        let input = rng.bytes(n);
        let align = rng.below(4);
        let out_bytes = *[0i32, 1, 3, 64, 1024, 8192].get(rng.below(6)).unwrap();
        let snap_len = (out_bytes.max(0) as usize + 128).min(OUT_CAP);
        let tag = format!(
            "C35a/#{i} n={n} align={align} out={out_bytes} sha={:016x}",
            fnv(&input)
        );
        diff_or_retry(&tag, || {
            (
                run_inflate(&p, &p.c, &input, align, n as i32, out_bytes, snap_len),
                run_inflate(&p, &p.rust, &input, align, n as i32, out_bytes, snap_len),
            )
        });
    }
}

/// C35b — random bytes with a *valid-looking* block header, so the decoders get
/// past `btype` dispatch far more often than by chance.
#[test]
fn c35b_random_with_valid_header() {
    let p = pair();
    let mut rng = Rng::new(0x1717_0000_0000_35B0);
    for btype in 0u8..4 {
        for i in 0..2500 {
            let n = 2 + rng.below(64);
            let mut input = rng.bytes(n);
            input[0] = (input[0] & !0x07) | 0x01 | (btype << 1);
            let align = rng.below(4);
            let out_bytes = *[0i32, 1, 17, 256, 4096].get(rng.below(5)).unwrap();
            let snap_len = (out_bytes.max(0) as usize + 128).min(OUT_CAP);
            let tag = format!(
                "C35b/btype={btype} #{i} n={n} align={align} out={out_bytes} sha={:016x}",
                fnv(&input)
            );
            diff_or_retry(&tag, || {
                (
                    run_inflate(&p, &p.c, &input, align, n as i32, out_bytes, snap_len),
                    run_inflate(&p, &p.rust, &input, align, n as i32, out_bytes, snap_len),
                )
            });
        }
    }
}

/// C35c — well-formed streams that are then corrupted by a single bit flip. This
/// reaches deep decoder state that pure random bytes rarely do.
#[test]
fn c35c_bit_flipped_valid_streams() {
    let p = pair();
    let mut rng = Rng::new(0x8177_0000_0000_35C0);
    for round in 0..250 {
        // build a well-formed stream
        let ntok = 12 + rng.below(30);
        let toks = random_toks(&mut rng, ntok, true, 64);
        let mut e = Enc::new();
        match round % 3 {
            0 => e.fixed_block(true, &toks),
            1 => {
                let (l, d) = alphabets_for(&toks, 288, 32);
                e.dynamic_block(true, &l, &d, &toks, RleMode::All)
            }
            _ => {
                let (l, d) = alphabets_for(&toks, 288, 32);
                e.dynamic_block(false, &l, &d, &toks, RleMode::None);
                e.stored_block(true, &rng.bytes(4))
            }
        }
        let base = e.finish();
        let total_bits = base.len() * 8;
        for _ in 0..12 {
            let bit = rng.below(total_bits);
            let mut input = base.clone();
            input[bit / 8] ^= 1 << (bit % 8);
            let align = rng.below(4);
            let out_bytes = *[8i32, 64, 4096].get(rng.below(3)).unwrap();
            let snap_len = out_bytes as usize + 128;
            let tag = format!(
                "C35c/round={round} bit={bit} align={align} out={out_bytes} sha={:016x}",
                fnv(&input)
            );
            diff_or_retry(&tag, || {
                (
                    run_inflate(&p, &p.c, &input, align, input.len() as i32, out_bytes, snap_len),
                    run_inflate(&p, &p.rust, &input, align, input.len() as i32, out_bytes, snap_len),
                )
            });
        }
    }
}

/// C35d — well-formed streams truncated to every possible length.
#[test]
fn c35d_truncated_valid_streams() {
    let p = pair();
    let mut rng = Rng::new(0x7A0C_0000_0000_35D0);
    for round in 0..40 {
        let toks = random_toks(&mut rng, 20, true, 32);
        let mut e = Enc::new();
        if round % 2 == 0 {
            e.fixed_block(true, &toks);
        } else {
            let (l, d) = alphabets_for(&toks, 288, 32);
            e.dynamic_block(true, &l, &d, &toks, RleMode::All);
        }
        let base = e.finish();
        for cut in 1..=base.len() {
            let align = cut % 4;
            let tag = format!("C35d/round={round} cut={cut} align={align}");
            let input = base[..cut].to_vec();
            diff_or_retry(&tag, || {
                (
                    run_inflate(&p, &p.c, &input, align, cut as i32, 4096, 4224),
                    run_inflate(&p, &p.rust, &input, align, cut as i32, 4096, 4224),
                )
            });
        }
    }
}

/// C35e — `unfilter` with fully random arguments, including out-of-range filter
/// bytes and hostile shapes, over a padded buffer.
#[test]
fn c35e_unfilter_random_everything() {
    let p = pair();
    let sh = &p.shared;
    let mut rng = Rng::new(0x0F17_0000_0000_35E0);
    const PAD: usize = 8192;
    for i in 0..8000 {
        let w = rng.range(-4, 40) as i32;
        let h = rng.range(-3, 10) as i32;
        let bpp = rng.range(-4, 10) as i32;
        let data: Vec<u8> = (0..2048).map(|_| rng.u8()).collect();
        let raw_off = OFF_OUT + PAD;
        let snap = (OFF_OUT, PAD + data.len() + 4096);
        let tag = format!("C35e/#{i} w={w},h={h},bpp={bpp}");
        diff_or_retry(&tag, || {
            let mut outs = Vec::new();
            for lib in [&p.c, &p.rust] {
                sh.fill_pattern();
                sh.write(raw_off, &data);
                outs.push(run(sh, lib, snap, |l| unsafe {
                    (l.unfilter)(w, h, bpp, sh.at(raw_off))
                }));
            }
            (outs[0].clone(), outs[1].clone())
        });
    }
}

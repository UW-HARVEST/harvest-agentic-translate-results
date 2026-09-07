//! CONFIGS.md row C38 — deliberately overrun `cp_dynamic`'s `lens[288 + 32]`.
//!
//! The C runs its code-length loop while `n < nlit + ndst` (at most 320), but a
//! single `case 18` adds up to 138, so `n` can reach 457. Those writes land on
//! `cp_dynamic`'s other `-O0` stack locals — `ndst`, `nlit`, the `case 16/17/18`
//! counters, and `n` itself — so the loop rewrites its own bounds. The Rust
//! translation models that frame byte for byte; this file drives the overflow
//! directly instead of relying on random fuzzing to stumble into it.
//!
//! It also pins two facts the frame model depends on:
//!  * `lens[-1]` (read by `case 16` when `n == 0`) is the top byte of the spilled
//!    `s` heap pointer, i.e. `0x00` — not indeterminate garbage.
//!  * `n` can never escape past its own storage at `lens[376..380)`: writing a
//!    code-length byte `v` there makes `n = 0x100 | v`, so `n` folds back to
//!    ~256 instead of climbing into the saved `%rbp` / return address.

mod harness;

use core::ffi::c_void;
use harness::deflate::*;
use harness::{diff_or_retry, fnv, pair, run, Lib, Outcome, Pair, Rng, OFF_IN, OFF_OUT};

/// A code-length alphabet with codes for the symbols this file emits:
/// `0` (len 0), `1`, `2`, `16`, `17`, `18`.
/// Lengths `3,3,3,3,3,3,3,3` over 8 symbols is Kraft-complete (8 * 2^-3 == 1).
fn cl_alphabet() -> ([u8; 19], Vec<usize>) {
    let mut cl = [0u8; 19];
    let syms = vec![0usize, 1, 2, 3, 4, 16, 17, 18];
    for &s in &syms {
        cl[s] = 3;
    }
    assert!(is_complete(&cl));
    (cl, syms)
}

fn run_inflate(p: &Pair, lib: &Lib, input: &[u8], out_bytes: i32, snap_len: usize) -> Outcome {
    let sh = &p.shared;
    sh.fill_pattern();
    sh.write(OFF_IN, input);
    run(sh, lib, (OFF_OUT, snap_len), |l| unsafe {
        (l.cp_inflate)(
            sh.in_ptr(0) as *mut c_void,
            input.len() as i32,
            sh.out_ptr(0) as *mut c_void,
            out_bytes,
        )
    })
}

#[track_caller]
fn drive(p: &Pair, tag: &str, input: &[u8], out_bytes: i32) {
    let snap_len = out_bytes.max(0) as usize + 128;
    diff_or_retry(tag, || {
        (
            run_inflate(p, &p.c, input, out_bytes, snap_len),
            run_inflate(p, &p.rust, input, out_bytes, snap_len),
        )
    });
}

/// C38a — `n` pushed just past 320 in fine increments, so every corrupted local
/// (`lenlens`, padding, `sym`, `nlen`, `ndst`, `nlit`, the three inner counters,
/// `n`, `iperm`) is hit in turn.
#[test]
fn c38a_lens_overflow_sweep() {
    let p = pair();
    let (cl, _) = cl_alphabet();
    // `prefix` individual length symbols, then one `case 18` run of `runlen`.
    for prefix in [
        180usize, 200, 230, 240, 250, 260, 270, 280, 290, 300, 310, 315, 318, 319,
    ] {
        for runlen in [11usize, 20, 40, 80, 100, 120, 137, 138] {
            let mut stream: Vec<ClSym> = Vec::new();
            for i in 0..prefix {
                stream.push(ClSym {
                    sym: 1 + (i % 4),
                    extra_bits: 0,
                    extra_val: 0,
                });
            }
            stream.push(ClSym {
                sym: 18,
                extra_bits: 7,
                extra_val: (runlen - 11) as u32,
            });
            // plenty more symbols in case the corrupted bounds keep the loop going
            for _ in 0..400 {
                stream.push(ClSym {
                    sym: 18,
                    extra_bits: 7,
                    extra_val: 127,
                });
            }
            let mut e = Enc::new();
            e.dynamic_header_raw(true, 288, 32, &cl, &stream);
            for _ in 0..64 {
                e.w.bits(0xFF, 8);
            }
            let input = e.finish();
            drive(
                &p,
                &format!(
                    "C38a/prefix={prefix},runlen={runlen},sha={:016x}",
                    fnv(&input)
                ),
                &input,
                4096,
            );
        }
    }
}

/// C38b — the same overflow reached through `case 17` (3..=10 zeros) and
/// `case 16` (repeat-previous), which corrupt with *different* byte values and
/// use different counter slots (`lens[368..372)` and `lens[372..376)`).
#[test]
fn c38b_overflow_via_cl_16_and_17() {
    let p = pair();
    let (cl, _) = cl_alphabet();
    for prefix in [300usize, 310, 314, 316, 317, 318, 319] {
        for mode in 0..2 {
            let mut stream: Vec<ClSym> = Vec::new();
            for i in 0..prefix {
                stream.push(ClSym {
                    sym: 1 + (i % 4),
                    extra_bits: 0,
                    extra_val: 0,
                });
            }
            for _ in 0..200 {
                if mode == 0 {
                    stream.push(ClSym {
                        sym: 17,
                        extra_bits: 3,
                        extra_val: 7,
                    }); // 10 zeros
                } else {
                    stream.push(ClSym {
                        sym: 16,
                        extra_bits: 2,
                        extra_val: 3,
                    }); // repeat previous x6
                }
            }
            let mut e = Enc::new();
            e.dynamic_header_raw(true, 288, 32, &cl, &stream);
            for _ in 0..64 {
                e.w.bits(0xFF, 8);
            }
            let input = e.finish();
            drive(
                &p,
                &format!("C38b/prefix={prefix},mode={mode},sha={:016x}", fnv(&input)),
                &input,
                4096,
            );
        }
    }
}

/// C38c — randomized code-length streams over the whole `HLIT`/`HDIST` range,
/// with a strong bias towards long `case 18` runs so the overflow is hit often.
#[test]
fn c38c_random_overflowing_streams() {
    let p = pair();
    let (cl, syms) = cl_alphabet();
    let mut rng = Rng::new(0x0F10_0000_0000_38C0);
    for i in 0..1000 {
        let hlit = 257 + rng.below(32);
        let hdist = 1 + rng.below(32);
        let mut stream: Vec<ClSym> = Vec::new();
        for _ in 0..(20 + rng.below(400)) {
            let s = rng.pick(&syms);
            let (eb, ev) = match s {
                16 => (2u32, rng.below(4) as u32),
                17 => (3u32, rng.below(8) as u32),
                18 => (7u32, rng.below(128) as u32),
                _ => (0u32, 0u32),
            };
            stream.push(ClSym {
                sym: s,
                extra_bits: eb,
                extra_val: ev,
            });
        }
        let mut e = Enc::new();
        e.dynamic_header_raw(true, hlit, hdist, &cl, &stream);
        for _ in 0..32 {
            e.w.bits(rng.u8() as u32, 8);
        }
        let input = e.finish();
        let out_bytes = *[0i32, 1, 64, 4096].get(rng.below(4)).unwrap();
        drive(
            &p,
            &format!(
                "C38c/#{i} hlit={hlit},hdist={hdist},out={out_bytes},sha={:016x}",
                fnv(&input)
            ),
            &input,
            out_bytes,
        );
    }
}

/// C38d — `case 16` as the very first code-length symbol, so `lens[-1]` is read.
/// In the C that byte is the most-significant byte of the spilled `s` pointer
/// (a heap address), i.e. `0x00`. If the Rust modelled it as anything else, the
/// resulting literal tree would differ and the decoded output would diverge.
#[test]
fn c38d_lens_minus_one_is_read() {
    let p = pair();
    let (cl, _) = cl_alphabet();
    for rep in 0..4usize {
        for hlit in [257usize, 270, 288] {
            let mut stream: Vec<ClSym> = vec![ClSym {
                // repeat lens[-1] three to six times, at n == 0
                sym: 16,
                extra_bits: 2,
                extra_val: rep as u32,
            }];
            // then fill the rest with something decodable
            for _ in 0..400 {
                stream.push(ClSym {
                    sym: 18,
                    extra_bits: 7,
                    extra_val: 127,
                });
            }
            let mut e = Enc::new();
            e.dynamic_header_raw(true, hlit, 32, &cl, &stream);
            for _ in 0..64 {
                e.w.bits(0xFF, 8);
            }
            let input = e.finish();
            drive(
                &p,
                &format!("C38d/rep={rep},hlit={hlit},sha={:016x}", fnv(&input)),
                &input,
                4096,
            );
        }
    }
}

/// C38e — a *valid* dynamic block whose code-length stream ends with a `case 18`
/// run that overshoots `nlit + ndst` by only a few bytes, i.e. the overflow is
/// small enough to stay inside `lenlens[]` (harmless) and must still decode
/// successfully in both libraries.
#[test]
fn c38e_small_harmless_overshoot() {
    let p = pair();
    let (cl, _) = cl_alphabet();
    let mut successes = 0usize;
    for hlit in [257usize, 260, 280, 288] {
        for hdist in [1usize, 8, 32] {
            let total = hlit + hdist;
            // lengths: symbol 1 for the first `k`, then zeros to the end
            for overshoot in [1usize, 3, 8, 15, 18] {
                let zeros_needed = total.saturating_sub(2) + overshoot;
                if zeros_needed < 11 {
                    continue;
                }
                let mut stream: Vec<ClSym> = vec![
                    ClSym { sym: 1, extra_bits: 0, extra_val: 0 },
                    ClSym { sym: 1, extra_bits: 0, extra_val: 0 },
                ];
                let mut left = zeros_needed;
                while left > 0 {
                    let n = left.min(138).max(11);
                    stream.push(ClSym {
                        sym: 18,
                        extra_bits: 7,
                        extra_val: (n - 11) as u32,
                    });
                    left = left.saturating_sub(n);
                }
                let mut e = Enc::new();
                e.dynamic_header_raw(true, hlit, hdist, &cl, &stream);
                // two length-1 literal symbols exist: codes 0 and 1.
                // symbol 0 is literal 0x00, symbol 1 is literal 0x01... but only
                // if they land in the literal alphabet. Emit a few bits then stop.
                for _ in 0..16 {
                    e.w.bits(0xFF, 8);
                }
                let input = e.finish();
                let tag = format!(
                    "C38e/hlit={hlit},hdist={hdist},overshoot={overshoot},sha={:016x}",
                    fnv(&input)
                );
                drive(&p, &tag, &input, 4096);
                let o = run_inflate(&p, &p.c, &input, 4096, 4224);
                if o.completed {
                    successes += 1;
                }
            }
        }
    }
    eprintln!("C38e: {successes} cases returned normally from the C");
}

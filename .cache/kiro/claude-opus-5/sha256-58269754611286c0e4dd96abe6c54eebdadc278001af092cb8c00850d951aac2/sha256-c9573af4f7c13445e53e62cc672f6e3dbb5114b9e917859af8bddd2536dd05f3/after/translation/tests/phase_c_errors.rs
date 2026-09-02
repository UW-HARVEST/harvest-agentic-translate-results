//! Phase C — error-path differential tests that stay in-process.
//!
//! Covers every `ERRORS.md` row whose C behaviour is a *return*, i.e. rows
//! E1..E6 and E10..E12, plus the generic API boundaries that do not trip an
//! `assert`. The rows whose C behaviour is an `abort()` (A1..A9) are covered by
//! `phase_c_subproc.rs`, which compares process exit status instead.

mod common;

use common::deflate::*;
use common::rng::{Rng, SEED};
use common::*;

const E1_MSG: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const E2_MSG: &str = "Stored block extends beyond end of input stream.";
const E3_MSG: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const E4_MSG: &str = "Attempted to write before out buffer (invalid backwards distance).";
const E5_MSG: &str = "Attempted to overwrite out buffer while outputting a string.";
const E6_MSG: &str = "Detected unknown block type within input stream.";

fn lits(bytes: &[u8]) -> Vec<Op> {
    bytes.iter().map(|&b| Op::Lit(b)).collect()
}

/// Assert both implementations reject with the same code and the same reason.
fn expect_reject(p: &Pair, ctx: &str, stream: &[u8], align: usize, out_bytes: usize, msg: &str) {
    let c = run_inflate(&p.c, stream, align, out_bytes, None);
    let r = run_inflate(&p.rust, stream, align, out_bytes, None);
    assert_eq!(c.ret, 0, "[{ctx}] C unexpectedly accepted: {c:?}");
    assert_eq!(
        c.err.as_deref(),
        Some(msg),
        "[{ctx}] C reported a different reason: {c:?}"
    );
    assert_eq!(r.ret, c.ret, "[{ctx}] ret\n C:{c:?}\n R:{r:?}");
    assert_eq!(r.err, c.err, "[{ctx}] err\n C:{c:?}\n R:{r:?}");
    assert_eq!(r.out, c.out, "[{ctx}] out\n C:{c:?}\n R:{r:?}");
}

// ===========================================================================
// E1 — stored block whose LEN and NLEN are not complements
// ===========================================================================

#[test]
fn e1_stored_len_nlen_mismatch() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for len in [0u16, 1, 2, 3, 4, 8, 33, 250] {
        // Every distinct way of breaking the complement relation.
        let bad_nlens: Vec<u16> = vec![
            len,                     // NLEN == LEN
            0,                       // NLEN zero
            0xFFFF,                  // NLEN all ones
            !len ^ 1,                // off by the low bit
            !len ^ 0x8000,           // off by the high bit
            len.wrapping_add(1),     // arbitrary
            rng.next_u32() as u16,   // random
        ];
        for nlen in bad_nlens {
            if nlen == !len {
                continue; // that would be valid
            }
            let payload = rng.bytes(len as usize);
            let mut w = BitWriter::new();
            emit_stored_lens(&mut w, true, &payload, len, nlen);
            let stream = w.finish();
            for align in 0..4 {
                expect_reject(
                    &p,
                    &format!("E1 len={len} nlen={nlen:#06x} align={align}"),
                    &stream,
                    align,
                    (len as usize) + 32,
                    E1_MSG,
                );
            }
        }
    }
}

// ===========================================================================
// E2 — stored block whose LEN is smaller than the remaining input
// ===========================================================================

#[test]
fn e2_stored_extends_beyond_input() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE2);
    for len in [0usize, 1, 2, 3, 7, 16, 64] {
        for extra in 1..=9usize {
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            emit_stored_lens(&mut w, true, &payload, len as u16, !(len as u16));
            let mut stream = w.finish();
            for _ in 0..extra {
                stream.push(rng.byte());
            }
            for align in 0..4 {
                expect_reject(
                    &p,
                    &format!("E2 len={len} extra={extra} align={align}"),
                    &stream,
                    align,
                    len + 64,
                    E2_MSG,
                );
            }
        }
    }

    // Two stored blocks: the first one already trips E2 (its LEN is smaller
    // than everything that follows).
    for _ in 0..32 {
        let l1 = rng.below(32);
        let l2 = 1 + rng.below(32);
        let a = rng.bytes(l1);
        let b = rng.bytes(l2);
        let mut w = BitWriter::new();
        emit_stored(&mut w, false, &a);
        emit_stored(&mut w, true, &b);
        let stream = w.finish();
        expect_reject(
            &p,
            &format!("E2 two-blocks l1={l1} l2={l2}"),
            &stream,
            0,
            l1 + l2 + 64,
            E2_MSG,
        );
    }
}

// ===========================================================================
// E3 — literal does not fit in the out buffer
// ===========================================================================

#[test]
fn e3_literal_overruns_out() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE3);
    for n in [1usize, 2, 3, 8, 40, 200] {
        let data = rng.bytes(n);
        let ops = lits(&data);
        // fixed block
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &ops);
        let fixed = w.finish();
        // dynamic block
        let d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::all(), 257, 1);
        let mut w = BitWriter::new();
        emit_dynamic(&mut w, true, &d, &ops);
        let dyn_ = w.finish();

        for (tag, stream) in [("fixed", &fixed), ("dynamic", &dyn_)] {
            for out_bytes in 0..n {
                for align in 0..4 {
                    expect_reject(
                        &p,
                        &format!("E3 {tag} n={n} out_bytes={out_bytes} align={align}"),
                        stream,
                        align,
                        out_bytes,
                        E3_MSG,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E4 — backwards distance reaches before the start of the out buffer
// ===========================================================================

#[test]
fn e4_distance_before_out_begin() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE4);
    // A match as the very first symbol: out == begin, so any distance >= 1 is
    // already before the buffer.
    for dsym in 0u16..30 {
        let dextra = 0;
        let ops = vec![Op::Raw {
            lsym: 257,
            lextra: 0,
            dsym,
            dextra,
        }];
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &ops);
        let stream = w.finish();
        for align in 0..4 {
            expect_reject(
                &p,
                &format!("E4 first-symbol dsym={dsym} align={align}"),
                &stream,
                align,
                4096,
                E4_MSG,
            );
        }
    }

    // A match whose distance is one past the number of bytes emitted so far.
    for prefix in [1usize, 2, 3, 5, 17, 64, 200] {
        let data = rng.bytes(prefix);
        let mut ops = lits(&data);
        let dist = prefix as u32 + 1;
        let mut dsym = 29usize;
        while DIST_BASE[dsym] > dist {
            dsym -= 1;
        }
        let dextra = dist - DIST_BASE[dsym];
        if dextra >= (1 << DIST_EXTRA[dsym]) {
            continue;
        }
        ops.push(Op::Raw {
            lsym: 257,
            lextra: 0,
            dsym: dsym as u16,
            dextra,
        });
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &ops);
        let stream = w.finish();
        expect_reject(
            &p,
            &format!("E4 prefix={prefix} dist={dist}"),
            &stream,
            0,
            4096,
            E4_MSG,
        );
    }
}

// ===========================================================================
// E5 — match length overruns the out buffer
// ===========================================================================

#[test]
fn e5_string_overruns_out() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE5);
    for prefix in [1usize, 4, 16, 100] {
        for &len in &[3u32, 5, 20, 100, 258] {
            let data = rng.bytes(prefix);
            let mut ops = lits(&data);
            ops.push(Op::Match { len, dist: 1 });
            let total = prefix + len as usize;
            let mut w = BitWriter::new();
            emit_fixed(&mut w, true, &ops);
            let stream = w.finish();
            // out_bytes anywhere in [prefix, total-1] lets the literals through
            // but makes the copy overrun.
            for out_bytes in [prefix, prefix + 1, total - 1] {
                if out_bytes < prefix || out_bytes >= total {
                    continue;
                }
                for align in 0..4 {
                    expect_reject(
                        &p,
                        &format!("E5 prefix={prefix} len={len} out_bytes={out_bytes} align={align}"),
                        &stream,
                        align,
                        out_bytes,
                        E5_MSG,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E6 — unknown block type (btype == 3)
// ===========================================================================

#[test]
fn e6_unknown_block_type() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE6);
    for bfinal in [0u32, 1] {
        for nbytes in 1..=12usize {
            let mut w = BitWriter::new();
            w.bits(bfinal, 1);
            w.bits(3, 2);
            let mut stream = w.finish();
            while stream.len() < nbytes {
                stream.push(rng.byte());
            }
            for align in 0..4 {
                for out_bytes in [0usize, 1, 64] {
                    expect_reject(
                        &p,
                        &format!("E6 bfinal={bfinal} nbytes={nbytes} align={align} out={out_bytes}"),
                        &stream,
                        align,
                        out_bytes,
                        E6_MSG,
                    );
                }
            }
        }
    }

    // btype 3 reached as the *second* block, after a valid fixed block.
    for _ in 0..16 {
        let n = 1 + rng.below(40);
        let data = rng.bytes(n);
        let ops = lits(&data);
        let mut w = BitWriter::new();
        emit_fixed(&mut w, false, &ops);
        w.bits(1, 1);
        w.bits(3, 2);
        let mut stream = w.finish();
        stream.extend_from_slice(&[0, 0, 0, 0]);
        expect_reject(&p, "E6 second-block", &stream, 0, n + 64, E6_MSG);
    }
}

// ===========================================================================
// E10..E12 — convert_pix has no error channel
// ===========================================================================

#[test]
fn e10_convert_pix_out_of_range_bpp() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE10);
    // `switch (bpp)` in the C has no `default:`, so any value with no matching
    // case stores nothing. A C `int` parameter accepts every integer, which is
    // exactly the out-of-range-enum case an FFI caller can produce.
    let bpps: Vec<i32> = vec![
        0, 5, 6, 7, 8, 9, 16, 32, 64, 127, 128, 255, 256, 1000, 65536, -1, -2, -4, -255,
    ];
    for &bpp in &bpps {
        for &(w, h) in &[(1usize, 1usize), (3, 2), (8, 8)] {
            // The src buffer is large enough for the pointer walk in either
            // direction so nothing is read out of our own allocation.
            let span = (w * bpp.unsigned_abs() as usize + 1) * h + 64;
            let mut src = rng.bytes(2 * span);
            let mid = span;
            // Feed the middle of the buffer so negative bpp stays in-bounds.
            let sub = &mut src[mid - 32..];
            let sub = sub.to_vec();
            diff_convert_pix(
                &p,
                &format!("E10 bpp={bpp} w={w} h={h}"),
                bpp,
                w as i32,
                h as i32,
                &sub,
                w * h + 4,
            );
        }
    }

    // Extreme values, with w == h == 1 so only one pointer step happens.
    for &bpp in &[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        diff_convert_pix_null(&p, &format!("E10 extreme bpp={bpp}"), bpp, 1, 1);
    }
}

#[test]
fn e11_e12_convert_pix_empty_dims() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE11);
    let src = rng.bytes(1024);
    for bpp in [-1i32, 0, 1, 2, 3, 4, 5, 255] {
        for h in [i32::MIN, -1000, -1, 0] {
            // E11: h <= 0 => not a single iteration, dst untouched, NULL is fine.
            diff_convert_pix(&p, &format!("E11 bpp={bpp} h={h}"), bpp, 8, h, &src, 16);
            diff_convert_pix_null(&p, &format!("E11 null bpp={bpp} h={h}"), bpp, 8, h);
        }
        for w in [i32::MIN, -1000, -1, 0] {
            // E12: w <= 0 with h > 0 => only `src++` per row, dst untouched.
            for h in [1i32, 2, 7] {
                diff_convert_pix(
                    &p,
                    &format!("E12 bpp={bpp} w={w} h={h}"),
                    bpp,
                    w,
                    h,
                    &src,
                    16,
                );
            }
        }
    }
}

// ===========================================================================
// Generic API boundaries that do not abort
// ===========================================================================

#[test]
fn generic_zero_and_negative_out_bytes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0x0B0);
    // out_bytes == 0 with a literal => E3. Covered above; here also check that
    // an *empty* fixed block (just end-of-block) succeeds with out_bytes == 0.
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &[]);
    let stream = w.finish();
    for align in 0..4 {
        let c = run_inflate(&p.c, &stream, align, 0, None);
        let r = run_inflate(&p.rust, &stream, align, 0, None);
        assert_eq!(c.ret, r.ret, "empty block out=0\n C:{c:?}\n R:{r:?}");
        assert_eq!(c.err, r.err, "empty block out=0\n C:{c:?}\n R:{r:?}");
        assert_eq!(c.out, r.out);
    }

    // A stored block of LEN 0 with out_bytes 0.
    let mut w = BitWriter::new();
    emit_stored(&mut w, true, &[]);
    let stream = w.finish();
    for align in 0..4 {
        let c = run_inflate(&p.c, &stream, align, 0, None);
        let r = run_inflate(&p.rust, &stream, align, 0, None);
        assert_eq!(c.ret, r.ret, "stored LEN=0 out=0\n C:{c:?}\n R:{r:?}");
        assert_eq!(c.err, r.err);
        assert_eq!(c.out, r.out);
    }

    let _ = &mut rng;
}

#[test]
fn generic_dist_symbols_30_and_31() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0x1E);
    // The fixed distance code has 32 symbols but `cp_dist_base` only defines 30;
    // symbols 30 and 31 have base 0 and 0 extra bits, so the decoder computes
    // `backwards_distance == 0` and copies `out` onto itself.
    for dsym in [30u16, 31] {
        for prefix in [0usize, 1, 8, 64] {
            let data = rng.bytes(prefix);
            let mut ops = lits(&data);
            ops.push(Op::Raw {
                lsym: 257,
                lextra: 0,
                dsym,
                dextra: 0,
            });
            let mut w = BitWriter::new();
            emit_fixed(&mut w, true, &ops);
            let stream = w.finish();
            for align in 0..4 {
                let c = run_inflate(&p.c, &stream, align, prefix + 64, None);
                let r = run_inflate(&p.rust, &stream, align, prefix + 64, None);
                assert_eq!(
                    c.ret, r.ret,
                    "dsym={dsym} prefix={prefix} align={align}\n C:{c:?}\n R:{r:?}"
                );
                assert_eq!(c.err, r.err, "dsym={dsym}\n C:{c:?}\n R:{r:?}");
                assert_eq!(c.out, r.out, "dsym={dsym}\n C:{c:?}\n R:{r:?}");
            }
        }
    }
}

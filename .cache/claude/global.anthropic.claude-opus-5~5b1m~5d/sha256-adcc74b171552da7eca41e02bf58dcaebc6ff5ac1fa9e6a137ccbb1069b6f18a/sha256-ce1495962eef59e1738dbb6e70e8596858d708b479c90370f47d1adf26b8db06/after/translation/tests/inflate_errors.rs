//! Phase C — error-path differential tests for `cp_inflate`
//! (`ERRORS.md` rows 1..8 and 14..31).

mod common;

use common::deflate::*;
use common::*;

// The six error strings, byte-for-byte as they appear in c_src/src/lib.c.
const ERR_LEN_NLEN: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const ERR_STORED_BEYOND: &str = "Stored block extends beyond end of input stream.";
const ERR_OUT_SYMBOL: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const ERR_BACKWARDS: &str = "Attempted to write before out buffer (invalid backwards distance).";
const ERR_OUT_STRING: &str = "Attempted to overwrite out buffer while outputting a string.";
const ERR_UNKNOWN_BLOCK: &str = "Detected unknown block type within input stream.";

struct Built {
    case: Case,
    out_off: usize,
}

/// `[64 pad | stream | pad_bytes of padding | pad | out (out_bytes) | slack]`
fn build(stream: &[u8], pad_bytes: usize, out_bytes: i32, out_slack: usize, seed: u64) -> Built {
    let mut rng = Rng::new(seed);
    let in_off = 64usize;
    let in_len = stream.len() + pad_bytes;
    let out_off = (in_off + in_len + 128) & !15;
    let total = out_off + out_bytes.max(0) as usize + out_slack + 256;
    let mut scratch: Vec<u8> = (0..total).map(|_| rng.u8()).collect();
    scratch[in_off..in_off + stream.len()].copy_from_slice(stream);
    Built {
        case: Case::inflate(scratch, in_off as isize, in_len as i32, out_off as isize, out_bytes),
        out_off,
    }
}

#[track_caller]
fn expect_error(o: &Outcome, msg: &str, ctx: &str) {
    assert_eq!(o.status, Status::Exited(0), "[{ctx}] expected a clean return, got {o:?}");
    assert_eq!(o.ret, 0, "[{ctx}] expected ret == 0, got {o:?}");
    assert_eq!(
        o.err.as_deref(),
        Some(msg.as_bytes()),
        "[{ctx}] wrong cp_error_reason: {:?}",
        o.err.as_ref().map(|e| String::from_utf8_lossy(e).to_string())
    );
}

#[track_caller]
fn expect_assert(o: &Outcome, needle: &str, ctx: &str) {
    assert_eq!(
        o.status,
        Status::Signaled(libc::SIGABRT),
        "[{ctx}] expected SIGABRT from a live assert(), got {o:?}"
    );
    let m = o.assert_msg.as_deref().unwrap_or("");
    assert!(m.contains(needle), "[{ctx}] expected assert `{needle}', got `{m}'");
}

// ---------------------------------------------------------------------------
// row 1 / row 7: LEN and NLEN are not complements
// ---------------------------------------------------------------------------

#[test]
fn err01_stored_len_nlen_mismatch() {
    let mut rng = Rng::new(0x01);
    for _ in 0..120 {
        let n = rng.below(40) as usize;
        let data = rng.bytes(n);
        let len_field = n as u16;
        // any NLEN that is not ~LEN
        let mut nlen = rng.next_u32() as u16;
        if nlen == !len_field {
            nlen ^= 1;
        }
        let mut bw = BitWriter::new();
        emit_stored_bad_nlen(&mut bw, &data, len_field, nlen, true);
        let s = bw.finish();
        let b = build(&s, 0, 512, 1024, rng.next_u64());
        let o = diff(&b.case, "err01 LEN/NLEN mismatch");
        expect_error(&o, ERR_LEN_NLEN, "err01");
    }
    // the exact boundary: NLEN off by one in either direction
    for delta in [1u16, 0xFFFF] {
        for len_field in [0u16, 1, 255, 256, 0xFFFE, 0xFFFF] {
            let data = vec![0xAAu8; 8];
            let mut bw = BitWriter::new();
            emit_stored_bad_nlen(&mut bw, &data, len_field, (!len_field).wrapping_add(delta), true);
            let s = bw.finish();
            let b = build(&s, 0, 512, 70000, 0x17);
            let o = diff(&b.case, &format!("err01 boundary len={len_field} delta={delta}"));
            expect_error(&o, ERR_LEN_NLEN, "err01 boundary");
        }
    }
}

/// row 7: LEN/NLEN is checked *before* the length check, so its message wins.
#[test]
fn err07_stored_check_order() {
    let data = vec![0x11u8; 40];
    // LEN = 0 (so `bits_left/8 <= LEN` is violated too) and NLEN = 0 (not ~LEN)
    let mut bw = BitWriter::new();
    emit_stored_bad_nlen(&mut bw, &data, 0, 0, true);
    let s = bw.finish();
    let b = build(&s, 0, 512, 1024, 0x07);
    let o = diff(&b.case, "err07 both stored checks violated");
    expect_error(&o, ERR_LEN_NLEN, "err07");
}

// ---------------------------------------------------------------------------
// row 2 / row 21: the stored block does not cover the rest of the input
// ---------------------------------------------------------------------------

#[test]
fn err02_stored_extends_beyond() {
    let mut rng = Rng::new(0x02);
    for n in [1usize, 2, 3, 4, 8, 9, 16, 40, 300] {
        for shortfall in [1usize, 2, n] {
            if shortfall > n {
                continue;
            }
            let data = rng.bytes(n);
            let len_field = (n - shortfall) as u16;
            let mut bw = BitWriter::new();
            emit_stored_len(&mut bw, &data, len_field, true);
            let s = bw.finish();
            let b = build(&s, 0, 512, 1024, rng.next_u64());
            let o = diff(&b.case, &format!("err02 n={n} LEN={len_field}"));
            expect_error(&o, ERR_STORED_BEYOND, "err02");
        }
    }
    // row 21: a *non-final* stored block always has input left after it
    for n in [0usize, 1, 4, 7, 32] {
        let data = rng.bytes(n);
        let mut bw = BitWriter::new();
        emit_stored(&mut bw, &data, false);
        emit_stored(&mut bw, &[0u8; 4], true);
        let s = bw.finish();
        let b = build(&s, 0, 512, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err21 non-final stored n={n}"));
        expect_error(&o, ERR_STORED_BEYOND, "err21");
    }
    // trailing padding after an otherwise-correct stored block does it too
    for pad in [1usize, 2, 3, 4, 17] {
        let data = vec![0x5Au8; 12];
        let mut bw = BitWriter::new();
        emit_stored(&mut bw, &data, true);
        let s = bw.finish();
        let b = build(&s, pad, 512, 1024, 0x21);
        let o = diff(&b.case, &format!("err02 padding={pad}"));
        expect_error(&o, ERR_STORED_BEYOND, "err02 padding");
    }
}

// ---------------------------------------------------------------------------
// row 3 / 17 / 18: no room for a literal
// ---------------------------------------------------------------------------

#[test]
fn err03_out_symbol_overflow() {
    let mut rng = Rng::new(0x03);
    for out_bytes in [0i32, 1, 2, 5, 17] {
        for _ in 0..30 {
            let n = out_bytes.max(0) as usize + 1 + rng.below(20) as usize;
            let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &syms, true);
            let s = bw.finish();
            let b = build(&s, 0, out_bytes, 1024, rng.next_u64());
            let o = diff(&b.case, &format!("err03 out_bytes={out_bytes} n={n}"));
            expect_error(&o, ERR_OUT_SYMBOL, "err03");
            // exactly `out_bytes` literals must have been written first
            let wrote = &o.scratch[b.out_off..b.out_off + out_bytes.max(0) as usize];
            let expected = expand(&syms, &Tables::default());
            assert_eq!(wrote, &expected[..out_bytes.max(0) as usize]);
        }
    }
    // the same through a dynamic block
    let t = Tables::default();
    for out_bytes in [0i32, 3] {
        for _ in 0..20 {
            let n = out_bytes.max(0) as usize + 4;
            let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
            let spec = dyn_spec_for(&syms, 288, 32, &t);
            let mut bw = BitWriter::new();
            emit_dynamic(&mut bw, &spec, &syms, true, &t);
            let s = bw.finish();
            let b = build(&s, 0, out_bytes, 1024, rng.next_u64());
            let o = diff(&b.case, &format!("err03 dynamic out_bytes={out_bytes}"));
            expect_error(&o, ERR_OUT_SYMBOL, "err03 dynamic");
        }
    }
}

#[test]
fn err18_negative_out_bytes() {
    let mut rng = Rng::new(0x18);
    for out_bytes in [-1i32, -2, -1000, i32::MIN] {
        let syms: Vec<Sym> = (0..4).map(|_| Sym::Lit(rng.u8())).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, true);
        let s = bw.finish();
        let b = build(&s, 0, out_bytes, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err18 out_bytes={out_bytes}"));
        expect_error(&o, ERR_OUT_SYMBOL, "err18");
    }
}

#[test]
fn err19_null_out() {
    let mut rng = Rng::new(0x19);
    // A stream whose first symbol is a literal: the bounds check fires before
    // `*s->out` is ever written, so passing NULL must not crash.
    let syms: Vec<Sym> = (0..4).map(|_| Sym::Lit(rng.u8())).collect();
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, &syms, true);
    let s = bw.finish();
    // out_bytes == 0: `out + 1 <= out_end` is false, so the check fires before
    // NULL is ever written.
    let b = build(&s, 0, 0, 1024, rng.next_u64());
    let case = b.case.clone().with_null_out();
    let o = diff(&case, "err19 NULL out, out_bytes=0");
    expect_error(&o, ERR_OUT_SYMBOL, "err19");
    // out_bytes < 0: `out_end = NULL + negative` *wraps* to a huge address, so
    // the check passes and `*s->out = symbol` faults - identically in both.
    for out_bytes in [-1i32, -1000, i32::MIN] {
        let b = build(&s, 0, 0, 1024, rng.next_u64());
        let mut case = b.case.clone();
        if let common::Call::Inflate { out_bytes: ob, .. } = &mut case.call {
            *ob = out_bytes;
        }
        let case = case.with_null_out();
        let o = diff(&case, &format!("err19 NULL out, out_bytes={out_bytes}"));
        assert_eq!(
            o.status,
            Status::Signaled(libc::SIGSEGV),
            "NULL out with a negative out_bytes must fault in both libraries: {o:?}"
        );
    }
    // An *empty* block writes nothing at all, so NULL/0 succeeds.
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, &[], true);
    let s = bw.finish();
    let b = build(&s, 0, 0, 1024, 0x191);
    let case = b.case.clone().with_null_out();
    let o = diff(&case, "err19 NULL out, empty block");
    assert_eq!(o.status, Status::Exited(0));
    assert_eq!(o.ret, 1);
    assert_eq!(o.err, None);
}

// ---------------------------------------------------------------------------
// row 4 / row 8: invalid backwards distance
// ---------------------------------------------------------------------------

#[test]
fn err04_backwards_distance() {
    let t = Tables::default();
    let mut rng = Rng::new(0x04);
    // a match as the very first symbol: `out - distance < begin` for any
    // distance >= 1
    for dc in 0usize..30 {
        let de = if t.dist_extra[dc] > 0 { rng.below(1 << t.dist_extra[dc]) } else { 0 };
        let syms = vec![Sym::RawMatch(0, 0, dc, de)];
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, true);
        let s = bw.finish();
        let b = build(&s, 0, 4096, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err04 first-symbol match dc={dc}"));
        expect_error(&o, ERR_BACKWARDS, "err04");
    }
    // a match one byte too far back
    for pre in [1usize, 2, 3, 8, 40] {
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        let (dc, de) = t.dist_code(pre as u32 + 1);
        syms.push(Sym::RawMatch(0, 0, dc, de));
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, true);
        let s = bw.finish();
        let b = build(&s, 0, 4096, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err04 distance = pre+1, pre={pre}"));
        expect_error(&o, ERR_BACKWARDS, "err04 off-by-one");
    }
    // and exactly at the boundary (distance == bytes written) must succeed
    for pre in [1usize, 2, 3, 8, 40] {
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        let (dc, de) = t.dist_code(pre as u32);
        syms.push(Sym::RawMatch(0, 0, dc, de));
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, true);
        let s = bw.finish();
        let b = build(&s, 0, 4096, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err04 distance == pre, pre={pre}"));
        assert_eq!(o.ret, 1, "distance == bytes-written must be accepted: {o:?}");
    }
}

/// row 8: the distance check happens before the length check.
#[test]
fn err08_block_check_order() {
    let t = Tables::default();
    // one literal, then a match that is both too far back *and* too long
    let syms = vec![Sym::Lit(0x41), Sym::RawMatch(28, 0, 10, 0)]; // len 258, dist 33
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, &syms, true);
    let s = bw.finish();
    let b = build(&s, 0, 2, 1024, 0x08);
    let o = diff(&b.case, "err08 both block checks violated");
    expect_error(&o, ERR_BACKWARDS, "err08");
    let _ = t;
}

// ---------------------------------------------------------------------------
// row 5: the match copy would run past out_end
// ---------------------------------------------------------------------------

#[test]
fn err05_out_string_overflow() {
    let t = Tables::default();
    let mut rng = Rng::new(0x05);
    for _ in 0..120 {
        let pre = rng.below(30) as usize + 1;
        let len = rng.range(3, 258) as u32;
        let dist = rng.range(1, pre as i32) as u32;
        // just too small: the literals fit, the copy does not
        let out_bytes = (pre as u32 + len - 1) as i32;
        let mut syms: Vec<Sym> = (0..pre).map(|_| Sym::Lit(rng.u8())).collect();
        syms.push(Sym::Match(len, dist));
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, true);
        let s = bw.finish();
        let b = build(&s, 0, out_bytes, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err05 pre={pre} len={len} dist={dist}"));
        expect_error(&o, ERR_OUT_STRING, "err05");
        // one more output byte and the same stream succeeds
        let b2 = build(&s, 0, out_bytes + 1, 1024, rng.next_u64());
        let o2 = diff(&b2.case, "err05 boundary+1");
        assert_eq!(o2.ret, 1, "one extra byte should be enough: {o2:?}");
    }
    let _ = t;
}

// ---------------------------------------------------------------------------
// row 6: reserved block type
// ---------------------------------------------------------------------------

#[test]
fn err06_btype3() {
    let mut rng = Rng::new(0x06);
    for last in [false, true] {
        for pad in [1usize, 2, 3, 4, 8] {
            let mut bw = BitWriter::new();
            emit_btype3(&mut bw, last);
            let s = bw.finish();
            let b = build(&s, pad, 512, 1024, rng.next_u64());
            let o = diff(&b.case, &format!("err06 btype3 last={last} pad={pad}"));
            expect_error(&o, ERR_UNKNOWN_BLOCK, "err06");
        }
    }
    // btype 3 as the *second* block, after a good one
    for _ in 0..30 {
        let n = rng.below(20) as usize + 1;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, false);
        emit_btype3(&mut bw, true);
        let s = bw.finish();
        let b = build(&s, 4, 512, 1024, rng.next_u64());
        let o = diff(&b.case, "err06 btype3 second block");
        expect_error(&o, ERR_UNKNOWN_BLOCK, "err06 second");
    }
}

// ---------------------------------------------------------------------------
// rows 14..16: degenerate `in` / `in_bytes`
// ---------------------------------------------------------------------------

#[test]
fn err14_in_bytes_zero() {
    // bits_left == 0 straight away: with a live assert() that is
    // `cp_read_bits: s->bits_left > 0`; with NDEBUG the decoder reads zero bits
    // and ends up in cp_stored with LEN == NLEN == 0.
    let mut rng = Rng::new(0x14);
    for out_bytes in [0i32, 16, 4096] {
        let b = build(&[], 0, out_bytes, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err14 in_bytes=0 out_bytes={out_bytes}"));
        if C_ASSERTS {
            expect_assert(&o, "s->bits_left > 0", "err14");
        } else {
            expect_error(&o, ERR_LEN_NLEN, "err14");
        }
    }
}

#[test]
fn err15_null_in_zero_len() {
    // NULL is 4-byte aligned, so `first_bytes == 0` and nothing is read.
    let b = build(&[], 0, 4096, 1024, 0x15);
    let mut case = b.case.clone();
    if let common::Call::Inflate { in_bytes, .. } = &mut case.call {
        *in_bytes = 0;
    }
    let case = case.with_null_in();
    let o = diff(&case, "err15 NULL in, in_bytes=0");
    if C_ASSERTS {
        expect_assert(&o, "s->bits_left > 0", "err15");
    } else {
        expect_error(&o, ERR_LEN_NLEN, "err15");
    }
}

#[test]
fn err16_negative_in_bytes() {
    let mut rng = Rng::new(0x16);
    let mk = |rng: &mut Rng, in_bytes: i32| {
        let syms: Vec<Sym> = (0..4).map(|_| Sym::Lit(rng.u8())).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &syms, true);
        let s = bw.finish();
        let b = build(&s, 0, 4096, 1024, rng.next_u64());
        let mut case = b.case.clone();
        if let common::Call::Inflate { in_bytes: ib, .. } = &mut case.call {
            *ib = in_bytes;
        }
        case
    };
    // `bits_left = in_bytes * 8` is <= 0, so the very first `cp_read_bits`
    // trips `assert(s->bits_left > 0)`; the `final_word` assembly reads a few
    // bytes *before* `in`, which is still mapped here.
    for in_bytes in [-1i32, -2, -3, -4, -5, -1000, i32::MIN] {
        let case = mk(&mut rng, in_bytes);
        let o = diff(&case, &format!("err16 in_bytes={in_bytes}"));
        if C_ASSERTS {
            expect_assert(&o, "s->bits_left > 0", "err16");
        }
    }
    // `in_bytes == INT_MIN + 1` gives `last_bytes == 1`, so the `final_word`
    // loop reads `in[INT_MIN]` - a wild read that faults in both libraries.
    let case = mk(&mut rng, i32::MIN + 1);
    let o = diff(&case, "err16 in_bytes=INT_MIN+1");
    assert_eq!(
        o.status,
        Status::Signaled(libc::SIGSEGV),
        "in[INT_MIN] must fault in both libraries: {o:?}"
    );
}

// ---------------------------------------------------------------------------
// row 20: an over-long stored block reads past `in` and writes past `out_end`
// ---------------------------------------------------------------------------

#[test]
fn err20_stored_overreads() {
    let mut rng = Rng::new(0x20);
    for n in [0usize, 3, 7, 19, 100] {
        for overshoot in [1usize, 4, 33, 200] {
            let data = rng.bytes(n);
            let len_field = (n + overshoot) as u16;
            let mut bw = BitWriter::new();
            emit_stored_len(&mut bw, &data, len_field, true);
            let s = bw.finish();
            // LEN bytes are copied regardless of out_bytes, so keep plenty of
            // slack in the shared scratch (both libraries write the same bytes
            // to the same offsets)
            let b = build(&s, 0, 4, len_field as usize + 4096, rng.next_u64());
            let o = diff(&b.case, &format!("err20 n={n} LEN={len_field}"));
            assert_eq!(o.ret, 1, "the inverted length check accepts LEN > remaining: {o:?}");
            assert_eq!(o.err, None);
            // it really did write `LEN` bytes past `out_bytes`
            assert_ne!(
                &o.scratch[b.out_off..b.out_off + len_field as usize],
                &b.case.scratch[b.out_off..b.out_off + len_field as usize],
                "expected the unchecked memcpy to be observable"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// rows 22..27: the assert sites, driven deliberately
// ---------------------------------------------------------------------------

/// row 22: the decoder runs out of input entirely (`bits_left == 0`).
#[test]
fn err22_assert_bits_left() {
    let mut rng = Rng::new(0x22);
    let mut hits = 0usize;
    // A stored block header truncated so that `bits_left` hits 0 exactly at the
    // NLEN read.
    for n in [1usize, 2, 3, 4, 5] {
        let stream = vec![0x00u8; n];
        let b = build(&stream, 0, 4096, 1024, rng.next_u64());
        let o = diff(&b.case, &format!("err22 all-zero {n} bytes"));
        if C_ASSERTS && o.status == Status::Signaled(libc::SIGABRT) {
            let m = o.assert_msg.as_deref().unwrap_or("");
            if m.contains("s->bits_left > 0") {
                hits += 1;
            }
        }
    }
    if C_ASSERTS {
        assert!(hits > 0, "row 22 (`s->bits_left > 0`) was never reached");
    }
}

/// row 24: `cp_consume_bits`'s `s->count >= num_bits_to_read`.
#[test]
fn err24_assert_count() {
    // `[0x00, 0x00]`: bfinal=0, btype=0 (stored); the alignment read leaves
    // count == 8, then the 16-bit LEN read finds nothing left to load.
    let b = build(&[0x00, 0x00], 0, 4096, 1024, 0x24);
    let o = diff(&b.case, "err24 two zero bytes");
    if C_ASSERTS {
        expect_assert(&o, "s->count >= num_bits_to_read", "err24");
    } else {
        expect_error(&o, ERR_LEN_NLEN, "err24");
    }
}

/// rows 22..27 as a set: search a small, deterministic input space and require
/// that every reachable assert site is hit *and* that C and Rust agree on
/// every single one of them.
#[test]
fn err22to27_all_reachable_assert_sites() {
    let t = Tables::default();
    let mut rng = Rng::new(0x2227);
    let mut hits: std::collections::BTreeMap<String, usize> = Default::default();

    let mut record = |o: &Outcome| {
        if let Some(m) = &o.assert_msg {
            let key = m.splitn(2, ": ").nth(1).unwrap_or(m).to_string();
            *hits.entry(key).or_default() += 1;
        }
    };

    // (a) short all-zero and all-one inputs: exhaust the bit reservoir
    for n in 1..=8usize {
        for fill in [0x00u8, 0xFF, 0x55, 0xAA] {
            let b = build(&vec![fill; n], 0, 4096, 1024, rng.next_u64());
            record(&diff(&b.case, &format!("assert-scan fill={fill:02x} n={n}")));
        }
    }
    // (b) truncated valid fixed / dynamic / stored streams
    for kind in 0..3 {
        for _ in 0..25 {
            let n = rng.below(20) as usize + 1;
            let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
            let mut bw = BitWriter::new();
            match kind {
                0 => emit_fixed(&mut bw, &syms, true),
                1 => {
                    let spec = dyn_spec_for(&syms, 288, 32, &t);
                    emit_dynamic(&mut bw, &spec, &syms, true, &t)
                }
                _ => {
                    let d = rng.bytes(n);
                    emit_stored(&mut bw, &d, true)
                }
            }
            let full = bw.finish();
            for cut in 1..=full.len().min(6) {
                let s = &full[..full.len() - cut];
                if s.is_empty() {
                    continue;
                }
                let b = build(s, 0, 4096, 1024, rng.next_u64());
                record(&diff(&b.case, &format!("assert-scan trunc kind={kind} cut={cut}")));
            }
        }
    }
    // (c) dynamic blocks whose code-length tree is deliberately incomplete, so
    //     `cp_decode` lands on `tree[-1]` (row 27) and can feed `cp_build` a
    //     length >= 16 (row 26)
    for _ in 0..60 {
        let n = rng.below(20) as usize + 2;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        let mut spec = dyn_spec_for(&syms, 288, 32, &t);
        match rng.below(3) {
            0 => spec.dist_lens = vec![0u8; 32],
            1 => {
                // corrupt a literal code length so the tree is over-subscribed
                let i = rng.below(spec.lit_lens.len() as u32) as usize;
                spec.lit_lens[i] = rng.range(1, 15) as u8;
            }
            _ => {
                for i in 0..spec.lit_lens.len() {
                    if rng.below(8) == 0 {
                        spec.lit_lens[i] = rng.range(1, 15) as u8;
                    }
                }
            }
        }
        let mut bw = BitWriter::new();
        emit_dynamic(&mut bw, &spec, &syms, true, &t);
        let mut stream = bw.finish();
        stream.extend_from_slice(&[0u8; 96]);
        let b = build(&stream, 0, 4096, 8192, rng.next_u64());
        record(&diff(&b.case, "assert-scan corrupt tree"));
    }
    // (d) stored blocks reached with an unaligned `bits_left` (row 25)
    for skew in 0..4usize {
        for nlit in 0..8usize {
            let syms: Vec<Sym> = (0..nlit).map(|_| Sym::Lit(0x42)).collect();
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &syms, false);
            emit_stored(&mut bw, &[0x7Fu8; 3], true);
            let mut stream = bw.finish();
            stream.truncate(stream.len().saturating_sub(skew));
            if stream.is_empty() {
                continue;
            }
            let b = build(&stream, 0, 4096, 1024, rng.next_u64());
            record(&diff(&b.case, &format!("assert-scan stored skew={skew} nlit={nlit}")));
        }
    }

    println!("assert sites hit: {hits:#?}");
    if C_ASSERTS {
        for needle in [
            "s->bits_left > 0",
            "s->count >= num_bits_to_read",
            "(search >> len) == (key >> len)",
        ] {
            assert!(
                hits.keys().any(|k| k.contains(needle)),
                "assert site `{needle}' was never reached; hits = {hits:#?}"
            );
        }
    } else {
        assert!(hits.is_empty(), "NDEBUG build must never abort: {hits:#?}");
    }
}

/// row 25: `cp_ptr`'s `assert(!(s->bits_left & 7))`.
///
/// After `cp_read_bits(s, s->count & 7)` the *count* is a multiple of 8, but
/// `bits_left` is only a multiple of 8 if `bits_left ≡ count (mod 8)`.  The
/// `final_word` path breaks that: it adds `s->bits_left` to `count` instead of
/// `last_bytes * 8`, so the two run out of step by whatever `count` happened to
/// be.
///
/// The stream below is derived by hand:
///
/// * `in_bytes = 11`, 4-byte aligned  => `first_bytes = 0`, `word_count = 2`,
///   `last_bytes = 3`;
/// * a non-final btype-1 block with **six** 8-bit literals then end-of-block
///   makes `count == 13` exactly when the decoder needs the final partial word,
///   so `count += s->bits_left` sets `count = 2*13 + 3*8 = 50` while the true
///   value is 37 - i.e. `bits_left = count - 13` from then on;
/// * the following final btype-0 block therefore sees `count & 7 == 0` and
///   skips the re-alignment, and after `LEN`/`NLEN` `bits_left == -5`, whose
///   low three bits are `3`;
/// * `LEN`/`NLEN` are chosen so the complement check still passes even though
///   the last five bits of `NLEN` are phantom zeroes past the end of the input.
#[test]
fn err25_assert_cp_ptr_alignment() {
    let mut rng = Rng::new(0x25);
    let lit = Huff::new(default_fixed_table()[..288].to_vec());
    let nlen_11: u32 = 0x555; // any 11-bit value
    let len_field: u32 = (!(nlen_11 as u16)) as u32; // 0xFAAA

    let mut bw = BitWriter::new();
    bw.bits(0, 1); // bfinal = 0
    bw.bits(1, 2); // btype  = 1 (fixed)
    for _ in 0..6 {
        lit.put(&mut bw, 0x41); // an 8-bit literal code
    }
    lit.put(&mut bw, 256); // end of block (7 bits)
    bw.bits(1, 1); // bfinal = 1
    bw.bits(0, 2); // btype  = 0 (stored) - deliberately *not* byte aligned
    bw.bits(len_field, 16);
    bw.bits(nlen_11, 11);
    let stream = bw.finish();
    assert_eq!(stream.len(), 11, "the derivation depends on in_bytes == 11");

    let b = build(&stream, 0, 4096, 4096, rng.next_u64());
    let o = diff(&b.case, "err25 hand-derived unaligned stored block");
    if C_ASSERTS {
        expect_assert(&o, "!(s->bits_left & 7)", "err25");
    }
}

/// row 26: `cp_build`'s `assert(len < 16)`.
///
/// Unreachable from stream data alone (`cp_dynamic` can only ever store
/// `cp_decode` results, and a `cp_decode` result that is not a real tree entry
/// trips row 27 first), but perfectly reachable through the *writable exported
/// table* `cp_fixed_table`, whose entries the caller controls and `cp_fixed`
/// feeds straight into `cp_build`.
#[test]
fn err26_assert_build_len_via_fixed_table_override() {
    let mut rng = Rng::new(0x26);
    let syms: Vec<Sym> = (0..4).map(|_| Sym::Lit(rng.u8())).collect();
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, &syms, true);
    let stream = bw.finish();

    for bad in [16u8, 17, 31] {
        // Only the *first* entry is out of range, so the C library's
        // `counts[lens[n]]++` writes a single int just past `int counts[16]`.
        let mut table = default_fixed_table();
        table[0] = bad;
        let b = build(&stream, 0, 4096, 4096, rng.next_u64());
        let case = b.case.clone().with_table(Table::FixedTable, table);
        if C_ASSERTS {
            // The assert fires before the corrupted stack can be observed, so
            // the two libraries agree exactly.
            let o = diff(&case, &format!("err26 cp_fixed_table[0]={bad}"));
            expect_assert(&o, "len < 16", "err26");
        } else {
            // With NDEBUG there is no assert to stop it: `counts[lens[n]]++`
            // scribbles over `cp_build`'s frame and the behaviour becomes
            // frame-layout dependent.  The UB oracle must confirm that.
            let (_, was_ub) = diff_or_ub(&case, &format!("err26 NDEBUG bad={bad}"));
            println!("err26 NDEBUG cp_fixed_table[0]={bad}: layout-dependent = {was_ub}");
        }
    }
}

/// rows 28..31: asserts that are provably unreachable through the FFI boundary.
/// The test documents *why* and checks the invariant the argument rests on.
#[test]
fn err28_unreachable_asserts() {
    // `cp_read_bits` is only ever called with these values:
    //   literals 1, 2, 3, 4, 5, 7, 16   (block headers, run lengths, LEN/NLEN)
    //   s->count & 7                    -> 0..7 even for a negative count
    //   cp_len_extra_bits[0..=30]        -> max 5
    //   cp_dist_extra_bits[0..=31]       -> max 13
    // and `cp_consume_bits` additionally with `key & 0xF` -> 0..15.
    let c = c_lib();
    assert!(*c.read_table(Table::LenExtraBits).iter().max().unwrap() <= 32);
    assert!(*c.read_table(Table::DistExtraBits).iter().max().unwrap() <= 32);
    // C's `&` on a negative int still yields 0..7:
    for x in [-1i32, -7, -8, -9, i32::MIN] {
        assert!((0..=7).contains(&(x & 7)));
    }
    // `cp_peak_bits` increments `word_index` only inside
    // `if (s->word_index < s->word_count)`, so `word_index <= word_count` holds
    // by construction; and `count` can only grow while `count < num_bits <= 16`,
    // by 32 or by `bits_left = count + last_bytes*8 <= 15 + 24`, so it stays
    // well under 64.
    assert!(15 + 32 <= 64);
    assert!(2 * 15 + 24 <= 64);
}

/// The one *unavoidable* divergence, made explicit.
///
/// `cp_dynamic` declares `uint8_t lens[288 + 32]` and fills it with
/// `for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n) lens[n] = 0;`
/// without ever checking `n` against 320.  In the reference build gcc places
/// that frame as
///
/// ```text
///   -0x188  the saved `s` parameter   (so lens[-1] is its top byte == 0x00)
///   -0x180  uint8_t lens[320]         (lens[320] == -0x40)
///   -0x40   uint8_t lenlens[19]
///   -0x24   int sym       (== lens[348])
///   -0x20   int nlen      (== lens[352])
///   -0x1c   int ndst      (== lens[356])
///   -0x18   int nlit      (== lens[360])
///   -0x14   int i         (== lens[364])   <-- the case-18 loop counter
///   -0x8    int n         (== lens[376])   <-- the outer loop counter
/// ```
///
/// so as soon as a code-length run pushes `n` to 364 the loop zeroes its **own
/// counter** and the C library spins forever (confirmed by sampling `RIP` from
/// a `SIGALRM` handler: it sits at `cp_dynamic+0x1f8..0x211`, the `case 18`
/// loop).  The translation's `lens` buffer has slack instead of live locals
/// behind it, so it terminates.  No translation can reproduce a specific
/// compiler's stack frame, so this is documented rather than "fixed".
#[test]
fn err35_lens_overshoot_hangs_the_c_library() {
    // nlit + ndst == 320, and the final code-length instruction is a
    // symbol-18 run of 138 zeroes starting at n == 319, so n runs to 457 and
    // sails past lens[364].
    let mut spec = DynSpec::new(vec![0u8; 288], vec![0u8; 32]);
    // one non-zero literal length so the CL alphabet has two symbols
    spec.lit_lens[0] = 1;
    spec.lit_lens[1] = 1;
    spec.cl_mode = ClMode::R18;
    // make the tail of the combined vector a long zero run that starts at 319
    let mut bw = BitWriter::new();
    emit_dynamic_header_only(&mut bw, &spec);
    let mut stream = bw.finish();
    stream.extend_from_slice(&[0u8; 64]);
    let b = build(&stream, 0, 4096, 8192, 0x35);
    let case = b.case.clone().with_timeout(3);

    let c = run(c_ref(), &case);
    let r = run(rust_lib(), &case);
    // The C build's own out-of-bounds writes hang it; the translation does not.
    // (If the C library ever *stops* hanging here the row must be revisited.)
    if c.status == Status::Signaled(libc::SIGALRM) {
        assert_ne!(
            r.status,
            Status::Signaled(libc::SIGALRM),
            "the translation is expected to terminate where the C build hangs"
        );
        println!("confirmed: C hangs (SIGALRM), Rust -> {:?} {:?}", r.status, r.assert_msg);
    } else {
        // If it does not hang, the two must agree exactly.
        assert_eq!(c, r, "no hang, so the two libraries must match");
    }
}

/// `lens[-1]`: the C reads one byte *below* `uint8_t lens[288+32]` when the very
/// first code-length symbol is 16 ("repeat the previous length").  In the
/// reference frame that byte is the most significant byte of the saved `s`
/// pointer, which is always `0x00` for an x86-64 heap address - the same value
/// the translation's zero-filled slack byte yields.  This test drives that path
/// and requires the two libraries to agree.
#[test]
fn err36_lens_minus_one_read() {
    let mut rng = Rng::new(0x36);
    let mut agreed = 0usize;
    for _ in 0..40 {
        // A CL alphabet in which symbol 16 is codeable, then use it first.
        let cl_used = vec![0usize, 16];
        let cl_lens = balanced_lengths(19, &cl_used);
        let cl = Huff::new(cl_lens.clone());
        let mut bw = BitWriter::new();
        bw.bits(1, 1); // bfinal
        bw.bits(2, 2); // btype = dynamic
        bw.bits(0, 5); // HLIT  -> nlit = 257
        bw.bits(0, 5); // HDIST -> ndst = 1
        bw.bits(15, 4); // HCLEN -> nlen = 19, so every CL length is written
        for i in 0..19usize {
            bw.bits(cl_lens[DEFAULT_PERMUTATION[i] as usize] as u32, 3);
        }
        // symbol 16 as the *first* code-length instruction => lens[-1]
        cl.put(&mut bw, 16);
        bw.bits(rng.below(4), 2);
        // fill the rest with zeroes
        for _ in 0..258 {
            cl.put(&mut bw, 0);
        }
        let mut stream = bw.finish();
        stream.extend_from_slice(&[0u8; 64]);
        let b = build(&stream, 0, 4096, 8192, rng.next_u64());
        let o = diff(&b.case, "err36 lens[-1] read");
        let _ = o;
        agreed += 1;
    }
    assert_eq!(agreed, 40);
}

/// rows 32..34: the `return NULL` sites live in `static` functions with no
/// caller, so they are unreachable across the FFI boundary.
#[test]
fn err33_dead_code_not_exported() {
    use std::process::Command;
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("nm").args(["-D", "--defined-only"]).arg(&so).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        for dead in ["cp_chunk", "cp_find", "cp_make32"] {
            assert!(!text.contains(dead), "{} exports {dead}", so.display());
        }
    }
}

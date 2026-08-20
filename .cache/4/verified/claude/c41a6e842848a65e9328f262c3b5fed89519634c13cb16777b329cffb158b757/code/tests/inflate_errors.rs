//! Phase C — error/rejection paths of `cp_inflate`
//! (`ERRORS.md` rows E1…E16 and E26…E30).

mod common;

use common::deflate::*;
use common::{InflateHarness, Outcome, Rng};
use std::collections::BTreeMap;

const ERR_LEN_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const ERR_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.";
const ERR_OUT_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.";
const ERR_BACKWARDS: &[u8] = b"Attempted to write before out buffer (invalid backwards distance).";
const ERR_OUT_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.";

#[track_caller]
fn expect_reject(ctx: &str, o: &Outcome, msg: &[u8]) {
    assert_eq!(o.signal, None, "[{ctx}] expected a clean rejection: {o:?}");
    assert_eq!(o.ret, 0, "[{ctx}] expected ret == 0: {o:?}");
    assert_eq!(
        o.err.as_deref(),
        Some(msg),
        "[{ctx}] wrong cp_error_reason: {:?}",
        o.err.as_ref().map(|e| String::from_utf8_lossy(e))
    );
}

#[track_caller]
fn expect_abort(ctx: &str, o: &Outcome, needle: &str) {
    assert_eq!(
        o.signal,
        Some(libc::SIGABRT),
        "[{ctx}] expected SIGABRT: {o:?}"
    );
    assert!(
        o.stderr.contains(needle),
        "[{ctx}] assertion message {:?} does not contain {needle:?}",
        o.stderr
    );
}

/// Stored block with explicit LEN / NLEN fields.
fn stored_raw(bfinal: bool, len_field: u16, nlen_field: u16, payload: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.bits(bfinal as u32, 1);
    bw.bits(0, 2);
    bw.align();
    bw.bits(len_field as u32, 16);
    bw.bits(nlen_field as u32, 16);
    bw.raw(payload);
    bw.finish()
}

fn fixed_stream(items: &[Item], bfinal: bool) -> Vec<u8> {
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, bfinal, items);
    bw.finish()
}

// ===========================================================================
// B. cp_error_reason + return 0
// ===========================================================================

/// E11 — `LEN != (uint16_t)~NLEN`
#[test]
fn e11_len_nlen_mismatch() {
    let h = InflateHarness::new("e11", 1 << 16, 1 << 14);
    let mut rng = Rng::new(0x4011);
    for it in 0..60 {
        let len = rng.range(3, 300) as usize;
        let payload = rng.bytes(len);
        // flip one bit of the (otherwise correct) NLEN field
        let bad_nlen = (!(len as u16)) ^ (1u16 << rng.below(16));
        let stream = stored_raw(true, len as u16, bad_nlen, &payload);
        let ctx = format!("E11 len={len} #{it}");
        let o = h.call(&ctx, &stream, 0, 4096);
        expect_reject(&ctx, &o, ERR_LEN_NLEN);
    }
    // and the degenerate LEN == NLEN == 0 case
    let stream = stored_raw(true, 0, 0, &[1, 2, 3, 4, 5, 6, 7, 8]);
    let o = h.call("E11 zero", &stream, 0, 4096);
    expect_reject("E11 zero", &o, ERR_LEN_NLEN);
}

/// E12 — `s->bits_left / 8 > LEN`, i.e. the stored block does not run to the
/// end of the input.
#[test]
fn e12_stored_beyond_end() {
    let h = InflateHarness::new("e12", 1 << 16, 1 << 14);
    let mut rng = Rng::new(0x4012);
    for it in 0..60 {
        let payload_len = rng.range(16, 400) as usize;
        let payload = rng.bytes(payload_len);
        // LEN smaller than the number of remaining input bytes
        let len_field = rng.range(0, payload_len as i32 - 8) as u16;
        let stream = stored_raw(true, len_field, !len_field, &payload);
        let ctx = format!("E12 LEN={len_field} remaining={payload_len} #{it}");
        let o = h.call(&ctx, &stream, 0, 4096);
        expect_reject(&ctx, &o, ERR_STORED_BEYOND);
    }
}

/// E13/E27 — literal decoded with a full (or NULL / zero-sized) output buffer.
#[test]
fn e13_out_full_on_literal() {
    let h = InflateHarness::new("e13", 1 << 16, 1 << 14);
    let stream = fixed_stream(&[Item::Lit(65), Item::Lit(66), Item::Lit(67)], true);

    for out_bytes in [0i32, 1, 2] {
        let ctx = format!("E13 out_bytes={out_bytes}");
        let o = h.call(&ctx, &stream, 0, out_bytes);
        expect_reject(&ctx, &o, ERR_OUT_SYMBOL);
    }
    // E27: out == NULL with out_bytes == 0 — the check happens before any store
    let o = h.call_raw(
        "E27 null out",
        &stream,
        0,
        stream.len() as i32,
        0,
        false,
        true,
    );
    expect_reject("E27 null out", &o, ERR_OUT_SYMBOL);
}

/// E28 — negative `out_bytes` makes `out_end < out`.
#[test]
fn e28_negative_out_bytes() {
    let h = InflateHarness::new("e28", 1 << 16, 1 << 14);
    let stream = fixed_stream(&[Item::Lit(65)], true);
    for out_bytes in [-1i32, -7, -4096, i32::MIN / 2] {
        let ctx = format!("E28 out_bytes={out_bytes}");
        let o = h.call(&ctx, &stream, 0, out_bytes);
        expect_reject(&ctx, &o, ERR_OUT_SYMBOL);
    }
}

/// E14 — a match whose `backwards_distance` reaches before the start of `out`.
#[test]
fn e14_backwards_distance() {
    let h = InflateHarness::new("e14", 1 << 16, 1 << 14);
    // first symbol of the stream is a match => nothing has been written yet
    for dist in [1u32, 2, 5, 100, 32768] {
        let stream = fixed_stream(&[Item::Match(3, dist)], true);
        let ctx = format!("E14 dist={dist} at offset 0");
        let o = h.call(&ctx, &stream, 0, 4096);
        expect_reject(&ctx, &o, ERR_BACKWARDS);
    }
    // one byte written, distance 2
    let stream = fixed_stream(&[Item::Lit(65), Item::Match(3, 2)], true);
    let o = h.call("E14 one past", &stream, 0, 4096);
    expect_reject("E14 one past", &o, ERR_BACKWARDS);

    // exactly at the boundary must succeed
    let stream = fixed_stream(&[Item::Lit(65), Item::Match(3, 1)], true);
    let o = h.call("E14 boundary ok", &stream, 0, 4096);
    assert_eq!(o.ret, 1, "distance == bytes written must be accepted: {o:?}");
}

/// E15 — a match that does not fit into the remaining output space.
#[test]
fn e15_out_full_on_match() {
    let h = InflateHarness::new("e15", 1 << 16, 1 << 14);
    let mut rng = Rng::new(0x4015);
    for it in 0..60 {
        let pre = rng.range(2, 20) as usize;
        let dist = rng.range(1, pre as i32) as u32;
        let length = rng.range(3, 258) as u32;
        let mut items: Vec<Item> = (0..pre).map(|_| Item::Lit(rng.u8() as u16)).collect();
        items.push(Item::Match(length, dist));
        let stream = fixed_stream(&items, true);
        // room for the literals but not for the whole match
        let out_bytes = pre as i32 + length as i32 - 1 - rng.below(length) as i32;
        let ctx = format!("E15 pre={pre} dist={dist} len={length} out={out_bytes} #{it}");
        let o = h.call(&ctx, &stream, 0, out_bytes.max(pre as i32));
        expect_reject(&ctx, &o, ERR_OUT_STRING);
    }
}

/// E16/E30 — `BTYPE == 3`, and all four values of the 2-bit block-type field.
#[test]
fn e16_btype_3() {
    let h = InflateHarness::new("e16", 1 << 16, 1 << 14);
    for bfinal in [0u32, 1] {
        let mut bw = BitWriter::new();
        bw.bits(bfinal, 1);
        bw.bits(3, 2);
        bw.align();
        let mut stream = bw.finish();
        stream.extend([0u8; 7]);
        let ctx = format!("E16 bfinal={bfinal}");
        let o = h.call(&ctx, &stream, 0, 4096);
        expect_reject(&ctx, &o, ERR_UNKNOWN_BLOCK);
    }
}

/// E30 — the whole 2-bit "enum": 0 and 1 and 2 are handled, 3 is rejected.
#[test]
fn e30_all_btype_values() {
    let h = InflateHarness::new("e30", 1 << 16, 1 << 14);

    // BTYPE 0
    let payload = vec![9u8; 32];
    let mut bw = BitWriter::new();
    emit_stored_block(&mut bw, true, &payload, None);
    let o = h.call("E30 btype=0", &bw.finish(), 0, 4096);
    assert_eq!((o.signal, o.ret), (None, 1), "btype 0: {o:?}");
    assert_eq!(&o.out[..32], &payload[..]);

    // BTYPE 1
    let o = h.call("E30 btype=1", &fixed_stream(&[Item::Lit(7)], true), 0, 4096);
    assert_eq!((o.signal, o.ret), (None, 1), "btype 1: {o:?}");

    // BTYPE 2
    let used = vec![7usize, 8, 256];
    let litlens = lengths_for(257, &used);
    let dstlens = lengths_for(1, &[0]);
    let mut bw = BitWriter::new();
    let (lit, dst) = emit_dynamic_header(&mut bw, true, &litlens, &dstlens, ClMode::Literal, None);
    emit_items(&mut bw, &lit, &dst, &[Item::Lit(7), Item::Lit(8)]);
    let o = h.call("E30 btype=2", &bw.finish(), 0, 4096);
    assert_eq!((o.signal, o.ret), (None, 1), "btype 2: {o:?}");

    // BTYPE 3
    let mut bw = BitWriter::new();
    bw.bits(1, 1);
    bw.bits(3, 2);
    bw.align();
    let mut s = bw.finish();
    s.extend([0u8; 7]);
    let o = h.call("E30 btype=3", &s, 0, 4096);
    expect_reject("E30 btype=3", &o, ERR_UNKNOWN_BLOCK);
}

// ===========================================================================
// A. assert() rejections
// ===========================================================================

/// E6 — `in_bytes == 0` (and negative) ⇒ `assert(s->bits_left > 0)`.
#[test]
fn e6_zero_and_negative_in_bytes() {
    let h = InflateHarness::new("e6", 1 << 16, 1 << 14);
    let stream = fixed_stream(&[Item::Lit(65)], true);
    for align in 0..4usize {
        let ctx = format!("E6 in_bytes=0 align={align}");
        let o = h.call_raw(&ctx, &stream, align, 0, 4096, false, false);
        expect_abort(
            &ctx,
            &o,
            "lib.c:125: cp_read_bits: Assertion `s->bits_left > 0' failed.",
        );
    }
    // Negative `in_bytes`: `last_bytes = (in_bytes - first_bytes) & 3` makes the
    // final-word loop read `in[in_bytes - last_bytes + i]`, i.e. |in_bytes| bytes
    // *before* `in`. Offset the stream 64 bytes into the mapping so that for
    // small negative sizes those reads stay mapped and the `bits_left > 0`
    // assert is what fires.
    for in_bytes in -8i32..0 {
        for align in 64..68usize {
            let ctx = format!("E6 in_bytes={in_bytes} align={align}");
            let o = h.call_raw(&ctx, &stream, align, in_bytes, 4096, false, false);
            expect_abort(
                &ctx,
                &o,
                "lib.c:125: cp_read_bits: Assertion `s->bits_left > 0' failed.",
            );
        }
    }
    // Large negative sizes make that same read fall off the front of the
    // mapping; both libraries fault identically.
    for in_bytes in [-4096i32, -100_000, i32::MIN + 1] {
        for align in 0..4usize {
            let ctx = format!("E6 in_bytes={in_bytes} align={align}");
            let o = h.call_raw(&ctx, &stream, align, in_bytes, 4096, false, false);
            assert!(
                o.signal == Some(libc::SIGSEGV) || o.signal == Some(libc::SIGABRT),
                "[{ctx}] {o:?}"
            );
        }
    }
}

/// E3 — truncated static block ⇒ `assert(s->count >= num_bits_to_read)` in
/// `cp_consume_bits`.
#[test]
fn e3_consume_bits_underflow() {
    let h = InflateHarness::new("e3", 1 << 16, 1 << 14);
    // 1 byte: BFINAL=1, BTYPE=01, then only 5 bits left for a >= 7 bit code
    let o = h.call("E3 [0x03]", &[0x03], 0, 4096);
    expect_abort(
        "E3 [0x03]",
        &o,
        "lib.c:115: cp_consume_bits: Assertion `s->count >= num_bits_to_read' failed.",
    );
}

/// E26 — `in == NULL` with `in_bytes >= 4` ⇒ `SIGSEGV` reading `words[0]`.
#[test]
fn e26_null_in() {
    let h = InflateHarness::new("e26", 1 << 16, 1 << 14);
    for in_bytes in [4i32, 8, 64] {
        let ctx = format!("E26 null in in_bytes={in_bytes}");
        let o = h.call_raw(&ctx, &[], 0, in_bytes, 4096, true, false);
        assert_eq!(o.signal, Some(libc::SIGSEGV), "[{ctx}] {o:?}");
    }
    // 1..3 bytes: no word load, but the final-partial-word loop still reads in[i]
    for in_bytes in [1i32, 2, 3] {
        let ctx = format!("E26 null in in_bytes={in_bytes}");
        let o = h.call_raw(&ctx, &[], 0, in_bytes, 4096, true, false);
        assert_eq!(o.signal, Some(libc::SIGSEGV), "[{ctx}] {o:?}");
    }
    // in_bytes == 0 never dereferences `in`
    let o = h.call_raw("E26 null in 0", &[], 0, 0, 4096, true, false);
    expect_abort(
        "E26 null in 0",
        &o,
        "lib.c:125: cp_read_bits: Assertion `s->bits_left > 0' failed.",
    );
}

/// E10 — incomplete Huffman tree ⇒ `assert((search >> len) == (key >> len))`.
///
/// The literal alphabet gets a *single* 1-bit code (for symbol 256), so the
/// Kraft sum is 1/2 and the bit pattern `1` matches no code at all.
#[test]
fn e10_decode_key_mismatch() {
    let h = InflateHarness::new("e10", 1 << 16, 1 << 14);
    let litlens = lengths_for(257, &[256]); // only symbol 256, length 1
    let dstlens = lengths_for(1, &[0]);
    assert_eq!(HuffEnc::new(litlens.clone()).kraft(), 1 << 14);

    for first_bit in [0u32, 1] {
        let mut bw = BitWriter::new();
        let _ = emit_dynamic_header(&mut bw, true, &litlens, &dstlens, ClMode::Literal, None);
        bw.bits(first_bit, 1);
        bw.align();
        let mut stream = bw.finish();
        stream.extend([0u8; 8]);
        let ctx = format!("E10 first_bit={first_bit}");
        let o = h.call(&ctx, &stream, 0, 4096);
        if first_bit == 0 {
            // `0` *is* the code for symbol 256 -> empty block, clean success
            assert_eq!((o.signal, o.ret), (None, 1), "[{ctx}] {o:?}");
        } else {
            expect_abort(
                &ctx,
                &o,
                "lib.c:217: cp_decode: Assertion `(search >> len) == (key >> len)' failed.",
            );
        }
    }
}

/// E4 — `cp_len_extra_bits` is a writable exported global; 33 extra bits trips
/// `assert(num_bits_to_read <= 32)`.
///
/// E5 — the closest reachable value on the other side (`255`) still trips 123
/// first, which is why `assert(num_bits_to_read >= 0)` (line 124) is
/// unreachable: the argument comes from a `uint8_t` table or a non-negative
/// literal.
#[test]
fn e4_num_bits_gt_32_via_global() {
    use common::libs;
    let h = InflateHarness::new("e4", 1 << 16, 1 << 14);
    let stream = fixed_stream(&[Item::Lit(65), Item::Match(3, 1)], true);
    let (c, r) = libs();

    for val in [33u8, 64, 128, 255] {
        let mut outs = Vec::new();
        for (lib, buf) in [(c, &h.out_c), (r, &h.out_r)] {
            h.inbuf.fill(0);
            h.inbuf.write_at(0, &stream);
            buf.fill(0xA5);
            let ip = h.inbuf.ptr() as *mut std::ffi::c_void;
            let op = buf.ptr() as *mut std::ffi::c_void;
            let f = lib.cp_inflate;
            let tbl = lib.cp_len_extra_bits;
            let n = stream.len() as i32;
            outs.push(
                h.runner
                    .run(lib.cp_error_reason, buf, move || unsafe {
                        *tbl = val; // cp_len_extra_bits[0] = val
                        f(ip, n, op, 4096)
                    })
                    .normalize(),
            );
        }
        let ctx = format!("E4 cp_len_extra_bits[0]={val}");
        common::same(&ctx, &outs[0], &outs[1]);
        expect_abort(
            &ctx,
            &outs[0],
            "lib.c:123: cp_read_bits: Assertion `num_bits_to_read <= 32' failed.",
        );
    }
}

/// E9 — `cp_fixed_table` is a writable exported global; a code length of 16
/// trips `assert(len < 16)` in `cp_build`.
#[test]
fn e9_code_length_ge_16_via_global() {
    use common::libs;
    let h = InflateHarness::new("e9", 1 << 16, 1 << 14);
    let stream = fixed_stream(&[Item::Lit(65)], true);
    let (c, r) = libs();

    for val in [16u8, 17, 15] {
        let mut outs = Vec::new();
        for (lib, buf) in [(c, &h.out_c), (r, &h.out_r)] {
            h.inbuf.fill(0);
            h.inbuf.write_at(0, &stream);
            buf.fill(0xA5);
            let ip = h.inbuf.ptr() as *mut std::ffi::c_void;
            let op = buf.ptr() as *mut std::ffi::c_void;
            let f = lib.cp_inflate;
            let tbl = lib.cp_fixed_table;
            let n = stream.len() as i32;
            outs.push(
                h.runner
                    .run(lib.cp_error_reason, buf, move || unsafe {
                        *tbl = val; // cp_fixed_table[0] = val
                        f(ip, n, op, 4096)
                    })
                    .normalize(),
            );
        }
        let ctx = format!("E9 cp_fixed_table[0]={val}");
        common::same(&ctx, &outs[0], &outs[1]);
        if val >= 16 {
            expect_abort(
                &ctx,
                &outs[0],
                "lib.c:154: cp_build: Assertion `len < 16' failed.",
            );
        }
    }
}

// ===========================================================================
// E29 / E1 / E7 / E8 / E2 / E5 — exhaustive truncation search
// ===========================================================================

/// The single 15-byte stream that reaches `cp_ptr()` with `bits_left & 7 != 0`.
///
/// Derivation (see `ERRORS.md` E1). `cp_stored()` aligns the bit stream with
/// `cp_read_bits(s, s->count & 7)`, which only really byte-aligns `bits_left`
/// while every top-up in `cp_peak_bits` was a whole 32-bit word. The
/// *final partial word* path instead does `count += s->bits_left`, so if it runs
/// while the consumed-bit count is not a multiple of 8, `count` and `bits_left`
/// desynchronise and the "alignment" read consumes the wrong number of bits.
///
/// Requirements, with `B`/`C` = `bits_left`/`count` at the stored block header:
/// `B >= (C & 7) + 17`, `C >= (C & 7) + 32`, `LEN == (uint16_t)~NLEN`, and
/// `(B - (C&7) - 32) & 7 != 0`. Since `B <= count_before + 8*last_bytes <= 39` at
/// the final-word load, the load must happen with `count_before == 15` and
/// `last_bytes == 3`, and exactly 10 bits (end-of-block + `BFINAL`/`BTYPE`) may
/// be consumed afterwards.
///
/// * 15 input bytes (`word_count == 3`, `last_bytes == 3`)
/// * static block, `BFINAL=0`: 3 literals with 8-bit codes + 6 with 9-bit codes
///   = 3 + 24 + 54 = 81 consumed bits, at which point `count == 15` and
///   `word_index == word_count`, so decoding the end-of-block symbol triggers
///   the final-word load
/// * then a stored block whose `LEN`/`NLEN` are read one bit early: the padding
///   bit at stream position 95 must be 1 and bytes 12/13/14 must be
///   `FF 7F 00`, which makes `LEN == 0xFFFF == (uint16_t)~NLEN`
fn cp_ptr_unaligned_stream() -> Vec<u8> {
    let lit = fixed_lit();
    let mut bw = BitWriter::new();
    bw.bits(0, 1); // BFINAL = 0
    bw.bits(1, 2); // BTYPE  = 01 (static)
    for _ in 0..3 {
        lit.emit(&mut bw, 0); // 8-bit code
    }
    for _ in 0..6 {
        lit.emit(&mut bw, 144); // 9-bit code
    }
    lit.emit(&mut bw, 256); // 7-bit end-of-block -> bit 88
    bw.bits(1, 1); // BFINAL = 1
    bw.bits(0, 2); // BTYPE  = 00 (stored)
    bw.bits(0, 4); // the 4 bits the (desynchronised) alignment read eats
    bw.bits(1, 1); // stream bit 95 — LEN's bit 0
    let s = bw.byte_len();
    assert_eq!(s, 12, "expected 12 bytes before LEN/NLEN");
    bw.raw(&[0xFF, 0x7F, 0x00]);
    let v = bw.finish();
    assert_eq!(v.len(), 15);
    v
}

/// A wide corpus of *malformed* inputs: every truncation of a set of valid
/// streams, at every input alignment, with several `out_bytes`.
fn truncation_corpus() -> Vec<(String, Vec<u8>)> {
    let mut rng = Rng::new(0x4029);
    let mut base: Vec<(String, Vec<u8>)> = Vec::new();

    base.push((
        "fixed-lit".into(),
        fixed_stream(
            &(0..24)
                .map(|i| Item::Lit((i * 11 % 256) as u16))
                .collect::<Vec<_>>(),
            true,
        ),
    ));
    base.push((
        "fixed-match".into(),
        fixed_stream(
            &[
                Item::Lit(65),
                Item::Lit(66),
                Item::Match(258, 2),
                Item::Match(11, 1),
                Item::Match(43, 200),
            ],
            true,
        ),
    ));
    {
        let used = vec![0usize, 1, 2, 3, 65, 66, 200, 256, 257, 260, 280];
        let litlens = lengths_for(288, &used);
        let dstlens = lengths_for(32, &[0, 1, 2, 5, 29, 30, 31]);
        let mut bw = BitWriter::new();
        let (lit, dst) =
            emit_dynamic_header(&mut bw, true, &litlens, &dstlens, ClMode::Repeats, None);
        // lengths chosen so their length symbols (257, 260, 280) are in `used`
        emit_items(
            &mut bw,
            &lit,
            &dst,
            &[
                Item::Lit(65),
                Item::Lit(66),
                Item::Lit(0),
                Item::Match(3, 1),
                Item::Match(6, 2),
                Item::Match(120, 7),
            ],
        );
        base.push(("dynamic".into(), bw.finish()));
    }
    {
        let payload = rng.bytes(40);
        let mut bw = BitWriter::new();
        emit_stored_block(&mut bw, true, &payload, None);
        base.push(("stored".into(), bw.finish()));
    }
    // E1: the exact stream that desynchronises `count` from `bits_left`
    base.push(("cp_ptr-unaligned".into(), cp_ptr_unaligned_stream()));
    // E12 shape: stored block whose LEN is smaller than the remaining input
    {
        let payload = rng.bytes(64);
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        bw.bits(4, 16);
        bw.bits(!4u16 as u32, 16);
        bw.raw(&payload);
        base.push(("stored-short-LEN".into(), bw.finish()));
    }
    // E15 shape: a match that cannot fit into the output buffer
    base.push((
        "match-too-long".into(),
        fixed_stream(
            &[Item::Lit(65), Item::Lit(66), Item::Lit(67), Item::Match(258, 3)],
            true,
        ),
    ));
    {
        // static block, then a stored block: the shape that can desynchronise
        // `count` from `bits_left` before `cp_ptr()` runs
        for nlit_pre in 0..8usize {
            let mut bw = BitWriter::new();
            let items: Vec<Item> = (0..nlit_pre).map(|i| Item::Lit(i as u16)).collect();
            emit_fixed_block(&mut bw, false, &items);
            let payload = rng.bytes(8);
            emit_stored_block(&mut bw, true, &payload, None);
            base.push((format!("fixed{nlit_pre}+stored"), bw.finish()));
        }
    }

    let mut out = Vec::new();
    for (name, s) in base {
        for n in 1..=s.len() {
            out.push((format!("{name}[..{n}]"), s[..n].to_vec()));
        }
    }
    // pure random noise
    for len in 1..=20usize {
        for i in 0..25 {
            out.push((format!("rand{len}#{i}"), rng.bytes(len)));
        }
    }
    out
}

/// The truncation sweep is expensive (two forks per input), so it is run exactly
/// once per test binary: every input is compared between the two libraries and
/// the distinct outcome classes are recorded together with an example input and
/// the exact `stderr` text.
struct Sweep {
    /// outcome class -> (example ctx, normalised stderr)
    classes: BTreeMap<String, (String, String)>,
    inputs: usize,
    /// full description of every C-vs-Rust disagreement
    diverged: Vec<String>,
    /// inputs on which the C code performs undefined behaviour (classified by
    /// the independent model in `tests/common/cmodel.rs`); they have no defined
    /// behaviour for the Rust port to reproduce
    ub_inputs: usize,
    ub_kinds: BTreeMap<String, String>,
    /// disagreements between a library and the independent C model
    model_mismatch: Vec<String>,
}

fn sweep() -> &'static Sweep {
    static SWEEP: std::sync::OnceLock<Sweep> = std::sync::OnceLock::new();
    SWEEP.get_or_init(|| {
        let h = InflateHarness::new("sweep", 1 << 16, 1 << 13);
        let mut classes: BTreeMap<String, (String, String)> = BTreeMap::new();
        let corpus = truncation_corpus();
        let mut inputs = 0usize;
        let mut diverged: Vec<String> = Vec::new();
        let mut ub_inputs = 0usize;
        let mut ub_kinds: BTreeMap<String, String> = BTreeMap::new();
        let mut model_mismatch: Vec<String> = Vec::new();
        for (name, stream) in &corpus {
            for align in [0usize, 1, 3] {
                for out_bytes in [0i32, 8, 4096] {
                    let ctx = format!("{name} align={align} out={out_bytes}");
                    let (oc, or) = h.call_pair(stream, align, out_bytes);
                    inputs += 1;
                    let m = h.model(stream, align, stream.len() as i32, out_bytes);
                    if !m.defined() {
                        ub_inputs += 1;
                        for k in &m.ub {
                            ub_kinds.entry(k.clone()).or_insert(ctx.clone());
                        }
                        continue;
                    }
                    if oc != or && diverged.len() < 20 {
                        diverged.push(format!(
                            "{ctx}\n    bytes = {:02x?}\n    C    = {oc:?}\n    Rust = {or:?}",
                            stream
                        ));
                    }
                    for (who, o) in [("C", &oc), ("Rust", &or)] {
                        if let Err(e) = common::model_matches(o, &m) {
                            if model_mismatch.len() < 20 {
                                model_mismatch.push(format!(
                                    "{ctx} [{who}] {e}\n    bytes = {:02x?}\n    model = {:?}",
                                    stream, m.end
                                ));
                            }
                        }
                    }
                    classes
                        .entry(classify(&oc))
                        .or_insert((ctx, oc.stderr.clone()));
                }
            }
        }
        Sweep {
            classes,
            inputs,
            diverged,
            ub_inputs,
            ub_kinds,
            model_mismatch,
        }
    })
}

fn sweep_report() -> String {
    let s = sweep();
    let mut r = format!("outcome classes over {} inputs:\n", s.inputs);
    for (k, (ctx, _)) in &s.classes {
        r += &format!("  {k}\n      e.g. {ctx}\n");
    }
    r
}

/// Was any input in the sweep classified with a `stderr` containing `needle`?
fn sweep_saw(needle: &str) -> Option<&'static str> {
    sweep()
        .classes
        .values()
        .find(|(_, err)| err.contains(needle))
        .map(|(ctx, _)| ctx.as_str())
}

fn classify(o: &Outcome) -> String {
    if let Some(sig) = o.signal {
        if sig == libc::SIGABRT {
            // "lib.c:<line>: <func>: Assertion `<expr>' failed."
            let line = o
                .stderr
                .split(':')
                .nth(1)
                .unwrap_or("?")
                .trim()
                .to_string();
            let func = o.stderr.split(':').nth(2).unwrap_or("?").trim().to_string();
            format!("abort lib.c:{line} {func}")
        } else {
            format!("signal {sig}")
        }
    } else if o.ret == 1 {
        "ok".to_string()
    } else {
        match o.err.as_deref() {
            None => "ret0 (no message)".to_string(),
            Some(m) => format!("ret0 {}", String::from_utf8_lossy(m)),
        }
    }
}

/// E32/E29 — the two libraries must agree on **every** well-defined input of
/// the sweep, and both must agree with the independent C model.
#[test]
fn sweep_libraries_agree_on_every_input() {
    let s = sweep();
    assert!(
        s.diverged.is_empty(),
        "{} of {} sweep inputs diverged:\n  {}",
        s.diverged.len(),
        s.inputs,
        s.diverged.join("\n  ")
    );
    assert!(
        s.model_mismatch.is_empty(),
        "{} sweep inputs disagree with the independent C model:\n  {}",
        s.model_mismatch.len(),
        s.model_mismatch.join("\n  ")
    );
    // the sweep must still contain a healthy majority of well-defined inputs
    assert!(
        s.ub_inputs * 4 < s.inputs,
        "{} of {} sweep inputs hit C undefined behaviour — the corpus is not \
         exercising defined paths",
        s.ub_inputs,
        s.inputs
    );
    let mut report = format!(
        "{} of {} inputs perform C undefined behaviour; kinds:\n",
        s.ub_inputs, s.inputs
    );
    for (k, ctx) in &s.ub_kinds {
        report += &format!("  {k}\n      e.g. {ctx}\n");
    }
    println!("{report}");
}

/// E29 (every truncation), E1, E8, and the *unreachability* of E2/E5/E7.
///
/// The sweep itself already asserts byte-identical behaviour of the two
/// libraries for every input; this test additionally establishes *mechanically*
/// which assert lines are reachable instead of guessing.
#[test]
fn e29_all_truncations_and_assert_coverage() {
    let report = sweep_report();
    println!("{report}");
    let seen: Vec<&String> = sweep().classes.keys().collect();

    // Reachable assert sites (E1, E3, E6, E8, E10) must really occur.
    for needle in [
        "abort lib.c:95 cp_ptr",
        "abort lib.c:115 cp_consume_bits",
        "abort lib.c:125 cp_read_bits",
        "abort lib.c:127 cp_read_bits",
        "abort lib.c:217 cp_decode",
    ] {
        assert!(
            seen.iter().any(|k| k.as_str() == needle),
            "expected outcome class {needle:?} was never produced.\n{report}"
        );
    }
    // E2 / E5 / E7 are unreachable by construction, and E9 needs a mutated
    // global, so none of them may show up here.
    for needle in [
        "abort lib.c:104 cp_peak_bits",
        "abort lib.c:124 cp_read_bits",
        "abort lib.c:126 cp_read_bits",
        "abort lib.c:154 cp_build",
    ] {
        assert!(
            !seen.iter().any(|k| k.as_str() == needle),
            "{needle:?} was considered unreachable but occurred.\n{report}"
        );
    }
    // All six `cp_error_reason` messages must be observed too.
    for msg in [
        "Failed to find LEN and NLEN",
        "Stored block extends beyond",
        "outputting a symbol",
        "invalid backwards distance",
        "outputting a string",
        "unknown block type",
    ] {
        assert!(
            seen.iter().any(|k| k.contains(msg)),
            "error message {msg:?} never observed in the sweep.\n{report}"
        );
    }
}

/// E1 — an input that reaches `cp_ptr()` with `bits_left & 7 != 0`.
///
/// See [`cp_ptr_unaligned_stream`] for the derivation.
#[test]
fn e1_cp_ptr_unaligned_bits_left() {
    let h = InflateHarness::new("e1", 1 << 16, 1 << 13);
    let stream = cp_ptr_unaligned_stream();
    let o = h.call("E1 cp_ptr unaligned", &stream, 0, 4096);
    expect_abort(
        "E1 cp_ptr unaligned",
        &o,
        "lib.c:95: cp_ptr: Assertion `!(s->bits_left & 7)' failed.",
    );
    // the sweep contains the same stream, so it must see this class too
    assert!(
        sweep_saw("lib.c:95: cp_ptr: Assertion `!(s->bits_left & 7)' failed.").is_some(),
        "sweep did not reproduce the cp_ptr assert.\n{}",
        sweep_report()
    );
}

/// E8 — an input that trips `cp_would_overflow`.
#[test]
fn e8_would_overflow() {
    let ctx = sweep_saw(
        "lib.c:127: cp_read_bits: Assertion `!cp_would_overflow(s, num_bits_to_read)' failed.",
    );
    assert!(
        ctx.is_some(),
        "cp_would_overflow was never tripped.\n{}",
        sweep_report()
    );
}

/// E2 — provably unreachable: `word_index` is only incremented inside
/// `if (word_index < word_count)`.
#[test]
fn e2_unreachable_word_index_invariant() {
    assert!(
        sweep_saw("lib.c:104: cp_peak_bits").is_none(),
        "assert at lib.c:104 fired, so it is not unreachable after all"
    );
}

/// E5 — `assert(num_bits_to_read >= 0)`: unreachable, the argument is a
/// non-negative literal, `count & 7`, or a `uint8_t` table entry (see also
/// `e4_num_bits_gt_32_via_global`, which shows that even 255 trips line 123
/// first). The closest reachable boundary is `0`.
#[test]
fn e5_unreachable_negative_num_bits() {
    assert!(
        sweep_saw("lib.c:124: cp_read_bits").is_none(),
        "assert at lib.c:124 fired, so it is not unreachable after all"
    );
    // `cp_read_bits(s, 0)` really does happen (stored block already aligned):
    let h = InflateHarness::new("e5", 1 << 16, 1 << 13);
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, false, &[Item::Lit(1), Item::Lit(2), Item::Lit(3)]);
    let payload = vec![7u8; 24];
    emit_stored_block(&mut bw, true, &payload, None);
    let stream = bw.finish();
    h.call("E5 zero-bit read", &stream, 0, 4096);
}

/// E7 — `assert(s->count <= 64)`: unreachable.
#[test]
fn e7_unreachable_count_gt_64() {
    assert!(
        sweep_saw("lib.c:126: cp_read_bits").is_none(),
        "assert at lib.c:126 fired, so it is not unreachable after all"
    );
}

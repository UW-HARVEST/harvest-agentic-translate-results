//! Phase C — error-path differential tests, one per ERRORS.md row.
//!
//! Every test asserts that C and Rust reject *identically*: same return value,
//! same `cp_error_reason` string, same termination signal. Where the C dies on a
//! live `assert()`, the assertion expression is read back out of the child's
//! stderr so the test proves it hit the intended row rather than "failed
//! somehow".

mod harness;

use harness::deflate::*;
use harness::{diff, diff_inflate, diff_unfilter, pair, run, Outcome, Pair, Rng, OFF_IN, OFF_OUT};

const PAD: usize = 4096;
const TAIL: usize = 4096;

#[track_caller]
fn expect_abort(tag: &str, c: &Outcome, expr: &str) {
    assert!(
        !c.completed && c.signal == libc::SIGABRT,
        "[{tag}] expected the C to abort on `{expr}`, got {c:?}"
    );
    let got = c
        .assert_expr()
        .unwrap_or_else(|| panic!("[{tag}] C aborted but printed no assertion: {c:?}"));
    assert_eq!(got, expr, "[{tag}] wrong assert fired");
}

#[track_caller]
fn expect_reject(tag: &str, c: &Outcome, msg: &str) {
    assert!(c.completed, "[{tag}] expected a normal return, got {c:?}");
    assert_eq!(c.ret, 0, "[{tag}] expected cp_inflate == 0, got {c:?}");
    assert_eq!(
        c.err.as_deref().map(String::from_utf8_lossy),
        Some(std::borrow::Cow::Borrowed(msg)),
        "[{tag}] wrong cp_error_reason"
    );
}

fn inflate_both(p: &Pair, tag: &str, input: &[u8], align: usize, in_bytes: i32, out_bytes: i32) -> Vec<Outcome> {
    diff_inflate(p, tag, input, align, in_bytes, out_bytes, 4096)
}

// ===========================================================================
// A. assert() rows
// ===========================================================================

/// E1 — `cp_ptr`: `assert(!(s->bits_left & 7))`.
///
/// Reaching this needs the `count ≡ bits_left (mod 8)` invariant to be broken
/// *before* a stored block's byte-alignment read. The invariant only breaks in
/// `cp_peak_bits`'s tail branch (`count += s->bits_left` rather than
/// `+= last_bytes * 8`) when `count > 0`, which cannot happen during the
/// stored-block prologue itself — so it must be broken by an earlier Huffman
/// block in the same stream. This test searches that space exhaustively and
/// requires C/Rust agreement on every candidate.
#[test]
fn e1_cp_ptr_bits_left_alignment() {
    let p = pair();
    let mut rng = Rng::new(0xE100_0000_0000_0001);
    let mut hits = 0usize;
    let mut aborts = 0usize;
    for nlit in 1usize..24 {
        for stored_len in 0usize..6 {
            for align in 0..4usize {
                let mut e = Enc::new();
                let toks: Vec<Tok> = (0..nlit).map(|_| Tok::Lit(rng.u8())).collect();
                e.fixed_block(false, &toks);
                let payload = rng.bytes(stored_len);
                e.stored_block(true, &payload);
                let input = e.finish();
                let tag = format!("E1/nlit={nlit},stored={stored_len},align={align}");
                let outs = inflate_both(&p, &tag, &input, align, input.len() as i32, 4096);
                if !outs[0].completed {
                    aborts += 1;
                    if outs[0].assert_expr().as_deref() == Some("!(s->bits_left & 7)") {
                        hits += 1;
                    }
                }
            }
        }
    }
    eprintln!("E1: {aborts} aborts over the search space, {hits} of them `!(s->bits_left & 7)`");
    // Whether or not the row is reachable, C and Rust agreed on every case above
    // (diff_inflate panics otherwise). Record which it was.
    if hits == 0 {
        eprintln!(
            "E1 appears unreachable from cp_inflate: the invariant count = bits_left (mod 8) \
             holds through every stored-block prologue."
        );
    }
}

/// E3 — `cp_consume_bits`: `assert(s->count >= num_bits_to_read)`.
///
/// A stored block whose LEN/NLEN read exhausts the buffered bits: `in_bytes = 4`
/// at `align = 3` gives `first_bytes = 1`, `word_count = 0`, `last_bytes = 3`, so
/// the second 16-bit read finds only 8 buffered bits and no tail word left.
#[test]
fn e3_cp_consume_bits_count_underflow() {
    let p = pair();
    let input = [0x00u8, 0x00, 0x00, 0x00];
    let outs = inflate_both(&p, "E3", &input, 3, 4, 4096);
    expect_abort("E3", &outs[0], "s->count >= num_bits_to_read");
}

/// E6 — `cp_read_bits`: `assert(s->bits_left > 0)`.
#[test]
fn e6_cp_read_bits_no_bits_left() {
    let p = pair();
    let mut rng = Rng::new(0xE600_0000_0000_0006);
    // in_bytes == 0 → bits_left == 0 on the very first read
    for align in 0..4usize {
        let outs = inflate_both(&p, &format!("E6/empty align={align}"), &[], align, 0, 64);
        expect_abort("E6/empty", &outs[0], "s->bits_left > 0");
    }
    // in_bytes < 0 → bits_left < 0
    for in_bytes in [-1i32, -4, -100, i32::MIN / 8] {
        let input = rng.bytes(16);
        let outs = inflate_both(&p, &format!("E6/neg={in_bytes}"), &input, 0, in_bytes, 64);
        expect_abort("E6/neg", &outs[0], "s->bits_left > 0");
    }
    // All-zero inputs: bfinal = 0, btype = 0, so these are *stored* blocks with
    // LEN = NLEN = 0 and are rejected by E11 rather than tripping an assert.
    // Only agreement is required (diff_inflate enforces it); the point of the row
    // is that neither library aborts here.
    for n in 1usize..=8 {
        let input = vec![0u8; n];
        let outs = inflate_both(&p, &format!("E6/zeros n={n}"), &input, 0, n as i32, 64);
        if n >= 5 {
            expect_reject(
                &format!("E6/zeros n={n}"),
                &outs[0],
                "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
            );
        }
    }
    // Truncated so short that the LEN/NLEN read itself runs out of input.
    for n in 1usize..=4 {
        let input = vec![0u8; n];
        let outs = inflate_both(&p, &format!("E6/short n={n}"), &input, 0, n as i32, 64);
        assert!(
            !outs[0].completed,
            "n={n} should not complete: {:?}",
            outs[0]
        );
    }
}

/// E8 — `cp_read_bits`: `assert(!cp_would_overflow(s, num_bits_to_read))`.
/// Truncated streams: a 1-byte input announcing a fixed-Huffman block.
#[test]
fn e8_cp_read_bits_would_overflow() {
    let p = pair();
    let mut rng = Rng::new(0xE800_0000_0000_0008);
    let mut hits = 0usize;
    // truncate well-formed fixed blocks at every byte length
    let toks = random_toks(&mut rng, 60, true, 32);
    let mut e = Enc::new();
    e.fixed_block(true, &toks);
    let full = e.finish();
    for cut in 1..full.len().min(30) {
        for align in 0..4usize {
            let tag = format!("E8/cut={cut},align={align}");
            let outs = inflate_both(&p, &tag, &full[..cut], align, cut as i32, 4096);
            if !outs[0].completed
                && outs[0].assert_expr().as_deref()
                    == Some("!cp_would_overflow(s, num_bits_to_read)")
            {
                hits += 1;
            }
        }
    }
    assert!(hits > 0, "E8 was never reached by truncation");
    eprintln!("E8: hit `!cp_would_overflow(...)` {hits} times");
}

/// E10 — `cp_decode`: `assert((search >> len) == (key >> len))`.
///
/// Two shapes: (a) a dynamic block whose HCLEN code lengths are all zero, so the
/// code-length tree is empty and `cp_decode` reads `tree[-1]`; (b) bit patterns
/// that are not valid codes for the fixed tree.
#[test]
fn e10_cp_decode_key_mismatch() {
    let p = pair();

    // (a) dynamic block, HCLEN = 4, all four transmitted lengths = 0.
    let mut w = BitWriter::new();
    w.bits(1, 1); // bfinal
    w.bits(2, 2); // btype = dynamic
    w.bits(0, 5); // HLIT  = 257
    w.bits(0, 5); // HDIST = 1
    w.bits(0, 4); // HCLEN = 4
    for _ in 0..4 {
        w.bits(0, 3); // every code length zero → empty tree
    }
    for _ in 0..8 {
        w.bits(0xFF, 8); // padding so bits remain available
    }
    let input = w.finish();
    let outs = inflate_both(&p, "E10/empty-cl-tree", &input, 0, input.len() as i32, 4096);
    expect_abort(
        "E10/empty-cl-tree",
        &outs[0],
        "(search >> len) == (key >> len)",
    );

    // (b) an *incomplete* literal code: only three symbols at length 2, so the
    // bit pattern `11` has no code at all. (The RFC-1951 fixed code is complete,
    // which is why no random bit string can ever be undecodable against it —
    // hence this row needs a dynamic block.)
    let mut lit_lens = vec![0u8; 257];
    lit_lens[65] = 2; // code 00
    lit_lens[66] = 2; // code 01
    lit_lens[256] = 2; // code 10
    assert!(!is_complete(&lit_lens), "the row needs an INCOMPLETE code");
    let dist_lens = vec![1u8, 1];

    for prefix in [0b11u32, 0b111, 0b1111] {
        let nbits = 32 - prefix.leading_zeros();
        let mut e = Enc::new();
        e.dynamic_header(true, &lit_lens, &dist_lens, RleMode::All);
        // emit `nbits` set bits: the first two are `11`, which is undecodable
        e.w.bits(prefix, nbits);
        for _ in 0..8 {
            e.w.bits(0xFF, 8);
        }
        let input = e.finish();
        let tag = format!("E10/incomplete-code prefix={prefix:#b}");
        let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, 4096);
        expect_abort(&tag, &outs[0], "(search >> len) == (key >> len)");
    }

    // (c) the same shape reached through a valid literal first, so cp_decode has
    // already succeeded once before the undecodable pattern appears.
    let lit_codes = canonical(&lit_lens);
    let mut e = Enc::new();
    e.dynamic_header(true, &lit_lens, &dist_lens, RleMode::All);
    e.w.code(lit_codes[65], 2);
    e.w.code(lit_codes[66], 2);
    e.w.bits(0b11, 2);
    for _ in 0..8 {
        e.w.bits(0xFF, 8);
    }
    let input = e.finish();
    let outs = inflate_both(&p, "E10/after-valid-literals", &input, 0, input.len() as i32, 4096);
    expect_abort(
        "E10/after-valid-literals",
        &outs[0],
        "(search >> len) == (key >> len)",
    );

    // (d) random bit strings against the fixed (complete) code: these can never
    // hit E10, and this pins that fact.
    let mut rng = Rng::new(0xE1_0000_0000_0010);
    let mut e10_hits = 0usize;
    for i in 0..300 {
        let mut body = vec![0u8; 24];
        for b in body.iter_mut() {
            *b = rng.u8();
        }
        body[0] = (body[0] & !0x07) | 0x03; // bfinal = 1, btype = 1 (fixed)
        let outs = inflate_both(
            &p,
            &format!("E10/random-fixed #{i}"),
            &body,
            0,
            body.len() as i32,
            4096,
        );
        if !outs[0].completed
            && outs[0].assert_expr().as_deref() == Some("(search >> len) == (key >> len)")
        {
            e10_hits += 1;
        }
    }
    assert_eq!(
        e10_hits, 0,
        "the RFC-1951 fixed code is complete, so E10 must be unreachable through it"
    );
}

/// E2 / E4 / E5 / E7 — proved unreachable by construction, so the requirement is
/// that neither library ever trips them. This exercises a wide corpus and
/// asserts the C never reports those four assertion expressions.
#[test]
fn e2_e4_e5_e7_unreachable_asserts() {
    let p = pair();
    let mut rng = Rng::new(0xE247_0000_0000_0007);
    let forbidden = [
        "s->word_index <= s->word_count",
        "num_bits_to_read <= 32",
        "num_bits_to_read >= 0",
        "s->count <= 64",
    ];
    for i in 0..6000 {
        let n = 1 + rng.below(40);
        let mut input = rng.bytes(n);
        // bias towards well-formed-looking block headers
        if i % 3 == 0 {
            input[0] = (input[0] & !0x06) | ((rng.below(3) as u8) << 1);
        }
        let align = i % 4;
        let out_bytes = [0i32, 1, 17, 4096][i % 4];
        eprint!("");
        let outs = inflate_both(
            &p,
            &format!("E2457/#{i} input={input:02x?} align={align} out={out_bytes}"),
            &input,
            align,
            input.len() as i32,
            out_bytes,
        );
        if let Some(a) = outs[0].assert_expr() {
            assert!(
                !forbidden.contains(&a.as_str()),
                "assert `{a}` was believed unreachable but fired on {input:02x?}"
            );
        }
    }
}

// ===========================================================================
// B. cp_error_reason rows
// ===========================================================================

/// E11 — stored block with `LEN != (uint16_t)~NLEN`.
#[test]
fn e11_stored_len_nlen_mismatch() {
    let p = pair();
    let mut rng = Rng::new(0xE110_0000_0000_0011);
    for len in [0usize, 1, 4, 8, 16] {
        for bad_nlen in [0u16, 1, 0xFFFF, 0x1234] {
            if bad_nlen == !(len as u16) {
                continue;
            }
            let payload = rng.bytes(len);
            let mut e = Enc::new();
            e.stored_block_len(true, &payload, len as u16, Some(bad_nlen));
            let input = e.finish();
            let tag = format!("E11/len={len},nlen={bad_nlen:#06x}");
            let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, 4096);
            expect_reject(
                &tag,
                &outs[0],
                "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
            );
        }
    }
}

/// E12 — stored block declaring a LEN shorter than the input that remains.
#[test]
fn e12_stored_extends_beyond_input() {
    let p = pair();
    let mut rng = Rng::new(0xE120_0000_0000_0012);
    for actual in [4usize, 8, 16, 64] {
        for declared in [0usize, 1, 2, 3] {
            if declared >= actual {
                continue;
            }
            let payload = rng.bytes(actual);
            let mut e = Enc::new();
            e.stored_block_len(true, &payload, declared as u16, None);
            let input = e.finish();
            let tag = format!("E12/actual={actual},declared={declared}");
            let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, 4096);
            expect_reject(&tag, &outs[0], "Stored block extends beyond end of input stream.");
        }
    }
    // also: a stored block that is not the last block in the stream
    let mut e = Enc::new();
    e.stored_block(false, &[1, 2, 3, 4]);
    e.stored_block(true, &[5, 6, 7, 8]);
    let input = e.finish();
    let outs = inflate_both(&p, "E12/not-last", &input, 0, input.len() as i32, 4096);
    expect_reject(
        "E12/not-last",
        &outs[0],
        "Stored block extends beyond end of input stream.",
    );
}

/// E13 — literal emitted with a full output buffer.
#[test]
fn e13_out_buffer_full_on_literal() {
    let p = pair();
    let mut rng = Rng::new(0xE130_0000_0000_0013);
    for n in [1usize, 2, 5, 20] {
        for out_bytes in 0..n as i32 {
            let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.u8())).collect();
            let mut e = Enc::new();
            e.fixed_block(true, &toks);
            let input = e.finish();
            let tag = format!("E13/n={n},out={out_bytes}");
            let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, out_bytes);
            expect_reject(
                &tag,
                &outs[0],
                "Attempted to overwrite out buffer while outputting a symbol.",
            );
        }
    }
    // negative out_bytes → out_end < out, so the very first literal is rejected
    for out_bytes in [-1i32, -100, i32::MIN / 2] {
        let toks = vec![Tok::Lit(0x41)];
        let mut e = Enc::new();
        e.fixed_block(true, &toks);
        let input = e.finish();
        let tag = format!("E13/negative out={out_bytes}");
        let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, out_bytes);
        expect_reject(
            &tag,
            &outs[0],
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
}

/// E14 — backwards distance reaching before the start of the output buffer.
#[test]
fn e14_distance_before_begin() {
    let p = pair();
    let mut rng = Rng::new(0xE140_0000_0000_0014);
    for produced in [0usize, 1, 2, 5] {
        for dist in [1u32, 2, 3, 10, 100, 32768] {
            if dist as usize <= produced {
                continue;
            }
            let mut toks: Vec<Tok> = (0..produced).map(|_| Tok::Lit(rng.u8())).collect();
            toks.push(Tok::RawMatch {
                len_sym: len_symbol(3).0,
                len_extra: 0,
                dist_sym: dist_symbol(dist).0,
                dist_extra: dist_symbol(dist).1,
            });
            let mut e = Enc::new();
            // `expand` cannot model this (it is invalid), so build by hand
            e.fixed_block(true, &toks_no_expand(&toks));
            let input = e.finish();
            let tag = format!("E14/produced={produced},dist={dist}");
            let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, 4096);
            expect_reject(
                &tag,
                &outs[0],
                "Attempted to write before out buffer (invalid backwards distance).",
            );
        }
    }
}

/// Identity helper: makes the intent explicit that these tokens are *invalid*
/// and must not be run through `expand`.
fn toks_no_expand(t: &[Tok]) -> Vec<Tok> {
    t.to_vec()
}

/// E15 — match length overrunning the output buffer.
///
/// The distance check (E14) is evaluated first, so the distance must be legal.
#[test]
fn e15_match_length_overruns_out() {
    let p = pair();
    let mut rng = Rng::new(0xE150_0000_0000_0015);
    for produced in [1usize, 4, 16] {
        for len in [3u32, 8, 100, 258] {
            for slack in 0..3usize {
                let out_bytes = (produced + slack) as i32 + (len as i32 - 1).min(0);
                if (out_bytes as usize) < produced {
                    continue;
                }
                if produced + slack >= produced + len as usize {
                    continue;
                }
                let mut toks: Vec<Tok> = (0..produced).map(|_| Tok::Lit(rng.u8())).collect();
                toks.push(Tok::Match(len, produced as u32));
                let mut e = Enc::new();
                e.fixed_block(true, &toks);
                let input = e.finish();
                let tag = format!("E15/produced={produced},len={len},slack={slack}");
                let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, out_bytes);
                expect_reject(
                    &tag,
                    &outs[0],
                    "Attempted to overwrite out buffer while outputting a string.",
                );
            }
        }
    }
}

/// E16 / E35 — `btype == 3`. Two bits can only encode 0..3, so this *is* the
/// complete "out-of-range block type" surface.
#[test]
fn e16_e35_unknown_block_type() {
    let p = pair();
    let mut rng = Rng::new(0xE160_0000_0000_0016);
    for bfinal in [0u32, 1] {
        for align in 0..4usize {
            for rep in 0..4 {
                let mut w = BitWriter::new();
                w.bits(bfinal, 1);
                w.bits(3, 2); // btype = 3
                for _ in 0..8 {
                    w.bits(rng.u8() as u32, 8);
                }
                let input = w.finish();
                let tag = format!("E16/bfinal={bfinal},align={align},rep={rep}");
                let outs = inflate_both(&p, &tag, &input, align, input.len() as i32, 4096);
                expect_reject(&tag, &outs[0], "Detected unknown block type within input stream.");
            }
        }
    }
    // btype = 3 reached only in a *later* block
    let mut rng2 = Rng::new(0xE160_0000_0000_0017);
    let toks = random_toks(&mut rng2, 10, false, 1);
    let mut e = Enc::new();
    e.fixed_block(false, &toks);
    drop(e);
    // Build the two-block stream by hand so the second block starts mid-byte,
    // exactly as a real bit-packed stream would.
    let mut w = BitWriter::new();
    w.bits(0, 1);
    w.bits(1, 2);
    let lit_lens = fixed_lit_lens();
    let lit_codes = canonical(&lit_lens);
    w.code(lit_codes[256], lit_lens[256] as u32); // immediate end-of-block
    w.bits(1, 1); // bfinal
    w.bits(3, 2); // btype = 3
    w.bits(0xFF, 8);
    let input = w.finish();
    let outs = inflate_both(&p, "E16/second-block", &input, 0, input.len() as i32, 4096);
    expect_reject(
        "E16/second-block",
        &outs[0],
        "Detected unknown block type within input stream.",
    );
}

// ===========================================================================
// C. unfilter rejection rows
// ===========================================================================

/// E17 / E27 / E36 — row-0 filter byte outside 0..=4, swept over *every*
/// invalid value 5..=255 (the out-of-range "enum" surface across the FFI).
#[test]
fn e17_e36_row0_invalid_filter_byte() {
    let p = pair();
    let mut rng = Rng::new(0xE170_0000_0000_0017);
    for f in 0u8..=255 {
        let mut data = rng.bytes(1 + 4 * 3 + 16);
        data[0] = f;
        let outs = diff_unfilter(&p, &format!("E17/f={f}"), 4, 1, 3, &data, PAD, TAIL);
        let c = &outs[0];
        assert!(c.completed, "[E17/f={f}] {c:?}");
        let want = if f <= 4 { 1 } else { 0 };
        assert_eq!(c.ret, want, "[E17/f={f}] unexpected return");
    }
}

/// E18 / E36 — invalid filter byte on a later row; earlier rows have already
/// been filtered in place, so the partial mutation must match too.
#[test]
fn e18_e36_later_row_invalid_filter_byte() {
    let p = pair();
    let mut rng = Rng::new(0xE180_0000_0000_0018);
    for bad_row in 1usize..4 {
        for f in 0u8..=255 {
            let (w, h, bpp) = (5i32, 4i32, 3i32);
            let len = (w * bpp) as usize;
            let mut data = rng.bytes(h as usize * (len + 1) + 16);
            for y in 0..h as usize {
                data[y * (len + 1)] = 4; // paeth, so earlier rows really change
            }
            data[bad_row * (len + 1)] = f;
            let outs = diff_unfilter(
                &p,
                &format!("E18/row={bad_row},f={f}"),
                w,
                h,
                bpp,
                &data,
                PAD,
                TAIL,
            );
            let c = &outs[0];
            assert!(c.completed, "[E18] {c:?}");
            let want = if f <= 4 { 1 } else { 0 };
            assert_eq!(c.ret, want, "[E18/row={bad_row},f={f}]");
        }
    }
}

// ===========================================================================
// D. generic FFI boundary rows
// ===========================================================================

/// E19 / E20 — `unfilter(raw = NULL)`.
#[test]
fn e19_e20_unfilter_null_raw() {
    let p = pair();
    let sh = &p.shared;
    for (w, h, bpp) in [
        (4i32, 1i32, 3i32),
        (4, 2, 3),
        (4, 0, 3),
        (4, -1, 3),
        (0, 0, 0),
    ] {
        let mut outs = Vec::new();
        for lib in [&p.c, &p.rust] {
            sh.fill_pattern();
            outs.push(run(sh, lib, (OFF_OUT, 64), |l| unsafe {
                (l.unfilter)(w, h, bpp, std::ptr::null_mut())
            }));
        }
        let tag = format!("E19/unfilter(NULL) w={w},h={h},bpp={bpp}");
        diff(&tag, &outs[0], &outs[1]);
        if h > 0 {
            assert!(
                !outs[0].completed && outs[0].signal == libc::SIGSEGV,
                "[{tag}] expected SIGSEGV, got {:?}",
                outs[0]
            );
        } else {
            assert!(outs[0].completed && outs[0].ret == 1, "[{tag}] {:?}", outs[0]);
        }
    }
}

/// E21..E24 — degenerate but well-defined `unfilter` shapes.
///
/// Note that `w == 0` with `bpp > 0` does *not* return 1: `len == 0`, but the
/// row-`y>=1` pre-loop `for (x = 0; x < bpp; x++)` still writes `bpp` bytes,
/// which land on the *following rows' filter bytes* and typically turn one of
/// them into an out-of-range value. That is ground truth, so only the shapes
/// where no such write happens (`h <= 0`, or `bpp == 0`) are pinned to 1.
#[test]
fn e21_e24_unfilter_degenerate_shapes() {
    let p = pair();
    let mut rng = Rng::new(0xE210_0000_0000_0021);
    let cases: &[(i32, i32, i32)] = &[
        (8, 0, 3),    // E21: h == 0
        (8, -1, 3),   // E22: h < 0
        (8, -999, 3),
        (0, 4, 3),    // E23: w == 0, bpp > 0 (self-corrupting, differential only)
        (0, 1, 3),    // E23: w == 0 but a single row -> no pre-loop at all
        (8, 4, 0),    // E24: bpp == 0
        (0, 4, 0),
        (0, 1, 0),
    ];
    for &(w, h, bpp) in cases {
        let mut data = rng.bytes(1024);
        let len = (w * bpp).max(0) as usize;
        for y in 0..h.max(0) as usize {
            data[y * (len + 1)] = rng.below(5) as u8;
        }
        let outs = diff_unfilter(
            &p,
            &format!("E21-24/w={w},h={h},bpp={bpp}"),
            w,
            h,
            bpp,
            &data,
            PAD,
            TAIL,
        );
        assert!(outs[0].completed, "w={w},h={h},bpp={bpp}: {:?}", outs[0]);
        let no_self_corruption = h <= 1 || bpp == 0;
        if no_self_corruption {
            assert_eq!(
                outs[0].ret, 1,
                "w={w},h={h},bpp={bpp} should succeed: {:?}",
                outs[0]
            );
        }
    }
}

/// E25 / E26 — `bpp > len` and negative `bpp` (already swept in C8/C11); pinned
/// here at the exact boundaries.
#[test]
fn e25_e26_unfilter_bpp_boundaries() {
    let p = pair();
    let mut rng = Rng::new(0xE250_0000_0000_0025);
    for bpp in [-4i32, -1, 0, 1, 2, 5, 9, 33] {
        for w in [0i32, 1, 2] {
            for h in [1i32, 2, 3] {
                for f in 0u8..=5 {
                    let mut data = rng.bytes(1024);
                    let len = (w * bpp).max(0) as usize;
                    for y in 0..h as usize {
                        data[y * (len + 1)] = f;
                    }
                    diff_unfilter(
                        &p,
                        &format!("E25-26/bpp={bpp},w={w},h={h},f={f}"),
                        w,
                        h,
                        bpp,
                        &data,
                        PAD,
                        TAIL,
                    );
                }
            }
        }
    }
}

/// E28 / E29 — `cp_inflate` with `in_bytes` zero or negative (covered by E6, kept
/// as its own row for the table).
#[test]
fn e28_e29_inflate_bad_in_bytes() {
    let p = pair();
    for in_bytes in [0i32, -1, -3, -8, -1000] {
        for align in 0..4usize {
            let input = [0x03u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let tag = format!("E28-29/in_bytes={in_bytes},align={align}");
            let outs = inflate_both(&p, &tag, &input, align, in_bytes, 64);
            expect_abort(&tag, &outs[0], "s->bits_left > 0");
        }
    }
}

/// E30 / E31 / E34 — `out_bytes` zero, negative, and NULL output.
#[test]
fn e30_e31_e34_output_buffer_boundaries() {
    let p = pair();
    let sh = &p.shared;
    let toks = vec![Tok::Lit(0x41), Tok::Lit(0x42)];
    let mut e = Enc::new();
    e.fixed_block(true, &toks);
    let input = e.finish();

    // E30/E31: zero and negative out_bytes with a real buffer
    for out_bytes in [0i32, -1, -1000] {
        let tag = format!("E30-31/out_bytes={out_bytes}");
        let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, out_bytes);
        expect_reject(
            &tag,
            &outs[0],
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }

    // E34: out == NULL, out_bytes == 0 → rejected before any dereference
    let mut outs = Vec::new();
    for lib in [&p.c, &p.rust] {
        sh.fill_pattern();
        sh.write(OFF_IN, &input);
        outs.push(run(sh, lib, (OFF_OUT, 64), |l| unsafe {
            (l.cp_inflate)(
                sh.in_ptr(0) as *mut _,
                input.len() as i32,
                std::ptr::null_mut(),
                0,
            )
        }));
    }
    diff("E34/out=NULL,out_bytes=0", &outs[0], &outs[1]);
    expect_reject(
        "E34",
        &outs[0],
        "Attempted to overwrite out buffer while outputting a symbol.",
    );

    // E33: out == NULL with room "available" → NULL write
    let mut outs = Vec::new();
    for lib in [&p.c, &p.rust] {
        sh.fill_pattern();
        sh.write(OFF_IN, &input);
        outs.push(run(sh, lib, (OFF_OUT, 64), |l| unsafe {
            (l.cp_inflate)(
                sh.in_ptr(0) as *mut _,
                input.len() as i32,
                std::ptr::null_mut(),
                16,
            )
        }));
    }
    diff("E33/out=NULL,out_bytes=16", &outs[0], &outs[1]);
    assert!(
        !outs[0].completed && outs[0].signal == libc::SIGSEGV,
        "E33: expected SIGSEGV, got {:?}",
        outs[0]
    );
}

/// E32 — `cp_inflate(in = NULL, in_bytes > 0)`.
#[test]
fn e32_inflate_null_input() {
    let p = pair();
    let sh = &p.shared;
    for in_bytes in [1i32, 4, 8, 64] {
        let mut outs = Vec::new();
        for lib in [&p.c, &p.rust] {
            sh.fill_pattern();
            outs.push(run(sh, lib, (OFF_OUT, 64), |l| unsafe {
                (l.cp_inflate)(
                    std::ptr::null_mut(),
                    in_bytes,
                    sh.out_ptr(0) as *mut _,
                    64,
                )
            }));
        }
        let tag = format!("E32/in=NULL,in_bytes={in_bytes}");
        diff(&tag, &outs[0], &outs[1]);
        assert!(
            !outs[0].completed && outs[0].signal == libc::SIGSEGV,
            "[{tag}] expected SIGSEGV, got {:?}",
            outs[0]
        );
    }
}

/// E37 — an empty stored block that is the whole input: the E12 boundary
/// `bits_left / 8 == 0 <= LEN == 0` must be *accepted*.
#[test]
fn e37_empty_stored_block_accepted() {
    let p = pair();
    let mut e = Enc::new();
    e.stored_block(true, &[]);
    let input = e.finish();
    assert_eq!(input.len(), 5, "1 header byte + LEN + NLEN");
    for align in 0..4usize {
        let tag = format!("E37/align={align}");
        let outs = inflate_both(&p, &tag, &input, align, input.len() as i32, 64);
        assert!(
            outs[0].completed && outs[0].ret == 1,
            "[{tag}] empty stored block should succeed: {:?}",
            outs[0]
        );
    }
}

/// E38 — a stored block declaring exactly one byte more than is present: the
/// `bits_left / 8 <= LEN` check passes and the C over-reads its input.
#[test]
fn e38_stored_len_one_too_large() {
    let p = pair();
    let mut rng = Rng::new(0xE380_0000_0000_0038);
    for actual in [0usize, 1, 3, 8] {
        for over in [1usize, 2, 5] {
            let payload = rng.bytes(actual);
            let mut e = Enc::new();
            e.stored_block_len(true, &payload, (actual + over) as u16, None);
            let input = e.finish();
            let tag = format!("E38/actual={actual},declared={}", actual + over);
            let outs = inflate_both(&p, &tag, &input, 0, input.len() as i32, 4096);
            // both libraries over-read identically (the source bytes live in the
            // same shared mapping, so the copied garbage is the same)
            assert!(outs[0].completed, "[{tag}] {:?}", outs[0]);
            assert_eq!(outs[0].ret, 1, "[{tag}] {:?}", outs[0]);
        }
    }
}

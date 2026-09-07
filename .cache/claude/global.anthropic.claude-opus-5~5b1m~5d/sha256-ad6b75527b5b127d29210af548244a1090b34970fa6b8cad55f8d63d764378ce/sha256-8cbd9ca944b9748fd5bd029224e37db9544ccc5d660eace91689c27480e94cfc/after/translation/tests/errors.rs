//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Soft failures (`pinflate` returns 0 and sets `cp_error_reason`) are compared
//! in-process.  Hard failures (a live `assert()` → `__assert_fail` → SIGABRT)
//! are compared in re-executed child processes so that the exit signal *and*
//! the assertion message on stderr can be compared byte-for-byte.

mod common;
use common::*;

define_child_runner!();

const E_LEN_NLEN: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const E_STORED_BEYOND: &str = "Stored block extends beyond end of input stream.";
const E_OUT_SYMBOL: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const E_BACK_DIST: &str = "Attempted to write before out buffer (invalid backwards distance).";
const E_OUT_STRING: &str = "Attempted to overwrite out buffer while outputting a string.";
const E_UNKNOWN_BLOCK: &str = "Detected unknown block type within input stream.";

fn fixed_stream(toks: &[Tok]) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, toks);
    w.finish()
}

// ===========================================================================
// Row 1 — cp_stored: LEN != (uint16_t)~NLEN
// ===========================================================================

#[test]
fn err01_stored_len_nlen_mismatch() {
    let mut rng = Rng::new(0xE0_0001);
    let mut checked = 0;
    for _ in 0..300 {
        let n = rng.range(0, 64);
        let payload = rng.bytes(n);
        let len = n as u16;
        // any NLEN that is not the complement
        let mut nlen = rng.next_u32() as u16;
        if nlen == !len {
            nlen ^= 1;
        }
        let mut w = BitWriter::new();
        write_stored_block_lens(&mut w, true, len, nlen, &payload);
        let s = w.finish();
        for align in 0..4 {
            let o = diff_get("err01", &s, align, n + 8);
            assert_eq!(o.ret, 0, "err01 must fail");
            assert_eq!(reason_str(&o), E_LEN_NLEN);
            checked += 1;
        }
    }
    // the exact boundary: LEN correct but off by one bit
    for bit in 0..16u32 {
        let len = 5u16;
        let nlen = (!len) ^ (1 << bit) as u16;
        let mut w = BitWriter::new();
        write_stored_block_lens(&mut w, true, len, nlen, b"hello");
        let s = w.finish();
        let o = diff_get("err01/bit", &s, 0, 16);
        assert_eq!(o.ret, 0);
        assert_eq!(reason_str(&o), E_LEN_NLEN);
        checked += 1;
    }
    assert!(checked > 1000, "only {checked} cases");
}

// ===========================================================================
// Row 2 — cp_stored: !(s->bits_left / 8 <= (int)LEN)
// ===========================================================================

#[test]
fn err02_stored_block_beyond_input() {
    // Well-formed LEN/NLEN pair, but more input remains than LEN claims.
    let mut rng = Rng::new(0xE0_0002);
    let mut checked = 0;
    for _ in 0..200 {
        let claimed = rng.range(0, 40);
        let actual = claimed + rng.range(1, 40); // strictly more data than LEN
        let payload = rng.bytes(actual);
        let mut w = BitWriter::new();
        write_stored_block_lens(&mut w, true, claimed as u16, !(claimed as u16), &payload);
        let s = w.finish();
        for align in 0..4 {
            let o = diff_get("err02", &s, align, actual + 16);
            assert_eq!(o.ret, 0, "err02 must fail (claimed={claimed} actual={actual})");
            assert_eq!(reason_str(&o), E_STORED_BEYOND);
            checked += 1;
        }
    }
    // exactly one byte too many is the boundary; one byte fewer must succeed
    for extra in 0..4usize {
        let claimed = 10usize;
        let payload = vec![0x77u8; claimed + extra];
        let mut w = BitWriter::new();
        write_stored_block_lens(&mut w, true, claimed as u16, !(claimed as u16), &payload);
        let s = w.finish();
        let o = diff_get("err02/boundary", &s, 0, 64);
        if extra == 0 {
            assert_eq!(o.ret, 1, "extra=0 must succeed");
            assert_eq!(reason_str(&o), "<null>");
        } else {
            assert_eq!(o.ret, 0, "extra={extra} must fail");
            assert_eq!(reason_str(&o), E_STORED_BEYOND);
        }
        checked += 1;
    }
    // and the "less input than LEN" direction is *accepted* by the C
    for short in 1..5usize {
        let claimed = 10usize;
        let payload = vec![0x33u8; claimed - short];
        let mut w = BitWriter::new();
        write_stored_block_lens(&mut w, true, claimed as u16, !(claimed as u16), &payload);
        let s = w.finish();
        let o = diff_get("err02/short", &s, 0, 128);
        assert_eq!(o.ret, 1, "the C only rejects *excess* input");
        checked += 1;
    }
    assert!(checked > 800, "only {checked} cases");
}

// ===========================================================================
// Row 3 — cp_block: literal with s->out + 1 > s->out_end
// ===========================================================================

#[test]
fn err03_out_full_on_literal() {
    let mut rng = Rng::new(0xE0_0003);
    let mut checked = 0;
    for _ in 0..200 {
        let n = rng.range(1, 64);
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let s = fixed_stream(&toks);
        // one byte short of what is needed -> fails on the last literal
        for align in 0..4 {
            let o = diff_get("err03", &s, align, n - 1);
            assert_eq!(o.ret, 0, "err03 must fail");
            assert_eq!(reason_str(&o), E_OUT_SYMBOL);
            // the first n-1 bytes were written before the failure
            assert_eq!(&o.out[..n - 1], &data[..n - 1]);
            checked += 1;
        }
        // and out_bytes == 0 fails on the very first literal
        let o = diff_get("err03/zero", &s, 0, 0);
        assert_eq!(o.ret, 0);
        assert_eq!(reason_str(&o), E_OUT_SYMBOL);
        checked += 1;
    }
    assert!(checked > 800, "only {checked} cases");
}

// ===========================================================================
// Row 4 — cp_block: s->out - backwards_distance < s->begin
// ===========================================================================

#[test]
fn err04_backwards_distance_before_begin() {
    let mut rng = Rng::new(0xE0_0004);
    let mut checked = 0;
    for _ in 0..300 {
        let prefix = rng.range(0, 30);
        let data = rng.bytes(prefix);
        // distance strictly greater than what has been produced so far
        let dist = (prefix + rng.range(1, 40)) as u32;
        let length = rng.range(3, 258) as u32;
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(length, dist));
        let s = fixed_stream(&toks);
        for align in 0..4 {
            // plenty of output room, so only the distance check can fail
            let o = diff_get("err04", &s, align, prefix + 512);
            assert_eq!(
                o.ret, 0,
                "err04 must fail (prefix={prefix} dist={dist} len={length})"
            );
            assert_eq!(reason_str(&o), E_BACK_DIST);
            checked += 1;
        }
    }
    // exact boundary: distance == produced is legal, distance == produced + 1
    // is not
    for prefix in 1..8usize {
        let data: Vec<u8> = (0..prefix as u8).collect();
        for delta in 0..2usize {
            let dist = (prefix + delta) as u32;
            let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
            toks.push(Tok::Match(3, dist));
            let s = fixed_stream(&toks);
            let o = diff_get("err04/boundary", &s, 0, prefix + 64);
            if delta == 0 {
                assert_eq!(o.ret, 1, "distance == produced must be accepted");
            } else {
                assert_eq!(o.ret, 0);
                assert_eq!(reason_str(&o), E_BACK_DIST);
            }
            checked += 1;
        }
    }
    assert!(checked > 1200, "only {checked} cases");
}

#[test]
fn err04b_both_violated_reports_distance() {
    // Violate the backwards-distance check *and* the output-overrun check in
    // the same token: the C checks the distance first, so its message must win.
    for prefix in 0..4usize {
        let data: Vec<u8> = (0..prefix as u8).collect();
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(258, (prefix + 5) as u32));
        let s = fixed_stream(&toks);
        let o = diff_get("err04b", &s, 0, prefix); // no room at all for the match
        assert_eq!(o.ret, 0);
        assert_eq!(
            reason_str(&o),
            E_BACK_DIST,
            "the distance check must be reported first"
        );
    }
}

// ===========================================================================
// Row 5 — cp_block: s->out + length > s->out_end
// ===========================================================================

#[test]
fn err05_match_overruns_out() {
    let mut rng = Rng::new(0xE0_0005);
    let mut checked = 0;
    for _ in 0..300 {
        let prefix = rng.range(1, 40);
        let data = rng.bytes(prefix);
        let dist = rng.range(1, prefix) as u32; // valid distance
        let length = rng.range(3, 258) as u32;
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(length, dist));
        let s = fixed_stream(&toks);
        // room for the literals plus 0..length-1 bytes of the match
        let short_by = rng.range(1, length as usize);
        let out_len = prefix + length as usize - short_by;
        for align in 0..4 {
            let o = diff_get("err05", &s, align, out_len);
            assert_eq!(
                o.ret, 0,
                "err05 must fail (prefix={prefix} len={length} out={out_len})"
            );
            assert_eq!(reason_str(&o), E_OUT_STRING);
            assert_eq!(&o.out[..prefix], &data[..]);
            checked += 1;
        }
    }
    // boundary: exactly enough room succeeds, one byte fewer fails
    for length in [3u32, 4, 130, 258] {
        let data: Vec<u8> = (0..8u8).collect();
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(length, 8));
        let s = fixed_stream(&toks);
        let need = 8 + length as usize;
        let o = diff_get("err05/fit", &s, 0, need);
        assert_eq!(o.ret, 1, "exact fit must succeed");
        let o = diff_get("err05/short", &s, 0, need - 1);
        assert_eq!(o.ret, 0);
        assert_eq!(reason_str(&o), E_OUT_STRING);
        checked += 2;
    }
    assert!(checked > 1200, "only {checked} cases");
}

// ===========================================================================
// Row 6 — pinflate: BTYPE == 3
// ===========================================================================

#[test]
fn err06_btype_3_reserved() {
    let mut rng = Rng::new(0xE0_0006);
    let mut checked = 0;
    for bfinal in 0..2u32 {
        for _ in 0..80 {
            let mut w = BitWriter::new();
            w.bits(bfinal, 1);
            w.bits(3, 2);
            // filler so the bit reader never runs dry
            for _ in 0..rng.range(1, 8) {
                w.bits(rng.byte() as u32, 8);
            }
            let s = w.finish();
            for align in 0..4 {
                let o = diff_get("err06", &s, align, rng.range(0, 64));
                assert_eq!(o.ret, 0, "err06 must fail");
                assert_eq!(reason_str(&o), E_UNKNOWN_BLOCK);
                checked += 1;
            }
        }
    }
    // BTYPE == 3 reached as a *later* block, after a valid fixed block
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, false, &[Tok::Lit(b'a'), Tok::Lit(b'b')]);
    w.bits(1, 1);
    w.bits(3, 2);
    for _ in 0..4 {
        w.bits(0, 8);
    }
    let s = w.finish();
    let o = diff_get("err06/second-block", &s, 0, 64);
    assert_eq!(o.ret, 0);
    assert_eq!(reason_str(&o), E_UNKNOWN_BLOCK);
    assert_eq!(&o.out[..2], b"ab");
    checked += 1;
    assert!(checked > 600, "only {checked} cases");
}

// ===========================================================================
// G3 / G5 — generic argument boundaries that do *not* abort
// ===========================================================================

#[test]
fn g03_null_out_empty_block() {
    // An empty fixed block writes nothing, so `out` is never dereferenced.
    let s = fixed_stream(&[]);
    for align in 0..4 {
        let o = diff_get_raw("g03", &s, align, s.len() as i32, true, 0, 0);
        assert_eq!(o.ret, 1, "an empty block must succeed even with out == NULL");
        assert_eq!(reason_str(&o), "<null>");
    }
    // and an empty *stored* block, likewise
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, &[]);
    let s = w.finish();
    for align in 0..4 {
        let o = diff_get_raw("g03/stored", &s, align, s.len() as i32, true, 0, 0);
        assert_eq!(o.ret, 1);
    }
}

#[test]
fn g05_negative_out_bytes() {
    // out_end = out + out_bytes < out, so the very first literal fails row 3.
    let s = fixed_stream(&[Tok::Lit(b'x')]);
    for ob in [-1i32, -2, -1000, i32::MIN / 2] {
        for align in 0..4 {
            let o = diff_get_raw("g05", &s, align, s.len() as i32, false, 16, ob);
            assert_eq!(o.ret, 0, "out_bytes={ob}");
            assert_eq!(reason_str(&o), E_OUT_SYMBOL);
        }
    }
    // An empty block does not touch `out`, so a negative out_bytes succeeds.
    let s = fixed_stream(&[]);
    for ob in [-1i32, -12345] {
        let o = diff_get_raw("g05/empty", &s, 0, s.len() as i32, false, 16, ob);
        assert_eq!(o.ret, 1, "out_bytes={ob}");
    }
}

#[test]
fn g07_btype_all_four() {
    // There is no out-of-range value for BTYPE (2 bits), so simply cover all
    // four and require identical behaviour.
    let mut rng = Rng::new(0xE0_0007);
    let mut cases = Vec::new();
    for btype in 0..4u32 {
        for bfinal in 0..2u32 {
            for align in 0..4usize {
                for _ in 0..8 {
                    let mut w = BitWriter::new();
                    w.bits(bfinal, 1);
                    w.bits(btype, 2);
                    for _ in 0..rng.range(1, 12) {
                        w.bits(rng.byte() as u32, 8);
                    }
                    let s = w.finish();
                    cases.push(
                        Case::new(&format!("g07/b{btype}/f{bfinal}"), &s)
                            .align(align)
                            .out(rng.range(0, 64)),
                    );
                }
            }
        }
    }
    // may abort for btype 1/2 with random payloads, so run in children
    diff_cases_in_children("g07", &cases);
}

// ===========================================================================
// Hard failures (rows 7, 9, 10, 12, 14, 15, 16 and G1, G4)
// ===========================================================================

/// Assert that BOTH libraries abort with byte-identical stderr, and that the
/// assertion is the expected one (file, line, function and expression).
fn expect_abort(tag: &str, case: Case, want_line: u32, want_func: &str, want_expr: &str) {
    let line = diff_one_in_child(tag, case);
    assert!(
        line.contains(" SIG6 "),
        "[{tag}] expected SIGABRT, transcript says:\n  {line}"
    );
    let want = format!("lib.c:{want_line}: {want_func}: Assertion `{want_expr}' failed.");
    assert!(
        line.contains(&want),
        "[{tag}] expected assertion\n  {want}\ngot\n  {line}"
    );
}

/// Deterministically SEARCH for an input that makes the C library reach a
/// specific `assert()`.  The search runs only against C; the returned case is
/// then required to behave identically in the Rust library.  This keeps the
/// row anchored to the real C behaviour instead of a hand-derived guess.
fn find_abort(tag: &str, want_line: u32, cases: &[Case]) -> Case {
    let t = one_library_transcript("c", tag, cases);
    let pat = format!("lib.c:{want_line}:");
    for (i, l) in t.iter().enumerate() {
        if l.contains(&pat) {
            return cases[i].clone();
        }
    }
    let seen = assert_lines(&t);
    panic!("[{tag}] no candidate reached lib.c:{want_line}; lines seen: {seen:?}");
}

/// Pick, out of an existing scan transcript, the first case that reached
/// `lib.c:<want_line>`.
fn find_in_scan(want_line: u32, cases: &[Case], transcript: &[String]) -> Case {
    let pat = format!("lib.c:{want_line}:");
    for (i, l) in transcript.iter().enumerate() {
        if l.contains(&pat) {
            return cases[i].clone();
        }
    }
    panic!(
        "no case in the shared scan reached lib.c:{want_line}; lines seen: {:?}",
        assert_lines(transcript)
    );
}

/// ONE shared candidate scan against the C library, computed once per test
/// binary and reused by every row that has to *find* an input reaching a
/// particular `assert()`.  `OnceLock` makes concurrent tests wait for it.
fn shared_scan() -> &'static (Vec<Case>, Vec<String>) {
    use std::sync::OnceLock;
    static S: OnceLock<(Vec<Case>, Vec<String>)> = OnceLock::new();
    S.get_or_init(|| {
        let cases = candidate_pool(0xA9_5CA4, 4000, 14);
        let t = one_library_transcript("c", "sharedscan", &cases);
        (cases, t)
    })
}

/// Candidate pool: short pseudo-random byte strings across all alignments and
/// a spread of `out_bytes`.
fn candidate_pool(seed: u64, n: usize, maxlen: usize) -> Vec<Case> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let len = rng.range(1, maxlen);
            let data = rng.bytes(len);
            Case::new("scan", &data)
                .align(rng.below(4))
                .out(rng.range(0, 96))
        })
        .collect()
}

/// Extract the `lib.c:<line>` numbers mentioned in a transcript.
fn assert_lines(lines: &[String]) -> std::collections::BTreeSet<String> {
    let mut set = std::collections::BTreeSet::new();
    for l in lines {
        if let Some(p) = l.find("lib.c:") {
            let rest = &l[p + 6..];
            let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !n.is_empty() {
                set.insert(n);
            }
        }
    }
    set
}

// --- G1 / row 12: in_bytes == 0 -------------------------------------------

#[test]
fn abort12_zero_length_input() {
    for align in 0..4usize {
        expect_abort(
            &format!("abort12_zero_a{align}"),
            Case::new("g1/zero-in", &[]).align(align).in_bytes(0).out(16),
            125,
            "cp_read_bits",
            "s->bits_left > 0",
        );
    }
    // also with a non-empty buffer but in_bytes == 0
    expect_abort(
        "abort12_zero_nonempty",
        Case::new("g1/zero-in2", &[0xFF, 0xFF, 0xFF, 0xFF]).in_bytes(0).out(16),
        125,
        "cp_read_bits",
        "s->bits_left > 0",
    );
}

// --- G4 / row 12: negative in_bytes ---------------------------------------

#[test]
fn abort_g04_negative_in_bytes() {
    for ib in [-1i32, -3, -4, -1000] {
        expect_abort(
            &format!("abort_g04_{ib}"),
            Case::new("g4/neg-in", &[0x01, 0x00, 0x00, 0x00]).in_bytes(ib).out(16),
            125,
            "cp_read_bits",
            "s->bits_left > 0",
        );
    }
}

// --- row 12 again: the stream runs out of bits while still decoding -------

#[test]
fn abort12_bits_exhausted() {
    // A non-final fixed block that decodes cleanly and then asks for the next
    // block header with zero bits left.
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, false, &[Tok::Lit(b'a')]);
    // pad exactly to a byte boundary; nothing follows
    let s = w.finish();
    expect_abort(
        "abort12_exhausted",
        Case::new("row12/exhausted", &s).out(64),
        125,
        "cp_read_bits",
        "s->bits_left > 0",
    );
}

// --- row 9: cp_consume_bits underflow ------------------------------------

#[test]
fn abort09_consume_bits_underflow() {
    // A stored-block header truncated inside LEN/NLEN: `cp_read_bits(16)` gets
    // fewer bits than it asks for and `cp_consume_bits` asserts.
    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(0, 2);
    w.align();
    let mut s = w.finish();
    s.push(0x00); // only 8 of the 32 header bits present
    expect_abort(
        "abort09_truncated_stored",
        Case::new("row9/truncated-stored", &s).out(64),
        115,
        "cp_consume_bits",
        "s->count >= num_bits_to_read",
    );
    // and a searched-for case, to prove the row is reachable from plain input
    let (pool, t) = shared_scan();
    let case = find_in_scan(115, pool, t);
    expect_abort(
        "abort09_found",
        case,
        115,
        "cp_consume_bits",
        "s->count >= num_bits_to_read",
    );
}

// --- row 14: cp_would_overflow --------------------------------------------

#[test]
fn abort14_would_overflow() {
    let (pool, t) = shared_scan();
    let case = find_in_scan(127, pool, t);
    expect_abort(
        "abort14_found",
        case,
        127,
        "cp_read_bits",
        "!cp_would_overflow(s, num_bits_to_read)",
    );
}

// --- row 7: cp_ptr's alignment assert -------------------------------------

/// Build the family of streams that can reach `cp_ptr`'s alignment assert.
///
/// `assert(!(s->bits_left & 7))` can only fail once `s->count` and
/// `s->bits_left` stop being congruent mod 8, and the *only* place that happens
/// is `cp_peak_bits`'s final-word fold, which does `count += bits_left` instead
/// of `count += 8 * last_bytes`.  Reaching it therefore needs
///
///   1. a first block (fixed Huffman) that consumes a non-multiple of 8 bits
///      and drives the fold, then
///   2. a following STORED block, whose `LEN`/`NLEN` are then read from a
///      misaligned position and must still be complements.
///
/// With `last_bytes == 3` and `W` full words the fold happens with
/// `count == 32*W - K` and `bits_left == 8*N - K`; choosing `K = 32*W - 15`
/// makes `count == 15`, `bits_left == 39`, and after the fold `count == 54`
/// while `bits_left` stays 39 -- no longer congruent mod 8.
fn cp_ptr_candidates() -> Vec<Case> {
    let mut cases = Vec::new();
    // literal code lengths in the fixed table: 0..=143 -> 8 bits,
    // 144..=255 -> 9 bits.  The exact sequence below consumes 78 bits.
    let lit_seq: [u8; 9] = [0, 200, 1, 201, 202, 203, 204, 205, 2];
    for len_pat in [0xFFFFu32, 0xFE00, 0xFF00, 0x0000, 0xFFFE] {
        for pad in 0..16u32 {
            let mut w = BitWriter::new();
            w.bits(0, 1); // BFINAL = 0
            w.bits(1, 2); // BTYPE  = 1 (fixed)
            let lit = canonical(&pristine_tables().fixed_table[..288]);
            for &b in &lit_seq {
                let (c, l) = lit[b as usize];
                w.huff(c, l as u32);
            }
            let (c, l) = lit[256];
            w.huff(c, l as u32); // end of block  (K = 88 bits so far)
            w.bits(1, 1); // BFINAL = 1
            w.bits(0, 2); // BTYPE  = 0 (stored)
            w.bits(pad, 4); // the 4 bits cp_stored discards to "align"
            w.bits(len_pat, 16); // LEN as the C will read it
            w.bits(!len_pat & 0x1FF, 9); // low 9 bits of NLEN; top 7 read as 0
            let s = w.finish();
            assert_eq!(s.len(), 15, "the derived stream must be 15 bytes");
            for align in 0..4usize {
                for out in [0usize, 16, 64] {
                    cases.push(
                        Case::new(&format!("row7/derived/p{pad}/l{len_pat:04x}"), &s)
                            .align(align)
                            .out(out),
                    );
                }
            }
        }
    }
    // Systematic variations around the derived shape, in case the frame or the
    // fold behaves slightly differently than derived: vary how many literals
    // the first block carries and how much trailing data there is.
    let mut rng = Rng::new(0xA9_0007);
    for nlits in 0..24usize {
        for trailing in 0..12usize {
            let mut w = BitWriter::new();
            w.bits(0, 1);
            w.bits(1, 2);
            let lit = canonical(&pristine_tables().fixed_table[..288]);
            for i in 0..nlits {
                let b = if i % 2 == 0 { 5u8 } else { 200u8 };
                let (c, l) = lit[b as usize];
                w.huff(c, l as u32);
            }
            let (c, l) = lit[256];
            w.huff(c, l as u32);
            w.bits(1, 1);
            w.bits(0, 2);
            for _ in 0..trailing {
                w.bits(rng.byte() as u32, 8);
            }
            let s = w.finish();
            for align in 0..4usize {
                cases.push(
                    Case::new(&format!("row7/var/n{nlits}/t{trailing}"), &s)
                        .align(align)
                        .out(64),
                );
            }
        }
    }
    cases
}

#[test]
fn abort07_cp_ptr_misaligned() {
    // `assert(!(s->bits_left & 7))` in `cp_ptr`.
    let pool = cp_ptr_candidates();
    let case = find_abort("abort07_scan", 95, &pool);
    eprintln!("row 7 reached by: {}", case.label);
    expect_abort("abort07_found", case, 95, "cp_ptr", "!(s->bits_left & 7)");
}

// --- row 16: cp_decode's prefix-match assert ------------------------------

#[test]
fn abort16_decode_prefix_mismatch() {
    let (pool, t) = shared_scan();
    let case = find_in_scan(217, pool, t);
    expect_abort(
        "abort16_found",
        case,
        217,
        "cp_decode",
        "(search >> len) == (key >> len)",
    );
    // Also reach it deterministically: a dynamic block whose literal tree is
    // empty makes `cp_decode` binary-search an empty range and read `tree[-1]`.
    let mut cl_lens = [0u8; 19];
    for sym in [0usize, 16, 17, 18] {
        cl_lens[sym] = 2;
    }
    let (nlit, ndst) = (257usize, 1usize);
    let total = nlit + ndst;
    let mut items: Vec<Cl> = Vec::new();
    let mut emitted = 0usize;
    while emitted < total {
        let room = total - emitted;
        if room >= 138 {
            items.push(Cl::Z11(127));
            emitted += 138;
        } else if room >= 11 {
            let e = (room - 11).min(127);
            items.push(Cl::Z11(e as u8));
            emitted += 11 + e;
        } else if room >= 3 {
            let e = (room - 3).min(7);
            items.push(Cl::Z3(e as u8));
            emitted += 3 + e;
        } else {
            items.push(Cl::Len(0));
            emitted += 1;
        }
    }
    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(2, 2);
    w.bits(0, 5);
    w.bits(0, 5);
    w.bits(0, 4);
    let perm = pristine_tables().permutation_order;
    for i in 0..4 {
        w.bits(cl_lens[perm[i] as usize] as u32, 3);
    }
    let clc = canonical(&cl_lens);
    for it in &items {
        let (c, l) = clc[it.symbol()];
        w.huff(c, l as u32);
        if let Some((e, n)) = it.extra() {
            w.bits(e, n);
        }
    }
    for _ in 0..8 {
        w.bits(0xFF, 8);
    }
    let s = w.finish();
    let line = diff_one_in_child("abort16_empty_tree", Case::new("row16/empty-tree", &s).out(64));
    assert!(
        line.contains("lib.c:217") || line.contains(" OK "),
        "unexpected outcome for the empty-tree case: {line}"
    );
}

// --- row 10: num_bits_to_read <= 32 (via a poisoned exported table) -------

#[test]
fn abort10_read_bits_gt_32() {
    // `cp_dist_extra_bits` is exported writable data; a caller can set an
    // entry above 32, which makes `cp_read_bits` assert.  Distance symbol 0 is
    // used by `Tok::Match(_, 1)`.
    let data: Vec<u8> = (0..16u8).collect();
    let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    toks.push(Tok::Match(3, 1));
    let s = fixed_stream(&toks);
    for v in [33u8, 40, 64, 255] {
        expect_abort(
            &format!("abort10_v{v}"),
            Case::new("row10/dist-extra", &s).out(1024).poke_dist_extra(0, v),
            123,
            "cp_read_bits",
            "num_bits_to_read <= 32",
        );
    }
    // the same through cp_len_extra_bits (length symbol 257)
    for v in [33u8, 99] {
        expect_abort(
            &format!("abort10_len_v{v}"),
            Case::new("row10/len-extra", &s).out(1024).poke_len_extra(0, v),
            123,
            "cp_read_bits",
            "num_bits_to_read <= 32",
        );
    }
}

// --- row 15: cp_build's len < 16 (via a poisoned cp_fixed_table) ----------

#[test]
fn abort15_build_len_ge_16() {
    let s = fixed_stream(&[Tok::Lit(b'a')]);
    for (idx, v) in [(0usize, 16u8), (5, 20), (287, 16), (100, 255)] {
        expect_abort(
            &format!("abort15_{idx}_{v}"),
            Case::new("row15/fixed-table", &s).out(64).poke_fixed(idx, v),
            154,
            "cp_build",
            "len < 16",
        );
    }
    // an entry in the *distance* half of the table (built with s == NULL)
    for idx in [288usize, 300, 319] {
        expect_abort(
            &format!("abort15_dst_{idx}"),
            Case::new("row15/fixed-table-dst", &s).out(64).poke_fixed(idx, 16),
            154,
            "cp_build",
            "len < 16",
        );
    }
}

// --- rows 8, 11, 13: documented as structurally unreachable ---------------

#[test]
fn unreachable_asserts_are_never_hit() {
    // ERRORS.md rows 8, 11 and 13 claim lib.c:104, :124 and :126 are
    // structurally unreachable.  Sweep a broad deterministic set of short
    // inputs, collect every assertion the C library reaches (each case runs in
    // its own forked grandchild, so aborts and hangs are recorded rather than
    // fatal) and require those three lines never to appear.  The same
    // transcript is required from the Rust library, case for case.
    let mut rng = Rng::new(0xE0_0F00);
    let mut cases = Vec::new();
    for _ in 0..1200 {
        let n = rng.range(1, 12);
        let data = rng.bytes(n);
        cases.push(
            Case::new("unreachable/probe", &data)
                .align(rng.below(4))
                .out(rng.range(0, 64)),
        );
    }
    // fold in the shared scan so the claim is checked over a wider set
    let (pool, ctrans) = shared_scan();
    let lines = diff_cases_in_children("unreachable", &cases);
    let lines: Vec<String> = lines.into_iter().chain(ctrans.iter().cloned()).collect();
    let _ = pool;
    let seen = assert_lines(&lines);
    for forbidden in ["104", "124", "126"] {
        assert!(
            !seen.contains(forbidden),
            "ERRORS.md claims lib.c:{forbidden} is unreachable, but it fired"
        );
    }
    let aborts = lines.iter().filter(|l| l.contains(" SIG6 ")).count();
    let hangs = lines.iter().filter(|l| l.contains("HANG")).count();
    let oks = lines.iter().filter(|l| l.contains(" OK ")).count();
    eprintln!(
        "sweep: {oks} returned, {aborts} aborted, {hangs} hung; assertion lines reached: {seen:?}"
    );
    assert!(!seen.is_empty(), "the sweep should reach *some* assertion");
}

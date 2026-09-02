//! Phase C — error-path differential tests.
//!
//! One test per `ERRORS.md` row. Rows E1..E8 are rejections the C actually
//! *returns*, so they are compared directly (return value + `cp_error_reason`
//! string + output buffer). Rows A1..A10 are `assert()` sites: the documented
//! CMake build has live asserts, so those are run in a forked child, the
//! as-built C is asserted to `SIGABRT`, and the Rust is compared against the
//! `-DNDEBUG` C build, which is bit-identical to the as-built C on every input
//! where the asserts hold.

mod common;

use common::deflate::*;
use common::*;

const AB: CBuild = CBuild::AsBuilt;

const E_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const E_STORED_LONG: &[u8] = b"Stored block extends beyond end of input stream.";
const E_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.";
const E_DISTANCE: &[u8] = b"Attempted to write before out buffer (invalid backwards distance).";
const E_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.";
const E_BTYPE: &[u8] = b"Detected unknown block type within input stream.";

fn expect_err(out: &InflateOutcome, msg: &[u8], label: &str) {
    assert_eq!(out.ret, 0, "{label}: expected rejection, got ret={}", out.ret);
    assert_eq!(
        out.err.as_deref().map(String::from_utf8_lossy),
        Some(String::from_utf8_lossy(msg)),
        "{label}: wrong cp_error_reason"
    );
}

// ===========================================================================
// E1 — stored block: LEN is not ~NLEN
// ===========================================================================

#[test]
fn e1_stored_len_nlen_mismatch() {
    let mut rng = Rng::new(0xE101);
    for align in 0..4 {
        for _ in 0..60 {
            let len = rng.range(0, 512);
            let data = rng.bytes(len);
            // Any NLEN other than !LEN must be rejected.
            let good = !(len as u16);
            let mut bad = rng.next_u32() as u16;
            if bad == good {
                bad ^= 1;
            }
            let mut d = Deflate::new();
            d.stored_bad_nlen(true, &data, bad);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, len + 64).in_align(align),
                AB,
                "E1",
            );
            expect_err(&out, E_NLEN, "E1");
            assert!(
                out.out.iter().all(|&b| b == 0),
                "E1: nothing may be copied on rejection"
            );
        }
    }
    // The exact boundary: NLEN off by one in either direction.
    for delta in [1u16, 0xFFFF] {
        let data = [1u8, 2, 3, 4];
        let mut d = Deflate::new();
        d.stored_bad_nlen(true, &data, (!(data.len() as u16)).wrapping_add(delta));
        let stream = d.finish();
        let out = diff_inflate(InflateCase::new(&stream, 64), AB, "E1-boundary");
        expect_err(&out, E_NLEN, "E1-boundary");
    }
}

// ===========================================================================
// E2 — stored block: bits_left/8 > LEN
// ===========================================================================

#[test]
fn e2_stored_extends_beyond_input() {
    let mut rng = Rng::new(0xE201);
    // (a) a stored block followed by more data
    for align in 0..4 {
        for _ in 0..40 {
            let len = rng.range(0, 256);
            let data = rng.bytes(len);
            let mut d = Deflate::new();
            d.stored(false, &data); // not final => another block follows
            let toks = rand_literals_n(&mut rng, 20);
            d.fixed(true, &toks);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, len + 128).in_align(align),
                AB,
                "E2a",
            );
            expect_err(&out, E_STORED_LONG, "E2a");
        }
    }
    // (b) a single stored block whose declared LEN is smaller than the tail
    for align in 0..4 {
        for _ in 0..40 {
            let real = rng.range(9, 256);
            let data = rng.bytes(real);
            // declared_len must satisfy declared < bits_left/8 == real
            let declared = rng.range(0, real - 1) as u16;
            let mut d = Deflate::new();
            d.stored_len_override(true, &data, declared);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, real + 128).in_align(align),
                AB,
                "E2b",
            );
            expect_err(&out, E_STORED_LONG, "E2b");
        }
    }
    // (c) the exact boundary: declared == real - 1 rejects, declared == real
    //     is accepted (already covered by Phase B C1..C5).
    let data: Vec<u8> = (0..40u8).collect();
    let mut d = Deflate::new();
    d.stored_len_override(true, &data, (data.len() - 1) as u16);
    let stream = d.finish();
    let out = diff_inflate(InflateCase::new(&stream, 256), AB, "E2c");
    expect_err(&out, E_STORED_LONG, "E2c");
}

fn rand_literals_n(rng: &mut Rng, n: usize) -> Vec<Tok> {
    rand_literals(rng, n)
}

// ===========================================================================
// E3 — literal with a full output buffer
// ===========================================================================

#[test]
fn e3_literal_overflows_out() {
    let mut rng = Rng::new(0xE301);
    // out_bytes == 0: the very first literal is rejected.
    for align in 0..4 {
        for _ in 0..30 {
            let toks = { let n = rng.range(1, 40); rand_literals_n(&mut rng, n) };
            let mut d = Deflate::new();
            d.fixed(true, &toks);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, 64).in_align(align).out_bytes(0),
                AB,
                "E3-zero",
            );
            expect_err(&out, E_SYMBOL, "E3-zero");
            assert!(out.out.iter().all(|&b| b == 0), "E3: wrote into a zero-sized out");
        }
    }
    // out_bytes == k with k+1 literals: rejected exactly on literal k+1, with
    // the first k bytes already written.
    for _ in 0..80 {
        let k = rng.range(1, 200);
        let toks = rand_literals_n(&mut rng, k + 1);
        let expected = expand(&toks);
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, k + 64).out_bytes(k as i32),
            AB,
            "E3-partial",
        );
        expect_err(&out, E_SYMBOL, "E3-partial");
        assert_eq!(&out.out[..k], &expected[..k], "E3: partial output");
        assert!(out.out[k..].iter().all(|&b| b == 0), "E3: wrote past out_bytes");
    }
    // Negative out_bytes: out_end < begin, so even the first literal fails.
    for &ob in &[-1i32, -64, i32::MIN / 4] {
        let toks = rand_literals_n(&mut rng, 4);
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, 64).out_bytes(ob),
            AB,
            "E3-negative",
        );
        expect_err(&out, E_SYMBOL, "E3-negative");
    }
    // NULL out with out_bytes 0: rejected without ever dereferencing.
    let toks = rand_literals_n(&mut rng, 4);
    let mut d = Deflate::new();
    d.fixed(true, &toks);
    let stream = d.finish();
    let l = libs();
    let a = inflate_in_child(&l.c, &stream, 0, stream.len() as i32, 0, 0, 0, false, true);
    let b = inflate_in_child(&l.r, &stream, 0, stream.len() as i32, 0, 0, 0, false, true);
    assert_child_match(&a, &b, "E3-null-out");
    assert_eq!(a.status, ChildStatus::Exited(0), "E3-null-out should not crash");
    assert_eq!(a.ret, 0);
    assert_eq!(a.err.as_deref(), Some(E_SYMBOL));
}

// ===========================================================================
// E4 — backwards distance reaches before the start of out
// ===========================================================================

#[test]
fn e4_backwards_distance_before_begin() {
    let mut rng = Rng::new(0xE401);
    for _ in 0..150 {
        // Emit `produced` literals, then a match whose distance exceeds them.
        let produced = rng.range(0, 40);
        let mut toks: Vec<Tok> = (0..produced).map(|_| Tok::Lit(rng.byte())).collect();
        let dist = produced as u32 + rng.range(1, 64) as u32;
        let (dc, dextra) = dist_code(dist);
        toks.push(Tok::MatchRaw { lc: 0, lextra: 0, dc, dextra });
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, produced + 512).out_bytes((produced + 512) as i32),
            AB,
            "E4",
        );
        expect_err(&out, E_DISTANCE, "E4");
    }
    // The exact boundary: distance == produced is legal, produced + 1 is not.
    for produced in 1..=40usize {
        for (dist, should_fail) in [(produced as u32, false), (produced as u32 + 1, true)] {
            let mut toks: Vec<Tok> = (0..produced).map(|i| Tok::Lit(i as u8)).collect();
            let (dc, dextra) = dist_code(dist);
            toks.push(Tok::MatchRaw { lc: 0, lextra: 0, dc, dextra });
            let mut d = Deflate::new();
            d.fixed(true, &toks);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, produced + 512).out_bytes((produced + 512) as i32),
                AB,
                "E4-boundary",
            );
            if should_fail {
                expect_err(&out, E_DISTANCE, "E4-boundary");
            } else {
                assert_eq!(out.ret, 1, "E4-boundary: dist == produced must be accepted");
            }
        }
    }
}

// ===========================================================================
// E5 — match copy runs past the end of out
// ===========================================================================

#[test]
fn e5_string_overflows_out() {
    let mut rng = Rng::new(0xE501);
    for _ in 0..150 {
        let produced = rng.range(4, 60);
        let mut toks: Vec<Tok> = (0..produced).map(|_| Tok::Lit(rng.byte())).collect();
        let length = rng.range(3, 258);
        let dist = rng.range(1, produced) as u32;
        toks.push(Tok::Match(length as u32, dist));
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        // out_bytes leaves room for the literals but not the whole match.
        let ob = produced + rng.range(0, length - 1);
        let out = diff_inflate(
            InflateCase::new(&stream, produced + length + 64).out_bytes(ob as i32),
            AB,
            "E5",
        );
        expect_err(&out, E_STRING, "E5");
        // The literals are already in place; nothing past out_bytes was touched.
        let expected = expand(&toks);
        assert_eq!(&out.out[..produced], &expected[..produced], "E5: literals");
        assert!(
            out.out[ob..].iter().all(|&b| b == 0),
            "E5: wrote past out_bytes"
        );
    }
    // Boundary: out_bytes == produced + length succeeds, one less fails.
    for produced in [4usize, 17, 40] {
        for length in [3usize, 4, 100, 258] {
            let mut toks: Vec<Tok> = (0..produced).map(|i| Tok::Lit(i as u8)).collect();
            toks.push(Tok::Match(length as u32, 1));
            let mut d = Deflate::new();
            d.fixed(true, &toks);
            let stream = d.finish();
            for (ob, should_fail) in [
                (produced + length, false),
                (produced + length - 1, true),
            ] {
                let out = diff_inflate(
                    InflateCase::new(&stream, produced + length + 64).out_bytes(ob as i32),
                    AB,
                    "E5-boundary",
                );
                if should_fail {
                    expect_err(&out, E_STRING, "E5-boundary");
                } else {
                    assert_eq!(out.ret, 1, "E5-boundary: exact fit must be accepted");
                }
            }
        }
    }
    // A zero-length match (length symbols 29/30) can never trip E5, but its
    // distance check still applies -- confirm it lands on E4, not E5.
    for lc in [29usize, 30] {
        let toks = vec![Tok::MatchRaw { lc, lextra: 0, dc: 0, dextra: 0 }];
        let mut d = Deflate::new();
        d.fixed(true, &toks);
        let stream = d.finish();
        let out = diff_inflate(InflateCase::new(&stream, 128), AB, "E5-zerolen");
        expect_err(&out, E_DISTANCE, "E5-zerolen");
    }
}

// ===========================================================================
// E6 — btype == 3
// ===========================================================================

#[test]
fn e6_reserved_block_type() {
    for align in 0..4 {
        for bfinal in [false, true] {
            let mut d = Deflate::new();
            d.bad_btype(bfinal);
            d.w.raw_pad(16);
            let stream = d.finish();
            let out = diff_inflate(
                InflateCase::new(&stream, 128).in_align(align),
                AB,
                "E6",
            );
            expect_err(&out, E_BTYPE, "E6");
        }
    }
    // btype 3 reached after a valid block, so the loop has already iterated.
    let mut rng = Rng::new(0xE601);
    for _ in 0..40 {
        let toks = { let n = rng.range(1, 60); rand_literals_n(&mut rng, n) };
        let expected = expand(&toks);
        let mut d = Deflate::new();
        d.fixed(false, &toks);
        d.bad_btype(false);
        d.w.raw_pad(16);
        let stream = d.finish();
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 128),
            AB,
            "E6-second-block",
        );
        expect_err(&out, E_BTYPE, "E6-second-block");
        assert_eq!(&out.out[..expected.len()], &expected[..], "E6: first block output");
    }
}

// ===========================================================================
// E7 — unfilter: row-0 filter byte >= 5
// ===========================================================================

#[test]
fn e7_unfilter_bad_row0_filter() {
    let mut rng = Rng::new(0xE701);
    // Exhaustive over all 251 invalid filter values.
    for f in 5u16..=255 {
        for &bpp in &[1i32, 3, 4] {
            let w = 7i32;
            let stride = (1 + w * bpp) as usize;
            let mut raw = vec![0u8; stride * 3 + 64];
            for b in raw.iter_mut() {
                *b = rng.byte();
            }
            raw[0] = f as u8;
            let r = diff_unfilter(w, 3, bpp, &raw, AB, "E7");
            assert_eq!(r.ret, 0, "E7: filter={f} bpp={bpp} must be rejected");
            assert_eq!(r.raw, raw, "E7: raw must be untouched on row-0 rejection");
        }
    }
    // h == 1 as well as h > 1, and the accepted boundary value 4.
    for &h in &[1i32, 2, 5] {
        let w = 4i32;
        let bpp = 2i32;
        let stride = (1 + w * bpp) as usize;
        let mut raw = vec![0u8; stride * h as usize + 64];
        for b in raw.iter_mut() {
            *b = rng.byte();
        }
        for y in 0..h as usize {
            raw[y * stride] = 0;
        }
        raw[0] = 5;
        let r = diff_unfilter(w, h, bpp, &raw, AB, "E7-h");
        assert_eq!(r.ret, 0, "E7: h={h}");
        raw[0] = 4;
        let r = diff_unfilter(w, h, bpp, &raw, AB, "E7-h-ok");
        assert_eq!(r.ret, 1, "E7: filter 4 must be accepted (h={h})");
    }
}

// ===========================================================================
// E8 — unfilter: a later row's filter byte >= 5
// ===========================================================================

#[test]
fn e8_unfilter_bad_row_filter() {
    let mut rng = Rng::new(0xE801);
    let mut mutated_prefixes = 0usize;
    for f in 5u16..=255 {
        let (w, h, bpp) = (6i32, 4i32, 3i32);
        let stride = (1 + w * bpp) as usize;
        let mut raw = vec![0u8; stride * h as usize + 64];
        for b in raw.iter_mut() {
            *b = rng.byte();
        }
        let bad_row = (f as usize % 3) + 1;
        for y in 0..h as usize {
            // Filter 1 (Sub) always rewrites the row, so the prefix really is
            // mutated before the rejection.
            raw[y * stride] = 1;
        }
        raw[bad_row * stride] = f as u8;
        let r = diff_unfilter(w, h, bpp, &raw, AB, "E8");
        assert_eq!(r.ret, 0, "E8: filter={f} row={bad_row} must be rejected");
        // Rows before the bad one are already unfiltered in place; the bytes
        // from the bad row onwards are untouched. diff_unfilter has already
        // asserted the two libraries agree byte-for-byte, including the
        // partial mutation.
        assert_eq!(
            &r.raw[bad_row * stride..],
            &raw[bad_row * stride..],
            "E8: bytes from the rejected row onwards must be untouched"
        );
        if r.raw[..bad_row * stride] != raw[..bad_row * stride] {
            mutated_prefixes += 1;
        }
    }
    // Reject on the very last row, and on every row index in turn.
    for bad_row in 1..6usize {
        let (w, h, bpp) = (5i32, 6i32, 4i32);
        let stride = (1 + w * bpp) as usize;
        let mut raw = vec![0u8; stride * h as usize + 64];
        for b in raw.iter_mut() {
            *b = rng.byte();
        }
        for y in 0..h as usize {
            raw[y * stride] = (y % 5) as u8;
        }
        raw[bad_row * stride] = 200;
        let r = diff_unfilter(w, h, bpp, &raw, AB, "E8-sweep");
        assert_eq!(r.ret, 0, "E8-sweep: row={bad_row}");
    }
    assert!(
        mutated_prefixes > 200,
        "E8: partial in-place mutation before the rejection was never observed ({mutated_prefixes})"
    );
}

// ===========================================================================
// A1 / A3 / A6 / A8 / A10 — assert sites (fork + NDEBUG comparison)
// ===========================================================================

/// Runs one input through (a) the as-built C, (b) the `-DNDEBUG` C, and
/// (c) the Rust, all in forked children. Asserts the Rust matches the NDEBUG C
/// exactly, and reports what the as-built C did.
fn assert_row(
    label: &str,
    stream: &[u8],
    in_bytes: Option<i32>,
    out_len: usize,
    out_bytes: Option<i32>,
) -> (ChildStatus, ChildStatus) {
    let _g = call_lock();
    let l = libs();
    let ib = in_bytes.unwrap_or(stream.len() as i32);
    let ob = out_bytes.unwrap_or(out_len as i32);
    let a = inflate_in_child(&l.c, stream, 0, ib, out_len, 0, ob, false, false);
    let n = inflate_in_child(&l.c_nd, stream, 0, ib, out_len, 0, ob, false, false);
    let r = inflate_in_child(&l.r, stream, 0, ib, out_len, 0, ob, false, false);
    // A child still running after the timeout is stuck in the C's
    // `while (!bfinal)` loop reading stale bits past the end of the input; that
    // is itself behaviour the Rust must reproduce, so Signaled(SIGALRM) is a
    // comparable outcome rather than a failure. The only divergence tolerated is
    // the cp_dynamic lens[] overrun, and only when the instrumented probe
    // confirms it (see fuzz_differential.rs for the mechanism).
    drop(_g);
    let v = compare_or_ub(&n, &r, stream, out_len, ob, label);
    assert_ne!(
        v,
        DiffVerdict::ProbeInconclusive,
        "{label}: divergence the UB probe could not classify"
    );
    (a.status, n.status)
}

#[test]
fn a6_a8_truncated_stream_bits_left_exhausted() {
    // Truncating a valid stream makes cp_read_bits run past the end of the
    // input: assert(s->bits_left > 0) (A6) and
    // assert(!cp_would_overflow(...)) (A8).
    let mut rng = Rng::new(0xA601);
    let mut aborts = 0usize;
    let mut cases = 0usize;
    for _ in 0..60 {
        let toks = { let n = rng.range(20, 120); rand_literals(&mut rng, n) };
        let mut d = Deflate::new();
        // No end-of-block symbol => the decoder keeps reading past the input.
        d.fixed_no_eob(true, &toks);
        let full = d.finish();
        for cut in [full.len(), full.len().saturating_sub(1), full.len() / 2, 1] {
            if cut == 0 {
                continue;
            }
            let stream = &full[..cut];
            let (as_built, _) = assert_row("A6/A8", stream, None, 4096, None);
            cases += 1;
            if as_built.is_abort() {
                aborts += 1;
            }
        }
    }
    assert!(cases > 100, "A6/A8: too few cases ({cases})");
    assert!(
        aborts > 0,
        "A6/A8: the as-built C never aborted, so the assert was never reached"
    );
}

#[test]
fn a6_in_bytes_zero_and_negative() {
    // in_bytes == 0 => bits_left == 0 => A6 on the very first read.
    // in_bytes < 0 => bits_left < 0, word_count < 0, last_bytes = neg & 3.
    let data = [0x63u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    for &ib in &[0i32, -1, -3, -4, -17, -4096] {
        let (as_built, ndebug) = assert_row("A6-in_bytes", &data, Some(ib), 4096, None);
        assert!(
            as_built.is_abort() || as_built == ChildStatus::Exited(0),
            "A6-in_bytes={ib}: unexpected as-built status {as_built:?}"
        );
        let _ = ndebug;
    }
}

#[test]
fn a1_a3_stored_unaligned_and_short_buffer() {
    // A stored block whose header cannot be fully read leaves bits_left
    // non-multiple-of-8 at cp_ptr (A1) and count < num_bits at cp_consume_bits
    // (A3).
    let mut aborts = 0usize;
    for len in 0..=6usize {
        let data: Vec<u8> = (0..len as u8).collect();
        let mut d = Deflate::new();
        d.stored(true, &data);
        let full = d.finish();
        for cut in 1..=full.len() {
            let (as_built, _) = assert_row("A1/A3", &full[..cut], None, 4096, None);
            if as_built.is_abort() {
                aborts += 1;
            }
        }
    }
    assert!(aborts > 0, "A1/A3: never reached an assert in the as-built C");
}

#[test]
fn a10_empty_and_mismatched_huffman_tree() {
    // An all-zero literal length set makes cp_build return 0, so cp_decode's
    // binary search leaves lo == 0 and it reads tree[-1] -- the struct field
    // preceding `lit`. assert((search >> len) == (key >> len)) then fails (A10).
    let mut aborts = 0usize;
    let mut cases = 0usize;

    // (a) HCLEN = 4: only permutation slots {16, 17, 18, 0} carry lengths, so
    //     every literal length decodes to 0 and the literal tree is empty. This
    //     is also the only way to reach nlen == 4 (see CONFIGS.md C15).
    {
        let mut w = BitWriter::new();
        w.bits(1, 1); // bfinal
        w.bits(2, 2); // btype = dynamic
        w.bits(0, 5); // HLIT  => nlit = 257
        w.bits(0, 5); // HDIST => ndst = 1
        w.bits(0, 4); // HCLEN => nlen = 4
        // lenlens[16] = 1, lenlens[17] = 1, lenlens[18] = 0, lenlens[0] = 0
        w.bits(1, 3);
        w.bits(1, 3);
        w.bits(0, 3);
        w.bits(0, 3);
        // Symbol 17 (a 1-bit code) repeated: writes zero runs for all 258 slots.
        for _ in 0..64 {
            w.bits(1, 1); // the code for symbol 17
            w.bits(7, 3); // repeat 3 + 7 = 10 zeros
        }
        for _ in 0..64 {
            w.bits(0, 8);
        }
        let stream = w.finish();
        let (as_built, _) = assert_row("A10-empty-lit", &stream, None, 4096, None);
        cases += 1;
        if as_built.is_abort() {
            aborts += 1;
        }
    }

    // (b) An incomplete literal tree: the peeked bits are not a prefix of any
    //     code, so the binary search lands on a key that does not match.
    {
        let mut lit_lens = vec![0u8; 257];
        lit_lens[b'A' as usize] = 3; // Kraft sum = 1/8 + 1/8 = 1/4 -- incomplete
        lit_lens[256] = 3;
        let mut d = Deflate::new();
        // dynamic() would assert on an incomplete tree, so drive the header by
        // hand through dynamic_rle, which does not check completeness.
        d.dynamic_rle(
            true,
            &[Tok::Lit(b'A'), Tok::Lit(b'A')],
            &lit_lens,
            &[1u8, 1u8],
            4,
            false,
            true,
            true,
        );
        let stream = d.finish();
        let (as_built, _) = assert_row("A10-incomplete-lit", &stream, None, 4096, None);
        cases += 1;
        if as_built.is_abort() {
            aborts += 1;
        }
    }

    // (c) Random garbage: the fastest way to reach mismatched keys and every
    //     other assert in the file.
    let mut rng = Rng::new(0xA1001);
    for _ in 0..250 {
        let n = rng.range(1, 64);
        let stream = rng.bytes(n);
        let (as_built, _) = assert_row("A10-garbage", &stream, None, 4096, None);
        cases += 1;
        if as_built.is_abort() {
            aborts += 1;
        }
    }

    assert!(cases > 200, "A10: too few cases ({cases})");
    assert!(aborts > 0, "A10: the as-built C never aborted");
}

#[test]
fn a9_corrupted_fixed_table_code_length() {
    // cp_fixed_table is exported and non-static, so a caller can put a code
    // length >= 16 in it. cp_build then trips assert(len < 16). The same store
    // also makes `counts[lens[n]]++` write one past a 16-int stack array, whose
    // layout is compiler- and language-specific, so only the assert itself is
    // compared -- the post-OOB behaviour of the NDEBUG build is not a defined
    // contract. This is recorded as such in ERRORS.md.
    let _g = call_lock();
    let l = libs();
    let toks = vec![Tok::Lit(b'x'), Tok::Lit(b'y')];
    let mut d = Deflate::new();
    d.fixed(true, &toks);
    let stream = d.finish();

    for &bad in &[16u8, 17, 31, 255] {
        let old_c = poke_table(&l.c, b"cp_fixed_table", 0, bad);
        let old_r = poke_table(&l.r, b"cp_fixed_table", 0, bad);
        assert_eq!(old_c, old_r, "A9: cp_fixed_table[0] differed before the poke");

        let a = inflate_in_child(&l.c, &stream, 0, stream.len() as i32, 4096, 0, 4096, false, false);
        let r = inflate_in_child(&l.r, &stream, 0, stream.len() as i32, 4096, 0, 4096, false, false);

        poke_table(&l.c, b"cp_fixed_table", 0, old_c);
        poke_table(&l.r, b"cp_fixed_table", 0, old_r);

        assert!(
            a.status.is_abort(),
            "A9: as-built C should abort on code length {bad}, got {:?}",
            a.status
        );
        assert!(
            !r.status.is_timeout(),
            "A9: Rust child hung on code length {bad}"
        );
    }

    // Verify the tables were restored so later tests are unaffected.
    assert_eq!(
        l.c.table(b"cp_fixed_table", 320),
        l.r.table(b"cp_fixed_table", 320),
        "A9: cp_fixed_table not restored identically"
    );
    assert_eq!(l.c.table(b"cp_fixed_table", 1)[0], 8, "A9: restore failed");
}

#[test]
fn a2_a4_a5_a7_unreachable_asserts() {
    // These four asserts cannot fire for any input:
    //   A2  cp_peak_bits:  the branch is guarded by word_index < word_count.
    //   A4  cp_read_bits:  every call site passes <= 16 bits (literals 1,2,3,4,
    //                      5,7,16, `count & 7` <= 7, or a table entry <= 13).
    //   A5  cp_read_bits:  the same call sites are all non-negative.
    //   A7  cp_read_bits:  peak only tops up when count < n <= 32, so count
    //                      never exceeds 63 before the += 32.
    // Verified statically against the source; asserted here as documentation so
    // the row is not silently dropped from ERRORS.md.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("c_src")
            .join("src")
            .join("lib.c"),
    )
    .expect("c_src/src/lib.c must be readable");

    // Every literal bit count passed to cp_read_bits.
    let mut widths: Vec<String> = Vec::new();
    const PAT: &str = "cp_read_bits(s, ";
    for (at, _) in src.match_indices(PAT) {
        let arg: String = src[at + PAT.len()..]
            .chars()
            .take_while(|&c| c != ')')
            .collect();
        widths.push(arg);
    }
    assert!(!widths.is_empty(), "no cp_read_bits call sites found");
    for w in &widths {
        let ok = match w.parse::<i32>() {
            Ok(n) => (0..=32).contains(&n),
            Err(_) => {
                // Non-literal arguments: `s->count & 7` (<= 7) and the two
                // extra-bit tables (max 5 and 13 respectively).
                w == "s->count & 7"
                    || w.starts_with("cp_len_extra_bits[")
                    || w.starts_with("cp_dist_extra_bits[")
            }
        };
        assert!(ok, "A4/A5: unexpected cp_read_bits width `{w}`");
    }
    assert_eq!(
        *common::deflate::LEN_EXTRA_FULL.iter().max().unwrap(),
        5,
        "A4: cp_len_extra_bits max changed"
    );
    assert_eq!(
        *common::deflate::DIST_EXTRA_FULL.iter().max().unwrap(),
        13,
        "A4: cp_dist_extra_bits max changed"
    );
    assert!(
        src.contains("if (s->word_index < s->word_count) {"),
        "A2: cp_peak_bits guard changed"
    );
}

// ===========================================================================
// Generic FFI boundary cases
// ===========================================================================

#[test]
fn boundary_null_pointers() {
    let _g = call_lock();
    let l = libs();

    // unfilter with h <= 0 never dereferences raw, so NULL is fine.
    for &h in &[0i32, -1, -100] {
        let a = unfilter_in_child(&l.c, 8, h, 4, &[], true);
        let r = unfilter_in_child(&l.r, 8, h, 4, &[], true);
        assert_child_match(&a, &r, "null-raw-h<=0");
        assert_eq!(a.status, ChildStatus::Exited(0), "h={h} should not crash");
        assert_eq!(a.ret, 1, "h={h} should return 1");
    }

    // unfilter with h > 0 reads *raw immediately => SIGSEGV on both sides.
    for &h in &[1i32, 2, 10] {
        let a = unfilter_in_child(&l.c, 8, h, 4, &[], true);
        let r = unfilter_in_child(&l.r, 8, h, 4, &[], true);
        assert_child_match(&a, &r, "null-raw-h>0");
        assert!(a.status.is_segv(), "h={h}: expected SIGSEGV, got {:?}", a.status);
    }

    // cp_inflate with a NULL input and a positive in_bytes dereferences the
    // word array => SIGSEGV on both sides.
    for &ib in &[1i32, 4, 8, 64] {
        let a = inflate_in_child(&l.c, &[], 0, ib, 4096, 0, 4096, true, false);
        let r = inflate_in_child(&l.r, &[], 0, ib, 4096, 0, 4096, true, false);
        assert_child_match(&a, &r, "null-in");
        assert!(a.status.crashed(), "in=NULL in_bytes={ib}: expected a crash");
    }

    // cp_inflate with both pointers NULL and both sizes 0. in_bytes == 0 means
    // bits_left == 0, which trips assert(s->bits_left > 0) (ERRORS.md A6), so
    // the as-built C aborts while the NDEBUG C -- and the Rust -- return a
    // rejection. Compare the Rust against the NDEBUG build and record the
    // as-built abort separately.
    let a = inflate_in_child(&l.c, &[], 0, 0, 0, 0, 0, true, true);
    let n = inflate_in_child(&l.c_nd, &[], 0, 0, 0, 0, 0, true, true);
    let r = inflate_in_child(&l.r, &[], 0, 0, 0, 0, 0, true, true);
    assert_child_match(&n, &r, "null-both");
    assert!(
        a.status.is_abort(),
        "null-both: as-built C should abort on bits_left == 0, got {:?}",
        a.status
    );
}

#[test]
fn boundary_oversized_and_negative_lengths() {
    let mut rng = Rng::new(0xB001);
    let toks = rand_literals(&mut rng, 32);
    let expected = expand(&toks);
    let mut d = Deflate::new();
    d.fixed(true, &toks);
    let stream = d.finish();

    // in_bytes larger than the real buffer: the decoder still stops at the
    // end-of-block symbol, so this must succeed identically.
    for extra in [1i32, 4, 32] {
        let out = diff_inflate(
            InflateCase::new(&stream, expected.len() + 64)
                .in_bytes(stream.len() as i32 + extra),
            AB,
            "oversized-in_bytes",
        );
        assert_eq!(out.ret, 1, "oversized in_bytes: err={:?}", out.err);
        assert_eq!(&out.out[..expected.len()], &expected[..]);
    }

    // in_bytes one below the real length: may or may not reach an assert, but
    // the two libraries must agree.
    for cut in 1..stream.len() {
        assert_row("short-in_bytes", &stream, Some(cut as i32), 4096, None);
    }

    // Extreme out_bytes values.
    for &ob in &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1, 0] {
        let l = libs();
        let _g = call_lock();
        let a = inflate_in_child(&l.c, &stream, 0, stream.len() as i32, 4096, 0, ob, false, false);
        let r = inflate_in_child(&l.r, &stream, 0, stream.len() as i32, 4096, 0, ob, false, false);
        assert_child_match(&a, &r, "extreme-out_bytes");
    }
}

#[test]
fn boundary_unfilter_out_of_range_filter_tag_and_extreme_dims() {
    // The C API has no enum parameters; the closest analogue is `unfilter`'s
    // per-row filter tag, which is read straight out of the data and is only
    // valid for 0..=4. All 251 out-of-range values are covered exhaustively by
    // E7/E8. Here: extreme w/h/bpp values passed across the FFI boundary.
    let mut rng = Rng::new(0xB101);
    let l = libs();

    // h == 1 with a huge w: len overflows into a large positive or negative
    // int. Run in a child because the C walks off the buffer.
    for &(w, bpp) in &[
        (i32::MAX, 1i32),
        (i32::MAX, 4),
        (1 << 20, 8),
        (-1, 4),
        (-1024, 3),
        (1, i32::MAX),
        (1, -1),
        (-1, -1),
        (i32::MIN, 1),
        (1, i32::MIN),
    ] {
        for &h in &[0i32, -1, 1] {
            let _g = call_lock();
            let raw = {
                let n = 4096usize;
                rng.bytes(n)
            };
            let a = unfilter_in_child(&l.c, w, h, bpp, &raw, false);
            let r = unfilter_in_child(&l.r, w, h, bpp, &raw, false);
            assert!(!a.status.is_timeout(), "C hung on w={w} h={h} bpp={bpp}");
            assert!(!r.status.is_timeout(), "Rust hung on w={w} h={h} bpp={bpp}");
            assert_child_match(&a, &r, "extreme-dims");
        }
    }

    // Well-formed dimensions with every possible filter byte in row 0 and in a
    // later row, so the accept/reject decision is covered for all 256 values.
    for f in 0u16..=255 {
        let (w, h, bpp) = (3i32, 3i32, 2i32);
        let stride = (1 + w * bpp) as usize;
        let mut raw = vec![0u8; stride * h as usize + 64];
        for b in raw.iter_mut() {
            *b = rng.byte();
        }
        for y in 0..h as usize {
            raw[y * stride] = 0;
        }
        raw[0] = f as u8;
        let r0 = diff_unfilter(w, h, bpp, &raw, AB, "filter-tag-row0");
        assert_eq!(r0.ret, (f <= 4) as i32, "row0 filter tag {f}");

        raw[0] = 0;
        raw[stride] = f as u8;
        let r1 = diff_unfilter(w, h, bpp, &raw, AB, "filter-tag-row1");
        assert_eq!(r1.ret, (f <= 4) as i32, "row1 filter tag {f}");
    }
}

//! Phase C — error-path differential tests for every *returning* row of
//! `ERRORS.md` (rows 1-6) plus the generic FFI boundary rows G2/G3/G5-G12.
//!
//! Assert/abort rows (11-20, G1, G4) live in `phase_c_aborts.rs`; unreachable
//! `static`-function rows (7-10) are verified structurally in `phase_c_dead.rs`.

mod common;

use common::deflate::*;
use common::*;

const SEED: u64 = 0x5EED_C0DE_1234_5678;

const E_NLEN: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const E_STORED_LONG: &str = "Stored block extends beyond end of input stream.";
const E_SYMBOL: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const E_DISTANCE: &str = "Attempted to write before out buffer (invalid backwards distance).";
const E_STRING: &str = "Attempted to overwrite out buffer while outputting a string.";
const E_BTYPE: &str = "Detected unknown block type within input stream.";

fn tail(v: &[u8]) -> Vec<u8> {
    let mut o = v.to_vec();
    o.extend([0u8; 8]);
    o
}

/// A fixed-Huffman block from raw tokens, with no expected-output computation
/// (these streams are deliberately invalid).
fn raw_fixed(toks: &[Tok]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, true, toks);
    tail(&bw.finish())
}

/// Assert both libraries reject with the same code AND the same message.
fn expect_error(p: &Pair, label: &str, stream: &[u8], out_bytes: i32, msg: &str) {
    let r = diff_inflate_pad(p, label, stream, 0, out_bytes, 0);
    assert_eq!(r.ret, 0, "[{label}] expected rejection, got ret=1");
    assert_eq!(
        r.reason.as_deref(),
        Some(msg),
        "[{label}] wrong cp_error_reason"
    );
}

// ===========================================================================
// ERRORS.md #1 — LEN != (uint16_t)~NLEN
// ===========================================================================

#[test]
fn err01_stored_len_nlen_mismatch() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 101);
    for i in 0..300 {
        let len: u16 = rng.range(0, 64) as u16;
        // any NLEN other than !len
        let mut nlen: u16 = rng.next_u32() as u16;
        if nlen == !len {
            nlen = nlen.wrapping_add(1);
        }
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align_to_byte();
        bw.raw_bytes(&len.to_le_bytes());
        bw.raw_bytes(&nlen.to_le_bytes());
        bw.raw_bytes(&rng.bytes(len as usize));
        let s = bw.finish();
        expect_error(&p, &format!("err01/{i}"), &s, 4096, E_NLEN);
    }
    // exact boundary: NLEN off by one in each direction
    for delta in [1i32, -1] {
        let len: u16 = 8;
        let nlen = ((!len) as i32 + delta) as u16;
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align_to_byte();
        bw.raw_bytes(&len.to_le_bytes());
        bw.raw_bytes(&nlen.to_le_bytes());
        bw.raw_bytes(&[0u8; 8]);
        expect_error(&p, "err01/offbyone", &bw.finish(), 4096, E_NLEN);
    }
}

// ===========================================================================
// ERRORS.md #2 — !(s->bits_left / 8 <= (int)LEN)
// ===========================================================================

#[test]
fn err02_stored_extends_beyond_input() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 102);
    for i in 0..300 {
        let len = rng.range(0, 64) as usize;
        let extra = rng.range(1, 16) as usize; // >= 1 byte too many
        let mut bw = BitWriter::new();
        write_stored_block(&mut bw, true, &rng.bytes(len));
        let mut s = bw.finish();
        s.extend(rng.bytes(extra));
        expect_error(&p, &format!("err02/{i}"), &s, 4096, E_STORED_LONG);
    }
    // exact boundary: one byte of slack is already too much
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, true, &[1, 2, 3, 4, 5, 6, 7, 8]);
    let ok = bw.finish();
    let r = diff_inflate_pad(&p, "err02/exact", &ok, 0, 4096, 0);
    assert_eq!(r.ret, 1, "exact-length stored block must be accepted");
    let mut one_more = ok.clone();
    one_more.push(0);
    expect_error(&p, "err02/plus1", &one_more, 4096, E_STORED_LONG);
}

// ===========================================================================
// ERRORS.md #3 — literal does not fit in the out buffer  (also G2, G5)
// ===========================================================================

#[test]
fn err03_literal_overruns_out() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 103);
    for i in 0..300 {
        let n = rng.range(1, 64) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let s = raw_fixed(&toks);
        // out_bytes strictly smaller than the literal count
        let out_bytes = rng.range(0, n as i64 - 1) as i32;
        expect_error(&p, &format!("err03/{i}"), &s, out_bytes, E_SYMBOL);
    }
}

#[test]
fn g02_out_bytes_zero_with_literal() {
    let p = load_pair();
    let s = raw_fixed(&[Tok::Lit(0x42)]);
    expect_error(&p, "g02", &s, 0, E_SYMBOL);
}

#[test]
fn g03_out_bytes_zero_stored_len_zero() {
    let p = load_pair();
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, true, &[]);
    let s = bw.finish();
    let r = diff_inflate_pad(&p, "g03", &s, 0, 0, 0);
    assert_eq!(r.ret, 1, "LEN=0 stored block with out_bytes=0 must succeed");
}

#[test]
fn g05_negative_out_bytes() {
    let p = load_pair();
    for out_bytes in [-1i32, -4, -1000] {
        let s = raw_fixed(&[Tok::Lit(1), Tok::Lit(2)]);
        let r = diff_inflate_pad(&p, &format!("g05/{out_bytes}"), &s, 0, out_bytes, 64);
        assert_eq!(r.ret, 0, "negative out_bytes must be rejected");
        assert_eq!(r.reason.as_deref(), Some(E_SYMBOL));
    }
    // and with a match, so the *string* check is the one that fires
    let toks = vec![
        Tok::Lit(1),
        Tok::Lit(2),
        Tok::RawMatch {
            len_sym: 0,
            len_extra: 0,
            dist_sym: 1,
            dist_extra: 0,
        },
    ];
    let s = raw_fixed(&toks);
    let r = diff_inflate_pad(&p, "g05/match", &s, 0, -8, 64);
    assert_eq!(r.ret, 0);
}

// ===========================================================================
// ERRORS.md #4 — backwards distance reaches before `begin`
// ===========================================================================

#[test]
fn err04_invalid_backwards_distance() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 104);
    for i in 0..300 {
        let nlit = rng.range(0, 8) as usize;
        let mut toks: Vec<Tok> = (0..nlit).map(|_| Tok::Lit(rng.byte())).collect();
        // distance strictly greater than the bytes produced so far
        let ds = rng.range(0, 29) as u8;
        let nextra = DIST_EXTRA[ds as usize] as u32;
        let de = if nextra == 0 {
            0
        } else {
            rng.next_u32() % (1u32 << nextra)
        };
        let dist = DIST_BASE[ds as usize] + de;
        if dist as usize <= nlit {
            continue;
        }
        toks.push(Tok::RawMatch {
            len_sym: 0, // length 3
            len_extra: 0,
            dist_sym: ds,
            dist_extra: de,
        });
        let s = raw_fixed(&toks);
        expect_error(&p, &format!("err04/{i}"), &s, 4096, E_DISTANCE);
    }
    // exact boundary: distance == produced is legal, distance == produced+1 is not
    for nlit in [1usize, 2, 5, 17] {
        let base: Vec<Tok> = (0..nlit).map(|k| Tok::Lit(k as u8)).collect();
        let (ok_ds, ok_de) = dist_to_sym(nlit as u16);
        let mut ok_toks = base.clone();
        ok_toks.push(Tok::RawMatch {
            len_sym: 0,
            len_extra: 0,
            dist_sym: ok_ds,
            dist_extra: ok_de,
        });
        let r = diff_inflate_pad(&p, "err04/at", &raw_fixed(&ok_toks), 0, 4096, 0);
        assert_eq!(r.ret, 1, "distance == produced must be accepted (nlit={nlit})");

        let (bad_ds, bad_de) = dist_to_sym(nlit as u16 + 1);
        let mut bad_toks = base;
        bad_toks.push(Tok::RawMatch {
            len_sym: 0,
            len_extra: 0,
            dist_sym: bad_ds,
            dist_extra: bad_de,
        });
        expect_error(
            &p,
            "err04/past",
            &raw_fixed(&bad_toks),
            4096,
            E_DISTANCE,
        );
    }
}

// ===========================================================================
// ERRORS.md #5 — match string does not fit in the out buffer
// ===========================================================================

#[test]
fn err05_string_overruns_out() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 105);
    for i in 0..300 {
        let nlit = rng.range(2, 16) as usize;
        let mut toks: Vec<Tok> = (0..nlit).map(|_| Tok::Lit(rng.byte())).collect();
        let len = rng.range(3, 258) as u16;
        let dist = rng.range(1, nlit as i64) as u16;
        toks.push(Tok::Match { len, dist });
        let s = raw_fixed(&toks);
        // room for the literals but not for the whole match
        let out_bytes = (nlit + rng.range(0, len as i64 - 1) as usize) as i32;
        expect_error(&p, &format!("err05/{i}"), &s, out_bytes, E_STRING);
    }
    // exact boundary
    let toks = vec![
        Tok::Lit(1),
        Tok::Lit(2),
        Tok::Match { len: 4, dist: 2 },
    ];
    let s = raw_fixed(&toks);
    let r = diff_inflate_pad(&p, "err05/fits", &s, 0, 6, 0);
    assert_eq!(r.ret, 1, "exactly-fitting match must be accepted");
    expect_error(&p, "err05/one_short", &s, 5, E_STRING);
    // distance == 1 (memset path) must reject identically
    let toks = vec![Tok::Lit(9), Tok::Match { len: 100, dist: 1 }];
    expect_error(&p, "err05/memset", &raw_fixed(&toks), 50, E_STRING);
}

// ===========================================================================
// ERRORS.md #6 — btype == 3
// ===========================================================================

#[test]
fn err06_unknown_block_type() {
    let p = load_pair();
    for bfinal in [0u32, 1] {
        let mut bw = BitWriter::new();
        bw.bits(bfinal, 1);
        bw.bits(3, 2);
        let s = tail(&bw.finish());
        expect_error(&p, &format!("err06/bfinal{bfinal}"), &s, 4096, E_BTYPE);
    }
    // btype 3 as the *second* block, after a valid one
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, false, &[Tok::Lit(7), Tok::Lit(8)]);
    bw.bits(0, 1);
    bw.bits(3, 2);
    let s = tail(&bw.finish());
    expect_error(&p, "err06/second", &s, 4096, E_BTYPE);
}

// ===========================================================================
// Generic boundary: the `+2` padding entries of the length/distance tables
// ===========================================================================

#[test]
fn g_padding_symbols_286_287_and_dist_30_31() {
    // Literal symbols 286/287 exist in `cp_fixed_table` but map to
    // `cp_len_base[29] == cp_len_base[30] == 0`, i.e. a zero-length match;
    // distance symbols 30/31 likewise map to distance 0. These are the two
    // "+2" padding slots the C deliberately keeps, and they are reachable.
    let p = load_pair();
    for ls in [29u8, 30] {
        for ds in [0u8, 29, 30, 31] {
            let toks = vec![
                Tok::Lit(0x11),
                Tok::Lit(0x22),
                Tok::RawMatch {
                    len_sym: ls,
                    len_extra: 0,
                    dist_sym: ds,
                    dist_extra: 0,
                },
                Tok::Lit(0x33),
            ];
            let s = raw_fixed(&toks);
            let lbl = format!("g_pad/len{ls}/dist{ds}");
            // Whatever the C does here (accept with a 0-byte copy, or reject),
            // the Rust must do exactly the same.
            diff_inflate_pad(&p, &lbl, &s, 0, 4096, 0);
        }
    }
}

// ===========================================================================
// Generic boundary: null / zero-size pointers that do NOT dereference
// ===========================================================================

#[test]
fn g_null_out_zero_len() {
    // out == NULL with out_bytes == 0: `cp_block` rejects before any store.
    let p = load_pair();
    let stream = raw_fixed(&[Tok::Lit(1)]);
    for imp in [&p.c, &p.rs] {
        imp.clear_error();
    }
    let mut a = AlignedBuf::new(&stream, 0, 8);
    let mut b = AlignedBuf::new(&stream, 0, 8);
    let (ra, rb) = unsafe {
        (
            (p.c.inflate)(a.ptr(), a.len(), std::ptr::null_mut(), 0),
            (p.rs.inflate)(b.ptr(), b.len(), std::ptr::null_mut(), 0),
        )
    };
    assert_eq!(ra, rb, "null out / zero len return mismatch");
    assert_eq!(ra, 0);
    assert_eq!(p.c.error_reason_str(), p.rs.error_reason_str());
    assert_eq!(p.c.error_reason_str().as_deref(), Some(E_SYMBOL));
}

#[test]
fn g_convert_pix_null_pointers_no_deref() {
    // h == 0: neither src nor dst is touched, so NULL is safe on both sides.
    let p = load_pair();
    for bpp in [-1i32, 0, 1, 2, 3, 4, 5, i32::MAX] {
        for w in [0i32, 1, 64, -1] {
            unsafe {
                (p.c.convert_pix)(bpp, w, 0, std::ptr::null_mut(), std::ptr::null_mut());
                (p.rs.convert_pix)(bpp, w, 0, std::ptr::null_mut(), std::ptr::null_mut());
            }
        }
    }
    // w == 0 writes no pixels, so a NULL dst is safe; src is still walked.
    let mut src = vec![0u8; 256];
    for bpp in [1i32, 2, 3, 4] {
        for h in [0i32, 1, 8] {
            unsafe {
                (p.c.convert_pix)(bpp, 0, h, src.as_mut_ptr(), std::ptr::null_mut());
                (p.rs.convert_pix)(bpp, 0, h, src.as_mut_ptr(), std::ptr::null_mut());
            }
        }
    }
}

// ===========================================================================
// G6 / out-of-range "enum" values: convert_pix's `switch (bpp)` has no default
// ===========================================================================

#[test]
fn g06_convert_pix_bpp_out_of_range() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 106);
    // Every value with no matching `case` in the C switch. `dst` must be left
    // untouched and `dst` must not advance, while `src` still advances by `bpp`.
    for bpp in [
        i32::MIN,
        -1000,
        -4,
        -1,
        0,
        5,
        6,
        7,
        8,
        255,
        256,
        65536,
        i32::MAX,
    ] {
        for (w, h) in [(0i32, 0i32), (1, 1), (2, 1), (1, 2), (3, 4)] {
            // A large |bpp| only moves the pointer; the switch never matches, so
            // nothing is read. Give the buffer generous slack in both directions.
            let src = rng.bytes(4096);
            diff_convert_pix(
                &p,
                &format!("g06/bpp{bpp}/{w}x{h}"),
                bpp,
                w,
                h,
                &src,
                (w.max(0) * h.max(0)) as usize + 4,
                2048,
            );
        }
    }
}

#[test]
fn g09_convert_pix_negative_w_h() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 109);
    for bpp in 1..=4i32 {
        for (w, h) in [
            (-1i32, 4i32),
            (4, -1),
            (-1, -1),
            (i32::MIN, 1),
            (1, i32::MIN),
            (0, -5),
            (-5, 0),
        ] {
            let src = rng.bytes(1024);
            diff_convert_pix(
                &p,
                &format!("g09/bpp{bpp}/{w}x{h}"),
                bpp,
                w,
                h,
                &src,
                8,
                512,
            );
        }
    }
}

// ===========================================================================
// One step past every documented valid range in cp_inflate's headers
// ===========================================================================

#[test]
fn g_dynamic_header_extremes() {
    // nlit is 257 + 5 bits (max 288 == 257+31), ndst is 1 + 5 bits (max 32),
    // nlen is 4 + 4 bits (max 19). All 5-/4-bit values are representable, so
    // there is no "one past" — but the maxima drive `cp_build` over the largest
    // possible alphabets, and nlit == 288 makes `lens[]` exactly full.
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 110);
    for nlit in [257usize, 258, 287, 288] {
        for ndst in [1usize, 2, 31, 32] {
            let toks: Vec<Tok> = (0..24).map(|_| Tok::Lit(rng.byte())).collect();
            let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, nlit, ndst, None);
            let expect = apply_tokens(&toks);
            let mut bw = BitWriter::new();
            write_dynamic_block(&mut bw, true, &spec, &toks);
            let s = tail(&bw.finish());
            let lbl = format!("g_hdr/nlit{nlit}/ndst{ndst}");
            let r = diff_inflate_pad(&p, &lbl, &s, 0, expect.len() as i32, 0);
            assert_eq!(r.ret, 1, "[{lbl}] {:?}", r.reason);
            assert_eq!(&r.out[..expect.len()], &expect[..]);
        }
    }
    for nlen in 4usize..=19 {
        let toks: Vec<Tok> = (0..16).map(|_| Tok::Lit(rng.byte())).collect();
        let mut spec =
            dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
        // only force values that still transmit every used CL symbol
        spec.force_nlen = Some(nlen);
        let mut bw = BitWriter::new();
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            write_dynamic_block(&mut bw, true, &spec, &toks);
        }));
        if built.is_err() {
            continue; // nlen too small for this alphabet; not a library case
        }
        let s = tail(&bw.finish());
        let expect = apply_tokens(&toks);
        let lbl = format!("g_hdr/nlen{nlen}");
        let r = diff_inflate_pad(&p, &lbl, &s, 0, expect.len() as i32, 0);
        assert_eq!(r.ret, 1, "[{lbl}] {:?}", r.reason);
        assert_eq!(&r.out[..expect.len()], &expect[..]);
    }
}

// ===========================================================================
// Fuzz sweep: random bytes as a deflate stream. Anything that does not trip a
// live C assert must be rejected/accepted identically.
// ===========================================================================

#[test]
fn g_random_garbage_streams() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 111);
    let mut agreed = 0usize;
    for i in 0..4000 {
        // >= 5 bytes so the 40 bits `cp_stored` reads are always available; any
        // shorter input trips the live `assert(s->bits_left > 0)` and aborts (that
        // case is covered in phase_c_aborts.rs).
        let n = rng.range(5, 48) as usize;
        let mut s = rng.bytes(n);
        // Force bfinal = 1, btype = 0 (stored). Huffman block types can trip the
        // live `cp_decode` assert, so the unrestricted random space is fuzzed in
        // the child-process harness instead.
        s[0] = (s[0] & !0b110) | 0b001;
        let out_bytes = rng.range(0, 4096) as i32;
        diff_inflate_pad(&p, &format!("garbage/{i}"), &s, rng.below(4), out_bytes, 64);
        agreed += 1;
    }
    assert_eq!(agreed, 4000);
}

// ===========================================================================
// ERRORS.md #7-#10 — rejections inside `static` functions with no caller
// ===========================================================================

#[test]
fn err07_to_err10_unreachable_static_rejections() {
    // `cp_unfilter` (#7, #8), `cp_chunk` (#9) and `cp_find` (#10) are `static` in
    // the C translation unit AND are never called from any non-`static` function,
    // so their rejection paths are unreachable across the ABI. Verify that
    // structurally: neither library exports them, so no differential call is
    // possible (or required) for these rows.
    let p = load_pair();
    for name in [
        b"cp_unfilter\0".as_slice(),
        b"cp_chunk\0".as_slice(),
        b"cp_find\0".as_slice(),
        b"cp_paeth\0".as_slice(),
        b"cp_make32\0".as_slice(),
        b"cp_stored\0".as_slice(),
        b"cp_block\0".as_slice(),
        b"cp_decode\0".as_slice(),
        b"cp_build\0".as_slice(),
        b"cp_dynamic\0".as_slice(),
        b"cp_fixed\0".as_slice(),
        b"cp_read_bits\0".as_slice(),
    ] {
        let n = String::from_utf8_lossy(&name[..name.len() - 1]).into_owned();
        assert!(
            !p.c.has_symbol(name),
            "C .so unexpectedly exports static fn {n}"
        );
        assert!(
            !p.rs.has_symbol(name),
            "Rust .so exports {n}, which the C .so keeps private — symbol parity broken"
        );
    }
}

//! Phase C — error-path differential tests (the non-aborting rows of
//! `ERRORS.md`). Each test builds the exact rejected input and requires that C
//! and Rust return the SAME sentinel *and* the SAME `cp_error_reason` string.

mod common;

use common::deflate::*;
use common::*;

const SEED: u64 = 0xE770_0001;

/// Assert both libraries reject `input` with return 0 and the given reason.
#[track_caller]
fn expect_error(
    label: &str,
    input: &[u8],
    align: usize,
    in_bytes: i32,
    out_bytes: i32,
    out_capacity: usize,
    reason: &str,
) {
    let o = diff_inflate_full(label, input, align, in_bytes, out_bytes, out_capacity);
    assert_eq!(o.ret, 0, "[{label}] expected rejection, got success");
    let got = o.err.expect(&format!("[{label}] cp_error_reason was NULL"));
    assert_eq!(
        String::from_utf8_lossy(&got),
        reason,
        "[{label}] wrong cp_error_reason"
    );
}

// ==========================================================================
// row 1 — stored block whose LEN is not the complement of NLEN
// ==========================================================================

#[test]
fn err01_stored_len_nlen_mismatch() {
    let rng = Rng::new(SEED ^ 1);
    for i in 0..120 {
        let n = rng.range(0, 40);
        let data = rng.bytes(n);
        let len = n as u16;
        // any NLEN other than !len
        let mut nlen = rng.next_u32() as u16;
        if nlen == !len {
            nlen = nlen.wrapping_add(1);
        }
        let mut w = BitWriter::new();
        write_stored_block_raw(&mut w, true, len, nlen, &data);
        expect_error(
            &format!("err01/len={len} nlen={nlen} #{i}"),
            &w.finish(),
            i % 4,
            (5 + n) as i32,
            (n + 8) as i32,
            n + 72,
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );
    }
}

// ==========================================================================
// row 2 — stored block that does not reach the end of the input
// ==========================================================================

#[test]
fn err02_stored_extends_beyond_input() {
    let rng = Rng::new(SEED ^ 2);
    for i in 0..120 {
        let n = rng.range(0, 40);
        let extra = rng.range(1, 16); // bytes left over after the block
        let data = rng.bytes(n);
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &data);
        let mut stream = w.finish();
        stream.extend(rng.bytes(extra));
        // remaining = n + extra > LEN = n  =>  "extends beyond end of input"
        expect_error(
            &format!("err02/len={n} extra={extra} #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            (n + 8) as i32,
            n + 72,
            "Stored block extends beyond end of input stream.",
        );
    }
}

// ==========================================================================
// row 3 / row 24 — literal with a full output buffer
// ==========================================================================

#[test]
fn err03_out_overflow_symbol() {
    let rng = Rng::new(SEED ^ 3);
    for i in 0..120 {
        let n = rng.range(1, 40);
        let data = rng.bytes(n);
        let stream = fixed_literal_stream(&data);
        // out_bytes strictly less than the payload: fails on literal #out_bytes
        let out_bytes = rng.below(n);
        expect_error(
            &format!("err03/n={n} out_bytes={out_bytes} #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            out_bytes as i32,
            out_bytes + 64,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
}

#[test]
fn err24_out_bytes_zero() {
    let rng = Rng::new(SEED ^ 24);
    for i in 0..60 {
        let data = rng.bytes(rng.range(1, 20));
        let stream = fixed_literal_stream(&data);
        expect_error(
            &format!("err24/out_bytes=0 #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            0,
            64,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
}

#[test]
fn err25_out_bytes_negative() {
    let rng = Rng::new(SEED ^ 25);
    for &ob in &[-1i32, -2, -17, -4096, i32::MIN / 2] {
        for i in 0..8 {
            let data = rng.bytes(rng.range(1, 20));
            let stream = fixed_literal_stream(&data);
            // out_end = out + ob is *before* out, so the very first literal fails
            expect_error(
                &format!("err25/out_bytes={ob} #{i}"),
                &stream,
                i % 4,
                stream.len() as i32,
                ob,
                64,
                "Attempted to overwrite out buffer while outputting a symbol.",
            );
        }
    }
}

#[test]
fn err26_out_one_byte_short() {
    let rng = Rng::new(SEED ^ 26);
    for i in 0..80 {
        let n = rng.range(2, 60);
        let data = rng.bytes(n);
        let stream = fixed_literal_stream(&data);
        expect_error(
            &format!("err26/one-short n={n} #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            (n - 1) as i32,
            n + 64,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
    // ...and one byte short of a *match* (row 5's neighbour)
    for i in 0..60 {
        let pre = rng.range(1, 20);
        let mlen = rng.range(3, 60) as u32;
        let mut toks: Vec<Tok> = rng.bytes(pre).into_iter().map(Tok::Lit).collect();
        toks.push(Tok::Match { len: mlen, dist: pre as u32 });
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &toks);
        let total = pre + mlen as usize;
        let stream = w.finish();
        expect_error(
            &format!("err26/match-one-short pre={pre} mlen={mlen} #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            (total - 1) as i32,
            total + 64,
            "Attempted to overwrite out buffer while outputting a string.",
        );
    }
}

// ==========================================================================
// row 4 — back-reference pointing before the start of the output buffer
// ==========================================================================

#[test]
fn err04_invalid_backwards_distance() {
    let rng = Rng::new(SEED ^ 4);
    for i in 0..120 {
        let pre = rng.below(8); // bytes emitted before the bad match
        let dist = (pre + rng.range(1, 40)) as u32; // strictly greater than `pre`
        let len = rng.range(3, 30) as u32;
        let mut toks: Vec<Tok> = rng.bytes(pre).into_iter().map(Tok::Lit).collect();
        toks.push(Tok::Match { len, dist });
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &toks);
        let stream = w.finish();
        expect_error(
            &format!("err04/pre={pre} dist={dist} len={len} #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            1024,
            1024 + 64,
            "Attempted to write before out buffer (invalid backwards distance).",
        );
    }
}

// ==========================================================================
// row 5 — back-reference copy that runs past the end of the output buffer
// ==========================================================================

#[test]
fn err05_out_overflow_string() {
    let rng = Rng::new(SEED ^ 5);
    for i in 0..120 {
        let pre = rng.range(1, 30);
        let len = rng.range(3, 258) as u32;
        let mut toks: Vec<Tok> = rng.bytes(pre).into_iter().map(Tok::Lit).collect();
        toks.push(Tok::Match { len, dist: pre as u32 });
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &toks);
        // room for every literal, but at least one byte too few for the copy
        let short_by = rng.range(1, len as usize);
        let out_bytes = pre + len as usize - short_by;
        let stream = w.finish();
        expect_error(
            &format!("err05/pre={pre} len={len} out={out_bytes} #{i}"),
            &stream,
            i % 4,
            stream.len() as i32,
            out_bytes as i32,
            out_bytes + 300,
            "Attempted to overwrite out buffer while outputting a string.",
        );
    }
}

// ==========================================================================
// row 6 — reserved block type 3
// ==========================================================================

#[test]
fn err06_unknown_block_type() {
    let rng = Rng::new(SEED ^ 6);
    for i in 0..80 {
        for &bfinal in &[0u8, 1] {
            // bit 0 = BFINAL, bits 1..2 = BTYPE = 11
            let first = bfinal | 0b110;
            let mut stream = vec![first];
            stream.extend(rng.bytes(rng.below(8)));
            expect_error(
                &format!("err06/bfinal={bfinal} len={} #{i}", stream.len()),
                &stream,
                i % 4,
                stream.len() as i32,
                64,
                128,
                "Detected unknown block type within input stream.",
            );
        }
    }
}

// ==========================================================================
// rows 27-29 — convert_pix with out-of-range / degenerate arguments
// ==========================================================================

#[test]
fn err27_convert_pix_bad_bpp() {
    let rng = Rng::new(SEED ^ 27);
    // Every `int` that is not one of the four `switch` arms: no pixel is ever
    // written, but `src` still advances by `bpp` per pixel.
    let bad: &[i32] = &[
        0,
        5,
        6,
        7,
        8,
        16,
        255,
        256,
        -1,
        -2,
        -4,
        1000,
        0x7FFF_FFFE,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ];
    for &bpp in bad {
        for i in 0..6 {
            let w = [0i32, 1, 3, 9][i % 4];
            let h = [0i32, 1, 2, 5][(i / 2) % 4];
            let src = rng.bytes(4096);
            diff_convert_pix(
                &format!("err27/bpp={bpp} w={w} h={h} #{i}"),
                bpp,
                w,
                h,
                &src,
                (w.max(0) * h.max(0)) as usize + 16,
            );
        }
    }
}

#[test]
fn err28_convert_pix_nonpositive_dims() {
    let rng = Rng::new(SEED ^ 28);
    for bpp in 1..=4i32 {
        for &w in &[0i32, -1, -7, i32::MIN, i32::MIN + 1] {
            for &h in &[0i32, -1, -7, i32::MIN, i32::MIN + 1] {
                let src = rng.bytes(512);
                diff_convert_pix(
                    &format!("err28/bpp={bpp} w={w} h={h}"),
                    bpp,
                    w,
                    h,
                    &src,
                    32,
                );
            }
        }
    }
    // negative w with positive h: the row loop body never runs, but the
    // per-row `src++` still happens h times
    for bpp in 1..=4i32 {
        for &h in &[1i32, 4, 9] {
            let src = rng.bytes(512);
            diff_convert_pix(&format!("err28/bpp={bpp} w=-3 h={h}"), bpp, -3, h, &src, 32);
        }
    }
}

#[test]
fn err29_convert_pix_zero_width() {
    let rng = Rng::new(SEED ^ 29);
    for bpp in 1..=4i32 {
        for &h in &[1i32, 2, 7, 64] {
            for i in 0..6 {
                let src = rng.bytes(512);
                diff_convert_pix(
                    &format!("err29/bpp={bpp} w=0 h={h} #{i}"),
                    bpp,
                    0,
                    h,
                    &src,
                    32,
                );
            }
        }
    }
}

// ==========================================================================
// rows 30-31 — cp_error_reason lifecycle
// ==========================================================================

/// dlopen a *private copy* of a shared object so its globals are pristine.
fn open_fresh(tag: &'static str, src: &std::path::Path, uniq: &str) -> Lib {
    let dir = std::env::temp_dir();
    let dst = dir.join(format!(
        "cp_fresh_{uniq}_{}_{}",
        std::process::id(),
        src.file_name().unwrap().to_string_lossy()
    ));
    std::fs::copy(src, &dst).expect("copy .so");
    Lib::open(tag, &dst)
}

#[test]
fn err30_error_reason_initially_null() {
    let c = open_fresh("C", &c_so_path(), "e30");
    let rs = open_fresh("RUST", &rust_so_path(), "e30");
    assert_eq!(c.error_reason(), None, "C: cp_error_reason must start NULL");
    assert_eq!(rs.error_reason(), None, "RUST: cp_error_reason must start NULL");
}

#[test]
fn err31_error_reason_not_cleared_on_success() {
    let c = open_fresh("C", &c_so_path(), "e31");
    let rs = open_fresh("RUST", &rust_so_path(), "e31");

    // 1. fail: BTYPE == 3
    let bad = vec![0b111u8, 0, 0, 0];
    // 2. succeed: a plain fixed literal block
    let good_payload = b"the quick brown fox".to_vec();
    let good = fixed_literal_stream(&good_payload);

    for (lib, tag) in [(&c, "C"), (&rs, "RUST")] {
        let mut inb = AlignedBuf::new(&bad, 0);
        let mut out = vec![0u8; 128];
        let r = unsafe {
            (lib.cp_inflate())(
                inb.ptr() as *mut std::ffi::c_void,
                bad.len() as i32,
                out.as_mut_ptr() as *mut std::ffi::c_void,
                out.len() as i32,
            )
        };
        assert_eq!(r, 0, "{tag}: expected failure");
    }
    let e_c = c.error_reason();
    let e_rs = rs.error_reason();
    assert_eq!(e_c, e_rs, "cp_error_reason after failure differs");
    assert_eq!(
        String::from_utf8_lossy(e_c.as_ref().unwrap()),
        "Detected unknown block type within input stream."
    );

    for (lib, tag) in [(&c, "C"), (&rs, "RUST")] {
        let mut inb = AlignedBuf::new(&good, 0);
        let mut out = vec![0u8; good_payload.len()];
        let r = unsafe {
            (lib.cp_inflate())(
                inb.ptr() as *mut std::ffi::c_void,
                good.len() as i32,
                out.as_mut_ptr() as *mut std::ffi::c_void,
                out.len() as i32,
            )
        };
        assert_eq!(r, 1, "{tag}: expected success");
        assert_eq!(out, good_payload, "{tag}: payload");
    }
    // C never clears cp_error_reason on success — Rust must not either.
    assert_eq!(c.error_reason(), e_c, "C cleared cp_error_reason on success?!");
    assert_eq!(
        rs.error_reason(),
        c.error_reason(),
        "cp_error_reason after a successful call differs"
    );
}

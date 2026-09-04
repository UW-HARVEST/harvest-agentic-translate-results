#![allow(non_snake_case)]

//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each row constructs the exact invalid input the C checks for, calls BOTH
//! `.so`s and asserts the *same* rejection: identical return value / NULL-ness,
//! identical `cp_error_reason` text, identical termination signal for the
//! `assert()` rows.

mod common;

use common::deflate::{self, Tok};
use common::png::{self, ColorType, PngSpec};
use common::*;

const SIGABRT: i32 = 6;
const SIGSEGV: i32 = 11;

/// Assert both sides agree *and* that the C produced the expected reason.
#[track_caller]
fn expect_reason(label: &str, c: &Outcome, r: &Outcome, reason: &str) {
    assert_same(label, c, r);
    assert_eq!(c.err_str(), reason, "[{label}] unexpected cp_error_reason");
}

/// A minimal valid grey PNG, used as the base for targeted corruption.
fn base_png(rng: &mut Rng, w: usize, h: usize, ct: ColorType) -> PngSpec {
    let bpp = ct.bpp();
    let filters = vec![0u8; h];
    let raw = png::raw_scanlines(rng, w, h, bpp, &filters);
    let def = deflate::stored_block(&raw, true);
    let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw);
    if ct == ColorType::Indexed {
        spec.plte = Some(rng.bytes(256 * 3));
    }
    spec
}

// ===========================================================================
// Row 1 — LEN / NLEN not complements
// ===========================================================================

#[test]
fn err01_stored_len_nlen_mismatch() {
    let mut rng = Rng::new(SEED ^ 0x101);
    for _ in 0..50 {
        let n = rng.range(1, 40) as usize;
        let data = rng.bytes(n);
        let mut def = deflate::stored_block(&data, true);
        // def[1..3] = LEN, def[3..5] = NLEN. Corrupt NLEN.
        let bad = rng.u8();
        if bad == def[3] {
            continue;
        }
        def[3] = bad;
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, n as i32, 2, |_| {});
        expect_reason(
            &format!("err01 LEN/NLEN mismatch n={n}"),
            &c,
            &r,
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );
        assert_eq!(c.ret, 0);
    }
    // Also via load_png_mem: the inner reason is overwritten (row 24).
    let w = 4usize;
    let h = 2usize;
    let mut spec = base_png(&mut rng, w, h, ColorType::Grey);
    spec.deflate[3] ^= 0xFF;
    let file = spec.build();
    let (c, r) = call_load_png(&file);
    expect_reason("err01 via load_png_mem", &c, &r, "DEFLATE algorithm failed");
    assert!(c.pix_null);
}

// ===========================================================================
// Row 2 — stored block extends beyond end of input
// ===========================================================================

#[test]
fn err02_stored_extends_beyond_input() {
    let mut rng = Rng::new(SEED ^ 0x102);
    for extra in [1usize, 2, 3, 4, 8, 9, 64] {
        let n = 16usize;
        let data = rng.bytes(n);
        let mut def = deflate::stored_block(&data, true);
        def.extend_from_slice(&rng.bytes(extra));
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, (n + extra) as i32, 2, |_| {});
        let label = format!("err02 extra={extra}");
        // `bits_left/8 <= LEN` — only whole *bytes* count, so `extra < 8` may
        // still satisfy it depending on the bit bookkeeping; assert the pair
        // agrees and, when the C did reject, that it used the right reason.
        assert_same(&label, &c, &r);
        if c.ret == 0 && c.signal.is_none() {
            assert_eq!(
                c.err_str(),
                "Stored block extends beyond end of input stream.",
                "[{label}] wrong reason"
            );
        }
    }
    // A definitive case: LEN declares 1 byte but 100 follow.
    let mut def = deflate::stored_block(&[0xAB], true);
    def.extend_from_slice(&[0u8; 100]);
    let (c, r) = call_inflate_cfg(&def, def.len() as i32, 128, 2, |_| {});
    expect_reason(
        "err02 LEN=1 with 100 trailing bytes",
        &c,
        &r,
        "Stored block extends beyond end of input stream.",
    );
    assert_eq!(c.ret, 0);
}

// ===========================================================================
// Rows 3-5 — cp_block output-bounds checks
// ===========================================================================

#[test]
fn err03_out_buffer_full_on_literal() {
    let mut rng = Rng::new(SEED ^ 0x103);
    for n in [1usize, 2, 5, 40] {
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let def = deflate::fixed_stream(&toks);
        for out_bytes in [0i32, (n as i32) - 1] {
            if out_bytes < 0 {
                continue;
            }
            let (c, r) = call_inflate(&def, def.len() as i32, out_bytes);
            expect_reason(
                &format!("err03 n={n} out_bytes={out_bytes}"),
                &c,
                &r,
                "Attempted to overwrite out buffer while outputting a symbol.",
            );
            assert_eq!(c.ret, 0);
        }
    }
    // Negative out_bytes ⇒ out_end < out ⇒ the very first literal is rejected.
    let def = deflate::fixed_stream(&[Tok::Lit(1)]);
    for ob in [-1i32, -100, i32::MIN] {
        let (c, r) = call_inflate(&def, def.len() as i32, ob);
        expect_reason(
            &format!("err03 negative out_bytes={ob}"),
            &c,
            &r,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
}

#[test]
fn err04_backwards_distance_before_begin() {
    let mut rng = Rng::new(SEED ^ 0x104);
    // A match before anything has been written: out == begin, so any distance
    // >= 1 points before the buffer.
    for dist in [1u32, 2, 3, 17, 256, 32768] {
        let toks = vec![Tok::Match { len: 3, dist }];
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate(&def, def.len() as i32, 4096);
        expect_reason(
            &format!("err04 empty history dist={dist}"),
            &c,
            &r,
            "Attempted to write before out buffer (invalid backwards distance).",
        );
        assert_eq!(c.ret, 0);
    }
    // A match whose distance exceeds the history written so far.
    for _ in 0..40 {
        let hist = rng.range(1, 30) as usize;
        let dist = hist as u32 + rng.range(1, 50);
        let mut toks: Vec<Tok> = rng.bytes(hist).iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match { len: 3, dist });
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate(&def, def.len() as i32, 4096);
        expect_reason(
            &format!("err04 hist={hist} dist={dist}"),
            &c,
            &r,
            "Attempted to write before out buffer (invalid backwards distance).",
        );
    }
}

#[test]
fn err05_string_overruns_out_buffer() {
    let mut rng = Rng::new(SEED ^ 0x105);
    for _ in 0..40 {
        let hist = rng.range(4, 30) as usize;
        let len = rng.range(3, 258);
        let dist = rng.range(1, hist as u32);
        let mut toks: Vec<Tok> = rng.bytes(hist).iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match { len, dist });
        let def = deflate::fixed_stream(&toks);
        // Room for the history and at least one more byte, but not the match.
        let out_bytes = (hist + (len as usize) - 1) as i32;
        let (c, r) = call_inflate(&def, def.len() as i32, out_bytes);
        expect_reason(
            &format!("err05 hist={hist} len={len} dist={dist} out={out_bytes}"),
            &c,
            &r,
            "Attempted to overwrite out buffer while outputting a string.",
        );
        assert_eq!(c.ret, 0);
    }
}

// ===========================================================================
// Row 6 — reserved block type
// ===========================================================================

#[test]
fn err06_reserved_block_type() {
    // bfinal=1, btype=3 ⇒ low three bits 111.
    for pad in 1..12usize {
        let mut buf = vec![0x07u8];
        buf.extend(std::iter::repeat(0u8).take(pad));
        let (c, r) = call_inflate(&buf, buf.len() as i32, 64);
        expect_reason(
            &format!("err06 btype=3 bfinal=1 in_bytes={}", buf.len()),
            &c,
            &r,
            "Detected unknown block type within input stream.",
        );
        assert_eq!(c.ret, 0);
    }
    // bfinal=0, btype=3 ⇒ low three bits 110.
    let mut buf = vec![0x06u8];
    buf.extend(std::iter::repeat(0u8).take(8));
    let (c, r) = call_inflate(&buf, buf.len() as i32, 64);
    expect_reason(
        "err06 btype=3 bfinal=0",
        &c,
        &r,
        "Detected unknown block type within input stream.",
    );
    // Row G11: all four btype values.
    for btype in 0..4u8 {
        let mut buf = vec![0x01u8 | (btype << 1)];
        buf.extend(std::iter::repeat(0u8).take(16));
        let (c, r) = call_inflate(&buf, buf.len() as i32, 64);
        assert_same(&format!("errG11 btype={btype}"), &c, &r);
    }
}

// ===========================================================================
// Row 7 / 8 — the PNG signature and IHDR chunk
// ===========================================================================

#[test]
fn err07_bad_signature() {
    let mut rng = Rng::new(SEED ^ 0x107);
    let good = base_png(&mut rng, 3, 3, ColorType::Grey).build();
    for i in 0..8usize {
        let mut bad = good.clone();
        bad[i] ^= 0xFF;
        let (c, r) = call_load_png(&bad);
        expect_reason(
            &format!("err07 signature byte {i} corrupted"),
            &c,
            &r,
            "incorrect file signature (is this a png file?)",
        );
        assert!(c.pix_null);
        assert_eq!((c.w, c.h), (0, 0), "w/h must stay 0 before IHDR parsing");
    }
    // Random 8+ byte buffers.
    for _ in 0..40 {
        let n = rng.range(8, 64) as usize;
        let buf = rng.bytes(n);
        if buf[..8] == png::SIG {
            continue;
        }
        let (c, r) = call_load_png(&buf);
        expect_reason(
            "err07 random buffer",
            &c,
            &r,
            "incorrect file signature (is this a png file?)",
        );
    }
}

#[test]
fn err08_missing_or_short_ihdr() {
    let mut rng = Rng::new(SEED ^ 0x108);
    // (a) signature only — cp_chunk reads a length from whatever follows.
    let mut buf = png::SIG.to_vec();
    buf.extend_from_slice(&[0u8; 32]);
    let (c, r) = call_load_png(&buf);
    expect_reason("err08 no chunks", &c, &r, "unable to find IHDR chunk");

    // (b) first chunk is not IHDR.
    for ty in [b"IDAT", b"PLTE", b"IEND", b"iHDR"] {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk(ty, &[0u8; 13]));
        buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(3, 3, 8, 0)));
        let (c, r) = call_load_png(&buf);
        expect_reason(
            &format!("err08 first chunk {:?}", std::str::from_utf8(ty).unwrap()),
            &c,
            &r,
            "unable to find IHDR chunk",
        );
    }

    // (c) IHDR shorter than the 13-byte minimum.
    for len in 0..13u32 {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk(b"IHDR", &vec![0u8; len as usize]));
        buf.extend_from_slice(&png::chunk(b"IEND", &[]));
        let (c, r) = call_load_png(&buf);
        expect_reason(
            &format!("err08 IHDR len={len}"),
            &c,
            &r,
            "unable to find IHDR chunk",
        );
    }

    // (d) IHDR declares a length that runs past the end of the buffer.
    for declared in [14u32, 20, 1000, 0x7FFF_FFF0] {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk_raw_len(
            b"IHDR",
            declared,
            &png::ihdr_data(3, 3, 8, 0),
        ));
        let (c, r) = call_load_png(&buf);
        assert_same(&format!("err08 IHDR declared_len={declared}"), &c, &r);
    }

    // (e) truncated right after the signature (png_length shorter than the data).
    let good = base_png(&mut rng, 3, 3, ColorType::Grey).build();
    for cut in [8usize, 9, 12, 16, 20, 25, 30] {
        let (c, r) = call_load_png_len(&good, cut as i32);
        assert_same(&format!("err08 png_length={cut}"), &c, &r);
    }
}

// ===========================================================================
// Rows 9-12 — IHDR field validation
// ===========================================================================

#[test]
fn err09_bit_depth_full_sweep() {
    // G10: an int-typed field crossing the FFI boundary — sweep all 256 values.
    let mut rng = Rng::new(SEED ^ 0x109);
    for bd in 0..=255u8 {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.bit_depth = bd;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err09 bit_depth={bd}");
        assert_same(&label, &c, &r);
        if bd == 8 {
            assert!(!c.pix_null, "[{label}] bit depth 8 must decode");
        } else {
            assert!(c.pix_null, "[{label}] must be rejected");
            assert_eq!(c.err_str(), "only bit-depth of 8 is supported", "[{label}]");
            assert_eq!((c.w, c.h), (0, 0));
        }
    }
}

#[test]
fn err10_color_type_full_sweep() {
    // G9: the colour type is an `int` switch — every one of the 256 possible
    // byte values is a real input.
    let mut rng = Rng::new(SEED ^ 0x110);
    let valid = [0u8, 2, 3, 4, 6];
    for ct in 0..=255u8 {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.color_type = ct;
        if ct == 3 {
            spec.plte = Some(rng.bytes(256 * 3));
        }
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err10 color_type={ct}");
        assert_same(&label, &c, &r);
        if !valid.contains(&ct) {
            assert!(c.pix_null, "[{label}] must be rejected");
            assert_eq!(c.err_str(), "unknown color type", "[{label}]");
            assert_eq!((c.w, c.h), (0, 0), "[{label}] w/h before IHDR sizing");
        }
    }
}

#[test]
fn err11_width_less_than_one() {
    let mut rng = Rng::new(SEED ^ 0x111);
    // w = cp_make32(ihdr) + 1 as int. w < 1 ⇔ field == 0xFFFFFFFF (w == 0) or
    // field >= 0x7FFFFFFF (w negative).
    for field in [0xFFFF_FFFFu32, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xC000_0000] {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.w = field;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err11 width field=0x{field:08x}");
        assert_same(&label, &c, &r);
        assert!(c.pix_null, "[{label}] must be rejected");
        assert_eq!(
            c.err_str(),
            "invalid IHDR chunk found, image width was less than 1",
            "[{label}]"
        );
    }
    // field == 0 gives w == 1, which is valid (img.w == 0).
    let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
    spec.w = 0;
    let file = spec.build();
    let (c, r) = call_load_png(&file);
    assert_same("err11 width field=0 (w==1, img.w==0)", &c, &r);
}

#[test]
fn err12_height_less_than_one() {
    let mut rng = Rng::new(SEED ^ 0x112);
    for field in [0u32, 0x8000_0000, 0xFFFF_FFFF, 0x9000_0000] {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.h = field;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err12 height field=0x{field:08x}");
        assert_same(&label, &c, &r);
        assert!(c.pix_null, "[{label}] must be rejected");
        assert_eq!(
            c.err_str(),
            "invalid IHDR chunk found, image height was less than 1",
            "[{label}]"
        );
    }
}

// ===========================================================================
// Rows 13-14, 22-23 — size arithmetic
// ===========================================================================

#[test]
fn err13_image_too_large() {
    let mut rng = Rng::new(SEED ^ 0x113);
    // The guard is `(int64_t)w*h*sizeof(cp_pixel_t) < INT_MAX`, evaluated
    // *unsigned* because of the sizeof. w = field+1.
    // Rejected: w*h*4 >= 0x7FFFFFFF, i.e. w*h >= 0x20000000.
    for (wf, hf) in [
        (0x1FFF_FFFFu32, 1u32),      // w = 0x20000000, h = 1  -> 2^31    (reject)
        (0xFFFF, 0x1_0000),          // w = 0x10000,  h = 65536 -> 2^34   (reject)
        (0x3FFF_FFFE, 1),            // w = 0x3FFFFFFF, h = 1   -> ~2^32  (reject)
        (0xFFFF, 0x2000),            // w = 0x10000, h = 8192   -> 2^31   (reject)
    ] {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.w = wf;
        spec.h = hf;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err13 wf=0x{wf:08x} hf=0x{hf:08x}");
        assert_same(&label, &c, &r);
        assert!(c.pix_null, "[{label}] must be rejected");
        assert_eq!(c.err_str(), "image too large", "[{label}]");
        assert_eq!((c.w, c.h), (0, 0), "[{label}] w/h assigned only after the guard");
    }
}

#[test]
fn err14_allocation_boundary() {
    // The largest size the "image too large" guard admits: w*h == 0x1FFFFFFF
    // ⇒ malloc(0x7FFFFFFC) (~2 GiB). btype == 3 in the IDAT makes cp_inflate
    // fail immediately so the outcome is decided by the allocation alone.
    let mut rng = Rng::new(SEED ^ 0x114);
    let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
    spec.w = 0x1FFF_FFFE; // w = 0x1FFFFFFF
    spec.h = 1;
    spec.deflate = vec![0x07, 0, 0, 0, 0, 0, 0, 0];
    let file = spec.build();
    let (c, r) = call_load_png(&file);
    assert_same("err14 2GiB allocation succeeds", &c, &r);
    eprintln!("err14 (unconstrained): {}", c.head());
    assert_eq!(
        c.err_str(),
        "DEFLATE algorithm failed",
        "the 2 GiB allocation was expected to succeed here"
    );
    assert_eq!((c.w, c.h), (0x1FFF_FFFE, 1));

    // Force the allocation to fail by capping the child's address space, which
    // is the only way to reach `!(img.pix)` — the row-13 guard keeps the request
    // below 2 GiB, so it otherwise always succeeds on this host.
    let f2 = file.clone();
    let (c, r) = run_pair(move |lib, shm| unsafe {
        let rl = libc::rlimit {
            rlim_cur: 1 << 30,
            rlim_max: 1 << 30,
        };
        assert_eq!(libc::setrlimit(libc::RLIMIT_AS, &rl), 0);
        let img = (lib.load_png_mem)(f2.as_ptr(), f2.len() as i32);
        (*shm).w = img.w;
        (*shm).h = img.h;
        (*shm).pix_null = img.pix.is_null() as i32;
        (*shm).ret = if img.pix.is_null() { 0 } else { 1 };
    });
    assert_same("err14 2GiB allocation fails (RLIMIT_AS)", &c, &r);
    eprintln!("err14 (RLIMIT_AS=1GiB): {}", c.head());
    assert!(c.pix_null);
    assert_eq!(
        c.err_str(),
        "unable to allocate raw image space",
        "expected the malloc-failure branch"
    );
    // img.w/img.h are assigned *before* the malloc, so they survive.
    assert_eq!((c.w, c.h), (0x1FFF_FFFE, 1));
}

#[test]
fn err22_err23_out_size_guards_are_unreachable() {
    // Rows 22/23 of ERRORS.md: `cp_out_size(&img,4) >= 1` and
    // `cp_out_size(&img,bpp) >= 1`. Given w >= 1, h >= 1 and the row-13 guard
    // (w*h*4 < 0x7FFFFFFF ⇒ w*h < 0x20000000), the int products
    // (img.w+1)*img.h*bpp for bpp in 1..=4 are always in 1..0x7FFFFFFF. The
    // branches therefore cannot be taken. This test pins the boundary: the
    // largest admissible geometry must NOT produce "invalid image size found".
    let mut rng = Rng::new(SEED ^ 0x123);
    for (wf, hf, ct) in [
        (0x1FFF_FFFEu32, 1u32, ColorType::Grey),
        (0x0FFF_FFFE, 2, ColorType::Rgb),
        (0xFFFE, 0x2000, ColorType::GreyAlpha),
        (0x1FFF_FFFE, 1, ColorType::Rgba),
    ] {
        let mut spec = base_png(&mut rng, 3, 2, ct);
        spec.w = wf;
        spec.h = hf;
        // A reserved block type fails cp_inflate straight away, which is *after*
        // both cp_out_size guards — so reaching "DEFLATE algorithm failed"
        // proves neither size guard fired, without decoding 2 GiB.
        spec.deflate = vec![0x07, 0, 0, 0, 0, 0, 0, 0];
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err22/23 boundary wf=0x{wf:08x} hf={hf} ct={}", ct as u8);
        assert_same(&label, &c, &r);
        assert_ne!(
            c.err_str(),
            "invalid image size found",
            "[{label}] unexpectedly reachable"
        );
    }
}

// ===========================================================================
// Rows 15-17 — compression / filter / interlace methods
// ===========================================================================

#[test]
fn err15_16_17_ihdr_method_sweeps() {
    let mut rng = Rng::new(SEED ^ 0x115);
    for v in 0..=255u8 {
        // compression
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.compression = v;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err15 compression={v}");
        assert_same(&label, &c, &r);
        if v != 0 {
            assert!(c.pix_null, "[{label}]");
            assert_eq!(
                c.err_str(),
                "only standard compression DEFLATE is supported",
                "[{label}]"
            );
            assert_eq!((c.w, c.h), (3, 2), "[{label}] img.w/h set before this check");
        }

        // filter method
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.filter = v;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err16 filter_method={v}");
        assert_same(&label, &c, &r);
        if v != 0 {
            assert!(c.pix_null, "[{label}]");
            assert_eq!(
                c.err_str(),
                "only standard adaptive filtering is supported",
                "[{label}]"
            );
        }

        // interlace
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.interlace = v;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err17 interlace={v}");
        assert_same(&label, &c, &r);
        if v != 0 {
            assert!(c.pix_null, "[{label}]");
            assert_eq!(c.err_str(), "interlacing is not supported", "[{label}]");
        }
    }
}

// ===========================================================================
// Row 18 — corrupt / missing zlib structure
// ===========================================================================

#[test]
fn err18_corrupt_zlib_structure() {
    let mut rng = Rng::new(SEED ^ 0x118);
    // (a) No IDAT at all.
    let mut buf = png::SIG.to_vec();
    buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(3, 3, 8, 0)));
    buf.extend_from_slice(&png::chunk(b"IEND", &[]));
    let (c, r) = call_load_png(&buf);
    expect_reason(
        "err18 no IDAT",
        &c,
        &r,
        "corrupt zlib structure in DEFLATE stream",
    );
    assert!(c.pix_null);
    assert_eq!((c.w, c.h), (3, 3));

    // (b) IDAT payloads of 0..5 bytes (datalen < 6).
    for n in 0..6usize {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(3, 3, 8, 0)));
        buf.extend_from_slice(&png::chunk(b"IDAT", &rng.bytes(n)));
        buf.extend_from_slice(&png::chunk(b"IEND", &[]));
        let (c, r) = call_load_png(&buf);
        expect_reason(
            &format!("err18 IDAT payload {n} bytes"),
            &c,
            &r,
            "corrupt zlib structure in DEFLATE stream",
        );
    }

    // (c) Several tiny IDATs that still sum to < 6.
    let mut buf = png::SIG.to_vec();
    buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(3, 3, 8, 0)));
    for _ in 0..5 {
        buf.extend_from_slice(&png::chunk(b"IDAT", &[0x78]));
    }
    buf.extend_from_slice(&png::chunk(b"IEND", &[]));
    let (c, r) = call_load_png(&buf);
    assert_same("err18 five 1-byte IDATs", &c, &r);
    eprintln!("err18(c): {}", c.head());
}

// ===========================================================================
// Rows 19-21 — zlib header validation
// ===========================================================================

#[test]
fn err19_zlib_compression_method() {
    let mut rng = Rng::new(SEED ^ 0x119);
    for cm in 0..16u8 {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.cmf = (spec.cmf & 0xF0) | cm;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err19 CM={cm}");
        assert_same(&label, &c, &r);
        if cm != 8 {
            assert!(c.pix_null, "[{label}]");
            assert_eq!(
                c.err_str(),
                "only zlib compression method (RFC 1950) is supported",
                "[{label}]"
            );
        }
    }
}

#[test]
fn err20_zlib_window_size() {
    let mut rng = Rng::new(SEED ^ 0x120);
    for cinfo in 0..16u8 {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.cmf = (cinfo << 4) | 0x08;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err20 CINFO={cinfo}");
        assert_same(&label, &c, &r);
        if cinfo > 7 {
            assert!(c.pix_null, "[{label}]");
            assert_eq!(c.err_str(), "innapropriate window size detected", "[{label}]");
        } else {
            assert!(!c.pix_null, "[{label}] should decode");
        }
    }
}

#[test]
fn err21_zlib_preset_dictionary() {
    let mut rng = Rng::new(SEED ^ 0x121);
    for flg in 0..=255u8 {
        let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
        spec.flg = flg;
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err21 FLG=0x{flg:02x}");
        assert_same(&label, &c, &r);
        if flg & 0x20 != 0 {
            assert!(c.pix_null, "[{label}]");
            assert_eq!(
                c.err_str(),
                "preset dictionary is present and not supported",
                "[{label}]"
            );
        } else {
            assert!(!c.pix_null, "[{label}] should decode");
        }
    }
}

// ===========================================================================
// Row 24 — DEFLATE failure surfaces as its own message
// ===========================================================================

#[test]
fn err24_deflate_algorithm_failed() {
    let mut rng = Rng::new(SEED ^ 0x124);
    // btype=3 inside the IDAT.
    let mut spec = base_png(&mut rng, 3, 2, ColorType::Grey);
    spec.deflate = vec![0x07, 0, 0, 0, 0, 0, 0, 0];
    let file = spec.build();
    let (c, r) = call_load_png(&file);
    expect_reason("err24 btype=3 in IDAT", &c, &r, "DEFLATE algorithm failed");
    assert!(c.pix_null);
    assert_eq!((c.w, c.h), (3, 2));

    // A deflate stream that stops short of filling the buffer is *not* an error
    // for this C (it just leaves the rest untouched), so force a real failure:
    // a literal-only stream longer than pix_bytes.
    let mut spec = base_png(&mut rng, 2, 2, ColorType::Grey);
    let toks: Vec<Tok> = (0..4096u32).map(|i| Tok::Lit((i & 0xFF) as u8)).collect();
    spec.deflate = deflate::fixed_stream(&toks);
    let file = spec.build();
    let (c, r) = call_load_png(&file);
    expect_reason("err24 output overrun", &c, &r, "DEFLATE algorithm failed");
}

// ===========================================================================
// Rows 25-26 — invalid filter byte, first row and later rows
// ===========================================================================

#[test]
fn err25_invalid_filter_first_row() {
    let mut rng = Rng::new(SEED ^ 0x125);
    for ct in ColorType::ALL {
        for f in [5u8, 6, 7, 127, 128, 200, 255] {
            let w = 4usize;
            let h = 3usize;
            let bpp = ct.bpp();
            let mut filters = vec![0u8; h];
            filters[0] = f;
            let raw = png::raw_scanlines(&mut rng, w, h, bpp, &filters);
            let def = deflate::stored_block(&raw, true);
            let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw);
            if ct == ColorType::Indexed {
                spec.plte = Some(rng.bytes(256 * 3));
            }
            let file = spec.build();
            let (c, r) = call_load_png(&file);
            let label = format!("err25 ct={} first-row filter={f}", ct as u8);
            expect_reason(&label, &c, &r, "invalid filter byte found");
            assert!(c.pix_null);
            assert_eq!((c.w, c.h), (w as i32, h as i32));
        }
    }
}

#[test]
fn err26_invalid_filter_later_row() {
    let mut rng = Rng::new(SEED ^ 0x126);
    for ct in ColorType::ALL {
        for row in [1usize, 2, 4] {
            for f in [5u8, 9, 255] {
                let w = 4usize;
                let h = 5usize;
                let bpp = ct.bpp();
                let mut filters = vec![0u8; h];
                filters[row] = f;
                let raw = png::raw_scanlines(&mut rng, w, h, bpp, &filters);
                let def = deflate::stored_block(&raw, true);
                let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw);
                if ct == ColorType::Indexed {
                    spec.plte = Some(rng.bytes(256 * 3));
                }
                let file = spec.build();
                let (c, r) = call_load_png(&file);
                let label = format!("err26 ct={} row={row} filter={f}", ct as u8);
                expect_reason(&label, &c, &r, "invalid filter byte found");
                assert!(c.pix_null);
            }
        }
    }
}

// ===========================================================================
// Row 27 — indexed colour without PLTE
// ===========================================================================

#[test]
fn err27_indexed_without_plte() {
    let mut rng = Rng::new(SEED ^ 0x127);
    for (w, h) in [(1usize, 1usize), (4, 3), (9, 7)] {
        let filters = vec![0u8; h];
        let raw = png::raw_scanlines(&mut rng, w, h, 1, &filters);
        let def = deflate::stored_block(&raw, true);
        let spec = PngSpec::new(w as u32, h as u32, 3, def, raw);
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        let label = format!("err27 indexed no PLTE {w}x{h}");
        expect_reason(&label, &c, &r, "color type of indexed requires a PLTE chunk");
        assert!(c.pix_null);
        assert_eq!((c.w, c.h), (w as i32, h as i32));
    }
    // A tRNS but still no PLTE.
    let filters = vec![0u8; 2];
    let raw = png::raw_scanlines(&mut rng, 3, 2, 1, &filters);
    let def = deflate::stored_block(&raw, true);
    let mut spec = PngSpec::new(3, 2, 3, def, raw);
    spec.trns = Some(rng.bytes(16));
    let file = spec.build();
    let (c, r) = call_load_png(&file);
    expect_reason(
        "err27 tRNS without PLTE",
        &c,
        &r,
        "color type of indexed requires a PLTE chunk",
    );
}

// ===========================================================================
// Rows 28-29 — cp_chunk / cp_find rejections
// ===========================================================================

#[test]
fn err28_cp_chunk_rejections() {
    let mut rng = Rng::new(SEED ^ 0x128);
    // A second IDAT whose declared length runs past the end stops the
    // concatenation loop; the first IDAT alone then decides the outcome.
    let w = 4usize;
    let h = 3usize;
    let raw = png::raw_scanlines(&mut rng, w, h, 1, &vec![0u8; h]);
    let z = deflate::zlib(&deflate::stored_block(&raw, true), &raw);
    let mid = z.len() / 2;

    let mut buf = png::SIG.to_vec();
    buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(w as u32, h as u32, 8, 0)));
    buf.extend_from_slice(&png::chunk(b"IDAT", &z[..mid]));
    buf.extend_from_slice(&png::chunk_raw_len(b"IDAT", 0x7000_0000, &z[mid..]));
    buf.extend_from_slice(&png::chunk(b"IEND", &[]));
    let (c, r) = call_load_png(&buf);
    assert_same("err28 second IDAT declared past end", &c, &r);
    eprintln!("err28(a): {}", c.head());

    // A non-IDAT chunk between two IDATs also terminates the loop (cp_chunk
    // only accepts an immediately following IDAT).
    let mut buf = png::SIG.to_vec();
    buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(w as u32, h as u32, 8, 0)));
    buf.extend_from_slice(&png::chunk(b"IDAT", &z[..mid]));
    buf.extend_from_slice(&png::chunk(b"tEXt", b"gap\0x"));
    buf.extend_from_slice(&png::chunk(b"IDAT", &z[mid..]));
    buf.extend_from_slice(&png::chunk(b"IEND", &[]));
    let (c, r) = call_load_png(&buf);
    assert_same("err28 non-IDAT chunk between IDATs", &c, &r);
    eprintln!("err28(b): {}", c.head());

    // A declared length that sign-extends negative in cp_chunk's `int offset`.
    for declared in [0x8000_0000u32, 0xFFFF_FFFF, 0x7FFF_FFF5] {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(w as u32, h as u32, 8, 0)));
        buf.extend_from_slice(&png::chunk(b"IDAT", &z[..mid]));
        buf.extend_from_slice(&png::chunk_raw_len(b"IDAT", declared, &z[mid..]));
        buf.extend_from_slice(&png::chunk(b"IEND", &[]));
        let (c, r) = call_load_png(&buf);
        assert_same(&format!("err28 declared=0x{declared:08x}"), &c, &r);
    }
}

#[test]
fn err29_cp_find_walks_off_the_end() {
    let mut rng = Rng::new(SEED ^ 0x129);
    // A chunk whose declared length pushes cp_find's cursor past `end` so the
    // scan terminates without finding PLTE / tRNS / IDAT.
    for declared in [0u32, 1, 100, 0xFFFF_FFFF, 0x8000_0000] {
        let mut buf = png::SIG.to_vec();
        buf.extend_from_slice(&png::chunk(b"IHDR", &png::ihdr_data(3, 3, 8, 0)));
        buf.extend_from_slice(&png::chunk_raw_len(b"junk", declared, &rng.bytes(8)));
        buf.extend_from_slice(&png::chunk(b"IEND", &[]));
        let (c, r) = call_load_png(&buf);
        assert_same(&format!("err29 junk declared=0x{declared:08x}"), &c, &r);
    }
}

// ===========================================================================
// Assertion rows — SIGABRT parity
// ===========================================================================

#[test]
fn errA6_input_exhausted_immediately() {
    // in_bytes == 0 ⇒ bits_left == 0 ⇒ assert(s->bits_left > 0).
    for shift in 0..4usize {
        let (c, r) = call_inflate_cfg(&[0u8; 16], 0, 64, shift, |_| {});
        assert_eq!(c.signal, Some(SIGABRT), "expected SIGABRT (shift={shift})");
        assert_same(&format!("errA6 in_bytes=0 shift={shift}"), &c, &r);
    }
    // Negative in_bytes ⇒ bits_left negative ⇒ same assert.
    for ib in [-1i32, -7, -1000, i32::MIN] {
        let (c, r) = call_inflate(&[0u8; 16], ib, 64);
        assert_eq!(c.signal, Some(SIGABRT), "expected SIGABRT (in_bytes={ib})");
        assert_same(&format!("errA6 in_bytes={ib}"), &c, &r);
    }
}

#[test]
fn errA3_A8_truncated_streams() {
    // Truncating a well-formed stream drives cp_would_overflow /
    // cp_consume_bits / cp_read_bits into their assertions (or into the
    // out-of-bounds error returns). Whatever happens, it must happen the same.
    let mut rng = Rng::new(SEED ^ 0x1A3);
    let mut aborts = 0;
    let mut total = 0;
    let mut timeouts = 0;
    let mut which: std::collections::BTreeMap<String, usize> = Default::default();
    for _ in 0..60 {
        let n = rng.range(4, 60) as usize;
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let def = deflate::fixed_stream(&toks);
        for cut in 1..def.len().min(14) {
            let (c, r) = call_inflate(&def[..cut], cut as i32, n as i32);
            assert_same(&format!("errA3/A8 truncated to {cut} of {}", def.len()), &c, &r);
            total += 1;
            if c.signal == Some(SIGABRT) {
                aborts += 1;
                let s = c.stderr_str();
                if let Some(i) = s.find("Assertion") {
                    let head = s[..i].rsplit(':').nth(1).unwrap_or("?").trim().to_string();
                    *which.entry(head).or_default() += 1;
                }
            }
            if c.signal == Some(14) {
                timeouts += 1;
            }
        }
    }
    eprintln!(
        "errA3/A8: {aborts}/{total} truncated streams hit an assertion \
         ({timeouts} timed out); assertions reached: {which:?}"
    );
    assert!(aborts > 0, "no assertion path reached ({total} cases)");
}

#[test]
fn errA4_extra_bits_table_out_of_range() {
    // cp_read_bits asserts num_bits_to_read <= 32; the exported
    // cp_len_extra_bits / cp_dist_extra_bits tables can supply larger values.
    let history: Vec<u8> = (0..100u32).map(|i| i as u8).collect();
    let mut toks: Vec<Tok> = history.iter().map(|&b| Tok::Lit(b)).collect();
    toks.push(Tok::Match { len: 20, dist: 5 });
    let def = deflate::fixed_stream(&toks);

    for bad in [33u8, 64, 100, 255] {
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, 4096, 0, move |lib| unsafe {
            for i in 0..31 {
                *lib.cp_len_extra_bits.add(i) = bad;
            }
        });
        let label = format!("errA4 cp_len_extra_bits={bad}");
        assert_same(&label, &c, &r);
        assert_eq!(c.signal, Some(SIGABRT), "[{label}] expected SIGABRT");

        let (c, r) = call_inflate_cfg(&def, def.len() as i32, 4096, 0, move |lib| unsafe {
            for i in 0..32 {
                *lib.cp_dist_extra_bits.add(i) = bad;
            }
        });
        let label = format!("errA4 cp_dist_extra_bits={bad}");
        assert_same(&label, &c, &r);
        assert_eq!(c.signal, Some(SIGABRT), "[{label}] expected SIGABRT");
    }
}

#[test]
fn errA9_fixed_table_code_length_too_long() {
    // cp_build asserts len < 16. Writing 16..=255 into cp_fixed_table before a
    // btype==1 block trips it. (The C also does `counts[lens[n]]++` *before* the
    // assert, which writes outside its 16-entry stack array — the observable
    // outcome is still the abort.)
    let toks = vec![Tok::Lit(1), Tok::Lit(2)];
    let def = deflate::fixed_stream(&toks);
    for bad in [16u8, 17, 31, 100, 255] {
        for idx in [0usize, 1, 143, 287, 288, 319] {
            let (c, r) = call_inflate_cfg(&def, def.len() as i32, 64, 0, move |lib| unsafe {
                *lib.cp_fixed_table.add(idx) = bad;
            });
            let label = format!("errA9 cp_fixed_table[{idx}]={bad}");
            assert_same(&label, &c, &r);
            assert!(
                c.signal.is_some(),
                "[{label}] expected the child to die, got {}",
                c.head()
            );
        }
    }
}

#[test]
fn errA10_incomplete_huffman_table() {
    // An under-subscribed lit/len code leaves bit patterns that match no code;
    // cp_decode's assert((search >> len) == (key >> len)) then fires (or, for
    // some patterns, a wrong symbol is decoded — both must match).
    let mut rng = Rng::new(SEED ^ 0x1AA);
    let mut aborts = 0;
    let mut total = 0;
    for trial in 0..40 {
        // Hand-built incomplete code: two symbols at length 3 (Kraft 1/4).
        let mut ll = vec![0u8; 288];
        ll[0] = 3;
        ll[256] = 3;
        let dl = vec![0u8; 2];
        let pn = rng.range(2, 20) as usize;
        let payload = rng.bytes(pn);
        let mut bw = deflate::BitWriter::new();
        bw.bits(1, 1);
        bw.bits(2, 2);
        bw.bits((ll.len() - 257) as u32, 5);
        bw.bits((dl.len() - 1) as u32, 5);
        // Code-length alphabet: symbols 0 and 3, plus RLE 17/18 for the zeros.
        let mut cfreq = [0u32; 19];
        cfreq[0] = 4;
        cfreq[3] = 2;
        cfreq[18] = 4;
        cfreq[17] = 2;
        let clens = deflate::huff_lengths(&cfreq, 7);
        let ccodes = deflate::canonical(&clens);
        let mut hclen = 19usize;
        while hclen > 4 && clens[deflate::CLEN_ORDER[hclen - 1]] == 0 {
            hclen -= 1;
        }
        bw.bits((hclen - 4) as u32, 4);
        for i in 0..hclen {
            bw.bits(clens[deflate::CLEN_ORDER[i]] as u32, 3);
        }
        let mut all = ll.clone();
        all.extend_from_slice(&dl);
        for e in deflate::rle_code_lengths(&all, true) {
            let s = e.sym as usize;
            assert!(clens[s] != 0, "clen symbol {s} unavailable");
            bw.huff(ccodes[s], clens[s] as u32);
            if e.extra_bits > 0 {
                bw.bits(e.extra, e.extra_bits);
            }
        }
        // Raw payload bits — most will not match any of the two codes.
        for &b in &payload {
            bw.bits(b as u32, 8);
        }
        let def = bw.finish();
        let (c, r) = call_inflate(&def, def.len() as i32, 256);
        assert_same(&format!("errA10 incomplete table trial={trial}"), &c, &r);
        total += 1;
        if c.signal == Some(SIGABRT) {
            aborts += 1;
        }
    }
    eprintln!("errA10: {aborts}/{total} incomplete-table cases aborted");
    assert!(aborts > 0, "no cp_decode assertion reached");
}

#[test]
fn errA1_stored_block_at_unaligned_bits_left() {
    // cp_ptr asserts `!(s->bits_left & 7)`.
    //
    // Reaching it needs the *final partial word* to be pulled into the bit
    // buffer at a consumed-bit position `C0` that is not a multiple of 8: from
    // then on `bits_left ≡ C0 (mod 8)`, because cp_peak_bits adds `bits_left`
    // (not a whole number of bytes) to `count`, which breaks the invariant that
    // cp_stored's `cp_read_bits(s, s->count & 7)` relies on. cp_stored must then
    // get past `LEN == (uint16_t)~NLEN` and `bits_left/8 <= LEN`.
    //
    // The search below drives a non-final fixed block (so the final word is
    // consumed mid-symbol) followed by a stored header, over a family of tail
    // patterns chosen so that the LEN field reads as all-ones while NLEN's real
    // bits are zeros — which makes `LEN == ~NLEN` hold for any split.
    let mut agreed = 0usize;
    let mut aborts = 0usize;
    let mut cp_ptr_hits = 0usize;
    let mut example = String::new();

    'outer: for k in 1..16usize {
        for pad in 0..8u32 {
            for ones in 1..7usize {
                for zeros in 0..7usize {
                    let mut bw = deflate::BitWriter::new();
                    let toks: Vec<Tok> = (0..k).map(|i| Tok::Lit((i * 37) as u8)).collect();
                    deflate::fixed_block(&mut bw, &toks, false);
                    // Stored block header, then `pad` extra bits to slide the
                    // C's own alignment fix-up around.
                    bw.bits(1, 1);
                    bw.bits(0, 2);
                    if pad > 0 {
                        bw.bits(0, pad);
                    }
                    let mut def = bw.finish();
                    def.extend(std::iter::repeat(0xFFu8).take(ones));
                    def.extend(std::iter::repeat(0x00u8).take(zeros));
                    // last_bytes must be 3 for the final-word window to be wide
                    // enough (see the derivation above).
                    while def.len() % 4 != 3 {
                        def.push(0x00);
                    }
                    let (c, r) = call_inflate(&def, def.len() as i32, 4096);
                    assert_same(
                        &format!("errA1 k={k} pad={pad} ones={ones} zeros={zeros}"),
                        &c,
                        &r,
                    );
                    agreed += 1;
                    if c.signal == Some(SIGABRT) {
                        aborts += 1;
                        if c.stderr_str().contains("cp_ptr") {
                            cp_ptr_hits += 1;
                            if example.is_empty() {
                                example = format!(
                                    "k={k} pad={pad} ones={ones} zeros={zeros} in_bytes={} :: {}",
                                    def.len(),
                                    c.stderr_str().trim_end()
                                );
                            }
                            if cp_ptr_hits >= 3 {
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "errA1: {agreed} cases agreed, {aborts} aborted, {cp_ptr_hits} reached cp_ptr's assertion"
    );
    eprintln!("errA1: first hit: {example}");
    assert!(
        cp_ptr_hits > 0,
        "cp_ptr's alignment assertion was never reached ({agreed} cases)"
    );
}

// ===========================================================================
// Generic FFI boundary rows
// ===========================================================================

#[test]
fn errG1_G2_G3_png_length_edge_cases() {
    let mut rng = Rng::new(SEED ^ 0x1F1);
    let good = base_png(&mut rng, 4, 3, ColorType::Grey).build();
    // Zero and negative png_length: the C never bounds-checks against it before
    // reading, so the buffer contents decide. Both must agree.
    for len in [0i32, -1, -8, -1000, i32::MIN, 1, 7, 8] {
        let (c, r) = call_load_png_len(&good, len);
        assert_same(&format!("errG1/G2 png_length={len}"), &c, &r);
    }
    // Truncated at every chunk boundary.
    for cut in (8..good.len()).step_by(3) {
        let (c, r) = call_load_png_len(&good, cut as i32);
        assert_same(&format!("errG3 png_length={cut}"), &c, &r);
    }
    // Truncated buffer (not just a short length).
    for cut in 8..good.len() {
        let (c, r) = call_load_png(&good[..cut]);
        assert_same(&format!("errG3 truncated buffer to {cut}"), &c, &r);
    }
}

#[test]
fn errG4_G5_out_bytes_edge_cases() {
    let mut rng = Rng::new(SEED ^ 0x1F4);
    let data = rng.bytes(20);
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let def = deflate::fixed_stream(&toks);
    for ob in [0i32, 1, 19, 20, 21, -1, -20, i32::MIN, i32::MAX] {
        let (c, r) = call_inflate(&def, def.len() as i32, ob);
        assert_same(&format!("errG4/G5 out_bytes={ob}"), &c, &r);
    }
}

#[test]
fn errG13_null_pointers() {
    let libs = libs();
    // load_png_mem(NULL, n) — memcmp against the signature dereferences it.
    let (c, r) = run_pair(|lib, shm| unsafe {
        let img = (lib.load_png_mem)(std::ptr::null(), 64);
        (*shm).w = img.w;
        (*shm).h = img.h;
        (*shm).pix_null = img.pix.is_null() as i32;
    });
    assert_same("errG13 load_png_mem(NULL, 64)", &c, &r);
    assert_eq!(c.signal, Some(SIGSEGV), "expected SIGSEGV, got {}", c.head());

    // cp_inflate(NULL, n, out, m)
    let (c, r) = run_pair(|lib, shm| unsafe {
        let out = libc::malloc(64) as *mut u8;
        libc::memset(out as *mut std::ffi::c_void, 0xAA, 64);
        let ret = (lib.cp_inflate)(std::ptr::null_mut(), 16, out as *mut std::ffi::c_void, 64);
        (*shm).ret = ret as i64;
        set_payload(shm, out, 64);
    });
    assert_same("errG13 cp_inflate(in=NULL)", &c, &r);
    assert_eq!(c.signal, Some(SIGSEGV), "expected SIGSEGV, got {}", c.head());

    // cp_inflate(in, n, NULL, m) — the literal write dereferences out.
    let def = deflate::fixed_stream(&[Tok::Lit(7), Tok::Lit(9)]);
    let d = def.clone();
    let (c, r) = run_pair(move |lib, shm| unsafe {
        let inp = libc::malloc(d.len() + 8) as *mut u8;
        std::ptr::copy_nonoverlapping(d.as_ptr(), inp, d.len());
        let ret = (lib.cp_inflate)(
            inp as *mut std::ffi::c_void,
            d.len() as std::ffi::c_int,
            std::ptr::null_mut(),
            64,
        );
        (*shm).ret = ret as i64;
    });
    assert_same("errG13 cp_inflate(out=NULL)", &c, &r);
    assert_eq!(c.signal, Some(SIGSEGV), "expected SIGSEGV, got {}", c.head());
    let _ = libs;
}

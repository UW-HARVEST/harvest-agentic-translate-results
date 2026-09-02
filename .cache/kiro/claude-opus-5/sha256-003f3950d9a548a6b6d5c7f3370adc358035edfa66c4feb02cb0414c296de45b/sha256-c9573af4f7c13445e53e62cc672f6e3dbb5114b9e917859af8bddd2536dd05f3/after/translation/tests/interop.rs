//! Cross-implementation interoperability differential tests. CONFIGS.md rows 161-166.
//!
//! Verifies that the C and Rust `.so` builds produce MUTUALLY DECODABLE output:
//! data compressed by one implementation must decompress correctly with the
//! other, recovering the ORIGINAL bytes exactly. Every call goes through the
//! `.so` exports of both libraries — Rust functions are never called directly.

mod common;
use common::*;
use libloading::Symbol;

// ============================================================ block signatures

type FnCompressDefault = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnCompressFast = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnCompressHC = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnDecompressSafe = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnDecompressSafePartial = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;

// ============================================================ stream signatures

type FnCreateStream = unsafe extern "C" fn() -> *mut u8;
type FnFreeStream = unsafe extern "C" fn(*mut u8) -> i32;
type FnLoadDict = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnCompressFastContinue =
    unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnCreateStreamDecode = unsafe extern "C" fn() -> *mut u8;
type FnFreeStreamDecode = unsafe extern "C" fn(*mut u8) -> i32;
type FnSetStreamDecode = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnDecompressSafeContinue =
    unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;

// ============================================================= frame ABI structs

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FrameInfo {
    pub block_size_id: i32,
    pub block_mode: i32,
    pub content_checksum_flag: i32,
    pub frame_type: i32,
    pub content_size: u64,
    pub dict_id: u32,
    pub block_checksum_flag: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Prefs {
    pub frame_info: FrameInfo,
    pub compression_level: i32,
    pub auto_flush: u32,
    pub favor_dec_speed: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CompressOptions {
    pub stable_src: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DecompressOptions {
    pub stable_dst: u32,
    pub skip_checksums: u32,
    pub reserved1: u32,
    pub reserved0: u32,
}

// ============================================================= frame signatures

type FnCompressFrame =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const Prefs) -> usize;
type FnCompressFrameBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;
type FnCreateCctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
type FnFreeCctx = unsafe extern "C" fn(*mut u8) -> usize;
type FnBeginUsingDict =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, usize, *const Prefs) -> usize;
type FnCompressUpdate =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, usize, *const CompressOptions) -> usize;
type FnCompressEnd =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const CompressOptions) -> usize;
type FnCompressBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;
type FnCreateDctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
type FnFreeDctx = unsafe extern "C" fn(*mut u8) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const DecompressOptions,
) -> usize;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const u8,
    usize,
    *const DecompressOptions,
) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> u32;

const LZ4F_VERSION: u32 = 100;

// =============================================================== block interop

/// Run block-level cross interop for one direction.
///
/// `swap == false` -> compress with C, decompress with Rust.
/// `swap == true`  -> compress with Rust, decompress with C.
fn block_cross(swap: bool) {
    let (c_def, r_def) = sym::<FnCompressDefault>("LZ4_compress_default");
    let (c_fast, r_fast) = sym::<FnCompressFast>("LZ4_compress_fast");
    let (c_dec, r_dec) = sym::<FnDecompressSafe>("LZ4_decompress_safe");
    let (c_part, r_part) = sym::<FnDecompressSafePartial>("LZ4_decompress_safe_partial");

    let comp_def = if swap { &r_def } else { &c_def };
    let comp_fast = if swap { &r_fast } else { &c_fast };
    let dec = if swap { &c_dec } else { &r_dec };
    let dec_part = if swap { &c_part } else { &r_part };

    let mut rng = Rng::new(0xB10C_C205_u64 ^ (swap as u64));
    let accels = [1i32, 0, -1, 2, 17, 100, 65537];

    for &shape in SHAPES.iter() {
        let mut sizes: Vec<usize> = BOUNDARY_SIZES.to_vec();
        for _ in 0..8 {
            sizes.push(rng.range(1, 200_000));
        }

        for &len in sizes.iter() {
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32).max(1);
            let mut comp = vec![0u8; bound as usize];

            let mut variants: Vec<(&str, i32, bool)> = vec![("default", 0, false)];
            for &a in accels.iter() {
                variants.push(("fast", a, true));
            }

            for (label, accel, is_fast) in variants {
                let clen = unsafe {
                    if is_fast {
                        comp_fast(src.as_ptr(), comp.as_mut_ptr(), len as i32, bound, accel)
                    } else {
                        comp_def(src.as_ptr(), comp.as_mut_ptr(), len as i32, bound)
                    }
                };
                assert!(
                    clen > 0 || len == 0,
                    "block_cross swap={swap} shape={shape:?} len={len} {label} accel={accel}: compress returned {clen}"
                );
                let clen = clen.max(0);

                let mut out = vec![0u8; len.max(1)];
                let dlen =
                    unsafe { dec(comp.as_ptr(), out.as_mut_ptr(), clen, out.len() as i32) };
                assert_eq!(
                    dlen, len as i32,
                    "block_cross swap={swap} shape={shape:?} len={len} {label} accel={accel}: decompress length mismatch"
                );
                eq_bytes(
                    &format!(
                        "block_cross swap={swap} shape={shape:?} len={len} {label} accel={accel}"
                    ),
                    &src,
                    &out[..len],
                );

                if len > 0 {
                    let targets = [0usize, 1, len / 3, len / 2, len.saturating_sub(1), len];
                    for &tgt in targets.iter() {
                        let cap = tgt.max(1);
                        let mut pout = vec![0u8; cap];
                        let pn = unsafe {
                            dec_part(
                                comp.as_ptr(),
                                pout.as_mut_ptr(),
                                clen,
                                tgt as i32,
                                cap as i32,
                            )
                        };
                        assert!(
                            pn >= 0,
                            "block_cross partial swap={swap} shape={shape:?} len={len} tgt={tgt}: returned {pn}"
                        );
                        let pn = pn as usize;
                        let expect = tgt.min(len);
                        assert!(
                            pn >= expect,
                            "block_cross partial swap={swap} shape={shape:?} len={len} tgt={tgt}: wrote {pn} < expected prefix {expect}"
                        );
                        eq_bytes(
                            &format!(
                                "block_cross partial swap={swap} shape={shape:?} len={len} tgt={tgt}"
                            ),
                            &src[..expect],
                            &pout[..expect],
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn block_c_compress_rust_decompress() {
    block_cross(false);
}

#[test]
fn block_rust_compress_c_decompress() {
    block_cross(true);
}

// ================================================================== hc interop

fn hc_cross(swap: bool) {
    let (c_hc, r_hc) = sym::<FnCompressHC>("LZ4_compress_HC");
    let (c_dec, r_dec) = sym::<FnDecompressSafe>("LZ4_decompress_safe");

    let comp = if swap { &r_hc } else { &c_hc };
    let dec = if swap { &c_dec } else { &r_dec };

    let mut rng = Rng::new(0x4C50_0001_u64 ^ (swap as u64));
    let levels = [-5i32, 0, 1, 2, 3, 6, 9, 10, 11, 12, 13];

    for &shape in SHAPES.iter() {
        let mut sizes: Vec<usize> = BOUNDARY_SIZES.to_vec();
        for _ in 0..6 {
            sizes.push(rng.range(1, 120_000));
        }

        for &len in sizes.iter() {
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32).max(1);
            let mut cbuf = vec![0u8; bound as usize];

            for &level in levels.iter() {
                let clen =
                    unsafe { comp(src.as_ptr(), cbuf.as_mut_ptr(), len as i32, bound, level) };
                assert!(
                    clen > 0 || len == 0,
                    "hc_cross swap={swap} shape={shape:?} len={len} level={level}: compress returned {clen}"
                );
                let clen = clen.max(0);

                let mut out = vec![0u8; len.max(1)];
                let dlen =
                    unsafe { dec(cbuf.as_ptr(), out.as_mut_ptr(), clen, out.len() as i32) };
                assert_eq!(
                    dlen, len as i32,
                    "hc_cross swap={swap} shape={shape:?} len={len} level={level}: decompress length mismatch"
                );
                eq_bytes(
                    &format!("hc_cross swap={swap} shape={shape:?} len={len} level={level}"),
                    &src,
                    &out[..len],
                );
            }
        }
    }
}

#[test]
fn hc_c_compress_rust_decompress() {
    hc_cross(false);
}

#[test]
fn hc_rust_compress_c_decompress() {
    hc_cross(true);
}

// ============================================================ streaming interop

/// Compress a multi-chunk blockLinked stream with implementation A, then
/// decompress the blocks with implementation B into one linear output buffer.
///
/// `a_is_c == true`  -> A=C compresses, B=Rust decompresses.
/// `a_is_c == false` -> A=Rust compresses, B=C decompresses.
fn stream_cross_dir(a_is_c: bool, rng: &mut Rng, shape: Shape, total: usize, dict_size: usize) {
    let (c_create, r_create) = sym::<FnCreateStream>("LZ4_createStream");
    let (c_free, r_free) = sym::<FnFreeStream>("LZ4_freeStream");
    let (c_load, r_load) = sym::<FnLoadDict>("LZ4_loadDict");
    let (c_cont, r_cont) = sym::<FnCompressFastContinue>("LZ4_compress_fast_continue");
    let (c_dcreate, r_dcreate) = sym::<FnCreateStreamDecode>("LZ4_createStreamDecode");
    let (c_dfree, r_dfree) = sym::<FnFreeStreamDecode>("LZ4_freeStreamDecode");
    let (c_dset, r_dset) = sym::<FnSetStreamDecode>("LZ4_setStreamDecode");
    let (c_dcont, r_dcont) = sym::<FnDecompressSafeContinue>("LZ4_decompress_safe_continue");

    let a_create = if a_is_c { &c_create } else { &r_create };
    let a_free = if a_is_c { &c_free } else { &r_free };
    let a_load = if a_is_c { &c_load } else { &r_load };
    let a_cont = if a_is_c { &c_cont } else { &r_cont };
    let b_dcreate = if a_is_c { &r_dcreate } else { &c_dcreate };
    let b_dfree = if a_is_c { &r_dfree } else { &c_dfree };
    let b_dset = if a_is_c { &r_dset } else { &c_dset };
    let b_dcont = if a_is_c { &r_dcont } else { &c_dcont };

    // Full original data lives in one contiguous buffer so blockLinked
    // back-references resolve within it.
    let src = make_data(rng, total, shape);
    let dict = make_data(rng, dict_size, shape);

    let mut chunk_bounds: Vec<usize> = Vec::new();
    let mut pos = 0usize;
    while pos < total {
        let step = rng.range(1, 8000).min(total - pos);
        pos += step;
        chunk_bounds.push(pos);
    }
    if chunk_bounds.is_empty() {
        chunk_bounds.push(0);
    }

    // ---- compress side ----
    let stream = unsafe { a_create() };
    assert!(!stream.is_null(), "createStream null");
    if dict_size > 0 {
        unsafe { a_load(stream, dict.as_ptr(), dict_size as i32) };
    }

    // Each compressed block prefixed with 4-byte LE clen + 4-byte LE raw len.
    let mut wire: Vec<u8> = Vec::new();
    let mut start = 0usize;
    for &end in chunk_bounds.iter() {
        let raw = (end - start) as i32;
        let bound = lz4_compress_bound(raw).max(1);
        let mut cbuf = vec![0u8; bound as usize];
        let clen = unsafe {
            a_cont(
                stream,
                src.as_ptr().add(start),
                cbuf.as_mut_ptr(),
                raw,
                bound,
                1,
            )
        };
        assert!(
            clen > 0 || raw == 0,
            "stream_cross a_is_c={a_is_c} shape={shape:?} total={total} dict={dict_size}: compress_fast_continue returned {clen}"
        );
        let clen = clen.max(0) as usize;
        wire.extend_from_slice(&(clen as u32).to_le_bytes());
        wire.extend_from_slice(&(raw as u32).to_le_bytes());
        wire.extend_from_slice(&cbuf[..clen]);
        start = end;
    }
    unsafe { a_free(stream) };

    // ---- decompress side (into ONE linear buffer) ----
    let dstream = unsafe { b_dcreate() };
    assert!(!dstream.is_null(), "createStreamDecode null");
    if dict_size > 0 {
        let ok = unsafe { b_dset(dstream, dict.as_ptr(), dict_size as i32) };
        assert_eq!(ok, 1, "setStreamDecode failed");
    } else {
        let ok = unsafe { b_dset(dstream, std::ptr::null(), 0) };
        assert_eq!(ok, 1, "setStreamDecode(null) failed");
    }

    let mut out = vec![0u8; total.max(1)];
    let mut out_off = 0usize;
    let mut wpos = 0usize;
    while wpos < wire.len() {
        let clen = u32::from_le_bytes(wire[wpos..wpos + 4].try_into().unwrap()) as usize;
        wpos += 4;
        let raw = u32::from_le_bytes(wire[wpos..wpos + 4].try_into().unwrap()) as usize;
        wpos += 4;
        let block = &wire[wpos..wpos + clen];
        wpos += clen;

        let written = unsafe {
            b_dcont(
                dstream,
                block.as_ptr(),
                out.as_mut_ptr().add(out_off),
                clen as i32,
                (out.len() - out_off) as i32,
            )
        };
        assert!(
            written >= 0,
            "stream_cross a_is_c={a_is_c} shape={shape:?} total={total} dict={dict_size}: decompress_safe_continue returned {written}"
        );
        assert_eq!(
            written as usize, raw,
            "stream_cross a_is_c={a_is_c} shape={shape:?} total={total} dict={dict_size}: block decoded {written} != expected {raw}"
        );
        out_off += written as usize;
    }
    unsafe { b_dfree(dstream) };

    assert_eq!(
        out_off, total,
        "stream_cross a_is_c={a_is_c} shape={shape:?} total={total} dict={dict_size}: total decoded {out_off} != {total}"
    );
    eq_bytes(
        &format!("stream_cross a_is_c={a_is_c} shape={shape:?} total={total} dict={dict_size}"),
        &src,
        &out[..total],
    );
}

#[test]
fn streaming_dict_cross() {
    let dict_sizes = [0usize, 1000, 65536];
    let totals = [0usize, 100, 5000, 40_000, 130_000];

    let mut rng = Rng::new(0x57EA_D001);
    for &dict in dict_sizes.iter() {
        for &shape in SHAPES.iter() {
            for &total in totals.iter() {
                stream_cross_dir(true, &mut rng, shape, total, dict);
            }
        }
    }
    let mut rng = Rng::new(0x57EA_D002);
    for &dict in dict_sizes.iter() {
        for &shape in SHAPES.iter() {
            for &total in totals.iter() {
                stream_cross_dir(false, &mut rng, shape, total, dict);
            }
        }
    }
}

// ================================================================ frame interop

/// One implementation's frame decompress-context symbols.
struct FrameDec {
    is_err: Symbol<'static, FnIsError>,
    create: Symbol<'static, FnCreateDctx>,
    free: Symbol<'static, FnFreeDctx>,
    decompress: Symbol<'static, FnDecompress>,
    decompress_dict: Symbol<'static, FnDecompressUsingDict>,
}

fn frame_dec(is_c: bool) -> FrameDec {
    let (c_err, r_err) = sym::<FnIsError>("LZ4F_isError");
    let (c_cr, r_cr) = sym::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (c_fr, r_fr) = sym::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (c_de, r_de) = sym::<FnDecompress>("LZ4F_decompress");
    let (c_dd, r_dd) = sym::<FnDecompressUsingDict>("LZ4F_decompress_usingDict");
    if is_c {
        FrameDec {
            is_err: c_err,
            create: c_cr,
            free: c_fr,
            decompress: c_de,
            decompress_dict: c_dd,
        }
    } else {
        FrameDec {
            is_err: r_err,
            create: r_cr,
            free: r_fr,
            decompress: r_de,
            decompress_dict: r_dd,
        }
    }
}

/// Decode a full frame in `comp`, looping until the returned hint is 0.
fn frame_decode(dec: &FrameDec, comp: &[u8], expect_len: usize) -> Vec<u8> {
    let mut ctx: *mut u8 = std::ptr::null_mut();
    let r = unsafe { (dec.create)(&mut ctx, LZ4F_VERSION) };
    assert_eq!(unsafe { (dec.is_err)(r) }, 0, "createDecompressionContext error");
    assert!(!ctx.is_null());

    let opts = DecompressOptions::default();
    let mut out = vec![0u8; expect_len.max(1)];
    let mut out_off = 0usize;
    let mut in_off = 0usize;
    loop {
        let mut dst_size = out.len() - out_off;
        let mut src_size = comp.len() - in_off;
        if dst_size == 0 {
            out.resize(out.len() + expect_len.max(64), 0);
            dst_size = out.len() - out_off;
        }
        let hint = unsafe {
            (dec.decompress)(
                ctx,
                out.as_mut_ptr().add(out_off),
                &mut dst_size,
                comp.as_ptr().add(in_off),
                &mut src_size,
                &opts,
            )
        };
        assert_eq!(unsafe { (dec.is_err)(hint) }, 0, "LZ4F_decompress error");
        out_off += dst_size;
        in_off += src_size;
        if hint == 0 {
            break;
        }
        if src_size == 0 && dst_size == 0 {
            break;
        }
    }
    unsafe { (dec.free)(ctx) };
    out.truncate(out_off);
    out
}

/// Decode a dict frame using LZ4F_decompress_usingDict.
fn frame_decode_dict(dec: &FrameDec, comp: &[u8], dict: &[u8], expect_len: usize) -> Vec<u8> {
    let mut ctx: *mut u8 = std::ptr::null_mut();
    let r = unsafe { (dec.create)(&mut ctx, LZ4F_VERSION) };
    assert_eq!(unsafe { (dec.is_err)(r) }, 0, "createDecompressionContext error");
    assert!(!ctx.is_null());

    let opts = DecompressOptions::default();
    let mut out = vec![0u8; expect_len.max(1)];
    let mut out_off = 0usize;
    let mut in_off = 0usize;
    loop {
        let mut dst_size = out.len() - out_off;
        let mut src_size = comp.len() - in_off;
        if dst_size == 0 {
            out.resize(out.len() + expect_len.max(64), 0);
            dst_size = out.len() - out_off;
        }
        let hint = unsafe {
            (dec.decompress_dict)(
                ctx,
                out.as_mut_ptr().add(out_off),
                &mut dst_size,
                comp.as_ptr().add(in_off),
                &mut src_size,
                dict.as_ptr(),
                dict.len(),
                &opts,
            )
        };
        assert_eq!(unsafe { (dec.is_err)(hint) }, 0, "LZ4F_decompress_usingDict error");
        out_off += dst_size;
        in_off += src_size;
        if hint == 0 {
            break;
        }
        if src_size == 0 && dst_size == 0 {
            break;
        }
    }
    unsafe { (dec.free)(ctx) };
    out.truncate(out_off);
    out
}

fn mk_prefs(bid: i32, bm: i32, cc: i32, bc: i32, lvl: i32, af: u32) -> Prefs {
    Prefs {
        frame_info: FrameInfo {
            block_size_id: bid,
            block_mode: bm,
            content_checksum_flag: cc,
            frame_type: 0,
            content_size: 0,
            dict_id: 0,
            block_checksum_flag: bc,
        },
        compression_level: lvl,
        auto_flush: af,
        favor_dec_speed: 0,
        reserved: [0; 3],
    }
}

fn frame_cross(swap: bool) {
    let (c_cf, r_cf) = sym::<FnCompressFrame>("LZ4F_compressFrame");
    let (c_cfb, r_cfb) = sym::<FnCompressFrameBound>("LZ4F_compressFrameBound");
    let (c_err, r_err) = sym::<FnIsError>("LZ4F_isError");

    // swap==false: compress with C, decompress with Rust.
    // swap==true:  compress with Rust, decompress with C.
    let comp = if swap { &r_cf } else { &c_cf };
    let comp_bound = if swap { &r_cfb } else { &c_cfb };
    let comp_err = if swap { &r_err } else { &c_err };
    let dec = frame_dec(swap); // decode with the OTHER impl

    let mut rng = Rng::new(0xF4A3_0001_u64 ^ (swap as u64));

    let block_ids = [0i32, 4, 5, 6, 7];
    let block_modes = [0i32, 1];
    let content_cksums = [0i32, 1];
    let block_cksums = [0i32, 1];
    let levels = [0i32, 1, 3, 9, 12];
    let auto_flushes = [0u32, 1];

    // Sample ~25% of the full grid -> ~200 combos, then append explicit
    // per-axis-extreme combos to guarantee full coverage.
    let mut combos: Vec<(Prefs, bool)> = Vec::new(); // (prefs, declare_content_size)
    for &bid in block_ids.iter() {
        for &bm in block_modes.iter() {
            for &cc in content_cksums.iter() {
                for &bc in block_cksums.iter() {
                    for &lvl in levels.iter() {
                        for &af in auto_flushes.iter() {
                            if rng.below(4) != 0 {
                                continue;
                            }
                            let declare_size = rng.bool();
                            let dict_id = if rng.bool() { rng.next_u32() } else { 0 };
                            let mut p = mk_prefs(bid, bm, cc, bc, lvl, af);
                            p.frame_info.dict_id = dict_id;
                            combos.push((p, declare_size));
                        }
                    }
                }
            }
        }
    }
    for &bid in block_ids.iter() {
        combos.push((mk_prefs(bid, 1, 1, 1, 12, 1), true));
    }
    combos.push((mk_prefs(7, 0, 0, 0, 0, 0), false));
    // A dictID-nonzero explicit combo.
    let mut dz = mk_prefs(0, 0, 1, 0, 3, 0);
    dz.frame_info.dict_id = 0xABCD_1234;
    combos.push((dz, true));

    let lens = [0usize, 1, 100, 4096, 70_000, 260_000];

    for &(prefs_template, declare_size) in combos.iter() {
        let shape = SHAPES[rng.below(SHAPES.len())];
        let len = lens[rng.below(lens.len())];
        let src = make_data(&mut rng, len, shape);

        let mut prefs = prefs_template;
        prefs.frame_info.content_size = if declare_size { len as u64 } else { 0 };

        let bound = unsafe { comp_bound(len, &prefs) };
        assert_eq!(unsafe { comp_err(bound) }, 0, "compressFrameBound error");
        let mut comp_buf = vec![0u8; bound.max(64)];
        let clen = unsafe {
            comp(comp_buf.as_mut_ptr(), comp_buf.len(), src.as_ptr(), len, &prefs)
        };
        assert_eq!(
            unsafe { comp_err(clen) },
            0,
            "frame_cross swap={swap} compressFrame error len={len} prefs={prefs:?}"
        );

        let recovered = frame_decode(&dec, &comp_buf[..clen], len);
        eq_bytes(
            &format!("frame_cross swap={swap} len={len} shape={shape:?} prefs={prefs:?}"),
            &src,
            &recovered,
        );
    }
}

#[test]
fn frame_c_compress_rust_decompress() {
    frame_cross(false);
}

#[test]
fn frame_rust_compress_c_decompress() {
    frame_cross(true);
}

// =========================================================== frame dict interop

fn frame_dict_cross_dir(swap: bool) {
    let (c_cr, r_cr) = sym::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (c_fr, r_fr) = sym::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (c_beg, r_beg) = sym::<FnBeginUsingDict>("LZ4F_compressBegin_usingDict");
    let (c_upd, r_upd) = sym::<FnCompressUpdate>("LZ4F_compressUpdate");
    let (c_end, r_end) = sym::<FnCompressEnd>("LZ4F_compressEnd");
    let (c_cb, r_cb) = sym::<FnCompressBound>("LZ4F_compressBound");
    let (c_err, r_err) = sym::<FnIsError>("LZ4F_isError");

    let create = if swap { &r_cr } else { &c_cr };
    let free = if swap { &r_fr } else { &c_fr };
    let begin = if swap { &r_beg } else { &c_beg };
    let update = if swap { &r_upd } else { &c_upd };
    let end = if swap { &r_end } else { &c_end };
    let cbound = if swap { &r_cb } else { &c_cb };
    let cerr = if swap { &r_err } else { &c_err };
    let dec = frame_dec(swap); // decode on OTHER impl

    let mut rng = Rng::new(0xF4D1_0001_u64 ^ (swap as u64));
    let dict_sizes = [1000usize, 65536, 120_000];
    let lens = [1usize, 200, 4096, 60_000, 130_000];

    for &dsize in dict_sizes.iter() {
        for &shape in SHAPES.iter() {
            for &len in lens.iter() {
                let dict = make_data(&mut rng, dsize, shape);
                let src = make_data(&mut rng, len, shape);

                let prefs = Prefs {
                    frame_info: FrameInfo {
                        block_size_id: 0,
                        block_mode: 0,
                        content_checksum_flag: rng.below(2) as i32,
                        frame_type: 0,
                        content_size: 0,
                        dict_id: 0,
                        block_checksum_flag: rng.below(2) as i32,
                    },
                    compression_level: [0i32, 3, 9, 12][rng.below(4)],
                    auto_flush: 0,
                    favor_dec_speed: 0,
                    reserved: [0; 3],
                };

                let mut ctx: *mut u8 = std::ptr::null_mut();
                let r = unsafe { create(&mut ctx, LZ4F_VERSION) };
                assert_eq!(unsafe { cerr(r) }, 0, "createCompressionContext error");

                let bound = unsafe { cbound(len, &prefs) };
                assert_eq!(unsafe { cerr(bound) }, 0, "compressBound error");
                let mut out = vec![0u8; bound + LZ4F_HEADER_SIZE_MAX + 64];
                let mut total = 0usize;

                let hdr = unsafe {
                    begin(ctx, out.as_mut_ptr(), out.len(), dict.as_ptr(), dsize, &prefs)
                };
                assert_eq!(
                    unsafe { cerr(hdr) },
                    0,
                    "frame_dict_cross swap={swap} compressBegin_usingDict error"
                );
                total += hdr;

                let copts = CompressOptions::default();
                let n = unsafe {
                    update(
                        ctx,
                        out.as_mut_ptr().add(total),
                        out.len() - total,
                        src.as_ptr(),
                        len,
                        &copts,
                    )
                };
                assert_eq!(unsafe { cerr(n) }, 0, "compressUpdate error");
                total += n;

                let e = unsafe {
                    end(ctx, out.as_mut_ptr().add(total), out.len() - total, &copts)
                };
                assert_eq!(unsafe { cerr(e) }, 0, "compressEnd error");
                total += e;

                unsafe { free(ctx) };

                let recovered = frame_decode_dict(&dec, &out[..total], &dict, len);
                eq_bytes(
                    &format!(
                        "frame_dict_cross swap={swap} dsize={dsize} len={len} shape={shape:?}"
                    ),
                    &src,
                    &recovered,
                );
            }
        }
    }
}

#[test]
fn frame_dict_cross() {
    frame_dict_cross_dir(false);
    frame_dict_cross_dir(true);
}

//! Differential tests for the LZ4 FILE api (lz4file.c).
//!
//! Every call goes through the exported symbols of BOTH the C and Rust `.so`
//! via libloading; C is ground truth. We assert byte-identical compressed
//! output and identical return codes between the two implementations.

mod common;
use common::*;

use libloading::Symbol;
use std::ffi::c_void;
use std::path::PathBuf;

// ---------------------------------------------------------------- libc glue
//
// The test binary links libc; declare the stdio functions we need.
#[repr(C)]
struct FileOpaque {
    _private: [u8; 0],
}
type FilePtr = *mut FileOpaque;

unsafe extern "C" {
    fn fopen(path: *const u8, mode: *const u8) -> FilePtr;
    fn fclose(fp: FilePtr) -> i32;
    fn fflush(fp: FilePtr) -> i32;
}

fn c_open(path: &str, mode: &str) -> FilePtr {
    let mut p = path.as_bytes().to_vec();
    p.push(0);
    let mut m = mode.as_bytes().to_vec();
    m.push(0);
    let fp = unsafe { fopen(p.as_ptr(), m.as_ptr()) };
    assert!(!fp.is_null(), "fopen failed for {path} mode {mode}");
    fp
}

// ------------------------------------------------------------ symbol types
//
// LZ4F_errorCode_t is size_t. Handles are opaque pointers.
type WriteOpenFn =
    unsafe extern "C" fn(*mut *mut c_void, FilePtr, *const LZ4FPreferences) -> usize;
type WriteFn = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type WriteCloseFn = unsafe extern "C" fn(*mut c_void) -> usize;
type ReadOpenFn = unsafe extern "C" fn(*mut *mut c_void, FilePtr) -> usize;
type ReadFn = unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> usize;
type ReadCloseFn = unsafe extern "C" fn(*mut c_void) -> usize;
type IsErrorFn = unsafe extern "C" fn(usize) -> u32;
type GetBlockSizeFn = unsafe extern "C" fn(i32) -> usize;

// ------------------------------------------------------------ preferences
//
// Verified against c_src/include/lz4frame.h.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LZ4FFrameInfo {
    block_size_id: i32,
    block_mode: i32,
    content_checksum_flag: i32,
    frame_type: i32,
    content_size: u64,
    dict_id: u32,
    block_checksum_flag: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LZ4FPreferences {
    frame_info: LZ4FFrameInfo,
    compression_level: i32,
    auto_flush: u32,
    favor_dec_speed: u32,
    reserved: [u32; 3],
}

// ---------------------------------------------------------------- helpers

/// Classify a raw return code as an error using the C `LZ4F_isError`.
/// Always assert the C and Rust raw codes are identical before classifying.
fn is_c_error(is_err_c: &Symbol<'static, IsErrorFn>, code: usize) -> bool {
    unsafe { is_err_c(code) != 0 }
}

fn tmp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("lz4_file_api_{tag}_{pid}_{n}.lz4"))
}

/// Compress `payload` into `path` using the given impl's symbols.
/// `prefs` NULL when None. Chunking controls how the payload is split across
/// LZ4F_write calls. Returns the vector of raw return codes:
/// [writeOpen, write*, writeClose].
#[allow(clippy::too_many_arguments)]
fn compress_to_file(
    open: &Symbol<'static, WriteOpenFn>,
    write: &Symbol<'static, WriteFn>,
    close: &Symbol<'static, WriteCloseFn>,
    is_err: &Symbol<'static, IsErrorFn>,
    path: &str,
    payload: &[u8],
    prefs: Option<&LZ4FPreferences>,
    chunks: &[usize],
) -> Vec<usize> {
    let fp = c_open(path, "wb");
    let mut handle: *mut c_void = std::ptr::null_mut();
    let prefs_ptr = prefs.map_or(std::ptr::null(), |p| p as *const LZ4FPreferences);
    let mut codes = Vec::new();

    let ro = unsafe { open(&mut handle, fp, prefs_ptr) };
    codes.push(ro);
    if unsafe { is_err(ro) } != 0 {
        unsafe { fclose(fp) };
        return codes;
    }

    let mut off = 0usize;
    for &c in chunks {
        let n = c.min(payload.len() - off);
        let rc = unsafe { write(handle, payload[off..].as_ptr(), n) };
        codes.push(rc);
        off += n;
        if unsafe { is_err(rc) } != 0 {
            break;
        }
        if off >= payload.len() {
            break;
        }
    }

    let rc_close = unsafe { close(handle) };
    codes.push(rc_close);

    unsafe {
        fflush(fp);
        fclose(fp);
    }
    codes
}

/// Single-write chunking: one big write of the whole payload.
fn one_chunk(len: usize) -> Vec<usize> {
    vec![len]
}

/// Decompress `path` fully using the given impl's symbols, reading in the
/// requested granularity. Returns (decoded_bytes, codes) where codes are the
/// raw return values [readOpen, read*, readClose].
fn decompress_file(
    open: &Symbol<'static, ReadOpenFn>,
    read: &Symbol<'static, ReadFn>,
    close: &Symbol<'static, ReadCloseFn>,
    is_err: &Symbol<'static, IsErrorFn>,
    path: &str,
    read_chunk: usize,
) -> (Vec<u8>, Vec<usize>) {
    let fp = c_open(path, "rb");
    let mut handle: *mut c_void = std::ptr::null_mut();
    let mut codes = Vec::new();

    let ro = unsafe { open(&mut handle, fp) };
    codes.push(ro);
    if unsafe { is_err(ro) } != 0 {
        unsafe { fclose(fp) };
        return (Vec::new(), codes);
    }

    let mut out = Vec::new();
    let chunk = read_chunk.max(1);
    let mut buf = vec![0u8; chunk];
    loop {
        let rc = unsafe { read(handle, buf.as_mut_ptr(), chunk) };
        codes.push(rc);
        if unsafe { is_err(rc) } != 0 {
            break;
        }
        if rc == 0 {
            break;
        }
        out.extend_from_slice(&buf[..rc]);
    }

    let rc_close = unsafe { close(handle) };
    codes.push(rc_close);

    unsafe { fclose(fp) };
    (out, codes)
}

fn read_file(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn rm(path: &str) {
    let _ = std::fs::remove_file(path);
}

// symbol getters (C, Rust) pairs
fn writeopen() -> (Symbol<'static, WriteOpenFn>, Symbol<'static, WriteOpenFn>) {
    sym::<WriteOpenFn>("LZ4F_writeOpen")
}
fn write_() -> (Symbol<'static, WriteFn>, Symbol<'static, WriteFn>) {
    sym::<WriteFn>("LZ4F_write")
}
fn writeclose() -> (Symbol<'static, WriteCloseFn>, Symbol<'static, WriteCloseFn>) {
    sym::<WriteCloseFn>("LZ4F_writeClose")
}
fn readopen() -> (Symbol<'static, ReadOpenFn>, Symbol<'static, ReadOpenFn>) {
    sym::<ReadOpenFn>("LZ4F_readOpen")
}
fn read_() -> (Symbol<'static, ReadFn>, Symbol<'static, ReadFn>) {
    sym::<ReadFn>("LZ4F_read")
}
fn readclose() -> (Symbol<'static, ReadCloseFn>, Symbol<'static, ReadCloseFn>) {
    sym::<ReadCloseFn>("LZ4F_readClose")
}
fn iserror() -> (Symbol<'static, IsErrorFn>, Symbol<'static, IsErrorFn>) {
    sym::<IsErrorFn>("LZ4F_isError")
}
fn getblocksize() -> (Symbol<'static, GetBlockSizeFn>, Symbol<'static, GetBlockSizeFn>) {
    sym::<GetBlockSizeFn>("LZ4F_getBlockSize")
}

/// Assert two code vectors match element-wise, and that the C/Rust
/// error-classification also agrees for each code.
fn assert_codes_match(ctx: &str, is_err_c: &Symbol<'static, IsErrorFn>, c: &[usize], r: &[usize]) {
    eq(&format!("{ctx}: code count"), c.len(), r.len());
    for (i, (cc, rc)) in c.iter().zip(r.iter()).enumerate() {
        eq(&format!("{ctx}: code[{i}]"), *cc, *rc);
        // sanity: both are same raw value so classification is trivially equal,
        // but exercise the C classifier to ensure it's loadable/consistent.
        let _ = is_c_error(is_err_c, *cc);
    }
}

// ==================================================================== tests

#[test]
fn write_then_read_roundtrip_default() {
    let (wo_c, wo_r) = writeopen();
    let (w_c, w_r) = write_();
    let (wc_c, wc_r) = writeclose();
    let (ie_c, _ie_r) = iserror();

    let mut rng = Rng::new(0xF11E_A901);
    let sizes = [0usize, 1, 100, 65535, 65536, 65537, 200_000];

    for &shape in &SHAPES {
        for &size in &sizes {
            let payload = make_data(&mut rng, size, shape);

            let pa = tmp_path("rt_c");
            let pb = tmp_path("rt_r");
            let pas = pa.to_str().unwrap().to_string();
            let pbs = pb.to_str().unwrap().to_string();

            let codes_c = compress_to_file(
                &wo_c, &w_c, &wc_c, &ie_c, &pas, &payload, None, &one_chunk(size),
            );
            let codes_r = compress_to_file(
                &wo_r, &w_r, &wc_r, &ie_c, &pbs, &payload, None, &one_chunk(size),
            );

            let ctx = format!("roundtrip shape={shape:?} size={size}");
            assert_codes_match(&ctx, &ie_c, &codes_c, &codes_r);

            let fa = read_file(&pas);
            let fb = read_file(&pbs);
            eq_bytes(&format!("{ctx}: compressed file"), &fa, &fb);

            rm(&pas);
            rm(&pbs);
        }
    }
}

#[test]
fn write_many_small_chunks() {
    let (wo_c, wo_r) = writeopen();
    let (w_c, w_r) = write_();
    let (wc_c, wc_r) = writeclose();
    let (ie_c, _ie_r) = iserror();

    let mut rng = Rng::new(0x5A5A_1234);
    let sizes = [1usize, 100, 4096, 65536, 65537, 130_000];

    for &shape in &SHAPES {
        for &size in &sizes {
            let payload = make_data(&mut rng, size, shape);

            // chunking: mix of 1-byte and small random chunks
            let mut chunks = Vec::new();
            let mut remaining = size;
            while remaining > 0 {
                let c = if rng.bool() {
                    1
                } else {
                    rng.range(1, 500)
                };
                let c = c.min(remaining);
                chunks.push(c);
                remaining -= c;
            }

            let pa = tmp_path("sc_c");
            let pb = tmp_path("sc_r");
            let pas = pa.to_str().unwrap().to_string();
            let pbs = pb.to_str().unwrap().to_string();

            let codes_c = compress_to_file(
                &wo_c, &w_c, &wc_c, &ie_c, &pas, &payload, None, &chunks,
            );
            let codes_r = compress_to_file(
                &wo_r, &w_r, &wc_r, &ie_c, &pbs, &payload, None, &chunks,
            );

            let ctx = format!("small_chunks shape={shape:?} size={size} nchunks={}", chunks.len());
            assert_codes_match(&ctx, &ie_c, &codes_c, &codes_r);

            let fa = read_file(&pas);
            let fb = read_file(&pbs);
            eq_bytes(&format!("{ctx}: compressed file"), &fa, &fb);

            rm(&pas);
            rm(&pbs);
        }
    }
}

#[test]
fn write_config_matrix() {
    let (wo_c, wo_r) = writeopen();
    let (w_c, w_r) = write_();
    let (wc_c, wc_r) = writeclose();
    let (ie_c, _ie_r) = iserror();

    let mut rng = Rng::new(0xC0FF_EE01);

    // Fixed payload per iteration but re-derived for reproducibility.
    let block_ids = [0i32, 4, 5, 6, 7];
    let block_modes = [0i32, 1];
    let content_ck = [0i32, 1];
    let block_ck = [0i32, 1];
    let clevels = [0i32, 1, 3, 9, 12];

    // Full product = 5*2*2*2*5 = 200 combos. Manageable, keep all.
    let mut count = 0usize;
    for &bid in &block_ids {
        for &bm in &block_modes {
            for &cc in &content_ck {
                for &bc in &block_ck {
                    for &cl in &clevels {
                        // vary payload size/shape deterministically
                        let shape = SHAPES[count % SHAPES.len()];
                        let size = [0usize, 1, 1000, 70_000, 300_000][count % 5];
                        let payload = make_data(&mut rng, size, shape);

                        let prefs = LZ4FPreferences {
                            frame_info: LZ4FFrameInfo {
                                block_size_id: bid,
                                block_mode: bm,
                                content_checksum_flag: cc,
                                frame_type: 0,
                                content_size: 0,
                                dict_id: 0,
                                block_checksum_flag: bc,
                            },
                            compression_level: cl,
                            auto_flush: 0,
                            favor_dec_speed: 0,
                            reserved: [0; 3],
                        };

                        let pa = tmp_path("cm_c");
                        let pb = tmp_path("cm_r");
                        let pas = pa.to_str().unwrap().to_string();
                        let pbs = pb.to_str().unwrap().to_string();

                        let codes_c = compress_to_file(
                            &wo_c, &w_c, &wc_c, &ie_c, &pas, &payload, Some(&prefs),
                            &one_chunk(size),
                        );
                        let codes_r = compress_to_file(
                            &wo_r, &w_r, &wc_r, &ie_c, &pbs, &payload, Some(&prefs),
                            &one_chunk(size),
                        );

                        let ctx = format!(
                            "config bid={bid} bm={bm} cc={cc} bc={bc} cl={cl} size={size} shape={shape:?}"
                        );
                        assert_codes_match(&ctx, &ie_c, &codes_c, &codes_r);

                        let fa = read_file(&pas);
                        let fb = read_file(&pbs);
                        eq_bytes(&format!("{ctx}: compressed file"), &fa, &fb);

                        rm(&pas);
                        rm(&pbs);
                        count += 1;
                    }
                }
            }
        }
    }
    assert_eq!(count, 200);
}

#[test]
fn write_boundary_sizes() {
    let (wo_c, wo_r) = writeopen();
    let (w_c, w_r) = write_();
    let (wc_c, wc_r) = writeclose();
    let (ie_c, _ie_r) = iserror();
    let (gbs_c, gbs_r) = getblocksize();

    // Confirm block sizes via LZ4F_getBlockSize for both libs.
    let expected = [(4i32, 65536usize), (5, 262_144), (6, 1_048_576), (7, 4_194_304)];
    for &(id, want) in &expected {
        let bc = unsafe { gbs_c(id) };
        let br = unsafe { gbs_r(id) };
        eq(&format!("getBlockSize({id})"), bc, br);
        eq(&format!("getBlockSize({id}) value"), bc, want);
    }

    let mut rng = Rng::new(0xB0DA_2026);

    for &(id, bs) in &expected {
        for &size in &[bs - 1, bs, bs + 1] {
            // use a couple of shapes to exercise the boundary
            for &shape in &[Shape::Random, Shape::Zeros, Shape::Mixed] {
                let payload = make_data(&mut rng, size, shape);

                let prefs = LZ4FPreferences {
                    frame_info: LZ4FFrameInfo {
                        block_size_id: id,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let pa = tmp_path("bs_c");
                let pb = tmp_path("bs_r");
                let pas = pa.to_str().unwrap().to_string();
                let pbs = pb.to_str().unwrap().to_string();

                let codes_c = compress_to_file(
                    &wo_c, &w_c, &wc_c, &ie_c, &pas, &payload, Some(&prefs),
                    &one_chunk(size),
                );
                let codes_r = compress_to_file(
                    &wo_r, &w_r, &wc_r, &ie_c, &pbs, &payload, Some(&prefs),
                    &one_chunk(size),
                );

                let ctx = format!("boundary id={id} bs={bs} size={size} shape={shape:?}");
                assert_codes_match(&ctx, &ie_c, &codes_c, &codes_r);

                let fa = read_file(&pas);
                let fb = read_file(&pbs);
                eq_bytes(&format!("{ctx}: compressed file"), &fa, &fb);

                rm(&pas);
                rm(&pbs);
            }
        }
    }
}

#[test]
fn read_granularity() {
    let (wo_c, _wo_r) = writeopen();
    let (w_c, _w_r) = write_();
    let (wc_c, _wc_r) = writeclose();
    let (ro_c, ro_r) = readopen();
    let (rd_c, rd_r) = read_();
    let (rc_c, rc_r) = readclose();
    let (ie_c, _ie_r) = iserror();

    let mut rng = Rng::new(0x00DE_C0DE);
    let sizes = [0usize, 1, 100, 65536, 65537, 200_000];

    for &shape in &SHAPES {
        for &size in &sizes {
            let payload = make_data(&mut rng, size, shape);

            // Produce ONE compressed file using the C lib.
            let src = tmp_path("rg_src");
            let srcs = src.to_str().unwrap().to_string();
            let _ = compress_to_file(
                &wo_c, &w_c, &wc_c, &ie_c, &srcs, &payload, None, &one_chunk(size),
            );

            // Read granularities: 1 byte, small random chunk, one large call.
            let big = (size + 16).max(16);
            let small = rng.range(2, 97);
            for &gran in &[1usize, small, big] {
                let (dec_c, codes_c) =
                    decompress_file(&ro_c, &rd_c, &rc_c, &ie_c, &srcs, gran);
                let (dec_r, codes_r) =
                    decompress_file(&ro_r, &rd_r, &rc_r, &ie_c, &srcs, gran);

                let ctx = format!("read_gran shape={shape:?} size={size} gran={gran}");
                // Core differential property: C and Rust must agree exactly on
                // both the raw return codes and the decoded bytes.
                assert_codes_match(&ctx, &ie_c, &codes_c, &codes_r);
                eq_bytes(&format!("{ctx}: C vs Rust decoded"), &dec_c, &dec_r);
                // C is ground truth: only require full-payload recovery when the
                // C reference itself recovered it. (The lz4 FILE read API cannot
                // open frames smaller than LZ4F_HEADER_SIZE_MAX bytes, so very
                // small payloads legitimately fail readOpen in BOTH libs.)
                if unsafe { ie_c(codes_c[0]) } == 0 {
                    eq_bytes(&format!("{ctx}: C decoded vs original"), &payload, &dec_c);
                    eq_bytes(&format!("{ctx}: Rust decoded vs original"), &payload, &dec_r);
                }
            }

            rm(&srcs);
        }
    }
}

#[test]
fn read_open_close_codes() {
    let (wo_c, wo_r) = writeopen();
    let (w_c, w_r) = write_();
    let (wc_c, wc_r) = writeclose();
    let (ro_c, ro_r) = readopen();
    let (rd_c, rd_r) = read_();
    let (rc_c, rc_r) = readclose();
    let (ie_c, _ie_r) = iserror();

    let mut rng = Rng::new(0x0A0B_0C0D);
    let payload = make_data(&mut rng, 12_345, Shape::Text);

    // Create valid files independently with C and Rust; compare write codes.
    let pa = tmp_path("oc_c");
    let pb = tmp_path("oc_r");
    let pas = pa.to_str().unwrap().to_string();
    let pbs = pb.to_str().unwrap().to_string();

    let wcodes_c = compress_to_file(
        &wo_c, &w_c, &wc_c, &ie_c, &pas, &payload, None, &one_chunk(payload.len()),
    );
    let wcodes_r = compress_to_file(
        &wo_r, &w_r, &wc_r, &ie_c, &pbs, &payload, None, &one_chunk(payload.len()),
    );
    assert_codes_match("writeOpen/write/writeClose codes", &ie_c, &wcodes_c, &wcodes_r);

    // Files must be byte-identical, so reading either yields the same codes.
    let fa = read_file(&pas);
    let fb = read_file(&pbs);
    eq_bytes("valid file bytes", &fa, &fb);

    // readOpen/read/readClose codes for a valid file.
    let (dec_c, rcodes_c) = decompress_file(&ro_c, &rd_c, &rc_c, &ie_c, &pas, 4096);
    let (dec_r, rcodes_r) = decompress_file(&ro_r, &rd_r, &rc_r, &ie_c, &pas, 4096);
    assert_codes_match("readOpen/read/readClose codes", &ie_c, &rcodes_c, &rcodes_r);
    eq_bytes("decoded matches original (C)", &payload, &dec_c);
    eq_bytes("decoded matches original (Rust)", &payload, &dec_r);

    rm(&pas);
    rm(&pbs);
}

#[test]
fn cross_interop() {
    let (wo_c, wo_r) = writeopen();
    let (w_c, w_r) = write_();
    let (wc_c, wc_r) = writeclose();
    let (ro_c, ro_r) = readopen();
    let (rd_c, rd_r) = read_();
    let (rc_c, rc_r) = readclose();
    let (ie_c, _ie_r) = iserror();

    let mut rng = Rng::new(0x1234_ABCD);
    let sizes = [0usize, 1, 100, 65537, 300_000];

    for &shape in &SHAPES {
        for &size in &sizes {
            let payload = make_data(&mut rng, size, shape);

            // File written by C, read by Rust — and also by C as the reference.
            let pc = tmp_path("ci_c");
            let pcs = pc.to_str().unwrap().to_string();
            let _ = compress_to_file(
                &wo_c, &w_c, &wc_c, &ie_c, &pcs, &payload, None, &one_chunk(size),
            );
            let (dec_by_rust, codes_r) =
                decompress_file(&ro_r, &rd_r, &rc_r, &ie_c, &pcs, 4096);
            let (dec_ref_c, codes_refc) =
                decompress_file(&ro_c, &rd_c, &rc_c, &ie_c, &pcs, 4096);
            let ctx = format!("C-written/Rust-read shape={shape:?} size={size}");
            // Rust reading the C file must match C reading the C file exactly.
            assert_codes_match(&ctx, &ie_c, &codes_refc, &codes_r);
            eq_bytes(&format!("{ctx}: C-read vs Rust-read"), &dec_ref_c, &dec_by_rust);
            // Full recovery only when the C reference recovers (tiny frames below
            // LZ4F_HEADER_SIZE_MAX cannot be opened by the FILE read API).
            if unsafe { ie_c(codes_refc[0]) } == 0 {
                eq_bytes(&format!("{ctx}: recovered payload"), &payload, &dec_by_rust);
            }
            rm(&pcs);

            // File written by Rust, read by C — and also by Rust as reference.
            let pr = tmp_path("ci_r");
            let prs = pr.to_str().unwrap().to_string();
            let _ = compress_to_file(
                &wo_r, &w_r, &wc_r, &ie_c, &prs, &payload, None, &one_chunk(size),
            );
            let (dec_by_c, codes_c) =
                decompress_file(&ro_c, &rd_c, &rc_c, &ie_c, &prs, 4096);
            let (dec_ref_r, codes_refr) =
                decompress_file(&ro_r, &rd_r, &rc_r, &ie_c, &prs, 4096);
            let ctx = format!("Rust-written/C-read shape={shape:?} size={size}");
            assert_codes_match(&ctx, &ie_c, &codes_c, &codes_refr);
            eq_bytes(&format!("{ctx}: C-read vs Rust-read"), &dec_by_c, &dec_ref_r);
            if unsafe { ie_c(codes_c[0]) } == 0 {
                eq_bytes(&format!("{ctx}: recovered payload"), &payload, &dec_by_c);
            }
            rm(&prs);
        }
    }
}

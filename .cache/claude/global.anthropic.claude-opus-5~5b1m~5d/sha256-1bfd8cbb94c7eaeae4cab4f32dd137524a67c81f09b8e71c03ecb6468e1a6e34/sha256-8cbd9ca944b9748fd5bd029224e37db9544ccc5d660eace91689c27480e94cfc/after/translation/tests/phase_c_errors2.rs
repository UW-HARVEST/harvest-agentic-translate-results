//! Phase C part 2: decompression-side error paths (ERRORS.md rows 145-271).
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnChunk = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnUsingDDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize;
type FnUsingDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const u8, usize) -> usize;
type FnStream = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> usize;
type FnStream2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;
type FnDBegin = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDBeginDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnNextSize = unsafe extern "C" fn(*mut c_void) -> usize;
type FnNextType = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnLoadDictAdv = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int, c_int) -> usize;
type FnRefDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> usize;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnSetMaxWindow = unsafe extern "C" fn(*mut c_void, usize) -> usize;
type FnSetFormat = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnU64FromBuf = unsafe extern "C" fn(*const u8, usize) -> c_ulonglong;
type FnSizeFromBuf = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnGetcBlockSize = unsafe extern "C" fn(*const u8, usize, *mut BlockProperties) -> usize;
type FnDecodingBufferSizeMin = unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> usize;
type FnInitStaticDCtx = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnEstimateDCtx = unsafe extern "C" fn() -> usize;
type FnInitDStream = unsafe extern "C" fn(*mut c_void) -> usize;
type FnTrain = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, c_uint) -> usize;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockProperties {
    pub blockType: c_int,
    pub lastBlock: c_uint,
    pub origSize: c_uint,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InBuffer {
    pub src: *const u8,
    pub size: usize,
    pub pos: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OutBuffer {
    pub dst: *mut u8,
    pub size: usize,
    pub pos: usize,
}

const C_COMPRESSIONLEVEL: c_int = 100;
const C_CHECKSUMFLAG: c_int = 201;
const C_CONTENTSIZEFLAG: c_int = 200;
const C_WINDOWLOG: c_int = 101;
const C_FORMAT: c_int = 10;
const C_MAXBLOCKSIZE: c_int = 1015;
const D_WINDOWLOGMAX: c_int = 100;
const D_FORMAT: c_int = 1000;
const D_STABLEOUT: c_int = 1001;
const D_IGNORECHECKSUM: c_int = 1002;
const D_REFMULTIPLE: c_int = 1003;
const D_DISABLEHUFASM: c_int = 1004;
const D_MAXBLOCKSIZE: c_int = 1005;
const E_CONTINUE: c_int = 0;
const E_END: c_int = 2;

pub const DPARAMS: &[(c_int, &str)] = &[
    (100, "windowLogMax"),
    (1000, "format"),
    (1001, "stableOutBuffer"),
    (1002, "forceIgnoreChecksum"),
    (1003, "refMultipleDDicts"),
    (1004, "disableHuffmanAssembly"),
    (1005, "maxBlockSize"),
];

// ------------------------------------------------------------------ helpers --

fn compress_both(params: &[(c_int, c_int)], src: &[u8]) -> Vec<u8> {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for &(p, v) in params {
        assert_eq!(unsafe { cs(cc, p, v) }, unsafe { rs(rc, p, v) }, "setParameter({p},{v})");
    }
    let cap = unsafe { cb(src.len()) } + 64;
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let x = unsafe { c2(cc, a.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let y = unsafe { r2(rc, b.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    assert_eq!(x, y, "compress2");
    assert!(!is_error(x));
    assert_bytes_eq("frame", &a[..x], &b[..y]);
    unsafe {
        cf(cc);
        rf(rc);
    }
    a.truncate(x);
    a
}

/// Feed `frame` to EVERY decoder entry point in both libraries and require
/// identical return values (and identical bytes on success).
fn all_decoders_agree(frame: &[u8], out_cap: usize, dparams: &[(c_int, c_int)], ctx: &str) {
    let (cd, rd) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cdd, rdd) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (cns, rns) = unsafe { pair::<FnNextSize>("ZSTD_nextSrcSizeToDecompress") };
    let (cdc, rdc) = unsafe { pair::<FnChunk>("ZSTD_decompressContinue") };
    let (cdblk, rdblk) = unsafe { pair::<FnChunk>("ZSTD_decompressBlock") };

    // 1) one-shot ZSTD_decompress
    let mut a = vec![0u8; out_cap.max(1)];
    let mut b = vec![0u8; out_cap.max(1)];
    let x = unsafe { cd(a.as_mut_ptr(), out_cap, frame.as_ptr(), frame.len()) };
    let y = unsafe { rd(b.as_mut_ptr(), out_cap, frame.as_ptr(), frame.len()) };
    assert_eq!(x, y, "{ctx}: ZSTD_decompress");
    if !is_error(x) {
        assert_bytes_eq(&format!("{ctx}: ZSTD_decompress bytes"), &a[..x], &b[..y]);
    }

    // 2) decompressDCtx with the given dparams
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for &(p, v) in dparams {
        assert_eq!(unsafe { cs(cc, p, v) }, unsafe { rs(rc, p, v) }, "{ctx}: dparam({p},{v})");
    }
    let x = unsafe { cdd(cc, a.as_mut_ptr(), out_cap, frame.as_ptr(), frame.len()) };
    let y = unsafe { rdd(rc, b.as_mut_ptr(), out_cap, frame.as_ptr(), frame.len()) };
    assert_eq!(x, y, "{ctx}: ZSTD_decompressDCtx");
    if !is_error(x) {
        assert_bytes_eq(&format!("{ctx}: decompressDCtx bytes"), &a[..x], &b[..y]);
    }
    unsafe {
        cf(cc);
        rf(rc);
    }

    // 3) streaming, with several input chunk sizes
    for &chunk in &[1usize, 7, 1000, usize::MAX] {
        let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
            for &(p, v) in dparams {
                let sp = if which == 0 { &cs } else { &rs };
                unsafe { sp(z, p, v) };
            }
            let mut obuf = vec![0u8; out_cap.max(1).min(1 << 20)];
            let mut input = InBuffer { src: frame.as_ptr(), size: 0, pos: 0 };
            let mut avail = 0usize;
            let mut collected = Vec::new();
            let mut guard = 0;
            loop {
                guard += 1;
                if guard > 100000 {
                    break;
                }
                if avail < frame.len() {
                    avail = avail.saturating_add(chunk).min(frame.len());
                    input.size = avail;
                }
                let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
                let before = input.pos;
                let rc = if which == 0 {
                    unsafe { cds(z, &mut out, &mut input) }
                } else {
                    unsafe { rds(z, &mut out, &mut input) }
                };
                rets[which].push(rc);
                collected.extend_from_slice(&obuf[..out.pos]);
                if is_error(rc) || rc == 0 {
                    break;
                }
                if input.pos == before && out.pos == 0 && avail == frame.len() {
                    break;
                }
            }
            outs[which] = collected;
            if which == 0 {
                unsafe { cf(z) };
            } else {
                unsafe { rf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "{ctx}: decompressStream(chunk={chunk}) returns");
        assert_bytes_eq(&format!("{ctx}: decompressStream(chunk={chunk}) bytes"), &outs[0],
                        &outs[1]);
    }

    // 4) low-level decompressBegin + decompressContinue
    {
        let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
            for &(p, v) in dparams {
                let sp = if which == 0 { &cs } else { &rs };
                unsafe { sp(z, p, v) };
            }
            let bg = if which == 0 { &cdb } else { &rdb };
            rets[which].push(unsafe { bg(z) });
            let cap = out_cap.max(1 << 18) + (1 << 18);
            let mut obuf = vec![0u8; cap];
            let mut written = 0usize;
            let mut ip = 0usize;
            let mut guard = 0;
            loop {
                guard += 1;
                if guard > 100000 {
                    break;
                }
                let ns = if which == 0 { unsafe { cns(z) } } else { unsafe { rns(z) } };
                rets[which].push(ns);
                if ns == 0 {
                    break;
                }
                let avail = (frame.len() - ip).min(ns);
                if avail == 0 {
                    break;
                }
                let f = if which == 0 { &cdc } else { &rdc };
                let w = unsafe {
                    f(z, obuf[written..].as_mut_ptr(), cap - written, frame[ip..].as_ptr(), avail)
                };
                rets[which].push(w);
                if is_error(w) {
                    break;
                }
                written += w;
                ip += avail;
            }
            obuf.truncate(written);
            outs[which] = obuf;
            if which == 0 {
                unsafe { cf(z) };
            } else {
                unsafe { rf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "{ctx}: decompressContinue trace");
        assert_bytes_eq(&format!("{ctx}: decompressContinue bytes"), &outs[0], &outs[1]);
    }

    // 5) raw ZSTD_decompressBlock on the buffer (treats it as a bare block)
    {
        let cc = unsafe { cn() };
        let rc = unsafe { rn() };
        unsafe {
            cdb(cc);
            rdb(rc);
        }
        let cap = out_cap.max(1 << 17);
        let mut a = vec![0u8; cap];
        let mut b = vec![0u8; cap];
        let x = unsafe { cdblk(cc, a.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
        let y = unsafe { rdblk(rc, b.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
        assert_eq!(x, y, "{ctx}: ZSTD_decompressBlock");
        if !is_error(x) {
            assert_bytes_eq(&format!("{ctx}: decompressBlock bytes"), &a[..x], &b[..y]);
        }
        unsafe {
            cf(cc);
            rf(rc);
        }
    }
}

/// Every distinct malformation of a valid frame.
fn mutations(frame: &[u8], rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    // truncations
    let mut lens: Vec<usize> = (0..=frame.len().min(24)).collect();
    for d in [1usize, 2, 3, 4, 5] {
        if frame.len() > d {
            lens.push(frame.len() - d);
        }
    }
    lens.push(frame.len() / 2);
    lens.push(frame.len() / 4);
    lens.push(frame.len() * 3 / 4);
    lens.sort_unstable();
    lens.dedup();
    for &n in &lens {
        if n <= frame.len() {
            out.push((format!("truncated to {n}"), frame[..n].to_vec()));
        }
    }
    // single-byte mutations at interesting and random offsets
    let mut offs: Vec<usize> = (0..frame.len().min(20)).collect();
    for _ in 0..40 {
        offs.push(rng.below(frame.len().max(1)));
    }
    if frame.len() >= 4 {
        offs.push(frame.len() - 1);
        offs.push(frame.len() - 2);
        offs.push(frame.len() - 3);
        offs.push(frame.len() - 4);
    }
    offs.sort_unstable();
    offs.dedup();
    for &o in &offs {
        if o < frame.len() {
            for delta in [0xFFu8, 0x01, 0x80] {
                let mut f = frame.to_vec();
                f[o] ^= delta;
                out.push((format!("byte {o} ^= {delta:#x}"), f));
            }
        }
    }
    // block-header manipulations: set reserved block type, oversized block size
    if frame.len() > 10 {
        for hdr in [5usize, 6, 7, 8, 9, 10, 13, 14] {
            if hdr + 3 <= frame.len() {
                let mut f = frame.to_vec();
                // block type = 3 (bt_reserved) lives in bits 1..2 of the first header byte
                f[hdr] = (f[hdr] & 0xF8) | ((f[hdr] & 0x01) | 0x06);
                out.push((format!("block type reserved @{hdr}"), f));
                let mut f = frame.to_vec();
                // huge block size field
                f[hdr] |= 0xF8;
                f[hdr + 1] = 0xFF;
                f[hdr + 2] = 0xFF;
                out.push((format!("oversized block size @{hdr}"), f));
            }
        }
    }
    // append / prepend garbage
    let mut f = frame.to_vec();
    f.extend(gen(Shape::Random, 7, rng));
    out.push(("trailing garbage 7".into(), f));
    let mut f = frame.to_vec();
    f.extend(gen(Shape::Random, 40, rng));
    out.push(("trailing garbage 40".into(), f));
    let mut f = frame.to_vec();
    f.extend_from_slice(frame);
    out.push(("frame twice".into(), f));
    let mut f = frame.to_vec();
    f.extend_from_slice(&frame[..frame.len() / 2]);
    out.push(("frame + half frame".into(), f));
    // wrong magic
    if frame.len() >= 4 {
        for magic in [
            0u32,
            0xFD2FB527,
            0xFD2FB525,
            0x184D2A50,
            0x184D2A5F,
            0xFFFFFFFF,
            0xFD2FB529,
        ] {
            let mut f = frame.to_vec();
            f[..4].copy_from_slice(&magic.to_le_bytes());
            out.push((format!("magic := {magic:#x}"), f));
        }
    }
    // windowLog field forced to the maximum
    if frame.len() > 5 {
        let mut f = frame.to_vec();
        f[5] = 0xFF;
        out.push(("windowDescriptor := 0xFF".into(), f));
    }
    // frame header descriptor reserved bit
    if frame.len() > 4 {
        let mut f = frame.to_vec();
        f[4] |= 0x08;
        out.push(("fhd reserved bit".into(), f));
    }
    out
}

fn train_dict(rng: &mut Rng, seed_shape: Shape) -> Vec<u8> {
    let (ct, rt) = unsafe { pair::<FnTrain>("ZDICT_trainFromBuffer") };
    let nb = 300;
    let mut samples = Vec::new();
    let mut sizes = Vec::new();
    for _ in 0..nb {
        let s = gen(seed_shape, rng.range(64, 400), rng);
        sizes.push(s.len());
        samples.extend_from_slice(&s);
    }
    let cap = 8192;
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let x = unsafe { ct(a.as_mut_ptr(), cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint) };
    let y = unsafe { rt(b.as_mut_ptr(), cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint) };
    assert_eq!(x, y);
    assert!(!is_error(x));
    assert_bytes_eq("trained dict", &a[..x], &b[..y]);
    a.truncate(x);
    a
}

// -------------------------------------------------------- rows 145-185 ------

#[test]
fn err_malformed_frames() {
    let mut rng = Rng::new(0xF001);
    for &shape in &[Shape::Text, Shape::Random, Shape::Rle] {
        for &size in &[0usize, 1, 1000, 200000] {
            for &(cksum, csize) in &[(0, 1), (1, 1), (0, 0)] {
                let src = gen(shape, size, &mut rng);
                let frame = compress_both(
                    &[(C_CHECKSUMFLAG, cksum), (C_CONTENTSIZEFLAG, csize),
                      (C_COMPRESSIONLEVEL, 3)],
                    &src,
                );
                for (name, bad) in mutations(&frame, &mut rng) {
                    let ctx =
                        format!("{shape:?}/{size} cksum={cksum} csize={csize}: {name}");
                    all_decoders_agree(&bad, src.len() + 4096, &[(D_WINDOWLOGMAX, 31)], &ctx);
                }
            }
        }
    }
    // pure garbage and tiny buffers
    for n in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 17, 18, 19, 64, 1000] {
        for &shape in &[Shape::Random, Shape::Rle] {
            let buf = gen(shape, n, &mut rng);
            all_decoders_agree(&buf, 65536, &[], &format!("garbage {shape:?} n={n}"));
        }
    }
}

#[test]
fn err_checksum_wrong() {
    let mut rng = Rng::new(0xF002);
    for &size in &[1usize, 1000, 131072] {
        let src = gen(Shape::Text, size, &mut rng);
        let frame = compress_both(&[(C_CHECKSUMFLAG, 1), (C_COMPRESSIONLEVEL, 3)], &src);
        // corrupt each of the 4 checksum bytes, and truncate the checksum
        for i in 1..=4usize {
            let mut f = frame.clone();
            let n = f.len();
            f[n - i] ^= 0xFF;
            all_decoders_agree(&f, size + 64, &[], &format!("checksum byte -{i} n={size}"));
        }
        for cut in 1..=4usize {
            let f = frame[..frame.len() - cut].to_vec();
            all_decoders_agree(&f, size + 64, &[], &format!("checksum truncated -{cut} n={size}"));
        }
    }
}

#[test]
fn err_checksum_ignored() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cd, rd) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let mut rng = Rng::new(0xF003);
    for &size in &[1usize, 1000, 131072] {
        let src = gen(Shape::Text, size, &mut rng);
        let frame = compress_both(&[(C_CHECKSUMFLAG, 1), (C_COMPRESSIONLEVEL, 3)], &src);
        let mut bad = frame.clone();
        let n = bad.len();
        bad[n - 1] ^= 0xFF;
        for &ignore in &[0, 1] {
            let cc = unsafe { cn() };
            let rc = unsafe { rn() };
            assert_eq!(unsafe { cs(cc, D_IGNORECHECKSUM, ignore) },
                       unsafe { rs(rc, D_IGNORECHECKSUM, ignore) });
            let cap = size + 64;
            let mut a = vec![0u8; cap];
            let mut b = vec![0u8; cap];
            let x = unsafe { cd(cc, a.as_mut_ptr(), cap, bad.as_ptr(), bad.len()) };
            let y = unsafe { rd(rc, b.as_mut_ptr(), cap, bad.as_ptr(), bad.len()) };
            assert_eq!(x, y, "forceIgnoreChecksum={ignore} n={size}");
            if !is_error(x) {
                assert_bytes_eq("ignored checksum output", &a[..x], &b[..y]);
            }
            unsafe {
                cf(cc);
                rf(rc);
            }
        }
    }
}

#[test]
fn err_dst_capacity_decompress() {
    let (cd, rd) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let mut rng = Rng::new(0xF004);
    for &shape in &[Shape::Text, Shape::Random, Shape::Rle] {
        for &size in &[0usize, 1, 100, 131072, 200000] {
            let src = gen(shape, size, &mut rng);
            let frame = compress_both(&[(C_COMPRESSIONLEVEL, 3)], &src);
            let mut caps: Vec<usize> = vec![0, 1];
            for d in [1usize, 2, 3, 100] {
                if size >= d {
                    caps.push(size - d);
                }
            }
            caps.push(size);
            caps.push(size + 1);
            caps.sort_unstable();
            caps.dedup();
            for &cap in &caps {
                let mut a = vec![0u8; cap.max(1)];
                let mut b = vec![0u8; cap.max(1)];
                let x = unsafe { cd(a.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
                let y = unsafe { rd(b.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
                assert_eq!(x, y, "decompress(cap={cap}, {shape:?}/{size})");
                if !is_error(x) {
                    assert_bytes_eq("dec bytes", &a[..x], &b[..y]);
                }
            }
        }
    }
}

#[test]
fn err_null_dst_decompress() {
    let (cd, rd) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdd, rdd) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let mut rng = Rng::new(0xF005);
    // raw-block frame (random data), RLE frame, and a compressed frame
    for &shape in &[Shape::Random, Shape::Rle, Shape::Text] {
        for &size in &[0usize, 1, 1000, 131072] {
            let src = gen(shape, size, &mut rng);
            let frame = compress_both(&[(C_COMPRESSIONLEVEL, 1)], &src);
            for &cap in &[0usize, 1] {
                let x = unsafe { cd(std::ptr::null_mut(), cap, frame.as_ptr(), frame.len()) };
                let y = unsafe { rd(std::ptr::null_mut(), cap, frame.as_ptr(), frame.len()) };
                assert_eq!(x, y, "decompress(NULL dst, cap={cap}, {shape:?}/{size})");
                let cc = unsafe { cn() };
                let rc = unsafe { rn() };
                let x = unsafe { cdd(cc, std::ptr::null_mut(), cap, frame.as_ptr(), frame.len()) };
                let y = unsafe { rdd(rc, std::ptr::null_mut(), cap, frame.as_ptr(), frame.len()) };
                assert_eq!(x, y, "decompressDCtx(NULL dst, cap={cap})");
                unsafe {
                    cf(cc);
                    rf(rc);
                }
            }
        }
    }
}

#[test]
fn err_dictionary_wrong() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cld, rld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdld, rdld) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary") };
    let (cdd, rdd) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xF006);

    let dict1 = train_dict(&mut rng, Shape::Text);
    let dict2 = train_dict(&mut rng, Shape::Repetitive);
    let src = gen(Shape::Text, 20000, &mut rng);

    // frame compressed with dict1
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    assert_eq!(unsafe { cld(cc, dict1.as_ptr(), dict1.len()) },
               unsafe { rld(rc, dict1.as_ptr(), dict1.len()) });
    let cap = unsafe { cb(src.len()) } + 64;
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let x = unsafe { c2(cc, a.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let y = unsafe { r2(rc, b.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    assert_eq!(x, y);
    assert_bytes_eq("dict frame", &a[..x], &b[..y]);
    a.truncate(x);
    unsafe {
        cf(cc);
        rf(rc);
    }

    // decode with: no dict, wrong dict, right dict
    for (label, d) in [
        ("no dict", None),
        ("wrong dict", Some(&dict2)),
        ("right dict", Some(&dict1)),
    ] {
        let dc = unsafe { cdn() };
        let dr = unsafe { rdn() };
        if let Some(d) = d {
            assert_eq!(unsafe { cdld(dc, d.as_ptr(), d.len()) },
                       unsafe { rdld(dr, d.as_ptr(), d.len()) });
        }
        let cap = src.len() + 64;
        let mut o1 = vec![0u8; cap];
        let mut o2 = vec![0u8; cap];
        let x = unsafe { cdd(dc, o1.as_mut_ptr(), cap, a.as_ptr(), a.len()) };
        let y = unsafe { rdd(dr, o2.as_mut_ptr(), cap, a.as_ptr(), a.len()) };
        assert_eq!(x, y, "decode dict frame with {label}");
        if !is_error(x) {
            assert_bytes_eq(label, &o1[..x], &o2[..y]);
        }
        unsafe {
            cdf(dc);
            rdf(dr);
        }
    }
    // also mutate the dictID field of the frame header
    for pos in 4..9usize.min(a.len()) {
        let mut f = a.clone();
        f[pos] ^= 0x55;
        all_decoders_agree(&f, src.len() + 64, &[], &format!("dictID mutation @{pos}"));
    }
}

#[test]
fn err_dctx_dict_corrupted() {
    let (cbd, rbd) = unsafe { pair::<FnDBeginDict>("ZSTD_decompressBegin_usingDict") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cld, rld) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let mut rng = Rng::new(0xF007);
    let dict = train_dict(&mut rng, Shape::Text);

    let mut variants: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [0usize, 1, 4, 8, 9, 12, 20, 40, 100, 200] {
        if n <= dict.len() {
            variants.push((format!("truncated {n}"), dict[..n].to_vec()));
        }
    }
    for pos in 0..dict.len().min(120) {
        let mut d = dict.clone();
        d[pos] ^= 0xFF;
        variants.push((format!("byte {pos} flipped"), d));
    }
    for (name, d) in variants {
        let dc = unsafe { cdn() };
        let dr = unsafe { rdn() };
        let x = unsafe { cbd(dc, d.as_ptr(), d.len()) };
        let y = unsafe { rbd(dr, d.as_ptr(), d.len()) };
        assert_eq!(x, y, "decompressBegin_usingDict({name})");
        let x = unsafe { cld(dc, d.as_ptr(), d.len()) };
        let y = unsafe { rld(dr, d.as_ptr(), d.len()) };
        assert_eq!(x, y, "DCtx_loadDictionary({name})");
        unsafe {
            cdf(dc);
            rdf(dr);
        }
        let a = unsafe { cdd(d.as_ptr(), d.len()) };
        let b = unsafe { rdd(d.as_ptr(), d.len()) };
        assert_eq!(a.is_null(), b.is_null(), "createDDict({name})");
        unsafe {
            if !a.is_null() {
                cfd(a);
            }
            if !b.is_null() {
                rfd(b);
            }
        }
    }
}

#[test]
fn err_dctx_loaddict_fulldict() {
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cla, rla) = unsafe { pair::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced") };
    let (crp, rrp) = unsafe { pair::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced") };
    let mut rng = Rng::new(0xF008);
    let raw = gen(Shape::Text, 4096, &mut rng);
    let dict = train_dict(&mut rng, Shape::Text);
    for (label, d) in [("raw", &raw), ("trained", &dict)] {
        for dct in [0, 1, 2, 3, 99, -1] {
            for dlm in [0, 1, 2, -1] {
                let dc = unsafe { cdn() };
                let dr = unsafe { rdn() };
                let x = unsafe { cla(dc, d.as_ptr(), d.len(), dlm, dct) };
                let y = unsafe { rla(dr, d.as_ptr(), d.len(), dlm, dct) };
                assert_eq!(x, y, "DCtx_loadDictionary_advanced({label},dlm={dlm},dct={dct})");
                let x = unsafe { crp(dc, d.as_ptr(), d.len(), dct) };
                let y = unsafe { rrp(dr, d.as_ptr(), d.len(), dct) };
                assert_eq!(x, y, "DCtx_refPrefix_advanced({label},dct={dct})");
                unsafe {
                    cdf(dc);
                    rdf(dr);
                }
            }
        }
    }
}

#[test]
fn err_literals_repeat_no_table() {
    // A block whose literals section claims "set_repeat" (mode 3) without any
    // previously loaded Huffman table -> dictionary_corrupted.
    let mut rng = Rng::new(0xF009);
    let src = gen(Shape::Text, 5000, &mut rng);
    let frame = compress_both(&[(C_COMPRESSIONLEVEL, 3), (C_CONTENTSIZEFLAG, 1)], &src);
    // literals header is the first byte of the block body; force its 2 low bits
    // (literalsBlockType) to 3 == set_repeat at every plausible offset.
    for hdr in 5..frame.len().min(24) {
        let mut f = frame.clone();
        f[hdr] |= 0x03;
        all_decoders_agree(&f, src.len() + 64, &[], &format!("set_repeat @{hdr}"));
        let mut f = frame.clone();
        f[hdr] = (f[hdr] & 0xFC) | 0x03;
        all_decoders_agree(&f, src.len() + 64, &[], &format!("set_repeat exact @{hdr}"));
    }
}

#[test]
fn err_literals_headerwrong() {
    // 4-stream literals with too few literals, and every literals-header
    // size-format combination applied to a real frame.
    let mut rng = Rng::new(0xF00A);
    for &size in &[10usize, 100, 1000] {
        let src = gen(Shape::Text, size, &mut rng);
        let frame = compress_both(&[(C_COMPRESSIONLEVEL, 3), (C_CONTENTSIZEFLAG, 1)], &src);
        for hdr in 5..frame.len().min(20) {
            for sf in 0..4u8 {
                let mut f = frame.clone();
                f[hdr] = (f[hdr] & 0xF3) | (sf << 2);
                all_decoders_agree(
                    &f,
                    size + 64,
                    &[],
                    &format!("lit sizeFormat={sf} @{hdr} n={size}"),
                );
            }
        }
    }
}

#[test]
fn err_decodeSeqHeaders() {
    let mut rng = Rng::new(0xF00B);
    // mutate the sequences section: nbSeq field, symbol compression modes byte
    for &size in &[1000usize, 40000] {
        let src = gen(Shape::Text, size, &mut rng);
        let frame = compress_both(&[(C_COMPRESSIONLEVEL, 3), (C_CONTENTSIZEFLAG, 1)], &src);
        for off in 5..frame.len().min(80) {
            for v in [0u8, 0x01, 0x7F, 0x80, 0xFE, 0xFF] {
                let mut f = frame.clone();
                f[off] = v;
                all_decoders_agree(&f, size + 64, &[], &format!("seq byte {off}:={v:#x} n={size}"));
            }
        }
    }
}

#[test]
fn err_decompressblock() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (cd, rd) = unsafe { pair::<FnChunk>("ZSTD_decompressBlock") };
    let mut rng = Rng::new(0xF00C);
    // srcSize > ZSTD_BLOCKSIZE_MAX, dst NULL / dstCapacity 0, and garbage blocks
    for &n in &[0usize, 1, 2, 3, 100, 131072, 131073, 200000] {
        let buf = gen(Shape::Random, n, &mut rng);
        for &cap in &[0usize, 1, 1000, 1 << 18] {
            let cc = unsafe { cn() };
            let rc = unsafe { rn() };
            assert_eq!(unsafe { cdb(cc) }, unsafe { rdb(rc) });
            let mut a = vec![0u8; cap.max(1)];
            let mut b = vec![0u8; cap.max(1)];
            let x = unsafe { cd(cc, a.as_mut_ptr(), cap, buf.as_ptr(), n) };
            let y = unsafe { rd(rc, b.as_mut_ptr(), cap, buf.as_ptr(), n) };
            assert_eq!(x, y, "decompressBlock(n={n}, cap={cap})");
            if !is_error(x) {
                assert_bytes_eq("block bytes", &a[..x], &b[..y]);
            }
            // NULL dst
            let x = unsafe { cd(cc, std::ptr::null_mut(), cap, buf.as_ptr(), n) };
            let y = unsafe { rd(rc, std::ptr::null_mut(), cap, buf.as_ptr(), n) };
            assert_eq!(x, y, "decompressBlock(NULL dst, n={n}, cap={cap})");
            unsafe {
                cf(cc);
                rf(rc);
            }
        }
    }
}

#[test]
fn err_getcblocksize() {
    let (cg, rg) = unsafe { pair::<FnGetcBlockSize>("ZSTD_getcBlockSize") };
    let mut rng = Rng::new(0xF00D);
    for n in 0..=8usize {
        for _ in 0..20 {
            let buf = gen(Shape::Random, n, &mut rng);
            let mut a = BlockProperties::default();
            let mut b = BlockProperties::default();
            let x = unsafe { cg(buf.as_ptr(), n, &mut a) };
            let y = unsafe { rg(buf.as_ptr(), n, &mut b) };
            assert_eq!(x, y, "getcBlockSize(n={n})");
            if !is_error(x) {
                assert_eq!(a, b, "block properties");
            }
        }
    }
    // block type reserved
    for t in 0..4u8 {
        for last in 0..2u8 {
            let mut buf = [0u8; 3];
            buf[0] = last | (t << 1) | (0x10);
            let mut a = BlockProperties::default();
            let mut b = BlockProperties::default();
            let x = unsafe { cg(buf.as_ptr(), 3, &mut a) };
            let y = unsafe { rg(buf.as_ptr(), 3, &mut b) };
            assert_eq!(x, y, "getcBlockSize(type={t}, last={last})");
            if !is_error(x) {
                assert_eq!(a, b);
            }
        }
    }
}

#[test]
fn err_decompresscontinue() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (cns, rns) = unsafe { pair::<FnNextSize>("ZSTD_nextSrcSizeToDecompress") };
    let (cnt, rnt) = unsafe { pair::<FnNextType>("ZSTD_nextInputType") };
    let (cdc, rdc) = unsafe { pair::<FnChunk>("ZSTD_decompressContinue") };
    let mut rng = Rng::new(0xF00E);
    let src = gen(Shape::Text, 200000, &mut rng);
    let frame = compress_both(
        &[(C_COMPRESSIONLEVEL, 5), (C_CHECKSUMFLAG, 1), (C_CONTENTSIZEFLAG, 1)],
        &src,
    );

    // feed the WRONG size to decompressContinue at each stage
    for wrong in [0usize, 1, 2, 5, 100000] {
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
            let bg = if which == 0 { &cdb } else { &rdb };
            rets[which].push(unsafe { bg(z) });
            let mut obuf = vec![0u8; 1 << 19];
            let ns = if which == 0 { unsafe { cns(z) } } else { unsafe { rns(z) } };
            let nt = if which == 0 { unsafe { cnt(z) } } else { unsafe { rnt(z) } };
            rets[which].push(ns);
            rets[which].push(nt as usize);
            let n = wrong.min(frame.len());
            let f = if which == 0 { &cdc } else { &rdc };
            let rc =
                unsafe { f(z, obuf.as_mut_ptr(), obuf.len(), frame.as_ptr(), n) };
            rets[which].push(rc);
            if which == 0 {
                unsafe { cf(z) };
            } else {
                unsafe { rf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "decompressContinue(wrong size {wrong})");
    }
    // corrupt each block header / body byte and run the low-level loop
    for (name, bad) in mutations(&frame, &mut rng).into_iter().take(120) {
        all_decoders_agree(&bad, src.len() + 4096, &[], &format!("llcont {name}"));
    }
    // truncated output buffer during decompressContinue
    for cap in [0usize, 1, 100, 1 << 16] {
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
            let bg = if which == 0 { &cdb } else { &rdb };
            rets[which].push(unsafe { bg(z) });
            let mut obuf = vec![0u8; cap.max(1)];
            let mut ip = 0usize;
            let mut guard = 0;
            loop {
                guard += 1;
                if guard > 10000 {
                    break;
                }
                let ns = if which == 0 { unsafe { cns(z) } } else { unsafe { rns(z) } };
                if ns == 0 || ip >= frame.len() {
                    rets[which].push(ns);
                    break;
                }
                let avail = (frame.len() - ip).min(ns);
                let f = if which == 0 { &cdc } else { &rdc };
                let rc = unsafe { f(z, obuf.as_mut_ptr(), cap, frame[ip..].as_ptr(), avail) };
                rets[which].push(rc);
                if is_error(rc) {
                    break;
                }
                ip += avail;
            }
            if which == 0 {
                unsafe { cf(z) };
            } else {
                unsafe { rf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "decompressContinue(out cap {cap})");
    }
}

// -------------------------------------------------------- dparams (244-258) --

#[test]
fn err_dparam_unknown() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cg, rg) = unsafe { pair::<FnGetParam>("ZSTD_DCtx_getParameter") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for p in [
        i32::MIN, -1000, -1, 0, 1, 99, 101, 200, 999, 1006, 1007, 100000, i32::MAX,
    ] {
        for v in [0, 1, -1, 999] {
            assert_eq!(
                unsafe { cs(cc, p, v) },
                unsafe { rs(rc, p, v) },
                "DCtx_setParameter({p},{v})"
            );
        }
        let mut a = 0;
        let mut b = 0;
        assert_eq!(
            unsafe { cg(cc, p, &mut a) },
            unsafe { rg(rc, p, &mut b) },
            "DCtx_getParameter({p})"
        );
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

#[test]
fn err_dparam_out_of_bound() {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Bounds {
        error: usize,
        lo: c_int,
        hi: c_int,
    }
    type FnGetBounds = unsafe extern "C" fn(c_int) -> Bounds;
    let (cgb, _) = unsafe { pair::<FnGetBounds>("ZSTD_dParam_getBounds") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (csf, rsf) = unsafe { pair::<FnSetFormat>("ZSTD_DCtx_setFormat") };
    let (cg, rg) = unsafe { pair::<FnGetParam>("ZSTD_DCtx_getParameter") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for &(p, name) in DPARAMS {
        let b = unsafe { cgb(p) };
        assert_eq!(b.error, 0, "{name}");
        for v in [
            b.lo.saturating_sub(1),
            b.lo.saturating_sub(2),
            b.hi.saturating_add(1),
            b.hi.saturating_add(2),
            i32::MIN,
            i32::MAX,
            -1,
        ] {
            let x = unsafe { cs(cc, p, v) };
            let y = unsafe { rs(rc, p, v) };
            assert_eq!(x, y, "DCtx_setParameter({name}={v})");
            let mut a = 0;
            let mut b2 = 0;
            assert_eq!(
                unsafe { cg(cc, p, &mut a) },
                unsafe { rg(rc, p, &mut b2) },
                "DCtx_getParameter({name}) after {v}"
            );
            assert_eq!(a, b2, "DCtx_getParameter({name}) value after {v}");
        }
    }
    for f in [-1, 0, 1, 2, 99, i32::MAX, i32::MIN] {
        assert_eq!(unsafe { csf(cc, f) }, unsafe { rsf(rc, f) }, "DCtx_setFormat({f})");
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

#[test]
fn err_dctx_setparam_stage_wrong() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let (cld, rld) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary") };
    let (clda, rlda) = unsafe { pair::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced") };
    let (cldr, rldr) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference") };
    let (crp, rrp) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_refPrefix") };
    let (crpa, rrpa) = unsafe { pair::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced") };
    let (crd, rrd) = unsafe { pair::<FnRefDDict>("ZSTD_DCtx_refDDict") };
    let (csmw, rsmw) = unsafe { pair::<FnSetMaxWindow>("ZSTD_DCtx_setMaxWindowSize") };
    let (cid, rid) = unsafe { pair::<FnInitDStream>("ZSTD_initDStream") };
    let (cidd, ridd) = unsafe { pair::<FnDBeginDict>("ZSTD_initDStream_usingDict") };
    let (cddc, rddc) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let mut rng = Rng::new(0xF00F);
    let dict = gen(Shape::Text, 4096, &mut rng);
    let src = gen(Shape::Text, 300000, &mut rng);
    let frame = compress_both(&[(C_COMPRESSIONLEVEL, 5)], &src);
    let cdd = unsafe { cddc(dict.as_ptr(), dict.len()) };
    let rdd = unsafe { rddc(dict.as_ptr(), dict.len()) };

    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    // start decoding but do not finish
    let mut obuf = vec![0u8; 4096];
    for which in 0..2 {
        let z = if which == 0 { cc } else { rc };
        let mut input = InBuffer { src: frame.as_ptr(), size: frame.len() / 2, pos: 0 };
        let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
        let rc0 = if which == 0 {
            unsafe { cds(z, &mut out, &mut input) }
        } else {
            unsafe { rds(z, &mut out, &mut input) }
        };
        assert!(!is_error(rc0));
    }
    macro_rules! chk {
        ($label:expr, $c:expr, $r:expr) => {{
            let x = unsafe { $c };
            let y = unsafe { $r };
            assert_eq!(x, y, "mid-decode {}", $label);
        }};
    }
    for &(p, name) in DPARAMS {
        chk!(format!("setParameter({name})"), cs(cc, p, 1), rs(rc, p, 1));
    }
    chk!("loadDictionary", cld(cc, dict.as_ptr(), dict.len()),
         rld(rc, dict.as_ptr(), dict.len()));
    chk!("loadDictionary_byReference", cldr(cc, dict.as_ptr(), dict.len()),
         rldr(rc, dict.as_ptr(), dict.len()));
    chk!("loadDictionary_advanced", clda(cc, dict.as_ptr(), dict.len(), 0, 0),
         rlda(rc, dict.as_ptr(), dict.len(), 0, 0));
    chk!("refPrefix", crp(cc, dict.as_ptr(), dict.len()), rrp(rc, dict.as_ptr(), dict.len()));
    chk!("refPrefix_advanced", crpa(cc, dict.as_ptr(), dict.len(), 0),
         rrpa(rc, dict.as_ptr(), dict.len(), 0));
    chk!("refDDict", crd(cc, cdd), rrd(rc, rdd));
    chk!("setMaxWindowSize", csmw(cc, 1 << 20), rsmw(rc, 1 << 20));
    chk!("initDStream", cid(cc), rid(rc));
    chk!("initDStream_usingDict", cidd(cc, dict.as_ptr(), dict.len()),
         ridd(rc, dict.as_ptr(), dict.len()));
    unsafe {
        cf(cc);
        rf(rc);
        cfdd(cdd);
        rfdd(rdd);
    }
}

#[test]
fn err_dctx_reset_midframe() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let (crst, rrst) = unsafe { pair::<FnReset>("ZSTD_DCtx_reset") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let mut rng = Rng::new(0xF010);
    let src = gen(Shape::Text, 300000, &mut rng);
    let frame = compress_both(&[(C_COMPRESSIONLEVEL, 5)], &src);
    for &kind in &[1, 2, 3] {
        let cc = unsafe { cn() };
        let rc = unsafe { rn() };
        let mut obuf = vec![0u8; 4096];
        for which in 0..2 {
            let z = if which == 0 { cc } else { rc };
            let mut input = InBuffer { src: frame.as_ptr(), size: frame.len() / 2, pos: 0 };
            let mut out = OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: 0 };
            let rc0 = if which == 0 {
                unsafe { cds(z, &mut out, &mut input) }
            } else {
                unsafe { rds(z, &mut out, &mut input) }
            };
            assert!(!is_error(rc0));
        }
        let x = unsafe { crst(cc, kind) };
        let y = unsafe { rrst(rc, kind) };
        assert_eq!(x, y, "DCtx_reset({kind}) mid-decode");
        // after a successful reset, setParameter must work again (identically)
        let x = unsafe { cs(cc, D_WINDOWLOGMAX, 31) };
        let y = unsafe { rs(rc, D_WINDOWLOGMAX, 31) };
        assert_eq!(x, y, "setParameter after reset({kind})");
        unsafe {
            cf(cc);
            rf(rc);
        }
    }
}

#[test]
fn err_dctx_maxwindowsize() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetMaxWindow>("ZSTD_DCtx_setMaxWindowSize") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for size in [
        0usize,
        1,
        (1 << 10) - 1,
        1 << 10,
        (1 << 10) + 1,
        1 << 27,
        1usize << 31,
        (1usize << 31) + 1,
        usize::MAX,
    ] {
        assert_eq!(unsafe { cs(cc, size) }, unsafe { rs(rc, size) }, "setMaxWindowSize({size})");
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

#[test]
fn err_window_too_large() {
    let mut rng = Rng::new(0xF011);
    // frames with large windows decoded under various windowLogMax limits
    for &wlog in &[10, 17, 20, 27, 28, 31] {
        let src = gen(Shape::Text, 300000, &mut rng);
        let frame = compress_both(&[(C_WINDOWLOG, wlog), (C_COMPRESSIONLEVEL, 5)], &src);
        for &limit in &[0, 10, 17, 20, 27, 28, 31] {
            let dp: Vec<(c_int, c_int)> =
                if limit == 0 { vec![] } else { vec![(D_WINDOWLOGMAX, limit)] };
            all_decoders_agree(&frame, src.len() + 64, &dp, &format!("wlog={wlog} limit={limit}"));
        }
    }
    // and a header whose window descriptor is maxed out
    let src = gen(Shape::Text, 1000, &mut rng);
    let frame = compress_both(&[(C_COMPRESSIONLEVEL, 3)], &src);
    let mut f = frame.clone();
    if f.len() > 5 {
        f[5] = 0xFF;
        all_decoders_agree(&f, 65536, &[], "window descriptor 0xFF");
    }
}

#[test]
fn err_magicless_mismatch() {
    let mut rng = Rng::new(0xF012);
    for &size in &[0usize, 100, 60000] {
        let src = gen(Shape::Text, size, &mut rng);
        let plain = compress_both(&[(C_COMPRESSIONLEVEL, 3), (C_FORMAT, 0)], &src);
        let magicless = compress_both(&[(C_COMPRESSIONLEVEL, 3), (C_FORMAT, 1)], &src);
        for (label, frame) in [("plain", &plain), ("magicless", &magicless)] {
            for fmt in [0, 1] {
                all_decoders_agree(
                    frame,
                    size + 64,
                    &[(D_FORMAT, fmt)],
                    &format!("{label} decoded with d_format={fmt} n={size}"),
                );
            }
        }
    }
}

#[test]
fn err_dstable_out_violation() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let mut rng = Rng::new(0xF013);
    let src = gen(Shape::Text, 200000, &mut rng);
    let frame = compress_both(&[(C_COMPRESSIONLEVEL, 5), (C_CONTENTSIZEFLAG, 1)], &src);

    for violation in 0..4 {
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
            let sp = if which == 0 { &cs } else { &rs };
            rets[which].push(unsafe { sp(z, D_STABLEOUT, 1) });
            let mut obuf = vec![0u8; src.len() + 128];
            let mut obuf2 = vec![0u8; src.len() + 128];
            let mut input = InBuffer { src: frame.as_ptr(), size: frame.len() / 2, pos: 0 };
            let mut out = OutBuffer {
                dst: obuf.as_mut_ptr(),
                size: if violation == 3 { src.len() / 2 } else { obuf.len() },
                pos: 0,
            };
            let f = if which == 0 { &cds } else { &rds };
            let rc = unsafe { f(z, &mut out, &mut input) };
            rets[which].push(rc);
            // now violate the contract in the second call
            let mut out2 = match violation {
                0 => OutBuffer { dst: obuf2.as_mut_ptr(), size: obuf.len(), pos: out.pos },
                1 => OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len() - 10, pos: out.pos },
                2 => OutBuffer { dst: obuf.as_mut_ptr(), size: obuf.len(), pos: out.pos + 5 },
                _ => OutBuffer { dst: obuf.as_mut_ptr(), size: src.len() / 2, pos: out.pos },
            };
            input.size = frame.len();
            let rc = unsafe { f(z, &mut out2, &mut input) };
            rets[which].push(rc);
            if which == 0 {
                unsafe { cf(z) };
            } else {
                unsafe { rf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "d_stableOutBuffer violation {violation}");
    }
}

#[test]
fn err_no_forward_progress() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let mut rng = Rng::new(0xF014);
    let src = gen(Shape::Text, 100000, &mut rng);
    let frame = compress_both(&[(C_COMPRESSIONLEVEL, 3)], &src);

    // (a) output buffer permanently full -> noForwardProgress_destFull
    // (b) input exhausted (truncated frame) -> noForwardProgress_inputEmpty
    for mode in 0..2 {
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
            let mut obuf = vec![0u8; 16];
            let f = if which == 0 { &cds } else { &rds };
            let feed = if mode == 0 { frame.len() } else { frame.len() / 2 };
            let mut input = InBuffer { src: frame.as_ptr(), size: feed, pos: 0 };
            for _ in 0..40 {
                let mut out = OutBuffer {
                    dst: obuf.as_mut_ptr(),
                    size: if mode == 0 { 0 } else { obuf.len() },
                    pos: 0,
                };
                let rc = unsafe { f(z, &mut out, &mut input) };
                rets[which].push(rc);
                if is_error(rc) || rc == 0 {
                    break;
                }
                if mode == 1 {
                    // never give more input, and drain the output so only the
                    // input side can stall
                    input.size = feed;
                }
            }
            if which == 0 {
                unsafe { cf(z) };
            } else {
                unsafe { rf(z) };
            }
        }
        assert_eq!(rets[0], rets[1], "noForwardProgress mode {mode}");
        assert!(
            rets[0].iter().any(|&r| is_error(r)),
            "mode {mode}: C should eventually report noForwardProgress (got {:?})",
            rets[0]
        );
    }
}

#[test]
fn err_decodingbuffersize() {
    let (cd, rd) = unsafe { pair::<FnDecodingBufferSizeMin>("ZSTD_decodingBufferSize_min") };
    for ws in [0u64, 1, 1 << 10, 1 << 17, 1 << 27, 1u64 << 31, 1u64 << 40, u64::MAX] {
        for fcs in [
            0u64,
            1,
            1 << 20,
            1u64 << 40,
            u64::MAX - 1,
            u64::MAX,
        ] {
            assert_eq!(
                unsafe { cd(ws, fcs) },
                unsafe { rd(ws, fcs) },
                "decodingBufferSize_min({ws},{fcs})"
            );
        }
    }
}

#[test]
fn err_static_legacy() {
    let (cis, ris) = unsafe { pair::<FnInitStaticDCtx>("ZSTD_initStaticDCtx") };
    let (ce, re) = unsafe { pair::<FnEstimateDCtx>("ZSTD_estimateDCtxSize") };
    let (cdd, rdd) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let (cds, rds) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let need_c = unsafe { ce() };
    let need_r = unsafe { re() };
    assert_eq!(need_c, need_r, "estimateDCtxSize");
    let extra = 1 << 20;
    let mut wa: Vec<u64> = vec![0; (need_c + extra) / 8 + 8];
    let mut wb: Vec<u64> = vec![0; (need_c + extra) / 8 + 8];
    let ca = unsafe { cis(wa.as_mut_ptr() as *mut c_void, need_c + extra) };
    let cb = unsafe { ris(wb.as_mut_ptr() as *mut c_void, need_r + extra) };
    assert!(!ca.is_null() && !cb.is_null());
    let mut rng = Rng::new(0xF015);
    // legacy magic frames (v05/v06/v07 are dispatched by ZSTD_isLegacy)
    for magic in [0xFD2FB51Eu32, 0xFD2FB522, 0xFD2FB523, 0xFD2FB524, 0xFD2FB525, 0xFD2FB526,
                  0xFD2FB527] {
        let mut buf = magic.to_le_bytes().to_vec();
        buf.extend(gen(Shape::Random, 64, &mut rng));
        let mut oa = vec![0u8; 65536];
        let mut ob = vec![0u8; 65536];
        let x = unsafe { cdd(ca, oa.as_mut_ptr(), oa.len(), buf.as_ptr(), buf.len()) };
        let y = unsafe { rdd(cb, ob.as_mut_ptr(), ob.len(), buf.as_ptr(), buf.len()) };
        assert_eq!(x, y, "static DCtx, legacy magic {magic:#x}");
        let mut input = InBuffer { src: buf.as_ptr(), size: buf.len(), pos: 0 };
        let mut input2 = InBuffer { src: buf.as_ptr(), size: buf.len(), pos: 0 };
        let mut o1 = OutBuffer { dst: oa.as_mut_ptr(), size: oa.len(), pos: 0 };
        let mut o2 = OutBuffer { dst: ob.as_mut_ptr(), size: ob.len(), pos: 0 };
        let x = unsafe { cds(ca, &mut o1, &mut input) };
        let y = unsafe { rds(cb, &mut o2, &mut input2) };
        assert_eq!(x, y, "static DStream, legacy magic {magic:#x}");
    }
    // NOTE: a static DCtx must not be freed with ZSTD_freeDCtx (see err_free_static_dctx)
}

/// ERRORS rows 217-219: ZSTD_createDDict_advanced rejections (dct_fullDict with a
/// short dict, a wrong magic, or corrupt entropy tables) must return NULL in both.
#[test]
fn err_ddict_bad() {
    let (cda, rda) = unsafe { pair::<FnCreateDDictAdvE>("ZSTD_createDDict_advanced") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cdr, rdr) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict_byReference") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let mut rng = Rng::new(0xF020);
    let good = train_dict(&mut rng, Shape::Text);

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [0usize, 1, 4, 7, 8, 9, 12, 20, 50, 200] {
        if n <= good.len() {
            cases.push((format!("truncated {n}"), good[..n].to_vec()));
        }
    }
    for pos in [0usize, 1, 2, 3, 4, 8, 9, 10, 12, 16, 30, 60, 120] {
        if pos < good.len() {
            let mut d = good.clone();
            d[pos] ^= 0xFF;
            cases.push((format!("byte {pos} flipped"), d));
        }
    }
    // magic + garbage / zeros, and raw content
    for n in [8usize, 40, 200] {
        let mut d = 0xEC30A437u32.to_le_bytes().to_vec();
        d.extend(gen(Shape::Random, n, &mut rng));
        cases.push((format!("magic+random{n}"), d));
        let mut d = 0xEC30A437u32.to_le_bytes().to_vec();
        d.extend(std::iter::repeat(0u8).take(n));
        cases.push((format!("magic+zeros{n}"), d));
    }
    cases.push(("raw 4k".into(), gen(Shape::Text, 4096, &mut rng)));
    cases.push(("empty".into(), Vec::new()));

    let nomem = CustomMemE { alloc: None, free: None, opaque: std::ptr::null_mut() };
    for (name, d) in cases {
        for dlm in [0, 1] {
            for dct in [0, 1, 2] {
                let a = unsafe { cda(d.as_ptr(), d.len(), dlm, dct, nomem) };
                let b = unsafe { rda(d.as_ptr(), d.len(), dlm, dct, nomem) };
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "createDDict_advanced({name}, dlm={dlm}, dct={dct})"
                );
                unsafe {
                    if !a.is_null() {
                        cfd(a);
                    }
                    if !b.is_null() {
                        rfd(b);
                    }
                }
            }
        }
        let a = unsafe { cdd(d.as_ptr(), d.len()) };
        let b = unsafe { rdd(d.as_ptr(), d.len()) };
        assert_eq!(a.is_null(), b.is_null(), "createDDict({name})");
        unsafe {
            if !a.is_null() {
                cfd(a);
            }
            if !b.is_null() {
                rfd(b);
            }
        }
        let a = unsafe { cdr(d.as_ptr(), d.len()) };
        let b = unsafe { rdr(d.as_ptr(), d.len()) };
        assert_eq!(a.is_null(), b.is_null(), "createDDict_byReference({name})");
        unsafe {
            if !a.is_null() {
                cfd(a);
            }
            if !b.is_null() {
                rfd(b);
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CustomMemE {
    pub alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}
type FnCreateDDictAdvE =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, CustomMemE) -> *mut c_void;

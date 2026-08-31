//! LZ4 Frame API (`lz4frame.c`).
mod common;

use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

/// `LZ4F_frameInfo_t`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInfo {
    pub block_size_id: u32,
    pub block_mode: u32,
    pub content_checksum_flag: u32,
    pub frame_type: u32,
    pub content_size: u64,
    pub dict_id: u32,
    pub block_checksum_flag: u32,
}

/// `LZ4F_preferences_t`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Preferences {
    pub frame_info: FrameInfo,
    pub compression_level: i32,
    pub auto_flush: u32,
    pub favor_dec_speed: u32,
    pub reserved: [u32; 3],
}

/// `LZ4F_compressOptions_t`
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CompressOptions {
    pub stable_src: u32,
    pub reserved: [u32; 3],
}

/// `LZ4F_decompressOptions_t`
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DecompressOptions {
    pub stable_dst: u32,
    pub skip_checksums: u32,
    pub reserved1: u32,
    pub reserved0: u32,
}

const LZ4F_VERSION: u32 = 100;

/* ---------------- error / trivial accessors ---------------- */

#[test]
fn frame_error_helpers() {
    let (c_iserr, r_iserr) = pair!("LZ4F_isError", fn(usize) -> u32);
    let (c_name, r_name) = pair!("LZ4F_getErrorName", fn(usize) -> *const c_char);
    let (c_code, r_code) = pair!("LZ4F_getErrorCode", fn(usize) -> i32);
    unsafe {
        let mut codes: Vec<usize> = vec![0, 1, 2, 100, 1 << 20];
        // error codes are returned as (size_t)-errorCode
        for e in 0..80usize {
            codes.push(0usize.wrapping_sub(e));
        }
        codes.push(usize::MAX);
        for &c in &codes {
            assert_eq!(c_iserr(c), r_iserr(c), "LZ4F_isError({})", c as isize);
            assert_eq!(c_code(c), r_code(c), "LZ4F_getErrorCode({})", c as isize);
            let a = CStr::from_ptr(c_name(c));
            let b = CStr::from_ptr(r_name(c));
            assert_eq!(a, b, "LZ4F_getErrorName({})", c as isize);
        }
    }
}

#[test]
fn frame_block_size_and_bounds() {
    let (c_bs, r_bs) = pair!("LZ4F_getBlockSize", fn(u32) -> usize);
    unsafe {
        for id in 0u32..12 {
            assert_eq!(c_bs(id), r_bs(id), "LZ4F_getBlockSize({})", id);
        }
        for id in [100u32, u32::MAX] {
            assert_eq!(c_bs(id), r_bs(id), "LZ4F_getBlockSize({})", id);
        }
    }

    let (c_cb, r_cb) = pair!("LZ4F_compressBound", fn(usize, *const Preferences) -> usize);
    let (c_fb, r_fb) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    unsafe {
        let mut prefs_list = vec![];
        for &bs in &[0u32, 4, 5, 6, 7] {
            for &bm in &[0u32, 1] {
                for &cc in &[0u32, 1] {
                    for &bc in &[0u32, 1] {
                        for &af in &[0u32, 1] {
                            let mut p = Preferences::default();
                            p.frame_info.block_size_id = bs;
                            p.frame_info.block_mode = bm;
                            p.frame_info.content_checksum_flag = cc;
                            p.frame_info.block_checksum_flag = bc;
                            p.auto_flush = af;
                            prefs_list.push(p);
                        }
                    }
                }
            }
        }
        for sz in [
            0usize, 1, 15, 16, 64, 65535, 65536, 65537, 262144, 1 << 20, 1 << 22, 10_000_000,
        ] {
            assert_eq!(
                c_cb(sz, std::ptr::null()),
                r_cb(sz, std::ptr::null()),
                "compressBound({}, NULL)",
                sz
            );
            assert_eq!(
                c_fb(sz, std::ptr::null()),
                r_fb(sz, std::ptr::null()),
                "compressFrameBound({}, NULL)",
                sz
            );
            for p in &prefs_list {
                assert_eq!(c_cb(sz, p), r_cb(sz, p), "compressBound({}, {:?})", sz, p);
                assert_eq!(
                    c_fb(sz, p),
                    r_fb(sz, p),
                    "compressFrameBound({}, {:?})",
                    sz,
                    p
                );
            }
        }
    }
}

#[test]
fn frame_context_lifecycle() {
    let (c_cnew, r_cnew) = pair!(
        "LZ4F_createCompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_cfree, r_cfree) = pair!("LZ4F_freeCompressionContext", fn(*mut c_void) -> usize);
    let (c_dnew, r_dnew) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_reset, r_reset) = pair!("LZ4F_resetDecompressionContext", fn(*mut c_void));
    unsafe {
        for ver in [0u32, 1, LZ4F_VERSION, LZ4F_VERSION + 1, u32::MAX] {
            let mut cc: *mut c_void = std::ptr::null_mut();
            let mut rc: *mut c_void = std::ptr::null_mut();
            let ra = c_cnew(&mut cc, ver);
            let rb = r_cnew(&mut rc, ver);
            assert_eq!(ra, rb, "createCompressionContext(ver={})", ver);
            assert_eq!(cc.is_null(), rc.is_null());
            assert_eq!(c_cfree(cc), r_cfree(rc));

            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            let ra = c_dnew(&mut cd, ver);
            let rb = r_dnew(&mut rd, ver);
            assert_eq!(ra, rb, "createDecompressionContext(ver={})", ver);
            assert_eq!(cd.is_null(), rd.is_null());
            if !cd.is_null() {
                c_reset(cd);
                r_reset(rd);
            }
            assert_eq!(c_dfree(cd), r_dfree(rd));
        }
        assert_eq!(
            c_cfree(std::ptr::null_mut()),
            r_cfree(std::ptr::null_mut())
        );
        assert_eq!(
            c_dfree(std::ptr::null_mut()),
            r_dfree(std::ptr::null_mut())
        );
    }
}

/// A representative spread of preference sets.
fn pref_matrix() -> Vec<Preferences> {
    let mut v = Vec::new();
    for &bs in &[0u32, 4, 5, 7] {
        for &bm in &[0u32, 1] {
            for &cc in &[0u32, 1] {
                for &bc in &[0u32, 1] {
                    for &lvl in &[0i32, 1, 12] {
                        for &af in &[0u32, 1] {
                            let mut p = Preferences::default();
                            p.frame_info.block_size_id = bs;
                            p.frame_info.block_mode = bm;
                            p.frame_info.content_checksum_flag = cc;
                            p.frame_info.block_checksum_flag = bc;
                            p.compression_level = lvl;
                            p.auto_flush = af;
                            v.push(p);
                        }
                    }
                }
            }
        }
    }
    // extra levels on the default frame layout
    for &lvl in &[-1i32, 2, 3, 9, 10, 11, 13, 100] {
        let mut p = Preferences::default();
        p.compression_level = lvl;
        v.push(p);
    }
    // a few with contentSize / dictID / favorDecSpeed set
    for &(csize, did, favor) in &[
        (0u64, 0u32, 0u32),
        (1000, 0, 0),
        (0, 0x1234_5678, 0),
        (12345, 0xABCD, 1),
    ] {
        let mut p = Preferences::default();
        p.frame_info.content_size = csize;
        p.frame_info.dict_id = did;
        p.favor_dec_speed = favor;
        p.compression_level = 11;
        v.push(p);
    }
    v
}

/// A smaller matrix for the more expensive tests.
fn pref_matrix_small() -> Vec<Preferences> {
    let mut v = Vec::new();
    for &bs in &[4u32, 7] {
        for &bm in &[0u32, 1] {
            for &cc in &[0u32, 1] {
                for &bc in &[0u32, 1] {
                    for &lvl in &[0i32, 12] {
                        let mut p = Preferences::default();
                        p.frame_info.block_size_id = bs;
                        p.frame_info.block_mode = bm;
                        p.frame_info.content_checksum_flag = cc;
                        p.frame_info.block_checksum_flag = bc;
                        p.compression_level = lvl;
                        v.push(p);
                    }
                }
            }
        }
    }
    v
}

/// Hand-picked set covering each axis at least once, for the tests that build
/// many whole frames.
fn pref_matrix_tiny() -> Vec<Preferences> {
    let mut v = Vec::new();
    let variants: [(u32, u32, u32, u32, i32, u32); 6] = [
        // blockSizeID, blockMode, contentChecksum, blockChecksum, level, autoFlush
        (0, 0, 0, 0, 0, 0),
        (4, 1, 1, 1, 1, 1),
        (5, 0, 1, 0, 9, 0),
        (6, 1, 0, 1, 12, 0),
        (7, 0, 1, 1, 3, 1),
        (7, 1, 1, 0, 10, 0),
    ];
    for &(bs, bm, cc, bc, lvl, af) in &variants {
        let mut p = Preferences::default();
        p.frame_info.block_size_id = bs;
        p.frame_info.block_mode = bm;
        p.frame_info.content_checksum_flag = cc;
        p.frame_info.block_checksum_flag = bc;
        p.compression_level = lvl;
        p.auto_flush = af;
        v.push(p);
    }
    v
}

#[test]
fn frame_compress_frame() {
    let (c_cf, r_cf) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);
    let prefs = pref_matrix();
    let prefs_big = pref_matrix_small();
    unsafe {
        // full preference matrix on small inputs
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 13, 100, 1000, 4096] {
                let data = g(sz, 151 + sz as u64);
                for (pi, p) in prefs.iter().enumerate() {
                    let bound = c_fb(sz, p);
                    let mut a = vec![0x3Fu8; bound + 64];
                    let mut b = vec![0x3Fu8; bound + 64];
                    let ra = c_cf(a.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    let rb = r_cf(b.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    assert_eq!(ra, rb, "compressFrame {} sz={} pref#{}", gname, sz, pi);
                    beq!(a, b, "compressFrame bytes {} sz={} pref#{}", gname, sz, pi);
                    assert_eq!(c_iserr(ra), 0, "compressFrame failed sz={} pref#{}", sz, pi);
                }
                // NULL preferences
                let bound = c_fb(sz, std::ptr::null());
                let mut a = vec![0x3Fu8; bound + 64];
                let mut b = vec![0x3Fu8; bound + 64];
                let ra = c_cf(a.as_mut_ptr(), bound, data.as_ptr(), sz, std::ptr::null());
                let rb = r_cf(b.as_mut_ptr(), bound, data.as_ptr(), sz, std::ptr::null());
                assert_eq!(ra, rb, "compressFrame NULL prefs {} sz={}", gname, sz);
                beq!(a, b, "compressFrame NULL prefs bytes {} sz={}", gname, sz);
            }
            // multi-block inputs with a reduced matrix
            for &sz in &[65536usize, 70000, 200000] {
                let data = g(sz, 151 + sz as u64);
                for (pi, p) in prefs_big.iter().enumerate() {
                    let bound = c_fb(sz, p);
                    let mut a = vec![0x3Fu8; bound + 64];
                    let mut b = vec![0x3Fu8; bound + 64];
                    let ra = c_cf(a.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    let rb = r_cf(b.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    assert_eq!(ra, rb, "compressFrame {} sz={} pref#{}", gname, sz, pi);
                    beq!(a, b, "compressFrame bytes {} sz={} pref#{}", gname, sz, pi);
                    assert_eq!(c_iserr(ra), 0);
                }
            }
        }
    }
}

#[test]
fn frame_compress_frame_small_capacity() {
    let (c_cf, r_cf) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    let prefs = pref_matrix_small();
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 100, 4096, 70000] {
                let data = g(sz, 161 + sz as u64);
                for (pi, p) in prefs.iter().enumerate() {
                    let bound = c_fb(sz, p);
                    for &cap in &[0usize, 1, 7, 15, 19, 32, bound / 2, bound - 1, bound] {
                        let mut a = vec![0x1Du8; cap + 128];
                        let mut b = vec![0x1Du8; cap + 128];
                        let ra = c_cf(a.as_mut_ptr(), cap, data.as_ptr(), sz, p);
                        let rb = r_cf(b.as_mut_ptr(), cap, data.as_ptr(), sz, p);
                        assert_eq!(
                            ra, rb,
                            "compressFrame {} sz={} pref#{} cap={}",
                            gname, sz, pi, cap
                        );
                        beq!(
                            a,
                            b,
                            "compressFrame bytes {} sz={} pref#{} cap={}",
                            gname,
                            sz,
                            pi,
                            cap
                        );
                    }
                }
            }
        }
    }
}

/// Streaming compression: begin / update / flush / end with many chunkings.
#[test]
fn frame_streaming_compression() {
    let (c_cnew, r_cnew) = pair!(
        "LZ4F_createCompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_cfree, r_cfree) = pair!("LZ4F_freeCompressionContext", fn(*mut c_void) -> usize);
    let (c_begin, r_begin) = pair!(
        "LZ4F_compressBegin",
        fn(*mut c_void, *mut u8, usize, *const Preferences) -> usize
    );
    let (c_upd, r_upd) = pair!(
        "LZ4F_compressUpdate",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const CompressOptions) -> usize
    );
    let (c_flush, r_flush) = pair!(
        "LZ4F_flush",
        fn(*mut c_void, *mut u8, usize, *const CompressOptions) -> usize
    );
    let (c_end, r_end) = pair!(
        "LZ4F_compressEnd",
        fn(*mut c_void, *mut u8, usize, *const CompressOptions) -> usize
    );
    let (c_cb, _) = pair!("LZ4F_compressBound", fn(usize, *const Preferences) -> usize);
    let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);

    let chunkings: [&[usize]; 5] = [
        &[1],
        &[7, 3, 100],
        &[4096],
        &[65536, 1, 1000],
        &[300, 30000, 5],
    ];
    let prefs = pref_matrix_tiny();
    unsafe {
        for (gname, g) in GENS {
            for cks in &chunkings {
                let total = if cks == &&[1usize][..] { 3_000 } else { 60_000 };
                let data = g(total, 171 + gname.len() as u64);
                for (pi, p) in prefs.iter().enumerate() {
                    for &stable in &[0u32, 1] {
                        let opts = CompressOptions {
                            stable_src: stable,
                            reserved: [0; 3],
                        };
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc: *mut c_void = std::ptr::null_mut();
                        assert_eq!(
                            c_cnew(&mut cc, LZ4F_VERSION),
                            r_cnew(&mut rc, LZ4F_VERSION)
                        );

                        let mut framec: Vec<u8> = Vec::new();
                        let mut framer: Vec<u8> = Vec::new();

                        let mut a = vec![0u8; 64];
                        let mut b = vec![0u8; 64];
                        let ra = c_begin(cc, a.as_mut_ptr(), a.len(), p);
                        let rb = r_begin(rc, b.as_mut_ptr(), b.len(), p);
                        assert_eq!(ra, rb, "compressBegin {} pref#{}", gname, pi);
                        assert_eq!(c_iserr(ra), 0);
                        beq!(a[..ra], b[..ra], "header bytes {} pref#{}", gname, pi);
                        framec.extend_from_slice(&a[..ra]);
                        framer.extend_from_slice(&b[..ra]);

                        let mut pos = 0usize;
                        let mut i = 0usize;
                        while pos < data.len() {
                            let n = cks[i % cks.len()].min(data.len() - pos);
                            let cap = c_cb(n, p);
                            let mut a = vec![0u8; cap + 16];
                            let mut b = vec![0u8; cap + 16];
                            let ra = c_upd(
                                cc,
                                a.as_mut_ptr(),
                                cap,
                                data[pos..].as_ptr(),
                                n,
                                &opts,
                            );
                            let rb = r_upd(
                                rc,
                                b.as_mut_ptr(),
                                cap,
                                data[pos..].as_ptr(),
                                n,
                                &opts,
                            );
                            assert_eq!(
                                ra, rb,
                                "compressUpdate {} pref#{} stable={} pos={} n={}",
                                gname, pi, stable, pos, n
                            );
                            assert_eq!(c_iserr(ra), 0, "compressUpdate error pos={}", pos);
                            beq!(
                                a[..ra],
                                b[..ra],
                                "compressUpdate bytes {} pref#{} pos={}",
                                gname,
                                pi,
                                pos
                            );
                            framec.extend_from_slice(&a[..ra]);
                            framer.extend_from_slice(&b[..ra]);
                            pos += n;
                            i += 1;

                            // occasional explicit flush
                            if i % 7 == 0 {
                                let cap = c_cb(0, p);
                                let mut a = vec![0u8; cap + 16];
                                let mut b = vec![0u8; cap + 16];
                                let ra = c_flush(cc, a.as_mut_ptr(), cap, &opts);
                                let rb = r_flush(rc, b.as_mut_ptr(), cap, &opts);
                                assert_eq!(ra, rb, "flush {} pref#{} pos={}", gname, pi, pos);
                                beq!(a[..ra], b[..ra], "flush bytes {} pref#{}", gname, pi);
                                framec.extend_from_slice(&a[..ra]);
                                framer.extend_from_slice(&b[..ra]);
                            }
                        }

                        let cap = c_cb(0, p);
                        let mut a = vec![0u8; cap + 32];
                        let mut b = vec![0u8; cap + 32];
                        let ra = c_end(cc, a.as_mut_ptr(), cap, &opts);
                        let rb = r_end(rc, b.as_mut_ptr(), cap, &opts);
                        assert_eq!(ra, rb, "compressEnd {} pref#{}", gname, pi);
                        assert_eq!(c_iserr(ra), 0);
                        beq!(a[..ra], b[..ra], "compressEnd bytes {} pref#{}", gname, pi);
                        framec.extend_from_slice(&a[..ra]);
                        framer.extend_from_slice(&b[..ra]);

                        beq!(framec, framer, "whole frame {} pref#{}", gname, pi);
                        assert_eq!(c_cfree(cc), r_cfree(rc));

                        // round trip through the C decoder
                        let out = decompress_frame_c(&framec);
                        assert_eq!(out.len(), data.len(), "roundtrip len {} pref#{}", gname, pi);
                        assert_eq!(&out[..], &data[..], "roundtrip {} pref#{}", gname, pi);
                    }
                }
            }
        }
    }
}

/// Decompress a whole frame using the C library (reference implementation).
fn decompress_frame_c(frame: &[u8]) -> Vec<u8> {
    let (c_dnew, _) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, _) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_dec, _) = pair!(
        "LZ4F_decompress",
        fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const DecompressOptions,
        ) -> usize
    );
    let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);
    unsafe {
        let mut ctx: *mut c_void = std::ptr::null_mut();
        assert_eq!(c_dnew(&mut ctx, LZ4F_VERSION), 0);
        let mut out: Vec<u8> = Vec::new();
        let mut buf = vec![0u8; 1 << 16];
        let mut spos = 0usize;
        loop {
            let mut dsize = buf.len();
            let mut ssize = frame.len() - spos;
            let r = c_dec(
                ctx,
                buf.as_mut_ptr(),
                &mut dsize,
                frame[spos..].as_ptr(),
                &mut ssize,
                std::ptr::null(),
            );
            assert_eq!(c_iserr(r), 0, "reference frame decode failed");
            out.extend_from_slice(&buf[..dsize]);
            spos += ssize;
            if r == 0 {
                break;
            }
            if dsize == 0 && ssize == 0 {
                panic!("reference frame decode stalled");
            }
        }
        c_dfree(ctx);
        out
    }
}

#[test]
fn frame_streaming_decompression() {
    let (c_dnew, r_dnew) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_dec, r_dec) = pair!(
        "LZ4F_decompress",
        fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const DecompressOptions,
        ) -> usize
    );
    let (c_hs, r_hs) = pair!("LZ4F_headerSize", fn(*const u8, usize) -> usize);
    let (c_gfi, r_gfi) = pair!(
        "LZ4F_getFrameInfo",
        fn(*mut c_void, *mut FrameInfo, *const u8, *mut usize) -> usize
    );
    let (c_cf, _) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );

    let prefs = pref_matrix_small();
    // src chunk size, dst chunk size
    let splits: [(usize, usize); 6] = [
        (1, 1),
        (1, 1 << 16),
        (7, 13),
        (1 << 16, 1 << 16),
        (300, 65536),
        (usize::MAX, usize::MAX),
    ];
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 100, 4096, 70000] {
                let data = g(sz, 181 + sz as u64);
                for (pi, p) in prefs.iter().enumerate() {
                    let bound = c_fb(sz, p);
                    let mut frame = vec![0u8; bound + 64];
                    let n = c_cf(frame.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    frame.truncate(n);

                    // LZ4F_headerSize on prefixes of the frame
                    for k in 0..=frame.len().min(24) {
                        assert_eq!(
                            c_hs(frame.as_ptr(), k),
                            r_hs(frame.as_ptr(), k),
                            "headerSize k={} pref#{}",
                            k,
                            pi
                        );
                    }

                    for &(sc, dc) in &splits {
                        // skip the pathological 1-byte-at-a-time case for big inputs
                        if sc == 1 && dc == 1 && sz > 4096 {
                            continue;
                        }
                        let mut cd: *mut c_void = std::ptr::null_mut();
                        let mut rd: *mut c_void = std::ptr::null_mut();
                        assert_eq!(
                            c_dnew(&mut cd, LZ4F_VERSION),
                            r_dnew(&mut rd, LZ4F_VERSION)
                        );

                        // getFrameInfo before consuming payload
                        let mut fic = FrameInfo::default();
                        let mut fir = FrameInfo::default();
                        let mut sc1 = frame.len();
                        let mut sr1 = frame.len();
                        let ra = c_gfi(cd, &mut fic, frame.as_ptr(), &mut sc1);
                        let rb = r_gfi(rd, &mut fir, frame.as_ptr(), &mut sr1);
                        assert_eq!(ra, rb, "getFrameInfo {} sz={} pref#{}", gname, sz, pi);
                        assert_eq!(sc1, sr1, "getFrameInfo consumed {} pref#{}", gname, pi);
                        assert_eq!(fic, fir, "getFrameInfo info {} sz={} pref#{}", gname, sz, pi);

                        let mut outc: Vec<u8> = Vec::new();
                        let mut outr: Vec<u8> = Vec::new();
                        let mut bufc = vec![0u8; 1 << 16];
                        let mut bufr = vec![0u8; 1 << 16];
                        let mut posc = sc1;
                        let mut posr = sr1;
                        loop {
                            let mut dsc = dc.min(bufc.len());
                            let mut ssc = sc.min(frame.len() - posc);
                            let mut dsr = dc.min(bufr.len());
                            let mut ssr = sc.min(frame.len() - posr);
                            let ra = c_dec(
                                cd,
                                bufc.as_mut_ptr(),
                                &mut dsc,
                                frame[posc..].as_ptr(),
                                &mut ssc,
                                std::ptr::null(),
                            );
                            let rb = r_dec(
                                rd,
                                bufr.as_mut_ptr(),
                                &mut dsr,
                                frame[posr..].as_ptr(),
                                &mut ssr,
                                std::ptr::null(),
                            );
                            assert_eq!(
                                ra, rb,
                                "decompress hint {} sz={} pref#{} split=({},{}) pos={}",
                                gname, sz, pi, sc, dc, posc
                            );
                            assert_eq!(dsc, dsr, "decompress dstSize {} pref#{}", gname, pi);
                            assert_eq!(ssc, ssr, "decompress srcSize {} pref#{}", gname, pi);
                            beq!(
                                bufc[..dsc],
                                bufr[..dsr],
                                "decompress bytes {} sz={} pref#{}",
                                gname,
                                sz,
                                pi
                            );
                            outc.extend_from_slice(&bufc[..dsc]);
                            outr.extend_from_slice(&bufr[..dsr]);
                            posc += ssc;
                            posr += ssr;
                            if ra == 0 {
                                break;
                            }
                            if dsc == 0 && ssc == 0 {
                                panic!("decode stalled {} pref#{}", gname, pi);
                            }
                        }
                        beq!(outc, outr, "decompress output {} sz={} pref#{}", gname, sz, pi);
                        assert_eq!(&outc[..], &data[..], "roundtrip {} sz={} pref#{}", gname, sz, pi);
                        assert_eq!(c_dfree(cd), r_dfree(rd));
                    }
                }
            }
        }
    }
}

#[test]
fn frame_decompress_options_and_reset() {
    let (c_dnew, r_dnew) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_reset, r_reset) = pair!("LZ4F_resetDecompressionContext", fn(*mut c_void));
    let (c_dec, r_dec) = pair!(
        "LZ4F_decompress",
        fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const DecompressOptions,
        ) -> usize
    );
    let (c_cf, _) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    unsafe {
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        assert_eq!(c_dnew(&mut cd, LZ4F_VERSION), r_dnew(&mut rd, LZ4F_VERSION));
        for (gname, g) in GENS {
            for &sz in &[1usize, 4096, 70000] {
                let data = g(sz, 191 + sz as u64);
                for p in pref_matrix_small() {
                    let bound = c_fb(sz, &p);
                    let mut frame = vec![0u8; bound + 64];
                    let n = c_cf(frame.as_mut_ptr(), bound, data.as_ptr(), sz, &p);
                    frame.truncate(n);
                    for &(sdst, skip) in &[(0u32, 0u32), (0, 1), (1, 0), (1, 1)] {
                        let opts = DecompressOptions {
                            stable_dst: sdst,
                            skip_checksums: skip,
                            reserved1: 0,
                            reserved0: 0,
                        };
                        c_reset(cd);
                        r_reset(rd);
                        // stableDst pledges the last 64 KB of output stays put, so
                        // decode into one large contiguous buffer.
                        let mut bufc = vec![0u8; sz + (1 << 16)];
                        let mut bufr = vec![0u8; sz + (1 << 16)];
                        let mut offc = 0usize;
                        let mut offr = 0usize;
                        let mut pos = 0usize;
                        loop {
                            let mut dsc = (bufc.len() - offc).min(1 << 14);
                            let mut dsr = dsc;
                            let mut ssc = (frame.len() - pos).min(1 << 13);
                            let mut ssr = ssc;
                            let ra = c_dec(
                                cd,
                                bufc[offc..].as_mut_ptr(),
                                &mut dsc,
                                frame[pos..].as_ptr(),
                                &mut ssc,
                                &opts,
                            );
                            let rb = r_dec(
                                rd,
                                bufr[offr..].as_mut_ptr(),
                                &mut dsr,
                                frame[pos..].as_ptr(),
                                &mut ssr,
                                &opts,
                            );
                            assert_eq!(
                                ra, rb,
                                "decompress opts {} sz={} stableDst={} skip={}",
                                gname, sz, sdst, skip
                            );
                            assert_eq!(dsc, dsr);
                            assert_eq!(ssc, ssr);
                            offc += dsc;
                            offr += dsr;
                            pos += ssc;
                            if ra == 0 {
                                break;
                            }
                            if dsc == 0 && ssc == 0 {
                                panic!("stalled");
                            }
                        }
                        beq!(bufc[..offc], bufr[..offr], "decompress opts output");
                        assert_eq!(&bufc[..sz], &data[..]);
                    }
                }
            }
        }
        assert_eq!(c_dfree(cd), r_dfree(rd));
    }
}

#[test]
fn frame_decompress_corrupt_and_truncated() {
    let (c_dnew, r_dnew) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_dec, r_dec) = pair!(
        "LZ4F_decompress",
        fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const DecompressOptions,
        ) -> usize
    );
    let (c_hs, r_hs) = pair!("LZ4F_headerSize", fn(*const u8, usize) -> usize);
    let (c_gfi, r_gfi) = pair!(
        "LZ4F_getFrameInfo",
        fn(*mut c_void, *mut FrameInfo, *const u8, *mut usize) -> usize
    );
    let (c_cf, _) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );

    /// Feed `input` to both decoders in one shot and compare everything.
    unsafe fn run(
        input: &[u8],
        ctx: &str,
        cd: *mut c_void,
        rd: *mut c_void,
        c_dec: &impl Fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const DecompressOptions,
        ) -> usize,
        r_dec: &impl Fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const DecompressOptions,
        ) -> usize,
        c_reset: &impl Fn(*mut c_void),
        r_reset: &impl Fn(*mut c_void),
    ) {
        unsafe {
            let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);
            c_reset(cd);
            r_reset(rd);
            let mut bufc = vec![0xEBu8; 1 << 17];
            let mut bufr = vec![0xEBu8; 1 << 17];
            let mut pos = 0usize;
            let mut steps = 0;
            loop {
                let mut dsc = bufc.len();
                let mut dsr = bufr.len();
                let mut ssc = input.len() - pos;
                let mut ssr = ssc;
                let ra = c_dec(
                    cd,
                    bufc.as_mut_ptr(),
                    &mut dsc,
                    input[pos..].as_ptr(),
                    &mut ssc,
                    std::ptr::null(),
                );
                let rb = r_dec(
                    rd,
                    bufr.as_mut_ptr(),
                    &mut dsr,
                    input[pos..].as_ptr(),
                    &mut ssr,
                    std::ptr::null(),
                );
                assert_eq!(ra, rb, "{}: return", ctx);
                assert_eq!(dsc, dsr, "{}: dstSize", ctx);
                assert_eq!(ssc, ssr, "{}: srcSize", ctx);
                cmp_bytes(&bufc[..dsc], &bufr[..dsr], &format!("{}: output", ctx));
                if c_iserr(ra) != 0 || ra == 0 {
                    break;
                }
                pos += ssc;
                steps += 1;
                if (dsc == 0 && ssc == 0) || pos >= input.len() || steps > 64 {
                    break;
                }
            }
        }
    }

    let (c_reset, r_reset) = pair!("LZ4F_resetDecompressionContext", fn(*mut c_void));
    unsafe {
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        assert_eq!(c_dnew(&mut cd, LZ4F_VERSION), r_dnew(&mut rd, LZ4F_VERSION));
        let cdec = |a, b, c, d, e, f| c_dec(a, b, c, d, e, f);
        let rdec = |a, b, c, d, e, f| r_dec(a, b, c, d, e, f);
        let cres = |a| c_reset(a);
        let rres = |a| r_reset(a);

        let mut rng = Rng::new(0xBEEF);
        for (gname, g) in GENS {
            for &sz in &[100usize, 5000, 70000] {
                let data = g(sz, 201 + sz as u64);
                for (pi, p) in pref_matrix_small().iter().enumerate() {
                    let bound = c_fb(sz, p);
                    let mut frame = vec![0u8; bound + 64];
                    let n = c_cf(frame.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    frame.truncate(n);

                    // truncations
                    for cut in [1usize, 2, 3, 4, 7, 8, 16, n / 2, n - 1] {
                        if cut >= n {
                            continue;
                        }
                        run(
                            &frame[..n - cut],
                            &format!("trunc {} sz={} pref#{} cut={}", gname, sz, pi, cut),
                            cd,
                            rd,
                            &cdec,
                            &rdec,
                            &cres,
                            &rres,
                        );
                    }
                    // single-byte corruptions
                    for _ in 0..12 {
                        let mut f = frame.clone();
                        let i = (rng.below(f.len() as u32)) as usize;
                        f[i] ^= 1 << (rng.below(8));
                        run(
                            &f,
                            &format!("corrupt {} sz={} pref#{} at={}", gname, sz, pi, i),
                            cd,
                            rd,
                            &cdec,
                            &rdec,
                            &cres,
                            &rres,
                        );
                    }
                }
            }
        }

        // arbitrary garbage / handcrafted headers
        let mut headers: Vec<Vec<u8>> = Vec::new();
        headers.push(vec![]);
        headers.push(vec![0x04]);
        headers.push(vec![0x04, 0x22, 0x4D, 0x18]);
        for flg in 0u8..=255 {
            headers.push(vec![0x04, 0x22, 0x4D, 0x18, flg, 0x40, 0x00, 0, 0, 0, 0]);
        }
        // skippable frames
        for magic in [0x184D2A50u32, 0x184D2A5Fu32, 0x184D2A60u32] {
            let mut v = magic.to_le_bytes().to_vec();
            v.extend_from_slice(&16u32.to_le_bytes());
            v.extend_from_slice(&[0u8; 20]);
            headers.push(v);
        }
        for i in 0..64 {
            headers.push(gen_random(4 + i, 3000 + i as u64));
        }
        for (hi, h) in headers.iter().enumerate() {
            run(
                h,
                &format!("garbage #{}", hi),
                cd,
                rd,
                &cdec,
                &rdec,
                &cres,
                &rres,
            );
            // also compare headerSize / getFrameInfo on the raw bytes
            assert_eq!(
                c_hs(h.as_ptr(), h.len()),
                r_hs(h.as_ptr(), h.len()),
                "headerSize garbage #{}",
                hi
            );
            c_reset(cd);
            r_reset(rd);
            let mut fic = FrameInfo::default();
            let mut fir = FrameInfo::default();
            let mut sc = h.len();
            let mut sr = h.len();
            let ra = c_gfi(cd, &mut fic, h.as_ptr(), &mut sc);
            let rb = r_gfi(rd, &mut fir, h.as_ptr(), &mut sr);
            assert_eq!(ra, rb, "getFrameInfo garbage #{}", hi);
            assert_eq!(sc, sr, "getFrameInfo consumed garbage #{}", hi);
            assert_eq!(fic, fir, "getFrameInfo info garbage #{}", hi);
        }
        assert_eq!(c_dfree(cd), r_dfree(rd));
    }
}

#[test]
fn frame_dictionary_apis() {
    let (c_ccd, r_ccd) = pair!("LZ4F_createCDict", fn(*const u8, usize) -> *mut c_void);
    let (c_fcd, r_fcd) = pair!("LZ4F_freeCDict", fn(*mut c_void));
    let (c_cnew, r_cnew) = pair!(
        "LZ4F_createCompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_cfree, r_cfree) = pair!("LZ4F_freeCompressionContext", fn(*mut c_void) -> usize);
    let (c_cfd, r_cfd) = pair!(
        "LZ4F_compressFrame_usingCDict",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void, *const Preferences)
            -> usize
    );
    let (c_bud, r_bud) = pair!(
        "LZ4F_compressBegin_usingDict",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_budo, r_budo) = pair!(
        "LZ4F_compressBegin_usingDictOnce",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_bcd, r_bcd) = pair!(
        "LZ4F_compressBegin_usingCDict",
        fn(*mut c_void, *mut u8, usize, *const c_void, *const Preferences) -> usize
    );
    let (c_bint, r_bint) = pair!(
        "LZ4F_compressBegin_internal",
        fn(
            *mut c_void,
            *mut u8,
            usize,
            *const u8,
            usize,
            *const c_void,
            *const Preferences,
        ) -> usize
    );
    let (c_upd, r_upd) = pair!(
        "LZ4F_compressUpdate",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const CompressOptions) -> usize
    );
    let (c_end, r_end) = pair!(
        "LZ4F_compressEnd",
        fn(*mut c_void, *mut u8, usize, *const CompressOptions) -> usize
    );
    let (c_cb, _) = pair!("LZ4F_compressBound", fn(usize, *const Preferences) -> usize);
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    let (c_dnew, r_dnew) = pair!(
        "LZ4F_createDecompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_dud, r_dud) = pair!(
        "LZ4F_decompress_usingDict",
        fn(
            *mut c_void,
            *mut u8,
            *mut usize,
            *const u8,
            *mut usize,
            *const u8,
            usize,
            *const DecompressOptions,
        ) -> usize
    );
    let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);

    let prefs = pref_matrix_tiny();
    unsafe {
        for (gname, g) in GENS.iter().take(3) {
            for &dsz in &[0usize, 1, 1000, 65536] {
                let dict = g(dsz, 211 + dsz as u64);
                let cdict = c_ccd(dict.as_ptr(), dsz);
                let rdict = r_ccd(dict.as_ptr(), dsz);
                assert_eq!(
                    cdict.is_null(),
                    rdict.is_null(),
                    "createCDict {} dsz={}",
                    gname,
                    dsz
                );
                for &sz in &[0usize, 1, 1000, 70000] {
                    let data = g(sz, 213 + sz as u64);
                    for (pi, p) in prefs.iter().enumerate() {
                        let bound = c_fb(sz, p);

                        // compressFrame_usingCDict
                        let mut cc: *mut c_void = std::ptr::null_mut();
                        let mut rc: *mut c_void = std::ptr::null_mut();
                        c_cnew(&mut cc, LZ4F_VERSION);
                        r_cnew(&mut rc, LZ4F_VERSION);
                        let mut a = vec![0u8; bound + 64];
                        let mut b = vec![0u8; bound + 64];
                        let ra = c_cfd(cc, a.as_mut_ptr(), bound, data.as_ptr(), sz, cdict, p);
                        let rb = r_cfd(rc, b.as_mut_ptr(), bound, data.as_ptr(), sz, rdict, p);
                        assert_eq!(
                            ra, rb,
                            "compressFrame_usingCDict {} dsz={} sz={} pref#{}",
                            gname, dsz, sz, pi
                        );
                        beq!(
                            a[..ra.min(a.len())],
                            b[..rb.min(b.len())],
                            "compressFrame_usingCDict bytes {} dsz={} sz={} pref#{}",
                            gname,
                            dsz,
                            sz,
                            pi
                        );
                        if c_iserr(ra) == 0 {
                            // decode with the dictionary
                            let out = decode_using_dict(
                                &a[..ra],
                                &dict[..dsz],
                                (&c_dud, &r_dud),
                                (&c_dnew, &r_dnew),
                                (&c_dfree, &r_dfree),
                            );
                            assert_eq!(&out[..], &data[..], "CDict roundtrip {} dsz={}", gname, dsz);
                        }
                        assert_eq!(c_cfree(cc), r_cfree(rc));

                        // the four compressBegin_* variants
                        for variant in 0..4 {
                            let mut cc: *mut c_void = std::ptr::null_mut();
                            let mut rc: *mut c_void = std::ptr::null_mut();
                            c_cnew(&mut cc, LZ4F_VERSION);
                            r_cnew(&mut rc, LZ4F_VERSION);
                            let mut ha = vec![0u8; 64];
                            let mut hb = vec![0u8; 64];
                            let (ra, rb) = match variant {
                                0 => (
                                    c_bud(cc, ha.as_mut_ptr(), ha.len(), dict.as_ptr(), dsz, p),
                                    r_bud(rc, hb.as_mut_ptr(), hb.len(), dict.as_ptr(), dsz, p),
                                ),
                                1 => (
                                    c_budo(cc, ha.as_mut_ptr(), ha.len(), dict.as_ptr(), dsz, p),
                                    r_budo(rc, hb.as_mut_ptr(), hb.len(), dict.as_ptr(), dsz, p),
                                ),
                                2 => (
                                    c_bcd(cc, ha.as_mut_ptr(), ha.len(), cdict, p),
                                    r_bcd(rc, hb.as_mut_ptr(), hb.len(), rdict, p),
                                ),
                                _ => (
                                    c_bint(
                                        cc,
                                        ha.as_mut_ptr(),
                                        ha.len(),
                                        dict.as_ptr(),
                                        dsz,
                                        std::ptr::null(),
                                        p,
                                    ),
                                    r_bint(
                                        rc,
                                        hb.as_mut_ptr(),
                                        hb.len(),
                                        dict.as_ptr(),
                                        dsz,
                                        std::ptr::null(),
                                        p,
                                    ),
                                ),
                            };
                            assert_eq!(
                                ra, rb,
                                "compressBegin variant{} {} dsz={} pref#{}",
                                variant, gname, dsz, pi
                            );
                            assert_eq!(c_iserr(ra), 0);
                            beq!(
                                ha[..ra],
                                hb[..rb],
                                "compressBegin variant{} header {} dsz={} pref#{}",
                                variant,
                                gname,
                                dsz,
                                pi
                            );
                            let mut framec = ha[..ra].to_vec();
                            let mut framer = hb[..rb].to_vec();

                            let cap = c_cb(sz, p);
                            let mut a = vec![0u8; cap + 32];
                            let mut b = vec![0u8; cap + 32];
                            let ra = c_upd(
                                cc,
                                a.as_mut_ptr(),
                                cap,
                                data.as_ptr(),
                                sz,
                                std::ptr::null(),
                            );
                            let rb = r_upd(
                                rc,
                                b.as_mut_ptr(),
                                cap,
                                data.as_ptr(),
                                sz,
                                std::ptr::null(),
                            );
                            assert_eq!(
                                ra, rb,
                                "dict update variant{} {} dsz={} sz={} pref#{}",
                                variant, gname, dsz, sz, pi
                            );
                            beq!(a[..ra], b[..rb], "dict update bytes variant{}", variant);
                            framec.extend_from_slice(&a[..ra]);
                            framer.extend_from_slice(&b[..rb]);

                            let cap = c_cb(0, p);
                            let mut a = vec![0u8; cap + 32];
                            let mut b = vec![0u8; cap + 32];
                            let ra = c_end(cc, a.as_mut_ptr(), cap, std::ptr::null());
                            let rb = r_end(rc, b.as_mut_ptr(), cap, std::ptr::null());
                            assert_eq!(ra, rb, "dict end variant{} {}", variant, gname);
                            beq!(a[..ra], b[..rb], "dict end bytes variant{}", variant);
                            framec.extend_from_slice(&a[..ra]);
                            framer.extend_from_slice(&b[..rb]);
                            beq!(framec, framer, "dict frame variant{} {}", variant, gname);

                            let out = decode_using_dict(
                                &framec,
                                &dict[..dsz],
                                (&c_dud, &r_dud),
                                (&c_dnew, &r_dnew),
                                (&c_dfree, &r_dfree),
                            );
                            assert_eq!(
                                &out[..],
                                &data[..],
                                "dict roundtrip variant{} {} dsz={} sz={}",
                                variant,
                                gname,
                                dsz,
                                sz
                            );
                            assert_eq!(c_cfree(cc), r_cfree(rc));
                        }
                    }
                }
                c_fcd(cdict);
                r_fcd(rdict);
            }
        }
        // free NULL
        c_fcd(std::ptr::null_mut());
        r_fcd(std::ptr::null_mut());
    }
}

type DudFn = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const u8,
    usize,
    *const DecompressOptions,
) -> usize;
type DNewFn = unsafe extern "C" fn(*mut *mut c_void, u32) -> usize;
type DFreeFn = unsafe extern "C" fn(*mut c_void) -> usize;

/// Decode `frame` with both libraries using `LZ4F_decompress_usingDict`,
/// asserting they agree, and return the decoded bytes.
#[allow(clippy::too_many_arguments)]
fn decode_using_dict(
    frame: &[u8],
    dict: &[u8],
    dud: (&DudFn, &DudFn),
    dnew: (&DNewFn, &DNewFn),
    dfree: (&DFreeFn, &DFreeFn),
) -> Vec<u8> {
    let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);
    unsafe {
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        assert_eq!(dnew.0(&mut cd, LZ4F_VERSION), dnew.1(&mut rd, LZ4F_VERSION));
        let mut outc: Vec<u8> = Vec::new();
        let mut outr: Vec<u8> = Vec::new();
        let mut bufc = vec![0u8; 1 << 16];
        let mut bufr = vec![0u8; 1 << 16];
        let mut pos = 0usize;
        loop {
            let mut dsc = bufc.len();
            let mut dsr = bufr.len();
            let mut ssc = frame.len() - pos;
            let mut ssr = ssc;
            let ra = dud.0(
                cd,
                bufc.as_mut_ptr(),
                &mut dsc,
                frame[pos..].as_ptr(),
                &mut ssc,
                dict.as_ptr(),
                dict.len(),
                std::ptr::null(),
            );
            let rb = dud.1(
                rd,
                bufr.as_mut_ptr(),
                &mut dsr,
                frame[pos..].as_ptr(),
                &mut ssr,
                dict.as_ptr(),
                dict.len(),
                std::ptr::null(),
            );
            assert_eq!(ra, rb, "decompress_usingDict return");
            assert_eq!(dsc, dsr, "decompress_usingDict dstSize");
            assert_eq!(ssc, ssr, "decompress_usingDict srcSize");
            cmp_bytes(&bufc[..dsc], &bufr[..dsr], "decompress_usingDict output");
            assert_eq!(c_iserr(ra), 0, "decompress_usingDict failed");
            outc.extend_from_slice(&bufc[..dsc]);
            outr.extend_from_slice(&bufr[..dsr]);
            pos += ssc;
            if ra == 0 {
                break;
            }
            if dsc == 0 && ssc == 0 {
                panic!("decompress_usingDict stalled");
            }
        }
        assert_eq!(dfree.0(cd), dfree.1(rd));
        assert_eq!(outc, outr);
        outc
    }
}

#[test]
fn frame_uncompressed_update() {
    let (c_cnew, r_cnew) = pair!(
        "LZ4F_createCompressionContext",
        fn(*mut *mut c_void, u32) -> usize
    );
    let (c_cfree, r_cfree) = pair!("LZ4F_freeCompressionContext", fn(*mut c_void) -> usize);
    let (c_begin, r_begin) = pair!(
        "LZ4F_compressBegin",
        fn(*mut c_void, *mut u8, usize, *const Preferences) -> usize
    );
    let (c_uu, r_uu) = pair!(
        "LZ4F_uncompressedUpdate",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const CompressOptions) -> usize
    );
    let (c_upd, r_upd) = pair!(
        "LZ4F_compressUpdate",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const CompressOptions) -> usize
    );
    let (c_end, r_end) = pair!(
        "LZ4F_compressEnd",
        fn(*mut c_void, *mut u8, usize, *const CompressOptions) -> usize
    );
    let (c_cb, _) = pair!("LZ4F_compressBound", fn(usize, *const Preferences) -> usize);
    let (c_iserr, _) = pair!("LZ4F_isError", fn(usize) -> u32);
    unsafe {
        for (gname, g) in GENS {
            let data = g(60_000, 221 + gname.len() as u64);
            for &bs in &[0u32, 4, 5, 6, 7] {
                for &bm in &[0u32, 1] {
                    for &cc_flag in &[0u32, 1] {
                        for &bc in &[0u32, 1] {
                            let mut p = Preferences::default();
                            p.frame_info.block_size_id = bs;
                            p.frame_info.block_mode = bm;
                            p.frame_info.content_checksum_flag = cc_flag;
                            p.frame_info.block_checksum_flag = bc;
                            let mut ctxc: *mut c_void = std::ptr::null_mut();
                            let mut ctxr: *mut c_void = std::ptr::null_mut();
                            c_cnew(&mut ctxc, LZ4F_VERSION);
                            r_cnew(&mut ctxr, LZ4F_VERSION);
                            let mut ha = vec![0u8; 64];
                            let mut hb = vec![0u8; 64];
                            let ra = c_begin(ctxc, ha.as_mut_ptr(), ha.len(), &p);
                            let rb = r_begin(ctxr, hb.as_mut_ptr(), hb.len(), &p);
                            assert_eq!(ra, rb);
                            let mut framec = ha[..ra].to_vec();
                            let mut framer = hb[..rb].to_vec();

                            let mut pos = 0usize;
                            let mut round = 0usize;
                            while pos < data.len() {
                                let n = [1usize, 100, 4096, 20000][round % 4].min(data.len() - pos);
                                let cap = c_cb(n, &p).max(n + 64);
                                let mut a = vec![0u8; cap + 64];
                                let mut b = vec![0u8; cap + 64];
                                let (ra, rb) = if round % 2 == 0 {
                                    (
                                        c_uu(
                                            ctxc,
                                            a.as_mut_ptr(),
                                            cap,
                                            data[pos..].as_ptr(),
                                            n,
                                            std::ptr::null(),
                                        ),
                                        r_uu(
                                            ctxr,
                                            b.as_mut_ptr(),
                                            cap,
                                            data[pos..].as_ptr(),
                                            n,
                                            std::ptr::null(),
                                        ),
                                    )
                                } else {
                                    (
                                        c_upd(
                                            ctxc,
                                            a.as_mut_ptr(),
                                            cap,
                                            data[pos..].as_ptr(),
                                            n,
                                            std::ptr::null(),
                                        ),
                                        r_upd(
                                            ctxr,
                                            b.as_mut_ptr(),
                                            cap,
                                            data[pos..].as_ptr(),
                                            n,
                                            std::ptr::null(),
                                        ),
                                    )
                                };
                                assert_eq!(
                                    ra, rb,
                                    "uncompressedUpdate {} bs={} bm={} pos={} round={}",
                                    gname, bs, bm, pos, round
                                );
                                if c_iserr(ra) != 0 {
                                    // both agreed on the error; stop this frame
                                    break;
                                }
                                beq!(
                                    a[..ra],
                                    b[..rb],
                                    "uncompressedUpdate bytes {} bs={} pos={}",
                                    gname,
                                    bs,
                                    pos
                                );
                                framec.extend_from_slice(&a[..ra]);
                                framer.extend_from_slice(&b[..rb]);
                                pos += n;
                                round += 1;
                            }

                            let cap = c_cb(0, &p);
                            let mut a = vec![0u8; cap + 64];
                            let mut b = vec![0u8; cap + 64];
                            let ra = c_end(ctxc, a.as_mut_ptr(), cap, std::ptr::null());
                            let rb = r_end(ctxr, b.as_mut_ptr(), cap, std::ptr::null());
                            assert_eq!(ra, rb, "uncompressedUpdate end {} bs={}", gname, bs);
                            if c_iserr(ra) == 0 {
                                beq!(a[..ra], b[..rb], "uncompressedUpdate end bytes");
                                framec.extend_from_slice(&a[..ra]);
                                framer.extend_from_slice(&b[..rb]);
                                beq!(framec, framer, "uncompressedUpdate frame {} bs={}", gname, bs);
                                // LZ4F_uncompressedUpdate is only defined for
                                // independent blocks; with linked blocks the C
                                // implementation still produces output but the
                                // resulting frame is not decodable, so only the
                                // C-vs-Rust agreement above is meaningful there.
                                if bm == 1 && pos == data.len() {
                                    let out = decompress_frame_c(&framec);
                                    assert_eq!(&out[..], &data[..], "uncompressedUpdate roundtrip");
                                }
                            }
                            assert_eq!(c_cfree(ctxc), r_cfree(ctxr));
                        }
                    }
                }
            }
        }
    }
}

/// `LZ4F_CustomMem` — four pointer-sized fields, so 32 bytes. The SysV AMD64 ABI
/// classifies it as MEMORY (it exceeds 16 bytes), meaning it is passed on the
/// stack rather than in registers. It must therefore be declared as a by-value
/// `#[repr(C)]` struct, not flattened into separate arguments.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CustomMem {
    pub custom_alloc: *const c_void,
    pub custom_calloc: *const c_void,
    pub custom_free: *const c_void,
    pub opaque_state: *mut c_void,
}

impl Default for CustomMem {
    /// `LZ4F_defaultCMem` — all NULL, meaning "use stdlib".
    fn default() -> Self {
        CustomMem {
            custom_alloc: std::ptr::null(),
            custom_calloc: std::ptr::null(),
            custom_free: std::ptr::null(),
            opaque_state: std::ptr::null_mut(),
        }
    }
}

#[test]
fn frame_advanced_constructors() {
    let (c_ca, r_ca) = pair!(
        "LZ4F_createCompressionContext_advanced",
        fn(CustomMem, u32) -> *mut c_void
    );
    let (c_da, r_da) = pair!(
        "LZ4F_createDecompressionContext_advanced",
        fn(CustomMem, u32) -> *mut c_void
    );
    let (c_cda, r_cda) = pair!(
        "LZ4F_createCDict_advanced",
        fn(CustomMem, *const u8, usize) -> *mut c_void
    );
    let (c_cfree, r_cfree) = pair!("LZ4F_freeCompressionContext", fn(*mut c_void) -> usize);
    let (c_dfree, r_dfree) = pair!("LZ4F_freeDecompressionContext", fn(*mut c_void) -> usize);
    let (c_fcd, r_fcd) = pair!("LZ4F_freeCDict", fn(*mut c_void));
    let (c_begin, r_begin) = pair!(
        "LZ4F_compressBegin",
        fn(*mut c_void, *mut u8, usize, *const Preferences) -> usize
    );
    let (c_upd, r_upd) = pair!(
        "LZ4F_compressUpdate",
        fn(*mut c_void, *mut u8, usize, *const u8, usize, *const CompressOptions) -> usize
    );
    let (c_end, r_end) = pair!(
        "LZ4F_compressEnd",
        fn(*mut c_void, *mut u8, usize, *const CompressOptions) -> usize
    );
    let (c_cb, _) = pair!("LZ4F_compressBound", fn(usize, *const Preferences) -> usize);
    unsafe {
        let cm = CustomMem::default();
        for ver in [0u32, 1, LZ4F_VERSION, LZ4F_VERSION + 1] {
            let a = c_ca(cm, ver);
            let b = r_ca(cm, ver);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "createCompressionContext_advanced ver={}",
                ver
            );
            if !a.is_null() {
                // the context must actually work
                let data = gen_textish(5000, 241);
                let p = Preferences::default();
                let mut ha = vec![0u8; 64];
                let mut hb = vec![0u8; 64];
                let ra = c_begin(a, ha.as_mut_ptr(), ha.len(), &p);
                let rb = r_begin(b, hb.as_mut_ptr(), hb.len(), &p);
                assert_eq!(ra, rb, "advanced cctx begin ver={}", ver);
                beq!(ha[..ra], hb[..rb], "advanced cctx header ver={}", ver);
                let cap = c_cb(data.len(), &p);
                let mut ba = vec![0u8; cap + 64];
                let mut bb = vec![0u8; cap + 64];
                let ua = c_upd(
                    a,
                    ba.as_mut_ptr(),
                    cap,
                    data.as_ptr(),
                    data.len(),
                    std::ptr::null(),
                );
                let ub = r_upd(
                    b,
                    bb.as_mut_ptr(),
                    cap,
                    data.as_ptr(),
                    data.len(),
                    std::ptr::null(),
                );
                assert_eq!(ua, ub, "advanced cctx update ver={}", ver);
                beq!(ba[..ua], bb[..ub], "advanced cctx update bytes ver={}", ver);
                let cap = c_cb(0, &p);
                let mut ea = vec![0u8; cap + 64];
                let mut eb = vec![0u8; cap + 64];
                let fa = c_end(a, ea.as_mut_ptr(), cap, std::ptr::null());
                let fb = r_end(b, eb.as_mut_ptr(), cap, std::ptr::null());
                assert_eq!(fa, fb, "advanced cctx end ver={}", ver);
                beq!(ea[..fa], eb[..fb], "advanced cctx end bytes ver={}", ver);
            }
            assert_eq!(c_cfree(a), r_cfree(b));

            let a = c_da(cm, ver);
            let b = r_da(cm, ver);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "createDecompressionContext_advanced ver={}",
                ver
            );
            assert_eq!(c_dfree(a), r_dfree(b));
        }
        for &dsz in &[0usize, 1, 1000, 70000] {
            let dict = gen_textish(dsz, 231 + dsz as u64);
            let a = c_cda(cm, dict.as_ptr(), dsz);
            let b = r_cda(cm, dict.as_ptr(), dsz);
            assert_eq!(a.is_null(), b.is_null(), "createCDict_advanced dsz={}", dsz);
            c_fcd(a);
            r_fcd(b);
        }
    }
}

//! Phase B — frame API (lz4frame.c) differential tests. CONFIGS.md rows 79-137.
//!
//! Every call goes through the `.so` exports of BOTH implementations.

mod common;
use common::*;

// ------------------------------------------------------------- ABI structs

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

// ------------------------------------------------------------------- symbols

type FnCompressFrame =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const Prefs) -> usize;
type FnFrameBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;
type FnCreateCctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
type FnFreeCctx = unsafe extern "C" fn(*mut u8) -> usize;
type FnCompressBegin = unsafe extern "C" fn(*mut u8, *mut u8, usize, *const Prefs) -> usize;
type FnCompressUpdate = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const CompressOptions,
) -> usize;
type FnFlush = unsafe extern "C" fn(*mut u8, *mut u8, usize, *const CompressOptions) -> usize;
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
type FnGetFrameInfo = unsafe extern "C" fn(*mut u8, *mut FrameInfo, *const u8, *mut usize) -> usize;
type FnHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnResetDctx = unsafe extern "C" fn(*mut u8);
type FnBeginUsingDict =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, usize, *const Prefs) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const u8, usize) -> *mut u8;
type FnFreeCDict = unsafe extern "C" fn(*mut u8);
type FnFrameUsingCDict = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const Prefs,
) -> usize;
type FnBeginUsingCDict =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, *const Prefs) -> usize;
type FnBeginInternal = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const Prefs,
) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> u32;
type FnErrName = unsafe extern "C" fn(usize) -> *const std::os::raw::c_char;
type FnErrCode = unsafe extern "C" fn(usize) -> i32;
type FnGetBlockSize = unsafe extern "C" fn(i32) -> usize;
type FnGetVersion = unsafe extern "C" fn() -> u32;
type FnLevelMax = unsafe extern "C" fn() -> i32;
type FnCompressBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;

/// A full set of frame entry points for ONE implementation.
struct FOps {
    frame: libloading::Symbol<'static, FnCompressFrame>,
    frame_bound: libloading::Symbol<'static, FnFrameBound>,
    bound: libloading::Symbol<'static, FnCompressBound>,
    create_c: libloading::Symbol<'static, FnCreateCctx>,
    free_c: libloading::Symbol<'static, FnFreeCctx>,
    begin: libloading::Symbol<'static, FnCompressBegin>,
    update: libloading::Symbol<'static, FnCompressUpdate>,
    uncompressed_update: libloading::Symbol<'static, FnCompressUpdate>,
    flush: libloading::Symbol<'static, FnFlush>,
    end: libloading::Symbol<'static, FnFlush>,
    create_d: libloading::Symbol<'static, FnCreateDctx>,
    free_d: libloading::Symbol<'static, FnFreeDctx>,
    decompress: libloading::Symbol<'static, FnDecompress>,
    decompress_dict: libloading::Symbol<'static, FnDecompressUsingDict>,
    frame_info: libloading::Symbol<'static, FnGetFrameInfo>,
    header_size: libloading::Symbol<'static, FnHeaderSize>,
    reset_d: libloading::Symbol<'static, FnResetDctx>,
    begin_dict: libloading::Symbol<'static, FnBeginUsingDict>,
    begin_dict_once: libloading::Symbol<'static, FnBeginUsingDict>,
    create_cdict: libloading::Symbol<'static, FnCreateCDict>,
    free_cdict: libloading::Symbol<'static, FnFreeCDict>,
    frame_cdict: libloading::Symbol<'static, FnFrameUsingCDict>,
    begin_cdict: libloading::Symbol<'static, FnBeginUsingCDict>,
    begin_internal: libloading::Symbol<'static, FnBeginInternal>,
    is_error: libloading::Symbol<'static, FnIsError>,
}

fn fops() -> (FOps, FOps) {
    macro_rules! g {
        ($n:literal, $t:ty) => {
            sym::<$t>($n)
        };
    }
    let (a_frame, b_frame) = g!("LZ4F_compressFrame", FnCompressFrame);
    let (a_fb, b_fb) = g!("LZ4F_compressFrameBound", FnFrameBound);
    let (a_bd, b_bd) = g!("LZ4F_compressBound", FnCompressBound);
    let (a_cc, b_cc) = g!("LZ4F_createCompressionContext", FnCreateCctx);
    let (a_fc, b_fc) = g!("LZ4F_freeCompressionContext", FnFreeCctx);
    let (a_bg, b_bg) = g!("LZ4F_compressBegin", FnCompressBegin);
    let (a_up, b_up) = g!("LZ4F_compressUpdate", FnCompressUpdate);
    let (a_uu, b_uu) = g!("LZ4F_uncompressedUpdate", FnCompressUpdate);
    let (a_fl, b_fl) = g!("LZ4F_flush", FnFlush);
    let (a_en, b_en) = g!("LZ4F_compressEnd", FnFlush);
    let (a_cd, b_cd) = g!("LZ4F_createDecompressionContext", FnCreateDctx);
    let (a_fd, b_fd) = g!("LZ4F_freeDecompressionContext", FnFreeDctx);
    let (a_dc, b_dc) = g!("LZ4F_decompress", FnDecompress);
    let (a_dd, b_dd) = g!("LZ4F_decompress_usingDict", FnDecompressUsingDict);
    let (a_fi, b_fi) = g!("LZ4F_getFrameInfo", FnGetFrameInfo);
    let (a_hs, b_hs) = g!("LZ4F_headerSize", FnHeaderSize);
    let (a_rd, b_rd) = g!("LZ4F_resetDecompressionContext", FnResetDctx);
    let (a_bd1, b_bd1) = g!("LZ4F_compressBegin_usingDict", FnBeginUsingDict);
    let (a_bd2, b_bd2) = g!("LZ4F_compressBegin_usingDictOnce", FnBeginUsingDict);
    let (a_ccd, b_ccd) = g!("LZ4F_createCDict", FnCreateCDict);
    let (a_fcd, b_fcd) = g!("LZ4F_freeCDict", FnFreeCDict);
    let (a_fcc, b_fcc) = g!("LZ4F_compressFrame_usingCDict", FnFrameUsingCDict);
    let (a_bcc, b_bcc) = g!("LZ4F_compressBegin_usingCDict", FnBeginUsingCDict);
    let (a_bi, b_bi) = g!("LZ4F_compressBegin_internal", FnBeginInternal);
    let (a_ie, b_ie) = g!("LZ4F_isError", FnIsError);
    (
        FOps {
            frame: a_frame,
            frame_bound: a_fb,
            bound: a_bd,
            create_c: a_cc,
            free_c: a_fc,
            begin: a_bg,
            update: a_up,
            uncompressed_update: a_uu,
            flush: a_fl,
            end: a_en,
            create_d: a_cd,
            free_d: a_fd,
            decompress: a_dc,
            decompress_dict: a_dd,
            frame_info: a_fi,
            header_size: a_hs,
            reset_d: a_rd,
            begin_dict: a_bd1,
            begin_dict_once: a_bd2,
            create_cdict: a_ccd,
            free_cdict: a_fcd,
            frame_cdict: a_fcc,
            begin_cdict: a_bcc,
            begin_internal: a_bi,
            is_error: a_ie,
        },
        FOps {
            frame: b_frame,
            frame_bound: b_fb,
            bound: b_bd,
            create_c: b_cc,
            free_c: b_fc,
            begin: b_bg,
            update: b_up,
            uncompressed_update: b_uu,
            flush: b_fl,
            end: b_en,
            create_d: b_cd,
            free_d: b_fd,
            decompress: b_dc,
            decompress_dict: b_dd,
            frame_info: b_fi,
            header_size: b_hs,
            reset_d: b_rd,
            begin_dict: b_bd1,
            begin_dict_once: b_bd2,
            create_cdict: b_ccd,
            free_cdict: b_fcd,
            frame_cdict: b_fcc,
            begin_cdict: b_bcc,
            begin_internal: b_bi,
            is_error: b_ie,
        },
    )
}

const LZ4F_VERSION: u32 = 100;

fn is_err(o: &FOps, code: usize) -> bool {
    unsafe { (o.is_error)(code) != 0 }
}

/// The configuration matrix Phase B sweeps. Derived from CONFIGS.md axes.
fn prefs_matrix() -> Vec<Prefs> {
    let mut v = Vec::new();
    for &bsid in &[0i32, 4, 5, 6, 7] {
        for &bmode in &[0i32, 1] {
            for &ccs in &[0i32, 1] {
                for &bcs in &[0i32, 1] {
                    for &lvl in &[0i32, 1, 3, 9, 12] {
                        for &af in &[0u32, 1] {
                            v.push(Prefs {
                                frame_info: FrameInfo {
                                    block_size_id: bsid,
                                    block_mode: bmode,
                                    content_checksum_flag: ccs,
                                    frame_type: 0,
                                    content_size: 0,
                                    dict_id: 0,
                                    block_checksum_flag: bcs,
                                },
                                compression_level: lvl,
                                auto_flush: af,
                                favor_dec_speed: 0,
                                reserved: [0; 3],
                            });
                        }
                    }
                }
            }
        }
    }
    // favorDecSpeed only matters for level >= LZ4HC_CLEVEL_OPT_MIN
    for &lvl in &[9i32, 10, 11, 12] {
        for &fds in &[0u32, 1] {
            v.push(Prefs {
                frame_info: FrameInfo {
                    block_size_id: 4,
                    ..Default::default()
                },
                compression_level: lvl,
                favor_dec_speed: fds,
                ..Default::default()
            });
        }
    }
    // negative levels select "fast acceleration"
    for &lvl in &[-1i32, -5, -100] {
        v.push(Prefs {
            frame_info: FrameInfo {
                block_size_id: 5,
                content_checksum_flag: 1,
                ..Default::default()
            },
            compression_level: lvl,
            ..Default::default()
        });
    }
    // dictID present
    for &did in &[1u32, 0xDEAD_BEEF] {
        v.push(Prefs {
            frame_info: FrameInfo {
                block_size_id: 4,
                dict_id: did,
                content_checksum_flag: 1,
                ..Default::default()
            },
            ..Default::default()
        });
    }
    v
}

// ================================================== rows 79-94 one-shot frame

#[test]
fn row79_91_compress_frame_matrix() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0079);
    let mat = prefs_matrix();

    // row 79: prefs == NULL
    for &shape in &SHAPES {
        for &len in BOUNDARY_SIZES.iter() {
            let src = make_data(&mut rng, len, shape);
            let (cb, rb) = unsafe {
                (
                    (co.frame_bound)(len, std::ptr::null()),
                    (ro.frame_bound)(len, std::ptr::null()),
                )
            };
            eq(&format!("frameBound(NULL) len={len}"), cb, rb);
            let mut cd = vec![0xA5u8; cb + 32];
            let mut rd = vec![0xA5u8; cb + 32];
            let (a, b) = unsafe {
                (
                    (co.frame)(cd.as_mut_ptr(), cb, src.as_ptr(), len, std::ptr::null()),
                    (ro.frame)(rd.as_mut_ptr(), cb, src.as_ptr(), len, std::ptr::null()),
                )
            };
            let ctx = format!("compressFrame(NULL prefs) shape={shape:?} len={len}");
            eq(&ctx, a, b);
            assert!(!is_err(&co, a), "{ctx}: C errored: {a}");
            eq_bytes(&ctx, &cd[..a], &rd[..b]);
        }
    }

    // rows 80-91: the full configuration matrix
    for (pi, p) in mat.iter().enumerate() {
        for &shape in &SHAPES {
            // sizes chosen relative to the block size in play
            let bs = unsafe { (co.frame_bound)(1, p) };
            let _ = bs;
            let sizes = [
                0usize,
                1,
                2,
                100,
                65535,
                65536,
                65537,
                200_000,
                rng.range(1, 300_000),
            ];
            for &len in &sizes {
                let src = make_data(&mut rng, len, shape);
                let (cb, rb) =
                    unsafe { ((co.frame_bound)(len, p), (ro.frame_bound)(len, p)) };
                eq(&format!("frameBound p={pi} len={len}"), cb, rb);
                let (cn, rn) = unsafe { ((co.bound)(len, p), (ro.bound)(len, p)) };
                eq(&format!("compressBound p={pi} len={len}"), cn, rn);

                let mut cd = vec![0xA5u8; cb + 32];
                let mut rd = vec![0xA5u8; cb + 32];
                let (a, b) = unsafe {
                    (
                        (co.frame)(cd.as_mut_ptr(), cb, src.as_ptr(), len, p),
                        (ro.frame)(rd.as_mut_ptr(), cb, src.as_ptr(), len, p),
                    )
                };
                let ctx = format!("compressFrame p={pi} {p:?} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                if !is_err(&co, a) {
                    eq_bytes(&ctx, &cd[..a], &rd[..b]);
                    eq_bytes(&format!("{ctx} full"), &cd, &rd);
                }
            }
        }
    }
}

#[test]
fn row85_content_size_declared() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0085);
    for &shape in &SHAPES {
        for &bsid in &[0i32, 4, 5, 6, 7] {
            for &ccs in &[0i32, 1] {
                for &len in &[0usize, 1, 100, 65536, 65537, 200_000] {
                    let src = make_data(&mut rng, len, shape);
                    let p = Prefs {
                        frame_info: FrameInfo {
                            block_size_id: bsid,
                            content_checksum_flag: ccs,
                            content_size: len as u64,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    let cb = unsafe { (co.frame_bound)(len, &p) };
                    let mut cd = vec![0u8; cb + 32];
                    let mut rd = vec![0u8; cb + 32];
                    let (a, b) = unsafe {
                        (
                            (co.frame)(cd.as_mut_ptr(), cb, src.as_ptr(), len, &p),
                            (ro.frame)(rd.as_mut_ptr(), cb, src.as_ptr(), len, &p),
                        )
                    };
                    let ctx =
                        format!("contentSize bsid={bsid} ccs={ccs} shape={shape:?} len={len}");
                    eq(&ctx, a, b);
                    if !is_err(&co, a) {
                        eq_bytes(&ctx, &cd[..a], &rd[..b]);
                    }
                }
            }
        }
    }
}

#[test]
fn row92_94_cdict_and_constants() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0092);

    // row 94: plain constants
    {
        let (c, r) = sym::<FnGetVersion>("LZ4F_getVersion");
        eq("getVersion", unsafe { c() }, unsafe { r() });
        let (c, r) = sym::<FnLevelMax>("LZ4F_compressionLevel_max");
        eq("compressionLevel_max", unsafe { c() }, unsafe { r() });
        let (c, r) = sym::<FnGetBlockSize>("LZ4F_getBlockSize");
        for id in [0i32, 4, 5, 6, 7] {
            eq(&format!("getBlockSize({id})"), unsafe { c(id) }, unsafe {
                r(id)
            });
        }
    }

    // row 92/93: CDict paths
    for &dictlen in &[0usize, 1, 100, 4096, 65535, 65536, 120_000] {
        for &shape in &SHAPES {
            let dict = make_data(&mut rng, dictlen, shape);
            for &lvl in &[0i32, 1, 3, 9, 12, -3] {
                for &bsid in &[4i32, 7] {
                    let p = Prefs {
                        frame_info: FrameInfo {
                            block_size_id: bsid,
                            content_checksum_flag: 1,
                            ..Default::default()
                        },
                        compression_level: lvl,
                        ..Default::default()
                    };
                    let len = rng.range(1, 120_000);
                    let mut src = make_data(&mut rng, len, shape);
                    if dictlen > 16 && len > 16 {
                        let n = (len / 3).min(dictlen);
                        src[..n].copy_from_slice(&dict[dictlen - n..]);
                    }
                    let cb = unsafe { (co.frame_bound)(len, &p) };
                    let mut out = Vec::new();
                    for o in [&co, &ro] {
                        unsafe {
                            let cdict = (o.create_cdict)(dict.as_ptr(), dictlen);
                            let mut cctx: *mut u8 = std::ptr::null_mut();
                            let cr = (o.create_c)(&mut cctx, LZ4F_VERSION);
                            assert!(!is_err(o, cr));
                            let mut d = vec![0x3Cu8; cb + 32];
                            let n = (o.frame_cdict)(
                                cctx,
                                d.as_mut_ptr(),
                                cb,
                                src.as_ptr(),
                                len,
                                cdict,
                                &p,
                            );
                            (o.free_c)(cctx);
                            (o.free_cdict)(cdict);
                            out.push((n, d));
                        }
                    }
                    let ctx = format!(
                        "compressFrame_usingCDict dict={dictlen} lvl={lvl} bsid={bsid} shape={shape:?} len={len}"
                    );
                    eq(&ctx, out[0].0, out[1].0);
                    if !is_err(&co, out[0].0) {
                        eq_bytes(&ctx, &out[0].1, &out[1].1);
                    }
                }
            }
        }
    }
}

// ======================================= rows 95-116 low-level streaming compress

/// How the frame session is started.
#[derive(Clone, Copy, Debug)]
enum Begin {
    Plain,
    UsingDict(usize),
    UsingDictOnce(usize),
    UsingCDict(usize),
    /// Exercise the exported `LZ4F_compressBegin_internal` directly.
    Internal { dict: usize, use_cdict: bool },
}

/// One step of the streaming script.
#[derive(Clone, Copy, Debug)]
enum Step {
    Update(usize),
    Uncompressed(usize),
    Flush,
}

/// Run a full low-level frame compression session on ONE implementation.
#[allow(clippy::too_many_arguments)]
fn run_frame(
    o: &FOps,
    begin: Begin,
    p: &Prefs,
    dict: &[u8],
    src: &[u8],
    script: &[Step],
    stable_src: u32,
) -> (Vec<usize>, Vec<u8>) {
    unsafe {
        let mut cctx: *mut u8 = std::ptr::null_mut();
        let mut codes = Vec::new();
        let cr = (o.create_c)(&mut cctx, LZ4F_VERSION);
        codes.push(cr);
        if is_err(o, cr) {
            return (codes, Vec::new());
        }
        let mut out: Vec<u8> = Vec::new();
        let mut hdr = vec![0u8; 64];
        let cdict = match begin {
            Begin::UsingCDict(n) | Begin::Internal { dict: n, use_cdict: true } => {
                (o.create_cdict)(dict.as_ptr(), n)
            }
            _ => std::ptr::null_mut(),
        };
        let n = match begin {
            Begin::Plain => (o.begin)(cctx, hdr.as_mut_ptr(), hdr.len(), p),
            Begin::UsingDict(d) => {
                (o.begin_dict)(cctx, hdr.as_mut_ptr(), hdr.len(), dict.as_ptr(), d, p)
            }
            Begin::UsingDictOnce(d) => {
                (o.begin_dict_once)(cctx, hdr.as_mut_ptr(), hdr.len(), dict.as_ptr(), d, p)
            }
            Begin::UsingCDict(_) => (o.begin_cdict)(cctx, hdr.as_mut_ptr(), hdr.len(), cdict, p),
            Begin::Internal { dict: d, use_cdict } => (o.begin_internal)(
                cctx,
                hdr.as_mut_ptr(),
                hdr.len(),
                if use_cdict {
                    std::ptr::null()
                } else {
                    dict.as_ptr()
                },
                if use_cdict { 0 } else { d },
                cdict,
                p,
            ),
        };
        codes.push(n);
        if is_err(o, n) {
            (o.free_c)(cctx);
            if !cdict.is_null() {
                (o.free_cdict)(cdict);
            }
            return (codes, out);
        }
        out.extend_from_slice(&hdr[..n]);

        let copt = CompressOptions {
            stable_src,
            reserved: [0; 3],
        };
        let mut off = 0usize;
        for st in script {
            match *st {
                Step::Update(len) | Step::Uncompressed(len) => {
                    let len = len.min(src.len() - off);
                    let cap = (o.bound)(len, p);
                    if is_err(o, cap) {
                        codes.push(cap);
                        break;
                    }
                    let mut d = vec![0x6Bu8; cap + 32];
                    let n = match *st {
                        Step::Update(_) => (o.update)(
                            cctx,
                            d.as_mut_ptr(),
                            cap,
                            src.as_ptr().add(off),
                            len,
                            &copt,
                        ),
                        _ => (o.uncompressed_update)(
                            cctx,
                            d.as_mut_ptr(),
                            cap,
                            src.as_ptr().add(off),
                            len,
                            &copt,
                        ),
                    };
                    codes.push(n);
                    if is_err(o, n) {
                        break;
                    }
                    out.extend_from_slice(&d[..n]);
                    off += len;
                }
                Step::Flush => {
                    let cap = (o.bound)(0, p);
                    let mut d = vec![0x6Bu8; cap + 32];
                    let n = (o.flush)(cctx, d.as_mut_ptr(), cap, &copt);
                    codes.push(n);
                    if is_err(o, n) {
                        break;
                    }
                    out.extend_from_slice(&d[..n]);
                }
            }
            if off >= src.len() {
                // keep going: remaining steps become zero-length / flush
            }
        }

        let cap = (o.bound)(0, p) + 64;
        let mut d = vec![0x6Bu8; cap + 32];
        let n = (o.end)(cctx, d.as_mut_ptr(), cap, &copt);
        codes.push(n);
        if !is_err(o, n) {
            out.extend_from_slice(&d[..n]);
        }
        (o.free_c)(cctx);
        if !cdict.is_null() {
            (o.free_cdict)(cdict);
        }
        (codes, out)
    }
}

/// Build a random streaming script over `total` bytes.
/// `allow_uncompressed` must only be set for LZ4F_blockIndependent frames:
/// the C header states `LZ4F_uncompressedUpdate` is supported only in that
/// block mode, so mixing it into a blockLinked frame yields a frame the
/// decoder cannot reproduce (in the C reference too).
fn make_script(
    rng: &mut Rng,
    total: usize,
    block_size: usize,
    mode: u8,
    allow_uncompressed: bool,
) -> Vec<Step> {
    let mut s = Vec::new();
    let mut acc = 0usize;
    while acc < total {
        let len = match mode {
            0 => 1,
            1 => rng.range(1, 64),
            2 => block_size,
            3 => block_size.saturating_sub(1).max(1),
            4 => block_size + 1,
            5 => rng.range(1, block_size * 2 + 2),
            _ => rng.range(1, 40_000),
        };
        let len = len.min(total - acc);
        if allow_uncompressed && rng.below(8) == 0 {
            s.push(Step::Uncompressed(len));
        } else {
            s.push(Step::Update(len));
        }
        acc += len;
        if rng.below(10) == 0 {
            s.push(Step::Flush);
        }
    }
    if s.is_empty() {
        s.push(Step::Update(0));
    }
    s
}

#[test]
fn row95_110_low_level_streaming() {
    let (co, ro) = fops();
    let (gbs, _) = sym::<FnGetBlockSize>("LZ4F_getBlockSize");
    let mut rng = Rng::new(0xF00D_0095);

    // Every combination of the frame-header axes the C branches on
    // (bsid x blockMode x contentChecksum x blockChecksum x autoFlush = 80),
    // each driven with several randomized level / stableSrc / feed-shape draws.
    // The level, stableSrc and chunk-shape axes additionally get their own
    // exhaustive sweeps below (rows 96-103, 106) so nothing is only sampled.
    for &bsid in &[0i32, 4, 5, 6, 7] {
        let block_size = unsafe { gbs(if bsid == 0 { 4 } else { bsid }) };
        for &bmode in &[0i32, 1] {
            for &ccs in &[0i32, 1] {
                for &bcs in &[0i32, 1] {
                    for &af in &[0u32, 1] {
                        for _ in 0..5 {
                            let lvl = *[0i32, 1, 3, 9, 12, -5].get(rng.below(6)).unwrap();
                            let stable = rng.below(2) as u32;
                            let mode = rng.below(7) as u8;
                            let p = Prefs {
                                frame_info: FrameInfo {
                                    block_size_id: bsid,
                                    block_mode: bmode,
                                    content_checksum_flag: ccs,
                                    frame_type: 0,
                                    content_size: 0,
                                    dict_id: 0,
                                    block_checksum_flag: bcs,
                                },
                                compression_level: lvl,
                                auto_flush: af,
                                favor_dec_speed: 0,
                                reserved: [0; 3],
                            };
                            let shape = SHAPES[rng.below(SHAPES.len())];
                            let total = rng.range(0, 130_000);
                            let src = make_data(&mut rng, total, shape);
                            let script =
                                make_script(&mut rng, total, block_size, mode, bmode == 1);
                            let (cc, cb) =
                                run_frame(&co, Begin::Plain, &p, &[], &src, &script, stable);
                            let (rc, rb) =
                                run_frame(&ro, Begin::Plain, &p, &[], &src, &script, stable);
                            let ctx = format!(
                                "frame stream bsid={bsid} bmode={bmode} ccs={ccs} bcs={bcs} af={af} lvl={lvl} stable={stable} mode={mode} shape={shape:?} total={total}"
                            );
                            eq(&format!("{ctx} codes"), &cc, &rc);
                            eq_bytes(&ctx, &cb, &rb);
                            if !cc.iter().any(|&c| is_err(&co, c)) {
                                let got = decode_all(&co, &cb, total + 64);
                                eq_bytes(&format!("{ctx} roundtrip"), &got, &src);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Rows 96-103, 106: exhaustive sweeps of the chunk-shape, stableSrc and
/// compression-level axes at a moderate payload size, over all data shapes.
#[test]
fn row96_106_chunking_stable_and_levels() {
    let (co, ro) = fops();
    let (gbs, _) = sym::<FnGetBlockSize>("LZ4F_getBlockSize");
    let mut rng = Rng::new(0xF00D_0096);

    for &bsid in &[4i32, 5] {
        let bs = unsafe { gbs(bsid) };
        for &bmode in &[0i32, 1] {
            for &af in &[0u32, 1] {
                // rows 96-102: every chunk-shape mode, incl. 1-byte, ==blockSize,
                // blockSize-1 and blockSize+1
                for mode in 0u8..7 {
                    // row 103: stableSrc on and off
                    for &stable in &[0u32, 1] {
                        // row 106: the full level sweep
                        for &lvl in &[-5i32, -1, 0, 1, 2, 3, 9, 10, 12, 13] {
                            let shape = SHAPES[rng.below(SHAPES.len())];
                            // keep 1-byte chunking payloads small enough to run
                            let total = if mode == 0 {
                                rng.range(0, 20_000)
                            } else {
                                rng.range(0, 90_000)
                            };
                            let src = make_data(&mut rng, total, shape);
                            let p = Prefs {
                                frame_info: FrameInfo {
                                    block_size_id: bsid,
                                    block_mode: bmode,
                                    content_checksum_flag: 1,
                                    block_checksum_flag: 1,
                                    ..Default::default()
                                },
                                compression_level: lvl,
                                auto_flush: af,
                                ..Default::default()
                            };
                            let script =
                                make_script(&mut rng, total, bs, mode, bmode == 1);
                            let (cc, cb) =
                                run_frame(&co, Begin::Plain, &p, &[], &src, &script, stable);
                            let (rc, rb) =
                                run_frame(&ro, Begin::Plain, &p, &[], &src, &script, stable);
                            let ctx = format!(
                                "chunking bsid={bsid} bmode={bmode} af={af} mode={mode} stable={stable} lvl={lvl} shape={shape:?} total={total}"
                            );
                            eq(&format!("{ctx} codes"), &cc, &rc);
                            eq_bytes(&ctx, &cb, &rb);
                            if !cc.iter().any(|&c| is_err(&co, c)) {
                                let got = decode_all(&co, &cb, total + 64);
                                eq_bytes(&format!("{ctx} roundtrip"), &got, &src);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row104_content_size_and_flush() {
    let (co, ro) = fops();
    let (gbs, _) = sym::<FnGetBlockSize>("LZ4F_getBlockSize");
    let mut rng = Rng::new(0xF00D_0104);
    for &bsid in &[4i32, 5, 6, 7] {
        let block_size = unsafe { gbs(bsid) };
        for &ccs in &[0i32, 1] {
            for &af in &[0u32, 1] {
                for mode in 0u8..7 {
                    let total = rng.range(0, 200_000);
                    let shape = SHAPES[rng.below(SHAPES.len())];
                    let src = make_data(&mut rng, total, shape);
                    let p = Prefs {
                        frame_info: FrameInfo {
                            block_size_id: bsid,
                            content_checksum_flag: ccs,
                            content_size: total as u64,
                            ..Default::default()
                        },
                        auto_flush: af,
                        ..Default::default()
                    };
                    let script = make_script(&mut rng, total, block_size, mode, false);
                    let (cc, cb) = run_frame(&co, Begin::Plain, &p, &[], &src, &script, 0);
                    let (rc, rb) = run_frame(&ro, Begin::Plain, &p, &[], &src, &script, 0);
                    let ctx = format!(
                        "frame contentSize bsid={bsid} ccs={ccs} af={af} mode={mode} total={total}"
                    );
                    eq(&format!("{ctx} codes"), &cc, &rc);
                    eq_bytes(&ctx, &cb, &rb);
                }
            }
        }
    }
}

#[test]
fn row107_108_explicit_flush() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0107);
    // Scripts made mostly of flushes, incl. flush with an empty tmp buffer.
    for &bsid in &[4i32, 6] {
        for &af in &[0u32, 1] {
            for _ in 0..25 {
                let total = rng.range(0, 100_000);
                let shape = SHAPES[rng.below(SHAPES.len())];
                let src = make_data(&mut rng, total, shape);
                let p = Prefs {
                    frame_info: FrameInfo {
                        block_size_id: bsid,
                        content_checksum_flag: 1,
                        block_checksum_flag: 1,
                        ..Default::default()
                    },
                    auto_flush: af,
                    ..Default::default()
                };
                let mut script = vec![Step::Flush, Step::Flush];
                let mut acc = 0usize;
                while acc < total {
                    let n = rng.range(1, 5000).min(total - acc);
                    script.push(Step::Update(n));
                    script.push(Step::Flush);
                    script.push(Step::Flush);
                    acc += n;
                }
                script.push(Step::Flush);
                let (cc, cb) = run_frame(&co, Begin::Plain, &p, &[], &src, &script, 0);
                let (rc, rb) = run_frame(&ro, Begin::Plain, &p, &[], &src, &script, 0);
                let ctx = format!("frame flush bsid={bsid} af={af} total={total}");
                eq(&format!("{ctx} codes"), &cc, &rc);
                eq_bytes(&ctx, &cb, &rb);
                if !cc.iter().any(|&c| is_err(&co, c)) {
                    let got = decode_all(&co, &cb, total + 64);
                    eq_bytes(&format!("{ctx} roundtrip"), &got, &src);
                }
            }
        }
    }
}

#[test]
fn row109_110_uncompressed_update() {
    let (co, ro) = fops();
    let (gbs, _) = sym::<FnGetBlockSize>("LZ4F_getBlockSize");
    let mut rng = Rng::new(0xF00D_0109);
    for &bsid in &[4i32, 5, 6] {
        let bs = unsafe { gbs(bsid) };
        for &bmode in &[0i32, 1] {
            for &bcs in &[0i32, 1] {
                for &af in &[0u32, 1] {
                    for _ in 0..8 {
                        let total = rng.range(1, 150_000);
                        let shape = SHAPES[rng.below(SHAPES.len())];
                        let src = make_data(&mut rng, total, shape);
                        let p = Prefs {
                            frame_info: FrameInfo {
                                block_size_id: bsid,
                                block_mode: bmode,
                                content_checksum_flag: 1,
                                block_checksum_flag: bcs,
                                ..Default::default()
                            },
                            auto_flush: af,
                            ..Default::default()
                        };
                        // Interleave compressed and stored blocks, incl. sizes
                        // above and below the block size.
                        let mut script = Vec::new();
                        let mut acc = 0usize;
                        let mut i = 0;
                        while acc < total {
                            let n = match i % 5 {
                                0 => bs,
                                1 => bs + 1,
                                2 => bs.saturating_sub(1).max(1),
                                3 => rng.range(1, 100),
                                _ => rng.range(1, bs * 2 + 2),
                            }
                            .min(total - acc);
                            if i % 2 == 0 {
                                script.push(Step::Uncompressed(n));
                            } else {
                                script.push(Step::Update(n));
                            }
                            acc += n;
                            i += 1;
                        }
                        let (cc, cb) = run_frame(&co, Begin::Plain, &p, &[], &src, &script, 0);
                        let (rc, rb) = run_frame(&ro, Begin::Plain, &p, &[], &src, &script, 0);
                        let ctx = format!(
                            "uncompressedUpdate bsid={bsid} bmode={bmode} bcs={bcs} af={af} total={total}"
                        );
                        eq(&format!("{ctx} codes"), &cc, &rc);
                        eq_bytes(&ctx, &cb, &rb);
                        // `uncompressedUpdate` is only a supported operation for
                        // blockIndependent frames (see lz4frame.h), so only those
                        // frames are required to decode back to the source. The
                        // C-vs-Rust byte comparison above still applies to both.
                        if bmode == 1 && !cc.iter().any(|&c| is_err(&co, c)) {
                            let got = decode_all(&co, &cb, total + 64);
                            eq_bytes(&format!("{ctx} roundtrip"), &got, &src);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row111_116_dict_begin_variants() {
    let (co, ro) = fops();
    let (gbs, _) = sym::<FnGetBlockSize>("LZ4F_getBlockSize");
    let mut rng = Rng::new(0xF00D_0111);
    for &dictlen in &[0usize, 1, 100, 4096, 65535, 65536, 120_000] {
        for &shape in &SHAPES {
            let dict = make_data(&mut rng, dictlen, shape);
            let begins = [
                Begin::UsingDict(dictlen),
                Begin::UsingDictOnce(dictlen),
                Begin::UsingCDict(dictlen),
                Begin::Internal {
                    dict: dictlen,
                    use_cdict: false,
                },
                Begin::Internal {
                    dict: dictlen,
                    use_cdict: true,
                },
            ];
            for begin in begins {
                // Sample the (bsid x blockMode x level) cross-product; those
                // axes are swept exhaustively in row96_106 / row79_91.
                for _ in 0..4 {
                    let bsid = *[4i32, 6].get(rng.below(2)).unwrap();
                    let bmode = rng.below(2) as i32;
                    let lvl = *[0i32, 3, 9, 12].get(rng.below(4)).unwrap();
                    let bs = unsafe { gbs(bsid) };
                    let total = rng.range(1, 120_000);
                    let mut src = make_data(&mut rng, total, shape);
                    if dictlen > 16 && total > 16 {
                        let n = (total / 3).min(dictlen);
                        src[..n].copy_from_slice(&dict[dictlen - n..]);
                    }
                    let p = Prefs {
                        frame_info: FrameInfo {
                            block_size_id: bsid,
                            block_mode: bmode,
                            content_checksum_flag: 1,
                            ..Default::default()
                        },
                        compression_level: lvl,
                        ..Default::default()
                    };
                    let mode = rng.below(7) as u8;
                    let script = make_script(&mut rng, total, bs, mode, bmode == 1);
                    let (cc, cb) = run_frame(&co, begin, &p, &dict, &src, &script, 0);
                    let (rc, rb) = run_frame(&ro, begin, &p, &dict, &src, &script, 0);
                    let ctx = format!(
                        "frame begin={begin:?} dict={dictlen} bsid={bsid} bmode={bmode} lvl={lvl} shape={shape:?} total={total}"
                    );
                    eq(&format!("{ctx} codes"), &cc, &rc);
                    eq_bytes(&ctx, &cb, &rb);
                }
            }
        }
    }
}

#[test]
fn row116_cctx_reuse_across_frames() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0116);
    for &shape in &SHAPES {
        for _ in 0..12 {
            let n_frames = rng.range(2, 5);
            let payloads: Vec<Vec<u8>> = (0..n_frames)
                .map(|_| {
                    let l = rng.range(0, 80_000);
                    make_data(&mut rng, l, shape)
                })
                .collect();
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: 5,
                    content_checksum_flag: 1,
                    block_checksum_flag: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut got = Vec::new();
            for o in [&co, &ro] {
                unsafe {
                    let mut cctx: *mut u8 = std::ptr::null_mut();
                    let mut codes = vec![(o.create_c)(&mut cctx, LZ4F_VERSION)];
                    let mut all = Vec::new();
                    for pl in &payloads {
                        let mut hdr = vec![0u8; 64];
                        let n = (o.begin)(cctx, hdr.as_mut_ptr(), hdr.len(), &p);
                        codes.push(n);
                        all.extend_from_slice(&hdr[..n]);
                        let cap = (o.bound)(pl.len(), &p);
                        let mut d = vec![0u8; cap + 32];
                        let opt = CompressOptions::default();
                        let n = (o.update)(
                            cctx,
                            d.as_mut_ptr(),
                            cap,
                            pl.as_ptr(),
                            pl.len(),
                            &opt,
                        );
                        codes.push(n);
                        all.extend_from_slice(&d[..n]);
                        let cap2 = (o.bound)(0, &p) + 64;
                        let mut e = vec![0u8; cap2 + 32];
                        let n = (o.end)(cctx, e.as_mut_ptr(), cap2, &opt);
                        codes.push(n);
                        all.extend_from_slice(&e[..n]);
                    }
                    (o.free_c)(cctx);
                    got.push((codes, all));
                }
            }
            let ctx = format!("cctx reuse shape={shape:?} frames={n_frames}");
            eq(&format!("{ctx} codes"), &got[0].0, &got[1].0);
            eq_bytes(&ctx, &got[0].1, &got[1].1);
        }
    }
}

#[test]
fn row115_136_advanced_custom_mem() {
    // Custom-mem creators take an LZ4F_CustomMem struct by value. Passing an
    // all-NULL struct selects the default allocator (see LZ4F_createCompression
    // Context_advanced in lz4frame.c), which is the reachable behaviour for an
    // external caller that zero-initialises the struct.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct CustomMem {
        alloc: Option<unsafe extern "C" fn(*mut u8, usize) -> *mut u8>,
        calloc: Option<unsafe extern "C" fn(*mut u8, usize) -> *mut u8>,
        free: Option<unsafe extern "C" fn(*mut u8, *mut u8)>,
        opaque: *mut u8,
    }
    type FnCctxAdv = unsafe extern "C" fn(CustomMem, u32) -> *mut u8;
    type FnDctxAdv = unsafe extern "C" fn(CustomMem, u32) -> *mut u8;
    type FnCDictAdv = unsafe extern "C" fn(CustomMem, *const u8, usize) -> *mut u8;

    let (co, ro) = fops();
    let (cca, rca) = sym::<FnCctxAdv>("LZ4F_createCompressionContext_advanced");
    let (cda, rda) = sym::<FnDctxAdv>("LZ4F_createDecompressionContext_advanced");
    let (ccda, rcda) = sym::<FnCDictAdv>("LZ4F_createCDict_advanced");

    let mut rng = Rng::new(0xF00D_0115);
    for &shape in &SHAPES {
        for _ in 0..8 {
            let total = rng.range(1, 90_000);
            let src = make_data(&mut rng, total, shape);
            let dictlen = rng.range(0, 70_000);
            let dict = make_data(&mut rng, dictlen, shape);
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: 5,
                    content_checksum_flag: 1,
                    ..Default::default()
                },
                compression_level: 3,
                ..Default::default()
            };
            let mut got = Vec::new();
            for (o, mk_c, mk_d, mk_cd) in
                [(&co, &cca, &cda, &ccda), (&ro, &rca, &rda, &rcda)]
            {
                unsafe {
                    let mem = CustomMem {
                        opaque: std::ptr::null_mut(),
                        ..Default::default()
                    };
                    let cctx = mk_c(mem, LZ4F_VERSION);
                    assert!(!cctx.is_null(), "advanced cctx NULL");
                    let cdict = mk_cd(mem, dict.as_ptr(), dictlen);
                    let cap = (o.frame_bound)(total, &p);
                    let mut d = vec![0u8; cap + 32];
                    let n = (o.frame_cdict)(
                        cctx,
                        d.as_mut_ptr(),
                        cap,
                        src.as_ptr(),
                        total,
                        cdict,
                        &p,
                    );
                    d.truncate(if is_err(o, n) { 0 } else { n });
                    (o.free_c)(cctx);
                    if !cdict.is_null() {
                        (o.free_cdict)(cdict);
                    }

                    // decompress with an advanced dctx
                    let dctx = mk_d(mem, LZ4F_VERSION);
                    assert!(!dctx.is_null(), "advanced dctx NULL");
                    let mut out = vec![0u8; total + 64];
                    let mut dsz = out.len();
                    let mut ssz = d.len();
                    let dr = (o.decompress_dict)(
                        dctx,
                        out.as_mut_ptr(),
                        &mut dsz,
                        d.as_ptr(),
                        &mut ssz,
                        dict.as_ptr(),
                        dictlen,
                        std::ptr::null(),
                    );
                    (o.free_d)(dctx);
                    out.truncate(dsz);
                    got.push((n, d, dr, dsz, ssz, out));
                }
            }
            let ctx = format!("advanced customMem shape={shape:?} total={total} dict={dictlen}");
            eq(&format!("{ctx} cRet"), got[0].0, got[1].0);
            eq_bytes(&format!("{ctx} frame"), &got[0].1, &got[1].1);
            eq(&format!("{ctx} dRet"), got[0].2, got[1].2);
            eq(&format!("{ctx} dstSize"), got[0].3, got[1].3);
            eq(&format!("{ctx} srcSize"), got[0].4, got[1].4);
            eq_bytes(&format!("{ctx} decoded"), &got[0].5, &got[1].5);
            eq_bytes(&format!("{ctx} roundtrip"), &got[0].5, &src);
        }
    }
}

// ============================================== rows 117-137 frame decompression

/// Decode an entire frame in one call using implementation `o`.
fn decode_all(o: &FOps, frame: &[u8], cap: usize) -> Vec<u8> {
    unsafe {
        let mut dctx: *mut u8 = std::ptr::null_mut();
        let r = (o.create_d)(&mut dctx, LZ4F_VERSION);
        assert!(!is_err(o, r));
        let mut out = vec![0u8; cap.max(1)];
        let mut total = 0usize;
        let mut soff = 0usize;
        loop {
            let mut dsz = out.len() - total;
            let mut ssz = frame.len() - soff;
            if dsz == 0 {
                out.resize(out.len() * 2 + 64, 0);
                continue;
            }
            let hint = (o.decompress)(
                dctx,
                out.as_mut_ptr().add(total),
                &mut dsz,
                frame.as_ptr().add(soff),
                &mut ssz,
                std::ptr::null(),
            );
            total += dsz;
            soff += ssz;
            if is_err(o, hint) || hint == 0 || (ssz == 0 && dsz == 0) {
                break;
            }
        }
        (o.free_d)(dctx);
        out.truncate(total);
        out
    }
}

/// Feeding pattern for the decompressor.
#[derive(Clone, Copy, Debug)]
enum Feed {
    OneShot,
    SrcByByte,
    SrcRandom,
    DstByByte,
    DstRandom,
    Both,
}

/// Decode `frame` on ONE implementation with the given feeding pattern,
/// returning every observable output.
fn decode_pattern(
    o: &FOps,
    frame: &[u8],
    plain_len: usize,
    feed: Feed,
    dopt: &DecompressOptions,
    dict: Option<&[u8]>,
    rng: &mut Rng,
) -> (Vec<usize>, Vec<u8>, usize, usize) {
    unsafe {
        let mut dctx: *mut u8 = std::ptr::null_mut();
        let mut hints = vec![(o.create_d)(&mut dctx, LZ4F_VERSION)];
        let mut out = vec![0u8; plain_len + 4096];
        let mut total = 0usize;
        let mut soff = 0usize;
        let mut guard = 0usize;
        loop {
            guard += 1;
            if guard > 4_000_000 {
                break;
            }
            let src_chunk = match feed {
                Feed::OneShot | Feed::DstByByte | Feed::DstRandom => frame.len() - soff,
                Feed::SrcByByte => 1.min(frame.len() - soff),
                Feed::SrcRandom | Feed::Both => rng.range(1, 512).min(frame.len() - soff),
            };
            let dst_room = out.len() - total;
            let dst_chunk = match feed {
                Feed::OneShot | Feed::SrcByByte | Feed::SrcRandom => dst_room,
                Feed::DstByByte => 1.min(dst_room),
                Feed::DstRandom | Feed::Both => rng.range(1, 512).min(dst_room),
            };
            if dst_room == 0 {
                out.resize(out.len() * 2 + 64, 0);
                continue;
            }
            let mut dsz = dst_chunk;
            let mut ssz = src_chunk;
            let hint = (o.decompress)(
                dctx,
                out.as_mut_ptr().add(total),
                &mut dsz,
                frame.as_ptr().add(soff),
                &mut ssz,
                dopt,
            );
            hints.push(hint);
            total += dsz;
            soff += ssz;
            if is_err(o, hint) || hint == 0 {
                break;
            }
            if ssz == 0 && dsz == 0 {
                if soff >= frame.len() {
                    break;
                }
                // no progress with input remaining: avoid an infinite loop
                break;
            }
        }
        let _ = dict;
        (o.free_d)(dctx);
        out.truncate(total);
        (hints, out, total, soff)
    }
}

#[test]
fn row117_126_decompress_all_patterns() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0117);
    let mat = prefs_matrix();
    let feeds = [
        Feed::OneShot,
        Feed::SrcByByte,
        Feed::SrcRandom,
        Feed::DstByByte,
        Feed::DstRandom,
        Feed::Both,
    ];

    for (pi, p) in mat.iter().enumerate() {
        // keep byte-at-a-time patterns to small payloads for runtime
        for &shape in &SHAPES {
            let len = rng.range(0, 40_000);
            let src = make_data(&mut rng, len, shape);
            let cap = unsafe { (co.frame_bound)(len, p) };
            let mut fr = vec![0u8; cap + 32];
            let n = unsafe { (co.frame)(fr.as_mut_ptr(), cap, src.as_ptr(), len, p) };
            if is_err(&co, n) {
                continue;
            }
            fr.truncate(n);

            for feed in feeds {
                // skip the O(n) byte-at-a-time patterns on big payloads
                if matches!(feed, Feed::SrcByByte | Feed::DstByByte) && len > 4096 {
                    continue;
                }
                for &(sd, sc) in &[(0u32, 0u32), (1, 0), (0, 1), (1, 1)] {
                    let dopt = DecompressOptions {
                        stable_dst: sd,
                        skip_checksums: sc,
                        reserved1: 0,
                        reserved0: 0,
                    };
                    let mut r1 = Rng::new(0x5EED_0001);
                    let mut r2 = Rng::new(0x5EED_0001);
                    let a = decode_pattern(&co, &fr, len, feed, &dopt, None, &mut r1);
                    let b = decode_pattern(&ro, &fr, len, feed, &dopt, None, &mut r2);
                    let ctx = format!(
                        "decompress p={pi} shape={shape:?} len={len} feed={feed:?} stableDst={sd} skipCk={sc}"
                    );
                    eq(&format!("{ctx} hints"), &a.0, &b.0);
                    eq(&format!("{ctx} dstTotal"), a.2, b.2);
                    eq(&format!("{ctx} srcConsumed"), a.3, b.3);
                    eq_bytes(&ctx, &a.1, &b.1);
                    eq_bytes(&format!("{ctx} roundtrip"), &a.1, &src);
                }
            }
        }
    }
}

#[test]
fn row127_128_skippable_and_concatenated() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0127);
    for &shape in &SHAPES {
        for _ in 0..10 {
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: 4,
                    content_checksum_flag: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            let l1 = rng.range(0, 30_000);
            let l2 = rng.range(0, 30_000);
            let s1 = make_data(&mut rng, l1, shape);
            let s2 = make_data(&mut rng, l2, shape);
            let mut stream = Vec::new();

            // optional leading skippable frame
            let skip_len = rng.range(0, 200);
            let with_skip = rng.bool();
            if with_skip {
                let magic: u32 = 0x184D2A50 | (rng.below(16) as u32);
                stream.extend_from_slice(&magic.to_le_bytes());
                stream.extend_from_slice(&(skip_len as u32).to_le_bytes());
                for _ in 0..skip_len {
                    stream.push(rng.byte());
                }
            }
            for s in [&s1, &s2] {
                let cap = unsafe { (co.frame_bound)(s.len(), &p) };
                let mut fr = vec![0u8; cap + 32];
                let n =
                    unsafe { (co.frame)(fr.as_mut_ptr(), cap, s.as_ptr(), s.len(), &p) };
                assert!(!is_err(&co, n));
                stream.extend_from_slice(&fr[..n]);
            }

            let total = l1 + l2;
            for feed in [Feed::OneShot, Feed::SrcRandom, Feed::DstRandom] {
                let dopt = DecompressOptions::default();
                let mut r1 = Rng::new(0x5EED_0002);
                let mut r2 = Rng::new(0x5EED_0002);
                let a = decode_pattern(&co, &stream, total, feed, &dopt, None, &mut r1);
                let b = decode_pattern(&ro, &stream, total, feed, &dopt, None, &mut r2);
                let ctx = format!(
                    "concat skip={with_skip} shape={shape:?} l1={l1} l2={l2} feed={feed:?}"
                );
                eq(&format!("{ctx} hints"), &a.0, &b.0);
                eq(&format!("{ctx} dstTotal"), a.2, b.2);
                eq(&format!("{ctx} srcConsumed"), a.3, b.3);
                eq_bytes(&ctx, &a.1, &b.1);
            }
        }
    }
}

#[test]
fn row129_130_decompress_using_dict() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0129);
    for &dictlen in &[0usize, 1, 100, 4096, 65535, 65536, 120_000] {
        for &shape in &SHAPES {
            let dict = make_data(&mut rng, dictlen, shape);
            for &bmode in &[0i32, 1] {
                for &lvl in &[0i32, 3, 12] {
                    let total = rng.range(1, 80_000);
                    let mut src = make_data(&mut rng, total, shape);
                    if dictlen > 16 && total > 16 {
                        let n = (total / 3).min(dictlen);
                        src[..n].copy_from_slice(&dict[dictlen - n..]);
                    }
                    let p = Prefs {
                        frame_info: FrameInfo {
                            block_size_id: 5,
                            block_mode: bmode,
                            content_checksum_flag: 1,
                            ..Default::default()
                        },
                        compression_level: lvl,
                        ..Default::default()
                    };
                    // Build the dict-compressed frame with the C implementation.
                    let (_codes, frame) = run_frame(
                        &co,
                        Begin::UsingDict(dictlen),
                        &p,
                        &dict,
                        &src,
                        &make_script(&mut rng, total, 262_144, 5, false),
                        0,
                    );

                    for chunked in [false, true] {
                        let mut got = Vec::new();
                        for o in [&co, &ro] {
                            unsafe {
                                let mut dctx: *mut u8 = std::ptr::null_mut();
                                let mut codes =
                                    vec![(o.create_d)(&mut dctx, LZ4F_VERSION)];
                                let mut out = vec![0u8; total + 4096];
                                let mut t = 0usize;
                                let mut soff = 0usize;
                                let mut r = Rng::new(0x5EED_0003);
                                loop {
                                    let sc = if chunked {
                                        r.range(1, 700).min(frame.len() - soff)
                                    } else {
                                        frame.len() - soff
                                    };
                                    let dc = if chunked {
                                        r.range(1, 700).min(out.len() - t)
                                    } else {
                                        out.len() - t
                                    };
                                    if out.len() - t == 0 {
                                        out.resize(out.len() * 2 + 64, 0);
                                        continue;
                                    }
                                    let mut dsz = dc;
                                    let mut ssz = sc;
                                    let h = (o.decompress_dict)(
                                        dctx,
                                        out.as_mut_ptr().add(t),
                                        &mut dsz,
                                        frame.as_ptr().add(soff),
                                        &mut ssz,
                                        dict.as_ptr(),
                                        dictlen,
                                        std::ptr::null(),
                                    );
                                    codes.push(h);
                                    t += dsz;
                                    soff += ssz;
                                    if is_err(o, h) || h == 0 {
                                        break;
                                    }
                                    if ssz == 0 && dsz == 0 {
                                        break;
                                    }
                                }
                                (o.free_d)(dctx);
                                out.truncate(t);
                                got.push((codes, out));
                            }
                        }
                        let ctx = format!(
                            "decompress_usingDict dict={dictlen} bmode={bmode} lvl={lvl} shape={shape:?} total={total} chunked={chunked}"
                        );
                        eq(&format!("{ctx} codes"), &got[0].0, &got[1].0);
                        eq_bytes(&ctx, &got[0].1, &got[1].1);
                        eq_bytes(&format!("{ctx} roundtrip"), &got[0].1, &src);
                    }
                }
            }
        }
    }
}

#[test]
fn row131_134_frame_info_and_header_size() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0131);
    let mat = prefs_matrix();
    for (pi, p) in mat.iter().enumerate() {
        let len = rng.range(0, 20_000);
        let shape = SHAPES[rng.below(SHAPES.len())];
        let src = make_data(&mut rng, len, shape);
        let cap = unsafe { (co.frame_bound)(len, p) };
        let mut fr = vec![0u8; cap + 32];
        let n = unsafe { (co.frame)(fr.as_mut_ptr(), cap, src.as_ptr(), len, p) };
        if is_err(&co, n) {
            continue;
        }
        fr.truncate(n);

        // row 134: headerSize over every prefix length
        for k in 0..=fr.len().min(24) {
            let (a, b) = unsafe {
                (
                    (co.header_size)(fr.as_ptr(), k),
                    (ro.header_size)(fr.as_ptr(), k),
                )
            };
            eq(&format!("headerSize p={pi} prefix={k}"), a, b);
        }

        // row 131: getFrameInfo with the whole frame available
        let mut got = Vec::new();
        for o in [&co, &ro] {
            unsafe {
                let mut dctx: *mut u8 = std::ptr::null_mut();
                (o.create_d)(&mut dctx, LZ4F_VERSION);
                let mut fi = FrameInfo::default();
                let mut ssz = fr.len();
                let r = (o.frame_info)(dctx, &mut fi, fr.as_ptr(), &mut ssz);
                (o.free_d)(dctx);
                got.push((r, fi, ssz));
            }
        }
        let ctx = format!("getFrameInfo p={pi} len={len}");
        eq(&format!("{ctx} ret"), got[0].0, got[1].0);
        eq(&format!("{ctx} info"), got[0].1, got[1].1);
        eq(&format!("{ctx} consumed"), got[0].2, got[1].2);

        // row 132: getFrameInfo driven byte-by-byte until it succeeds
        let mut got = Vec::new();
        for o in [&co, &ro] {
            unsafe {
                let mut dctx: *mut u8 = std::ptr::null_mut();
                (o.create_d)(&mut dctx, LZ4F_VERSION);
                let mut rets = Vec::new();
                let mut fi = FrameInfo::default();
                let mut soff = 0usize;
                for _ in 0..fr.len().min(30) {
                    let mut ssz = 1usize;
                    let r = (o.frame_info)(
                        dctx,
                        &mut fi,
                        fr.as_ptr().add(soff),
                        &mut ssz,
                    );
                    rets.push((r, ssz));
                    soff += ssz;
                    if !is_err(o, r) && r == 0 {
                        break;
                    }
                    if is_err(o, r) {
                        break;
                    }
                }
                (o.free_d)(dctx);
                got.push((rets, fi));
            }
        }
        let ctx = format!("getFrameInfo byte-by-byte p={pi} len={len}");
        eq(&format!("{ctx} rets"), &got[0].0, &got[1].0);
        eq(&format!("{ctx} info"), got[0].1, got[1].1);

        // row 133: getFrameInfo after decoding has started (must be an error)
        let mut got = Vec::new();
        for o in [&co, &ro] {
            unsafe {
                let mut dctx: *mut u8 = std::ptr::null_mut();
                (o.create_d)(&mut dctx, LZ4F_VERSION);
                let mut out = vec![0u8; len + 64];
                let mut dsz = out.len();
                let mut ssz = fr.len();
                let h = (o.decompress)(
                    dctx,
                    out.as_mut_ptr(),
                    &mut dsz,
                    fr.as_ptr(),
                    &mut ssz,
                    std::ptr::null(),
                );
                let mut fi = FrameInfo::default();
                let mut ssz2 = 0usize;
                let r = (o.frame_info)(dctx, &mut fi, fr.as_ptr(), &mut ssz2);
                (o.free_d)(dctx);
                got.push((h, r, fi, ssz2));
            }
        }
        let ctx = format!("getFrameInfo mid-frame p={pi} len={len}");
        eq(&format!("{ctx} decHint"), got[0].0, got[1].0);
        eq(&format!("{ctx} ret"), got[0].1, got[1].1);
        eq(&format!("{ctx} info"), got[0].2, got[1].2);
    }
}

#[test]
fn row135_reset_decompression_context() {
    let (co, ro) = fops();
    let mut rng = Rng::new(0xF00D_0135);
    let p = Prefs {
        frame_info: FrameInfo {
            block_size_id: 4,
            content_checksum_flag: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    for &shape in &SHAPES {
        for _ in 0..12 {
            let len = rng.range(1, 40_000);
            let src = make_data(&mut rng, len, shape);
            let cap = unsafe { (co.frame_bound)(len, &p) };
            let mut fr = vec![0u8; cap + 32];
            let n = unsafe { (co.frame)(fr.as_mut_ptr(), cap, src.as_ptr(), len, &p) };
            fr.truncate(n);
            // Corrupt a copy to force an error, then reset and decode the good one.
            let mut bad = fr.clone();
            let bi = 7 + rng.below(bad.len().saturating_sub(8).max(1));
            bad[bi] ^= 0xFF;

            let mut got = Vec::new();
            for o in [&co, &ro] {
                unsafe {
                    let mut dctx: *mut u8 = std::ptr::null_mut();
                    let mut codes = vec![(o.create_d)(&mut dctx, LZ4F_VERSION)];
                    // pass 1: corrupt input
                    let mut out = vec![0u8; len + 4096];
                    let mut dsz = out.len();
                    let mut ssz = bad.len();
                    let h = (o.decompress)(
                        dctx,
                        out.as_mut_ptr(),
                        &mut dsz,
                        bad.as_ptr(),
                        &mut ssz,
                        std::ptr::null(),
                    );
                    codes.push(h);
                    // reset and decode the valid frame
                    (o.reset_d)(dctx);
                    let mut out2 = vec![0u8; len + 4096];
                    let mut d2 = out2.len();
                    let mut s2 = fr.len();
                    let h2 = (o.decompress)(
                        dctx,
                        out2.as_mut_ptr(),
                        &mut d2,
                        fr.as_ptr(),
                        &mut s2,
                        std::ptr::null(),
                    );
                    codes.push(h2);
                    (o.free_d)(dctx);
                    out2.truncate(d2);
                    got.push((codes, out2, d2, s2));
                }
            }
            let ctx = format!("resetDctx shape={shape:?} len={len} bi={bi}");
            eq(&format!("{ctx} codes"), &got[0].0, &got[1].0);
            eq(&format!("{ctx} dstSize"), got[0].2, got[1].2);
            eq(&format!("{ctx} srcSize"), got[0].3, got[1].3);
            eq_bytes(&ctx, &got[0].1, &got[1].1);
        }
    }
}

#[test]
fn row137_error_helpers() {
    let (cie, rie) = sym::<FnIsError>("LZ4F_isError");
    let (cen, ren) = sym::<FnErrName>("LZ4F_getErrorName");
    let (cec, rec) = sym::<FnErrCode>("LZ4F_getErrorCode");

    // every enum code, plus non-error values and far-out-of-range codes
    let mut codes: Vec<usize> = Vec::new();
    for i in 0..30usize {
        codes.push(0usize.wrapping_sub(i));
    }
    for v in [
        0usize,
        1,
        2,
        100,
        1 << 20,
        usize::MAX / 2,
        usize::MAX - 1000,
        usize::MAX - 100,
        usize::MAX - 25,
        usize::MAX - 24,
        usize::MAX - 23,
        usize::MAX,
    ] {
        codes.push(v);
    }
    let mut rng = Rng::new(0xF00D_0137);
    for _ in 0..500 {
        codes.push(rng.next_u64() as usize);
    }

    for &c in &codes {
        eq(&format!("isError({c})"), unsafe { cie(c) }, unsafe { rie(c) });
        eq(&format!("getErrorCode({c})"), unsafe { cec(c) }, unsafe {
            rec(c)
        });
        unsafe {
            let a = std::ffi::CStr::from_ptr(cen(c));
            let b = std::ffi::CStr::from_ptr(ren(c));
            eq(&format!("getErrorName({c})"), a, b);
        }
    }
}

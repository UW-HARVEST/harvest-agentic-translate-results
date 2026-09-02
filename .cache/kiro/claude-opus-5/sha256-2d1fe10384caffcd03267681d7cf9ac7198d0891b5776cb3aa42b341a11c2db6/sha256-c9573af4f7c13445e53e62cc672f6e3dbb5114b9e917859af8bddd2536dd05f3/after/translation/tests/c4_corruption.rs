//! Phase C: the constructive-corruption sweep.
//!
//! This is the coverage vehicle for the deep bitstream checks in
//! `ERRORS.md` — the 357 `corruption_detected` sites, the
//! `tableLog_tooLarge` / `maxSymbolValue_too*` / `literals_headerWrong` /
//! `checksum_wrong` / `frameParameter_*` sites in
//! `decompress/zstd_decompress_block.c`, `decompress/huf_decompress.c`,
//! `common/entropy_common.c` and `common/fse_decompress.c`. Those conditions
//! cannot be constructed by calling a public function with a bad argument; they
//! are only reachable from a malformed bitstream.
//!
//! Method: take frames produced in many `CONFIGS.md` configurations, then
//! - flip EVERY bit of EVERY byte (exhaustive, for small frames),
//! - stamp every byte to 0x00 and 0xFF,
//! - truncate at every length,
//! and assert C and Rust return the IDENTICAL error code (or the identical
//! success AND identical plaintext) for each mutant.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_ulonglong, c_void};

type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCtxDecompress =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnGetFCS = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;
type FnGetFH = unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t) -> size_t;

/// Build a frame with the given parameter list applied to a CCtx.
fn frame_with(params: &[(c_int, c_int)], src: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let e = Err2::new();
        let (cn, _) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cf, _) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        let (sp, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = both::<FnCompress2>("ZSTD_compress2");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let cc = cn();
        for (id, v) in params {
            if e.c.is_err(sp(cc, *id, *v)) {
                cf(cc);
                return None;
            }
        }
        let cap = bnd(src.len()) + 64;
        let mut o = vec![0u8; cap];
        let s = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
        let n = c2(cc, o.as_mut_ptr() as *mut c_void, cap, s, src.len());
        cf(cc);
        if e.c.is_err(n) {
            return None;
        }
        o.truncate(n);
        Some(o)
    }
}

struct D {
    e: Err2,
    cd: *mut c_void,
    rd: *mut c_void,
}
impl D {
    fn new() -> D {
        unsafe {
            let (a, b) = both::<FnVoidToPtr>("ZSTD_createDCtx");
            D { e: Err2::new(), cd: a(), rd: b() }
        }
    }
    fn reset(&self) {
        unsafe {
            let (a, b) = both::<FnReset>("ZSTD_DCtx_reset");
            a(self.cd, ZSTD_reset_session_and_parameters);
            b(self.rd, ZSTD_reset_session_and_parameters);
            let (c, d) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
            c(self.cd, ZSTD_d_windowLogMax, 31);
            d(self.rd, ZSTD_d_windowLogMax, 31);
        }
    }
    /// Decompress `buf` with both libraries and assert identical outcome.
    #[track_caller]
    fn cmp(&self, ctx: &str, buf: &[u8], outcap: usize) {
        unsafe {
            let (cdd, rdd) = both::<FnCtxDecompress>("ZSTD_decompressDCtx");
            self.reset();
            let mut o1 = vec![0u8; outcap.max(1)];
            let mut o2 = vec![0u8; outcap.max(1)];
            let p = if buf.is_empty() { std::ptr::null() } else { buf.as_ptr() as *const c_void };
            let a = cdd(self.cd, o1.as_mut_ptr() as *mut c_void, outcap, p, buf.len());
            let b = rdd(self.rd, o2.as_mut_ptr() as *mut c_void, outcap, p, buf.len());
            self.e.eq(ctx, a, b);
            if !self.e.c.is_err(a) {
                assert_bytes_eq(ctx, &o1[..a], &o2[..b]);
            }
        }
    }
}
impl Drop for D {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrToSize>("ZSTD_freeDCtx");
            a(self.cd);
            b(self.rd);
        }
    }
}

/// A representative set of frame-producing configurations, kept small enough
/// that an exhaustive bit sweep is affordable.
fn configs() -> Vec<(String, Vec<(c_int, c_int)>)> {
    let mut v: Vec<(String, Vec<(c_int, c_int)>)> = Vec::new();
    for strat in [1i32, 2, 3, 5, 6, 7, 9] {
        v.push((format!("strat={strat}"), vec![(ZSTD_c_strategy, strat)]));
    }
    v.push(("ck=1".into(), vec![(ZSTD_c_checksumFlag, 1)]));
    v.push(("cs=0".into(), vec![(ZSTD_c_contentSizeFlag, 0)]));
    v.push(("cs=0,ck=1".into(), vec![(ZSTD_c_contentSizeFlag, 0), (ZSTD_c_checksumFlag, 1)]));
    v.push(("lit=disable".into(), vec![(ZSTD_c_literalCompressionMode, 2)]));
    v.push(("lit=enable".into(), vec![(ZSTD_c_literalCompressionMode, 1)]));
    v.push(("rmf=enable".into(), vec![(ZSTD_c_useRowMatchFinder, 1), (ZSTD_c_strategy, 5)]));
    v.push(("ldm".into(), vec![(ZSTD_c_enableLongDistanceMatching, 1), (ZSTD_c_windowLog, 20)]));
    v.push(("maxBlk=1024".into(), vec![(ZSTD_c_maxBlockSize, 1024)]));
    v.push(("tcbs=1340".into(), vec![(ZSTD_c_targetCBlockSize, 1340)]));
    v.push(("split=6".into(), vec![(ZSTD_c_blockSplitterLevel, 6)]));
    v.push(("wlog=10".into(), vec![(ZSTD_c_windowLog, 10)]));
    v.push(("lvl=22".into(), vec![(ZSTD_c_compressionLevel, 22)]));
    v.push(("lvl=-5".into(), vec![(ZSTD_c_compressionLevel, -5)]));
    v
}

/// Exhaustive single-bit / byte-stamp sweep over small frames from every
/// configuration. This is the primary driver for the `corruption_detected`,
/// `tableLog_tooLarge`, `maxSymbolValue_*`, `literals_headerWrong` and
/// `checksum_wrong` sites.
#[test]
fn exhaustive_bit_sweep_small_frames() {
    let d = D::new();
    let mut rng = Rng::new(0xC401);
    let mut mutants = 0usize;
    for (cname, params) in configs() {
        for &shape in &[
            Shape::Text, Shape::Random, Shape::Zeros, Shape::TwoSymbols, Shape::Repeating,
            Shape::LowEntropy,
        ] {
            for &len in &[1usize, 37, 300, 2000] {
                let src = gen(shape, len, &mut rng);
                let frame = match frame_with(&params, &src) {
                    Some(f) => f,
                    None => continue,
                };
                let outcap = src.len() + 64;
                // sanity: the intact frame must decode identically
                d.cmp(&format!("{cname} {shape:?} len={} intact", src.len()), &frame, outcap);
                for off in 0..frame.len() {
                    for bit in 0..8 {
                        let mut f = frame.clone();
                        f[off] ^= 1u8 << bit;
                        d.cmp(
                            &format!("{cname} {shape:?} len={} flip off={off} bit={bit}",
                                     src.len()),
                            &f, outcap,
                        );
                        mutants += 1;
                    }
                    for stamp in [0x00u8, 0xFF] {
                        let mut f = frame.clone();
                        f[off] = stamp;
                        d.cmp(
                            &format!("{cname} {shape:?} len={} stamp off={off} v={stamp:#02x}",
                                     src.len()),
                            &f, outcap,
                        );
                        mutants += 1;
                    }
                }
                // every truncation
                for cut in 0..=frame.len() {
                    d.cmp(
                        &format!("{cname} {shape:?} len={} cut={cut}", src.len()),
                        &frame[..cut], outcap,
                    );
                    mutants += 1;
                }
            }
        }
    }
    assert!(mutants > 200_000, "expected a large mutant population, got {mutants}");
    eprintln!("exhaustive_bit_sweep_small_frames: {mutants} mutants compared");
}

/// ERRORS row 32: the XXH64 checksum trailer specifically.
#[test]
fn checksum_mutation() {
    let d = D::new();
    let mut rng = Rng::new(0xC402);
    for &shape in ALL_SHAPES {
        for &len in &[1usize, 100, 5000, 70_000] {
            let src = gen(shape, len, &mut rng);
            let frame = match frame_with(&[(ZSTD_c_checksumFlag, 1)], &src) {
                Some(f) => f,
                None => continue,
            };
            let outcap = src.len() + 64;
            let n = frame.len();
            // the last 4 bytes are the checksum
            for off in n.saturating_sub(4)..n {
                for bit in 0..8 {
                    let mut f = frame.clone();
                    f[off] ^= 1u8 << bit;
                    d.cmp(
                        &format!("checksum {shape:?} len={} off={off} bit={bit}", src.len()),
                        &f, outcap,
                    );
                }
            }
            // and dropping the checksum entirely
            d.cmp(&format!("checksum {shape:?} len={} dropped", src.len()),
                  &frame[..n - 4], outcap);
        }
    }
}

/// ERRORS row 34: the frame header specifically — every bit of the first 18
/// bytes, over every configuration, checked through `ZSTD_getFrameHeader`,
/// `ZSTD_frameHeaderSize` and a full decompress.
#[test]
fn header_bit_sweep() {
    unsafe {
        let d = D::new();
        let (cgfh, rgfh) = both::<FnGetFH>("ZSTD_getFrameHeader");
        let (cfhs, rfhs) = both::<unsafe extern "C" fn(*const c_void, size_t) -> size_t>(
            "ZSTD_frameHeaderSize",
        );
        let (cfcs, rfcs) = both::<FnGetFCS>("ZSTD_getFrameContentSize");
        let (cdid, rdid) = both::<unsafe extern "C" fn(*const c_void, size_t) -> std::os::raw::c_uint>(
            "ZSTD_getDictID_fromFrame",
        );
        let mut rng = Rng::new(0xC403);
        for (cname, params) in configs() {
            for &len in &[1usize, 500, 70_000] {
                let src = gen(Shape::Text, len, &mut rng);
                let frame = match frame_with(&params, &src) {
                    Some(f) => f,
                    None => continue,
                };
                let outcap = src.len() + 64;
                let hdr = 18.min(frame.len());
                for off in 0..hdr {
                    for bit in 0..8 {
                        let mut f = frame.clone();
                        f[off] ^= 1u8 << bit;
                        let ctx = format!("{cname} len={} hdrflip off={off} bit={bit}", src.len());
                        let p = f.as_ptr() as *const c_void;
                        let mut h1: ZSTD_frameHeader = std::mem::zeroed();
                        let mut h2: ZSTD_frameHeader = std::mem::zeroed();
                        let a = cgfh(&mut h1, p, f.len());
                        let b = rgfh(&mut h2, p, f.len());
                        d.e.eq(&format!("getFrameHeader {ctx}"), a, b);
                        if a == 0 {
                            assert_eq!(h1, h2, "getFrameHeader struct {ctx}");
                        }
                        d.e.eq(&format!("frameHeaderSize {ctx}"), cfhs(p, f.len()), rfhs(p, f.len()));
                        assert_eq!(cfcs(p, f.len()), rfcs(p, f.len()),
                                   "getFrameContentSize {ctx}");
                        assert_eq!(cdid(p, f.len()), rdid(p, f.len()),
                                   "getDictID_fromFrame {ctx}");
                        d.cmp(&ctx, &f, outcap);
                    }
                }
            }
        }
    }
}

/// The block header specifically: every bit of the 3-byte block header of the
/// first block, over every configuration.
#[test]
fn block_mutation() {
    unsafe {
        let d = D::new();
        let (cgfh, _) = both::<FnGetFH>("ZSTD_getFrameHeader");
        let mut rng = Rng::new(0xC404);
        for (cname, params) in configs() {
            for &shape in &[Shape::Text, Shape::Random, Shape::Zeros, Shape::TwoSymbols] {
                for &len in &[100usize, 5000, 70_000, 200_000] {
                    let src = gen(shape, len, &mut rng);
                    let frame = match frame_with(&params, &src) {
                        Some(f) => f,
                        None => continue,
                    };
                    let outcap = src.len() + 64;
                    let mut h: ZSTD_frameHeader = std::mem::zeroed();
                    if cgfh(&mut h, frame.as_ptr() as *const c_void, frame.len()) != 0 {
                        continue;
                    }
                    let hs = h.headerSize as usize;
                    // sweep the block header and the first 64 bytes of the body
                    let hi = (hs + 67).min(frame.len());
                    for off in hs..hi {
                        for bit in 0..8 {
                            let mut f = frame.clone();
                            f[off] ^= 1u8 << bit;
                            d.cmp(
                                &format!("{cname} {shape:?} len={} blkflip off={off} bit={bit}",
                                         src.len()),
                                &f, outcap,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Randomized multi-byte corruption over large frames from every
/// configuration — reaches the sequence-decoding and match-copy checks that a
/// single-bit flip near the start rarely gets to.
#[test]
fn randomized_multibyte_corruption() {
    let d = D::new();
    let mut rng = Rng::new(0xC405);
    let cfgs = configs();
    for i in 0..40_000 {
        let (cname, params) = &cfgs[rng.below(cfgs.len())];
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let len = LENS[rng.below(LENS.len())];
        let src = gen(shape, len, &mut rng);
        let frame = match frame_with(params, &src) {
            Some(f) => f,
            None => continue,
        };
        if frame.is_empty() {
            continue;
        }
        let mut f = frame.clone();
        let nmut = 1 + rng.below(6);
        let mut desc = String::new();
        for _ in 0..nmut {
            let off = rng.below(f.len());
            let v = match rng.below(4) {
                0 => 0x00,
                1 => 0xFF,
                2 => rng.byte(),
                _ => f[off] ^ (1u8 << rng.below(8)),
            };
            desc.push_str(&format!("{off}:{v:02x} "));
            f[off] = v;
        }
        // sometimes also truncate
        if rng.below(3) == 0 {
            let cut = rng.below(f.len() + 1);
            f.truncate(cut);
            desc.push_str(&format!("cut={cut}"));
        }
        // sometimes vary the output capacity
        let outcap = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => src.len() / 2 + 1,
            _ => src.len() + 64,
        };
        d.cmp(
            &format!("#{i} {cname} {shape:?} len={} [{desc}] outcap={outcap}", src.len()),
            &f, outcap,
        );
    }
}

/// Structured attack: build buffers that have a valid magic and a plausible
/// header but a fully random body. Reaches the entropy-table and literals
/// header checks directly.
#[test]
fn random_body_with_valid_header() {
    unsafe {
        let d = D::new();
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let mut rng = Rng::new(0xC406);
        let mut out = vec![0u8; 1 << 18];
        for i in 0..60_000 {
            let mut v: Vec<u8> = 0xFD2FB528u32.to_le_bytes().to_vec();
            // frame header descriptor + window descriptor + optional FCS
            let fhd = rng.byte();
            v.push(fhd);
            let nbody = 1 + rng.below(80);
            for _ in 0..nbody {
                v.push(rng.byte());
            }
            let p = v.as_ptr() as *const c_void;
            let ctx = format!("#{i} fhd={fhd:#02x} len={} [{}]", v.len(), hexdump(&v, 24));
            d.e.eq(&format!("decompress {ctx}"),
                   cd(out.as_mut_ptr() as *mut c_void, out.len(), p, v.len()),
                   rd(out.as_mut_ptr() as *mut c_void, out.len(), p, v.len()));
            // and via the DCtx path with a tight output buffer
            d.cmp(&format!("dctx {ctx}"), &v, rng.below(4096));
        }
    }
}

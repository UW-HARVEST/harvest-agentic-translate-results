//! Phase B, CONFIGS.md rows 17–31: `ZSTD_compress2` driven through the
//! advanced-parameter surface. Every row sets the options on BOTH libraries'
//! CCtx, compresses the same input, and asserts byte-identical frames plus a
//! successful cross-decompression.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_void};

type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;

/// A pair of contexts, one per library, plus the symbols needed to drive them.
struct Pair {
    e: Err2,
    cc: *mut c_void,
    rc: *mut c_void,
    cd: *mut c_void,
    rd: *mut c_void,
    set_c: libloading::Symbol<'static, FnSetParam>,
    set_r: libloading::Symbol<'static, FnSetParam>,
    dset_c: libloading::Symbol<'static, FnSetParam>,
    dset_r: libloading::Symbol<'static, FnSetParam>,
    rst_c: libloading::Symbol<'static, FnReset>,
    rst_r: libloading::Symbol<'static, FnReset>,
    drst_c: libloading::Symbol<'static, FnReset>,
    drst_r: libloading::Symbol<'static, FnReset>,
    c2_c: libloading::Symbol<'static, FnCompress2>,
    c2_r: libloading::Symbol<'static, FnCompress2>,
    dd_c: libloading::Symbol<'static, FnDecompressDCtx>,
    dd_r: libloading::Symbol<'static, FnDecompressDCtx>,
    bound: libloading::Symbol<'static, FnBound>,
}

impl Pair {
    fn new() -> Pair {
        unsafe {
            let (cn, rn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
            let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
            let (set_c, set_r) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
            let (dset_c, dset_r) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
            let (rst_c, rst_r) = both::<FnReset>("ZSTD_CCtx_reset");
            let (drst_c, drst_r) = both::<FnReset>("ZSTD_DCtx_reset");
            let (c2_c, c2_r) = both::<FnCompress2>("ZSTD_compress2");
            let (dd_c, dd_r) = both::<FnDecompressDCtx>("ZSTD_decompressDCtx");
            let (bound, _) = both::<FnBound>("ZSTD_compressBound");
            Pair {
                e: Err2::new(),
                cc: cn(),
                rc: rn(),
                cd: cdn(),
                rd: rdn(),
                set_c, set_r, dset_c, dset_r, rst_c, rst_r, drst_c, drst_r,
                c2_c, c2_r, dd_c, dd_r, bound,
            }
        }
    }

    fn reset(&self) {
        unsafe {
            (self.rst_c)(self.cc, ZSTD_reset_session_and_parameters);
            (self.rst_r)(self.rc, ZSTD_reset_session_and_parameters);
            (self.drst_c)(self.cd, ZSTD_reset_session_and_parameters);
            (self.drst_r)(self.rd, ZSTD_reset_session_and_parameters);
        }
    }

    /// Set a compression parameter on both libraries, asserting identical
    /// return values. Returns false if the parameter was rejected.
    #[track_caller]
    fn set(&self, ctx: &str, id: c_int, v: c_int) -> bool {
        unsafe {
            let a = (self.set_c)(self.cc, id, v);
            let b = (self.set_r)(self.rc, id, v);
            self.e.eq_or_oom(&format!("{ctx}: CCtx_setParameter({id},{v})"), a, b);
            !self.e.c.is_err(a) && !self.e.r.is_err(b)
        }
    }

    #[track_caller]
    fn dset(&self, ctx: &str, id: c_int, v: c_int) -> bool {
        unsafe {
            let a = (self.dset_c)(self.cd, id, v);
            let b = (self.dset_r)(self.rd, id, v);
            self.e.eq_or_oom(&format!("{ctx}: DCtx_setParameter({id},{v})"), a, b);
            !self.e.c.is_err(a) && !self.e.r.is_err(b)
        }
    }

    /// Compress `src` through both libraries and assert byte-identical frames.
    /// On success, cross-decompresses (C decodes RS's frame and vice versa) and
    /// checks the plaintext round-trips. Returns the C frame.
    #[track_caller]
    fn run(&self, ctx: &str, src: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let cap = (self.bound)(src.len()) + 256;
            let mut o1 = vec![0xA5u8; cap];
            let mut o2 = vec![0xA5u8; cap];
            let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
            let a = (self.c2_c)(self.cc, o1.as_mut_ptr() as *mut c_void, cap, sp, src.len());
            let b = (self.c2_r)(self.rc, o2.as_mut_ptr() as *mut c_void, cap, sp, src.len());
            if !self.e.eq_or_oom(&format!("{ctx}: compress2"), a, b) {
                return None;
            }
            if self.e.c.is_err(a) {
                return None;
            }
            assert_bytes_eq(&format!("{ctx}: frame bytes"), &o1[..a], &o2[..b]);

            // cross-decompress
            let mut d1 = vec![0u8; src.len() + 64];
            let mut d2 = vec![0u8; src.len() + 64];
            let x = (self.dd_c)(self.cd, d1.as_mut_ptr() as *mut c_void, d1.len(),
                                o2.as_ptr() as *const c_void, b);
            let y = (self.dd_r)(self.rd, d2.as_mut_ptr() as *mut c_void, d2.len(),
                                o1.as_ptr() as *const c_void, a);
            if !self.e.eq_or_oom(&format!("{ctx}: cross-decompress"), x, y) {
                o1.truncate(a);
                return Some(o1);
            }
            if !self.e.c.is_err(x) {
                assert_eq!(x, src.len(), "{ctx}: round-trip length");
                assert_bytes_eq(&format!("{ctx}: C decoded RS frame"), &d1[..x], src);
                assert_bytes_eq(&format!("{ctx}: RS decoded C frame"), &d2[..y], src);
            }
            o1.truncate(a);
            Some(o1)
        }
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        unsafe {
            let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
            cf(self.cc);
            rf(self.rc);
            let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
            cdf(self.cd);
            rdf(self.rd);
        }
    }
}

fn corpus(rng: &mut Rng, lens: &[usize]) -> Vec<(Shape, Vec<u8>)> {
    let mut v = Vec::new();
    for &shape in ALL_SHAPES {
        for &len in lens {
            v.push((shape, gen(shape, len, rng)));
        }
    }
    v
}

/// CONFIGS row 17: every `strategy` × every shape × several lengths.
#[test]
fn strategy_sweep() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB401);
    let data = corpus(&mut rng, &[0, 1, 1024, 20_000, 131_100]);
    for &strat in STRATEGIES {
        for (shape, src) in &data {
            p.reset();
            let ctx = format!("strategy={strat} shape={shape:?} len={}", src.len());
            if !p.set(&ctx, ZSTD_c_strategy, strat) {
                continue;
            }
            p.run(&ctx, src);
        }
    }
}

/// CONFIGS row 18: `strategy` × `useRowMatchFinder`.
#[test]
fn strategy_x_row_match_finder() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB402);
    let data = corpus(&mut rng, &[1, 4096, 40_000, 200_000]);
    for &strat in STRATEGIES {
        for rmf in [0i32, 1, 2] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("strat={strat} rmf={rmf} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_strategy, strat) {
                    continue;
                }
                if !p.set(&ctx, ZSTD_c_useRowMatchFinder, rmf) {
                    continue;
                }
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS row 19: `windowLog` × the other table logs at their bounds.
#[test]
fn window_and_table_logs() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB403);
    let data = corpus(&mut rng, &[1, 1024, 70_000, 200_000]);
    for wlog in [10i32, 11, 15, 17, 20, 23, 27, 31] {
        for &(hlog, clog, slog) in &[
            (6i32, 6i32, 1i32),
            (30, 30, 30),
            (17, 16, 1),
            (23, 24, 5),
            (12, 13, 3),
        ] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!(
                    "wlog={wlog} hlog={hlog} clog={clog} slog={slog} shape={shape:?} len={}",
                    src.len()
                );
                if !p.set(&ctx, ZSTD_c_windowLog, wlog) { continue; }
                if !p.set(&ctx, ZSTD_c_hashLog, hlog) { continue; }
                if !p.set(&ctx, ZSTD_c_chainLog, clog) { continue; }
                if !p.set(&ctx, ZSTD_c_searchLog, slog) { continue; }
                // A large window needs decoder opt-in.
                p.dset(&ctx, ZSTD_d_windowLogMax, 31);
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS row 20: `minMatch` 3..7 × `strategy` 1..9 (per-strategy clamp).
#[test]
fn min_match_x_strategy() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB404);
    let data = corpus(&mut rng, &[1, 100, 8192, 60_000]);
    for mm in [2i32, 3, 4, 5, 6, 7, 8] {
        for &strat in STRATEGIES {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("mm={mm} strat={strat} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_minMatch, mm) { continue; }
                if !p.set(&ctx, ZSTD_c_strategy, strat) { continue; }
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS row 21: `targetLength` × the strategies that use it.
#[test]
fn target_length_sweep() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB405);
    let data = corpus(&mut rng, &[1, 1024, 50_000]);
    for tl in [0i32, 1, 16, 32, 63, 64, 999, 1024, 131_071, 131_072] {
        for &strat in &[1i32, 2, 3, 4, 5, 6, 7, 8, 9] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("tl={tl} strat={strat} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_targetLength, tl) { continue; }
                if !p.set(&ctx, ZSTD_c_strategy, strat) { continue; }
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS row 22: the long-distance matcher and all four of its knobs.
#[test]
fn long_distance_matching() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB406);
    // LDM only matters for larger inputs.
    let mut data = Vec::new();
    for &shape in &[Shape::LongMatches, Shape::Repeating, Shape::Random, Shape::Text, Shape::Zeros] {
        for &len in &[200_000usize, 400_000] {
            data.push((shape, gen(shape, len, &mut rng)));
        }
    }
    for ldm in [0i32, 1, 2] {
        for &(hl, mm, bs, hr) in &[
            (0i32, 0i32, 0i32, 0i32),
            (6, 4, 1, 0),
            (30, 4096, 8, 25),
            (20, 64, 3, 7),
            (14, 32, 4, 12),
        ] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!(
                    "ldm={ldm} hl={hl} mm={mm} bs={bs} hr={hr} shape={shape:?} len={}",
                    src.len()
                );
                if !p.set(&ctx, ZSTD_c_enableLongDistanceMatching, ldm) { continue; }
                if !p.set(&ctx, ZSTD_c_ldmHashLog, hl) { continue; }
                if !p.set(&ctx, ZSTD_c_ldmMinMatch, mm) { continue; }
                if !p.set(&ctx, ZSTD_c_ldmBucketSizeLog, bs) { continue; }
                if !p.set(&ctx, ZSTD_c_ldmHashRateLog, hr) { continue; }
                p.dset(&ctx, ZSTD_d_windowLogMax, 31);
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS row 23: the 8 combinations of the three frame flags.
#[test]
fn frame_flag_combinations() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB407);
    let data = corpus(&mut rng, &[0, 1, 1024, 70_000]);
    for cs in [0i32, 1] {
        for ck in [0i32, 1] {
            for di in [0i32, 1] {
                for (shape, src) in &data {
                    p.reset();
                    let ctx =
                        format!("cs={cs} ck={ck} di={di} shape={shape:?} len={}", src.len());
                    if !p.set(&ctx, ZSTD_c_contentSizeFlag, cs) { continue; }
                    if !p.set(&ctx, ZSTD_c_checksumFlag, ck) { continue; }
                    if !p.set(&ctx, ZSTD_c_dictIDFlag, di) { continue; }
                    p.run(&ctx, src);
                }
            }
        }
    }
}

/// CONFIGS row 24: the magicless frame format, decoded with the matching
/// `ZSTD_d_format`.
#[test]
fn magicless_format() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB408);
    let data = corpus(&mut rng, &[0, 1, 1024, 70_000]);
    for fmt in [0i32, 1] {
        for ck in [0i32, 1] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("fmt={fmt} ck={ck} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_format, fmt) { continue; }
                if !p.set(&ctx, ZSTD_c_checksumFlag, ck) { continue; }
                if !p.dset(&ctx, ZSTD_d_format, fmt) { continue; }
                p.run(&ctx, src);
                // and a MISMATCHED decoder format must fail identically
                p.dset(&ctx, ZSTD_d_format, 1 - fmt);
                p.run(&format!("{ctx} mismatched-dformat"), src);
            }
        }
    }
}

/// CONFIGS rows 25–26: `targetCBlockSize` and `maxBlockSize`.
#[test]
fn block_size_params() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB409);
    let mut data = Vec::new();
    for &shape in ALL_SHAPES {
        for &len in &[1usize, 131_072, 200_000, 400_000] {
            data.push((shape, gen(shape, len, &mut rng)));
        }
    }
    for tcbs in [0i32, 1340, 2000, 65536, 131_072] {
        for (shape, src) in &data {
            p.reset();
            let ctx = format!("tcbs={tcbs} shape={shape:?} len={}", src.len());
            if !p.set(&ctx, ZSTD_c_targetCBlockSize, tcbs) { continue; }
            p.run(&ctx, src);
        }
    }
    for mbs in [1024i32, 4096, 16384, 65536, 131_072] {
        for dmbs in [1024i32, 131_072] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("mbs={mbs} dmbs={dmbs} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_maxBlockSize, mbs) { continue; }
                if !p.dset(&ctx, ZSTD_d_maxBlockSize, dmbs) { continue; }
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS row 27: `blockSplitterLevel` 0..6 × `splitAfterSequences`.
#[test]
fn block_splitter() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB40A);
    let mut data = Vec::new();
    for &shape in ALL_SHAPES {
        for &len in &[131_072usize, 300_000] {
            data.push((shape, gen(shape, len, &mut rng)));
        }
    }
    for bsl in [0i32, 1, 2, 3, 4, 5, 6] {
        for sas in [0i32, 1, 2] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("bsl={bsl} sas={sas} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_blockSplitterLevel, bsl) { continue; }
                if !p.set(&ctx, ZSTD_c_splitAfterSequences, sas) { continue; }
                p.run(&ctx, src);
            }
        }
    }
}

/// CONFIGS rows 28–31: literal mode, srcSizeHint, forceMaxWindow, rsyncable,
/// plus the remaining boolean/tri-state experimental parameters.
#[test]
fn remaining_experimental_params() {
    let p = Pair::new();
    let mut rng = Rng::new(0xB40B);
    let data = corpus(&mut rng, &[0, 1, 1024, 40_000, 200_000]);

    // row 28: literalCompressionMode
    for lcm in [0i32, 1, 2] {
        for (shape, src) in &data {
            p.reset();
            let ctx = format!("lcm={lcm} shape={shape:?} len={}", src.len());
            if !p.set(&ctx, ZSTD_c_literalCompressionMode, lcm) { continue; }
            p.run(&ctx, src);
        }
    }
    // row 29: srcSizeHint (deliberately mismatching the real length)
    for hint in [0i32, 1, 1024, 1 << 20, i32::MAX] {
        for (shape, src) in &data {
            p.reset();
            let ctx = format!("srcSizeHint={hint} shape={shape:?} len={}", src.len());
            if !p.set(&ctx, ZSTD_c_srcSizeHint, hint) { continue; }
            p.run(&ctx, src);
        }
    }
    // row 30: forceMaxWindow × windowLog
    for fmw in [0i32, 1] {
        for wl in [10i32, 17, 27, 31] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("fmw={fmw} wl={wl} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, ZSTD_c_forceMaxWindow, fmw) { continue; }
                if !p.set(&ctx, ZSTD_c_windowLog, wl) { continue; }
                p.dset(&ctx, ZSTD_d_windowLogMax, 31);
                p.run(&ctx, src);
            }
        }
    }
    // row 31 + the remaining knobs
    for (name, id) in [
        ("rsyncable", ZSTD_c_rsyncable),
        ("enableDedicatedDictSearch", ZSTD_c_enableDedicatedDictSearch),
        ("deterministicRefPrefix", ZSTD_c_deterministicRefPrefix),
        ("prefetchCDictTables", ZSTD_c_prefetchCDictTables),
        ("enableSeqProducerFallback", ZSTD_c_enableSeqProducerFallback),
        ("repcodeResolution", ZSTD_c_repcodeResolution),
        ("validateSequences", ZSTD_c_validateSequences),
        ("blockDelimiters", ZSTD_c_blockDelimiters),
        ("forceAttachDict", ZSTD_c_forceAttachDict),
    ] {
        for v in [0i32, 1, 2] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("{name}={v} shape={shape:?} len={}", src.len());
                if !p.set(&ctx, id, v) { continue; }
                p.run(&ctx, src);
            }
        }
    }
    // decompressor-side booleans
    for (name, id) in [
        ("forceIgnoreChecksum", ZSTD_d_forceIgnoreChecksum),
        ("refMultipleDDicts", ZSTD_d_refMultipleDDicts),
        ("disableHuffmanAssembly", ZSTD_d_disableHuffmanAssembly),
        ("stableOutBuffer", ZSTD_d_stableOutBuffer),
    ] {
        for v in [0i32, 1] {
            for (shape, src) in &data {
                p.reset();
                let ctx = format!("d_{name}={v} shape={shape:?} len={}", src.len());
                p.set(&ctx, ZSTD_c_checksumFlag, 1);
                if !p.dset(&ctx, id, v) { continue; }
                p.run(&ctx, src);
            }
        }
    }
}

/// Randomized multi-parameter property sweep: pick a random valid value for a
/// random subset of every parameter, then compress. This is the combination
/// space that per-parameter tests cannot reach.
#[test]
fn random_multi_param_sweep() {
    unsafe {
        let p = Pair::new();
        let (cbnd, _) = both::<unsafe extern "C" fn(c_int) -> ZSTD_bounds>("ZSTD_cParam_getBounds");
        let mut rng = Rng::new(0xB40C);
        for i in 0..2500 {
            p.reset();
            let mut desc = String::new();
            let nparams = 1 + rng.below(8);
            for _ in 0..nparams {
                let (name, id) = ALL_CPARAMS[rng.below(ALL_CPARAMS.len())];
                let b = cbnd(id);
                if b.error != 0 {
                    continue;
                }
                // mostly in-range, occasionally out of range
                let v = if rng.below(8) == 0 {
                    rng.next_u32() as c_int
                } else {
                    rng.range(b.lowerBound as i64, b.upperBound as i64) as c_int
                };
                desc.push_str(&format!("{name}={v} "));
                p.set(&format!("#{i} {desc}"), id, v);
            }
            // decoder side: allow big windows / blocks so decode isn't the limiter
            p.dset(&format!("#{i}"), ZSTD_d_windowLogMax, 31);
            p.dset(&format!("#{i}"), ZSTD_d_maxBlockSize, 131_072);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = LENS[rng.below(LENS.len())];
            let src = gen(shape, len, &mut rng);
            p.run(&format!("#{i} [{desc}] shape={shape:?} len={}", src.len()), &src);
        }
    }
}

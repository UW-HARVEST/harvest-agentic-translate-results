//! Phase B — CONFIGS.md rows 147..156: `dictBuilder/` (zdict, cover, fastcover,
//! divsufsort).
//!
//! Everything is reached through `dlsym` on the two shared objects, so the C
//! reference and the Rust port are driven with bit-identical arguments and the
//! *entire* destination buffer is compared, not just the returned length.
//!
//! `notificationLevel` is kept at 0 for the bulk of the grid so nothing is
//! written to stderr; a small dedicated test walks levels 1..4 with tiny
//! corpora (stderr is deliberately not compared, both libraries just get the
//! same value).
#![allow(dead_code)]
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_void};

// ---------------------------------------------------------------- structs
// Layouts copied verbatim from c_src/src/include/zdict.h and
// c_src/src/dictBuilder/cover.h.  The C build has ZSTD_MULTITHREAD *undefined*
// (no pthread_* relocations in c_src/build/libzstd.so), so
// ZSTD_pthread_mutex_t / ZSTD_pthread_cond_t are plain `int`.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct ZDICT_params_t {
    compressionLevel: c_int,
    notificationLevel: c_uint,
    dictID: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct ZDICT_cover_params_t {
    k: c_uint,
    d: c_uint,
    steps: c_uint,
    nbThreads: c_uint,
    splitPoint: f64,
    shrinkDict: c_uint,
    shrinkDictMaxRegression: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct ZDICT_fastCover_params_t {
    k: c_uint,
    d: c_uint,
    f: c_uint,
    steps: c_uint,
    nbThreads: c_uint,
    splitPoint: f64,
    accel: c_uint,
    shrinkDict: c_uint,
    shrinkDictMaxRegression: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct ZDICT_legacy_params_t {
    selectivityLevel: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct COVER_epoch_info_t {
    num: u32,
    size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct COVER_dictSelection_t {
    dictContent: *mut u8,
    dictSize: usize,
    totalCompressedSize: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct COVER_best_t {
    mutex: c_int,
    cond: c_int,
    liveJobs: usize,
    dict: *mut c_void,
    dictSize: usize,
    parameters: ZDICT_cover_params_t,
    compressedSize: usize,
}

// Guard the ABI assumptions (`python3 ctypes` on the C build reports 12/48/56/16).
const _: () = assert!(std::mem::size_of::<ZDICT_params_t>() == 12);
const _: () = assert!(std::mem::size_of::<ZDICT_cover_params_t>() == 48);
const _: () = assert!(std::mem::size_of::<ZDICT_fastCover_params_t>() == 56);
const _: () = assert!(std::mem::size_of::<ZDICT_legacy_params_t>() == 16);
const _: () = assert!(std::mem::size_of::<COVER_dictSelection_t>() == 24);
const _: () = assert!(std::mem::size_of::<COVER_best_t>() == 88);

const ZDICT_DICTSIZE_MIN: usize = 256;

// ---------------------------------------------------------------- fn types

type FnTrain =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize;
type FnTrainCover = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_cover_params_t,
) -> usize;
type FnOptCover = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    *mut ZDICT_cover_params_t,
) -> usize;
type FnTrainFast = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_fastCover_params_t,
) -> usize;
type FnOptFast = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    *mut ZDICT_fastCover_params_t,
) -> usize;
type FnTrainLegacy = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_legacy_params_t,
) -> usize;
type FnFinalize = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_params_t,
) -> usize;
type FnGetDictID = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnGetHdrSize = unsafe extern "C" fn(*const c_void, usize) -> usize;

type FnCoverSum = unsafe extern "C" fn(*const usize, c_uint) -> usize;
type FnComputeEpochs = unsafe extern "C" fn(u32, u32, u32, u32) -> COVER_epoch_info_t;
type FnCheckTotal = unsafe extern "C" fn(
    ZDICT_cover_params_t,
    *const usize,
    *const u8,
    *mut usize,
    usize,
    usize,
    *mut u8,
    usize,
) -> usize;
type FnBestVoid = unsafe extern "C" fn(*mut COVER_best_t);
type FnBestFinish =
    unsafe extern "C" fn(*mut COVER_best_t, ZDICT_cover_params_t, COVER_dictSelection_t);
type FnDsError = unsafe extern "C" fn(usize) -> COVER_dictSelection_t;
type FnDsIsError = unsafe extern "C" fn(COVER_dictSelection_t) -> c_uint;
type FnDsFree = unsafe extern "C" fn(COVER_dictSelection_t);
type FnSelectDict = unsafe extern "C" fn(
    *mut u8,
    usize,
    usize,
    *const u8,
    *const usize,
    c_uint,
    usize,
    usize,
    ZDICT_cover_params_t,
    *mut usize,
    usize,
) -> COVER_dictSelection_t;

type FnDivsufsort = unsafe extern "C" fn(*const u8, *mut c_int, c_int, c_int) -> c_int;
type FnDivbwt = unsafe extern "C" fn(
    *const u8,
    *mut u8,
    *mut c_int,
    c_int,
    *mut u8,
    *mut c_int,
    c_int,
) -> c_int;

// ---------------------------------------------------------------- corpora

/// A flat concatenated sample buffer + its `samplesSizes` array.
///
/// Samples are cut out of one shared "master" blob at rotating offsets so they
/// have plenty of common substrings — otherwise every training call would just
/// return `dictionaryCreation_failed` and the grid would be vacuous.
struct Corpus {
    buf: Vec<u8>,
    sizes: Vec<usize>,
}

impl Corpus {
    fn new(class: usize, sizes: Vec<usize>, seed: u64) -> Corpus {
        let maxs = sizes.iter().copied().max().unwrap_or(0);
        let master = gen_class(class, maxs * 2 + 256, seed);
        let mut buf = Vec::with_capacity(sizes.iter().sum());
        for (i, &s) in sizes.iter().enumerate() {
            let span = master.len() - s;
            let off = if span == 0 { 0 } else { (i * 4099 + 17) % span };
            buf.extend_from_slice(&master[off..off + s]);
        }
        Corpus { buf, sizes }
    }
    /// `nb` equally sized samples.
    fn uniform(class: usize, nb: usize, size: usize, seed: u64) -> Corpus {
        Corpus::new(class, vec![size; nb], seed)
    }
    fn nb(&self) -> c_uint {
        self.sizes.len() as c_uint
    }
    fn sp(&self) -> *const c_void {
        self.buf.as_ptr() as *const c_void
    }
    fn szp(&self) -> *const usize {
        self.sizes.as_ptr()
    }
    fn total(&self) -> usize {
        self.buf.len()
    }
    /// `offsets[i]` = start of sample i, `offsets[nb]` = total (COVER convention).
    fn offsets(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(self.sizes.len() + 1);
        let mut a = 0usize;
        v.push(0usize);
        for &s in &self.sizes {
            a += s;
            v.push(a);
        }
        v
    }
}

// ---------------------------------------------------------------- helpers

/// `dictBufferCapacity` grid: far too small, exactly `ZDICT_DICTSIZE_MIN`,
/// small, generous.
const CAPS: [usize; 5] = [100, 256, 1024, 8192, 110 * 1024];

fn zparams(level: c_int, dict_id: c_uint) -> ZDICT_params_t {
    ZDICT_params_t { compressionLevel: level, notificationLevel: 0, dictID: dict_id }
}

/// Call a `(dict, cap, samples, sizes, nbSamples, params_by_value)` trainer in
/// both libraries and compare the return code *and* the whole dict buffer.
#[track_caller]
unsafe fn diff_train_p<P: Copy>(
    what: &str,
    f: (
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint, P) -> usize,
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint, P) -> usize,
    ),
    cap: usize,
    co: &Corpus,
    p: P,
) -> usize {
    let (ic, ir) = duo::<FnIsError>("ZDICT_isError");
    let mut dc = vec![0xA5u8; cap.max(1)];
    let mut dr = vec![0xA5u8; cap.max(1)];
    let rc = (f.0)(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), p);
    let rr = (f.1)(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), p);
    eqv(&format!("{what}: return"), rc, rr);
    eqv(&format!("{what}: isError"), ic(rc), ir(rr));
    eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
    if !is_err(rc) {
        assert!(rc <= cap, "{what}: returned {rc} > capacity {cap}");
    }
    rc
}

/// Same, but the parameter block is an in/out pointer (the `optimize*` family).
#[track_caller]
unsafe fn diff_train_io<P: Copy + PartialEq + std::fmt::Debug>(
    what: &str,
    f: (
        unsafe extern "C" fn(
            *mut c_void,
            usize,
            *const c_void,
            *const usize,
            c_uint,
            *mut P,
        ) -> usize,
        unsafe extern "C" fn(
            *mut c_void,
            usize,
            *const c_void,
            *const usize,
            c_uint,
            *mut P,
        ) -> usize,
    ),
    cap: usize,
    co: &Corpus,
    p: P,
) -> usize {
    let (ic, ir) = duo::<FnIsError>("ZDICT_isError");
    let mut dc = vec![0xA5u8; cap.max(1)];
    let mut dr = vec![0xA5u8; cap.max(1)];
    let mut pc = p;
    let mut pr = p;
    let rc = (f.0)(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), &mut pc);
    let rr = (f.1)(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), &mut pr);
    eqv(&format!("{what}: return"), rc, rr);
    eqv(&format!("{what}: isError"), ic(rc), ir(rr));
    eqv(&format!("{what}: out-params"), pc, pr);
    eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
    rc
}

/// `ZDICT_getDictID` / `ZDICT_getDictHeaderSize` on the same buffer in both.
#[track_caller]
unsafe fn diff_dict_probe(what: &str, dict: &[u8]) {
    let (idc, idr) = duo::<FnGetDictID>("ZDICT_getDictID");
    let (hc, hr) = duo::<FnGetHdrSize>("ZDICT_getDictHeaderSize");
    let (ic, ir) = duo::<FnIsError>("ZDICT_isError");
    let (nc, nr) = duo::<FnErrName>("ZDICT_getErrorName");
    let p = dict.as_ptr() as *const c_void;
    eqv(&format!("{what}: getDictID"), idc(p, dict.len()), idr(p, dict.len()));
    let a = hc(p, dict.len());
    let b = hr(p, dict.len());
    eqv(&format!("{what}: getDictHeaderSize"), a, b);
    eqv(&format!("{what}: headerSize isError"), ic(a), ir(b));
    eqv(&format!("{what}: headerSize errName"), cstr(nc(a)), cstr(nr(b)));
}

#[track_caller]
fn eqi32(what: &str, c: &[c_int], r: &[c_int]) {
    assert_eq!(c.len(), r.len(), "{what}: length mismatch");
    for i in 0..c.len() {
        if c[i] != r[i] {
            panic!("{what}: first difference at index {i} (C={} Rust={}); len={}", c[i], r[i], c.len());
        }
    }
}

// =================================================================== row 147

#[test]
fn row147_train_from_buffer() {
    unsafe {
        let (fc, fr) = duo::<FnTrain>("ZDICT_trainFromBuffer");
        let (ic, ir) = duo::<FnIsError>("ZDICT_isError");
        let mut successes = 0usize;
        let mut errors = 0usize;

        // nbSamples {1,4,32,256} × per-sample size × capacity grid × all classes
        let profiles: [(usize, usize); 9] = [
            (1, 4096),
            (2, 2048),
            (4, 1024),
            (8, 512),
            (16, 400),
            (32, 512),
            (64, 256),
            (128, 200),
            (256, 128),
        ];
        for class in 0..N_CLASSES {
            for &(nb, size) in &profiles {
              for seed_i in 0..2u64 {
                let co = Corpus::uniform(
                    class,
                    nb,
                    size,
                    0xD1C7 ^ (class as u64) << 8 ^ (seed_i << 32),
                );
                for &cap in &CAPS {
                    let what = format!(
                        "row147 class={} nb={nb} size={size} cap={cap} seed={seed_i}",
                        CLASS_NAMES[class]
                    );
                    let mut dc = vec![0xA5u8; cap];
                    let mut dr = vec![0xA5u8; cap];
                    let rc = fc(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
                    let rr = fr(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
                    eqv(&format!("{what}: return"), rc, rr);
                    eqv(&format!("{what}: isError"), ic(rc), ir(rr));
                    eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
                    if is_err(rc) {
                        errors += 1;
                    } else {
                        successes += 1;
                        assert!(rc <= cap);
                        diff_dict_probe(&what, &dc[..rc]);
                    }
                }
              }
            }
        }

        // ragged sample sizes, including 0/1/7-byte samples
        for class in 0..N_CLASSES {
          for (ri, ladder) in [
            [0usize, 1, 7, 8, 97, 300, 1000],
            [1, 2, 3, 4, 5, 6, 7],
            [8, 8, 9, 63, 64, 65, 4096],
          ]
          .iter()
          .enumerate()
          {
           for &cap in &[CAPS[1], CAPS[2], CAPS[3]] {
            let sizes: Vec<usize> = (0..40).map(|i| ladder[i % 7]).collect();
            let co = Corpus::new(class, sizes, 0xBEEF + class as u64 + (ri as u64) * 977);
            let what = format!("row147 ragged#{ri} class={} cap={cap}", CLASS_NAMES[class]);
            let mut dc = vec![0xA5u8; cap];
            let mut dr = vec![0xA5u8; cap];
            let rc = fc(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
            let rr = fr(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
            eqv(&format!("{what}: return"), rc, rr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            if is_err(rc) {
                errors += 1
            } else {
                successes += 1
            }
           }
          }
        }

        // one very large sample mixed with tiny ones, generous capacity
        for class in [4usize, 5, 3] {
            let mut sizes = vec![128 * 1024usize, 1, 7, 3, 3000, 3000, 3000, 3000];
            sizes.extend(std::iter::repeat(200).take(8));
            let co = Corpus::new(class, sizes, 0x5177 + class as u64);
            let what = format!("row147 huge class={}", CLASS_NAMES[class]);
            let cap = CAPS[4];
            let mut dc = vec![0xA5u8; cap];
            let mut dr = vec![0xA5u8; cap];
            let rc = fc(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
            let rr = fr(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
            eqv(&format!("{what}: return"), rc, rr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            if is_err(rc) {
                errors += 1
            } else {
                successes += 1;
                diff_dict_probe(&what, &dc[..rc]);
            }
        }

        // nbSamples == 0 (valid call, expected to be rejected identically)
        let empty: [usize; 0] = [];
        let cap = 1024usize;
        let mut dc = vec![0xA5u8; cap];
        let mut dr = vec![0xA5u8; cap];
        let rc = fc(dc.as_mut_ptr() as *mut c_void, cap, dc.as_ptr() as *const c_void, empty.as_ptr(), 0);
        let rr = fr(dr.as_mut_ptr() as *mut c_void, cap, dr.as_ptr() as *const c_void, empty.as_ptr(), 0);
        eqv("row147 nbSamples=0: return", rc, rr);

        // randomized sweep: fresh corpus shape + capacity on every iteration
        {
            let mut rng = Rng::new(0x147_0FF);
            for it in 0..600 {
                let class = rng.below(N_CLASSES);
                let nb = 1 + rng.below(80);
                let sizes: Vec<usize> = (0..nb)
                    .map(|_| {
                        let pick = rng.below(6);
                        match pick {
                            0 => rng.below(16),
                            1 => 8 + rng.below(64),
                            2 => 100 + rng.below(400),
                            3 => 512,
                            4 => 1000 + rng.below(2000),
                            _ => rng.below(300),
                        }
                    })
                    .collect();
                let co = Corpus::new(class, sizes, 0x9000 + it as u64);
                let cap = [100usize, 256, 300, 1024, 4096, 8192, 40_000][rng.below(7)];
                let what = format!(
                    "row147 fuzz#{it} class={} nb={nb} total={} cap={cap}",
                    CLASS_NAMES[class],
                    co.total()
                );
                let mut dc = vec![0xA5u8; cap];
                let mut dr = vec![0xA5u8; cap];
                let rc = fc(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
                let rr = fr(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
                eqv(&format!("{what}: return"), rc, rr);
                eqv(&format!("{what}: isError"), ic(rc), ir(rr));
                eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
                if is_err(rc) {
                    errors += 1;
                } else {
                    successes += 1;
                    diff_dict_probe(&what, &dc[..rc]);
                }
            }
        }

        assert!(successes >= 900, "row147 only produced {successes} dictionaries");
        assert!(errors >= 200, "row147 exercised no failure paths ({errors})");
    }
}

// =================================================================== row 148

#[test]
fn row148_train_cover() {
    unsafe {
        let f = duo::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let mut ok = 0usize;
        let mut rng = Rng::new(0x148);

        // k × d × nbThreads × splitPoint × shrinkDict × shrinkDictMaxRegression
        for &k in &[6u32, 16, 32, 50, 100, 200, 512, 1024, 2048] {
            for &d in &[6u32, 8] {
                for &nb_threads in &[0u32, 1] {
                    for &shrink in &[0u32, 1] {
                        let class = rng.below(N_CLASSES);
                        let co = Corpus::uniform(class, 32, 512, 0x1480 + k as u64 + d as u64);
                        let p = ZDICT_cover_params_t {
                            k,
                            d,
                            steps: 4,
                            nbThreads: nb_threads,
                            // forced to 1.0 by the entry point, but pass a
                            // variety anyway so both see the same value
                            splitPoint: if shrink == 0 { 0.0 } else { 0.75 },
                            shrinkDict: shrink,
                            shrinkDictMaxRegression: if shrink == 0 { 0 } else { 5 },
                            zParams: zparams(0, 0),
                        };
                        let what = format!(
                            "row148 k={k} d={d} nbThreads={nb_threads} shrinkDict={shrink} class={}",
                            CLASS_NAMES[class]
                        );
                        let r = diff_train_p(&what, f, CAPS[3], &co, p);
                        if !is_err(r) {
                            ok += 1;
                            let mut buf = vec![0xA5u8; CAPS[3]];
                            let rr = (f.0)(
                                buf.as_mut_ptr() as *mut c_void,
                                CAPS[3],
                                co.sp(),
                                co.szp(),
                                co.nb(),
                                p,
                            );
                            diff_dict_probe(&what, &buf[..rr]);
                        }
                    }
                }
            }
        }

        // zParams grid (compressionLevel × dictID) and capacity grid
        for &level in &[0i32, 1, 3, 9, 19, 22, -1, -5] {
            for &dict_id in &[0u32, 1, 32767, 32768, 0x7FFF_FFFF, 0xFFFF_FFFF] {
                let co = Corpus::uniform(4, 40, 400, 0x1481);
                let p = ZDICT_cover_params_t {
                    k: 128,
                    d: 8,
                    steps: 4,
                    nbThreads: 1,
                    splitPoint: 1.0,
                    shrinkDict: 0,
                    shrinkDictMaxRegression: 0,
                    zParams: zparams(level, dict_id),
                };
                let what = format!("row148 level={level} dictID={dict_id}");
                let r = diff_train_p(&what, f, CAPS[3], &co, p);
                if !is_err(r) {
                    ok += 1;
                }
            }
        }

        for &cap in &CAPS {
            for class in 0..N_CLASSES {
                for &reg in &[0u32, 1, 5, 50, 100] {
                    let co = Corpus::uniform(class, 48, 300, 0x1482 + class as u64);
                    let p = ZDICT_cover_params_t {
                        k: 64,
                        d: 6,
                        steps: 4,
                        nbThreads: 1,
                        splitPoint: 1.0,
                        shrinkDict: (reg % 2 == 1) as c_uint,
                        shrinkDictMaxRegression: reg,
                        zParams: zparams(3, 0),
                    };
                    let what =
                        format!("row148 cap={cap} class={} reg={reg}", CLASS_NAMES[class]);
                    if !is_err(diff_train_p(&what, f, cap, &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // nbSamples {1,4,32,256}
        for &(nb, size) in &[
            (1usize, 4096usize),
            (2, 2048),
            (4, 1024),
            (8, 512),
            (16, 400),
            (32, 512),
            (64, 256),
            (128, 200),
            (256, 128),
        ] {
            let co = Corpus::uniform(4, nb, size, 0x1483);
            let p = ZDICT_cover_params_t {
                k: 100,
                d: 8,
                steps: 4,
                nbThreads: 1,
                splitPoint: 1.0,
                shrinkDict: 0,
                shrinkDictMaxRegression: 0,
                zParams: zparams(3, 0),
            };
            let what = format!("row148 nb={nb} size={size}");
            if !is_err(diff_train_p(&what, f, CAPS[3], &co, p)) {
                ok += 1;
            }
        }

        // randomized sweep over the whole ZDICT_cover_params_t surface
        {
            let mut rng = Rng::new(0x148_0FF);
            for it in 0..600 {
                let class = rng.below(N_CLASSES);
                let nb = 1 + rng.below(64);
                let sz = 1 + rng.below(900);
                let co = Corpus::uniform(class, nb, sz, 0xA000 + it as u64);
                let cap = [256usize, 300, 1024, 4096, 8192, 30_000][rng.below(6)];
                let p = ZDICT_cover_params_t {
                    k: [1u32, 6, 8, 16, 31, 50, 128, 257, 1024, 2048, 4096][rng.below(11)],
                    d: [6u32, 8][rng.below(2)],
                    steps: rng.below(9) as c_uint,
                    nbThreads: rng.below(2) as c_uint,
                    splitPoint: [0.0f64, 0.1, 0.5, 0.75, 1.0][rng.below(5)],
                    shrinkDict: rng.below(2) as c_uint,
                    shrinkDictMaxRegression: [0u32, 1, 5, 20, 50, 100][rng.below(6)],
                    zParams: zparams(
                        [0i32, 1, 3, 6, 12, 19, 22, -1][rng.below(8)],
                        [0u32, 1, 7, 32768, 0xFFFF_FFFF][rng.below(5)],
                    ),
                };
                let what = format!(
                    "row148 fuzz#{it} class={} nb={nb} sz={sz} cap={cap} k={} d={}",
                    CLASS_NAMES[class], p.k, p.d
                );
                if !is_err(diff_train_p(&what, f, cap, &co, p)) {
                    ok += 1;
                }
            }
        }

        assert!(ok >= 650, "row148 only produced {ok} dictionaries");
    }
}

// =================================================================== row 149

#[test]
fn row149_optimize_cover() {
    unsafe {
        let f = duo::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
        let mut ok = 0usize;

        // steps {0,2,4} × d {0,6,8} × k = 0 (auto).  steps=0 means 40 steps,
        // which is slow, so it gets exactly one (small) corpus.
        for &steps in &[2u32, 4] {
            for &d in &[0u32, 6, 8] {
                for class in [0usize, 3, 4, 6] {
                    let co = Corpus::uniform(class, 24, 400, 0x1490 + class as u64);
                    let p = ZDICT_cover_params_t {
                        k: 0,
                        d,
                        steps,
                        nbThreads: 1,
                        splitPoint: 0.0, // default (1.0 for cover)
                        shrinkDict: 0,
                        shrinkDictMaxRegression: 0,
                        zParams: zparams(3, 0),
                    };
                    let what =
                        format!("row149 steps={steps} d={d} class={}", CLASS_NAMES[class]);
                    if !is_err(diff_train_io(&what, f, CAPS[3], &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // splitPoint / nbThreads / shrinkDict axes
        for &sp in &[0.0f64, 0.25, 0.5, 0.75, 1.0] {
            for &nb_threads in &[0u32, 1] {
                for &shrink in &[0u32, 1] {
                    let co = Corpus::uniform(4, 20, 400, 0x1491);
                    let p = ZDICT_cover_params_t {
                        k: 0,
                        d: 8,
                        steps: 2,
                        nbThreads: nb_threads,
                        splitPoint: sp,
                        shrinkDict: shrink,
                        shrinkDictMaxRegression: shrink * 10,
                        zParams: zparams(3, 0),
                    };
                    let what =
                        format!("row149 splitPoint={sp} nbThreads={nb_threads} shrink={shrink}");
                    if !is_err(diff_train_io(&what, f, CAPS[3], &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // explicit k (single k value => 1 iteration per d), capacity grid
        for &cap in &CAPS {
            let co = Corpus::uniform(5, 20, 400, 0x1492);
            let p = ZDICT_cover_params_t {
                k: 200,
                d: 6,
                steps: 2,
                nbThreads: 1,
                splitPoint: 1.0,
                shrinkDict: 0,
                shrinkDictMaxRegression: 0,
                zParams: zparams(1, 12345),
            };
            let what = format!("row149 k=200 cap={cap}");
            if !is_err(diff_train_io(&what, f, cap, &co, p)) {
                ok += 1;
            }
        }

        // steps = 0 => 40 steps, single small case
        {
            let co = Corpus::uniform(4, 12, 200, 0x1493);
            let p = ZDICT_cover_params_t {
                k: 0,
                d: 8,
                steps: 0,
                nbThreads: 1,
                splitPoint: 1.0,
                shrinkDict: 0,
                shrinkDictMaxRegression: 0,
                zParams: zparams(3, 0),
            };
            if !is_err(diff_train_io("row149 steps=0(default 40)", f, CAPS[2], &co, p)) {
                ok += 1;
            }
        }

        // randomized sweep
        {
            let mut rng = Rng::new(0x149_0FF);
            for it in 0..200 {
                let class = rng.below(N_CLASSES);
                let nb = 1 + rng.below(32);
                let sz = 1 + rng.below(600);
                let co = Corpus::uniform(class, nb, sz, 0xB000 + it as u64);
                let cap = [256usize, 300, 1024, 8192][rng.below(4)];
                let p = ZDICT_cover_params_t {
                    k: [0u32, 50, 128, 300, 1024, 2000][rng.below(6)],
                    d: [0u32, 6, 8][rng.below(3)],
                    steps: 1 + rng.below(4) as c_uint,
                    nbThreads: rng.below(2) as c_uint,
                    splitPoint: [0.0f64, 0.1, 0.5, 0.75, 1.0][rng.below(5)],
                    shrinkDict: rng.below(2) as c_uint,
                    shrinkDictMaxRegression: [0u32, 1, 20, 100][rng.below(4)],
                    zParams: zparams([0i32, 1, 3, 9, 19, -1][rng.below(6)], rng.below(9) as c_uint),
                };
                let what = format!(
                    "row149 fuzz#{it} class={} nb={nb} sz={sz} cap={cap} k={} d={} steps={}",
                    CLASS_NAMES[class], p.k, p.d, p.steps
                );
                if !is_err(diff_train_io(&what, f, cap, &co, p)) {
                    ok += 1;
                }
            }
        }

        assert!(ok >= 120, "row149 only produced {ok} dictionaries");
    }
}

// =================================================================== row 150

#[test]
fn row150_train_fastcover() {
    unsafe {
        let f = duo::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
        let mut ok = 0usize;

        // k × d × f × accel
        for &k in &[16u32, 50, 200, 1024, 2048] {
            for &d in &[6u32, 8] {
                for &ff in &[0u32, 6, 10, 15, 20, 23, 25] {
                    for &accel in &[0u32, 1, 2, 3, 5, 7, 10] {
                        let co = Corpus::uniform(4, 32, 512, 0x1500);
                        let p = ZDICT_fastCover_params_t {
                            k,
                            d,
                            f: ff,
                            steps: 4,
                            nbThreads: 1,
                            splitPoint: 1.0,
                            accel,
                            shrinkDict: 0,
                            shrinkDictMaxRegression: 0,
                            zParams: zparams(3, 0),
                        };
                        let what = format!("row150 k={k} d={d} f={ff} accel={accel}");
                        if !is_err(diff_train_p(&what, f, CAPS[3], &co, p)) {
                            ok += 1;
                        }
                    }
                }
            }
        }

        // steps × splitPoint × shrinkDict × shrinkDictMaxRegression × nbThreads
        for &steps in &[0u32, 1, 2, 4, 8, 16] {
            for &sp in &[0.0f64, 0.25, 0.5, 0.75, 1.0] {
                for &shrink in &[0u32, 1] {
                    for &reg in &[0u32, 1, 5, 50, 100] {
                        let co = Corpus::uniform(6, 40, 300, 0x1501);
                        let p = ZDICT_fastCover_params_t {
                            k: 128,
                            d: 8,
                            f: 20,
                            steps,
                            nbThreads: steps % 2,
                            splitPoint: sp,
                            accel: 1,
                            shrinkDict: shrink,
                            shrinkDictMaxRegression: reg,
                            zParams: zparams(3, 0),
                        };
                        let what =
                            format!("row150 steps={steps} sp={sp} shrink={shrink} reg={reg}");
                        if !is_err(diff_train_p(&what, f, CAPS[3], &co, p)) {
                            ok += 1;
                        }
                    }
                }
            }
        }

        // all content classes × capacity grid, plus zParams
        for class in 0..N_CLASSES {
            for &cap in &CAPS {
                let co = Corpus::uniform(class, 48, 256, 0x1502 + class as u64);
                let p = ZDICT_fastCover_params_t {
                    k: 64,
                    d: 6,
                    f: 18,
                    steps: 4,
                    nbThreads: 0,
                    splitPoint: 1.0,
                    accel: 2,
                    shrinkDict: (class % 2) as u32,
                    shrinkDictMaxRegression: 3,
                    zParams: zparams(if class % 2 == 0 { 0 } else { 12 }, class as u32 * 7),
                };
                let what = format!("row150 class={} cap={cap}", CLASS_NAMES[class]);
                let r = diff_train_p(&what, f, cap, &co, p);
                if !is_err(r) {
                    ok += 1;
                    let mut buf = vec![0xA5u8; cap];
                    let n = (f.0)(
                        buf.as_mut_ptr() as *mut c_void,
                        cap,
                        co.sp(),
                        co.szp(),
                        co.nb(),
                        p,
                    );
                    diff_dict_probe(&what, &buf[..n]);
                }
            }
        }

        // nbSamples ladder + ragged sizes
        for &(nb, size) in &[
            (1usize, 4096usize),
            (2, 2048),
            (4, 1024),
            (8, 512),
            (16, 400),
            (32, 512),
            (64, 256),
            (128, 200),
            (256, 128),
        ] {
            let co = Corpus::uniform(4, nb, size, 0x1503);
            let p = ZDICT_fastCover_params_t {
                k: 100,
                d: 8,
                f: 20,
                steps: 4,
                nbThreads: 1,
                splitPoint: 1.0,
                accel: 1,
                shrinkDict: 0,
                shrinkDictMaxRegression: 0,
                zParams: zparams(3, 0),
            };
            if !is_err(diff_train_p(&format!("row150 nb={nb} size={size}"), f, CAPS[3], &co, p)) {
                ok += 1;
            }
        }
        {
            let sizes: Vec<usize> = (0..48).map(|i| [1usize, 7, 8, 63, 200, 900][i % 6]).collect();
            let co = Corpus::new(4, sizes, 0x1504);
            let p = ZDICT_fastCover_params_t {
                k: 60,
                d: 6,
                f: 16,
                steps: 4,
                nbThreads: 1,
                splitPoint: 1.0,
                accel: 3,
                shrinkDict: 1,
                shrinkDictMaxRegression: 8,
                zParams: zparams(5, 0),
            };
            if !is_err(diff_train_p("row150 ragged", f, CAPS[3], &co, p)) {
                ok += 1;
            }
        }

        // randomized sweep over the whole ZDICT_fastCover_params_t surface
        {
            let mut rng = Rng::new(0x150_0FF);
            for it in 0..800 {
                let class = rng.below(N_CLASSES);
                let nb = 1 + rng.below(80);
                let sz = 1 + rng.below(900);
                let co = Corpus::uniform(class, nb, sz, 0xC000 + it as u64);
                let cap = [256usize, 300, 1024, 4096, 8192, 30_000][rng.below(6)];
                let p = ZDICT_fastCover_params_t {
                    k: [1u32, 6, 8, 16, 31, 50, 128, 257, 1024, 2048, 4096][rng.below(11)],
                    d: [6u32, 8][rng.below(2)],
                    f: [0u32, 6, 8, 12, 15, 17, 20, 23, 25][rng.below(9)],
                    steps: rng.below(9) as c_uint,
                    nbThreads: rng.below(2) as c_uint,
                    splitPoint: [0.0f64, 0.1, 0.5, 0.75, 1.0][rng.below(5)],
                    accel: rng.below(11) as c_uint,
                    shrinkDict: rng.below(2) as c_uint,
                    shrinkDictMaxRegression: [0u32, 1, 5, 20, 50, 100][rng.below(6)],
                    zParams: zparams(
                        [0i32, 1, 3, 6, 12, 19, 22, -1][rng.below(8)],
                        [0u32, 1, 7, 32768, 0xFFFF_FFFF][rng.below(5)],
                    ),
                };
                let what = format!(
                    "row150 fuzz#{it} class={} nb={nb} sz={sz} cap={cap} k={} d={} f={} accel={}",
                    CLASS_NAMES[class], p.k, p.d, p.f, p.accel
                );
                if !is_err(diff_train_p(&what, f, cap, &co, p)) {
                    ok += 1;
                }
            }
        }

        assert!(ok >= 1300, "row150 only produced {ok} dictionaries");
    }
}

// =================================================================== row 151

#[test]
fn row151_optimize_fastcover() {
    unsafe {
        let f = duo::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
        let mut ok = 0usize;

        // f {15,20,23} × accel {1,5,10} × steps {0,2}
        for &ff in &[15u32, 20, 23] {
            for &accel in &[1u32, 2, 5, 7, 10] {
                for &steps in &[0u32, 1, 2, 3] {
                    let co = Corpus::uniform(4, 24, 400, 0x1510);
                    let p = ZDICT_fastCover_params_t {
                        k: 0,
                        d: 8,
                        f: ff,
                        steps,
                        nbThreads: 1,
                        splitPoint: 0.0,
                        accel,
                        shrinkDict: 0,
                        shrinkDictMaxRegression: 0,
                        zParams: zparams(3, 0),
                    };
                    let what = format!("row151 f={ff} accel={accel} steps={steps}");
                    if !is_err(diff_train_io(&what, f, CAPS[3], &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // d {0,6,8} × splitPoint × nbThreads × shrinkDict
        for &d in &[0u32, 6, 8] {
            for &sp in &[0.0f64, 0.1, 0.25, 0.5, 0.75, 1.0] {
                for &nb_threads in &[0u32, 1] {
                    let co = Corpus::uniform(5, 20, 400, 0x1511);
                    let p = ZDICT_fastCover_params_t {
                        k: 0,
                        d,
                        f: 0,
                        steps: 2,
                        nbThreads: nb_threads,
                        splitPoint: sp,
                        accel: 0,
                        shrinkDict: 1,
                        shrinkDictMaxRegression: 5,
                        zParams: zparams(0, 0),
                    };
                    let what = format!("row151 d={d} sp={sp} nbThreads={nb_threads}");
                    if !is_err(diff_train_io(&what, f, CAPS[3], &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // all classes × capacity grid (steps=2 keeps it cheap)
        for class in 0..N_CLASSES {
            for &cap in &[CAPS[0], CAPS[1], CAPS[2], CAPS[3]] {
                let co = Corpus::uniform(class, 20, 300, 0x1512 + class as u64);
                let p = ZDICT_fastCover_params_t {
                    k: 0,
                    d: 8,
                    f: 17,
                    steps: 2,
                    nbThreads: 1,
                    splitPoint: 1.0,
                    accel: 4,
                    shrinkDict: 0,
                    shrinkDictMaxRegression: 0,
                    zParams: zparams(3, class as u32),
                };
                let what = format!("row151 class={} cap={cap}", CLASS_NAMES[class]);
                let r = diff_train_io(&what, f, cap, &co, p);
                if !is_err(r) {
                    ok += 1;
                }
            }
        }

        // explicit k, nbSamples ladder
        for &(nb, size) in &[(1usize, 2048usize), (4, 1024), (32, 256), (256, 64)] {
            let co = Corpus::uniform(4, nb, size, 0x1513);
            let p = ZDICT_fastCover_params_t {
                k: 200,
                d: 6,
                f: 20,
                steps: 2,
                nbThreads: 1,
                splitPoint: 1.0,
                accel: 1,
                shrinkDict: 0,
                shrinkDictMaxRegression: 0,
                zParams: zparams(3, 0),
            };
            if !is_err(diff_train_io(&format!("row151 nb={nb} size={size}"), f, CAPS[3], &co, p)) {
                ok += 1;
            }
        }

        // randomized sweep
        {
            let mut rng = Rng::new(0x151_0FF);
            for it in 0..300 {
                let class = rng.below(N_CLASSES);
                let nb = 1 + rng.below(40);
                let sz = 1 + rng.below(600);
                let co = Corpus::uniform(class, nb, sz, 0xE000 + it as u64);
                let cap = [256usize, 300, 1024, 8192][rng.below(4)];
                let p = ZDICT_fastCover_params_t {
                    k: [0u32, 50, 128, 300, 1024, 2000][rng.below(6)],
                    d: [0u32, 6, 8][rng.below(3)],
                    f: [0u32, 8, 15, 17, 20, 23][rng.below(6)],
                    steps: 1 + rng.below(4) as c_uint,
                    nbThreads: rng.below(2) as c_uint,
                    splitPoint: [0.0f64, 0.1, 0.5, 0.75, 1.0][rng.below(5)],
                    accel: rng.below(11) as c_uint,
                    shrinkDict: rng.below(2) as c_uint,
                    shrinkDictMaxRegression: [0u32, 1, 20, 100][rng.below(4)],
                    zParams: zparams([0i32, 1, 3, 9, 19, -1][rng.below(6)], rng.below(9) as c_uint),
                };
                let what = format!(
                    "row151 fuzz#{it} class={} nb={nb} sz={sz} cap={cap} k={} d={} f={} accel={}",
                    CLASS_NAMES[class], p.k, p.d, p.f, p.accel
                );
                if !is_err(diff_train_io(&what, f, cap, &co, p)) {
                    ok += 1;
                }
            }
        }

        assert!(ok >= 250, "row151 only produced {ok} dictionaries");
    }
}

// =================================================================== row 152

#[test]
fn row152_train_legacy() {
    unsafe {
        let f = duo::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
        let mut ok = 0usize;

        // selectivityLevel 0..12 × content classes
        for sel in 0u32..=12 {
            for class in 0..N_CLASSES {
                for &cap in &[CAPS[1], CAPS[2], CAPS[3]] {
                    let co = Corpus::uniform(class, 32, 512, 0x1520 + class as u64);
                    let p = ZDICT_legacy_params_t {
                        selectivityLevel: sel,
                        zParams: zparams(3, 0),
                    };
                    let what =
                        format!("row152 sel={sel} class={} cap={cap}", CLASS_NAMES[class]);
                    let r = diff_train_p(&what, f, cap, &co, p);
                    if !is_err(r) && r > 0 {
                        ok += 1;
                        let mut buf = vec![0xA5u8; cap];
                        let n = (f.0)(
                            buf.as_mut_ptr() as *mut c_void,
                            cap,
                            co.sp(),
                            co.szp(),
                            co.nb(),
                            p,
                        );
                        diff_dict_probe(&what, &buf[..n]);
                    }
                }
            }
        }

        // capacity grid × nbSamples ladder × zParams
        for &cap in &CAPS {
            for &(nb, size) in &[
                (1usize, 4096usize),
                (2, 2048),
                (4, 1024),
                (8, 512),
                (16, 400),
                (32, 512),
                (64, 256),
                (128, 200),
                (256, 128),
            ] {
                let co = Corpus::uniform(4, nb, size, 0x1521);
                let p = ZDICT_legacy_params_t {
                    selectivityLevel: 9,
                    zParams: zparams(if cap > 1024 { 6 } else { 0 }, (cap % 65536) as u32),
                };
                let what = format!("row152 cap={cap} nb={nb} size={size}");
                if !is_err(diff_train_p(&what, f, cap, &co, p)) {
                    ok += 1;
                }
            }
        }

        // selectivity > 30 (takes the MINRATIO branch) and ragged/huge corpora
        for &sel in &[31u32, 40, 100] {
            let co = Corpus::uniform(4, 32, 512, 0x1522);
            let p = ZDICT_legacy_params_t { selectivityLevel: sel, zParams: zparams(3, 0) };
            if !is_err(diff_train_p(&format!("row152 sel={sel}"), f, CAPS[3], &co, p)) {
                ok += 1;
            }
        }
        {
            let sizes: Vec<usize> = (0..40).map(|i| [0usize, 1, 7, 8, 97, 300, 1000][i % 7]).collect();
            let co = Corpus::new(4, sizes, 0x1523);
            let p = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3, 0) };
            diff_train_p("row152 ragged", f, CAPS[2], &co, p);
        }
        {
            let mut sizes = vec![64 * 1024usize];
            sizes.extend(std::iter::repeat(1000).take(16));
            let co = Corpus::new(5, sizes, 0x1524);
            let p = ZDICT_legacy_params_t { selectivityLevel: 4, zParams: zparams(3, 0) };
            diff_train_p("row152 huge", f, CAPS[4], &co, p);
        }
        // too little content => returns 0 (not an error)
        {
            let co = Corpus::uniform(4, 2, 8, 0x1525);
            let p = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3, 0) };
            let r = diff_train_p("row152 tiny corpus", f, CAPS[3], &co, p);
            assert_eq!(r, 0, "expected 0 (no dictionary) for a 16-byte corpus");
        }

        // randomized sweep
        {
            let mut rng = Rng::new(0x152_0FF);
            for it in 0..600 {
                let class = rng.below(N_CLASSES);
                let nb = 1 + rng.below(80);
                let sizes: Vec<usize> =
                    (0..nb).map(|_| [0usize, 1, 8, 63, 200, 512, 1500][rng.below(7)]).collect();
                let co = Corpus::new(class, sizes, 0xF000 + it as u64);
                let cap = [100usize, 256, 300, 1024, 4096, 8192, 30_000][rng.below(7)];
                let p = ZDICT_legacy_params_t {
                    selectivityLevel: rng.below(13) as c_uint,
                    zParams: zparams(
                        [0i32, 1, 3, 9, 19, -1][rng.below(6)],
                        [0u32, 1, 32768, 0xFFFF_FFFF][rng.below(4)],
                    ),
                };
                let what = format!(
                    "row152 fuzz#{it} class={} nb={nb} total={} cap={cap} sel={}",
                    CLASS_NAMES[class], co.total(), p.selectivityLevel
                );
                let r = diff_train_p(&what, f, cap, &co, p);
                if !is_err(r) && r > 0 {
                    ok += 1;
                }
            }
        }

        assert!(ok >= 330, "row152 only produced {ok} dictionaries");
    }
}

// =================================================================== row 153

#[test]
fn row153_finalize_dictionary() {
    unsafe {
        let f = duo::<FnFinalize>("ZDICT_finalizeDictionary");
        let (ic, ir) = duo::<FnIsError>("ZDICT_isError");
        let mut ok = 0usize;

        #[track_caller]
        unsafe fn run(
            what: &str,
            f: (FnFinalize, FnFinalize),
            iserr: (FnIsError, FnIsError),
            cap: usize,
            content: &[u8],
            co: &Corpus,
            p: ZDICT_params_t,
        ) -> usize {
            let mut dc = vec![0xA5u8; cap.max(1)];
            let mut dr = vec![0xA5u8; cap.max(1)];
            let rc = (f.0)(
                dc.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                content.len(),
                co.sp(),
                co.szp(),
                co.nb(),
                p,
            );
            let rr = (f.1)(
                dr.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                content.len(),
                co.sp(),
                co.szp(),
                co.nb(),
                p,
            );
            eqv(&format!("{what}: return"), rc, rr);
            eqv(&format!("{what}: isError"), (iserr.0)(rc), (iserr.1)(rr));
            eqbuf(&format!("{what}: dstDictBuffer"), &dc, &dr);
            if !is_err(rc) {
                assert!(rc <= cap);
                diff_dict_probe(what, &dc[..rc]);
            }
            rc
        }

        // custom content (all classes) × maxDictSize grid × zParams grid
        for class in 0..N_CLASSES {
            let content = gen_class(class, 3000, 0x1530 + class as u64);
            let co = Corpus::uniform(class, 32, 512, 0x1531 + class as u64);
            for &cap in &[100usize, 256, 1024, 4096, 8192] {
                for &level in &[0i32, 1, 3, 9, 19] {
                    for &dict_id in &[0u32, 1, 32767, 32768, 0xFFFF_FFFF] {
                        let p = zparams(level, dict_id);
                        let what = format!(
                            "row153 class={} cap={cap} level={level} dictID={dict_id}",
                            CLASS_NAMES[class]
                        );
                        // content longer than the capacity is rejected; clamp
                        let cl = content.len().min(cap.saturating_sub(0));
                        let r = run(&what, f, (ic, ir), cap, &content[..cl], &co, p);
                        if !is_err(r) {
                            ok += 1;
                        }
                    }
                }
            }
        }

        // dictContentSize axis (including 0 and >= capacity) with fixed params
        {
            let content = gen_class(4, 20_000, 0x1532);
            let co = Corpus::uniform(4, 64, 400, 0x1533);
            for &clen in &[0usize, 1, 8, 127, 128, 255, 256, 1000, 4096, 20_000] {
                for &cap in &[256usize, 1024, 4096, 32 * 1024] {
                    let p = zparams(3, 0);
                    let what = format!("row153 contentSize={clen} cap={cap}");
                    if !is_err(run(&what, f, (ic, ir), cap, &content[..clen], &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // nbSamples ladder (few samples / uncompressible samples are the
        // documented failure modes; both libraries must agree)
        {
            let content = gen_class(4, 2000, 0x1534);
            for &(nb, size) in &[(1usize, 4096usize), (4, 1024), (32, 512), (256, 128)] {
                for class in [0usize, 3, 4, 7] {
                    let co = Corpus::uniform(class, nb, size, 0x1535);
                    let p = zparams(3, 0);
                    let what = format!("row153 nb={nb} size={size} class={}", CLASS_NAMES[class]);
                    if !is_err(run(&what, f, (ic, ir), 8192, &content, &co, p)) {
                        ok += 1;
                    }
                }
            }
        }

        // one very large content + one very large sample
        {
            let content = gen_class(5, 100_000, 0x1536);
            let mut sizes = vec![128 * 1024usize];
            sizes.extend(std::iter::repeat(500).take(32));
            let co = Corpus::new(5, sizes, 0x1537);
            if !is_err(run("row153 huge", f, (ic, ir), 110 * 1024, &content, &co, zparams(3, 0))) {
                ok += 1;
            }
        }

        // randomized sweep
        {
            let mut rng = Rng::new(0x153_0FF);
            for it in 0..1000 {
                let cclass = rng.below(N_CLASSES);
                let sclass = rng.below(N_CLASSES);
                let clen = [0usize, 1, 8, 127, 128, 255, 256, 700, 3000, 9000][rng.below(10)];
                let content = gen_class(cclass, clen, 0x1_0000 + it as u64);
                let nb = 1 + rng.below(64);
                let sz = 1 + rng.below(700);
                let co = Corpus::uniform(sclass, nb, sz, 0x2_0000 + it as u64);
                let cap = [100usize, 255, 256, 300, 1024, 4096, 16_384][rng.below(7)];
                let p = zparams(
                    [0i32, 1, 3, 6, 9, 12, 19, 22, -1, -5][rng.below(10)],
                    [0u32, 1, 32767, 32768, 0x8000_0000, 0xFFFF_FFFF][rng.below(6)],
                );
                let what = format!(
                    "row153 fuzz#{it} content={}({clen}) samples={} nb={nb} sz={sz} cap={cap}",
                    CLASS_NAMES[cclass], CLASS_NAMES[sclass]
                );
                if !is_err(run(&what, f, (ic, ir), cap, &content, &co, p)) {
                    ok += 1;
                }
            }
        }

        assert!(ok >= 1400, "row153 only produced {ok} dictionaries");
    }
}

#[test]
fn row153_finalize_notification_levels() {
    unsafe {
        let f = duo::<FnFinalize>("ZDICT_finalizeDictionary");
        // Tiny corpora so the (uncompared) stderr chatter stays bounded.  Both
        // libraries receive the same notificationLevel.
        let content = gen_class(4, 600, 0x1538);
        let co = Corpus::uniform(4, 16, 200, 0x1539);
        for level in 0u32..=4 {
            let p = ZDICT_params_t {
                compressionLevel: 3,
                notificationLevel: level,
                dictID: 7,
            };
            let cap = 2048usize;
            let mut dc = vec![0xA5u8; cap];
            let mut dr = vec![0xA5u8; cap];
            let rc = (f.0)(
                dc.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                content.len(),
                co.sp(),
                co.szp(),
                co.nb(),
                p,
            );
            let rr = (f.1)(
                dr.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                content.len(),
                co.sp(),
                co.szp(),
                co.nb(),
                p,
            );
            eqv(&format!("row153 notificationLevel={level}: return"), rc, rr);
            eqbuf(&format!("row153 notificationLevel={level}: dst"), &dc, &dr);
        }

        // ... and the same axis on the legacy + cover trainers.
        let lf = duo::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
        for level in 0u32..=2 {
            let p = ZDICT_legacy_params_t {
                selectivityLevel: 9,
                zParams: ZDICT_params_t {
                    compressionLevel: 3,
                    notificationLevel: level,
                    dictID: 0,
                },
            };
            diff_train_p(&format!("row152 notificationLevel={level}"), lf, 1024, &co, p);
        }
        // NOTE: the COVER / FASTCOVER trainers are deliberately *not* exercised
        // with notificationLevel > 0.  `cover.c` / `fastcover.c` stash it in a
        // process-global `g_displayLevel`, and `ZDICT_optimize*` copies that
        // global back into `parameters->zParams.notificationLevel`.  Since
        // libtest runs the test functions of this binary on parallel threads in
        // one process, a non-zero level here would leak into the other tests'
        // observations.  `zdict.c` (finalizeDictionary / legacy) uses a local
        // variable, so it is safe.
    }
}

// =================================================================== row 154

#[test]
fn row154_dict_helpers() {
    unsafe {
        let (nc, nr) = duo::<FnErrName>("ZDICT_getErrorName");
        let (ic, ir) = duo::<FnIsError>("ZDICT_isError");

        // ZDICT_isError / ZDICT_getErrorName over the whole error-code range
        // plus a scattering of "not an error" sizes.
        for code in 0usize..200 {
            eqv(&format!("row154 isError({code})"), ic(code), ir(code));
            eqv(
                &format!("row154 getErrorName({code})"),
                cstr(nc(code)),
                cstr(nr(code)),
            );
        }
        for i in 0usize..200 {
            let code = usize::MAX - i;
            eqv(&format!("row154 isError(-{i})"), ic(code), ir(code));
            eqv(
                &format!("row154 getErrorName(-{i})"),
                cstr(nc(code)),
                cstr(nr(code)),
            );
        }
        for &code in &[
            255usize,
            256,
            1024,
            usize::MAX / 2,
            usize::MAX - 130,
            usize::MAX - 129,
            usize::MAX - 128,
        ] {
            eqv(&format!("row154 isError({code})"), ic(code), ir(code));
            eqv(
                &format!("row154 getErrorName({code})"),
                cstr(nc(code)),
                cstr(nr(code)),
            );
        }

        // raw buffers: empty, short, magic-only, truncated header, random
        let mut rng = Rng::new(0x154);
        let mut probes: Vec<Vec<u8>> = Vec::new();
        probes.push(vec![]);
        for n in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 16, 100, 256] {
            probes.push(rng.bytes(n));
        }
        for n in [4usize, 5, 6, 7, 8, 9, 16, 64, 256, 1000] {
            let mut v = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
            let tail = rng.bytes(n.saturating_sub(4));
            v.extend_from_slice(&tail);
            v.truncate(n.max(4));
            probes.push(v);
        }
        for class in 0..N_CLASSES {
            probes.push(gen_class(class, 300, 0x1540 + class as u64));
        }
        // a real zstd frame (not a dictionary)
        probes.push(c_compress(&gen_class(4, 4096, 3), 3));
        for (i, p) in probes.iter().enumerate() {
            diff_dict_probe(&format!("row154 raw probe #{i} (len {})", p.len()), p);
            // also probe with a *lying* size, the C code only reads dictSize
            if p.len() >= 8 {
                diff_dict_probe(&format!("row154 raw probe #{i} short size"), &p[..8]);
            }
        }

        // dictionaries actually produced by rows 147..153
        let co = Corpus::uniform(4, 64, 400, 0x1541);
        let cap = 8192usize;

        let (t, _) = duo::<FnTrain>("ZDICT_trainFromBuffer");
        let mut d = vec![0u8; cap];
        let n = t(d.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
        assert!(!is_err(n), "row154 setup: trainFromBuffer failed");
        diff_dict_probe("row154 trainFromBuffer dict", &d[..n]);
        for cut in [0usize, 1, 4, 7, 8, 9, 12, 20, n / 2] {
            diff_dict_probe(&format!("row154 trainFromBuffer dict truncated to {cut}"), &d[..cut]);
        }
        // corrupt the magic / dictID
        for k in 0..8 {
            let mut bad = d[..n].to_vec();
            bad[k] ^= 0xFF;
            diff_dict_probe(&format!("row154 dict with byte {k} flipped"), &bad);
        }

        let (cvr, _) = duo::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let cp = ZDICT_cover_params_t {
            k: 128,
            d: 8,
            steps: 4,
            nbThreads: 1,
            splitPoint: 1.0,
            shrinkDict: 0,
            shrinkDictMaxRegression: 0,
            zParams: zparams(3, 0),
        };
        let mut d2 = vec![0u8; cap];
        let n2 = cvr(d2.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), cp);
        assert!(!is_err(n2), "row154 setup: cover failed");
        diff_dict_probe("row154 cover dict", &d2[..n2]);

        let (lg, _) = duo::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
        let lp = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3, 4242) };
        let mut d3 = vec![0u8; cap];
        let n3 = lg(d3.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), lp);
        assert!(!is_err(n3), "row154 setup: legacy failed");
        diff_dict_probe("row154 legacy dict", &d3[..n3]);

        let (fin, _) = duo::<FnFinalize>("ZDICT_finalizeDictionary");
        for &dict_id in &[0u32, 1, 32767, 32768, 0x8000_0000, 0xFFFF_FFFF] {
            let content = gen_class(4, 2000, 0x1542);
            let mut d4 = vec![0u8; cap];
            let n4 = fin(
                d4.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                content.len(),
                co.sp(),
                co.szp(),
                co.nb(),
                zparams(3, dict_id),
            );
            if !is_err(n4) {
                diff_dict_probe(&format!("row154 finalize dict dictID={dict_id}"), &d4[..n4]);
                let (idc, idr) = duo::<FnGetDictID>("ZDICT_getDictID");
                let a = idc(d4.as_ptr() as *const c_void, n4);
                let b = idr(d4.as_ptr() as *const c_void, n4);
                eqv("row154 finalize getDictID", a, b);
                if dict_id != 0 {
                    eqv("row154 finalize dictID roundtrip", a, dict_id);
                }
            }
        }
    }
}

// =================================================================== row 155

#[test]
fn row155_cover_internals() {
    unsafe {
        // -------- COVER_sum
        let (sc, sr) = duo::<FnCoverSum>("COVER_sum");
        let mut rng = Rng::new(0x155);
        for case in 0..40 {
            let n = rng.below(64);
            let sizes: Vec<usize> = (0..n).map(|_| rng.below(100_000)).collect();
            let p = sizes.as_ptr();
            eqv(
                &format!("row155 COVER_sum case {case} n={n}"),
                sc(p, n as c_uint),
                sr(p, n as c_uint),
            );
            // partial sums / nbSamples smaller than the array
            for k in [0usize, 1, n / 2] {
                if k <= n {
                    eqv(
                        &format!("row155 COVER_sum case {case} prefix {k}"),
                        sc(p, k as c_uint),
                        sr(p, k as c_uint),
                    );
                }
            }
        }
        {
            let big = vec![usize::MAX / 4; 8];
            eqv(
                "row155 COVER_sum overflow-ish",
                sc(big.as_ptr(), 8),
                sr(big.as_ptr(), 8),
            );
        }

        // -------- COVER_computeEpochs
        let (ec, er) = duo::<FnComputeEpochs>("COVER_computeEpochs");
        for &max_dict in &[256u32, 1024, 8192, 110 * 1024, 1 << 20, u32::MAX / 2] {
            // nbDmers == 0 is excluded: the reference C divides by
            // `epochs.size == MIN(k*10, nbDmers) == 0` and traps (SIGFPE).
            // COVER/FASTCOVER never call it with 0 (the corpus-size checks in
            // `*_ctx_init` guarantee at least one dmer).
            for &nb_dmers in &[1u32, 7, 100, 4096, 1 << 20, u32::MAX / 4] {
                for &k in &[1u32, 6, 16, 50, 200, 2048, 65536] {
                    for &passes in &[1u32, 2, 4, 40, 1000] {
                        let a = ec(max_dict, nb_dmers, k, passes);
                        let b = er(max_dict, nb_dmers, k, passes);
                        eqv(
                            &format!(
                                "row155 COVER_computeEpochs({max_dict},{nb_dmers},{k},{passes})"
                            ),
                            a,
                            b,
                        );
                    }
                }
            }
        }

        // -------- COVER_dictSelectionError / IsError / Free
        let (dec, der) = duo::<FnDsError>("COVER_dictSelectionError");
        let (iec, ier) = duo::<FnDsIsError>("COVER_dictSelectionIsError");
        let (dfc, dfr) = duo::<FnDsFree>("COVER_dictSelectionFree");
        for &code in &[
            0usize,
            1,
            100,
            256,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX - 12,
            usize::MAX - 63,
            usize::MAX - 130,
        ] {
            let a = dec(code);
            let b = der(code);
            eqv(&format!("row155 dictSelectionError({code}).dictSize"), a.dictSize, b.dictSize);
            eqv(
                &format!("row155 dictSelectionError({code}).totalCompressedSize"),
                a.totalCompressedSize,
                b.totalCompressedSize,
            );
            eqv(
                &format!("row155 dictSelectionError({code}).dictContent null"),
                a.dictContent.is_null(),
                b.dictContent.is_null(),
            );
            eqv(&format!("row155 dictSelectionIsError({code}) C-struct"), iec(a), ier(a));
            eqv(&format!("row155 dictSelectionIsError({code}) R-struct"), iec(b), ier(b));
            // freeing a NULL dictContent must be a no-op in both
            dfc(a);
            dfr(b);
        }
        // non-error selections, and one with a NULL content but a valid size
        {
            let mut buf = vec![0u8; 32];
            for &(sz, csz) in
                &[(0usize, 0usize), (1, 1), (32, 1000), (32, usize::MAX - 12), (0, usize::MAX)]
            {
                let sel = COVER_dictSelection_t {
                    dictContent: buf.as_mut_ptr(),
                    dictSize: sz,
                    totalCompressedSize: csz,
                };
                eqv(
                    &format!("row155 dictSelectionIsError(non-null,{sz},{csz})"),
                    iec(sel),
                    ier(sel),
                );
                let sel_null = COVER_dictSelection_t {
                    dictContent: std::ptr::null_mut(),
                    dictSize: sz,
                    totalCompressedSize: csz,
                };
                eqv(
                    &format!("row155 dictSelectionIsError(null,{sz},{csz})"),
                    iec(sel_null),
                    ier(sel_null),
                );
                dfc(sel_null);
                dfr(sel_null);
            }
        }

        // -------- COVER_best_init / start / finish / wait / destroy
        let (bic, bir) = duo::<FnBestVoid>("COVER_best_init");
        let (bsc, bsr) = duo::<FnBestVoid>("COVER_best_start");
        let (bwc, bwr) = duo::<FnBestVoid>("COVER_best_wait");
        let (bdc, bdr) = duo::<FnBestVoid>("COVER_best_destroy");
        let (bfc, bfr) = duo::<FnBestFinish>("COVER_best_finish");

        // NULL is explicitly tolerated by every entry point
        bic(std::ptr::null_mut());
        bir(std::ptr::null_mut());
        bsc(std::ptr::null_mut());
        bsr(std::ptr::null_mut());
        bwc(std::ptr::null_mut());
        bwr(std::ptr::null_mut());
        bdc(std::ptr::null_mut());
        bdr(std::ptr::null_mut());
        {
            let sel = COVER_dictSelection_t {
                dictContent: std::ptr::null_mut(),
                dictSize: 0,
                totalCompressedSize: 0,
            };
            bfc(std::ptr::null_mut(), ZDICT_cover_params_t::default(), sel);
            bfr(std::ptr::null_mut(), ZDICT_cover_params_t::default(), sel);
        }

        #[track_caller]
        unsafe fn cmp_best(what: &str, c: &COVER_best_t, r: &COVER_best_t) {
            eqv(&format!("{what}: liveJobs"), c.liveJobs, r.liveJobs);
            eqv(&format!("{what}: dictSize"), c.dictSize, r.dictSize);
            eqv(&format!("{what}: compressedSize"), c.compressedSize, r.compressedSize);
            eqv(&format!("{what}: parameters"), c.parameters, r.parameters);
            eqv(&format!("{what}: dict null"), c.dict.is_null(), r.dict.is_null());
            if !c.dict.is_null() && c.dictSize > 0 {
                let cs = std::slice::from_raw_parts(c.dict as *const u8, c.dictSize);
                let rs = std::slice::from_raw_parts(r.dict as *const u8, r.dictSize);
                eqbuf(&format!("{what}: dict content"), cs, rs);
            }
        }

        let zero = COVER_best_t {
            mutex: 0,
            cond: 0,
            liveJobs: 0,
            dict: std::ptr::null_mut(),
            dictSize: 0,
            parameters: ZDICT_cover_params_t::default(),
            compressedSize: 0,
        };
        let mut bc = zero;
        let mut br = zero;
        bic(&mut bc);
        bir(&mut br);
        cmp_best("row155 best after init", &bc, &br);
        eqv("row155 best init compressedSize", bc.compressedSize, usize::MAX);

        // A script of jobs: improving, then worse, then better with a bigger
        // dictionary (forces the realloc branch), then a NULL-content job.
        let mut content = Rng::new(0x1550).bytes(4096);
        let script: [(usize, usize, u32); 6] = [
            (300, 5000, 1),
            (300, 9000, 2),
            (64, 4000, 3),
            (1024, 3500, 4),
            (2048, 3500, 5),
            (16, 1, 6),
        ];
        for (i, &(dsz, csz, tag)) in script.iter().enumerate() {
            let params = ZDICT_cover_params_t {
                k: tag * 10,
                d: 6 + (tag % 2) * 2,
                steps: tag,
                nbThreads: 1,
                splitPoint: 1.0 / (tag as f64),
                shrinkDict: tag % 2,
                shrinkDictMaxRegression: tag,
                zParams: zparams(tag as c_int, tag),
            };
            bsc(&mut bc);
            bsr(&mut br);
            eqv(&format!("row155 best start #{i} liveJobs"), bc.liveJobs, br.liveJobs);
            let sel = COVER_dictSelection_t {
                dictContent: content.as_mut_ptr(),
                dictSize: dsz,
                totalCompressedSize: csz,
            };
            bfc(&mut bc, params, sel);
            bfr(&mut br, params, sel);
            cmp_best(&format!("row155 best after finish #{i}"), &bc, &br);
            bwc(&mut bc);
            bwr(&mut br);
            cmp_best(&format!("row155 best after wait #{i}"), &bc, &br);
        }
        // a job whose selection carries a NULL dict (best_finish must not copy)
        {
            bsc(&mut bc);
            bsr(&mut br);
            let sel = COVER_dictSelection_t {
                dictContent: std::ptr::null_mut(),
                dictSize: 8,
                totalCompressedSize: 1,
            };
            bfc(&mut bc, ZDICT_cover_params_t::default(), sel);
            bfr(&mut br, ZDICT_cover_params_t::default(), sel);
            cmp_best("row155 best after NULL-content finish", &bc, &br);
        }
        bdc(&mut bc);
        bdr(&mut br);

        // -------- COVER_checkTotalCompressedSize
        let (ctc, ctr) = duo::<FnCheckTotal>("COVER_checkTotalCompressedSize");
        let (fin, _) = duo::<FnFinalize>("ZDICT_finalizeDictionary");
        for class in 0..N_CLASSES {
            let co = Corpus::uniform(class, 24, 400, 0x1551 + class as u64);
            let mut offsets = co.offsets();
            // a real dictionary built by the C library, plus a raw-content one
            let content = gen_class(class, 1500, 0x1552 + class as u64);
            let mut real = vec![0u8; 4096];
            let rn = fin(
                real.as_mut_ptr() as *mut c_void,
                4096,
                content.as_ptr() as *const c_void,
                content.len(),
                co.sp(),
                co.szp(),
                co.nb(),
                zparams(3, 0),
            );
            let dicts: Vec<Vec<u8>> = if is_err(rn) {
                vec![content.clone()]
            } else {
                vec![real[..rn].to_vec(), content.clone()]
            };
            for (di, dict) in dicts.iter().enumerate() {
                let mut dbuf = dict.clone();
                for &sp in &[1.0f64, 0.5] {
                    for &level in &[0i32, 1, 3, 12] {
                        let params = ZDICT_cover_params_t {
                            k: 64,
                            d: 8,
                            steps: 4,
                            nbThreads: 1,
                            splitPoint: sp,
                            shrinkDict: 0,
                            shrinkDictMaxRegression: 0,
                            zParams: zparams(level, 0),
                        };
                        let n_train = (co.sizes.len() as f64 * sp) as usize;
                        let what = format!(
                            "row155 checkTotalCompressedSize class={} dict#{di} sp={sp} level={level}",
                            CLASS_NAMES[class]
                        );
                        let a = ctc(
                            params,
                            co.szp(),
                            co.buf.as_ptr(),
                            offsets.as_mut_ptr(),
                            n_train,
                            co.sizes.len(),
                            dbuf.as_mut_ptr(),
                            dbuf.len(),
                        );
                        let b = ctr(
                            params,
                            co.szp(),
                            co.buf.as_ptr(),
                            offsets.as_mut_ptr(),
                            n_train,
                            co.sizes.len(),
                            dbuf.as_mut_ptr(),
                            dbuf.len(),
                        );
                        eqv(&what, a, b);
                    }
                }
            }
        }

        // -------- COVER_selectDict
        let (sdc, sdr) = duo::<FnSelectDict>("COVER_selectDict");
        let mut selects_ok = 0usize;
        for class in 0..N_CLASSES {
            let co = Corpus::uniform(class, 24, 400, 0x1553 + class as u64);
            let mut offsets = co.offsets();
            for &cap in &[512usize, 2048, 8192] {
                for &shrink in &[0u32, 1] {
                    for &reg in &[0u32, 5, 50] {
                        // mirror the real call site: the content lives at the
                        // END of a `cap`-sized buffer so the shrink loop's
                        // backwards reads stay in bounds.
                        let tail = cap / 3;
                        let mut bc_ = gen_class(class, cap, 0x1554 + cap as u64);
                        let mut br_ = bc_.clone();
                        let params = ZDICT_cover_params_t {
                            k: 64,
                            d: 8,
                            steps: 4,
                            nbThreads: 1,
                            splitPoint: 1.0,
                            shrinkDict: shrink,
                            shrinkDictMaxRegression: reg,
                            zParams: zparams(3, 0),
                        };
                        let what = format!(
                            "row155 selectDict class={} cap={cap} shrink={shrink} reg={reg}",
                            CLASS_NAMES[class]
                        );
                        let a = sdc(
                            bc_.as_mut_ptr().add(tail),
                            cap,
                            cap - tail,
                            co.buf.as_ptr(),
                            co.szp(),
                            co.nb(),
                            co.sizes.len(),
                            co.sizes.len(),
                            params,
                            offsets.as_mut_ptr(),
                            0,
                        );
                        let b = sdr(
                            br_.as_mut_ptr().add(tail),
                            cap,
                            cap - tail,
                            co.buf.as_ptr(),
                            co.szp(),
                            co.nb(),
                            co.sizes.len(),
                            co.sizes.len(),
                            params,
                            offsets.as_mut_ptr(),
                            0,
                        );
                        eqv(&format!("{what}: dictSize"), a.dictSize, b.dictSize);
                        eqv(
                            &format!("{what}: totalCompressedSize"),
                            a.totalCompressedSize,
                            b.totalCompressedSize,
                        );
                        eqv(
                            &format!("{what}: dictContent null"),
                            a.dictContent.is_null(),
                            b.dictContent.is_null(),
                        );
                        let (iec2, ier2) = (iec, ier);
                        eqv(&format!("{what}: isError"), iec2(a), ier2(b));
                        if !a.dictContent.is_null() && a.dictSize > 0 {
                            let cs = std::slice::from_raw_parts(a.dictContent, a.dictSize);
                            let rs = std::slice::from_raw_parts(b.dictContent, b.dictSize);
                            eqbuf(&format!("{what}: dictContent"), cs, rs);
                            selects_ok += 1;
                        }
                        // input buffers must have been left identical too
                        eqbuf(&format!("{what}: source buffer"), &bc_, &br_);
                        dfc(a);
                        dfr(b);
                    }
                }
            }
        }
        assert!(selects_ok >= 130, "row155 COVER_selectDict produced only {selects_ok} dicts");

        // randomized COVER_computeEpochs sweep
        {
            let mut r2 = Rng::new(0x155_0FF);
            for it in 0..20000 {
                let max_dict = 1 + r2.next_u32() % (1 << 22);
                let nb_dmers = 1 + r2.next_u32() % (1 << 24);
                let k = 1 + r2.next_u32() % 65536;
                let passes = 1 + r2.next_u32() % 200;
                let a = ec(max_dict, nb_dmers, k, passes);
                let b = er(max_dict, nb_dmers, k, passes);
                eqv(
                    &format!("row155 computeEpochs fuzz#{it} ({max_dict},{nb_dmers},{k},{passes})"),
                    a,
                    b,
                );
            }
        }

        // COVER_warnOnSmallCorpus is reachable but writes only to stderr and
        // returns nothing; drive it with displayLevel 0 so nothing is printed.
        type FnWarn = unsafe extern "C" fn(usize, usize, c_int);
        let (wc, wr) = duo::<FnWarn>("COVER_warnOnSmallCorpus");
        for &(md, nd) in &[(256usize, 0usize), (256, 2560), (1024, 100), (110 * 1024, 1 << 20)] {
            wc(md, nd, 0);
            wr(md, nd, 0);
        }
    }
}

// =================================================================== row 156

/// Structured byte arrays that stress divsufsort's type-B*/bucket logic.
fn ds_inputs(n: usize, seed: u64) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    for class in 0..N_CLASSES {
        v.push((format!("class={}", CLASS_NAMES[class]), gen_class(class, n, seed)));
    }
    v.push(("ascending".into(), (0..n).map(|i| (i % 256) as u8).collect()));
    v.push(("descending".into(), (0..n).map(|i| (255 - (i % 256)) as u8).collect()));
    v.push(("sawtooth-17".into(), (0..n).map(|i| (i % 17) as u8).collect()));
    v.push(("abab".into(), (0..n).map(|i| if i % 2 == 0 { b'a' } else { b'b' }).collect()));
    v.push(("abcabc".into(), (0..n).map(|i| b'a' + (i % 3) as u8).collect()));
    v.push(("all-0xFF".into(), vec![0xFFu8; n]));
    v.push(("all-0x00".into(), vec![0u8; n]));
    // fibonacci word
    v.push((
        "fibword".into(),
        {
            let mut a = vec![b'a'];
            let mut b = vec![b'a', b'b'];
            while b.len() < n.max(1) {
                let mut c = b.clone();
                c.extend_from_slice(&a);
                a = b;
                b = c;
            }
            b.truncate(n);
            while b.len() < n {
                b.push(b'a');
            }
            b
        },
    ));
    // two-symbol de-Bruijn-ish
    v.push(("bit8".into(), (0..n).map(|i| ((i.wrapping_mul(2654435761usize)) >> 13 & 1) as u8).collect()));
    v.push(("binary-random".into(), {
        let mut r = Rng::new(seed ^ 0xB17);
        (0..n).map(|_| r.byte() & 1).collect()
    }));
    v.push(("dna".into(), {
        let mut r = Rng::new(seed ^ 0xD1A);
        (0..n).map(|_| b"ACGT"[r.below(4)]).collect()
    }));
    for (_, b) in v.iter_mut() {
        b.truncate(n);
        while b.len() < n {
            b.push(0);
        }
    }
    v
}

#[test]
fn row156_divsufsort() {
    unsafe {
        let (dc, dr) = duo::<FnDivsufsort>("divsufsort");
        for &n in &[0usize, 1, 2, 3, 255, 4096, 65536] {
          for seed_i in 0..4u64 {
            for (name, t) in ds_inputs(n, 0x1560 + n as u64 + seed_i * 7919) {
                assert_eq!(t.len(), n);
                // keep the allocation non-NULL even for n == 0 (the C code
                // rejects NULL pointers before looking at n)
                let mut tt = t.clone();
                tt.push(0);
                for &openmp in &[0i32, 1] {
                    let mut sa_c = vec![-12345i32; n + 1];
                    let mut sa_r = vec![-12345i32; n + 1];
                    let a = dc(tt.as_ptr(), sa_c.as_mut_ptr(), n as c_int, openmp);
                    let b = dr(tt.as_ptr(), sa_r.as_mut_ptr(), n as c_int, openmp);
                    let what = format!("row156 divsufsort n={n} {name} openMP={openmp}");
                    eqv(&format!("{what}: return"), a, b);
                    eqi32(&format!("{what}: SA"), &sa_c, &sa_r);
                    if a == 0 && n > 0 {
                        // sanity: SA must be a permutation of 0..n
                        let mut seen = vec![false; n];
                        for i in 0..n {
                            let s = sa_c[i];
                            assert!(
                                s >= 0 && (s as usize) < n,
                                "{what}: SA[{i}]={s} out of range"
                            );
                            assert!(!seen[s as usize], "{what}: SA[{i}]={s} duplicated");
                            seen[s as usize] = true;
                        }
                        assert_eq!(sa_c[n], -12345, "{what}: wrote past SA[n-1]");
                    }
                }
            }
          }
        }
        // NULL arguments / negative n
        {
            let t = vec![1u8, 2, 3, 4];
            let mut sa = vec![0i32; 4];
            eqv(
                "row156 divsufsort NULL T",
                dc(std::ptr::null(), sa.as_mut_ptr(), 4, 0),
                dr(std::ptr::null(), sa.as_mut_ptr(), 4, 0),
            );
            eqv(
                "row156 divsufsort NULL SA",
                dc(t.as_ptr(), std::ptr::null_mut(), 4, 0),
                dr(t.as_ptr(), std::ptr::null_mut(), 4, 0),
            );
            eqv(
                "row156 divsufsort n<0",
                dc(t.as_ptr(), sa.as_mut_ptr(), -1, 0),
                dr(t.as_ptr(), sa.as_mut_ptr(), -1, 0),
            );
        }
    }
}

#[test]
fn row156_divbwt() {
    unsafe {
        let (dc, dr) = duo::<FnDivbwt>("divbwt");
        for &n in &[0usize, 1, 2, 3, 255, 4096, 65536] {
          for seed_i in 0..2u64 {
            for (name, t) in ds_inputs(n, 0x1561 + n as u64 + seed_i * 6151) {
                let mut tt = t.clone();
                tt.push(0);
                // (provide scratch A?, provide num_indexes/indexes?)
                for &(with_a, with_idx) in
                    &[(false, false), (true, false), (true, true), (false, true)]
                {
                    for &openmp in &[0i32, 1] {
                        let mut uc = vec![0xA5u8; n + 1];
                        let mut ur = vec![0xA5u8; n + 1];
                        let mut ac = vec![-12345i32; n + 2];
                        let mut ar = vec![-12345i32; n + 2];
                        let mut nic = 0xEEu8;
                        let mut nir = 0xEEu8;
                        let mut idc = vec![-12345i32; 4096];
                        let mut idr = vec![-12345i32; 4096];
                        let ap = if with_a { ac.as_mut_ptr() } else { std::ptr::null_mut() };
                        let arp = if with_a { ar.as_mut_ptr() } else { std::ptr::null_mut() };
                        let (nip, idp) = if with_idx {
                            (&mut nic as *mut u8, idc.as_mut_ptr())
                        } else {
                            (std::ptr::null_mut(), std::ptr::null_mut())
                        };
                        let (nirp, idrp) = if with_idx {
                            (&mut nir as *mut u8, idr.as_mut_ptr())
                        } else {
                            (std::ptr::null_mut(), std::ptr::null_mut())
                        };
                        let a = dc(tt.as_ptr(), uc.as_mut_ptr(), ap, n as c_int, nip, idp, openmp);
                        let b = dr(tt.as_ptr(), ur.as_mut_ptr(), arp, n as c_int, nirp, idrp, openmp);
                        let what = format!(
                            "row156 divbwt n={n} {name} A={with_a} idx={with_idx} openMP={openmp}"
                        );
                        eqv(&format!("{what}: return"), a, b);
                        eqbuf(&format!("{what}: U"), &uc, &ur);
                        if with_a {
                            eqi32(&format!("{what}: A"), &ac, &ar);
                        }
                        if with_idx {
                            eqv(&format!("{what}: num_indexes"), nic, nir);
                            eqi32(&format!("{what}: indexes"), &idc, &idr);
                        }
                        assert_eq!(uc[n], 0xA5, "{what}: wrote past U[n-1]");
                    }
                }
            }
          }
        }
        // NULL arguments / negative n
        {
            let t = vec![1u8, 2, 3, 4];
            let mut u = vec![0u8; 4];
            eqv(
                "row156 divbwt NULL T",
                dc(std::ptr::null(), u.as_mut_ptr(), std::ptr::null_mut(), 4, std::ptr::null_mut(), std::ptr::null_mut(), 0),
                dr(std::ptr::null(), u.as_mut_ptr(), std::ptr::null_mut(), 4, std::ptr::null_mut(), std::ptr::null_mut(), 0),
            );
            eqv(
                "row156 divbwt NULL U",
                dc(t.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), 4, std::ptr::null_mut(), std::ptr::null_mut(), 0),
                dr(t.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), 4, std::ptr::null_mut(), std::ptr::null_mut(), 0),
            );
            eqv(
                "row156 divbwt n<0",
                dc(t.as_ptr(), u.as_mut_ptr(), std::ptr::null_mut(), -1, std::ptr::null_mut(), std::ptr::null_mut(), 0),
                dr(t.as_ptr(), u.as_mut_ptr(), std::ptr::null_mut(), -1, std::ptr::null_mut(), std::ptr::null_mut(), 0),
            );
        }
    }
}

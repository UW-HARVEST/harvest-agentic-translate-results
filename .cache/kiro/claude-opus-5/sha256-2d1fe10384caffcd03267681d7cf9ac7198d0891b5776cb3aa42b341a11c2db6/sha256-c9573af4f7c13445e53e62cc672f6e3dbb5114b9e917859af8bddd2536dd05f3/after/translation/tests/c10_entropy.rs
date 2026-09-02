//! Phase C: error-path differential tests for the low-level entropy primitives
//! (FSE / HUF / HIST). Feeds invalid sizes, undersized destinations, oversized
//! symbol values / tableLogs, undersized workspaces, truncated / corrupted
//! headers, malformed normalized counters, and thousands of random garbage
//! buffers to every decode entry point — asserting C and Rust agree on every
//! return value / error code (and byte output where anything is produced).
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

// ---------------------------------------------------------------- signatures

type FnIsErr = unsafe extern "C" fn(size_t) -> c_uint;
type FnGetErrName = unsafe extern "C" fn(size_t) -> *const std::os::raw::c_char;

type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnHistCountWksp =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t, *mut c_void, size_t) -> size_t;

type FnNormalize =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, size_t, c_uint, c_uint) -> size_t;
type FnWriteNCount =
    unsafe extern "C" fn(*mut c_void, size_t, *const i16, c_uint, c_uint) -> size_t;
type FnReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t, c_int) -> size_t;
type FnBuildCTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, c_uint, *mut c_void, size_t, c_int,
) -> size_t;

type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut u64, *const c_uint, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnHufWriteCTableWksp =
    unsafe extern "C" fn(*mut c_void, size_t, *const u64, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8, size_t, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, size_t,
) -> size_t;
type FnHufReadStatsWksp = unsafe extern "C" fn(
    *mut u8, size_t, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, size_t,
    *mut c_void, size_t, c_int,
) -> size_t;
type FnHufReadCTable =
    unsafe extern "C" fn(*mut u64, *mut c_uint, *const c_void, size_t, *mut c_uint) -> size_t;
type FnHufReadDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const c_void, size_t, *mut c_void, size_t, c_int) -> size_t;
type FnHufDecompressUsingDTable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const c_uint, c_int) -> size_t;
type FnHufDecompressDCtxWksp = unsafe extern "C" fn(
    *mut c_uint, *mut c_void, size_t, *const c_void, size_t, *mut c_void, size_t, c_int,
) -> size_t;

// ------------------------------------------------------------ error helpers

struct EntErr {
    c_is: (Symbol<FnIsErr>, Symbol<FnIsErr>),
    c_name: Option<(Symbol<FnGetErrName>, Symbol<FnGetErrName>)>,
}
type Symbol<T> = libloading::Symbol<'static, T>;

impl EntErr {
    unsafe fn new(is_err: &str, get_name: Option<&str>) -> Self {
        EntErr {
            c_is: both::<FnIsErr>(is_err),
            c_name: get_name.map(|n| both::<FnGetErrName>(n)),
        }
    }
    unsafe fn c_err(&self, r: size_t) -> bool {
        (self.c_is.0)(r) != 0
    }
    #[track_caller]
    unsafe fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let ce = (self.c_is.0)(cr) != 0;
        let re = (self.c_is.1)(rr) != 0;
        assert_eq!(ce, re, "{ctx}: isError mismatch C={ce}({cr:#x}) RS={re}({rr:#x})");
        if ce {
            if let Some((cn, rn)) = &self.c_name {
                let cs = cstr(cn(cr));
                let rs = cstr(rn(rr));
                assert_eq!(cs, rs, "{ctx}: error name mismatch C={cs:?} RS={rs:?}");
            }
        } else {
            assert_eq!(cr, rr, "{ctx}: value mismatch C={cr:#x} RS={rr:#x}");
        }
    }
}

fn fse_err() -> EntErr {
    unsafe { EntErr::new("FSE_isError", Some("FSE_getErrorName")) }
}
fn huf_err() -> EntErr {
    unsafe { EntErr::new("HUF_isError", Some("HUF_getErrorName")) }
}
fn hist_err() -> EntErr {
    unsafe { EntErr::new("HIST_isError", None) }
}

const HUF_TABLELOG_MAX: c_uint = 12;
const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = (4 * 256) + 192;
const HUF_CTABLE_SIZE_ST: usize = 257;

// ---------------------------------------------------- HIST error / edge cases

#[test]
fn hist_error_and_edge() {
    unsafe {
        let e = hist_err();
        let (cc, rc) = both::<FnHistCount>("HIST_count");
        let (ccw, rcw) = both::<FnHistCountWksp>("HIST_count_wksp");
        let mut rng = Rng::new(0xC0_0001);

        // srcSize 0 and 1, and maxSymbolValue above 255.
        for &ssz in &[0usize, 1, 2, 100] {
            let src = gen(Shape::Random, ssz, &mut rng);
            for &msv in &[0u32, 1, 255, 256, 300, 1000, u32::MAX] {
                let mut cnt_c = vec![0u32; 512];
                let mut cnt_r = vec![0u32; 512];
                let mut m1 = msv;
                let mut m2 = msv;
                let a = cc(cnt_c.as_mut_ptr(), &mut m1, src.as_ptr() as *const c_void, src.len());
                let b = rc(cnt_r.as_mut_ptr(), &mut m2, src.as_ptr() as *const c_void, src.len());
                let ctx = format!("HIST_count ssz={ssz} msv={msv}");
                e.eq(&ctx, a, b);
                assert_eq!(m1, m2, "{ctx}: msv out");

                // workspace too small (HIST_WKSP_SIZE == 1024 U32).
                for wsz_u32 in [0usize, 1, 100, 1023] {
                    let mut cnt_c = vec![0u32; 512];
                    let mut cnt_r = vec![0u32; 512];
                    let mut m1 = msv;
                    let mut m2 = msv;
                    let mut wc = vec![0u32; wsz_u32.max(1)];
                    let mut wr = vec![0u32; wsz_u32.max(1)];
                    let a = ccw(cnt_c.as_mut_ptr(), &mut m1, src.as_ptr() as *const c_void, src.len(), wc.as_mut_ptr() as *mut c_void, wsz_u32 * 4);
                    let b = rcw(cnt_r.as_mut_ptr(), &mut m2, src.as_ptr() as *const c_void, src.len(), wr.as_mut_ptr() as *mut c_void, wsz_u32 * 4);
                    e.eq(&format!("HIST_count_wksp ssz={ssz} msv={msv} wsz={wsz_u32}"), a, b);
                }
            }
        }
    }
}

// -------------------------------------------------- FSE normalize/write errors

#[test]
fn fse_normalize_write_errors() {
    unsafe {
        let e = fse_err();
        let (cn, rn) = both::<FnNormalize>("FSE_normalizeCount");
        let (cw, rw) = both::<FnWriteNCount>("FSE_writeNCount");

        // A well-formed count over a couple of symbols. Sized to 2048 so that a
        // read of count[0..=maxSymbolValue] cannot over-read when we probe
        // maxSymbolValue values above 255.
        let mut count = vec![0u32; 2048];
        count[0] = 50;
        count[1] = 30;
        count[2] = 20;
        let msv = 2u32;
        let total = 100usize;

        // tableLog above max (12) and below min (5), plus msv above 255.
        for tl in [0u32, 1, 4, 5, 12, 13, 14, 15, 16, 31, 100] {
            for m in [msv, 255, 256, 1000] {
                // normalizedCounter must have (maxSymbolValue+1) cells; size for
                // the largest `m` swept so a (valid or partial) write cannot
                // overflow the buffer.
                let mut nc = vec![0i16; 2048];
                let mut nr = vec![0i16; 2048];
                let a = cn(nc.as_mut_ptr(), tl, count.as_ptr(), total, m, 1);
                let b = rn(nr.as_mut_ptr(), tl, count.as_ptr(), total, m, 1);
                let ctx = format!("FSE_normalizeCount tl={tl} msv={m}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_eq!(&nc[..=m.min(255) as usize], &nr[..=m.min(255) as usize], "{ctx}");
                }
            }
        }
        // srcSize == 1/2 with an inconsistent (larger) count triggers the
        // maxSymbolValue / normalization error paths. srcSize==0 is skipped: it
        // is an unsupported precondition that makes BOTH libraries divide by
        // zero (FSE_normalizeCount divides by srcSize).
        for total in [1usize, 2, 3] {
            let mut nc = vec![0i16; 2048];
            let mut nr = vec![0i16; 2048];
            let a = cn(nc.as_mut_ptr(), 6, count.as_ptr(), total, msv, 1);
            let b = rn(nr.as_mut_ptr(), 6, count.as_ptr(), total, msv, 1);
            e.eq(&format!("FSE_normalizeCount total={total}"), a, b);
        }

        // writeNCount: destination too small + a normalized table that does not
        // sum to 1<<tableLog.
        let mut norm = vec![0i16; 256];
        let tl = 6u32;
        let r = cn(norm.as_mut_ptr(), tl, count.as_ptr(), total, msv, 1);
        assert!(!e.c_err(r));
        for cap in [0usize, 1, 2, 3, 4, 5, 8] {
            let mut oc = vec![0u8; cap.max(1)];
            let mut or = vec![0u8; cap.max(1)];
            let a = cw(oc.as_mut_ptr() as *mut c_void, cap, norm.as_ptr(), msv, tl);
            let b = rw(or.as_mut_ptr() as *mut c_void, cap, norm.as_ptr(), msv, tl);
            let ctx = format!("FSE_writeNCount cap={cap}");
            e.eq(&ctx, a, b);
            if !e.c_err(a) {
                assert_bytes_eq(&ctx, &oc[..a], &or[..b]);
            }
        }
        // Bad tableLog to writeNCount.
        for badtl in [0u32, 1, 4, 13, 16, 100] {
            let a = cw(vec![0u8; 512].as_mut_ptr() as *mut c_void, 512, norm.as_ptr(), msv, badtl);
            let b = rw(vec![0u8; 512].as_mut_ptr() as *mut c_void, 512, norm.as_ptr(), msv, badtl);
            e.eq(&format!("FSE_writeNCount badtl={badtl}"), a, b);
        }
    }
}

// --------------------------------------- FSE build table workspace-too-small

#[test]
fn fse_build_table_workspace_too_small() {
    unsafe {
        let e = fse_err();
        let (cn, _) = both::<FnNormalize>("FSE_normalizeCount");
        let (cbc, rbc) = both::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
        let (cbd, rbd) = both::<FnBuildDTableWksp>("FSE_buildDTable_wksp");

        let mut count = vec![0u32; 256];
        count[0] = 40;
        count[1] = 35;
        count[2] = 25;
        let msv = 2u32;
        let tl = 8u32;
        let mut norm = vec![0i16; 256];
        let r = cn(norm.as_mut_ptr(), tl, count.as_ptr(), 100, msv, 1);
        assert!(!e.c_err(r));

        // Undersized C/D-table workspaces (correct requirement is large).
        for wsz in [0usize, 4, 16, 64, 128] {
            let mut ctc = vec![0u32; 4096];
            let mut ctr = vec![0u32; 4096];
            let mut wc = vec![0u32; (wsz / 4).max(1)];
            let mut wr = vec![0u32; (wsz / 4).max(1)];
            let a = cbc(ctc.as_mut_ptr(), norm.as_ptr(), msv, tl, wc.as_mut_ptr() as *mut c_void, wsz);
            let b = rbc(ctr.as_mut_ptr(), norm.as_ptr(), msv, tl, wr.as_mut_ptr() as *mut c_void, wsz);
            e.eq(&format!("FSE_buildCTable_wksp wsz={wsz}"), a, b);

            let mut dtc = vec![0u32; 4096];
            let mut dtr = vec![0u32; 4096];
            let mut wc = vec![0u32; (wsz / 4).max(1)];
            let mut wr = vec![0u32; (wsz / 4).max(1)];
            let a = cbd(dtc.as_mut_ptr(), norm.as_ptr(), msv, tl, wc.as_mut_ptr() as *mut c_void, wsz);
            let b = rbd(dtr.as_mut_ptr(), norm.as_ptr(), msv, tl, wr.as_mut_ptr() as *mut c_void, wsz);
            e.eq(&format!("FSE_buildDTable_wksp wsz={wsz}"), a, b);
        }

        // tableLog above max fed to build.
        for badtl in [13u32, 15, 16, 100] {
            let mut ctc = vec![0u32; 1 << 18];
            let mut ctr = vec![0u32; 1 << 18];
            let mut wc = vec![0u32; 1 << 16];
            let mut wr = vec![0u32; 1 << 16];
            let a = cbc(ctc.as_mut_ptr(), norm.as_ptr(), msv, badtl, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
            let b = rbc(ctr.as_mut_ptr(), norm.as_ptr(), msv, badtl, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
            e.eq(&format!("FSE_buildCTable_wksp badtl={badtl}"), a, b);
        }
    }
}

// ------------------------------------- FSE_readNCount truncated / corrupted

#[test]
fn fse_read_ncount_corrupted() {
    unsafe {
        let e = fse_err();
        let (cn, _) = both::<FnNormalize>("FSE_normalizeCount");
        let (cw, _) = both::<FnWriteNCount>("FSE_writeNCount");
        let (crd, rrd) = both::<FnReadNCount>("FSE_readNCount");
        let (crb, rrb) = both::<FnReadNCountBmi2>("FSE_readNCount_bmi2");

        // Produce a valid header, then feed truncations & corruptions.
        let mut count = vec![0u32; 256];
        count[0] = 40;
        count[1] = 35;
        count[2] = 25;
        let msv = 2u32;
        let tl = 7u32;
        let mut norm = vec![0i16; 256];
        cn(norm.as_mut_ptr(), tl, count.as_ptr(), 100, msv, 1);
        let mut header = vec![0u8; 512];
        let hn = cw(header.as_mut_ptr() as *mut c_void, header.len(), norm.as_ptr(), msv, tl);
        assert!(!e.c_err(hn));
        header.truncate(hn);

        let mut rng = Rng::new(0xC0_0002);
        // Truncations (including 0/1/2/3 bytes) and single-byte corruptions.
        let mut cases: Vec<Vec<u8>> = Vec::new();
        for cut in 0..=header.len() {
            cases.push(header[..cut].to_vec());
        }
        for _ in 0..500 {
            let mut h = header.clone();
            if !h.is_empty() {
                let i = rng.below(h.len());
                h[i] ^= rng.byte();
            }
            cases.push(h);
        }
        // Pure random short buffers.
        for _ in 0..500 {
            let n = rng.below(8);
            cases.push((0..n).map(|_| rng.byte()).collect());
        }

        for (i, buf) in cases.iter().enumerate() {
            let p = buf.as_ptr() as *const c_void;
            let mut n1 = vec![0i16; 256];
            let mut n2 = vec![0i16; 256];
            let (mut m1, mut m2) = (255u32, 255u32);
            let (mut t1, mut t2) = (0u32, 0u32);
            let a = crd(n1.as_mut_ptr(), &mut m1, &mut t1, p, buf.len());
            let b = rrd(n2.as_mut_ptr(), &mut m2, &mut t2, p, buf.len());
            let ctx = format!("FSE_readNCount corrupt #{i} len={}", buf.len());
            e.eq(&ctx, a, b);
            if !e.c_err(a) {
                assert_eq!(m1, m2, "{ctx}: msv");
                assert_eq!(t1, t2, "{ctx}: tableLog");
                assert_eq!(&n1[..=m1.min(255) as usize], &n2[..=m2.min(255) as usize], "{ctx}: norm");
            }
            for bmi2 in [0i32, 1] {
                let mut n1 = vec![0i16; 256];
                let mut n2 = vec![0i16; 256];
                let (mut m1, mut m2) = (255u32, 255u32);
                let (mut t1, mut t2) = (0u32, 0u32);
                let a = crb(n1.as_mut_ptr(), &mut m1, &mut t1, p, buf.len(), bmi2);
                let b = rrb(n2.as_mut_ptr(), &mut m2, &mut t2, p, buf.len(), bmi2);
                e.eq(&format!("{ctx} bmi2={bmi2}"), a, b);
            }
        }
    }
}

// ---------------------------- FSE_decompress_wksp on garbage / bad workspace

#[test]
fn fse_decompress_wksp_errors() {
    unsafe {
        let e = fse_err();
        let (cdw, rdw) = both::<FnDecompressWkspBmi2>("FSE_decompress_wksp_bmi2");
        let mut rng = Rng::new(0xC0_0003);

        // 2000+ random garbage buffers fed to the decode entry point.
        for i in 0..2200 {
            let n = rng.below(64);
            let buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let maxlog = rng.range(0, 15) as c_uint;
            let dcap = rng.below(256);
            let mut d1 = vec![0u8; dcap.max(1)];
            let mut d2 = vec![0u8; dcap.max(1)];
            // workspace at/below/above requirement (randomized)
            let wsz_u32 = rng.below(2048);
            let mut wc = vec![0u32; wsz_u32.max(1)];
            let mut wr = vec![0u32; wsz_u32.max(1)];
            for bmi2 in [0i32, 1] {
                let a = cdw(d1.as_mut_ptr() as *mut c_void, dcap, buf.as_ptr() as *const c_void, buf.len(), maxlog, wc.as_mut_ptr() as *mut c_void, wsz_u32 * 4, bmi2);
                let b = rdw(d2.as_mut_ptr() as *mut c_void, dcap, buf.as_ptr() as *const c_void, buf.len(), maxlog, wr.as_mut_ptr() as *mut c_void, wsz_u32 * 4, bmi2);
                let ctx = format!("FSE_decompress_wksp garbage #{i} bmi2={bmi2}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_bytes_eq(&ctx, &d1[..a], &d2[..b]);
                }
            }
        }
    }
}

// ------------------------------------------------- HUF build/write errors

#[test]
fn huf_build_write_errors() {
    unsafe {
        let e = huf_err();
        let (cbc, rbc) = both::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp");
        let (cwc, rwc) = both::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp");

        let mut count = vec![0u32; 256];
        count[0] = 40;
        count[1] = 35;
        count[2] = 25;

        // maxSymbolValue above 255 (-> maxSymbolValue_tooLarge) and huffLog
        // above the max (accepted; tree just fits). huffLog values BELOW the
        // minimum depth needed for the symbol set are an unsupported precondition
        // that segfaults BOTH libraries, so they are not swept here. huffLog==0
        // is remapped to HUF_TABLELOG_DEFAULT internally and is safe.
        for msv in [2u32, 255, 256, 300, u32::MAX] {
            for hl in [0u32, 11, 12, 13, 16, 100] {
                let mut ctc = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                let mut ctr = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                let mut wc = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                let mut wr = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                let a = cbc(ctc.as_mut_ptr(), count.as_ptr(), msv, hl, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                let b = rbc(ctr.as_mut_ptr(), count.as_ptr(), msv, hl, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                let ctx = format!("HUF_buildCTable_wksp msv={msv} hl={hl}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_eq!(ctc, ctr, "{ctx}: CTable");
                }
            }
        }

        // buildCTable_wksp with workspace too small.
        for wsz in [0usize, 16, 64, 256, 512] {
            let mut ctc = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
            let mut ctr = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
            let mut wc = vec![0u32; (wsz / 4).max(1)];
            let mut wr = vec![0u32; (wsz / 4).max(1)];
            let a = cbc(ctc.as_mut_ptr(), count.as_ptr(), 2, HUF_TABLELOG_MAX, wc.as_mut_ptr() as *mut c_void, wsz);
            let b = rbc(ctr.as_mut_ptr(), count.as_ptr(), 2, HUF_TABLELOG_MAX, wr.as_mut_ptr() as *mut c_void, wsz);
            e.eq(&format!("HUF_buildCTable_wksp wsz={wsz}"), a, b);
        }

        // Build a valid CTable then feed writeCTable_wksp small dst + small wksp.
        let mut ctable = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
        let mut wksp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
        let used = cbc(ctable.as_mut_ptr(), count.as_ptr(), 2, HUF_TABLELOG_MAX, wksp.as_mut_ptr() as *mut c_void, wksp.len() * 4);
        if !e.c_err(used) {
            let ul = used as c_uint;
            for cap in [0usize, 1, 2, 4, 8, 16, 32] {
                let mut oc = vec![0u8; cap.max(1)];
                let mut or = vec![0u8; cap.max(1)];
                let mut wc = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                let mut wr = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                let a = cwc(oc.as_mut_ptr() as *mut c_void, cap, ctable.as_ptr(), 2, ul, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                let b = rwc(or.as_mut_ptr() as *mut c_void, cap, ctable.as_ptr(), 2, ul, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                let ctx = format!("HUF_writeCTable_wksp cap={cap}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_bytes_eq(&ctx, &oc[..a], &or[..b]);
                }
            }
            // writeCTable_wksp workspace too small.
            for wsz in [0usize, 16, 64, 256] {
                let mut oc = vec![0u8; 512];
                let mut or = vec![0u8; 512];
                let mut wc = vec![0u32; (wsz / 4).max(1)];
                let mut wr = vec![0u32; (wsz / 4).max(1)];
                let a = cwc(oc.as_mut_ptr() as *mut c_void, 512, ctable.as_ptr(), 2, ul, wc.as_mut_ptr() as *mut c_void, wsz);
                let b = rwc(or.as_mut_ptr() as *mut c_void, 512, ctable.as_ptr(), 2, ul, wr.as_mut_ptr() as *mut c_void, wsz);
                e.eq(&format!("HUF_writeCTable_wksp wsz={wsz}"), a, b);
            }
        }
    }
}

// ------------------------------------- HUF_readStats truncated / corrupted

#[test]
fn huf_read_stats_corrupted() {
    unsafe {
        let e = huf_err();
        let (cbc, _) = both::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp");
        let (cwc, _) = both::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp");
        let (crs, rrs) = both::<FnHufReadStats>("HUF_readStats");
        let (crsw, rrsw) = both::<FnHufReadStatsWksp>("HUF_readStats_wksp");
        let (crc, rrc) = both::<FnHufReadCTable>("HUF_readCTable");

        // Build a valid huffman header.
        let mut count = vec![0u32; 256];
        for i in 0..20 {
            count[i] = (40 - i as u32) * 3 + 1;
        }
        let msv = 19u32;
        let mut ctable = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
        let mut wksp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
        let used = cbc(ctable.as_mut_ptr(), count.as_ptr(), msv, HUF_TABLELOG_MAX, wksp.as_mut_ptr() as *mut c_void, wksp.len() * 4);
        assert!(!e.c_err(used));
        let mut header = vec![0u8; 512];
        let mut wtmp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
        let hn = cwc(header.as_mut_ptr() as *mut c_void, header.len(), ctable.as_ptr(), msv, used as c_uint, wtmp.as_mut_ptr() as *mut c_void, wtmp.len() * 4);
        assert!(!e.c_err(hn));
        header.truncate(hn);

        let mut rng = Rng::new(0xC0_0004);
        let mut cases: Vec<Vec<u8>> = Vec::new();
        for cut in 0..=header.len() {
            cases.push(header[..cut].to_vec());
        }
        for _ in 0..800 {
            let mut h = header.clone();
            if !h.is_empty() {
                let i = rng.below(h.len());
                h[i] ^= rng.byte();
            }
            cases.push(h);
        }
        for _ in 0..800 {
            let n = rng.below(12);
            cases.push((0..n).map(|_| rng.byte()).collect());
        }

        for (i, buf) in cases.iter().enumerate() {
            let p = buf.as_ptr() as *const c_void;
            // readStats: also exercise a too-small huffWeight destination.
            for hw_sz in [256usize, 8, 1] {
                let mut hw1 = vec![0u8; hw_sz.max(1)];
                let mut hw2 = vec![0u8; hw_sz.max(1)];
                let mut rk1 = vec![0u32; 16];
                let mut rk2 = vec![0u32; 16];
                let (mut ns1, mut ns2) = (0u32, 0u32);
                let (mut tl1, mut tl2) = (0u32, 0u32);
                let a = crs(hw1.as_mut_ptr(), hw_sz, rk1.as_mut_ptr(), &mut ns1, &mut tl1, p, buf.len());
                let b = rrs(hw2.as_mut_ptr(), hw_sz, rk2.as_mut_ptr(), &mut ns2, &mut tl2, p, buf.len());
                let ctx = format!("HUF_readStats corrupt #{i} len={} hw={hw_sz}", buf.len());
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_eq!(hw1, hw2, "{ctx}: huffWeight");
                    assert_eq!(rk1, rk2, "{ctx}: rankStats");
                    assert_eq!(ns1, ns2, "{ctx}: nbSymbols");
                    assert_eq!(tl1, tl2, "{ctx}: tableLog");
                }
            }
            for flags in [0i32, 1] {
                let mut hw1 = vec![0u8; 256];
                let mut hw2 = vec![0u8; 256];
                let mut rk1 = vec![0u32; 16];
                let mut rk2 = vec![0u32; 16];
                let (mut ns1, mut ns2) = (0u32, 0u32);
                let (mut tl1, mut tl2) = (0u32, 0u32);
                let mut wc = vec![0u32; 1024];
                let mut wr = vec![0u32; 1024];
                let a = crsw(hw1.as_mut_ptr(), 256, rk1.as_mut_ptr(), &mut ns1, &mut tl1, p, buf.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
                let b = rrsw(hw2.as_mut_ptr(), 256, rk2.as_mut_ptr(), &mut ns2, &mut tl2, p, buf.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
                e.eq(&format!("HUF_readStats_wksp corrupt #{i} flags={flags}"), a, b);
            }
            // readCTable also parses the same header.
            let mut ct2c = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
            let mut ct2r = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
            let (mut m1, mut m2) = (255u32, 255u32);
            let (mut z1, mut z2) = (0u32, 0u32);
            let a = crc(ct2c.as_mut_ptr(), &mut m1, p, buf.len(), &mut z1);
            let b = rrc(ct2r.as_mut_ptr(), &mut m2, p, buf.len(), &mut z2);
            e.eq(&format!("HUF_readCTable corrupt #{i}"), a, b);
        }
    }
}

// ------------------------ HUF readDTableX1/X2 + decode garbage entry points

#[test]
fn huf_read_dtable_and_decode_garbage() {
    unsafe {
        let e = huf_err();
        let (crd1, rrd1) = both::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp");
        let (crd2, rrd2) = both::<FnHufReadDTableWksp>("HUF_readDTableX2_wksp");
        let (cdu1, rdu1) = both::<FnHufDecompressUsingDTable>("HUF_decompress1X_usingDTable");
        let (cdu4, rdu4) = both::<FnHufDecompressUsingDTable>("HUF_decompress4X_usingDTable");
        let (cd11, rd11) = both::<FnHufDecompressDCtxWksp>("HUF_decompress1X1_DCtx_wksp");
        let (cd12, rd12) = both::<FnHufDecompressDCtxWksp>("HUF_decompress1X2_DCtx_wksp");
        let (cd1g, rd1g) = both::<FnHufDecompressDCtxWksp>("HUF_decompress1X_DCtx_wksp");
        let (cd4h, rd4h) = both::<FnHufDecompressDCtxWksp>("HUF_decompress4X_hufOnly_wksp");

        let mut rng = Rng::new(0xC0_0005);
        let dsz = 1usize << (HUF_TABLELOG_MAX + 1);

        // Build a *valid* Huffman header, then valid X1 and X2 DTables from it.
        // HUF_decompress*_usingDTable REQUIRES a validly-built DTable — feeding an
        // unbuilt (zeroed) table is an unsupported precondition that segfaults
        // BOTH libraries. With a valid table, the garbage goes in `cSrc`.
        let valid_dtable_x1: Vec<u32>;
        let valid_dtable_x2: Vec<u32>;
        {
            let (cbc, _) = both::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp");
            let (cwc, _) = both::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp");
            let mut count = vec![0u32; 256];
            for i in 0..20usize {
                count[i] = (40 - i as u32) * 3 + 1;
            }
            let msv = 19u32;
            let mut ctable = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
            let mut wksp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
            let used = cbc(ctable.as_mut_ptr(), count.as_ptr(), msv, HUF_TABLELOG_MAX, wksp.as_mut_ptr() as *mut c_void, wksp.len() * 4);
            assert!(!e.c_err(used));
            let mut header = vec![0u8; 512];
            let mut wtmp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
            let hn = cwc(header.as_mut_ptr() as *mut c_void, header.len(), ctable.as_ptr(), msv, used as c_uint, wtmp.as_mut_ptr() as *mut c_void, wtmp.len() * 4);
            assert!(!e.c_err(hn));
            header.truncate(hn);

            let mut d1 = vec![0u32; dsz];
            d1[0] = (HUF_TABLELOG_MAX - 1) * 0x0100_0001;
            let mut w = vec![0u32; 4096];
            let r = crd1(d1.as_mut_ptr(), header.as_ptr() as *const c_void, header.len(), w.as_mut_ptr() as *mut c_void, w.len() * 4, 0);
            assert!(!e.c_err(r), "failed to build valid X1 DTable");
            valid_dtable_x1 = d1;

            let mut d2 = vec![0u32; dsz];
            d2[0] = HUF_TABLELOG_MAX * 0x0100_0001;
            let mut w = vec![0u32; 4096];
            let r = crd2(d2.as_mut_ptr(), header.as_ptr() as *const c_void, header.len(), w.as_mut_ptr() as *mut c_void, w.len() * 4, 0);
            assert!(!e.c_err(r), "failed to build valid X2 DTable");
            valid_dtable_x2 = d2;
        }

        // readDTableX1/X2 on truncated / corrupted / garbage headers.
        for i in 0..1200 {
            let n = rng.below(48);
            let buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let p = buf.as_ptr() as *const c_void;
            for flags in [0i32, 1] {
                let mut dc = vec![0u32; dsz];
                let mut dr = vec![0u32; dsz];
                dc[0] = (HUF_TABLELOG_MAX - 1) * 0x0100_0001;
                dr[0] = (HUF_TABLELOG_MAX - 1) * 0x0100_0001;
                let mut wc = vec![0u32; 4096];
                let mut wr = vec![0u32; 4096];
                let a = crd1(dc.as_mut_ptr(), p, buf.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
                let b = rrd1(dr.as_mut_ptr(), p, buf.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
                let ctx = format!("HUF_readDTableX1_wksp garbage #{i} flags={flags}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_eq!(dc, dr, "{ctx}: DTable");
                }

                let mut dc = vec![0u32; dsz];
                let mut dr = vec![0u32; dsz];
                dc[0] = HUF_TABLELOG_MAX * 0x0100_0001;
                dr[0] = HUF_TABLELOG_MAX * 0x0100_0001;
                let mut wc = vec![0u32; 4096];
                let mut wr = vec![0u32; 4096];
                let a = crd2(dc.as_mut_ptr(), p, buf.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
                let b = rrd2(dr.as_mut_ptr(), p, buf.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
                let ctx = format!("HUF_readDTableX2_wksp garbage #{i} flags={flags}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_eq!(dc, dr, "{ctx}: DTable");
                }
            }
        }

        // 2000+ garbage buffers fed to every decode entry point.
        for i in 0..2200 {
            let n = rng.below(80);
            let buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let p = buf.as_ptr() as *const c_void;
            let dcap = rng.below(512);

            // A validly-built DTable; the garbage is fed via `cSrc` so the
            // decode entry points reject corrupt compressed data cleanly.
            let dtab1 = &valid_dtable_x1;
            let dtab4 = &valid_dtable_x2;
            for flags in [0i32, 1] {
                let mut d1 = vec![0u8; dcap.max(1)];
                let mut d2 = vec![0u8; dcap.max(1)];
                let a = cdu1(d1.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), dtab1.as_ptr(), flags);
                let b = rdu1(d2.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), dtab1.as_ptr(), flags);
                let ctx = format!("HUF_decompress1X_usingDTable garbage #{i} flags={flags}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_bytes_eq(&ctx, &d1[..a], &d2[..b]);
                }

                let mut d1 = vec![0u8; dcap.max(1)];
                let mut d2 = vec![0u8; dcap.max(1)];
                let a = cdu4(d1.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), dtab4.as_ptr(), flags);
                let b = rdu4(d2.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), dtab4.as_ptr(), flags);
                let ctx = format!("HUF_decompress4X_usingDTable garbage #{i} flags={flags}");
                e.eq(&ctx, a, b);
                if !e.c_err(a) {
                    assert_bytes_eq(&ctx, &d1[..a], &d2[..b]);
                }

                // One-shot DCtx_wksp decoders build the table internally.
                for (name, cf, rf) in [
                    ("1X1", &cd11, &rd11),
                    ("1X2", &cd12, &rd12),
                    ("1Xg", &cd1g, &rd1g),
                    ("4Xh", &cd4h, &rd4h),
                ] {
                    let mut dc = vec![0u32; dsz];
                    let mut dr = vec![0u32; dsz];
                    let mut d1 = vec![0u8; dcap.max(1)];
                    let mut d2 = vec![0u8; dcap.max(1)];
                    let mut wc = vec![0u32; 4096];
                    let mut wr = vec![0u32; 4096];
                    let a = cf(dc.as_mut_ptr(), d1.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
                    let b = rf(dr.as_mut_ptr(), d2.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
                    let ctx = format!("HUF_decompress{name}_DCtx_wksp garbage #{i} flags={flags}");
                    e.eq(&ctx, a, b);
                    if !e.c_err(a) {
                        assert_bytes_eq(&ctx, &d1[..a], &d2[..b]);
                    }

                    // workspace too small variant.
                    let mut dc = vec![0u32; dsz];
                    let mut dr = vec![0u32; dsz];
                    let mut d1 = vec![0u8; dcap.max(1)];
                    let mut d2 = vec![0u8; dcap.max(1)];
                    let wsz = rng.below(64);
                    let mut wc = vec![0u32; (wsz / 4).max(1)];
                    let mut wr = vec![0u32; (wsz / 4).max(1)];
                    let a = cf(dc.as_mut_ptr(), d1.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), wc.as_mut_ptr() as *mut c_void, wsz, flags);
                    let b = rf(dr.as_mut_ptr(), d2.as_mut_ptr() as *mut c_void, dcap, p, buf.len(), wr.as_mut_ptr() as *mut c_void, wsz, flags);
                    e.eq(&format!("HUF_decompress{name}_DCtx_wksp smallwksp #{i} flags={flags}"), a, b);
                }
            }
        }
    }
}

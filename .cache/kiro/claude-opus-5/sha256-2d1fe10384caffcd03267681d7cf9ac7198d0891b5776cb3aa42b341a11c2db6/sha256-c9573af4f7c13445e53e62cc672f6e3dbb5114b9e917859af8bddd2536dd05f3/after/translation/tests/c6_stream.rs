//! Phase C, ERRORS.md rows 48–58: the streaming error surface.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_ulonglong, c_void};

type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCStream2 =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int) -> size_t;
type FnDStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> size_t;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;
type FnPledged = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> size_t;

struct P {
    e: Err2,
    cc: *mut c_void,
    rc: *mut c_void,
    cd: *mut c_void,
    rd: *mut c_void,
}
impl P {
    fn new() -> P {
        unsafe {
            let (a, b) = both::<FnVoidToPtr>("ZSTD_createCCtx");
            let (c, d) = both::<FnVoidToPtr>("ZSTD_createDCtx");
            P { e: Err2::new(), cc: a(), rc: b(), cd: c(), rd: d() }
        }
    }
    fn reset(&self) {
        unsafe {
            let (a, b) = both::<FnReset>("ZSTD_CCtx_reset");
            a(self.cc, ZSTD_reset_session_and_parameters);
            b(self.rc, ZSTD_reset_session_and_parameters);
            let (c, d) = both::<FnReset>("ZSTD_DCtx_reset");
            c(self.cd, ZSTD_reset_session_and_parameters);
            d(self.rd, ZSTD_reset_session_and_parameters);
        }
    }
    #[track_caller]
    fn cset(&self, ctx: &str, id: c_int, v: c_int) -> bool {
        unsafe {
            let (a, b) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
            let x = a(self.cc, id, v);
            let y = b(self.rc, id, v);
            self.e.eq_or_oom(&format!("{ctx}: CCtx_setParameter({id},{v})"), x, y);
            !self.e.c.is_err(x) && !self.e.r.is_err(y)
        }
    }
    #[track_caller]
    fn dset(&self, ctx: &str, id: c_int, v: c_int) -> bool {
        unsafe {
            let (a, b) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
            let x = a(self.cd, id, v);
            let y = b(self.rd, id, v);
            self.e.eq_or_oom(&format!("{ctx}: DCtx_setParameter({id},{v})"), x, y);
            !self.e.c.is_err(x) && !self.e.r.is_err(y)
        }
    }
}
impl Drop for P {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrToSize>("ZSTD_freeCCtx");
            a(self.cc);
            b(self.rc);
            let (c, d) = both::<FnPtrToSize>("ZSTD_freeDCtx");
            c(self.cd);
            d(self.rd);
        }
    }
}

fn mkframe(src: &[u8], level: c_int, checksum: c_int) -> Vec<u8> {
    unsafe {
        let e = Err2::new();
        let (cn, _) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cf, _) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        let (sp, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = both::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t,
        >("ZSTD_compress2");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let cc = cn();
        sp(cc, ZSTD_c_compressionLevel, level);
        sp(cc, ZSTD_c_checksumFlag, checksum);
        let cap = bnd(src.len()) + 64;
        let mut o = vec![0u8; cap];
        let s = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
        let n = c2(cc, o.as_mut_ptr() as *mut c_void, cap, s, src.len());
        assert!(!e.c.is_err(n), "mkframe failed");
        cf(cc);
        o.truncate(n);
        o
    }
}

/// ERRORS row 49: `endOp` values that are not 0/1/2.
#[test]
fn bad_end_directive() {
    let p = P::new();
    let mut rng = Rng::new(0xC601);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let src = gen(Shape::Text, 5000, &mut rng);
        for endop in [3i32, 4, 100, -1, -2, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
            p.reset();
            let mut o1 = vec![0u8; 65536];
            let mut o2 = vec![0u8; 65536];
            let mut cib =
                ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            let a = ccs(p.cc, &mut cob, &mut cib, endop);
            let b = rcs(p.rc, &mut rob, &mut rib, endop);
            p.e.eq(&format!("compressStream2 endOp={endop}"), a, b);
            assert_eq!(cib.pos, rib.pos, "endOp={endop}: in pos");
            assert_eq!(cob.pos, rob.pos, "endOp={endop}: out pos");
            assert_bytes_eq(&format!("endOp={endop}"), &o1[..cob.pos], &o2[..rob.pos]);
        }
    }
}

/// ERRORS rows 48, 56: `pos > size` on either buffer.
#[test]
fn bad_buffer_positions() {
    let p = P::new();
    let mut rng = Rng::new(0xC602);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (cds, rds) = both::<FnDStream>("ZSTD_decompressStream");
        let src = gen(Shape::Text, 5000, &mut rng);
        let frame = mkframe(&src, 3, 1);

        // compression side
        for (isize_, ipos, osize, opos) in [
            (10usize, 20usize, 4096usize, 0usize),
            (0, 1, 4096, 0),
            (10, 10, 4096, 0),
            (100, 0, 10, 20),
            (100, 0, 0, 1),
            (100, 0, 10, 10),
            (100, 200, 10, 20),
            // Note: `size` values larger than the real buffer (e.g. usize::MAX
            // with pos == 0) are NOT tested. zstd only validates `pos > size`;
            // an over-large `size` makes the C library read/write past the
            // caller's allocation, which is undefined behaviour in the C itself
            // (it segfaults), so there is no defined result to compare against.
            // Every `pos > size` combination — which the C DOES check — is here.
            (100, usize::MAX, 4096, 0),
            (100, 0, 4096, usize::MAX),
            (0, usize::MAX, 0, usize::MAX),
        ] {
            for endop in [ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_end] {
                p.reset();
                let mut o1 = vec![0u8; 8192];
                let mut o2 = vec![0u8; 8192];
                let mut cib = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void, size: isize_, pos: ipos,
                };
                let mut rib = cib;
                let mut cob = ZSTD_outBuffer {
                    dst: o1.as_mut_ptr() as *mut c_void, size: osize.min(o1.len()), pos: opos,
                };
                let mut rob = ZSTD_outBuffer {
                    dst: o2.as_mut_ptr() as *mut c_void, size: osize.min(o2.len()), pos: opos,
                };
                let a = ccs(p.cc, &mut cob, &mut cib, endop);
                let b = rcs(p.rc, &mut rob, &mut rib, endop);
                let ctx = format!(
                    "compressStream2 in{{size:{isize_},pos:{ipos}}} out{{size:{osize},pos:{opos}}} endop={endop}"
                );
                p.e.eq(&ctx, a, b);
                assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                assert_eq!(cob.pos, rob.pos, "{ctx}: out pos");
            }
        }

        // decompression side
        for (isize_, ipos, osize, opos) in [
            (10usize, 20usize, 4096usize, 0usize),
            (0, 1, 4096, 0),
            (frame.len(), 0, 10, 20),
            (frame.len(), 0, 0, 1),
            (frame.len(), frame.len() + 1, 4096, 0),
            (frame.len(), usize::MAX, 4096, 0),
            (frame.len(), 0, 4096, usize::MAX),
            (0, usize::MAX, 0, usize::MAX),
        ] {
            p.reset();
            let mut o1 = vec![0u8; 8192];
            let mut o2 = vec![0u8; 8192];
            let mut cib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void, size: isize_, pos: ipos,
            };
            let mut rib = cib;
            let mut cob = ZSTD_outBuffer {
                dst: o1.as_mut_ptr() as *mut c_void, size: osize.min(o1.len()), pos: opos,
            };
            let mut rob = ZSTD_outBuffer {
                dst: o2.as_mut_ptr() as *mut c_void, size: osize.min(o2.len()), pos: opos,
            };
            let a = cds(p.cd, &mut cob, &mut cib);
            let b = rds(p.rd, &mut rob, &mut rib);
            let ctx = format!(
                "decompressStream in{{size:{isize_},pos:{ipos}}} out{{size:{osize},pos:{opos}}}"
            );
            p.e.eq(&ctx, a, b);
            assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
            assert_eq!(cob.pos, rob.pos, "{ctx}: out pos");
        }
    }
}

/// ERRORS row 50: `ZSTD_c_stableInBuffer` set and the input buffer changes.
#[test]
fn stable_in_violation() {
    let p = P::new();
    let mut rng = Rng::new(0xC603);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let src = gen(Shape::Text, 200_000, &mut rng);
        let other = gen(Shape::Random, 200_000, &mut rng);
        // three flavours of violation: different pointer, shorter size, mutated content
        for flavour in 0..3 {
            p.reset();
            let ctx = format!("stableInBuffer violation flavour={flavour}");
            if !p.cset(&ctx, ZSTD_c_stableInBuffer, 1) {
                continue;
            }
            let mut o1 = vec![0u8; 4096];
            let mut o2 = vec![0u8; 4096];
            let mut cib =
                ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            let a = ccs(p.cc, &mut cob, &mut cib, ZSTD_e_continue);
            let b = rcs(p.rc, &mut rob, &mut rib, ZSTD_e_continue);
            if !p.e.eq_or_oom(&format!("{ctx}: first call"), a, b) {
                continue;
            }
            // now violate stability
            match flavour {
                0 => {
                    cib.src = other.as_ptr() as *const c_void;
                    rib.src = other.as_ptr() as *const c_void;
                }
                1 => {
                    cib.size = cib.pos.max(1) - 1;
                    rib.size = rib.pos.max(1) - 1;
                }
                _ => {
                    cib.size = src.len() / 2;
                    rib.size = src.len() / 2;
                }
            }
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            let a = ccs(p.cc, &mut cob, &mut cib, ZSTD_e_end);
            let b = rcs(p.rc, &mut rob, &mut rib, ZSTD_e_end);
            p.e.eq(&format!("{ctx}: second call"), a, b);
        }
    }
}

/// ERRORS rows 51, 57: `stableOutBuffer` violated on the compression and the
/// decompression side.
#[test]
fn stable_out_violation() {
    let p = P::new();
    let mut rng = Rng::new(0xC604);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (cds, rds) = both::<FnDStream>("ZSTD_decompressStream");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let src = gen(Shape::Text, 200_000, &mut rng);
        let frame = mkframe(&src, 3, 1);

        for flavour in 0..3 {
            p.reset();
            let ctx = format!("c stableOutBuffer violation flavour={flavour}");
            if !p.cset(&ctx, ZSTD_c_stableOutBuffer, 1) {
                continue;
            }
            let cap = bnd(src.len()) + 64;
            let mut o1 = vec![0u8; cap];
            let mut o1b = vec![0u8; cap];
            let mut o2 = vec![0u8; cap];
            let mut o2b = vec![0u8; cap];
            let mut cib =
                ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len() / 2, pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let a = ccs(p.cc, &mut cob, &mut cib, ZSTD_e_continue);
            let b = rcs(p.rc, &mut rob, &mut rib, ZSTD_e_continue);
            if !p.e.eq_or_oom(&format!("{ctx}: first call"), a, b) {
                continue;
            }
            match flavour {
                0 => {
                    cob.dst = o1b.as_mut_ptr() as *mut c_void;
                    rob.dst = o2b.as_mut_ptr() as *mut c_void;
                }
                1 => {
                    cob.size = cob.pos;
                    rob.size = rob.pos;
                }
                _ => {
                    cob.pos = 0;
                    rob.pos = 0;
                }
            }
            cib.size = src.len();
            rib.size = src.len();
            let a = ccs(p.cc, &mut cob, &mut cib, ZSTD_e_end);
            let b = rcs(p.rc, &mut rob, &mut rib, ZSTD_e_end);
            p.e.eq(&format!("{ctx}: second call"), a, b);
        }

        // decompression side (ERRORS row 57)
        for flavour in 0..3 {
            p.reset();
            let ctx = format!("d stableOutBuffer violation flavour={flavour}");
            if !p.dset(&ctx, ZSTD_d_stableOutBuffer, 1) {
                continue;
            }
            let mut o1 = vec![0u8; src.len() + 64];
            let mut o1b = vec![0u8; src.len() + 64];
            let mut o2 = vec![0u8; src.len() + 64];
            let mut o2b = vec![0u8; src.len() + 64];
            let mut cib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void, size: frame.len() / 2, pos: 0,
            };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            let a = cds(p.cd, &mut cob, &mut cib);
            let b = rds(p.rd, &mut rob, &mut rib);
            if !p.e.eq_or_oom(&format!("{ctx}: first call"), a, b) {
                continue;
            }
            match flavour {
                0 => {
                    cob.dst = o1b.as_mut_ptr() as *mut c_void;
                    rob.dst = o2b.as_mut_ptr() as *mut c_void;
                }
                1 => {
                    cob.size = cob.pos;
                    rob.size = rob.pos;
                }
                _ => {
                    cob.pos = 0;
                    rob.pos = 0;
                }
            }
            cib.size = frame.len();
            rib.size = frame.len();
            let a = cds(p.cd, &mut cob, &mut cib);
            let b = rds(p.rd, &mut rob, &mut rib);
            p.e.eq(&format!("{ctx}: second call"), a, b);
        }
    }
}

/// ERRORS rows 52–53: more or fewer bytes than the pledged src size.
#[test]
fn over_and_under_pledge() {
    let p = P::new();
    let mut rng = Rng::new(0xC605);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (cpl, rpl) = both::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize");
        let src = gen(Shape::Text, 40_000, &mut rng);
        for pledged in [
            0u64,
            1,
            (src.len() / 2) as u64,
            src.len() as u64,
            (src.len() + 1) as u64,
            (src.len() * 2) as u64,
        ] {
            for endop in [ZSTD_e_continue, ZSTD_e_end] {
                p.reset();
                let ctx = format!("pledged={pledged} endop={endop} feed={}", src.len());
                let a = cpl(p.cc, pledged);
                let b = rpl(p.rc, pledged);
                p.e.eq(&format!("{ctx}: setPledgedSrcSize"), a, b);
                if p.e.c.is_err(a) {
                    continue;
                }
                let mut o1 = vec![0u8; 1 << 20];
                let mut o2 = vec![0u8; 1 << 20];
                let mut cib =
                    ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
                let mut rib = cib;
                let mut cob =
                    ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
                let mut rob =
                    ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
                let x = ccs(p.cc, &mut cob, &mut cib, endop);
                let y = rcs(p.rc, &mut rob, &mut rib, endop);
                p.e.eq(&ctx, x, y);
                assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                assert_eq!(cob.pos, rob.pos, "{ctx}: out pos");
                assert_bytes_eq(&ctx, &o1[..cob.pos], &o2[..rob.pos]);
            }
        }
    }
}

/// ERRORS rows 54–55: `noForwardProgress_destFull` / `_inputEmpty`.
#[test]
fn no_forward_progress() {
    let p = P::new();
    let mut rng = Rng::new(0xC606);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (cds, rds) = both::<FnDStream>("ZSTD_decompressStream");
        let src = gen(Shape::Text, 300_000, &mut rng);
        let frame = mkframe(&src, 3, 1);

        // compression: zero-size output buffer, repeatedly
        p.reset();
        let mut sink = [0u8; 1];
        for i in 0..64 {
            let mut cib =
                ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: sink.as_mut_ptr() as *mut c_void, size: 0, pos: 0 };
            let mut rob = cob;
            let a = ccs(p.cc, &mut cob, &mut cib, ZSTD_e_end);
            let b = rcs(p.rc, &mut rob, &mut rib, ZSTD_e_end);
            p.e.eq(&format!("compress no-progress iter={i}"), a, b);
            assert_eq!(cib.pos, rib.pos, "iter={i}: in pos");
            if p.e.c.is_err(a) {
                break;
            }
        }
        // compression: empty input, endOp=continue, repeatedly
        p.reset();
        let mut o = vec![0u8; 4096];
        for i in 0..64 {
            let mut cib = ZSTD_inBuffer { src: std::ptr::null(), size: 0, pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o.as_mut_ptr() as *mut c_void, size: o.len(), pos: 0 };
            let mut rob = cob;
            let a = ccs(p.cc, &mut cob, &mut cib, ZSTD_e_continue);
            let b = rcs(p.rc, &mut rob, &mut rib, ZSTD_e_continue);
            p.e.eq(&format!("compress empty-input iter={i}"), a, b);
            if p.e.c.is_err(a) {
                break;
            }
        }
        // decompression: zero-size output buffer, repeatedly
        p.reset();
        for i in 0..64 {
            let mut cib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0,
            };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: sink.as_mut_ptr() as *mut c_void, size: 0, pos: 0 };
            let mut rob = cob;
            let a = cds(p.cd, &mut cob, &mut cib);
            let b = rds(p.rd, &mut rob, &mut rib);
            p.e.eq(&format!("decompress no-progress iter={i}"), a, b);
            assert_eq!(cib.pos, rib.pos, "iter={i}: in pos");
            if p.e.c.is_err(a) {
                break;
            }
        }
        // decompression: empty input, repeatedly
        p.reset();
        for i in 0..64 {
            let mut cib = ZSTD_inBuffer { src: std::ptr::null(), size: 0, pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o.as_mut_ptr() as *mut c_void, size: o.len(), pos: 0 };
            let mut rob = cob;
            let a = cds(p.cd, &mut cob, &mut cib);
            let b = rds(p.rd, &mut rob, &mut rib);
            p.e.eq(&format!("decompress empty-input iter={i}"), a, b);
            if p.e.c.is_err(a) {
                break;
            }
        }
    }
}

/// ERRORS row 58: `flushStream` / `endStream` / `compressStream` before init,
/// plus `initCStream_advanced` with invalid cParams (ERRORS row 59).
#[test]
fn flush_without_init_and_bad_init() {
    let p = P::new();
    unsafe {
        let (cfs, rfs) = both::<FnFlush>("ZSTD_flushStream");
        let (ces, res) = both::<FnFlush>("ZSTD_endStream");
        let (ccs, rcs) = both::<FnDStream>("ZSTD_compressStream");
        let mut o1 = vec![0u8; 4096];
        let mut o2 = vec![0u8; 4096];
        for stage in 0..2 {
            p.reset();
            let ctx = format!("no-init stage={stage}");
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            p.e.eq(&format!("{ctx}: flushStream"), cfs(p.cc, &mut cob), rfs(p.rc, &mut rob));
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            p.e.eq(&format!("{ctx}: endStream"), ces(p.cc, &mut cob), res(p.rc, &mut rob));
            let data = [1u8, 2, 3, 4];
            let mut cib =
                ZSTD_inBuffer { src: data.as_ptr() as *const c_void, size: 4, pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            p.e.eq(
                &format!("{ctx}: compressStream"),
                ccs(p.cc, &mut cob, &mut cib),
                rcs(p.rc, &mut rob, &mut rib),
            );
        }

        // ERRORS row 59: initCStream_advanced with out-of-bound cParams
        type FnInitAdv = unsafe extern "C" fn(
            *mut c_void, *const c_void, size_t, ZSTD_parameters, c_ulonglong,
        ) -> size_t;
        let (cia, ria) = both::<FnInitAdv>("ZSTD_initCStream_advanced");
        let base = ZSTD_compressionParameters {
            windowLog: 20, chainLog: 16, hashLog: 17, searchLog: 1,
            minMatch: 5, targetLength: 0, strategy: 1,
        };
        for (field, bad) in [
            (0usize, 9u32), (0, 32), (0, u32::MAX),
            (1, 5), (1, 31), (1, u32::MAX),
            (2, 5), (2, 31), (2, u32::MAX),
            (3, 0), (3, 31), (3, u32::MAX),
            (4, 2), (4, 8), (4, u32::MAX),
            (5, 131_073), (5, u32::MAX),
            (6, 0), (6, 10), (6, u32::MAX),
        ] {
            let mut c = base;
            match field {
                0 => c.windowLog = bad,
                1 => c.chainLog = bad,
                2 => c.hashLog = bad,
                3 => c.searchLog = bad,
                4 => c.minMatch = bad,
                5 => c.targetLength = bad,
                _ => c.strategy = bad,
            }
            p.reset();
            let params = ZSTD_parameters { cParams: c, fParams: Default::default() };
            let a = cia(p.cc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN);
            let b = ria(p.rc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN);
            p.e.eq(&format!("initCStream_advanced(bad {c:?})"), a, b);
        }
    }
}

/// Streaming on truncated / corrupted frames: exhaustive truncation sweep plus
/// randomized mutation, asserting identical error codes at the identical step.
#[test]
fn decompress_stream_truncation_and_mutation() {
    let p = P::new();
    let mut rng = Rng::new(0xC607);
    unsafe {
        let (cds, rds) = both::<FnDStream>("ZSTD_decompressStream");
        for &shape in &[Shape::Text, Shape::Random, Shape::Zeros, Shape::LongMatches] {
            for &ck in &[0i32, 1] {
                let src = gen(shape, 20_000, &mut rng);
                let frame = mkframe(&src, 5, ck);
                // every truncation length
                for cut in 0..=frame.len() {
                    p.reset();
                    let mut o1 = vec![0u8; 65536];
                    let mut o2 = vec![0u8; 65536];
                    let mut cib = ZSTD_inBuffer {
                        src: frame.as_ptr() as *const c_void, size: cut, pos: 0,
                    };
                    let mut rib = cib;
                    let mut cob = ZSTD_outBuffer {
                        dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0,
                    };
                    let mut rob = ZSTD_outBuffer {
                        dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0,
                    };
                    let a = cds(p.cd, &mut cob, &mut cib);
                    let b = rds(p.rd, &mut rob, &mut rib);
                    let ctx = format!("{shape:?} ck={ck} truncate={cut}");
                    p.e.eq(&ctx, a, b);
                    assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                    assert_bytes_eq(&ctx, &o1[..cob.pos], &o2[..rob.pos]);
                }
                // randomized single-byte mutations
                for i in 0..4000 {
                    let mut f = frame.clone();
                    let off = rng.below(f.len());
                    f[off] = match i % 3 {
                        0 => 0x00,
                        1 => 0xFF,
                        _ => f[off] ^ (1u8 << rng.below(8)),
                    };
                    p.reset();
                    let mut o1 = vec![0u8; 65536];
                    let mut o2 = vec![0u8; 65536];
                    let mut cib = ZSTD_inBuffer {
                        src: f.as_ptr() as *const c_void, size: f.len(), pos: 0,
                    };
                    let mut rib = cib;
                    let mut cob = ZSTD_outBuffer {
                        dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0,
                    };
                    let mut rob = ZSTD_outBuffer {
                        dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0,
                    };
                    let a = cds(p.cd, &mut cob, &mut cib);
                    let b = rds(p.rd, &mut rob, &mut rib);
                    let ctx = format!("{shape:?} ck={ck} mutate#{i} off={off}");
                    p.e.eq(&ctx, a, b);
                    assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                    assert_bytes_eq(&ctx, &o1[..cob.pos], &o2[..rob.pos]);
                }
            }
        }
    }
}

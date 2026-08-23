//! Read-side rejection ("error surface") differential tests.
//!
//! Every individual malformed datastream is driven through the C `.so` and the
//! Rust `.so` in its own `diff(...)` run, so a fatal error on one input can
//! never mask a later input.  The complete trace (messages, warning-vs-fatal
//! behaviour, longjmp `rc`, partially decoded info state, decoded rows) is
//! compared byte for byte.
//!
//! Triggers were derived from the guarding conditions in `c_src/src/pngrutil.c`,
//! `pngread.c`, `pngpread.c` and `pngrio.c`; the corresponding source line is
//! given in a comment for each family.
mod support;

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};
use support::core::*;
use support::pngbuild::{join, split, zlib_stored, Builder, Chunk};
use support::*;

// ---------------------------------------------------------------------------
// input construction helpers
// ---------------------------------------------------------------------------

const STRIDE: usize = 1024;
const MAXH: usize = 24;

fn ck(name: &[u8; 4], data: Vec<u8>) -> Chunk {
    Chunk::new(name, data)
}

fn g(n: usize, seed: u64) -> Vec<u8> {
    Rng::new(seed).bytes(n)
}

fn ihdr(w: u32, h: u32, bd: u8, ct: u8, il: u8) -> Chunk {
    ck(b"IHDR", Builder::new(w, h, bd, ct).interlace(il).ihdr_bytes())
}

fn rawrows(w: u32, h: u32, bd: u8, ct: u8, il: u8) -> Vec<u8> {
    Builder::new(w, h, bd, ct).interlace(il).raw_rows(0x1234)
}

fn idat(w: u32, h: u32, bd: u8, ct: u8, il: u8) -> Chunk {
    ck(b"IDAT", zlib_stored(&rawrows(w, h, bd, ct, il)))
}

fn iend() -> Chunk {
    ck(b"IEND", Vec::new())
}

fn plte(n: usize) -> Chunk {
    ck(b"PLTE", g(3 * n, 0x5eed_1111))
}

/// 4x2 grey 8-bit stream with `extra` inserted in front of IDAT.
fn grey(extra: &[Chunk]) -> Vec<u8> {
    let mut v = vec![ihdr(4, 2, 8, 0, 0)];
    v.extend(extra.iter().cloned());
    v.push(idat(4, 2, 8, 0, 0));
    v.push(iend());
    join(&v)
}

/// 4x2 palette 8-bit stream (256 entry PLTE) with `extra` after the PLTE.
fn palimg(extra: &[Chunk]) -> Vec<u8> {
    let mut v = vec![ihdr(4, 2, 8, 3, 0), plte(256)];
    v.extend(extra.iter().cloned());
    v.push(idat(4, 2, 8, 3, 0));
    v.push(iend());
    join(&v)
}

/// 4x2 stream of colour type `ct`, bit depth `bd`, `extra` before IDAT.
fn img(ct: u8, bd: u8, extra: &[Chunk]) -> Vec<u8> {
    let mut v = vec![ihdr(4, 2, bd, ct, 0)];
    if ct == 3 {
        v.push(plte(1usize << bd.min(8)));
    }
    v.extend(extra.iter().cloned());
    v.push(idat(4, 2, bd, ct, 0));
    v.push(iend());
    join(&v)
}

// ---------------------------------------------------------------------------
// drivers
// ---------------------------------------------------------------------------

fn noset(_c: &Core, _p: Png, _i: Info) {}

fn benign_on(c: &Core, p: Png, _i: Info) {
    unsafe { (c.set_benign_errors)(p, 1) }
}

fn benign_off(c: &Core, p: Png, _i: Info) {
    unsafe { (c.set_benign_errors)(p, 0) }
}

/// mode 0: `png_read_info` only.  1: full read (rows + end).  2: info + end.
unsafe fn seq_body(c: &Core, p: Png, i: Info, mode: u8, bp: *mut u8) {
    (c.read_info)(p, i);
    log_all_info(c, p, i);
    if mode == 0 {
        return;
    }
    let h = (c.get_image_height)(p, i) as usize;
    let passes = (c.set_interlace_handling)(p);
    (c.read_update_info)(p, i);
    let rb = (c.get_rowbytes)(p, i);
    log(format!("rb={rb} passes={passes} h={h}"));
    if mode == 1 {
        if rb + 8 <= STRIDE && h <= MAXH && h > 0 {
            for pass in 0..passes {
                for y in 0..h {
                    let rp = bp.add(y * STRIDE);
                    (c.read_row)(p, rp, std::ptr::null_mut());
                    log(format!(
                        "p{pass}r{y}={}",
                        hex(std::slice::from_raw_parts(rp, rb))
                    ));
                }
            }
        } else {
            log("SKIP_ROWS".to_string());
        }
    }
    (c.read_end)(p, i);
    log("after_end".to_string());
    log_all_info(c, p, i);
}

fn drive(label: &str, png: &[u8], mode: u8, pre: &dyn Fn(&Core, Png, Info)) {
    let mut buf = vec![0u8; MAXH * STRIDE];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| {
        with_read(lib, png, &mut |c, p, i| unsafe {
            std::ptr::write_bytes(bp, 0, MAXH * STRIDE);
            pre(c, p, i);
            seq_body(c, p, i, mode, bp);
        })
    });
}

/// Same input run with `png_set_benign_errors(0)` and `(1)`.
fn drive_both(label: &str, png: &[u8], mode: u8) {
    drive(&format!("{label} benign=0"), png, mode, &benign_off);
    drive(&format!("{label} benign=1"), png, mode, &benign_on);
    drive(&format!("{label} benign=default"), png, mode, &noset);
}

// --- failing allocator ------------------------------------------------------

extern "C" {
    #[link_name = "calloc"]
    fn c_calloc(n: usize, size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn c_free(p: *mut c_void);
}

thread_local! {
    static FAIL_SIZE: Cell<usize> = const { Cell::new(usize::MAX) };
}

fn set_fail(n: usize) {
    FAIL_SIZE.with(|f| f.set(n));
}

unsafe extern "C" fn m_alloc(_p: *mut c_void, size: usize) -> *mut c_void {
    if size == FAIL_SIZE.with(|f| f.get()) {
        log(format!("MALLOC_FAIL({size})"));
        return std::ptr::null_mut();
    }
    unsafe { c_calloc(1, if size == 0 { 1 } else { size }) }
}

unsafe extern "C" fn m_free(_p: *mut c_void, q: *mut c_void) {
    if !q.is_null() {
        unsafe { c_free(q) }
    }
}

/// Sequential read through a struct with a user allocator that fails every
/// allocation of exactly `fail` bytes.
fn drive_mem(label: &str, png: &[u8], fail: usize, mode: u8, pre: &dyn Fn(&Core, Png, Info)) {
    let mut buf = vec![0u8; MAXH * STRIDE];
    let bp = buf.as_mut_ptr();
    diff(label, |lib| {
        session_reset(png.to_vec());
        set_fail(fail);
        let c = Core::new(lib);
        let rc = protected(|| unsafe {
            std::ptr::write_bytes(bp, 0, MAXH * STRIDE);
            let p = (c.create_read_2)(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
                std::ptr::null_mut(),
                m_alloc as Cb,
                m_free as Cb,
            );
            log(format!("create={}", (!p.is_null()) as u8));
            if p.is_null() {
                return;
            }
            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
            (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
            let i = (c.create_info)(p);
            pre(&c, p, i);
            seq_body(&c, p, i, mode, bp);
            let mut pp = p;
            let mut ii = i;
            (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
            log("destroyed".to_string());
        });
        set_fail(usize::MAX);
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

// --- progressive ------------------------------------------------------------

// P_UPDATE holds png_read_update_info of the library currently under test: a
// progressive reader has to call it from the info callback, otherwise
// png_read_start_row never runs (num_rows / row_buf stay unset).
thread_local! {
    static P_UPDATE: Cell<usize> = const { Cell::new(0) };
    static P_COMBINE: Cell<usize> = const { Cell::new(0) };
    static P_ROWBUF: Cell<usize> = const { Cell::new(0) };
    /// bit 0: call png_read_update_info from info_fn
    /// bit 1: call png_progressive_combine_row from row_fn
    /// bit 2: call png_progressive_combine_row from info_fn (before any row)
    static P_MODE: Cell<u8> = const { Cell::new(0) };
}

unsafe extern "C" fn p_info(png: *mut c_void, info: *mut c_void) {
    log("PROG_INFO".to_string());
    let m = P_MODE.with(|c| c.get());
    if m & 1 != 0 {
        let f: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            std::mem::transmute(P_UPDATE.with(|c| c.get()));
        f(png, info);
        log("PROG_UPDATE_INFO".to_string());
    }
    if m & 4 != 0 {
        // new_row is only a flag for png_progressive_combine_row; a non-NULL
        // value makes it call png_combine_row, which is what this exercises.
        let f: unsafe extern "C" fn(*mut c_void, *mut u8, *const u8) =
            std::mem::transmute(P_COMBINE.with(|c| c.get()));
        let bp = P_ROWBUF.with(|c| c.get()) as *mut u8;
        f(png, bp, bp as *const u8);
        log("PROG_COMBINE_EARLY".to_string());
    }
}

unsafe extern "C" fn p_row(png: *mut c_void, row: *mut u8, n: u32, pass: c_int) {
    log(format!(
        "PROG_ROW n={n} pass={pass} null={}",
        row.is_null() as u8
    ));
    let m = P_MODE.with(|c| c.get());
    if m & 2 != 0 && !row.is_null() {
        let bp = P_ROWBUF.with(|c| c.get()) as *mut u8;
        if !bp.is_null() && (n as usize) < MAXH {
            let f: unsafe extern "C" fn(*mut c_void, *mut u8, *const u8) =
                std::mem::transmute(P_COMBINE.with(|c| c.get()));
            let dst = bp.add(n as usize * STRIDE);
            f(png, dst, row);
            log(format!(
                "PROG_COMBINE n={n} {}",
                hex(std::slice::from_raw_parts(dst, 16))
            ));
        }
    }
}

unsafe extern "C" fn p_end(_png: *mut c_void, _info: *mut c_void) {
    log("PROG_END".to_string());
}

/// Feed `png` to the progressive reader in `step`-byte pieces.  `fail` is the
/// allocation size the user allocator refuses (`usize::MAX` = never).  `mode`
/// selects what the callbacks do (see `P_MODE`).
fn prog_m(
    label: &str,
    png: &[u8],
    step: usize,
    fail: usize,
    mode: u8,
    pre: &dyn Fn(&Core, Png, Info),
) {
    let mut buf = vec![0u8; MAXH * STRIDE];
    let bp = buf.as_mut_ptr() as usize;
    let mut data = png.to_vec();
    let dp = data.as_mut_ptr();
    let len = data.len();
    diff(label, |lib| {
        session_reset(Vec::new());
        set_fail(fail);
        let c = Core::new(lib);
        P_UPDATE.with(|x| x.set(lib.raw("png_read_update_info") as usize));
        P_COMBINE.with(|x| x.set(lib.raw("png_progressive_combine_row") as usize));
        P_ROWBUF.with(|x| x.set(bp));
        P_MODE.with(|x| x.set(mode));
        let rc = protected(|| unsafe {
            std::ptr::write_bytes(bp as *mut u8, 0, MAXH * STRIDE);
            let p = (c.create_read_2)(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
                std::ptr::null_mut(),
                m_alloc as Cb,
                m_free as Cb,
            );
            log(format!("create={}", (!p.is_null()) as u8));
            if p.is_null() {
                return;
            }
            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
            let i = (c.create_info)(p);
            (c.set_progressive_read_fn)(
                p,
                std::ptr::null_mut(),
                p_info as Cb,
                p_row as Cb,
                p_end as Cb,
            );
            pre(&c, p, i);
            let mut off = 0usize;
            while off < len {
                let n = std::cmp::min(step, len - off);
                (c.process_data)(p, i, dp.add(off), n);
                off += n;
            }
            log("PROG_DONE".to_string());
            log_all_info(&c, p, i);
            let mut pp = p;
            let mut ii = i;
            (c.destroy_read)(&mut pp, &mut ii, std::ptr::null_mut());
            log("destroyed".to_string());
        });
        set_fail(usize::MAX);
        P_MODE.with(|x| x.set(0));
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

/// The normal progressive-reader usage: `png_read_update_info` from the info
/// callback and `png_progressive_combine_row` from the row callback.
fn prog(label: &str, png: &[u8], step: usize, fail: usize, pre: &dyn Fn(&Core, Png, Info)) {
    prog_m(label, png, step, fail, 3, pre)
}

// ===========================================================================
// 1. signature + chunk header + IHDR
// ===========================================================================

#[test]
fn signature_and_header() {
    let good = grey(&[]);

    // pngrutil.c:139 -- "Not a PNG file": the first four bytes are wrong.
    let mut bad = good.clone();
    bad[0..8].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    drive("SIG all-zero", &bad, 0, &noset);
    let mut bad = good.clone();
    bad[1] = b'Q';
    drive("SIG byte1", &bad, 0, &noset);

    // pngrutil.c:141 -- "PNG file corrupted by ASCII conversion": the first
    // four bytes are correct, a later one is not (CRLF/LF mangling).
    let mut bad = good.clone();
    bad[0..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0a, 0x1a, 0x0a, 0x0a]);
    drive("SIG lf-mangled", &bad, 0, &noset);
    let mut bad = good.clone();
    bad[0..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0d, 0x0a, 0x1a]);
    drive("SIG crlf-mangled", &bad, 0, &noset);

    // Truncated input: the harness read callback longjmps.
    drive("SIG short-4", &good[..4], 0, &noset);
    drive("SIG short-0", &[], 0, &noset);

    // png_set_sig_bytes: correct count (the app consumed 4 bytes).
    drive("SIG sig_bytes=4 stripped", &good[4..], 0, &|c, p, _i| unsafe {
        (c.set_sig_bytes)(p, 4)
    });
    // Mismatched count: the file still contains the whole signature.
    drive("SIG sig_bytes=4 mismatch", &good, 0, &|c, p, _i| unsafe {
        (c.set_sig_bytes)(p, 4)
    });
    // 8 => no signature check at all, the signature is read as a chunk header
    // (pngrutil.c:46 "PNG unsigned integer out of range").
    drive("SIG sig_bytes=8", &good, 0, &|c, p, _i| unsafe {
        (c.set_sig_bytes)(p, 8)
    });
    // png.c:66 "Too many bytes for PNG signature"
    drive("SIG sig_bytes=9", &good, 0, &|c, p, _i| unsafe {
        (c.set_sig_bytes)(p, 9)
    });

    // pngrutil.c:46 -- chunk length with bit 31 set.
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0).with_len(0x8000_0000),
        idat(4, 2, 8, 0, 0),
        iend(),
    ]);
    drive("HDR length>2^31", &bad, 0, &noset);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"gAMA", vec![0, 1, 0, 0]).with_len(0xffff_ffff),
        idat(4, 2, 8, 0, 0),
        iend(),
    ]);
    drive("HDR anc length=0xffffffff", &bad, 0, &noset);

    // pngrutil.c:215 -- "bad header (invalid type)"
    for name in [b"IH1R", b"ih\0r", b"    ", b"IHD{"] {
        let bad = join(&[
            ck(name, Builder::new(4, 2, 8, 0).ihdr_bytes()),
            idat(4, 2, 8, 0, 0),
            iend(),
        ]);
        drive(
            &format!("HDR bad type {}", hex(name)),
            &bad,
            0,
            &noset,
        );
    }

    // pngrutil.c:3135 -- "missing IHDR": a known chunk before IHDR.
    for c in [
        ck(b"gAMA", 45455u32.to_be_bytes().to_vec()),
        ck(b"PLTE", g(9, 1)),
        ck(b"IEND", Vec::new()),
        ck(b"tEXt", b"k\0v".to_vec()),
    ] {
        let nm = String::from_utf8_lossy(&c.name).into_owned();
        let bad = join(&[c, ihdr(4, 2, 8, 0, 0), idat(4, 2, 8, 0, 0), iend()]);
        drive_both(&format!("HDR {nm} before IHDR"), &bad, 0);
    }

    // IHDR length errors (the generic length checks in png_handle_chunk).
    let mut d = Builder::new(4, 2, 8, 0).ihdr_bytes();
    d.truncate(12);
    let bad = join(&[ck(b"IHDR", d), idat(4, 2, 8, 0, 0), iend()]);
    drive("IHDR len=12", &bad, 0, &noset);
    let mut d = Builder::new(4, 2, 8, 0).ihdr_bytes();
    d.push(0);
    let bad = join(&[ck(b"IHDR", d), idat(4, 2, 8, 0, 0), iend()]);
    drive("IHDR len=14", &bad, 0, &noset);
    let bad = join(&[ck(b"IHDR", Vec::new()), idat(4, 2, 8, 0, 0), iend()]);
    drive("IHDR len=0", &bad, 0, &noset);

    // IHDR duplicated (critical duplicate -> chunk_error).
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        iend(),
    ]);
    drive("IHDR duplicate", &bad, 0, &noset);

    // png_check_IHDR: every illegal field value.
    let cases: &[(u32, u32, u8, u8, u8, u8, u8, &str)] = &[
        (0, 2, 8, 0, 0, 0, 0, "width0"),
        (4, 0, 8, 0, 0, 0, 0, "height0"),
        (0, 0, 8, 0, 0, 0, 0, "wh0"),
        (0x8000_0000, 2, 8, 0, 0, 0, 0, "width2^31"),
        (4, 0x8000_0000, 8, 0, 0, 0, 0, "height2^31"),
        (0xffff_ffff, 2, 8, 0, 0, 0, 0, "widthmax"),
        (4, 2, 0, 0, 0, 0, 0, "depth0"),
        (4, 2, 3, 0, 0, 0, 0, "depth3"),
        (4, 2, 32, 0, 0, 0, 0, "depth32"),
        (4, 2, 8, 1, 0, 0, 0, "ct1"),
        (4, 2, 8, 5, 0, 0, 0, "ct5"),
        (4, 2, 8, 7, 0, 0, 0, "ct7"),
        (4, 2, 8, 255, 0, 0, 0, "ct255"),
        (4, 2, 16, 3, 0, 0, 0, "pal16"),
        (4, 2, 1, 2, 0, 0, 0, "rgb1"),
        (4, 2, 2, 2, 0, 0, 0, "rgb2"),
        (4, 2, 4, 2, 0, 0, 0, "rgb4"),
        (4, 2, 1, 4, 0, 0, 0, "ga1"),
        (4, 2, 2, 6, 0, 0, 0, "rgba2"),
        (4, 2, 4, 6, 0, 0, 0, "rgba4"),
        (4, 2, 8, 0, 1, 0, 0, "comp1"),
        (4, 2, 8, 0, 255, 0, 0, "comp255"),
        (4, 2, 8, 0, 0, 1, 0, "filter1"),
        (4, 2, 8, 0, 0, 64, 0, "filter64"),
        (4, 2, 8, 0, 0, 0, 2, "interlace2"),
        (4, 2, 8, 0, 0, 0, 255, "interlace255"),
    ];
    for &(w, h, bd, ct, cm, fm, il, name) in cases {
        let mut d = Vec::new();
        d.extend_from_slice(&w.to_be_bytes());
        d.extend_from_slice(&h.to_be_bytes());
        d.extend_from_slice(&[bd, ct, cm, fm, il]);
        let bad = join(&[ck(b"IHDR", d), idat(4, 2, 8, 0, 0), iend()]);
        drive_both(&format!("IHDR {name}"), &bad, 0);
    }
}

// ===========================================================================
// 2. critical chunk ordering
// ===========================================================================

#[test]
fn chunk_order() {
    // pngread.c:118 -- "Missing IHDR before IDAT"
    let bad = join(&[idat(4, 2, 8, 0, 0), iend()]);
    drive_both("ORD IDAT first", &bad, 0);

    // pngread.c:122 -- "Missing PLTE before IDAT"
    let bad = join(&[ihdr(4, 2, 8, 3, 0), idat(4, 2, 8, 3, 0), iend()]);
    drive_both("ORD palette without PLTE", &bad, 0);

    // PLTE in a grey image ("ignored in grayscale PNG", benign)
    let bad = grey(&[plte(4)]);
    drive_both("ORD PLTE in grey", &bad, 1);

    // duplicate PLTE
    let bad = join(&[
        ihdr(4, 2, 8, 3, 0),
        plte(256),
        plte(4),
        idat(4, 2, 8, 3, 0),
        iend(),
    ]);
    drive_both("ORD PLTE duplicate", &bad, 0);

    // PLTE after IDAT (handled by png_read_end)
    let bad = join(&[
        ihdr(4, 2, 8, 3, 0),
        plte(256),
        idat(4, 2, 8, 3, 0),
        plte(4),
        iend(),
    ]);
    drive_both("ORD PLTE after IDAT pal", &bad, 1);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        plte(4),
        iend(),
    ]);
    drive_both("ORD PLTE after IDAT grey", &bad, 1);

    // PLTE with an invalid length
    for n in [1usize, 2, 4, 5, 770] {
        let bad = join(&[
            ihdr(4, 2, 8, 3, 0),
            ck(b"PLTE", g(n, 7)),
            idat(4, 2, 8, 3, 0),
            iend(),
        ]);
        drive_both(&format!("ORD PLTE len={n}"), &bad, 0);
    }

    // No IDAT at all: IEND is out of place.
    let bad = join(&[ihdr(4, 2, 8, 0, 0), iend()]);
    drive_both("ORD no IDAT", &bad, 0);

    // IEND with data (pngrutil.c:1092 "invalid")
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        ck(b"IEND", vec![1, 2, 3]),
    ]);
    drive_both("ORD IEND len=3", &bad, 1);

    // A chunk after IEND is simply never read.
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        iend(),
        ck(b"tEXt", b"k\0after".to_vec()),
    ]);
    drive_both("ORD chunk after IEND", &bad, 1);

    // Missing IEND -> the read callback runs dry in png_read_end.
    let bad = join(&[ihdr(4, 2, 8, 0, 0), idat(4, 2, 8, 0, 0)]);
    drive_both("ORD no IEND", &bad, 1);

    // pngread.c:747 -- "..Too many IDATs found" (an IDAT after another chunk)
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        ck(b"tEXt", b"k\0v".to_vec()),
        ck(b"IDAT", vec![]),
        iend(),
    ]);
    drive_both("ORD IDAT after tEXt (empty)", &bad, 1);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        ck(b"tEXt", b"k\0v".to_vec()),
        ck(b"IDAT", vec![1, 2, 3]),
        iend(),
    ]);
    drive_both("ORD IDAT after tEXt (data)", &bad, 1);

    // pngread.c:729 -- ".Too many IDATs found": IDAT handled as unknown.
    let keep_idat = |c: &Core, p: Png, _i: Info| unsafe {
        (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, b"IDAT\0".as_ptr(), 1)
    };
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        ck(b"tEXt", b"k\0v".to_vec()),
        ck(b"IDAT", vec![9, 9, 9]),
        iend(),
    ]);
    drive("ORD keep-IDAT dot", &bad, 2, &keep_idat);

    // pngread.c:125 -- "Too many IDATs found" seen by png_read_info: the first
    // IDAT is consumed as an unknown chunk, so a second png_read_info call
    // reaches the second IDAT with PNG_AFTER_IDAT already set.
    let mut buf = vec![0u8; MAXH * STRIDE];
    let bp = buf.as_mut_ptr();
    let png = bad.clone();
    diff("ORD read_info twice keep-IDAT", |lib| {
        with_read(lib, &png, &mut |c, p, i| unsafe {
            keep_idat(c, p, i);
            (c.read_info)(p, i);
            log("first read_info done".to_string());
            (c.read_info)(p, i);
            log("second read_info done".to_string());
            log_all_info(c, p, i);
            let _ = bp;
        })
    });

    // pngrutil.c:2957 -- "unhandled critical chunk"
    let bad = grey(&[ck(b"TEST", vec![1, 2, 3, 4])]);
    drive_both("ORD unknown critical", &bad, 0);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"ZZZZ", Vec::new()),
        idat(4, 2, 8, 0, 0),
        iend(),
    ]);
    drive_both("ORD unknown critical empty", &bad, 0);
    // IDAT explicitly discarded => unhandled critical chunk
    let ok = grey(&[]);
    drive("ORD keep-IDAT never", &ok, 0, &|c, p, _i| unsafe {
        (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_NEVER, b"IDAT\0".as_ptr(), 1)
    });

    // "out of place" / "duplicate" for ancillary chunks.
    let after = |name: &[u8; 4], data: Vec<u8>| {
        join(&[
            ihdr(4, 2, 8, 0, 0),
            idat(4, 2, 8, 0, 0),
            ck(name, data),
            iend(),
        ])
    };
    for (name, data) in [
        (b"gAMA", 45455u32.to_be_bytes().to_vec()),
        (b"sBIT", vec![7]),
        (b"cHRM", g(32, 3)),
        (b"sRGB", vec![0]),
        (b"pHYs", g(9, 4)),
        (b"oFFs", g(9, 5)),
        (b"bKGD", vec![0, 1]),
        (b"hIST", vec![0, 1]),
        (b"sPLT", b"n\0\x08\0\0\0\0\0\0".to_vec()),
        (b"cICP", vec![1, 13, 0, 1]),
        (b"cLLI", g(8, 6)),
        (b"mDCV", g(24, 7)),
    ] {
        let nm = String::from_utf8_lossy(name).into_owned();
        let bad = after(name, data.clone());
        drive_both(&format!("ORD {nm} after IDAT"), &bad, 1);
        let dup = grey(&[ck(name, data.clone()), ck(name, data)]);
        drive_both(&format!("ORD {nm} duplicate"), &dup, 0);
    }
}

// ===========================================================================
// 3. chunk CRC and chunk length
// ===========================================================================

#[test]
fn crc_and_length() {
    let actions: &[(c_int, &str)] = &[
        (PNG_CRC_DEFAULT, "DEFAULT"),
        (PNG_CRC_ERROR_QUIT, "ERROR_QUIT"),
        (PNG_CRC_WARN_DISCARD, "WARN_DISCARD"),
        (PNG_CRC_WARN_USE, "WARN_USE"),
        (PNG_CRC_QUIET_USE, "QUIET_USE"),
        (PNG_CRC_NO_CHANGE, "NO_CHANGE"),
        (-1, "neg1"),
        (6, "six"),
        (99, "n99"),
    ];

    // Bad CRC on a critical chunk (IHDR, IDAT) x every crit_action.
    let bad_ihdr = join(&[
        ihdr(4, 2, 8, 0, 0).bad_crc(),
        idat(4, 2, 8, 0, 0),
        iend(),
    ]);
    let bad_idat = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0).bad_crc(),
        iend(),
    ]);
    let bad_iend = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        iend().bad_crc(),
    ]);
    let bad_plte = join(&[
        ihdr(4, 2, 8, 3, 0),
        plte(256).bad_crc(),
        idat(4, 2, 8, 3, 0),
        iend(),
    ]);
    let bad_gama = grey(&[ck(b"gAMA", 45455u32.to_be_bytes().to_vec()).bad_crc()]);
    let bad_text = grey(&[ck(b"tEXt", b"key\0value".to_vec()).bad_crc()]);
    for &(a, name) in actions {
        for (inp, tag) in [
            (&bad_ihdr, "IHDR"),
            (&bad_idat, "IDAT"),
            (&bad_iend, "IEND"),
            (&bad_plte, "PLTE"),
        ] {
            drive(
                &format!("CRC crit={name} on {tag}"),
                inp,
                1,
                &move |c, p, _i| unsafe { (c.set_crc_action)(p, a, PNG_CRC_DEFAULT) },
            );
        }
        for (inp, tag) in [(&bad_gama, "gAMA"), (&bad_text, "tEXt")] {
            drive(
                &format!("CRC anc={name} on {tag}"),
                inp,
                1,
                &move |c, p, _i| unsafe { (c.set_crc_action)(p, PNG_CRC_DEFAULT, a) },
            );
        }
    }

    // Chunk lengths that are not the required size.
    let cases: &[(&[u8; 4], usize, u8, u8)] = &[
        (b"gAMA", 3, 0, 8),
        (b"gAMA", 5, 0, 8),
        (b"sBIT", 0, 0, 8),
        (b"sBIT", 2, 0, 8),
        (b"sBIT", 4, 0, 8),
        (b"sBIT", 5, 0, 8),
        (b"sBIT", 2, 2, 8),
        (b"sBIT", 4, 2, 8),
        (b"sBIT", 2, 3, 8),
        (b"sBIT", 5, 6, 8),
        (b"cHRM", 31, 0, 8),
        (b"cHRM", 33, 0, 8),
        (b"sRGB", 0, 0, 8),
        (b"sRGB", 2, 0, 8),
        (b"pHYs", 8, 0, 8),
        (b"pHYs", 10, 0, 8),
        (b"oFFs", 8, 0, 8),
        (b"oFFs", 10, 0, 8),
        (b"tIME", 6, 0, 8),
        (b"tIME", 8, 0, 8),
        (b"cICP", 3, 0, 8),
        (b"cICP", 5, 0, 8),
        (b"cLLI", 7, 0, 8),
        (b"cLLI", 9, 0, 8),
        (b"mDCV", 23, 0, 8),
        (b"mDCV", 25, 0, 8),
        (b"eXIf", 3, 0, 8),
        (b"sCAL", 3, 0, 8),
        (b"pCAL", 13, 0, 8),
        (b"tEXt", 1, 0, 8),
        (b"iTXt", 5, 0, 8),
        (b"zTXt", 13, 0, 8),
        (b"sPLT", 2, 0, 8),
        (b"hIST", 3, 3, 8),
        (b"hIST", 1026, 3, 8),
        (b"tRNS", 1, 0, 8),
        (b"tRNS", 3, 0, 8),
        (b"tRNS", 5, 2, 8),
        (b"tRNS", 7, 2, 8),
        (b"tRNS", 257, 3, 8),
        (b"tRNS", 0, 3, 8),
        (b"tRNS", 2, 4, 8),
        (b"tRNS", 2, 6, 8),
        (b"bKGD", 0, 0, 8),
        (b"bKGD", 1, 0, 8),
        (b"bKGD", 3, 0, 8),
        (b"bKGD", 7, 2, 8),
        (b"bKGD", 2, 3, 8),
    ];
    for &(name, len, ct, bd) in cases {
        let nm = String::from_utf8_lossy(name).into_owned();
        let data = g(len, 0x100 + len as u64);
        let png = img(ct, bd, &[ck(name, data)]);
        drive_both(&format!("LEN {nm} len={len} ct={ct}"), &png, 0);
    }

    // hIST with the right multiple but the wrong count for the palette.
    let png = palimg(&[ck(b"hIST", g(8, 11))]);
    drive_both("LEN hIST 4 entries vs 256", &png, 0);

    // "length exceeds libpng limit" (the Limit chunks) via chunk_malloc_max.
    for name in [b"zTXt", b"eXIf", b"sCAL"] {
        let nm = String::from_utf8_lossy(name).into_owned();
        let png = grey(&[ck(name, g(64, 12))]);
        drive(
            &format!("LEN {nm} over malloc_max"),
            &png,
            0,
            &|c, p, _i| unsafe { (c.set_chunk_malloc_max)(p, 32) },
        );
    }
}

// ===========================================================================
// 4. ancillary chunk content validation
// ===========================================================================

#[test]
fn ancillary_values() {
    // pngrutil.c:1118 gAMA "invalid"
    for v in [0x8000_0000u32, 0xffff_ffff] {
        let png = grey(&[ck(b"gAMA", v.to_be_bytes().to_vec())]);
        drive_both(&format!("gAMA {v:#x}"), &png, 0);
    }
    let png = grey(&[ck(b"gAMA", 0u32.to_be_bytes().to_vec())]);
    drive_both("gAMA 0", &png, 0);

    // pngrutil.c:1300 sRGB "invalid"
    for v in [4u8, 5, 255] {
        let png = grey(&[ck(b"sRGB", vec![v])]);
        drive_both(&format!("sRGB intent={v}"), &png, 0);
    }

    // pngrutil.c:1251 cHRM "invalid" (a value that is not representable)
    let mut d = vec![0u8; 32];
    d[0] = 0x80;
    let png = grey(&[ck(b"cHRM", d)]);
    drive_both("cHRM unrepresentable", &png, 0);
    let png = grey(&[ck(b"cHRM", vec![0xff; 32])]);
    drive_both("cHRM all-ff", &png, 0);

    // pngrutil.c:1178 sBIT "invalid"
    for v in [0u8, 9, 255] {
        let png = grey(&[ck(b"sBIT", vec![v])]);
        drive_both(&format!("sBIT value={v}"), &png, 0);
    }
    let png = img(2, 8, &[ck(b"sBIT", vec![8, 0, 8])]);
    drive_both("sBIT rgb zero", &png, 0);
    let png = palimg(&[ck(b"sBIT", vec![9, 8, 8])]);
    drive_both("sBIT palette 9", &png, 0);

    // pngrutil.c:1823/1844/1862 bKGD
    let png = palimg(&[ck(b"bKGD", vec![255])]);
    drive_both("bKGD index in range", &png, 0);
    let png = join(&[
        ihdr(4, 2, 8, 3, 0),
        plte(4),
        ck(b"bKGD", vec![9]),
        ck(b"IDAT", zlib_stored(&vec![0u8; 10])),
        iend(),
    ]);
    drive_both("bKGD invalid index", &png, 0);
    let png = join(&[
        ihdr(4, 2, 8, 3, 0),
        ck(b"bKGD", vec![0]),
        plte(4),
        ck(b"IDAT", zlib_stored(&vec![0u8; 10])),
        iend(),
    ]);
    drive_both("bKGD out of place", &png, 0);
    for bd in [1u8, 2, 4, 8] {
        let png = img(0, bd, &[ck(b"bKGD", vec![1, 0])]);
        drive_both(&format!("bKGD gray hi nonzero bd={bd}"), &png, 0);
        let png = img(0, bd, &[ck(b"bKGD", vec![0, 0xff])]);
        drive_both(&format!("bKGD gray too big bd={bd}"), &png, 0);
    }
    let png = img(2, 8, &[ck(b"bKGD", vec![1, 0, 0, 0, 0, 0])]);
    drive_both("bKGD color red hi", &png, 0);
    let png = img(2, 8, &[ck(b"bKGD", vec![0, 0, 1, 0, 0, 0])]);
    drive_both("bKGD color green hi", &png, 0);
    let png = img(2, 8, &[ck(b"bKGD", vec![0, 0, 0, 0, 1, 0])]);
    drive_both("bKGD color blue hi", &png, 0);

    // pngrutil.c:2029 eXIf "invalid"
    let png = grey(&[ck(b"eXIf", vec![0x49, 0x49, 0x2a, 0x01])]);
    drive_both("eXIf bad header", &png, 0);
    let png = grey(&[ck(b"eXIf", vec![0x49, 0x49, 0x2a, 0x00, 8, 0, 0, 0])]);
    drive_both("eXIf ok", &png, 0);

    // pngrutil.c:1732/1741/1752 tRNS
    let png = join(&[
        ihdr(4, 2, 8, 3, 0),
        ck(b"tRNS", vec![1, 2]),
        plte(256),
        ck(b"IDAT", zlib_stored(&vec![0u8; 10])),
        iend(),
    ]);
    drive_both("tRNS before PLTE", &png, 0);
    let png = join(&[
        ihdr(4, 2, 8, 3, 0),
        plte(4),
        ck(b"tRNS", g(5, 2)),
        ck(b"IDAT", zlib_stored(&vec![0u8; 10])),
        iend(),
    ]);
    drive_both("tRNS longer than PLTE", &png, 0);
    for ct in [4u8, 6] {
        let png = img(ct, 8, &[ck(b"tRNS", vec![0, 1])]);
        drive_both(&format!("tRNS with alpha ct={ct}"), &png, 0);
    }

    // pngrutil.c:2292..2319 sCAL
    let scal: &[(&[u8], &str)] = &[
        (b"\x03" as &[u8], "unit3"),
        (b"\x001\x001", "unit0"),
        (b"\x01abc\x001", "badwidth"),
        (b"\x010\x001", "zerowidth"),
        (b"\x01-1\x001", "negwidth"),
        (b"\x011\x00abc", "badheight"),
        (b"\x011\x000", "zeroheight"),
        (b"\x011\x00-2", "negheight"),
        (b"\x011", "noheight"),
        (b"\x011\x00", "emptyheight"),
        (b"\x0112\x0034", "ok"),
        (b"\x021e2\x001e-2", "expok"),
        (b"\x011\x001\x001", "trailing"),
    ];
    for &(d, name) in scal {
        let png = grey(&[ck(b"sCAL", d.to_vec())]);
        drive_both(&format!("sCAL {name}"), &png, 0);
    }

    // pngrutil.c:2183..2240 pCAL
    let pcal: &[(Vec<u8>, &str)] = &[
        (
            {
                let mut v = b"purpose\0".to_vec();
                v.extend_from_slice(&0i32.to_be_bytes());
                v.extend_from_slice(&100i32.to_be_bytes());
                v.push(0); // linear
                v.push(2); // nparams
                v.extend_from_slice(b"unit\0");
                v.extend_from_slice(b"1\0");
                v.extend_from_slice(b"2");
                v
            },
            "ok",
        ),
        (
            {
                let mut v = b"purpose\0".to_vec();
                v.extend_from_slice(&0i32.to_be_bytes());
                v.extend_from_slice(&100i32.to_be_bytes());
                v.push(0);
                v.push(3); // wrong count for linear
                v.extend_from_slice(b"unit\0");
                v.extend_from_slice(b"1\0002\0003");
                v
            },
            "count",
        ),
        (
            {
                let mut v = b"purpose\0".to_vec();
                v.extend_from_slice(&0i32.to_be_bytes());
                v.extend_from_slice(&100i32.to_be_bytes());
                v.push(4); // unrecognized equation type
                v.push(0);
                v.extend_from_slice(b"unit\0");
                v
            },
            "type4",
        ),
        (
            {
                let mut v = b"purpose\0".to_vec();
                v.extend_from_slice(&0i32.to_be_bytes());
                v.extend_from_slice(&100i32.to_be_bytes());
                v.push(255);
                v.push(1);
                v.extend_from_slice(b"unit\0");
                v.extend_from_slice(b"5");
                v
            },
            "type255",
        ),
        (
            {
                // purpose string only: fewer than 12 bytes of trailing data
                let mut v = b"purpose\0".to_vec();
                v.extend_from_slice(&[0; 11]);
                v
            },
            "short",
        ),
        (
            {
                // hyperbolic, 4 params but the data runs out
                let mut v = b"p\0".to_vec();
                v.extend_from_slice(&0i32.to_be_bytes());
                v.extend_from_slice(&100i32.to_be_bytes());
                v.push(3);
                v.push(4);
                v.extend_from_slice(b"u\0");
                v.extend_from_slice(b"1\0002");
                v
            },
            "truncparams",
        ),
        (
            {
                let mut v = b"p\0".to_vec();
                v.extend_from_slice(&0i32.to_be_bytes());
                v.extend_from_slice(&100i32.to_be_bytes());
                v.push(2);
                v.push(3);
                v.extend_from_slice(b"u\0");
                v.extend_from_slice(b"x\0y\0z");
                v
            },
            "badfp",
        ),
    ];
    for (d, name) in pcal {
        let png = grey(&[ck(b"pCAL", d.clone())]);
        drive_both(&format!("pCAL {name}"), &png, 0);
    }

    // pngrutil.c:2063 hIST
    let png = palimg(&[ck(b"hIST", g(512, 13))]);
    drive_both("hIST ok", &png, 0);

    // pngrutil.c:1612..1646 sPLT
    let splt: &[(Vec<u8>, &str)] = &[
        (b"abc".to_vec(), "no-nul"),
        (b"a\0\x08".to_vec(), "no-entries"),
        (b"a\0\x08\x01".to_vec(), "badlen8"),
        (b"a\0\x10\x01\x02".to_vec(), "badlen16"),
        (b"a\0\x08\x01\x02\x03\x04\x05\x06".to_vec(), "ok8"),
        (
            b"a\0\x10\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a".to_vec(),
            "ok16",
        ),
        (b"\0\x08\x01\x02\x03\x04\x05\x06".to_vec(), "emptyname"),
        (b"a\0\x07\x01\x02\x03\x04\x05\x06".to_vec(), "depth7"),
    ];
    for (d, name) in splt {
        let png = grey(&[ck(b"sPLT", d.clone())]);
        drive_both(&format!("sPLT {name}"), &png, 0);
    }

    // pngrutil.c:1588/2161/2279/2408/2594 "out of memory" through
    // png_set_chunk_malloc_max (png_read_buffer refuses length+1).
    let small = |c: &Core, p: Png, _i: Info| unsafe { (c.set_chunk_malloc_max)(p, 16) };
    for (name, data) in [
        (b"tEXt", b"key\0value-value".to_vec()),
        (b"iTXt", b"key\0\0\0en\0k\0text".to_vec()),
        (b"sPLT", b"nm\0\x08\x01\x02\x03\x04\x05\x06".to_vec()),
        (b"pCAL", {
            let mut v = b"purpose\0".to_vec();
            v.extend_from_slice(&[0; 12]);
            v.extend_from_slice(b"u\0");
            v
        }),
    ] {
        let nm = String::from_utf8_lossy(name).into_owned();
        let png = grey(&[ck(name, data)]);
        drive(&format!("OOM-limit {nm}"), &png, 0, &small);
    }
    // sCAL is length-limited, so make length == the limit: length+1 overflows.
    let png = grey(&[ck(b"sCAL", b"\x0112\x0034".to_vec())]);
    drive("OOM-limit sCAL", &png, 0, &|c, p, _i| unsafe {
        (c.set_chunk_malloc_max)(p, 8)
    });
}

// ===========================================================================
// 5. tEXt / zTXt / iTXt
// ===========================================================================

#[test]
fn text_chunks() {
    // tEXt
    for (d, name) in [
        (b"\0".to_vec(), "empty-key"),
        (b"k\0".to_vec(), "empty-text"),
        (b"ab".to_vec(), "no-separator"),
        (b"k\0a\0b".to_vec(), "embedded-nul"),
        (vec![b'k', 0, 1, 2, 3], "control-chars"),
        ({
            let mut v = g(90, 21);
            for b in v.iter_mut() {
                *b = b'x';
            }
            v.push(0);
            v.extend_from_slice(b"t");
            v
        }, "long-key"),
    ] {
        let png = grey(&[ck(b"tEXt", d)]);
        drive_both(&format!("tEXt {name}"), &png, 0);
    }

    // zTXt
    let zok = zlib_stored(b"hello world");
    let mk_ztxt = |key: &[u8], cm: u8, body: &[u8]| {
        let mut v = key.to_vec();
        v.push(0);
        v.push(cm);
        v.extend_from_slice(body);
        v
    };
    for (d, name) in [
        (mk_ztxt(b"k", 0, &zok), "ok"),
        (mk_ztxt(b"", 0, &zok), "empty-key"),
        (mk_ztxt(b"k", 1, &zok), "bad-method"),
        (mk_ztxt(b"k", 255, &zok), "method-255"),
        (mk_ztxt(b"k", 0, &zok[..4]), "truncated-lz"),
        (mk_ztxt(b"k", 0, &[0x78, 0x01, 0xff]), "garbage-lz"),
        (mk_ztxt(b"k", 0, &[0x00, 0x01, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]), "bad-cmf"),
        ({
            let mut v = mk_ztxt(b"k", 0, &zok);
            v.extend_from_slice(&[0xaa; 6]);
            v
        }, "extra-compressed"),
        ({
            // keyword longer than 79 bytes
            let mut k = vec![b'x'; 90];
            k.push(0);
            k.push(0);
            k.extend_from_slice(&zok);
            k
        }, "long-key"),
        (b"k\0".to_vec(), "no-lz"),
        (b"key\0\0".to_vec(), "empty-lz"),
    ] {
        let png = grey(&[ck(b"zTXt", d)]);
        drive_both(&format!("zTXt {name}"), &png, 0);
    }

    // iTXt
    let mk_itxt = |key: &[u8], cf: u8, cm: u8, lang: &[u8], tk: &[u8], text: &[u8]| {
        let mut v = key.to_vec();
        v.push(0);
        v.push(cf);
        v.push(cm);
        v.extend_from_slice(lang);
        v.push(0);
        v.extend_from_slice(tk);
        v.push(0);
        v.extend_from_slice(text);
        v
    };
    let zt = zlib_stored(b"compressed text");
    for (d, name) in [
        (mk_itxt(b"k", 0, 0, b"en", b"kk", b"text"), "ok-plain"),
        (mk_itxt(b"k", 1, 0, b"en", b"kk", &zt), "ok-compressed"),
        (mk_itxt(b"k", 2, 0, b"en", b"kk", b"text"), "bad-flag"),
        (mk_itxt(b"k", 1, 1, b"en", b"kk", &zt), "bad-method"),
        (mk_itxt(b"", 0, 0, b"en", b"kk", b"text"), "empty-key"),
        (mk_itxt(b"k", 1, 0, b"en", b"kk", &zt[..5]), "truncated-lz"),
        (mk_itxt(b"k", 1, 0, b"en", b"kk", b""), "compressed-empty"),
        (b"k\0\0\0en".to_vec(), "truncated"),
        ({
            let mut k = vec![b'y'; 85];
            k.extend_from_slice(b"\0\0\0en\0kk\0text");
            k
        }, "long-key"),
    ] {
        let png = grey(&[ck(b"iTXt", d)]);
        drive_both(&format!("iTXt {name}"), &png, 0);
    }

    // chunk cache limit: "no space in chunk cache" (tEXt/zTXt/iTXt/sPLT)
    for n in [1u32, 2, 3] {
        let png = grey(&[
            ck(b"tEXt", b"k1\0v".to_vec()),
            ck(b"zTXt", mk_ztxt(b"k2", 0, &zok)),
            ck(b"iTXt", mk_itxt(b"k3", 0, 0, b"en", b"kk", b"t")),
            ck(b"sPLT", b"s\0\x08\x01\x02\x03\x04\x05\x06".to_vec()),
            ck(b"weRd", vec![1, 2, 3]),
        ]);
        drive(
            &format!("cache_max={n}"),
            &png,
            0,
            &move |c, p, _i| unsafe { (c.set_chunk_cache_max)(p, n) },
        );
        drive(
            &format!("cache_max={n} keep-always"),
            &png,
            0,
            &move |c, p, _i| unsafe {
                (c.set_chunk_cache_max)(p, n);
                (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
            },
        );
    }
}

// ===========================================================================
// 5b. truncation, overrunning lengths, duplicates, tIME ranges
// ===========================================================================

#[test]
fn truncation_and_duplicates() {
    // A chunk whose declared length runs past the end of the datastream: the
    // read callback runs dry (harness longjmp) in both libraries.
    let png = grey(&[]);
    for extra in [1u32, 8, 100, 0x7fff_ffff] {
        let bad = join(&[
            ihdr(4, 2, 8, 0, 0),
            ck(b"gAMA", vec![0, 1, 0, 0]).with_len(4 + extra),
            idat(4, 2, 8, 0, 0),
            iend(),
        ]);
        drive(&format!("TRUNC gAMA len+{extra}"), &bad, 0, &noset);
    }
    // IDAT length overrunning the file
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"IDAT", zlib_stored(&rawrows(4, 2, 8, 0, 0))).with_len(9999),
        iend(),
    ]);
    drive("TRUNC IDAT len 9999", &bad, 1, &noset);

    // Every prefix of a valid stream.
    for n in [9usize, 12, 16, 25, 30, 40, 45, 50, 55] {
        if n < png.len() {
            drive(&format!("TRUNC prefix {n}"), &png[..n], 1, &noset);
        }
    }

    // Duplicate ancillary chunks not covered by the ORD group.
    let exif = {
        let mut v = vec![0x49, 0x49, 0x2a, 0x00];
        v.extend_from_slice(&[8, 0, 0, 0]);
        v
    };
    for (name, data) in [
        (b"eXIf", exif.clone()),
        (b"tIME", vec![0x07, 0xe8, 1, 2, 3, 4, 5]),
        (b"sCAL", b"\x011\x002".to_vec()),
        (b"pCAL", {
            let mut v = b"p\0".to_vec();
            v.extend_from_slice(&0i32.to_be_bytes());
            v.extend_from_slice(&100i32.to_be_bytes());
            v.push(0);
            v.push(2);
            v.extend_from_slice(b"u\0");
            v.extend_from_slice(b"1\0002");
            v
        }),
        (b"iCCP", {
            let mut v = b"i\0\0".to_vec();
            v.extend_from_slice(&zlib_stored(&icc_profile(true, 1)));
            v
        }),
    ] {
        let nm = String::from_utf8_lossy(name).into_owned();
        let dup = grey(&[ck(name, data.clone()), ck(name, data)]);
        drive_both(&format!("DUP {nm}"), &dup, 0);
    }

    // tIME with out-of-range fields (png_set_tIME warns).
    for (tag, d) in [
        ("month0", vec![0x07, 0xe8, 0, 2, 3, 4, 5]),
        ("month13", vec![0x07, 0xe8, 13, 2, 3, 4, 5]),
        ("day0", vec![0x07, 0xe8, 1, 0, 3, 4, 5]),
        ("day32", vec![0x07, 0xe8, 1, 32, 3, 4, 5]),
        ("hour24", vec![0x07, 0xe8, 1, 2, 24, 4, 5]),
        ("min60", vec![0x07, 0xe8, 1, 2, 3, 60, 5]),
        ("sec61", vec![0x07, 0xe8, 1, 2, 3, 4, 61]),
        ("sec60", vec![0x07, 0xe8, 1, 2, 3, 4, 60]),
        ("year0", vec![0, 0, 1, 2, 3, 4, 5]),
    ] {
        let png = grey(&[ck(b"tIME", d)]);
        drive_both(&format!("tIME {tag}"), &png, 0);
    }

    // The same malformed inputs through the progressive reader, one byte at a
    // time (pngpread.c save_buffer handling).
    for (tag, inp) in [
        ("gAMA invalid", grey(&[ck(b"gAMA", 0x8000_0000u32.to_be_bytes().to_vec())])),
        ("sBIT bad length", grey(&[ck(b"sBIT", vec![1, 2])])),
        ("tRNS with alpha", img(6, 8, &[ck(b"tRNS", vec![0, 1])])),
        ("bad crc idat", join(&[ihdr(4, 2, 8, 0, 0), idat(4, 2, 8, 0, 0).bad_crc(), iend()])),
        ("iend data", join(&[ihdr(4, 2, 8, 0, 0), idat(4, 2, 8, 0, 0), ck(b"IEND", vec![1])])),
    ] {
        prog(&format!("PROG1 {tag}"), &inp, 1, usize::MAX, &noset);
        prog(&format!("PROG1 {tag} benign0"), &inp, 1, usize::MAX, &benign_off);
    }
}

// ===========================================================================
// 5c. seeded structural mutations of valid datastreams
// ===========================================================================

/// Dissect a valid PNG and delete / duplicate / reorder / truncate / resize
/// chunks or flip single bits; feed the result to both libraries.
#[test]
fn random_mutations() {
    let exif = {
        let mut v = vec![0x49, 0x49, 0x2a, 0x00];
        v.extend_from_slice(&[8, 0, 0, 0]);
        v
    };
    let bases: Vec<Vec<u8>> = vec![
        grey(&[]),
        grey(&[
            ck(b"gAMA", 45455u32.to_be_bytes().to_vec()),
            ck(b"sBIT", vec![7]),
            ck(b"tEXt", b"key\0value".to_vec()),
            ck(b"eXIf", exif),
        ]),
        palimg(&[
            ck(b"tRNS", g(256, 91)),
            ck(b"bKGD", vec![3]),
            ck(b"hIST", g(512, 92)),
        ]),
        img(6, 16, &[
            ck(b"sRGB", vec![1]),
            ck(b"cHRM", {
                let mut v = Vec::new();
                for x in [31270u32, 32900, 64000, 33000, 30000, 60000, 15000, 6000] {
                    v.extend_from_slice(&x.to_be_bytes());
                }
                v
            }),
            ck(b"pHYs", {
                let mut v = 300u32.to_be_bytes().to_vec();
                v.extend_from_slice(&300u32.to_be_bytes());
                v.push(1);
                v
            }),
        ]),
        {
            let b = Builder::new(9, 9, 4, 3).interlace(1);
            let mut v = vec![ck(b"IHDR", b.ihdr_bytes()), plte(16)];
            v.push(ck(b"IDAT", zlib_stored(&b.raw_rows(0x99))));
            v.push(iend());
            join(&v)
        },
    ];

    for (bi, base) in bases.iter().enumerate() {
        drive(&format!("MUT base{bi}"), base, 1, &noset);
        for seed in 0u64..24 {
            let mut r = Rng::new(0x4d75_7400 + seed * 131 + bi as u64 * 7919);
            let mut v = base.clone();
            let kind = r.below(6);
            match kind {
                0 => {
                    let i = r.below(v.len() as u32) as usize;
                    v[i] ^= 1u8 << r.below(8);
                }
                1 => {
                    let n = 1 + r.below(v.len() as u32) as usize;
                    v.truncate(n);
                }
                2 => {
                    let mut cs = split(&v);
                    if cs.len() > 1 {
                        let k = r.below(cs.len() as u32) as usize;
                        cs.remove(k);
                    }
                    v = join(&cs);
                }
                3 => {
                    let mut cs = split(&v);
                    let k = r.below(cs.len() as u32) as usize;
                    let c = cs[k].clone();
                    cs.insert(k, c);
                    v = join(&cs);
                }
                4 => {
                    let mut cs = split(&v);
                    if cs.len() > 2 {
                        let a = r.below(cs.len() as u32) as usize;
                        let b = r.below(cs.len() as u32) as usize;
                        cs.swap(a, b);
                    }
                    v = join(&cs);
                }
                _ => {
                    let mut cs = split(&v);
                    let k = r.below(cs.len() as u32) as usize;
                    let nl = r.below(64) as usize;
                    cs[k].data.resize(nl, 0xa5);
                    v = join(&cs);
                }
            }
            drive(&format!("MUT b{bi} s{seed} k{kind}"), &v, 1, &noset);
        }
    }
}

// ===========================================================================
// 6. iCCP
// ===========================================================================

/// A minimal ICC profile that passes every check in `png_icc_check_*`.
fn icc_profile(gray: bool, tags: u32) -> Vec<u8> {
    let len = 132 + 12 * tags;
    let mut p = vec![0u8; len as usize];
    p[0..4].copy_from_slice(&len.to_be_bytes());
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(if gray { b"GRAY" } else { b"RGB " });
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    // rendering intent 0
    p[68..80].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]);
    p[128..132].copy_from_slice(&tags.to_be_bytes());
    for t in 0..tags as usize {
        let o = 132 + 12 * t;
        p[o..o + 4].copy_from_slice(b"desc");
        p[o + 4..o + 8].copy_from_slice(&len.to_be_bytes()); // start == length, size 0
        p[o + 8..o + 12].copy_from_slice(&0u32.to_be_bytes());
    }
    p
}

fn iccp_chunk(key: &[u8], method: u8, profile: &[u8], trailing: usize) -> Chunk {
    let mut d = key.to_vec();
    d.push(0);
    d.push(method);
    d.extend_from_slice(&zlib_stored(profile));
    d.extend(std::iter::repeat(0xa5).take(trailing));
    ck(b"iCCP", d)
}

#[test]
fn iccp_chunks() {
    let prof = icc_profile(true, 1);

    // A valid profile.
    let png = grey(&[iccp_chunk(b"icc", 0, &prof, 0)]);
    drive_both("iCCP ok", &png, 0);

    // pngrutil.c:1455 -- "extra compressed data" (chunk warning): trailing
    // bytes that png_inflate_read never needs to consume.
    let png = grey(&[iccp_chunk(b"icc", 0, &prof, 4096)]);
    drive_both("iCCP extra compressed data", &png, 0);

    // bad keyword / compression method / too short
    let png = grey(&[iccp_chunk(b"", 0, &prof, 0)]);
    drive_both("iCCP empty keyword", &png, 0);
    let png = grey(&[iccp_chunk(&vec![b'k'; 90], 0, &prof, 0)]);
    drive_both("iCCP long keyword", &png, 0);
    let png = grey(&[iccp_chunk(b"icc", 1, &prof, 0)]);
    drive_both("iCCP bad method", &png, 0);
    let png = grey(&[ck(b"iCCP", b"icc\0\0abcdefghij".to_vec())]);
    drive_both("iCCP short lz", &png, 0);
    let mut d = b"icc\0\0".to_vec();
    d.extend_from_slice(&zlib_stored(&prof)[..8]);
    let png = grey(&[ck(b"iCCP", d)]);
    drive_both("iCCP truncated lz", &png, 0);

    // profile header rejections
    let mut p = prof.clone();
    p[0..4].copy_from_slice(&99u32.to_be_bytes());
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP length mismatch", &png, 0);
    let png = grey(&[iccp_chunk(b"i", 0, &prof[..100], 0)]);
    drive_both("iCCP too short", &png, 0);
    let mut p = prof.clone();
    p[36..40].copy_from_slice(b"xxxx");
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP bad signature", &png, 0);
    let mut p = prof.clone();
    p[128..132].copy_from_slice(&1000u32.to_be_bytes());
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP tag count too large", &png, 0);
    let mut p = prof.clone();
    p[64..68].copy_from_slice(&5u32.to_be_bytes());
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP intent 5", &png, 0);
    let mut p = prof.clone();
    p[64..68].copy_from_slice(&0x1_0000u32.to_be_bytes());
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP intent huge", &png, 0);
    let mut p = prof.clone();
    p[68] = 1;
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP not D50", &png, 0);
    // colour space vs image colour type
    let png = grey(&[iccp_chunk(b"i", 0, &icc_profile(false, 1), 0)]);
    drive_both("iCCP RGB on grey", &png, 0);
    let png = img(2, 8, &[iccp_chunk(b"i", 0, &icc_profile(true, 1), 0)]);
    drive_both("iCCP GRAY on rgb", &png, 0);
    let mut p = prof.clone();
    p[16..20].copy_from_slice(b"CMYK");
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP bad colour space", &png, 0);
    for cls in [b"abst", b"link", b"nmcl", b"zzzz"] {
        let mut p = prof.clone();
        p[12..16].copy_from_slice(cls);
        let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
        drive_both(
            &format!("iCCP class {}", String::from_utf8_lossy(cls)),
            &png,
            0,
        );
    }
    let mut p = prof.clone();
    p[20..24].copy_from_slice(b"zzzz");
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP bad PCS", &png, 0);
    // tag table checks
    let mut p = icc_profile(true, 1);
    p[136..140].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP tag outside", &png, 0);
    let mut p = icc_profile(true, 1);
    p[136..140].copy_from_slice(&1u32.to_be_bytes());
    p[140..144].copy_from_slice(&0u32.to_be_bytes());
    let png = grey(&[iccp_chunk(b"i", 0, &p, 0)]);
    drive_both("iCCP tag misaligned", &png, 0);
    // profile longer than the chunk-malloc limit
    let png = grey(&[iccp_chunk(b"i", 0, &prof, 0)]);
    drive("iCCP profile too long", &png, 0, &|c, p, _i| unsafe {
        (c.set_chunk_malloc_max)(p, 140)
    });
}

// ===========================================================================
// 7. the zlib/inflate error paths of the IDAT stream
// ===========================================================================

#[test]
fn zlib_stream() {
    let raw = rawrows(4, 2, 8, 0, 0);
    let good = zlib_stored(&raw);

    let with_idat = |z: Vec<u8>| join(&[ihdr(4, 2, 8, 0, 0), ck(b"IDAT", z), iend()]);

    // invalid window size (CINFO > 7) -- png_zlib_inflate
    let mut z = good.clone();
    z[0] = 0x88;
    drive_both("ZLIB cinfo=8", &with_idat(z), 1);
    // incorrect header check (FCHECK wrong)
    let mut z = good.clone();
    z[1] = 0x02;
    drive_both("ZLIB bad fcheck", &with_idat(z), 1);
    // CM != 8
    let mut z = good.clone();
    z[0] = 0x79;
    z[1] = 0x00;
    drive_both("ZLIB cm=9", &with_idat(z), 1);
    // FDICT set -> Z_NEED_DICT
    let mut z = good.clone();
    z[0] = 0x78;
    z[1] = 0x20;
    drive_both("ZLIB fdict", &with_idat(z), 1);
    // wrong ADLER32
    let mut z = good.clone();
    let n = z.len();
    z[n - 1] ^= 0xff;
    drive_both("ZLIB bad adler", &with_idat(z), 1);
    // truncated stream (no adler32)
    drive_both("ZLIB truncated tail", &with_idat(good[..n - 4].to_vec()), 1);
    // truncated in the middle of the data
    drive_both("ZLIB truncated mid", &with_idat(good[..8].to_vec()), 1);
    // extra bytes after the end of the stream
    let mut z = good.clone();
    z.extend_from_slice(&[0x5a; 7]);
    drive_both("ZLIB extra after end", &with_idat(z), 1);
    // corrupt stored-block length
    let mut z = good.clone();
    z[3] = 0xff;
    drive_both("ZLIB bad stored len", &with_idat(z), 1);
    // empty IDAT
    drive_both("ZLIB empty idat", &with_idat(Vec::new()), 1);
    // decompresses to too few bytes
    drive_both("ZLIB short data", &with_idat(zlib_stored(&raw[..6])), 1);
    // decompresses to too many bytes
    let mut big = raw.clone();
    big.extend_from_slice(&raw);
    drive_both("ZLIB long data", &with_idat(zlib_stored(&big)), 1);
    // no IDAT data at all after the header: "Not enough image data"
    let png = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"IDAT", good[..3].to_vec()),
        ck(b"tEXt", b"k\0v".to_vec()),
        iend(),
    ]);
    drive_both("ZLIB non-IDAT mid stream", &png, 1);
    // IDAT split into many chunks with a hole
    let mut chunks = vec![ihdr(4, 2, 8, 0, 0)];
    for part in good.chunks(4) {
        chunks.push(ck(b"IDAT", part.to_vec()));
    }
    chunks.push(iend());
    drive_both("ZLIB split idat", &join(&chunks), 1);
}

// ===========================================================================
// 8. rows, filters and row-init errors
// ===========================================================================

#[test]
fn interlace_and_rows() {
    // pngread.c:456 -- "bad adaptive filter value"
    for f in [5u8, 6, 255] {
        let mut raw = rawrows(4, 2, 8, 0, 0);
        raw[0] = f;
        let png = join(&[ihdr(4, 2, 8, 0, 0), ck(b"IDAT", zlib_stored(&raw)), iend()]);
        drive_both(&format!("ROW filter={f} row0"), &png, 1);
        let mut raw = rawrows(4, 2, 8, 0, 0);
        raw[5] = f;
        let png = join(&[ihdr(4, 2, 8, 0, 0), ck(b"IDAT", zlib_stored(&raw)), iend()]);
        drive_both(&format!("ROW filter={f} row1"), &png, 1);
    }

    // interlaced stream whose later passes are missing
    let full = rawrows(8, 8, 8, 0, 1);
    for cut in [1usize, 10, 30, 60] {
        let png = join(&[
            ihdr(8, 8, 8, 0, 1),
            ck(b"IDAT", zlib_stored(&full[..cut.min(full.len())])),
            iend(),
        ]);
        drive_both(&format!("ROW interlace short cut={cut}"), &png, 1);
    }
    // bad filter in a later interlace pass
    let mut raw = full.clone();
    let k = raw.len() - 3;
    raw[k] = 7;
    let png = join(&[ihdr(8, 8, 8, 0, 1), ck(b"IDAT", zlib_stored(&raw)), iend()]);
    drive_both("ROW interlace bad filter", &png, 1);

    // pngread.c:444 -- "Invalid attempt to read row data": png_read_row
    // without png_read_info.
    let png = grey(&[]);
    let mut buf = vec![0u8; STRIDE];
    let bp = buf.as_mut_ptr();
    diff("ROW read_row without read_info", |lib| {
        with_read(lib, &png, &mut |c, p, _i| unsafe {
            (c.read_row)(p, bp, std::ptr::null_mut());
            log("read_row returned".to_string());
        })
    });

    // pngread.c:191/214 -- duplicate png_read_update_info / start_read_image
    for benign in [0, 1] {
        for order in 0..4 {
            diff(
                &format!("ROW dup-init order={order} benign={benign}"),
                |lib| {
                    with_read(lib, &png, &mut |c, p, i| unsafe {
                        (c.set_benign_errors)(p, benign);
                        (c.read_info)(p, i);
                        match order {
                            0 => {
                                (c.read_update_info)(p, i);
                                (c.read_update_info)(p, i);
                            }
                            1 => {
                                (c.start_read_image)(p);
                                (c.start_read_image)(p);
                            }
                            2 => {
                                (c.read_update_info)(p, i);
                                (c.start_read_image)(p);
                            }
                            _ => {
                                (c.start_read_image)(p);
                                (c.read_update_info)(p, i);
                            }
                        }
                        log("init done".to_string());
                        log_all_info(c, p, i);
                    })
                },
            );
        }
    }

    // pngrutil.c:3478 -- "invalid user transform pixel depth"
    let png = join(&[
        ihdr(8, 8, 8, 0, 1),
        ck(b"IDAT", zlib_stored(&rawrows(8, 8, 8, 0, 1))),
        iend(),
    ]);
    for (d, ch, name) in [(3u8, 3u8, "3x3"), (1, 9, "1x9"), (8, 1, "8x1")] {
        drive(
            &format!("ROW user transform depth {name}"),
            &png,
            1,
            &move |c, p, _i| unsafe {
                (c.set_read_user_transform_fn)(p, u_transform as Cb);
                (c.set_user_transform_info)(p, std::ptr::null_mut(), d as c_int, ch as c_int);
            },
        );
    }
}

unsafe extern "C" fn u_transform(_png: *mut c_void, _row_info: *mut c_void, _row: *mut u8) {
    log("USER_TRANSFORM".to_string());
}

// ---------------------------------------------------------------------------
// row_info damaging user transforms: these reach the pixel-depth consistency
// checks in png_read_row / png_push_process_row / png_combine_row.  Note that
// png_do_read_transformations keeps whatever the callback stored in row_info
// when png_set_user_transform_info() was *not* called.
// ---------------------------------------------------------------------------

thread_local! {
    static UT_N: Cell<u32> = const { Cell::new(0) };
}

fn ut_reset() {
    UT_N.with(|c| c.set(0));
}

unsafe fn ut_log(tag: &str, ri: *mut PngRowInfo) {
    let r = &*ri;
    log(format!(
        "{tag} w={} rb={} ct={} bd={} ch={} pd={}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
}

/// pixel_depth 64 on an 8-bit grey image: exceeds maximum_pixel_depth.
unsafe extern "C" fn ut_big(_png: *mut c_void, ri: *mut PngRowInfo, _row: *mut u8) {
    ut_log("UT_BIG", ri);
    (*ri).bit_depth = 16;
    (*ri).channels = 4;
}

/// pixel_depth 4 on an 8-bit grey image: consistent from row to row but it
/// does not match png_struct::info_rowbytes.
unsafe extern "C" fn ut_small(_png: *mut c_void, ri: *mut PngRowInfo, _row: *mut u8) {
    ut_log("UT_SMALL", ri);
    (*ri).bit_depth = 4;
}

/// The first row keeps the file depth, later rows do not.
unsafe extern "C" fn ut_alt(_png: *mut c_void, ri: *mut PngRowInfo, _row: *mut u8) {
    let n = UT_N.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v
    });
    ut_log(&format!("UT_ALT{n}"), ri);
    if n > 0 {
        (*ri).bit_depth = 4;
    }
}

#[test]
fn row_depth_checks() {
    let png = grey(&[]); // 4x2, grey, 8 bit: rowbytes 4, maximum_pixel_depth 8

    // pngread.c:489 -- "sequential row overflow"
    drive("ROWD seq overflow", &png, 1, &|c, p, _i| unsafe {
        (c.set_read_user_transform_fn)(p, ut_big as Cb)
    });
    // pngread.c:493 -- "internal sequential row size calculation error"
    drive("ROWD seq size calc", &png, 1, &|c, p, _i| unsafe {
        ut_reset();
        (c.set_read_user_transform_fn)(p, ut_alt as Cb)
    });
    // pngrutil.c:3251 -- "internal row size calculation error"
    drive("ROWD combine size calc", &png, 1, &|c, p, _i| unsafe {
        (c.set_read_user_transform_fn)(p, ut_small as Cb)
    });

    // pngpread.c:647 -- "progressive row overflow"
    prog_m("ROWD prog overflow", &png, 1 << 20, usize::MAX, 3, &|c, p, _i| unsafe {
        (c.set_read_user_transform_fn)(p, ut_big as Cb)
    });
    // pngpread.c:651 -- "internal progressive row size calculation error"
    prog_m("ROWD prog size calc", &png, 1 << 20, usize::MAX, 3, &|c, p, _i| unsafe {
        ut_reset();
        (c.set_read_user_transform_fn)(p, ut_alt as Cb)
    });
    // pngrutil.c:3243 -- "internal row logic error":
    // png_progressive_combine_row before any row has been transformed.
    prog_m("ROWD combine before row", &png, 1 << 20, usize::MAX, 5, &noset);
    prog_m("ROWD combine before row (no update)", &png, 1 << 20, usize::MAX, 4, &noset);

    // The same transforms with png_set_user_transform_info in agreement.
    drive("ROWD user info consistent", &png, 1, &|c, p, _i| unsafe {
        (c.set_read_user_transform_fn)(p, ut_small as Cb);
        (c.set_user_transform_info)(p, std::ptr::null_mut(), 4, 1);
    });
}

// pngrutil.c:1577 -- "No space in chunk cache for sPLT": sPLT must be the
// chunk that decrements user_chunk_cache_max to 1.
#[test]
fn splt_chunk_cache() {
    let png = grey(&[ck(b"sPLT", b"s\0\x08\x01\x02\x03\x04\x05\x06".to_vec())]);
    for n in [1u32, 2, 3] {
        drive(
            &format!("sPLT-only cache_max={n}"),
            &png,
            0,
            &move |c, p, _i| unsafe { (c.set_chunk_cache_max)(p, n) },
        );
    }
    // two sPLT chunks with room for exactly one
    let png2 = grey(&[
        ck(b"sPLT", b"a\0\x08\x01\x02\x03\x04\x05\x06".to_vec()),
        ck(b"sPLT", b"b\0\x08\x01\x02\x03\x04\x05\x06".to_vec()),
    ]);
    for n in [2u32, 3, 4] {
        drive(
            &format!("sPLT-two cache_max={n}"),
            &png2,
            0,
            &move |c, p, _i| unsafe { (c.set_chunk_cache_max)(p, n) },
        );
    }
}

// ===========================================================================
// 9. user limits
// ===========================================================================

#[test]
fn limits() {
    let png = grey(&[]);
    // png.c:2017/2039 -- width/height exceeding the user limit
    for (w, h) in [(1u32, 1u32), (3, 1), (1, 1000), (4, 1), (3, 2)] {
        drive(
            &format!("LIM user_limits {w}x{h}"),
            &png,
            0,
            &move |c, p, _i| unsafe { (c.set_user_limits)(p, w, h) },
        );
    }

    // pngread.c:881 -- "Image is too high to process with png_read_png()"
    let tall = join(&[ihdr(4, 0x2000_0000, 8, 0, 0), idat(4, 1, 8, 0, 0), iend()]);
    diff("LIM read_png too high", |lib| {
        with_read(lib, &tall, &mut |c, p, i| unsafe {
            (c.set_user_limits)(p, 0x7fff_ffff, 0x7fff_ffff);
            (c.read_png)(p, i, PNG_TRANSFORM_IDENTITY, std::ptr::null_mut());
            log("read_png returned".to_string());
            log_all_info(c, p, i);
        })
    });

    // pngrutil.c:2745 -- "unknown chunk exceeds memory limits"
    let png = grey(&[ck(b"weRd", g(64, 31))]);
    for lim in [1usize, 16, 63, 64] {
        drive(
            &format!("LIM unknown malloc_max={lim}"),
            &png,
            0,
            &move |c, p, _i| unsafe {
                (c.set_chunk_malloc_max)(p, lim);
                (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
            },
        );
    }
    // the same with a user callback (png_cache_unknown_chunk from that path)
    drive("LIM unknown cb malloc_max", &png, 0, &|c, p, _i| unsafe {
        (c.set_chunk_malloc_max)(p, 8);
        (c.set_read_user_chunk_fn)(p, std::ptr::null_mut(), uc_ok as Cb);
    });

    // chunk_malloc_max = 0 means "unlimited"
    drive("LIM malloc_max=0", &png, 0, &|c, p, _i| unsafe {
        (c.set_chunk_malloc_max)(p, 0);
        (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
    });
}

// ===========================================================================
// 10. unknown chunk handling
// ===========================================================================

unsafe extern "C" fn uc_err(_png: *mut c_void, _chunk: *mut c_void) -> c_int {
    log("USER_CHUNK -1".to_string());
    -1
}

unsafe extern "C" fn uc_skip(_png: *mut c_void, _chunk: *mut c_void) -> c_int {
    log("USER_CHUNK 0".to_string());
    0
}

unsafe extern "C" fn uc_ok(_png: *mut c_void, _chunk: *mut c_void) -> c_int {
    log("USER_CHUNK 1".to_string());
    1
}

#[test]
fn unknown_chunks() {
    let anc = grey(&[ck(b"weRd", vec![1, 2, 3, 4])]);
    let crit = grey(&[ck(b"WERD", vec![1, 2, 3, 4])]);

    // pngrutil.c:2812 -- "error in user chunk"
    for (inp, tag) in [(&anc, "anc"), (&crit, "crit")] {
        drive(
            &format!("UNK user cb -1 {tag}"),
            inp,
            0,
            &|c, p, _i| unsafe { (c.set_read_user_chunk_fn)(p, std::ptr::null_mut(), uc_err as Cb) },
        );
        // pngrutil.c:2832/2833 -- "Saving unknown chunk:" + app warning
        drive(
            &format!("UNK user cb 0 {tag}"),
            inp,
            0,
            &|c, p, _i| unsafe {
                (c.set_read_user_chunk_fn)(p, std::ptr::null_mut(), uc_skip as Cb)
            },
        );
        drive(
            &format!("UNK user cb 0 {tag} benign"),
            inp,
            0,
            &|c, p, _i| unsafe {
                (c.set_benign_errors)(p, 1);
                (c.set_read_user_chunk_fn)(p, std::ptr::null_mut(), uc_skip as Cb)
            },
        );
        drive(
            &format!("UNK user cb 0 {tag} if-safe"),
            inp,
            0,
            &|c, p, _i| unsafe {
                (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_IF_SAFE, std::ptr::null(), 0);
                (c.set_read_user_chunk_fn)(p, std::ptr::null_mut(), uc_skip as Cb)
            },
        );
        drive(
            &format!("UNK user cb 1 {tag}"),
            inp,
            0,
            &|c, p, _i| unsafe { (c.set_read_user_chunk_fn)(p, std::ptr::null_mut(), uc_ok as Cb) },
        );
    }

    // pngrutil.c:2912 -- "no space in chunk cache" from the store path
    for n in [1u32, 2, 3] {
        drive(
            &format!("UNK store cache={n}"),
            &anc,
            0,
            &move |c, p, _i| unsafe {
                (c.set_chunk_cache_max)(p, n);
                (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
            },
        );
    }

    // png_set_keep_unknown_chunks with an out-of-range keep
    for k in [-2i32, -1, 4, 99] {
        drive(
            &format!("UNK keep={k}"),
            &anc,
            0,
            &move |c, p, _i| unsafe {
                (c.set_keep_unknown_chunks)(p, k, b"weRd\0".as_ptr(), 1);
                log("set_keep returned".to_string());
            },
        );
        drive(
            &format!("UNK keep={k} benign"),
            &anc,
            0,
            &move |c, p, _i| unsafe {
                (c.set_benign_errors)(p, 1);
                (c.set_keep_unknown_chunks)(p, k, b"weRd\0".as_ptr(), 1);
                log("set_keep returned".to_string());
            },
        );
    }
    // NULL chunk list with a non-default keep
    drive("UNK keep null list", &anc, 0, &|c, p, _i| unsafe {
        (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_NEVER, std::ptr::null(), 1);
        log("set_keep returned".to_string());
    });

    // known chunks forced to unknown handling
    drive("UNK gAMA as unknown", &grey(&[ck(b"gAMA", 45455u32.to_be_bytes().to_vec())]), 0,
        &|c, p, _i| unsafe {
            (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, b"gAMA\0".as_ptr(), 1);
        });
    drive("UNK IHDR as unknown", &grey(&[]), 0, &|c, p, _i| unsafe {
        (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_NEVER, b"IHDR\0".as_ptr(), 1);
    });
    drive("UNK PLTE as unknown", &palimg(&[]), 0, &|c, p, _i| unsafe {
        (c.set_keep_unknown_chunks)(p, PNG_HANDLE_CHUNK_ALWAYS, b"PLTE\0".as_ptr(), 1);
    });
}

// ===========================================================================
// 11. progressive reader (pngpread.c)
// ===========================================================================

#[test]
fn progressive_errors() {
    let good = grey(&[]);
    for step in [1usize, 3, 1 << 20] {
        prog(&format!("PROG ok step={step}"), &good, step, usize::MAX, &noset);
    }

    // pngpread.c:166 / 169 -- signature errors
    let mut bad = good.clone();
    bad[2] = b'X';
    prog("PROG not a png (1 byte)", &bad, 1, usize::MAX, &noset);
    prog("PROG not a png (all)", &bad, 1 << 20, usize::MAX, &noset);
    let mut bad = good.clone();
    bad[0..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0a, 0x1a, 0x0a, 0x0a]);
    prog("PROG ascii conversion", &bad, 1 << 20, usize::MAX, &noset);
    prog("PROG ascii conversion b1", &bad, 1, usize::MAX, &noset);

    // pngpread.c:213 -- "Missing IHDR before IDAT"
    let bad = join(&[idat(4, 2, 8, 0, 0), iend()]);
    prog("PROG missing IHDR", &bad, 1, usize::MAX, &noset);
    // pngpread.c:217 -- "Missing PLTE before IDAT"
    let bad = join(&[ihdr(4, 2, 8, 3, 0), idat(4, 2, 8, 3, 0), iend()]);
    prog("PROG missing PLTE", &bad, 3, usize::MAX, &noset);
    // pngpread.c:243 -- "Invalid IHDR length"
    let mut d = Builder::new(4, 2, 8, 0).ihdr_bytes();
    d.push(7);
    let bad = join(&[ck(b"IHDR", d), idat(4, 2, 8, 0, 0), iend()]);
    prog("PROG bad IHDR length", &bad, 5, usize::MAX, &noset);

    // pngpread.c:229 -- "Too many IDATs found"
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        idat(4, 2, 8, 0, 0),
        ck(b"tEXt", b"k\0v".to_vec()),
        ck(b"IDAT", vec![1, 2, 3]),
        iend(),
    ]);
    prog("PROG too many idats", &bad, 7, usize::MAX, &noset);
    prog("PROG too many idats b1", &bad, 1, usize::MAX, &benign_off);

    // pngpread.c:425 -- "Not enough compressed data"
    let raw = rawrows(4, 2, 8, 0, 0);
    let z = zlib_stored(&raw);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"IDAT", z[..z.len() - 6].to_vec()),
        ck(b"tEXt", b"k\0v".to_vec()),
        iend(),
    ]);
    prog("PROG not enough compressed data", &bad, 4, usize::MAX, &noset);

    // pngpread.c:560/562 -- ADLER32 mismatch / "Decompression error in IDAT"
    // (the stream ends while rows are still outstanding)
    let short = zlib_stored(&raw[..5]);
    let mut z2 = short.clone();
    let n = z2.len();
    z2[n - 1] ^= 0xff;
    let bad = join(&[ihdr(4, 4, 8, 0, 0), ck(b"IDAT", z2), iend()]);
    prog("PROG adler mismatch", &bad, 1 << 20, usize::MAX, &noset);
    prog("PROG adler mismatch benign0", &bad, 1 << 20, usize::MAX, &benign_off);
    let mut z3 = zlib_stored(&raw);
    z3[0] = 0x78;
    z3[1] = 0x20; // FDICT -> Z_NEED_DICT
    let bad = join(&[ihdr(4, 4, 8, 0, 0), ck(b"IDAT", z3), iend()]);
    prog("PROG need dict", &bad, 1 << 20, usize::MAX, &noset);
    let mut z4 = zlib_stored(&raw);
    z4[0] = 0x88; // invalid window size
    let bad = join(&[ihdr(4, 4, 8, 0, 0), ck(b"IDAT", z4), iend()]);
    prog("PROG invalid window", &bad, 1 << 20, usize::MAX, &noset);

    // pngpread.c:555 -- "Truncated compressed data in IDAT"
    let mut z5 = zlib_stored(&raw);
    let n = z5.len();
    z5[n - 2] ^= 0xff;
    let bad = join(&[ihdr(4, 2, 8, 0, 0), ck(b"IDAT", z5), iend()]);
    prog("PROG truncated compressed", &bad, 1 << 20, usize::MAX, &noset);

    // pngpread.c:580 -- "Extra compressed data in IDAT"
    let mut big = raw.clone();
    big.extend_from_slice(&raw);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"IDAT", zlib_stored(&big)),
        iend(),
    ]);
    prog("PROG extra compressed data", &bad, 1 << 20, usize::MAX, &noset);

    // pngpread.c:605 -- "Extra compression data in IDAT"
    let mut z6 = zlib_stored(&raw);
    z6.extend_from_slice(&[0x33; 9]);
    let bad = join(&[ihdr(4, 2, 8, 0, 0), ck(b"IDAT", z6), iend()]);
    prog("PROG extra compression data", &bad, 1 << 20, usize::MAX, &noset);

    // pngpread.c:627 -- "bad adaptive filter value"
    let mut raw2 = rawrows(4, 2, 8, 0, 0);
    raw2[0] = 9;
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"IDAT", zlib_stored(&raw2)),
        iend(),
    ]);
    prog("PROG bad filter", &bad, 1 << 20, usize::MAX, &noset);

    // pngpread.c:372 -- "Insufficient memory for save_buffer" (the first save
    // allocates save+current+256 == 257 bytes)
    prog("PROG save_buffer oom", &good, 1, 257, &noset);

    // chunk errors reached through the progressive path
    let bad = grey(&[ck(b"sRGB", vec![9])]);
    prog("PROG sRGB invalid", &bad, 2, usize::MAX, &noset);
    prog("PROG sRGB invalid benign0", &bad, 2, usize::MAX, &benign_off);
    let bad = grey(&[ck(b"TEST", vec![1])]);
    prog("PROG unknown critical", &bad, 2, usize::MAX, &noset);
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"gAMA", 1u32.to_be_bytes().to_vec()).bad_crc(),
        idat(4, 2, 8, 0, 0),
        iend(),
    ]);
    prog("PROG bad crc", &bad, 3, usize::MAX, &noset);
    prog("PROG bad crc quiet", &bad, 3, usize::MAX, &|c, p, _i| unsafe {
        (c.set_crc_action)(p, PNG_CRC_QUIET_USE, PNG_CRC_QUIET_USE)
    });

    // interlaced progressive read
    let il = join(&[
        ihdr(8, 8, 8, 0, 1),
        ck(b"IDAT", zlib_stored(&rawrows(8, 8, 8, 0, 1))),
        iend(),
    ]);
    prog("PROG interlaced", &il, 11, usize::MAX, &noset);
    let mut r = rawrows(8, 8, 8, 0, 1);
    let k = r.len() - 2;
    r[k] = 6;
    let il = join(&[ihdr(8, 8, 8, 0, 1), ck(b"IDAT", zlib_stored(&r)), iend()]);
    prog("PROG interlaced bad filter", &il, 1 << 20, usize::MAX, &noset);

    // pngpread.c:99 -- png_process_data_skip
    diff("PROG data_skip/pause", |lib| {
        session_reset(Vec::new());
        let c = Core::new(lib);
        let mut data = good.clone();
        let dp = data.as_mut_ptr();
        let rc = protected(|| unsafe {
            let p = (c.create_read)(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
            );
            if p.is_null() {
                return;
            }
            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
            let i = (c.create_info)(p);
            (c.set_progressive_read_fn)(
                p,
                std::ptr::null_mut(),
                p_info as Cb,
                p_row as Cb,
                p_end as Cb,
            );
            (c.process_data)(p, i, dp, 20);
            log(format!("pause0={}", (c.process_data_pause)(p, 0)));
            log(format!("pause1={}", (c.process_data_pause)(p, 1)));
            log(format!("skip={}", (c.process_data_skip)(p)));
            log("after skip".to_string());
        });
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

// ===========================================================================
// 12. allocation failures on the read path
// ===========================================================================

#[test]
fn oom_paths() {
    // pngrutil.c:2010 -- eXIf "out of memory" (png_read_buffer(length))
    let png = grey(&[ck(b"eXIf", {
        let mut v = vec![0x49, 0x49, 0x2a, 0x00];
        v.extend(g(995, 41));
        v
    })]);
    drive_mem("OOM eXIf", &png, 999, 0, &noset);

    // pngrutil.c:2482 -- zTXt "out of memory" (png_read_buffer(length))
    let mut zt = b"k\0\0".to_vec();
    zt.extend_from_slice(&zlib_stored(b"some text"));
    let l = zt.len();
    let png = grey(&[ck(b"zTXt", zt)]);
    drive_mem("OOM zTXt", &png, l, 0, &noset);

    // pngrutil.c:1646 -- "sPLT chunk requires too much memory"
    // (1000 entries of 10 bytes each)
    let mut sp = b"s\0\x10".to_vec();
    sp.extend(g(10 * 1000, 43));
    let png = grey(&[ck(b"sPLT", sp)]);
    drive_mem("OOM sPLT entries", &png, 10 * 1000, 0, &noset);

    // pngrutil.c:2437 -- tEXt: png_set_text_2 cannot allocate the text
    let mut tx = b"k\0".to_vec();
    tx.extend(std::iter::repeat(b'a').take(500));
    let png = grey(&[ck(b"tEXt", tx)]);
    drive_mem("OOM tEXt set_text", &png, 505, 0, &noset);

    // pngrutil.c:4222 -- IDAT "out of memory" (the read buffer for the chunk)
    let raw = rawrows(4, 2, 8, 0, 0);
    let mut z = zlib_stored(&raw);
    z.resize(777, 0);
    let png = join(&[ihdr(4, 2, 8, 0, 0), ck(b"IDAT", z), iend()]);
    drive_mem("OOM IDAT buffer", &png, 777, 1, &noset);

    // A run with no failure at all, to prove the allocator itself is neutral.
    drive_mem("OOM none", &grey(&[]), 0xdead_beef, 1, &noset);
}

// ===========================================================================
// 13. FFI boundary: NULL pointers and out-of-range arguments
// ===========================================================================

#[test]
fn ffi_null_and_range() {
    let png = grey(&[]);
    let mut buf = vec![0u8; STRIDE];
    let bp = buf.as_mut_ptr();
    let mut rows: Vec<*mut u8> = vec![bp, bp];
    let rp = rows.as_mut_ptr();

    // Each NULL-argument case is its own run so that a crash in one cannot
    // hide the others.
    for which in 0u8..17 {
        diff(&format!("FFI null case {which}"), |lib| {
            with_read(lib, &png, &mut |c, p, i| unsafe {
                let n: Png = std::ptr::null_mut();
                match which {
                    0 => (c.read_info)(n, i),
                    1 => (c.read_info)(p, std::ptr::null_mut()),
                    2 => (c.read_update_info)(n, i),
                    3 => (c.start_read_image)(n),
                    4 => (c.read_row)(n, bp, std::ptr::null_mut()),
                    5 => (c.read_rows)(n, rp, std::ptr::null_mut(), 1),
                    6 => (c.read_image)(n, rp),
                    7 => (c.read_end)(n, i),
                    8 => (c.read_png)(n, i, 0, std::ptr::null_mut()),
                    9 => (c.read_png)(p, std::ptr::null_mut(), 0, std::ptr::null_mut()),
                    10 => (c.process_data)(n, i, bp, 1),
                    11 => (c.process_data)(p, std::ptr::null_mut(), bp, 1),
                    12 => log(format!("pause_null={}", (c.process_data_pause)(n, 1))),
                    // NOTE: png_process_data_skip(NULL) is not tested: the C
                    // implementation (pngpread.c:92) has no NULL check and
                    // dereferences png_ptr inside png_app_warning, so it
                    // segfaults in the reference library as well.
                    // (png_get_io_state has no NULL check either.)
                    13 => log(format!(
                        "err_ptr_null={}",
                        (c.get_error_ptr)(n).is_null() as u8
                    )),
                    14 => log(format!(
                        "prog_ptr_null={}",
                        (c.get_progressive_ptr)(n).is_null() as u8
                    )),
                    15 => {
                        (c.set_crc_action)(n, 1, 1);
                        (c.set_sig_bytes)(n, 3);
                        (c.set_keep_unknown_chunks)(n, 1, std::ptr::null(), 0);
                    }
                    _ => {
                        (c.set_user_limits)(n, 1, 1);
                        (c.set_chunk_cache_max)(n, 1);
                        (c.set_chunk_malloc_max)(n, 1);
                    }
                }
                log(format!("null case {which} survived"));
                log_all_info(c, p, i);
            })
        });
    }

    // pngrutil.c:46 -- png_get_uint_31 with a value >= 2^31
    for v in [0u32, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        diff(&format!("FFI get_uint_31 {v:#x}"), |lib| {
            with_read(lib, &png, &mut |c, p, _i| unsafe {
                let b = v.to_be_bytes();
                log(format!("uint31={}", (c.get_uint_31)(p, b.as_ptr())));
            })
        });
    }

    // png_set_crc_action with out-of-range actions
    for a in [-1i32, 6, 99, 1000] {
        drive(
            &format!("FFI crc_action {a}"),
            &png,
            1,
            &move |c, p, _i| unsafe { (c.set_crc_action)(p, a, a) },
        );
    }
}

// ===========================================================================
// 14. the I/O layer (pngrio.c)
// ===========================================================================

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

#[test]
fn io_functions() {
    let png = grey(&[]);

    // pngrio.c:39 -- "Call to NULL read function" (png_set_write_fn clears
    // read_data_fn and warns from pngwio.c).
    diff("IO set_write_fn on read struct", |lib| {
        with_read(lib, &png, &mut |c, p, i| unsafe {
            (c.set_write_fn)(
                p,
                std::ptr::null_mut(),
                cb_write as Cb,
                cb_flush as Cb,
            );
            log("write fn set".to_string());
            (c.read_info)(p, i);
            log("read_info returned".to_string());
        })
    });

    // pngrio.c:109 -- the reverse order warns from png_set_read_fn.
    diff("IO set_read_fn after write_fn", |lib| {
        with_read(lib, &png, &mut |c, p, i| unsafe {
            (c.set_write_fn)(
                p,
                std::ptr::null_mut(),
                cb_write as Cb,
                cb_flush as Cb,
            );
            (c.set_read_fn)(p, std::ptr::null_mut(), cb_read as Cb);
            log("read fn re-set".to_string());
            (c.read_info)(p, i);
            log_all_info(c, p, i);
        })
    });

    // pngrio.c:62 -- "Read Error" from png_default_read_data: an empty stdio
    // stream with libpng's own read function.
    diff("IO default_read_data on /dev/null", |lib| {
        session_reset(Vec::new());
        let c = Core::new(lib);
        let rc = protected(|| unsafe {
            let fp = fopen(
                b"/dev/null\0".as_ptr() as *const c_char,
                b"rb\0".as_ptr() as *const c_char,
            );
            log(format!("fopen={}", (!fp.is_null()) as u8));
            if fp.is_null() {
                return;
            }
            let p = (c.create_read)(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
            );
            if p.is_null() {
                return;
            }
            (c.set_longjmp)(p, shim().longjmp_ptr, shim().jmp_buf_size);
            let i = (c.create_info)(p);
            // io_ptr = FILE*, read_data_fn = png_default_read_data
            (c.set_read_fn)(p, fp, std::ptr::null_mut());
            (c.read_info)(p, i);
            log("read_info returned".to_string());
            fclose(fp);
        });
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

// ===========================================================================
// 15. simplified read API (png_image_*)
// ===========================================================================

type BeginMem = unsafe extern "C" fn(*mut PngImage, *const c_void, usize) -> c_int;
type BeginFile = unsafe extern "C" fn(*mut PngImage, *const c_char) -> c_int;
type BeginStdio = unsafe extern "C" fn(*mut PngImage, *mut c_void) -> c_int;
type FinishRead =
    unsafe extern "C" fn(*mut PngImage, *const c_void, *mut c_void, i32, *mut c_void) -> c_int;
type ImgFree = unsafe extern "C" fn(*mut PngImage);

fn log_img(tag: &str, im: &PngImage, rc: c_int) {
    log(format!(
        "IMAGE_ERR({}) {tag} rc={rc} w={} h={} fmt={:#x} flags={:#x} cmap={} woe={} opaque={}",
        im.msg(),
        im.width,
        im.height,
        im.format,
        im.flags,
        im.colormap_entries,
        im.warning_or_error,
        (!im.opaque.is_null()) as u8
    ));
}

/// Run a simplified-API scenario against both libraries.
fn simg(label: &str, png: &[u8], f: &dyn Fn(&Lib, *mut PngImage, *const u8, usize)) {
    let data = png.to_vec();
    let ptr = data.as_ptr();
    let len = data.len();
    diff(label, |lib| {
        session_reset(Vec::new());
        let mut im = PngImage::default();
        let ip = &mut im as *mut PngImage;
        let rc = protected(|| f(lib, ip, ptr, len));
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}

#[test]
fn simplified_args() {
    let grey8 = grey(&[]);

    // Argument validation of png_image_begin_read_from_memory.
    simg("SIMG memory null", &grey8, &|lib, ip, _p, _l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let rc = begin(ip, std::ptr::null(), 0);
        log_img("null-memory", &*ip, rc);
    });
    simg("SIMG size 0", &grey8, &|lib, ip, p, _l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let rc = begin(ip, p as *const c_void, 0);
        log_img("size0", &*ip, rc);
    });
    simg("SIMG image null", &grey8, &|lib, _ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let rc = begin(std::ptr::null_mut(), p as *const c_void, l);
        log(format!("IMAGE_ERR(<image-null>) rc={rc}"));
    });
    simg("SIMG bad version", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        (*ip).version = 99;
        let rc = begin(ip, p as *const c_void, l);
        log_img("bad-version", &*ip, rc);
    });

    // pngread.c:1172 -- "png_image_read: opaque pointer not NULL".  The fake
    // control block is all zeroes, so png_image_free_function returns
    // immediately (png_ptr == NULL) in both libraries.
    let mut fake = [0u8; 512];
    let fp = fake.as_mut_ptr() as *mut c_void;
    simg("SIMG opaque not null", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        (*ip).opaque = fp;
        let rc = begin(ip, p as *const c_void, l);
        log_img("opaque", &*ip, rc);
    });

    // png_image_finish_read argument validation.
    let mut out = vec![0u8; 1 << 16];
    let op = out.as_mut_ptr() as *mut c_void;
    let mut cmap = vec![0u8; 1 << 12];
    let cp = cmap.as_mut_ptr() as *mut c_void;
    simg("SIMG finish null buffer", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        let rc = finish(ip, std::ptr::null(), std::ptr::null_mut(), 0, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });
    simg("SIMG finish bad version", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        (*ip).version = 7;
        let rc = finish(ip, std::ptr::null(), op, 0, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });
    simg("SIMG finish no colormap", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        (*ip).format |= PNG_FORMAT_FLAG_COLORMAP;
        let rc = finish(ip, std::ptr::null(), op, 0, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });
    simg("SIMG finish zero cmap entries", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        (*ip).format |= PNG_FORMAT_FLAG_COLORMAP;
        (*ip).colormap_entries = 0;
        let rc2 = finish(ip, std::ptr::null(), op, 0, cp);
        log_img("finish", &*ip, rc2);
        log(format!("begin_rc={rc}"));
    });
    simg("SIMG finish image too large", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        (*ip).height = 8;
        let rc = finish(ip, std::ptr::null(), op, 0x4000_0000, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });
    simg("SIMG finish row_stride too large", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        (*ip).width = 0x2000_0000;
        (*ip).format = PNG_FORMAT_RGBA;
        let rc = finish(ip, std::ptr::null(), op, 0, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });
    simg("SIMG finish negative stride", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        let rc = finish(ip, std::ptr::null(), op, -1, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });

}

// pngread.c:1355/1360/1393/1398 -- argument validation of the stdio entry
// points of the simplified API.
#[test]
fn simplified_stdio_args() {
    let grey8 = grey(&[]);

    simg("SIMG from_file NULL name", &grey8, &|lib, ip, _p, _l| unsafe {
        let f: BeginFile = lib.f("png_image_begin_read_from_file");
        let rc = f(ip, std::ptr::null());
        log_img("file-null", &*ip, rc);
    });
    simg("SIMG from_file bad version", &grey8, &|lib, ip, _p, _l| unsafe {
        let f: BeginFile = lib.f("png_image_begin_read_from_file");
        (*ip).version = 42;
        let rc = f(ip, b"/nonexistent/err_stream.png\0".as_ptr() as *const c_char);
        log_img("file-version", &*ip, rc);
    });
    simg("SIMG from_file NULL image", &grey8, &|lib, _ip, _p, _l| unsafe {
        let f: BeginFile = lib.f("png_image_begin_read_from_file");
        let rc = f(std::ptr::null_mut(), b"/dev/null\0".as_ptr() as *const c_char);
        log(format!("IMAGE_ERR(<image-null>) file rc={rc}"));
    });
    simg("SIMG from_file enoent", &grey8, &|lib, ip, _p, _l| unsafe {
        let f: BeginFile = lib.f("png_image_begin_read_from_file");
        let rc = f(ip, b"/nonexistent/err_stream.png\0".as_ptr() as *const c_char);
        log_img("file-enoent", &*ip, rc);
    });
    simg("SIMG from_stdio NULL file", &grey8, &|lib, ip, _p, _l| unsafe {
        let f: BeginStdio = lib.f("png_image_begin_read_from_stdio");
        let rc = f(ip, std::ptr::null_mut());
        log_img("stdio-null", &*ip, rc);
    });
    // version != PNG_IMAGE_VERSION is checked before the FILE* is used, so the
    // bogus (never dereferenced) pointer below is safe.
    simg("SIMG from_stdio bad version", &grey8, &|lib, ip, _p, _l| unsafe {
        let f: BeginStdio = lib.f("png_image_begin_read_from_stdio");
        (*ip).version = 3;
        let rc = f(ip, 1usize as *mut c_void);
        log_img("stdio-version", &*ip, rc);
    });
    simg("SIMG from_stdio NULL image", &grey8, &|lib, _ip, _p, _l| unsafe {
        let f: BeginStdio = lib.f("png_image_begin_read_from_stdio");
        let rc = f(std::ptr::null_mut(), std::ptr::null_mut());
        log(format!("IMAGE_ERR(<image-null>) stdio rc={rc}"));
    });
}

// pngread.c:3925 -- "png_read_image: unsupported transformation": an
// undefined bit in image->format is a change libpng cannot implement.
#[test]
fn simplified_bad_format() {
    let grey8 = grey(&[]);
    let mut out = vec![0u8; 1 << 16];
    let op = out.as_mut_ptr() as *mut c_void;
    for bit in [0x80u32, 0x100, 0x8000_0000] {
        simg(
            &format!("SIMG format bit {bit:#x}"),
            &grey8,
            &move |lib, ip, p, l| unsafe {
                let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
                let finish: FinishRead = lib.f("png_image_finish_read");
                let rc = begin(ip, p as *const c_void, l);
                log_img("begin", &*ip, rc);
                (*ip).format |= bit;
                std::ptr::write_bytes(op as *mut u8, 0, 1 << 16);
                let rc = finish(ip, std::ptr::null(), op, 0, std::ptr::null_mut());
                log_img("finish", &*ip, rc);
            },
        );
    }
}

#[test]
fn simplified_plain_read() {
    let grey8 = grey(&[]);
    let mut out = vec![0u8; 1 << 16];
    let op = out.as_mut_ptr() as *mut c_void;

    // A plain successful simplified read, for a baseline.
    simg("SIMG plain read", &grey8, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let free: ImgFree = lib.f("png_image_free");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        std::ptr::write_bytes(op as *mut u8, 0, 1 << 16);
        let rc = finish(ip, std::ptr::null(), op, 0, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
        log(format!(
            "pixels={}",
            hex(std::slice::from_raw_parts(op as *const u8, 16))
        ));
        free(ip);
    });

}

/// One colour-map scenario: read `input`, then override `image->format` and
/// `image->colormap_entries` before `png_image_finish_read`.
/// `entries == 0` keeps the value libpng computed.
fn cmap_case(tag: &str, input: &[u8], fmt: u32, entries: u32, need_back: bool) {
    let mut out = vec![0u8; 1 << 16];
    let op = out.as_mut_ptr() as *mut c_void;
    let mut cmap = vec![0u8; 1 << 12];
    let cp = cmap.as_mut_ptr() as *mut c_void;
    let back: [u8; 3] = [1, 2, 3];
    let backp = back.as_ptr() as *const c_void;
    simg(
        &format!("SIMG cmap {tag}"),
        input,
        &move |lib, ip, p, l| unsafe {
            let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
            let finish: FinishRead = lib.f("png_image_finish_read");
            let rc = begin(ip, p as *const c_void, l);
            log_img("begin", &*ip, rc);
            (*ip).format = fmt;
            if entries != 0 {
                (*ip).colormap_entries = entries;
            }
            let b = if need_back { backp } else { std::ptr::null() };
            std::ptr::write_bytes(op as *mut u8, 0, 1 << 16);
            std::ptr::write_bytes(cp as *mut u8, 0, 1 << 12);
            let rc = finish(ip, b, op, 0, cp);
            log_img("finish", &*ip, rc);
            log(format!(
                "cmap={} pixels={}",
                hex(std::slice::from_raw_parts(cp as *const u8, 24)),
                hex(std::slice::from_raw_parts(op as *const u8, 16))
            ));
        },
    );
}

const CMAP: u32 = PNG_FORMAT_FLAG_COLORMAP;

// pngread.c:1997 -- "background color must be supplied to remove
// alpha/transparency"
#[test]
fn simplified_cmap_no_background() {
    cmap_case("no-background", &img(4, 8, &[]), CMAP, 0, false);
}

// pngread.c:2056..2695 -- one test per "too few entries" colour-map branch, so
// that a divergence in one cannot hide the others.
#[test]
fn simplified_cmap_gray8() {
    cmap_case("gray8", &img(0, 8, &[]), CMAP, 1, false);
}

#[test]
fn simplified_cmap_gray16() {
    cmap_case(
        "gray16",
        &img(0, 16, &[]),
        CMAP | PNG_FORMAT_FLAG_LINEAR,
        1,
        false,
    );
}

#[test]
fn simplified_cmap_gray_plus_alpha() {
    cmap_case(
        "gray+alpha",
        &img(4, 8, &[]),
        CMAP | PNG_FORMAT_FLAG_ALPHA,
        1,
        false,
    );
}

#[test]
fn simplified_cmap_gray_minus_alpha() {
    cmap_case("gray-alpha", &img(4, 8, &[]), CMAP, 1, true);
}

#[test]
fn simplified_cmap_ga_alpha() {
    cmap_case(
        "ga-alpha",
        &img(4, 8, &[]),
        CMAP | PNG_FORMAT_FLAG_COLOR,
        1,
        true,
    );
}

#[test]
fn simplified_cmap_rgb_ga() {
    cmap_case(
        "rgb[ga]",
        &img(6, 8, &[]),
        CMAP | PNG_FORMAT_FLAG_ALPHA,
        1,
        false,
    );
}

#[test]
fn simplified_cmap_rgb_gray() {
    cmap_case("rgb[gray]", &img(2, 8, &[]), CMAP, 1, false);
}

#[test]
fn simplified_cmap_rgb_plus_alpha() {
    cmap_case(
        "rgb+alpha",
        &img(6, 8, &[]),
        CMAP | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA,
        1,
        false,
    );
}

#[test]
fn simplified_cmap_rgb_minus_alpha() {
    cmap_case(
        "rgb-alpha",
        &img(6, 8, &[]),
        CMAP | PNG_FORMAT_FLAG_COLOR,
        1,
        true,
    );
}

#[test]
fn simplified_cmap_rgb() {
    cmap_case(
        "rgb",
        &img(2, 8, &[]),
        CMAP | PNG_FORMAT_FLAG_COLOR,
        1,
        false,
    );
}

#[test]
fn simplified_cmap_palette() {
    let pal = join(&[
        ihdr(4, 2, 8, 3, 0),
        plte(8),
        ck(b"IDAT", zlib_stored(&vec![0u8; 10])),
        iend(),
    ]);
    cmap_case("palette", &pal, CMAP | PNG_FORMAT_FLAG_COLOR, 1, false);
}

/// The same colour-map scenarios with a colour-map big enough to succeed.
#[test]
fn simplified_cmap_ok() {
    cmap_case("ok gray8", &img(0, 8, &[]), CMAP, 0, false);
    cmap_case("ok rgb", &img(2, 8, &[]), CMAP | PNG_FORMAT_FLAG_COLOR, 0, false);
    cmap_case("ok palette", &palimg(&[]), CMAP | PNG_FORMAT_FLAG_COLOR, 0, false);
}

/// Malformed streams read through the simplified API: the C library reports
/// the error through `image->message` (`png_safe_execute` catches the
/// `png_error` with `setjmp`) and returns 0.
#[test]
fn simplified_malformed() {
    let grey8 = grey(&[]);
    let mut out = vec![0u8; 1 << 16];
    let op = out.as_mut_ptr() as *mut c_void;

    // pngread.c:1427 -- "read beyond end of data"
    for cut in [8usize, 20, 33] {
        let short = grey8[..cut.min(grey8.len())].to_vec();
        simg(
            &format!("SIMG truncated {cut}"),
            &short,
            &|lib, ip, p, l| unsafe {
                let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
                let rc = begin(ip, p as *const c_void, l);
                log_img("truncated", &*ip, rc);
            },
        );
    }

    let bad = join(&[ihdr(4, 2, 8, 3, 0), idat(4, 2, 8, 3, 0), iend()]);
    simg("SIMG missing PLTE", &bad, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
    });
    let mut bad = grey8.clone();
    bad[1] = b'X';
    simg("SIMG bad signature", &bad, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
    });
    let bad = join(&[
        ihdr(4, 2, 8, 0, 0),
        ck(b"IDAT", zlib_stored(&vec![0u8; 4])),
        iend(),
    ]);
    simg("SIMG short idat", &bad, &|lib, ip, p, l| unsafe {
        let begin: BeginMem = lib.f("png_image_begin_read_from_memory");
        let finish: FinishRead = lib.f("png_image_finish_read");
        let rc = begin(ip, p as *const c_void, l);
        log_img("begin", &*ip, rc);
        std::ptr::write_bytes(op as *mut u8, 0, 1 << 16);
        let rc = finish(ip, std::ptr::null(), op, 0, std::ptr::null_mut());
        log_img("finish", &*ip, rc);
    });
}

//! Differential verification of the READ side of libpng (CONFIGS.md rows
//! 38-51, 54-56, 64, 65, 70).
//!
//! All PNG inputs are produced once with the **C** library (ground truth) and
//! then decoded with both the C and the Rust `.so`; every getter, every decoded
//! row byte, every callback invocation, the message log and the number of bytes
//! consumed from the input stream are compared.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;

use common::api::*;
use common::*;
use libloading::Library;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr};
use std::ptr;
use std::sync::{Mutex, OnceLock};

// The floating-point cHRM getter is the one accessor row 54 needs that
// `tests/common/api.rs` does not declare.
crate::decl_api! {
    fn png_get_cHRM(pp: png_structp, ip: png_infop, wx: *mut f64, wy: *mut f64,
                    rx: *mut f64, ry: *mut f64, gx: *mut f64, gy: *mut f64,
                    bx: *mut f64, by: *mut f64) -> png_uint_32;
}

// ---------------------------------------------------------------------------
// The reference C `libpng.so` was linked without `-lm`, so `floor`/`pow` are
// undefined in it.  Pull `libm` into the global symbol scope with RTLD_GLOBAL
// before touching either library so that the lazy PLT bindings resolve.
static LIBM: OnceLock<()> = OnceLock::new();

fn ensure_libm() {
    LIBM.get_or_init(|| {
        use libloading::os::unix as u;
        let l = unsafe { u::Library::open(Some("libm.so.6"), u::RTLD_NOW | u::RTLD_GLOBAL) }
            .expect("dlopen libm.so.6");
        std::mem::forget(l);
    });
}

/// [`common::libs`] with the `libm` fixup applied.
fn L() -> &'static Libs {
    ensure_libm();
    libs()
}

// ===========================================================================
// snapshot type + comparison
// ===========================================================================

#[derive(Clone, Debug, Default, PartialEq)]
struct Snap {
    vals: Vec<(String, i64)>,
    rows: Vec<Vec<u8>>,
    blobs: Vec<(String, Vec<u8>)>,
    notes: Vec<String>,
}

impl Snap {
    fn v(&mut self, t: &str, k: &str, x: i64) {
        self.vals.push((format!("{t}.{k}"), x));
    }
    fn vf(&mut self, t: &str, k: &str, x: f32) {
        self.vals.push((format!("{t}.{k}"), x.to_bits() as i64));
    }
    fn note(&mut self, s: String) {
        self.notes.push(s);
    }
}

fn first_val_diff(a: &[(String, i64)], b: &[(String, i64)]) -> Option<String> {
    for i in 0..a.len().max(b.len()) {
        match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) if x == y => continue,
            (x, y) => {
                return Some(format!(
                    "entry #{i}: C={:?} Rust={:?} (C has {} entries, Rust {})",
                    x,
                    y,
                    a.len(),
                    b.len()
                ))
            }
        }
    }
    None
}

fn bytes_diff(label: &str, x: &[u8], y: &[u8]) -> Option<String> {
    if x == y {
        return None;
    }
    let off = x
        .iter()
        .zip(y.iter())
        .position(|(p, q)| p != q)
        .unwrap_or(x.len().min(y.len()));
    let e1 = (off + 16).min(x.len());
    let e2 = (off + 16).min(y.len());
    Some(format!(
        "{label} differs at byte {off} (len C={} Rust={})\n     C  : {}\n     Rust: {}",
        x.len(),
        y.len(),
        hex(&x[off.min(x.len())..e1]),
        hex(&y[off.min(y.len())..e2]),
    ))
}

fn assert_snap(c: &Run<Snap>, r: &Run<Snap>, label: &str) {
    let cl: Vec<String> = c.log.iter().map(|m| m.to_string()).collect();
    let rl: Vec<String> = r.log.iter().map(|m| m.to_string()).collect();
    assert_eq!(cl, rl, "{label}: message log differs (C first)");
    match (&c.out, &r.out) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            if let Some(d) = first_val_diff(&a.vals, &b.vals) {
                panic!("{label}: getter values differ -> {d}\n  C log={cl:?}");
            }
            assert_eq!(a.notes, b.notes, "{label}: notes differ (C first)");
            assert_eq!(
                a.rows.len(),
                b.rows.len(),
                "{label}: number of recorded rows differs (C first)"
            );
            for (i, (x, y)) in a.rows.iter().zip(b.rows.iter()).enumerate() {
                if let Some(d) = bytes_diff(&format!("row[{i}]"), x, y) {
                    panic!("{label}: {d}\n  C log={cl:?}");
                }
            }
            assert_eq!(
                a.blobs.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
                b.blobs.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
                "{label}: blob names differ"
            );
            for (x, y) in a.blobs.iter().zip(b.blobs.iter()) {
                if let Some(d) = bytes_diff(&x.0, &x.1, &y.1) {
                    panic!("{label}: {d}\n  C log={cl:?}");
                }
            }
        }
        _ => panic!(
            "{label}: one library errored and the other did not (C errored={})\n  C log={cl:?}\n  Rust log={rl:?}",
            c.out.is_none()
        ),
    }
}

// ===========================================================================
// helpers: geometry, CRC, chunk surgery
// ===========================================================================

fn nchan(ct: c_int) -> usize {
    match ct {
        PNG_COLOR_TYPE_GRAY => 1,
        PNG_COLOR_TYPE_RGB => 3,
        PNG_COLOR_TYPE_PALETTE => 1,
        PNG_COLOR_TYPE_GRAY_ALPHA => 2,
        PNG_COLOR_TYPE_RGB_ALPHA => 4,
        _ => 1,
    }
}

fn rb(w: u32, ct: c_int, bd: c_int) -> usize {
    (w as usize * nchan(ct) * bd as usize + 7) / 8
}

const PASS_START: [u32; 7] = [0, 4, 0, 2, 0, 1, 0];
const PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];

fn pass_cols(w: u32, pass: usize) -> u32 {
    if w <= PASS_START[pass] {
        0
    } else {
        (w - PASS_START[pass] + PASS_INC[pass] - 1) / PASS_INC[pass]
    }
}

/// PNG_ROWBYTES(pixel_depth, width)
fn png_rowbytes(pixel_depth: usize, width: u32) -> usize {
    if pixel_depth >= 8 {
        width as usize * (pixel_depth / 8)
    } else {
        (width as usize * pixel_depth + 7) / 8
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn mk_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(12 + data.len());
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut c = name.to_vec();
    c.extend_from_slice(data);
    v.extend_from_slice(&crc32(&c).to_be_bytes());
    v
}

fn chunk_name_u32(n: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*n)
}

/// walk the chunk list: returns (offset-of-length-field, data-length, name)
fn walk(png: &[u8]) -> Vec<(usize, usize, [u8; 4])> {
    let mut out = Vec::new();
    let mut p = 8usize;
    while p + 8 <= png.len() {
        let len = u32::from_be_bytes([png[p], png[p + 1], png[p + 2], png[p + 3]]) as usize;
        let nm = [png[p + 4], png[p + 5], png[p + 6], png[p + 7]];
        out.push((p, len, nm));
        p += 12 + len;
    }
    out
}

fn insert_chunk_after(png: &[u8], after: &[u8; 4], ch: &[u8]) -> Vec<u8> {
    let w = walk(png);
    let mut at = png.len();
    for (off, len, nm) in &w {
        if nm == after {
            at = off + 12 + len;
            break;
        }
    }
    let mut v = png[..at].to_vec();
    v.extend_from_slice(ch);
    v.extend_from_slice(&png[at..]);
    v
}

fn insert_chunk_before_iend(png: &[u8], ch: &[u8]) -> Vec<u8> {
    let w = walk(png);
    let mut at = png.len();
    for (off, _len, nm) in &w {
        if nm == b"IEND" {
            at = *off;
            break;
        }
    }
    let mut v = png[..at].to_vec();
    v.extend_from_slice(ch);
    v.extend_from_slice(&png[at..]);
    v
}

/// flip one bit of the stored CRC of the first chunk with the given name
fn corrupt_crc(png: &[u8], name: &[u8; 4]) -> Vec<u8> {
    let mut v = png.to_vec();
    for (off, len, nm) in walk(png) {
        if &nm == name {
            let crcpos = off + 8 + len;
            v[crcpos + 3] ^= 0x01;
            return v;
        }
    }
    panic!("chunk {} not found", String::from_utf8_lossy(name));
}

fn fix_crc(png: &mut [u8], off: usize, len: usize) {
    let c = crc32(&png[off + 4..off + 8 + len]);
    png[off + 8 + len..off + 12 + len].copy_from_slice(&c.to_be_bytes());
}

/// set the IHDR filter method byte (MNG intrapixel differencing == 64)
fn patch_ihdr_filter(png: &[u8], filter: u8) -> Vec<u8> {
    let mut v = png.to_vec();
    // signature(8) len(4) name(4) => data at 16; filter method is data[11]
    v[16 + 11] = filter;
    fix_crc(&mut v, 8, 13);
    v
}

unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".to_string()
    } else {
        String::from_utf8_lossy(CStr::from_ptr(p).to_bytes()).into_owned()
    }
}

// ===========================================================================
// PNG generation with the C library
// ===========================================================================

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
struct Extras {
    /// `png_set_filter` mask used by the *writer* (0 = leave libpng's default).
    /// Forcing a single filter is how the read-side `png_read_filter_row`
    /// variants get exercised deterministically.
    filter: c_int,
    trns: bool,
    gama: bool,
    bkgd: bool,
    sbit: bool,
    hist: bool,
    text: bool,
    rich: bool,
    srgb: bool,
    iccp: bool,
    /// cICP / cLLI / mDCV (PNG v3 HDR chunks)
    hdr: bool,
    bad_index: bool,
}

fn gen_rows(w: u32, h: u32, ct: c_int, bd: c_int, seed: u64) -> Vec<Vec<u8>> {
    let mut r = Rng::new(seed);
    (0..h.max(1)).map(|_| r.bytes(rb(w, ct, bd))).collect()
}

fn icc_profile() -> Vec<u8> {
    // 128-byte header + 4-byte tag count (== 0)  => 132 bytes
    let mut p = vec![0u8; 132];
    p[0..4].copy_from_slice(&132u32.to_be_bytes()); // profile size
    p[8] = 2; // version major
    p[9] = 0x10;
    p[12..16].copy_from_slice(b"mntr"); // device class
    p[16..20].copy_from_slice(b"GRAY"); // data colour space (patched below)
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[64..68].copy_from_slice(&0u32.to_be_bytes()); // rendering intent
    p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes()); // D50 X
    p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes()); // D50 Y
    p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes()); // D50 Z
    p[128..132].copy_from_slice(&0u32.to_be_bytes()); // tag count
    p
}

fn icc_profile_for(ct: c_int) -> Vec<u8> {
    let mut p = icc_profile();
    if ct & PNG_COLOR_MASK_COLOR != 0 {
        p[16..20].copy_from_slice(b"RGB ");
    }
    p
}

static CACHE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn cached(key: String, f: impl FnOnce() -> Vec<u8>) -> Vec<u8> {
    let m = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = m.lock().unwrap().get(&key) {
        return v.clone();
    }
    let v = f();
    m.lock().unwrap().insert(key, v.clone());
    v
}

fn make_png(ct: c_int, bd: c_int, il: c_int, w: u32, h: u32, seed: u64) -> Vec<u8> {
    make_png_ex(ct, bd, il, w, h, seed, Extras::default())
}

fn make_png_ex(
    ct: c_int,
    bd: c_int,
    il: c_int,
    w: u32,
    h: u32,
    seed: u64,
    ex: Extras,
) -> Vec<u8> {
    let key = format!("{ct}/{bd}/{il}/{w}/{h}/{seed}/{ex:?}");
    cached(key, || unsafe { build_png(ct, bd, il, w, h, seed, ex) })
}

unsafe fn build_png(
    ct: c_int,
    bd: c_int,
    il: c_int,
    w: u32,
    h: u32,
    seed: u64,
    ex: Extras,
) -> Vec<u8> {
    let l = &L().c;
    let mut rows = gen_rows(w, h, ct, bd, seed);
    if ex.bad_index {
        for r in rows.iter_mut() {
            for b in r.iter_mut() {
                *b = 3;
            }
        }
    }
    sink_reset();
    let pp = new_writer(l);
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    if ex.bad_index {
        png_set_check_for_invalid_index(l, pp, 0);
    }
    if ex.filter != 0 {
        png_set_filter(l, pp, PNG_FILTER_TYPE_BASE, ex.filter);
    }
    png_set_IHDR(
        l,
        pp,
        ip,
        w,
        h,
        bd,
        ct,
        il,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );

    let npal: usize = if ex.bad_index {
        2
    } else if bd >= 8 {
        256
    } else {
        1usize << bd
    };
    let mut pal: Vec<png_color> = Vec::new();
    if ct == PNG_COLOR_TYPE_PALETTE {
        let mut r = Rng::new(seed ^ 0xa5a5_5a5a);
        for _ in 0..npal {
            pal.push(png_color {
                red: r.u8(),
                green: r.u8(),
                blue: r.u8(),
            });
        }
        png_set_PLTE(l, pp, ip, pal.as_ptr(), npal as c_int);
    }

    if ex.gama || ex.rich {
        png_set_gAMA_fixed(l, pp, ip, 45455);
    }
    if ex.rich {
        png_set_cHRM_fixed(l, pp, ip, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
    }
    if ex.srgb {
        png_set_sRGB(l, pp, ip, 1);
    }
    if ex.iccp {
        let prof = icc_profile_for(ct);
        let nm = cs("t");
        png_set_iCCP(
            l,
            pp,
            ip,
            nm.as_ptr(),
            PNG_COMPRESSION_TYPE_BASE,
            prof.as_ptr(),
            prof.len() as png_uint_32,
        );
    }
    if ex.sbit || ex.rich {
        let d = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd as u8 };
        let sb = png_color_8 {
            red: d,
            green: d,
            blue: d,
            gray: d,
            alpha: d,
        };
        png_set_sBIT(l, pp, ip, &sb);
    }
    if ex.trns || ex.rich {
        match ct {
            PNG_COLOR_TYPE_PALETTE => {
                let n = npal.min(4);
                let t: Vec<u8> = (0..n).map(|i| (i * 37) as u8).collect();
                png_set_tRNS(l, pp, ip, t.as_ptr(), n as c_int, ptr::null());
            }
            PNG_COLOR_TYPE_GRAY => {
                let maxv = if bd >= 16 { 0xffffu32 } else { (1u32 << bd) - 1 };
                let tc = png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: (maxv / 2) as u16,
                };
                png_set_tRNS(l, pp, ip, ptr::null(), 0, &tc);
            }
            PNG_COLOR_TYPE_RGB => {
                let maxv = if bd >= 16 { 0xffffu32 } else { (1u32 << bd) - 1 };
                let tc = png_color_16 {
                    index: 0,
                    red: (maxv / 2) as u16,
                    green: (maxv / 3) as u16,
                    blue: (maxv / 5) as u16,
                    gray: 0,
                };
                png_set_tRNS(l, pp, ip, ptr::null(), 0, &tc);
            }
            _ => {}
        }
    }
    if ex.bkgd || ex.rich {
        let maxv = if ct == PNG_COLOR_TYPE_PALETTE {
            (npal - 1) as u32
        } else if bd >= 16 {
            0xffff
        } else {
            (1u32 << bd) - 1
        };
        let bg = if ct == PNG_COLOR_TYPE_PALETTE {
            png_color_16 {
                index: (maxv / 2) as u8,
                red: 0,
                green: 0,
                blue: 0,
                gray: 0,
            }
        } else {
            png_color_16 {
                index: 0,
                red: (maxv / 3) as u16,
                green: (maxv / 4) as u16,
                blue: (maxv / 5) as u16,
                gray: (maxv / 2) as u16,
            }
        };
        png_set_bKGD(l, pp, ip, &bg);
    }
    if (ex.hist || ex.rich) && ct == PNG_COLOR_TYPE_PALETTE {
        let hist: Vec<u16> = (0..npal).map(|i| ((i * 7 + 1) & 0xffff) as u16).collect();
        png_set_hIST(l, pp, ip, hist.as_ptr());
    }
    if ex.rich {
        png_set_oFFs(l, pp, ip, -12, 34, 0);
        png_set_pHYs(l, pp, ip, 3000, 4000, 1);
        png_set_sCAL_fixed(l, pp, ip, 1, 250000, 500000);
        let p0 = cs("1.0");
        let p1 = cs("2.5");
        let mut params: Vec<png_charp> = vec![p0.as_ptr() as png_charp, p1.as_ptr() as png_charp];
        let purpose = cs("cal");
        let units = cs("m");
        png_set_pCAL(
            l,
            pp,
            ip,
            purpose.as_ptr(),
            -5,
            77,
            0,
            2,
            units.as_ptr(),
            params.as_mut_ptr(),
        );
        let t = png_time {
            year: 2001,
            month: 2,
            day: 3,
            hour: 4,
            minute: 5,
            second: 6,
        };
        png_set_tIME(l, pp, ip, &t);
        let mut exif: Vec<u8> = b"MM\0*\0\0\0\x08".to_vec();
        png_set_eXIf_1(l, pp, ip, exif.len() as png_uint_32, exif.as_mut_ptr());
        // sPLT
        let sname = cs("sp1");
        let mut sent = vec![
            png_sPLT_entry {
                red: 1,
                green: 2,
                blue: 3,
                alpha: 4,
                frequency: 5,
            },
            png_sPLT_entry {
                red: 6,
                green: 7,
                blue: 8,
                alpha: 9,
                frequency: 10,
            },
        ];
        let sp = png_sPLT_t {
            name: sname.as_ptr() as png_charp,
            depth: 8,
            entries: sent.as_mut_ptr(),
            nentries: 2,
        };
        png_set_sPLT(l, pp, ip, &sp, 1);
    }
    if ex.hdr {
        png_set_cICP(l, pp, ip, 9, 16, 0, 1);
        png_set_cLLI_fixed(l, pp, ip, 1000 * 10000, 400 * 10000);
        png_set_mDCV_fixed(
            l, pp, ip, 34000, 16000, 13250, 34500, 7500, 3000, 15635, 16450, 10000000, 500,
        );
    }
    if ex.text || ex.rich {
        let k1 = cs("Title");
        let t1 = cs("hello world");
        let k2 = cs("Author");
        let t2 = cs("compressed text payload compressed text payload");
        let txt = vec![
            png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: k1.as_ptr() as png_charp,
                text: t1.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            },
            png_text {
                compression: PNG_TEXT_COMPRESSION_zTXt,
                key: k2.as_ptr() as png_charp,
                text: t2.as_ptr() as png_charp,
                text_length: 0,
                itxt_length: 0,
                lang: ptr::null_mut(),
                lang_key: ptr::null_mut(),
            },
        ];
        png_set_text(l, pp, ip, txt.as_ptr(), 2);
    }
    if ex.rich {
        png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
        let mut d1 = b"unknown-payload".to_vec();
        let unk = [
            png_unknown_chunk {
                name: *b"unSf\0",
                data: d1.as_mut_ptr(),
                size: d1.len(),
                location: PNG_HAVE_IHDR as png_byte,
            },
        ];
        png_set_unknown_chunks(l, pp, ip, unk.as_ptr(), 1);
    }

    png_write_info(l, pp, ip);
    let mut rp: Vec<png_bytep> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    png_write_image(l, pp, rp.as_mut_ptr());
    png_write_end(l, pp, ip);
    let mut p = pp;
    let mut i = ip;
    png_destroy_write_struct(l, &mut p, &mut i);
    let (data, _) = sink_take();
    assert!(!data.is_empty(), "C writer produced nothing");
    data
}

// ===========================================================================
// thread-local recorders
// ===========================================================================

thread_local! {
    static CURLIB: Cell<*const Library> = const { Cell::new(ptr::null()) };
    /// colour type as it appears in IHDR, captured before any call to
    /// `png_read_update_info` can rewrite `info_ptr->color_type`
    static FILE_CT: Cell<c_int> = const { Cell::new(-1) };
    static UT: RefCell<Vec<(png_row_info, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    static ST: RefCell<Vec<(png_uint_32, c_int)>> = const { RefCell::new(Vec::new()) };
    static IOREC: RefCell<Vec<(png_uint_32, png_uint_32, usize)>> = const { RefCell::new(Vec::new()) };
    static QPAL: RefCell<Vec<png_color>> = const { RefCell::new(Vec::new()) };
    static UCHUNK: RefCell<Vec<(String, Vec<u8>, png_byte)>> = const { RefCell::new(Vec::new()) };
    static UCHUNK_RET: Cell<c_int> = const { Cell::new(0) };
}

unsafe fn curlib() -> &'static Library {
    let p = CURLIB.with(|c| c.get());
    assert!(!p.is_null(), "CURLIB not set");
    &*p
}

fn reset_recorders() {
    UT.with(|x| x.borrow_mut().clear());
    ST.with(|x| x.borrow_mut().clear());
    IOREC.with(|x| x.borrow_mut().clear());
    QPAL.with(|x| x.borrow_mut().clear());
    UCHUNK.with(|x| x.borrow_mut().clear());
}

fn drain_recorders(s: &mut Snap) {
    UT.with(|x| {
        for (i, (ri, before)) in x.borrow().iter().enumerate() {
            s.note(format!(
                "ut[{i}] w={} rb={} ct={} bd={} ch={} pd={}",
                ri.width, ri.rowbytes, ri.color_type, ri.bit_depth, ri.channels, ri.pixel_depth
            ));
            s.blobs.push((format!("ut[{i}].in"), before.clone()));
        }
    });
    ST.with(|x| {
        for (i, (r, p)) in x.borrow().iter().enumerate() {
            s.note(format!("status[{i}] row={r} pass={p}"));
        }
    });
    IOREC.with(|x| {
        for (i, (st, ck, len)) in x.borrow().iter().enumerate() {
            s.note(format!("io[{i}] state=0x{st:x} chunk=0x{ck:08x} len={len}"));
        }
    });
    QPAL.with(|x| {
        for (i, c) in x.borrow().iter().enumerate() {
            s.v(
                "qpal",
                &format!("{i}"),
                ((c.red as i64) << 16) | ((c.green as i64) << 8) | c.blue as i64,
            );
        }
    });
    UCHUNK.with(|x| {
        for (i, (nm, data, loc)) in x.borrow().iter().enumerate() {
            s.note(format!("uchunk[{i}] name={nm} size={} loc={loc}", data.len()));
            s.blobs.push((format!("uchunk[{i}]"), data.clone()));
        }
    });
}

/// In-place user transform: records the `png_row_info` it was handed and
/// mutates the row deterministically without changing its size.
unsafe extern "C-unwind" fn ut_cb(_pp: png_structp, ri: *mut png_row_info, row: png_bytep) {
    let info = *ri;
    let n = info.rowbytes;
    let before = std::slice::from_raw_parts(row, n).to_vec();
    for i in 0..n {
        *row.add(i) = (*row.add(i)).wrapping_add(((i as u8) ^ 0x5a).wrapping_mul(3));
    }
    UT.with(|u| u.borrow_mut().push((info, before)));
}

/// Size-changing user transform: expands 1/2/4-bit grayscale to one byte per
/// pixel, which is exactly what `png_set_user_transform_info(pp, p, 8, 1)`
/// announces, and updates `png_row_info` accordingly.
unsafe extern "C-unwind" fn ut_expand_cb(
    _pp: png_structp,
    ri: *mut png_row_info,
    row: png_bytep,
) {
    let info = *ri;
    let n = info.rowbytes;
    let before = std::slice::from_raw_parts(row, n).to_vec();
    UT.with(|u| u.borrow_mut().push((info, before.clone())));
    if info.color_type != PNG_COLOR_TYPE_GRAY as png_byte || info.bit_depth >= 8 {
        return;
    }
    let bd = info.bit_depth as usize;
    let w = info.width as usize;
    let mask = (1u16 << bd) - 1;
    let mut out = vec![0u8; w];
    for x in 0..w {
        let bit = x * bd;
        let byte = before[bit / 8];
        let shift = 8 - bd - (bit % 8);
        out[x] = ((byte as u16 >> shift) & mask) as u8;
    }
    std::ptr::copy_nonoverlapping(out.as_ptr(), row, w);
    (*ri).bit_depth = 8;
    (*ri).channels = 1;
    (*ri).pixel_depth = 8;
    (*ri).rowbytes = w;
}

unsafe extern "C-unwind" fn status_cb(_pp: png_structp, row: png_uint_32, pass: c_int) {
    ST.with(|x| x.borrow_mut().push((row, pass)));
}

unsafe extern "C-unwind" fn read_cb_io(pp: png_structp, out: png_bytep, len: usize) {
    let l = curlib();
    let st = png_get_io_state(l, pp);
    let ck = png_get_io_chunk_type(l, pp);
    IOREC.with(|x| x.borrow_mut().push((st, ck, len)));
    read_cb(pp, out, len);
}

unsafe extern "C-unwind" fn user_chunk_cb(
    _pp: png_structp,
    ch: *mut png_unknown_chunk,
) -> c_int {
    let c = &*ch;
    let nm = String::from_utf8_lossy(&c.name[..4]).into_owned();
    let data = if c.data.is_null() || c.size == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(c.data, c.size).to_vec()
    };
    UCHUNK.with(|x| x.borrow_mut().push((nm, data, c.location)));
    UCHUNK_RET.with(|x| x.get())
}

// ===========================================================================
// getter snapshots
// ===========================================================================

const VALID_BITS: &[(&str, png_uint_32)] = &[
    ("gAMA", PNG_INFO_gAMA),
    ("sBIT", PNG_INFO_sBIT),
    ("cHRM", PNG_INFO_cHRM),
    ("PLTE", PNG_INFO_PLTE),
    ("tRNS", PNG_INFO_tRNS),
    ("bKGD", PNG_INFO_bKGD),
    ("hIST", PNG_INFO_hIST),
    ("pHYs", PNG_INFO_pHYs),
    ("oFFs", PNG_INFO_oFFs),
    ("tIME", PNG_INFO_tIME),
    ("pCAL", PNG_INFO_pCAL),
    ("sRGB", PNG_INFO_sRGB),
    ("iCCP", PNG_INFO_iCCP),
    ("sPLT", PNG_INFO_sPLT),
    ("sCAL", PNG_INFO_sCAL),
    ("IDAT", PNG_INFO_IDAT),
    ("eXIf", PNG_INFO_eXIf),
    ("cICP", PNG_INFO_cICP),
    ("cLLI", PNG_INFO_cLLI),
    ("mDCV", PNG_INFO_mDCV),
];

unsafe fn snap_hdr(l: &Library, pp: png_structp, ip: png_infop, s: &mut Snap, t: &str) {
    let mut w = 0u32;
    let mut h = 0u32;
    let mut bd = 0;
    let mut ct = 0;
    let mut il = 0;
    let mut cm = 0;
    let mut ft = 0;
    let r = png_get_IHDR(
        l, pp, ip, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut ft,
    );
    s.v(t, "IHDR_ret", r as i64);
    s.v(t, "IHDR_w", w as i64);
    s.v(t, "IHDR_h", h as i64);
    s.v(t, "IHDR_bd", bd as i64);
    s.v(t, "IHDR_ct", ct as i64);
    s.v(t, "IHDR_il", il as i64);
    s.v(t, "IHDR_cm", cm as i64);
    s.v(t, "IHDR_ft", ft as i64);
    s.v(t, "width", png_get_image_width(l, pp, ip) as i64);
    s.v(t, "height", png_get_image_height(l, pp, ip) as i64);
    s.v(t, "bit_depth", png_get_bit_depth(l, pp, ip) as i64);
    s.v(t, "color_type", png_get_color_type(l, pp, ip) as i64);
    s.v(t, "interlace", png_get_interlace_type(l, pp, ip) as i64);
    s.v(t, "comp_type", png_get_compression_type(l, pp, ip) as i64);
    s.v(t, "filter_type", png_get_filter_type(l, pp, ip) as i64);
    s.v(t, "channels", png_get_channels(l, pp, ip) as i64);
    s.v(t, "rowbytes", png_get_rowbytes(l, pp, ip) as i64);
    s.v(t, "palette_max", png_get_palette_max(l, pp, ip) as i64);
    s.v(t, "rgb2gray", png_get_rgb_to_gray_status(l, pp) as i64);
    s.v(t, "cur_row", png_get_current_row_number(l, pp) as i64);
    s.v(t, "cur_pass", png_get_current_pass_number(l, pp) as i64);
    s.v(t, "io_state", png_get_io_state(l, pp) as i64);
    s.v(t, "io_chunk", png_get_io_chunk_type(l, pp) as i64);
    for (nm, bit) in VALID_BITS {
        s.v(t, &format!("valid_{nm}"), png_get_valid(l, pp, ip, *bit) as i64);
    }

    // PLTE
    let mut pal: *mut png_color = ptr::null_mut();
    let mut np: c_int = 0;
    let pr = png_get_PLTE(l, pp, ip, &mut pal, &mut np);
    s.v(t, "PLTE_ret", pr as i64);
    s.v(t, "PLTE_n", np as i64);
    if pr != 0 && !pal.is_null() {
        for i in 0..np.max(0) as usize {
            let c = *pal.add(i);
            s.v(
                t,
                &format!("pal{i}"),
                ((c.red as i64) << 16) | ((c.green as i64) << 8) | c.blue as i64,
            );
        }
    }
    // tRNS
    let mut tr: *mut png_byte = ptr::null_mut();
    let mut ntr: c_int = 0;
    let mut tc: *mut png_color_16 = ptr::null_mut();
    let r2 = png_get_tRNS(l, pp, ip, &mut tr, &mut ntr, &mut tc);
    s.v(t, "tRNS_ret", r2 as i64);
    s.v(t, "tRNS_n", ntr as i64);
    if r2 != 0 {
        // `trans_alpha` is only an array for palette images.  For a colour-key
        // tRNS libpng sets num_trans = 1 as a flag while `trans_alpha` is not a
        // meaningful buffer (and, once png_set_quantize has rewritten
        // info_ptr->color_type to PALETTE, it may even be a stale allocation),
        // so only the *file's* colour type may be used to decide.
        if !tr.is_null() && FILE_CT.with(|c| c.get()) == PNG_COLOR_TYPE_PALETTE {
            for i in 0..ntr.max(0) as usize {
                s.v(t, &format!("trns{i}"), *tr.add(i) as i64);
            }
        }
        if !tc.is_null() {
            let x = *tc;
            s.v(t, "trns_index", x.index as i64);
            s.v(t, "trns_r", x.red as i64);
            s.v(t, "trns_g", x.green as i64);
            s.v(t, "trns_b", x.blue as i64);
            s.v(t, "trns_gray", x.gray as i64);
        }
    }
    // bKGD
    let mut bg: *mut png_color_16 = ptr::null_mut();
    let r3 = png_get_bKGD(l, pp, ip, &mut bg);
    s.v(t, "bKGD_ret", r3 as i64);
    if r3 != 0 && !bg.is_null() {
        let x = *bg;
        s.v(t, "bkgd_index", x.index as i64);
        s.v(t, "bkgd_r", x.red as i64);
        s.v(t, "bkgd_g", x.green as i64);
        s.v(t, "bkgd_b", x.blue as i64);
        s.v(t, "bkgd_gray", x.gray as i64);
    }
    // sBIT
    let mut sb: *mut png_color_8 = ptr::null_mut();
    let r4 = png_get_sBIT(l, pp, ip, &mut sb);
    s.v(t, "sBIT_ret", r4 as i64);
    if r4 != 0 && !sb.is_null() {
        let x = *sb;
        s.v(t, "sbit_r", x.red as i64);
        s.v(t, "sbit_g", x.green as i64);
        s.v(t, "sbit_b", x.blue as i64);
        s.v(t, "sbit_gray", x.gray as i64);
        s.v(t, "sbit_a", x.alpha as i64);
    }
    // gAMA
    let mut g: png_fixed_point = 0;
    let r5 = png_get_gAMA_fixed(l, pp, ip, &mut g);
    s.v(t, "gAMA_ret", r5 as i64);
    s.v(t, "gAMA", g as i64);
}

unsafe fn snap_full(l: &Library, pp: png_structp, ip: png_infop, s: &mut Snap, t: &str) {
    snap_hdr(l, pp, ip, s, t);

    // cHRM
    let mut a = [0i32; 8];
    let rc = png_get_cHRM_fixed(
        l, pp, ip, &mut a[0], &mut a[1], &mut a[2], &mut a[3], &mut a[4], &mut a[5], &mut a[6],
        &mut a[7],
    );
    s.v(t, "cHRM_ret", rc as i64);
    for (i, x) in a.iter().enumerate() {
        s.v(t, &format!("chrm{i}"), *x as i64);
    }
    let mut b = [0i32; 9];
    let rcz = png_get_cHRM_XYZ_fixed(
        l, pp, ip, &mut b[0], &mut b[1], &mut b[2], &mut b[3], &mut b[4], &mut b[5], &mut b[6],
        &mut b[7], &mut b[8],
    );
    s.v(t, "cHRM_XYZ_ret", rcz as i64);
    for (i, x) in b.iter().enumerate() {
        s.v(t, &format!("chrmxyz{i}"), *x as i64);
    }
    // sRGB
    let mut intent: c_int = -1;
    s.v(t, "sRGB_ret", png_get_sRGB(l, pp, ip, &mut intent) as i64);
    s.v(t, "sRGB_intent", intent as i64);
    // iCCP
    let mut nm: png_charp = ptr::null_mut();
    let mut ctype: c_int = -1;
    let mut prof: png_bytep = ptr::null_mut();
    let mut plen: png_uint_32 = 0;
    let ri = png_get_iCCP(l, pp, ip, &mut nm, &mut ctype, &mut prof, &mut plen);
    s.v(t, "iCCP_ret", ri as i64);
    s.v(t, "iCCP_ctype", ctype as i64);
    s.v(t, "iCCP_len", plen as i64);
    if ri != 0 {
        s.note(format!("{t}.iCCP_name={}", cstr(nm)));
        if !prof.is_null() {
            s.blobs.push((
                format!("{t}.iCCP"),
                std::slice::from_raw_parts(prof, plen as usize).to_vec(),
            ));
        }
    }
    // hIST
    let mut hist: *mut png_uint_16 = ptr::null_mut();
    let rh = png_get_hIST(l, pp, ip, &mut hist);
    s.v(t, "hIST_ret", rh as i64);
    if rh != 0 && !hist.is_null() {
        let mut np: c_int = 0;
        let mut pal: *mut png_color = ptr::null_mut();
        png_get_PLTE(l, pp, ip, &mut pal, &mut np);
        for i in 0..np.max(0) as usize {
            s.v(t, &format!("hist{i}"), *hist.add(i) as i64);
        }
    }
    // oFFs
    let mut ox: png_int_32 = 0;
    let mut oy: png_int_32 = 0;
    let mut ou: c_int = -1;
    s.v(
        t,
        "oFFs_ret",
        png_get_oFFs(l, pp, ip, &mut ox, &mut oy, &mut ou) as i64,
    );
    s.v(t, "oFFs_x", ox as i64);
    s.v(t, "oFFs_y", oy as i64);
    s.v(t, "oFFs_u", ou as i64);
    // pHYs
    let mut px: png_uint_32 = 0;
    let mut py: png_uint_32 = 0;
    let mut pu: c_int = -1;
    s.v(
        t,
        "pHYs_ret",
        png_get_pHYs(l, pp, ip, &mut px, &mut py, &mut pu) as i64,
    );
    s.v(t, "pHYs_x", px as i64);
    s.v(t, "pHYs_y", py as i64);
    s.v(t, "pHYs_u", pu as i64);
    let mut dx: png_uint_32 = 0;
    let mut dy: png_uint_32 = 0;
    let mut du: c_int = -1;
    s.v(
        t,
        "pHYs_dpi_ret",
        png_get_pHYs_dpi(l, pp, ip, &mut dx, &mut dy, &mut du) as i64,
    );
    s.v(t, "dpi_x", dx as i64);
    s.v(t, "dpi_y", dy as i64);
    s.v(t, "dpi_u", du as i64);
    s.v(t, "xppm", png_get_x_pixels_per_meter(l, pp, ip) as i64);
    s.v(t, "yppm", png_get_y_pixels_per_meter(l, pp, ip) as i64);
    s.v(t, "ppm", png_get_pixels_per_meter(l, pp, ip) as i64);
    s.v(t, "xppi", png_get_x_pixels_per_inch(l, pp, ip) as i64);
    s.v(t, "yppi", png_get_y_pixels_per_inch(l, pp, ip) as i64);
    s.v(t, "ppi", png_get_pixels_per_inch(l, pp, ip) as i64);
    s.vf(t, "aspect", png_get_pixel_aspect_ratio(l, pp, ip));
    s.v(
        t,
        "aspect_fixed",
        png_get_pixel_aspect_ratio_fixed(l, pp, ip) as i64,
    );
    s.v(t, "xoff_px", png_get_x_offset_pixels(l, pp, ip) as i64);
    s.v(t, "yoff_px", png_get_y_offset_pixels(l, pp, ip) as i64);
    s.v(t, "xoff_um", png_get_x_offset_microns(l, pp, ip) as i64);
    s.v(t, "yoff_um", png_get_y_offset_microns(l, pp, ip) as i64);
    s.vf(t, "xoff_in", png_get_x_offset_inches(l, pp, ip));
    s.vf(t, "yoff_in", png_get_y_offset_inches(l, pp, ip));
    s.v(
        t,
        "xoff_in_fixed",
        png_get_x_offset_inches_fixed(l, pp, ip) as i64,
    );
    s.v(
        t,
        "yoff_in_fixed",
        png_get_y_offset_inches_fixed(l, pp, ip) as i64,
    );
    // sCAL
    let mut su: c_int = -1;
    let mut sw: png_fixed_point = 0;
    let mut sh: png_fixed_point = 0;
    s.v(
        t,
        "sCAL_fixed_ret",
        png_get_sCAL_fixed(l, pp, ip, &mut su, &mut sw, &mut sh) as i64,
    );
    s.v(t, "sCAL_u", su as i64);
    s.v(t, "sCAL_w", sw as i64);
    s.v(t, "sCAL_h", sh as i64);
    let mut su2: c_int = -1;
    let mut sws: png_charp = ptr::null_mut();
    let mut shs: png_charp = ptr::null_mut();
    let rs = png_get_sCAL_s(l, pp, ip, &mut su2, &mut sws, &mut shs);
    s.v(t, "sCAL_s_ret", rs as i64);
    if rs != 0 {
        s.note(format!(
            "{t}.sCAL_s u={su2} w={} h={}",
            cstr(sws),
            cstr(shs)
        ));
    }
    // pCAL
    let mut purpose: png_charp = ptr::null_mut();
    let mut x0: png_int_32 = 0;
    let mut x1: png_int_32 = 0;
    let mut eq: c_int = -1;
    let mut nparams: c_int = -1;
    let mut units: png_charp = ptr::null_mut();
    let mut params: *mut png_charp = ptr::null_mut();
    let rp = png_get_pCAL(
        l,
        pp,
        ip,
        &mut purpose,
        &mut x0,
        &mut x1,
        &mut eq,
        &mut nparams,
        &mut units,
        &mut params,
    );
    s.v(t, "pCAL_ret", rp as i64);
    s.v(t, "pCAL_x0", x0 as i64);
    s.v(t, "pCAL_x1", x1 as i64);
    s.v(t, "pCAL_eq", eq as i64);
    s.v(t, "pCAL_np", nparams as i64);
    if rp != 0 {
        s.note(format!(
            "{t}.pCAL purpose={} units={}",
            cstr(purpose),
            cstr(units)
        ));
        if !params.is_null() {
            for i in 0..nparams.max(0) as usize {
                s.note(format!("{t}.pCAL_p{i}={}", cstr(*params.add(i))));
            }
        }
    }
    // tIME
    let mut tm: *mut png_time = ptr::null_mut();
    let rt = png_get_tIME(l, pp, ip, &mut tm);
    s.v(t, "tIME_ret", rt as i64);
    if rt != 0 && !tm.is_null() {
        let x = *tm;
        s.v(t, "tIME_year", x.year as i64);
        s.v(t, "tIME_month", x.month as i64);
        s.v(t, "tIME_day", x.day as i64);
        s.v(t, "tIME_hour", x.hour as i64);
        s.v(t, "tIME_min", x.minute as i64);
        s.v(t, "tIME_sec", x.second as i64);
    }
    // eXIf
    let mut nex: png_uint_32 = 0;
    let mut exp: png_bytep = ptr::null_mut();
    let re = png_get_eXIf_1(l, pp, ip, &mut nex, &mut exp);
    s.v(t, "eXIf_ret", re as i64);
    s.v(t, "eXIf_n", nex as i64);
    if re != 0 && !exp.is_null() {
        s.blobs.push((
            format!("{t}.eXIf"),
            std::slice::from_raw_parts(exp, nex as usize).to_vec(),
        ));
    }
    // sPLT
    let mut spp: *mut png_sPLT_t = ptr::null_mut();
    let ns = png_get_sPLT(l, pp, ip, &mut spp);
    s.v(t, "sPLT_n", ns as i64);
    for i in 0..ns.max(0) as usize {
        let x = *spp.add(i);
        s.note(format!(
            "{t}.sPLT{i} name={} depth={} n={}",
            cstr(x.name),
            x.depth,
            x.nentries
        ));
        if !x.entries.is_null() {
            for j in 0..x.nentries.max(0) as usize {
                let e = *x.entries.add(j);
                s.note(format!(
                    "{t}.sPLT{i}.e{j}={},{},{},{},{}",
                    e.red, e.green, e.blue, e.alpha, e.frequency
                ));
            }
        }
    }
    // text
    let mut tp: *mut png_text = ptr::null_mut();
    let mut nt: c_int = 0;
    let ntx = png_get_text(l, pp, ip, &mut tp, &mut nt);
    s.v(t, "text_ret", ntx as i64);
    s.v(t, "text_num", nt as i64);
    for i in 0..ntx.max(0) as usize {
        let x = *tp.add(i);
        s.note(format!(
            "{t}.text{i} comp={} key={} text={} tlen={} ilen={} lang={} langkey={}",
            x.compression,
            cstr(x.key),
            cstr(x.text),
            x.text_length,
            x.itxt_length,
            cstr(x.lang),
            cstr(x.lang_key)
        ));
    }
    // unknown chunks
    let mut uc: *mut png_unknown_chunk = ptr::null_mut();
    let nu = png_get_unknown_chunks(l, pp, ip, &mut uc);
    s.v(t, "unknown_n", nu as i64);
    for i in 0..nu.max(0) as usize {
        let x = *uc.add(i);
        s.note(format!(
            "{t}.unk{i} name={} size={} loc={}",
            String::from_utf8_lossy(&x.name[..4]),
            x.size,
            x.location
        ));
        if !x.data.is_null() {
            s.blobs.push((
                format!("{t}.unk{i}"),
                std::slice::from_raw_parts(x.data, x.size).to_vec(),
            ));
        }
    }
    // cICP / cLLI / mDCV
    let mut cp = 0u8;
    let mut tf = 0u8;
    let mut mc = 0u8;
    let mut vf = 0u8;
    s.v(
        t,
        "cICP_ret",
        png_get_cICP(l, pp, ip, &mut cp, &mut tf, &mut mc, &mut vf) as i64,
    );
    s.v(t, "cICP_cp", cp as i64);
    s.v(t, "cICP_tf", tf as i64);
    s.v(t, "cICP_mc", mc as i64);
    s.v(t, "cICP_vf", vf as i64);
    let mut mxc = 0u32;
    let mut mxf = 0u32;
    s.v(
        t,
        "cLLI_fixed_ret",
        png_get_cLLI_fixed(l, pp, ip, &mut mxc, &mut mxf) as i64,
    );
    s.v(t, "cLLI_maxCLL", mxc as i64);
    s.v(t, "cLLI_maxFALL", mxf as i64);
    let mut dc = 0f64;
    let mut df = 0f64;
    s.v(
        t,
        "cLLI_ret",
        png_get_cLLI(l, pp, ip, &mut dc, &mut df) as i64,
    );
    s.v(t, "cLLI_d0", dc.to_bits() as i64);
    s.v(t, "cLLI_d1", df.to_bits() as i64);
    let mut m = [0u32; 10];
    let rm = png_get_mDCV_fixed(
        l, pp, ip, &mut m[0], &mut m[1], &mut m[2], &mut m[3], &mut m[4], &mut m[5], &mut m[6],
        &mut m[7], &mut m[8], &mut m[9],
    );
    s.v(t, "mDCV_ret", rm as i64);
    for (i, x) in m.iter().enumerate() {
        s.v(t, &format!("mdcv{i}"), *x as i64);
    }
    // floating-point accessor variants
    let mut gd = 0f64;
    s.v(t, "gAMA_d_ret", png_get_gAMA(l, pp, ip, &mut gd) as i64);
    s.v(t, "gAMA_d", gd.to_bits() as i64);
    let mut cd = [0f64; 8];
    let rcd = png_get_cHRM(
        l, pp, ip, &mut cd[0], &mut cd[1], &mut cd[2], &mut cd[3], &mut cd[4], &mut cd[5],
        &mut cd[6], &mut cd[7],
    );
    s.v(t, "cHRM_d_ret", rcd as i64);
    for (i, x) in cd.iter().enumerate() {
        s.v(t, &format!("chrmd{i}"), x.to_bits() as i64);
    }
    let mut sud: c_int = -1;
    let mut swd = 0f64;
    let mut shd = 0f64;
    s.v(
        t,
        "sCAL_d_ret",
        png_get_sCAL(l, pp, ip, &mut sud, &mut swd, &mut shd) as i64,
    );
    s.v(t, "sCAL_d_u", sud as i64);
    s.v(t, "sCAL_d_w", swd.to_bits() as i64);
    s.v(t, "sCAL_d_h", shd.to_bits() as i64);
    let mut exp2: png_bytep = ptr::null_mut();
    s.v(t, "eXIf_ptr_ret", png_get_eXIf(l, pp, ip, &mut exp2) as i64);

    // signature / rows
    let sig = png_get_signature(l, pp, ip);
    if sig.is_null() {
        s.v(t, "sig_null", 1);
    } else {
        s.v(t, "sig_null", 0);
        s.blobs.push((
            format!("{t}.sig"),
            std::slice::from_raw_parts(sig, 8).to_vec(),
        ));
    }
    s.v(t, "rows_null", png_get_rows(l, pp, ip).is_null() as i64);
    // limits
    s.v(t, "user_w_max", png_get_user_width_max(l, pp) as i64);
    s.v(t, "user_h_max", png_get_user_height_max(l, pp) as i64);
    s.v(t, "chunk_cache_max", png_get_chunk_cache_max(l, pp) as i64);
    s.v(t, "chunk_malloc_max", png_get_chunk_malloc_max(l, pp) as i64);
    s.v(t, "cbuf_size", png_get_compression_buffer_size(l, pp) as i64);
    s.v(
        t,
        "utrans_ptr",
        (png_get_user_transform_ptr(l, pp) as usize) as i64,
    );
    s.v(
        t,
        "uchunk_ptr",
        (png_get_user_chunk_ptr(l, pp) as usize) as i64,
    );
}

// ===========================================================================
// sequential read driver
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Image,
    RowNull,
    RowDisplay,
    /// `png_read_row(row, display_row)` with *both* pointers supplied
    RowBoth,
    Rows,
    /// `png_read_rows(rows, display_rows, n)` with both arrays supplied
    RowsBoth,
    NoRows,
    ReadPng(c_int),
}

#[derive(Clone, Copy)]
struct Opt {
    mode: Mode,
    updates: u32,
    start_img: bool,
    full: bool,
    io_hook: bool,
    sig_bytes: Option<c_int>,
}

impl Default for Opt {
    fn default() -> Self {
        Opt {
            mode: Mode::Image,
            updates: 1,
            start_img: false,
            full: false,
            io_hook: false,
            sig_bytes: None,
        }
    }
}

type Hook<'a> = &'a dyn Fn(&Library, png_structp, png_infop, &mut Snap);

fn nohook(_l: &Library, _p: png_structp, _i: png_infop, _s: &mut Snap) {}

unsafe fn decode(l: &Library, png: &[u8], o: Opt, pre: Hook, post: Hook) -> Snap {
    let mut s = Snap::default();
    reset_recorders();
    CURLIB.with(|c| c.set(l as *const Library));
    src_set(png);
    let pp = png_create_read_struct(
        l,
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
    );
    assert!(!pp.is_null());
    if o.io_hook {
        png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb_io));
    } else {
        png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb));
    }
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    if let Some(n) = o.sig_bytes {
        png_set_sig_bytes(l, pp, n);
    }
    pre(l, pp, ip, &mut s);

    let snapf = if o.full { snap_full } else { snap_hdr };

    FILE_CT.with(|c| c.set(-1));
    if let Mode::ReadPng(tr) = o.mode {
        png_read_png(l, pp, ip, tr, ptr::null_mut());
        FILE_CT.with(|c| c.set(png_get_color_type(l, pp, ip) as c_int));
        snapf(l, pp, ip, &mut s, "after");
        let rows = png_get_rows(l, pp, ip);
        let h = png_get_image_height(l, pp, ip) as usize;
        let n = png_get_rowbytes(l, pp, ip);
        // png_read_png allocates the row buffers with png_malloc, i.e. they are
        // *uninitialised*.  png_combine_row deliberately preserves the
        // destination bits beyond the last pixel of the row
        // (`end_mask = 0xff >> ((pixel_depth * width) & 7)`), so those bits are
        // whatever the allocator happened to hand out and cannot be compared.
        let pd = png_get_channels(l, pp, ip) as usize * png_get_bit_depth(l, pp, ip) as usize;
        let tail = (pd * png_get_image_width(l, pp, ip) as usize) % 8;
        if !rows.is_null() {
            for y in 0..h {
                let p = *rows.add(y);
                if p.is_null() {
                    s.rows.push(Vec::new());
                } else {
                    let mut v = std::slice::from_raw_parts(p, n).to_vec();
                    if tail != 0 {
                        let last = v.len() - 1;
                        v[last] &= !(0xffu8 >> tail);
                    }
                    s.rows.push(v);
                }
            }
        }
    } else {
        png_read_info(l, pp, ip);
        FILE_CT.with(|c| c.set(png_get_color_type(l, pp, ip) as c_int));
        snapf(l, pp, ip, &mut s, "info");
        post(l, pp, ip, &mut s);
        let passes = match o.mode {
            Mode::Image | Mode::NoRows => 1,
            _ => {
                let n = png_set_interlace_handling(l, pp);
                s.v("x", "passes", n as i64);
                n
            }
        };
        for _ in 0..o.updates {
            png_read_update_info(l, pp, ip);
        }
        if o.start_img {
            png_start_read_image(l, pp);
        }
        snapf(l, pp, ip, &mut s, "upd");
        let h = png_get_image_height(l, pp, ip) as usize;
        let n = png_get_rowbytes(l, pp, ip);
        let mut buf: Vec<Vec<u8>> = (0..h.max(1)).map(|_| vec![0u8; n.max(1)]).collect();
        let mut dbuf: Vec<Vec<u8>> = (0..h.max(1)).map(|_| vec![0u8; n.max(1)]).collect();
        match o.mode {
            Mode::Image => {
                let mut rp: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                png_read_image(l, pp, rp.as_mut_ptr());
            }
            Mode::RowNull => {
                for _ in 0..passes {
                    for y in 0..h {
                        png_read_row(l, pp, buf[y].as_mut_ptr(), ptr::null_mut());
                    }
                }
            }
            Mode::RowDisplay => {
                for _ in 0..passes {
                    for y in 0..h {
                        png_read_row(l, pp, ptr::null_mut(), buf[y].as_mut_ptr());
                    }
                }
            }
            Mode::RowBoth => {
                for _ in 0..passes {
                    for y in 0..h {
                        png_read_row(l, pp, buf[y].as_mut_ptr(), dbuf[y].as_mut_ptr());
                    }
                }
            }
            Mode::Rows => {
                for _ in 0..passes {
                    let mut rp: Vec<png_bytep> =
                        buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    png_read_rows(l, pp, rp.as_mut_ptr(), ptr::null_mut(), h as png_uint_32);
                }
            }
            Mode::RowsBoth => {
                for _ in 0..passes {
                    let mut rp: Vec<png_bytep> =
                        buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    let mut dp: Vec<png_bytep> =
                        dbuf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    png_read_rows(l, pp, rp.as_mut_ptr(), dp.as_mut_ptr(), h as png_uint_32);
                }
            }
            Mode::NoRows => {}
            Mode::ReadPng(_) => unreachable!(),
        }
        if o.mode != Mode::NoRows {
            snapf(l, pp, ip, &mut s, "rows");
            for r in &buf {
                s.rows.push(r.clone());
            }
            if matches!(o.mode, Mode::RowBoth | Mode::RowsBoth) {
                for r in &dbuf {
                    s.rows.push(r.clone());
                }
            }
        }
        png_read_end(l, pp, ip);
        snapf(l, pp, ip, &mut s, "end");
    }
    s.note(format!("src_pos={}", src_pos()));
    drain_recorders(&mut s);
    let mut p = pp;
    let mut i = ip;
    png_destroy_read_struct(l, &mut p, &mut i, ptr::null_mut());
    s
}

/// Returns `true` when the reference (C) decode ran to completion; used by the
/// callers to make sure a whole test is not passing vacuously because both
/// libraries reject every input.
fn diff(label: &str, png: &[u8], o: Opt, pre: Hook, post: Hook) -> bool {
    let c = capture(|| unsafe { decode(&L().c, png, o, pre, post) });
    let r = capture(|| unsafe { decode(&L().rs, png, o, pre, post) });
    assert_snap(&c, &r, label);
    if c.out.is_none() && std::env::var_os("SHOW_ERR").is_some() {
        eprintln!(
            "  [err] {label}: {:?}",
            c.log.iter().map(|m| m.to_string()).collect::<Vec<_>>()
        );
    }
    let ok = c.out.is_some();
    if ok && o.mode != Mode::NoRows {
        assert!(
            c.out.as_ref().unwrap().rows.iter().any(|r| !r.is_empty()),
            "{label}: C decode produced no row data at all"
        );
    }
    ok
}

fn diff_tx(label: &str, png: &[u8], post: Hook) -> bool {
    diff(label, png, Opt::default(), &nohook, post)
}

/// A transform case driven through the per-row entry points as well, so that
/// `png_combine_row`'s display/interlace paths see the transformed rows too.
fn diff_tx_rows(label: &str, png: &[u8], post: Hook) {
    for mode in [Mode::RowDisplay, Mode::RowsBoth] {
        diff(
            &format!("{label} {mode:?}"),
            png,
            Opt {
                mode,
                ..Opt::default()
            },
            &nohook,
            post,
        );
    }
}

/// A background colour that is *in range* for the given image and
/// `need_expand` setting.
///
/// This matters: for `need_expand == 0` libpng interprets the components as
/// 8-bit output samples and indexes its 256-entry gamma tables with them
/// directly (`pngrtran.c`, `gamma_table[png_ptr->background.red]` in the
/// palette+tRNS path), and for `need_expand != 0` it first scales a low-bit-depth
/// grey value by 0xff/0x55/0x11.  Out-of-range components therefore make the C
/// reference read past the end of its gamma tables, which is undefined
/// behaviour and not something a translation can be held to.
fn valid_bg(ct: c_int, bd: c_int, need_expand: c_int) -> png_color_16 {
    if need_expand != 0 {
        if ct == PNG_COLOR_TYPE_PALETTE {
            // only `index` is used, and it must address a real palette entry
            return png_color_16 {
                index: 1,
                red: 0,
                green: 0,
                blue: 0,
                gray: 0,
            };
        }
        let max: u32 = if bd >= 16 { 0xffff } else { (1u32 << bd) - 1 };
        png_color_16 {
            index: 0,
            red: (max / 2) as png_uint_16,
            green: (max / 3) as png_uint_16,
            blue: (max / 5) as png_uint_16,
            gray: (max / 2) as png_uint_16,
        }
    } else {
        // output space: 8 bits per component for everything except an
        // unstripped 16-bit image, so stay inside 0..=255
        png_color_16 {
            index: 1,
            red: 100,
            green: 200,
            blue: 50,
            gray: 130,
        }
    }
}

// ===========================================================================
// row 38 - plain round-trip decode of every (color_type, bit_depth)
// ===========================================================================

const CT_BD: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 2),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

const SHAPES: &[(u32, u32)] = &[(1, 1), (7, 3), (8, 8), (9, 17), (33, 5)];

#[test]
fn cfg38_read_image_all_shapes() {
    for &(ct, bd) in CT_BD {
        for il in [0, 1] {
            for &(w, h) in SHAPES {
                let png = make_png(ct, bd, il, w, h, 0x38_0001);
                let lbl = format!("cfg38 image ct={ct} bd={bd} il={il} {w}x{h}");
                diff(
                    &lbl,
                    &png,
                    Opt {
                        mode: Mode::Image,
                        ..Opt::default()
                    },
                    &nohook,
                    &nohook,
                );
            }
        }
    }
}

#[test]
fn cfg38_read_row_variants() {
    for &(ct, bd) in CT_BD {
        for il in [0, 1] {
            for &(w, h) in &[(7u32, 3u32), (9, 17), (33, 5)] {
                let png = make_png(ct, bd, il, w, h, 0x38_0001);
                for (nm, mode) in [
                    ("row", Mode::RowNull),
                    ("display", Mode::RowDisplay),
                    ("row+display", Mode::RowBoth),
                    ("rows", Mode::Rows),
                    ("rows+display", Mode::RowsBoth),
                ] {
                    let lbl = format!("cfg38 {nm} ct={ct} bd={bd} il={il} {w}x{h}");
                    diff(
                        &lbl,
                        &png,
                        Opt {
                            mode,
                            ..Opt::default()
                        },
                        &nohook,
                        &nohook,
                    );
                }
            }
        }
    }
}

#[test]
fn cfg38_read_png_identity() {
    for &(ct, bd) in CT_BD {
        for il in [0, 1] {
            for &(w, h) in SHAPES {
                let png = make_png(ct, bd, il, w, h, 0x38_0001);
                let lbl = format!("cfg38 read_png ct={ct} bd={bd} il={il} {w}x{h}");
                diff(
                    &lbl,
                    &png,
                    Opt {
                        mode: Mode::ReadPng(PNG_TRANSFORM_IDENTITY),
                        ..Opt::default()
                    },
                    &nohook,
                    &nohook,
                );
            }
        }
    }
}

#[test]
fn cfg38_filter_types() {
    // one PNG per adaptive-filter type so that every png_read_filter_row
    // variant is hit for bpp 1..8
    for filter in [
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_ALL_FILTERS,
    ] {
        let ex = Extras {
            filter,
            ..Extras::default()
        };
        for &(ct, bd) in CT_BD {
            for il in [0, 1] {
                for &(w, h) in &[(9u32, 7u32), (1, 1), (33, 5)] {
                    let png = make_png_ex(ct, bd, il, w, h, 0x38_0002, ex);
                    diff(
                        &format!("cfg38 filter={filter:#x} ct={ct} bd={bd} il={il} {w}x{h}"),
                        &png,
                        Opt::default(),
                        &nohook,
                        &nohook,
                    );
                }
            }
        }
    }
}

#[test]
fn cfg38_more_seeds() {
    for seed in [1u64, 0xdead_beef, 0x9e37_79b9_7f4a_7c15] {
        for &(ct, bd) in CT_BD {
            for il in [0, 1] {
                for &(w, h) in &[(9u32, 17u32), (33, 5)] {
                    let png = make_png(ct, bd, il, w, h, seed);
                    for mode in [Mode::Image, Mode::RowDisplay] {
                        diff(
                            &format!("cfg38 seed={seed:#x} ct={ct} bd={bd} il={il} {w}x{h} {mode:?}"),
                            &png,
                            Opt {
                                mode,
                                ..Opt::default()
                            },
                            &nohook,
                            &nohook,
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// row 39 - one test per read transform
// ===========================================================================

/// run a transform hook over a list of (color_type, bit_depth) and both
/// interlace settings
/// shapes used by the transform rows: 1x1, a shape where several Adam7 passes
/// are empty, a width that is not a byte multiple, and a wide one
const TX_SHAPES: &[(u32, u32)] = &[(9, 7), (1, 1), (2, 2), (33, 3)];

fn over_x(list: &[(c_int, c_int)], ex: Extras, label: &str, post: Hook, require_ok: bool) {
    let mut oks = 0usize;
    let mut total = 0usize;
    for &(ct, bd) in list {
        for il in [0, 1] {
            for (i, &(w, h)) in TX_SHAPES.iter().enumerate() {
                let png = make_png_ex(ct, bd, il, w, h, 0x39_0002, ex);
                total += 1;
                let lbl = format!("{label} ct={ct} bd={bd} il={il} {w}x{h}");
                if diff_tx(&lbl, &png, post) {
                    oks += 1;
                }
                if i == 0 {
                    diff_tx_rows(&lbl, &png, post);
                }
            }
        }
    }
    if oks == 0 && std::env::var_os("SHOW_ERR").is_some() {
        eprintln!("[all-error] {label} ({total} configs)");
    }
    assert!(
        !require_ok || oks > 0,
        "{label}: all {total} configurations failed to decode - the assertion would be vacuous"
    );
}

fn over(list: &[(c_int, c_int)], ex: Extras, label: &str, post: Hook) {
    over_x(list, ex, label, post, true)
}

/// Like [`over`] but for configurations where libpng legitimately rejects every
/// input (the error messages are still compared).
fn over_may_err(list: &[(c_int, c_int)], ex: Extras, label: &str, post: Hook) {
    over_x(list, ex, label, post, false)
}

const PAL: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
];
const GRAYS: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
];
const ALL: &[(c_int, c_int)] = CT_BD;
const RGBS: &[(c_int, c_int)] = &[(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_RGB, 16)];
const ALPHAS: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

#[test]
fn cfg39_palette_to_rgb() {
    over(PAL, Extras::default(), "cfg39 palette_to_rgb", &|l, pp, _i, _s| unsafe {
        png_set_palette_to_rgb(l, pp)
    });
}

#[test]
fn cfg39_trns_to_alpha() {
    let ex = Extras {
        trns: true,
        ..Extras::default()
    };
    let list: Vec<(c_int, c_int)> = PAL
        .iter()
        .chain(GRAYS.iter())
        .chain(RGBS.iter())
        .copied()
        .collect();
    over(&list, ex, "cfg39 tRNS_to_alpha", &|l, pp, _i, _s| unsafe {
        png_set_tRNS_to_alpha(l, pp)
    });
}

#[test]
fn cfg39_expand() {
    let ex = Extras {
        trns: true,
        ..Extras::default()
    };
    over(ALL, ex, "cfg39 expand", &|l, pp, _i, _s| unsafe {
        png_set_expand(l, pp)
    });
    over(ALL, Extras::default(), "cfg39 expand_notrns", &|l, pp, _i, _s| unsafe {
        png_set_expand(l, pp)
    });
}

#[test]
fn cfg39_expand_gray_1_2_4_to_8() {
    over(
        GRAYS,
        Extras::default(),
        "cfg39 expand_gray",
        &|l, pp, _i, _s| unsafe { png_set_expand_gray_1_2_4_to_8(l, pp) },
    );
}

#[test]
fn cfg39_expand_16() {
    over(ALL, Extras::default(), "cfg39 expand_16", &|l, pp, _i, _s| unsafe {
        png_set_expand_16(l, pp)
    });
}

#[test]
fn cfg39_gray_to_rgb() {
    let list: Vec<(c_int, c_int)> = GRAYS
        .iter()
        .chain(ALPHAS.iter())
        .copied()
        .collect();
    over(&list, Extras::default(), "cfg39 gray_to_rgb", &|l, pp, _i, _s| unsafe {
        png_set_gray_to_rgb(l, pp)
    });
}

#[test]
fn cfg39_rgb_to_gray_fixed() {
    let list: Vec<(c_int, c_int)> = RGBS
        .iter()
        .chain(&[
            (PNG_COLOR_TYPE_RGB_ALPHA, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 16),
            (PNG_COLOR_TYPE_PALETTE, 8),
        ])
        .copied()
        .collect();
    for action in [1, 2, 3] {
        for &(r, g) in &[(-1i32, -1i32), (30000, 50000), (0, 100000)] {
            let lbl = format!("cfg39 rgb_to_gray action={action} coef=({r},{g})");
            let hook = move |l: &Library, pp: png_structp, _i: png_infop, _s: &mut Snap| unsafe {
                png_set_rgb_to_gray_fixed(l, pp, action, r, g)
            };
            // PNG_ERROR_ACTION_ERROR (3) makes libpng png_error on the first
            // non-grey pixel, and the fixtures are random colour, so every
            // configuration is expected to fail there.
            if action == PNG_ERROR_ACTION_ERROR {
                over_may_err(&list, Extras::default(), &lbl, &hook);
            } else {
                over(&list, Extras::default(), &lbl, &hook);
            }
        }
    }
}

#[test]
fn cfg39_background_fixed() {
    let list: Vec<(c_int, c_int)> = [
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    ]
    .to_vec();
    for code in [
        PNG_BACKGROUND_GAMMA_SCREEN,
        PNG_BACKGROUND_GAMMA_FILE,
        PNG_BACKGROUND_GAMMA_UNIQUE,
    ] {
        for need_expand in [0, 1] {
            let lbl = format!("cfg39 background code={code} expand={need_expand}");
            over(
                &list,
                Extras {
                    gama: true,
                    ..Extras::default()
                },
                &lbl,
                &move |l, pp, ip, _s| unsafe {
                    let ct = png_get_color_type(l, pp, ip) as c_int;
                    let bd = png_get_bit_depth(l, pp, ip) as c_int;
                    let bg = valid_bg(ct, bd, need_expand);
                    png_set_background_fixed(l, pp, &bg, code, need_expand, 100000);
                },
            );
        }
    }
}

#[test]
fn cfg39_alpha_mode_fixed() {
    let list: Vec<(c_int, c_int)> = [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ]
    .to_vec();
    for mode in [
        PNG_ALPHA_PNG,
        PNG_ALPHA_STANDARD,
        PNG_ALPHA_OPTIMIZED,
        PNG_ALPHA_BROKEN,
    ] {
        for g in [PNG_FP_1, 220000, PNG_DEFAULT_sRGB] {
            let lbl = format!("cfg39 alpha_mode mode={mode} gamma={g}");
            over(&list, Extras::default(), &lbl, &move |l, pp, _i, _s| unsafe {
                png_set_alpha_mode_fixed(l, pp, mode, g)
            });
        }
    }
}

#[test]
fn cfg39_gamma_fixed() {
    let list: Vec<(c_int, c_int)> = [
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    ]
    .to_vec();
    for screen in [100000, 45455, 220000] {
        for file in [100000, 45455, 220000] {
            let lbl = format!("cfg39 gamma screen={screen} file={file}");
            over(&list, Extras::default(), &lbl, &move |l, pp, _i, _s| unsafe {
                png_set_gamma_fixed(l, pp, screen, file)
            });
        }
    }
}

#[test]
fn cfg39_quantize() {
    // palette input: quantize the file's own palette
    for max in [2, 16, 255] {
        for full in [0, 1] {
            for with_hist in [false, true] {
                let lbl = format!("cfg39 quantize pal max={max} full={full} hist={with_hist}");
                over(
                    &[(PNG_COLOR_TYPE_PALETTE, 8), (PNG_COLOR_TYPE_PALETTE, 4)],
                    Extras {
                        hist: true,
                        ..Extras::default()
                    },
                    &lbl,
                    &move |l, pp, ip, _s| unsafe {
                        let mut pal: *mut png_color = ptr::null_mut();
                        let mut np: c_int = 0;
                        if png_get_PLTE(l, pp, ip, &mut pal, &mut np) == 0 {
                            return;
                        }
                        // a caller-supplied histogram (an hIST chunk can never
                        // be read back - see `fixture_sanity`)
                        let hist: Vec<png_uint_16> =
                            (0..np.max(0) as usize).map(|i| (i * 11 + 1) as u16).collect();
                        let hp = if with_hist {
                            hist.as_ptr()
                        } else {
                            ptr::null()
                        };
                        png_set_quantize(l, pp, pal, np, max, hp, full);
                    },
                );
                // truecolor input: caller-supplied palette (modified in place)
                let lbl = format!("cfg39 quantize rgb max={max} full={full} hist={with_hist}");
                over(
                    &[(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_RGB_ALPHA, 8)],
                    Extras::default(),
                    &lbl,
                    &move |l, pp, _ip, _s| unsafe {
                        QPAL.with(|q| {
                            let mut q = q.borrow_mut();
                            q.clear();
                            let mut r = Rng::new(0x9e37);
                            for _ in 0..64 {
                                q.push(png_color {
                                    red: r.u8(),
                                    green: r.u8(),
                                    blue: r.u8(),
                                });
                            }
                        });
                        let hist: Vec<png_uint_16> = (0..64).map(|i| (i * 3 + 1) as u16).collect();
                        let (p, n) = QPAL.with(|q| {
                            let mut q = q.borrow_mut();
                            (q.as_mut_ptr(), q.len() as c_int)
                        });
                        let hp = if with_hist {
                            hist.as_ptr()
                        } else {
                            ptr::null()
                        };
                        png_set_quantize(l, pp, p, n, max, hp, full);
                    },
                );
            }
        }
    }
}

#[test]
fn cfg39_strip_16() {
    over(ALL, Extras::default(), "cfg39 strip_16", &|l, pp, _i, _s| unsafe {
        png_set_strip_16(l, pp)
    });
}

#[test]
fn cfg39_scale_16() {
    over(ALL, Extras::default(), "cfg39 scale_16", &|l, pp, _i, _s| unsafe {
        png_set_scale_16(l, pp)
    });
}

#[test]
fn cfg39_packing() {
    over(ALL, Extras::default(), "cfg39 packing", &|l, pp, _i, _s| unsafe {
        png_set_packing(l, pp)
    });
}

#[test]
fn cfg39_packswap() {
    over(ALL, Extras::default(), "cfg39 packswap", &|l, pp, _i, _s| unsafe {
        png_set_packswap(l, pp)
    });
}

#[test]
fn cfg39_swap() {
    over(ALL, Extras::default(), "cfg39 swap", &|l, pp, _i, _s| unsafe {
        png_set_swap(l, pp)
    });
}

#[test]
fn cfg39_swap_alpha() {
    over(ALL, Extras::default(), "cfg39 swap_alpha", &|l, pp, _i, _s| unsafe {
        png_set_swap_alpha(l, pp)
    });
}

#[test]
fn cfg39_invert_mono() {
    over(ALL, Extras::default(), "cfg39 invert_mono", &|l, pp, _i, _s| unsafe {
        png_set_invert_mono(l, pp)
    });
}

#[test]
fn cfg39_invert_alpha() {
    over(ALL, Extras::default(), "cfg39 invert_alpha", &|l, pp, _i, _s| unsafe {
        png_set_invert_alpha(l, pp)
    });
}

#[test]
fn cfg39_filler() {
    for flags in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
        let lbl = format!("cfg39 filler flags={flags}");
        over(ALL, Extras::default(), &lbl, &move |l, pp, _i, _s| unsafe {
            png_set_filler(l, pp, 0xa5, flags)
        });
    }
}

#[test]
fn cfg39_add_alpha() {
    for flags in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
        let lbl = format!("cfg39 add_alpha flags={flags}");
        over(ALL, Extras::default(), &lbl, &move |l, pp, _i, _s| unsafe {
            png_set_add_alpha(l, pp, 0x5a, flags)
        });
    }
}

#[test]
fn cfg39_shift() {
    let ex = Extras {
        sbit: true,
        ..Extras::default()
    };
    over(ALL, ex, "cfg39 shift", &|l, pp, _i, _s| unsafe {
        let sb = png_color_8 {
            red: 3,
            green: 2,
            blue: 1,
            gray: 2,
            alpha: 4,
        };
        png_set_shift(l, pp, &sb)
    });
}

#[test]
fn cfg39_strip_alpha() {
    over(ALL, Extras::default(), "cfg39 strip_alpha", &|l, pp, _i, _s| unsafe {
        png_set_strip_alpha(l, pp)
    });
}

#[test]
fn cfg39_bgr() {
    over(ALL, Extras::default(), "cfg39 bgr", &|l, pp, _i, _s| unsafe {
        png_set_bgr(l, pp)
    });
}

#[test]
fn cfg39_interlace_handling() {
    // `decode` already records the return value of png_set_interlace_handling
    // for the row-based modes; check it explicitly for both interlace types.
    for &(ct, bd) in &[(PNG_COLOR_TYPE_GRAY, 8), (PNG_COLOR_TYPE_RGB_ALPHA, 16)] {
        for il in [0, 1] {
            let png = make_png(ct, bd, il, 9, 7, 0x39_0002);
            for mode in [Mode::RowNull, Mode::RowDisplay, Mode::Rows] {
                diff(
                    &format!("cfg39 interlace_handling ct={ct} bd={bd} il={il} {mode:?}"),
                    &png,
                    Opt {
                        mode,
                        ..Opt::default()
                    },
                    &nohook,
                    &nohook,
                );
            }
        }
    }
}

#[test]
fn cfg39_read_user_transform_fn() {
    // in-place transform: user_transform_info announces no change
    over(ALL, Extras::default(), "cfg39 user_transform", &|l, pp, _i, s| unsafe {
        png_set_read_user_transform_fn(l, pp, Some(ut_cb));
        png_set_user_transform_info(l, pp, 0x1234usize as png_voidp, 0, 0);
        s.v(
            "ut",
            "ptr",
            (png_get_user_transform_ptr(l, pp) as usize) as i64,
        );
    });
    // size-changing transform: 1/2/4-bit gray -> 8-bit gray, one byte/pixel
    over(
        &[
            (PNG_COLOR_TYPE_GRAY, 1),
            (PNG_COLOR_TYPE_GRAY, 2),
            (PNG_COLOR_TYPE_GRAY, 4),
        ],
        Extras::default(),
        "cfg39 user_transform_expand",
        &|l, pp, _i, s| unsafe {
            png_set_read_user_transform_fn(l, pp, Some(ut_expand_cb));
            png_set_user_transform_info(l, pp, 0x4321usize as png_voidp, 8, 1);
            s.v(
                "ut",
                "ptr",
                (png_get_user_transform_ptr(l, pp) as usize) as i64,
            );
        },
    );
}

// ===========================================================================
// row 40 - transform combinations
// ===========================================================================

#[test]
fn cfg40_expand_gray_to_rgb_add_alpha_swap() {
    let ex = Extras {
        trns: true,
        ..Extras::default()
    };
    over(ALL, ex, "cfg40 expand+gray2rgb+add_alpha+swap", &|l, pp, _i, _s| unsafe {
        png_set_expand(l, pp);
        png_set_gray_to_rgb(l, pp);
        png_set_add_alpha(l, pp, 0x77, PNG_FILLER_AFTER);
        png_set_swap(l, pp);
    });
}

#[test]
fn cfg40_background_gamma_strip16() {
    let ex = Extras {
        gama: true,
        bkgd: true,
        ..Extras::default()
    };
    over(ALL, ex, "cfg40 background+gamma+strip16", &|l, pp, ip, _s| unsafe {
        let bg = valid_bg(
            png_get_color_type(l, pp, ip) as c_int,
            png_get_bit_depth(l, pp, ip) as c_int,
            0,
        );
        png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_FILE, 0, 100000);
        png_set_gamma_fixed(l, pp, 220000, 45455);
        png_set_strip_16(l, pp);
    });
}

#[test]
fn cfg40_palette_to_rgb_expand16_bgr() {
    over(PAL, Extras::default(), "cfg40 pal2rgb+expand16+bgr", &|l, pp, _i, _s| unsafe {
        png_set_palette_to_rgb(l, pp);
        png_set_expand_16(l, pp);
        png_set_bgr(l, pp);
    });
}

#[test]
fn cfg40_expand_then_quantize() {
    over(
        &[
            (PNG_COLOR_TYPE_PALETTE, 8),
            (PNG_COLOR_TYPE_GRAY, 4),
            (PNG_COLOR_TYPE_RGB, 8),
        ],
        Extras {
            trns: true,
            ..Extras::default()
        },
        "cfg40 expand+quantize",
        &|l, pp, _i, _s| unsafe {
            png_set_expand(l, pp);
            QPAL.with(|q| {
                let mut q = q.borrow_mut();
                q.clear();
                let mut r = Rng::new(0x1234_5678);
                for _ in 0..32 {
                    q.push(png_color {
                        red: r.u8(),
                        green: r.u8(),
                        blue: r.u8(),
                    });
                }
            });
            let (p, n) = QPAL.with(|q| {
                let mut q = q.borrow_mut();
                (q.as_mut_ptr(), q.len() as c_int)
            });
            png_set_quantize(l, pp, p, n, 16, ptr::null(), 1);
        },
    );
}

#[test]
fn cfg40_rgb_to_gray_background_alpha_mode() {
    over(
        &[
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB, 16),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        ],
        Extras {
            gama: true,
            ..Extras::default()
        },
        "cfg40 rgb2gray+background+alpha_mode",
        &|l, pp, _i: png_infop, _s| unsafe {
            png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_WARN, -1, -1);
            // png_set_alpha_mode must come first: png_set_background sets
            // PNG_COMPOSE and png_set_alpha_mode then reports "conflicting
            // calls to set alpha mode and background".
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_STANDARD, PNG_FP_1);
            let bg = valid_bg(
                png_get_color_type(l, pp, _i) as c_int,
                png_get_bit_depth(l, pp, _i) as c_int,
                1,
            );
            png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 1, 100000);
        },
    );
    // the reverse order, which libpng rejects - the error must match too
    over_may_err(
        &[
            (PNG_COLOR_TYPE_RGB, 8),
            (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        ],
        Extras {
            gama: true,
            ..Extras::default()
        },
        "cfg40 rgb2gray+background+alpha_mode (conflicting order)",
        &|l, pp, _i: png_infop, _s| unsafe {
            png_set_rgb_to_gray_fixed(l, pp, PNG_ERROR_ACTION_WARN, -1, -1);
            let bg = valid_bg(
                png_get_color_type(l, pp, _i) as c_int,
                png_get_bit_depth(l, pp, _i) as c_int,
                1,
            );
            png_set_background_fixed(l, pp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 1, 100000);
            png_set_alpha_mode_fixed(l, pp, PNG_ALPHA_STANDARD, PNG_FP_1);
        },
    );
}

#[test]
fn cfg40_strip_alpha_packing_packswap_invert_mono() {
    over(
        ALL,
        Extras::default(),
        "cfg40 strip_alpha+packing+packswap+invert_mono",
        &|l, pp, _i, _s| unsafe {
            png_set_strip_alpha(l, pp);
            png_set_packing(l, pp);
            png_set_packswap(l, pp);
            png_set_invert_mono(l, pp);
        },
    );
}

/// Randomised transform combinations (fixed seed, so the case list is stable).
/// The transforms are always *applied* in a legal order; only the subset is
/// random.  Cases that libpng rejects are compared through their error message.
#[test]
fn cfg40_random_transform_combos() {
    let mut rng = Rng::new(0xc0ffee_1234_5678);
    for case in 0..40000u32 {
        let (ct, bd) = CT_BD[rng.below(CT_BD.len() as u32) as usize];
        let il = rng.below(2) as c_int;
        let (w, h) = TX_SHAPES[rng.below(TX_SHAPES.len() as u32) as usize];
        let ex = Extras {
            trns: rng.below(2) == 1,
            gama: rng.below(2) == 1,
            bkgd: rng.below(2) == 1,
            sbit: rng.below(2) == 1,
            filter: [
                0,
                PNG_FILTER_NONE,
                PNG_FILTER_SUB,
                PNG_FILTER_PAETH,
                PNG_ALL_FILTERS,
            ][rng.below(5) as usize],
            ..Extras::default()
        };
        // 24 independent switches
        let bits = rng.u32() ^ (rng.u32() << 1);
        let gsel = rng.below(3) as usize;
        let asel = rng.below(4) as usize;
        let msel = rng.below(3) as usize;
        let qmax = [2i32, 16, 255][rng.below(3) as usize];
        let png = make_png_ex(ct, bd, il, w, h, 0x40_1000 + case as u64, ex);
        let hook = move |l: &Library, pp: png_structp, ip: png_infop, _s: &mut Snap| unsafe {
            if bits & 1 != 0 {
                png_set_expand(l, pp);
            }
            if bits & 2 != 0 {
                png_set_palette_to_rgb(l, pp);
            }
            if bits & 4 != 0 {
                png_set_tRNS_to_alpha(l, pp);
            }
            if bits & 8 != 0 {
                png_set_expand_gray_1_2_4_to_8(l, pp);
            }
            if bits & 0x10 != 0 {
                png_set_expand_16(l, pp);
            }
            if bits & 0x20 != 0 {
                png_set_gray_to_rgb(l, pp);
            }
            if bits & 0x40 != 0 {
                let (r, g) = [(-1i32, -1i32), (30000, 50000), (0, 100000)][gsel];
                // never ERROR_ACTION_ERROR here: it would reject nearly every
                // fixture and hide the rest of the combination
                png_set_rgb_to_gray_fixed(l, pp, 1 + (gsel as c_int % 2), r, g);
            }
            if bits & 0x80 != 0 {
                let m = [
                    PNG_ALPHA_PNG,
                    PNG_ALPHA_STANDARD,
                    PNG_ALPHA_OPTIMIZED,
                    PNG_ALPHA_BROKEN,
                ][asel];
                let g = [PNG_FP_1, 220000, PNG_DEFAULT_sRGB][msel];
                png_set_alpha_mode_fixed(l, pp, m, g);
            }
            if bits & 0x100 != 0 {
                let ne = (bits >> 9 & 1) as c_int;
                let bg = valid_bg(
                    png_get_color_type(l, pp, ip) as c_int,
                    png_get_bit_depth(l, pp, ip) as c_int,
                    ne,
                );
                let code = [
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    PNG_BACKGROUND_GAMMA_FILE,
                    PNG_BACKGROUND_GAMMA_UNIQUE,
                ][msel];
                png_set_background_fixed(l, pp, &bg, code, ne, 100000);
            }
            if bits & 0x400 != 0 {
                png_set_gamma_fixed(
                    l,
                    pp,
                    [100000, 45455, 220000][gsel],
                    [100000, 45455, 220000][msel],
                );
            }
            if bits & 0x800 != 0 {
                QPAL.with(|q| {
                    let mut q = q.borrow_mut();
                    q.clear();
                    let mut r = Rng::new(0x5eed_0000 + case as u64);
                    for _ in 0..64 {
                        q.push(png_color {
                            red: r.u8(),
                            green: r.u8(),
                            blue: r.u8(),
                        });
                    }
                });
                let (p, n) = QPAL.with(|q| {
                    let mut q = q.borrow_mut();
                    (q.as_mut_ptr(), q.len() as c_int)
                });
                png_set_quantize(l, pp, p, n, qmax, ptr::null(), (bits >> 12 & 1) as c_int);
            }
            if bits & 0x2000 != 0 {
                png_set_strip_16(l, pp);
            }
            if bits & 0x4000 != 0 {
                png_set_scale_16(l, pp);
            }
            if bits & 0x8000 != 0 {
                png_set_strip_alpha(l, pp);
            }
            if bits & 0x1_0000 != 0 {
                png_set_packing(l, pp);
            }
            if bits & 0x2_0000 != 0 {
                png_set_packswap(l, pp);
            }
            if bits & 0x4_0000 != 0 {
                png_set_swap(l, pp);
            }
            if bits & 0x8_0000 != 0 {
                png_set_swap_alpha(l, pp);
            }
            if bits & 0x10_0000 != 0 {
                png_set_invert_alpha(l, pp);
            }
            if bits & 0x20_0000 != 0 {
                png_set_invert_mono(l, pp);
            }
            if bits & 0x40_0000 != 0 {
                png_set_bgr(l, pp);
            }
            if bits & 0x80_0000 != 0 {
                if bits & 0x100_0000 != 0 {
                    png_set_add_alpha(l, pp, 0x33, (bits >> 25 & 1) as c_int);
                } else {
                    png_set_filler(l, pp, 0x44, (bits >> 25 & 1) as c_int);
                }
            }
            if bits & 0x200_0000 != 0 {
                let mut sb: *mut png_color_8 = ptr::null_mut();
                if png_get_sBIT(l, pp, ip, &mut sb) != 0 && !sb.is_null() {
                    png_set_shift(l, pp, sb);
                }
            }
        };
        let lbl = format!(
            "cfg40 random case={case} ct={ct} bd={bd} il={il} {w}x{h} bits={bits:#x}"
        );
        diff(&lbl, &png, Opt::default(), &nohook, &hook);
        if case % 8 == 0 {
            diff(
                &format!("{lbl} display"),
                &png,
                Opt {
                    mode: Mode::RowDisplay,
                    ..Opt::default()
                },
                &nohook,
                &hook,
            );
        }
    }
}

// ===========================================================================
// row 41 - update_info / start_read_image ordering
// ===========================================================================

#[test]
fn cfg41_update_info_ordering() {
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [0, 1] {
            let png = make_png(ct, bd, il, 9, 7, 0x41_0003);
            for updates in [0u32, 1, 2] {
                for start_img in [false, true] {
                    for mode in [Mode::Image, Mode::NoRows] {
                        let lbl = format!(
                            "cfg41 ct={ct} bd={bd} il={il} updates={updates} start={start_img} {mode:?}"
                        );
                        diff(
                            &lbl,
                            &png,
                            Opt {
                                mode,
                                updates,
                                start_img,
                                ..Opt::default()
                            },
                            &nohook,
                            &nohook,
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// rows 42-44 - progressive read
// ===========================================================================

#[derive(Default)]
struct Prog {
    vals: Vec<(String, i64)>,
    notes: Vec<String>,
    rows: Vec<Vec<u8>>,
    disp: Vec<Vec<u8>>,
    combine: bool,
    interlace_handling: bool,
    do_skip: bool,
    pause_save: Option<c_int>,
    paused_once: bool,
    rewind: usize,
    rowbytes: usize,
    pixel_depth: usize,
    width: u32,
    height: u32,
    il: c_int,
    ends: u32,
    infos: u32,
}

thread_local! {
    static PG: RefCell<Prog> = RefCell::new(Prog::default());
}

unsafe extern "C-unwind" fn pinfo_cb(pp: png_structp, ip: png_infop) {
    let l = curlib();
    let want_ih = PG.with(|g| {
        let mut g = g.borrow_mut();
        g.infos += 1;
        g.combine || g.interlace_handling
    });
    let mut s = Snap::default();
    FILE_CT.with(|c| c.set(png_get_color_type(l, pp, ip) as c_int));
    snap_hdr(l, pp, ip, &mut s, "pinfo");
    if want_ih {
        let n = png_set_interlace_handling(l, pp);
        s.v("pinfo", "passes", n as i64);
    }
    let do_skip = PG.with(|g| g.borrow().do_skip);
    if do_skip {
        let r = png_process_data_skip(l, pp);
        s.v("pinfo", "skip_ret", r as i64);
    }
    png_read_update_info(l, pp, ip);
    snap_hdr(l, pp, ip, &mut s, "pupd");
    // `png_process_data_pause` may only be called from within
    // `png_process_data`; the info callback is a safe point (the IDAT branch of
    // png_push_read_chunk returns immediately afterwards).  See the note on
    // `prow_cb` for why the row callback is *not* usable.
    let save = PG.with(|g| {
        let mut g = g.borrow_mut();
        if g.paused_once {
            None
        } else {
            let v = g.pause_save;
            if v.is_some() {
                g.paused_once = true;
            }
            v
        }
    });
    if let Some(save) = save {
        let r = png_process_data_pause(l, pp, save);
        s.v("pinfo", "pause_ret", r as i64);
        if save == 0 {
            PG.with(|g| g.borrow_mut().rewind = r);
        }
    }
    let w = png_get_image_width(l, pp, ip);
    let h = png_get_image_height(l, pp, ip);
    let n = png_get_rowbytes(l, pp, ip);
    let ch = png_get_channels(l, pp, ip) as usize;
    let bd = png_get_bit_depth(l, pp, ip) as usize;
    let il = png_get_interlace_type(l, pp, ip) as c_int;
    PG.with(|g| {
        let mut g = g.borrow_mut();
        g.vals.extend(s.vals.drain(..));
        g.notes.extend(s.notes.drain(..));
        g.rowbytes = n;
        g.pixel_depth = ch * bd;
        g.width = w;
        g.height = h;
        g.il = il;
        if g.combine {
            g.disp = (0..h.max(1) as usize).map(|_| vec![0u8; n.max(1)]).collect();
        }
    });
}

unsafe extern "C-unwind" fn pend_cb(pp: png_structp, ip: png_infop) {
    let l = curlib();
    let mut s = Snap::default();
    snap_hdr(l, pp, ip, &mut s, "pend");
    PG.with(|g| {
        let mut g = g.borrow_mut();
        g.ends += 1;
        g.vals.extend(s.vals.drain(..));
        g.notes.extend(s.notes.drain(..));
    });
}

/// NOTE: `png_process_data_pause` is deliberately *not* called from here.
/// When invoked from the row callback the C reference library zeroes
/// `png_struct::buffer_size` while `png_push_read_IDAT` is still executing;
/// on return `png_push_read_IDAT` performs `buffer_size -= save_size`, which
/// underflows and makes `png_process_data`'s `while (buffer_size)` loop spin
/// forever.  The C library therefore cannot act as an oracle for that case.
unsafe extern "C-unwind" fn prow_cb(
    pp: png_structp,
    new_row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    let l = curlib();
    let (combine, rowbytes, pd, width, il) = PG.with(|g| {
        let g = g.borrow();
        (g.combine, g.rowbytes, g.pixel_depth, g.width, g.il)
    });
    if combine {
        let dp = PG.with(|g| {
            let mut g = g.borrow_mut();
            let i = row_num as usize;
            if i < g.disp.len() {
                g.disp[i].as_mut_ptr()
            } else {
                ptr::null_mut()
            }
        });
        if !dp.is_null() {
            png_progressive_combine_row(l, pp, dp, new_row);
        }
        PG.with(|g| {
            let mut g = g.borrow_mut();
            g.notes
                .push(format!("prow row={row_num} pass={pass} null={}", new_row.is_null()));
            let i = row_num as usize;
            if i < g.disp.len() {
                let snap = g.disp[i].clone();
                g.rows.push(snap);
            }
        });
    } else {
        let n = if new_row.is_null() {
            0
        } else if il != 0 {
            png_rowbytes(pd, pass_cols(width, pass.clamp(0, 6) as usize))
        } else {
            rowbytes
        };
        let data = if new_row.is_null() || n == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(new_row, n).to_vec()
        };
        PG.with(|g| {
            let mut g = g.borrow_mut();
            g.notes
                .push(format!("prow row={row_num} pass={pass} null={}", new_row.is_null()));
            g.rows.push(data);
        });
    }
}

#[derive(Clone, Copy)]
struct ProgOpt {
    granule: usize,
    combine: bool,
    interlace_handling: bool,
    pause_save: Option<c_int>,
    do_skip: bool,
}

impl Default for ProgOpt {
    fn default() -> Self {
        ProgOpt {
            granule: usize::MAX,
            combine: false,
            interlace_handling: false,
            pause_save: None,
            do_skip: false,
        }
    }
}

unsafe fn prog_decode(l: &Library, png: &[u8], o: ProgOpt) -> Snap {
    CURLIB.with(|c| c.set(l as *const Library));
    PG.with(|g| {
        let mut g = g.borrow_mut();
        *g = Prog::default();
        g.combine = o.combine;
        g.interlace_handling = o.interlace_handling || o.combine;
        g.pause_save = o.pause_save;
        g.do_skip = o.do_skip;
    });
    let mut s = Snap::default();
    let pp = png_create_read_struct(
        l,
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
    );
    assert!(!pp.is_null());
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    png_set_progressive_read_fn(
        l,
        pp,
        7usize as png_voidp,
        Some(pinfo_cb),
        Some(prow_cb),
        Some(pend_cb),
    );
    s.v(
        "prog",
        "progressive_ptr",
        (png_get_progressive_ptr(l, pp) as usize) as i64,
    );
    let mut buf = png.to_vec();
    let len = buf.len();
    let mut pos = 0usize;
    let mut guard = 0usize;
    while pos < len {
        let n = o.granule.min(len - pos);
        png_process_data(l, pp, ip, buf[pos..].as_mut_ptr(), n);
        pos += n;
        let rw = PG.with(|g| std::mem::replace(&mut g.borrow_mut().rewind, 0));
        if rw > 0 && rw <= pos {
            pos -= rw;
        }
        guard += 1;
        if guard > 4 * len + 64 {
            s.note("prog: feed loop guard tripped".to_string());
            break;
        }
    }
    snap_hdr(l, pp, ip, &mut s, "final");
    PG.with(|g| {
        let g = g.borrow();
        s.v("prog", "infos", g.infos as i64);
        s.v("prog", "ends", g.ends as i64);
        s.vals.extend(g.vals.iter().cloned());
        s.notes.extend(g.notes.iter().cloned());
        for r in g.rows.iter() {
            s.rows.push(r.clone());
        }
        for r in g.disp.iter() {
            s.rows.push(r.clone());
        }
    });
    let mut p = pp;
    let mut i = ip;
    png_destroy_read_struct(l, &mut p, &mut i, ptr::null_mut());
    s
}

fn diff_prog(label: &str, png: &[u8], o: ProgOpt) -> bool {
    let c = capture(|| unsafe { prog_decode(&L().c, png, o) });
    let r = capture(|| unsafe { prog_decode(&L().rs, png, o) });
    assert_snap(&c, &r, label);
    if let Some(sn) = &c.out {
        assert!(
            sn.vals.iter().any(|(k, v)| k == "prog.ends" && *v == 1),
            "{label}: the C progressive end callback never fired"
        );
        assert!(
            sn.rows.iter().any(|r| !r.is_empty()),
            "{label}: the C progressive row callback delivered no data"
        );
    }
    c.out.is_some()
}

#[test]
fn cfg42_progressive_read() {
    for &(ct, bd) in CT_BD {
        for il in [0, 1] {
            for &(w, h) in &[(9u32, 7u32), (1, 1), (33, 5)] {
                let png = make_png(ct, bd, il, w, h, 0x42_0004);
                for granule in [1usize, 2, 3, 7, 13, usize::MAX] {
                    let g = if granule == usize::MAX {
                        "all".to_string()
                    } else {
                        granule.to_string()
                    };
                    diff_prog(
                        &format!("cfg42 ct={ct} bd={bd} il={il} {w}x{h} granule={g}"),
                        &png,
                        ProgOpt {
                            granule,
                            ..ProgOpt::default()
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn cfg42_progressive_read_interlace_handling() {
    // same, but with png_set_interlace_handling() enabled in the info callback
    for &(ct, bd) in CT_BD {
        for il in [0, 1] {
            for &(w, h) in &[(9u32, 7u32), (17, 9)] {
                let png = make_png(ct, bd, il, w, h, 0x42_0007);
                for granule in [1usize, 5, usize::MAX] {
                    diff_prog(
                        &format!("cfg42 ih ct={ct} bd={bd} il={il} {w}x{h} g={granule}"),
                        &png,
                        ProgOpt {
                            granule,
                            interlace_handling: true,
                            ..ProgOpt::default()
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn cfg43_process_data_pause_and_skip() {
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [0, 1] {
            let png = make_png(ct, bd, il, 17, 11, 0x43_0005);
            for save in [0, 1] {
                for granule in [7usize, 29] {
                    diff_prog(
                        &format!("cfg43 pause save={save} ct={ct} bd={bd} il={il} g={granule}"),
                        &png,
                        ProgOpt {
                            granule,
                            pause_save: Some(save),
                            ..ProgOpt::default()
                        },
                    );
                }
            }
            diff_prog(
                &format!("cfg43 skip ct={ct} bd={bd} il={il}"),
                &png,
                ProgOpt {
                    granule: 64,
                    do_skip: true,
                    ..ProgOpt::default()
                },
            );
        }
    }
}

#[test]
fn cfg44_progressive_combine_row() {
    for &(ct, bd) in CT_BD {
        for &(w, h) in &[(17u32, 9u32), (1, 1), (8, 8), (33, 5)] {
            let png = make_png(ct, bd, PNG_INTERLACE_ADAM7, w, h, 0x44_0006);
            for granule in [3usize, usize::MAX] {
                diff_prog(
                    &format!("cfg44 combine ct={ct} bd={bd} {w}x{h} g={granule}"),
                    &png,
                    ProgOpt {
                        granule,
                        combine: true,
                        ..ProgOpt::default()
                    },
                );
            }
        }
        let png = make_png(ct, bd, PNG_INTERLACE_ADAM7, 17, 9, 0x44_0006);
        for granule in [3usize, usize::MAX] {
            diff_prog(
                &format!("cfg44 combine ct={ct} bd={bd} g={granule}"),
                &png,
                ProgOpt {
                    granule,
                    combine: true,
                    ..ProgOpt::default()
                },
            );
        }
    }
    // non-interlaced control
    for &(ct, bd) in &[(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_GRAY, 2)] {
        let png = make_png(ct, bd, PNG_INTERLACE_NONE, 17, 9, 0x44_0006);
        diff_prog(
            &format!("cfg44 combine-noninterlaced ct={ct} bd={bd}"),
            &png,
            ProgOpt {
                combine: true,
                ..ProgOpt::default()
            },
        );
    }
}

// ===========================================================================
// row 45 - crc actions
// ===========================================================================

#[test]
fn cfg45_crc_action() {
    let base = make_png_ex(
        PNG_COLOR_TYPE_RGB,
        8,
        0,
        9,
        7,
        0x45_0007,
        Extras {
            text: true,
            ..Extras::default()
        },
    );
    let bad_idat = corrupt_crc(&base, b"IDAT");
    let bad_text = corrupt_crc(&base, b"tEXt");
    let acts = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
    ];
    for crit in acts {
        for ancil in acts {
            for (nm, png) in [("idat", &bad_idat), ("text", &bad_text)] {
                let lbl = format!("cfg45 {nm} crit={crit} ancil={ancil}");
                diff(
                    &lbl,
                    png,
                    Opt {
                        full: true,
                        ..Opt::default()
                    },
                    &move |l, pp, _i, _s| unsafe { png_set_crc_action(l, pp, crit, ancil) },
                    &nohook,
                );
            }
        }
    }
}

// ===========================================================================
// row 46 - benign errors
// ===========================================================================

#[test]
fn cfg46_benign_errors() {
    let base = make_png(PNG_COLOR_TYPE_GRAY, 8, 0, 9, 7, 0x46_0008);
    // a bKGD chunk with the wrong data length for a grayscale image
    let bad_bkgd = insert_chunk_after(&base, b"IHDR", &mk_chunk(b"bKGD", &[0x12]));
    // a tEXt chunk with no NUL separator at all
    let bad_text = insert_chunk_after(&base, b"IHDR", &mk_chunk(b"tEXt", b"nokeysep"));
    // an sBIT chunk that is too long
    let bad_sbit = insert_chunk_after(&base, b"IHDR", &mk_chunk(b"sBIT", &[1, 2, 3, 4, 5]));
    // a truncated zTXt (bad deflate stream)
    let bad_ztxt = insert_chunk_after(&base, b"IHDR", &mk_chunk(b"zTXt", b"k\0\0\xff\xff\xff"));
    for allowed in [0, 1] {
        for (nm, png) in [
            ("bkgd", &bad_bkgd),
            ("text", &bad_text),
            ("sbit", &bad_sbit),
            ("ztxt", &bad_ztxt),
        ] {
            let lbl = format!("cfg46 {nm} benign={allowed}");
            diff(
                &lbl,
                png,
                Opt {
                    full: true,
                    ..Opt::default()
                },
                &move |l, pp, _i, _s| unsafe { png_set_benign_errors(l, pp, allowed) },
                &nohook,
            );
        }
    }
}

// ===========================================================================
// row 47 - user limits
// ===========================================================================

#[test]
fn cfg47_user_limits() {
    let png = make_png_ex(
        PNG_COLOR_TYPE_RGB,
        8,
        0,
        9,
        7,
        0x47_0009,
        Extras {
            rich: true,
            ..Extras::default()
        },
    );
    for (w, h) in [(8u32, 6u32), (9, 7), (10, 8), (1, 1), (1000000, 1000000)] {
        let lbl = format!("cfg47 user_limits w={w} h={h}");
        diff(
            &lbl,
            &png,
            Opt {
                full: true,
                ..Opt::default()
            },
            &move |l, pp, _i, _s| unsafe { png_set_user_limits(l, pp, w, h) },
            &nohook,
        );
    }
    for n in [0u32, 1, 2, 3, 8, 1000] {
        let lbl = format!("cfg47 chunk_cache_max={n}");
        diff(
            &lbl,
            &png,
            Opt {
                full: true,
                ..Opt::default()
            },
            &move |l, pp, _i, _s| unsafe { png_set_chunk_cache_max(l, pp, n) },
            &nohook,
        );
    }
    for n in [0usize, 1, 8, 16, 132, 1024, 8_000_000] {
        let lbl = format!("cfg47 chunk_malloc_max={n}");
        diff(
            &lbl,
            &png,
            Opt {
                full: true,
                ..Opt::default()
            },
            &move |l, pp, _i, _s| unsafe { png_set_chunk_malloc_max(l, pp, n) },
            &nohook,
        );
    }
}

// ===========================================================================
// row 48 - read user chunk callback
// ===========================================================================

#[test]
fn cfg48_read_user_chunk_fn() {
    let base = make_png(PNG_COLOR_TYPE_RGB, 8, 0, 9, 7, 0x48_000a);
    let png = insert_chunk_after(&base, b"IHDR", &mk_chunk(b"prVt", b"private-chunk-data"));
    let png = insert_chunk_before_iend(&png, &mk_chunk(b"prVt", b"tail"));
    for ret in [-1, 0, 1] {
        let lbl = format!("cfg48 user_chunk ret={ret}");
        UCHUNK_RET.with(|x| x.set(ret));
        diff(
            &lbl,
            &png,
            Opt {
                full: true,
                ..Opt::default()
            },
            &move |l, pp, _i, _s| unsafe {
                UCHUNK_RET.with(|x| x.set(ret));
                png_set_read_user_chunk_fn(l, pp, 0x99usize as png_voidp, Some(user_chunk_cb));
            },
            &nohook,
        );
    }
    UCHUNK_RET.with(|x| x.set(0));
}

// ===========================================================================
// row 49 - keep_unknown_chunks
// ===========================================================================

#[test]
fn cfg49_keep_unknown_chunks() {
    let base = make_png(PNG_COLOR_TYPE_RGB, 8, 0, 9, 7, 0x49_000b);
    let png = insert_chunk_after(&base, b"IHDR", &mk_chunk(b"unSf", b"safe-copy"));
    let png = insert_chunk_after(&png, b"IHDR", &mk_chunk(b"unSF", b"unsafe-copy"));
    let png = insert_chunk_before_iend(&png, &mk_chunk(b"enDx", b"after-idat"));
    for keep in [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ] {
        for per_chunk in [false, true] {
            let lbl = format!("cfg49 keep={keep} per_chunk={per_chunk}");
            diff(
                &lbl,
                &png,
                Opt {
                    full: true,
                    ..Opt::default()
                },
                &move |l, pp, _i, s| unsafe {
                    if per_chunk {
                        let list: Vec<u8> = b"unSf\0unSF\0".to_vec();
                        png_set_keep_unknown_chunks(l, pp, keep, list.as_ptr(), 2);
                    } else {
                        png_set_keep_unknown_chunks(l, pp, keep, ptr::null(), 0);
                    }
                    for nm in [b"unSf", b"unSF", b"enDx", b"tEXt"] {
                        let mut z = nm.to_vec();
                        z.push(0);
                        s.v(
                            "hau",
                            &String::from_utf8_lossy(nm),
                            png_handle_as_unknown(l, pp, z.as_ptr()) as i64,
                        );
                        s.v(
                            "cuh",
                            &String::from_utf8_lossy(nm),
                            png_chunk_unknown_handling(l, pp, chunk_name_u32(nm)) as i64,
                        );
                    }
                },
                &nohook,
            );
        }
    }
}

// ===========================================================================
// row 50 - png_set_option
// ===========================================================================

#[test]
fn cfg50_set_option() {
    let png = make_png_ex(
        PNG_COLOR_TYPE_RGB,
        8,
        0,
        9,
        7,
        0x50_000c,
        Extras {
            srgb: true,
            text: true,
            ..Extras::default()
        },
    );
    for opt in [
        PNG_MAXIMUM_INFLATE_WINDOW,
        PNG_SKIP_sRGB_CHECK_PROFILE,
        PNG_IGNORE_ADLER32,
    ] {
        for onoff in [PNG_OPTION_OFF, PNG_OPTION_ON] {
            let lbl = format!("cfg50 option={opt} onoff={onoff}");
            diff(
                &lbl,
                &png,
                Opt {
                    full: true,
                    ..Opt::default()
                },
                &move |l, pp, _i, s| unsafe {
                    let r = png_set_option(l, pp, opt, onoff);
                    s.v("opt", "set_option", r as i64);
                    let r2 = png_set_option(l, pp, opt, onoff);
                    s.v("opt", "set_option_again", r2 as i64);
                },
                &nohook,
            );
        }
    }
    // iCCP stream with the sRGB-profile check skipped / not skipped
    let iccp = make_png_ex(
        PNG_COLOR_TYPE_RGB,
        8,
        0,
        9,
        7,
        0x50_000d,
        Extras {
            iccp: true,
            ..Extras::default()
        },
    );
    for onoff in [PNG_OPTION_OFF, PNG_OPTION_ON] {
        let lbl = format!("cfg50 iccp skip_srgb onoff={onoff}");
        diff(
            &lbl,
            &iccp,
            Opt {
                full: true,
                ..Opt::default()
            },
            &move |l, pp, _i, s| unsafe {
                let r = png_set_option(l, pp, PNG_SKIP_sRGB_CHECK_PROFILE, onoff);
                s.v("opt", "set_option", r as i64);
            },
            &nohook,
        );
    }
}

// ===========================================================================
// row 51 - MNG features
// ===========================================================================

/// A filter-method-64 (MNG intrapixel differencing) stream.  `png_check_IHDR`
/// only accepts filter method 64 when libpng has *not* seen a PNG signature, so
/// the signature is stripped and the reader is told about it with
/// `png_set_sig_bytes(pp, 8)`.
fn mng_stream(ct: c_int) -> Vec<u8> {
    let base = make_png(ct, 8, 0, 9, 7, 0x51_000e);
    patch_ihdr_filter(&base, 64)[8..].to_vec()
}

#[test]
fn cfg51_permit_mng_features() {
    let base = make_png(PNG_COLOR_TYPE_RGB, 8, 0, 9, 7, 0x51_000e);
    let nosig = base[8..].to_vec();
    for feats in [0u32, PNG_FLAG_MNG_FILTER_64 as u32, PNG_ALL_MNG_FEATURES as u32] {
        let mut cases: Vec<(String, Vec<u8>, Option<c_int>)> = vec![
            ("plain".to_string(), base.clone(), None),
            ("plain_nosig".to_string(), nosig.clone(), Some(8)),
        ];
        for ct in [
            PNG_COLOR_TYPE_RGB,
            PNG_COLOR_TYPE_RGB_ALPHA,
            PNG_COLOR_TYPE_GRAY,
        ] {
            cases.push((format!("filter64_ct{ct}"), mng_stream(ct), Some(8)));
        }
        for (nm, png, sig) in cases {
            let lbl = format!("cfg51 {nm} feats={feats}");
            diff(
                &lbl,
                &png,
                Opt {
                    full: true,
                    sig_bytes: sig,
                    ..Opt::default()
                },
                &move |l, pp, _i, s| unsafe {
                    let r = png_permit_mng_features(l, pp, feats);
                    s.v("mng", "permit_ret", r as i64);
                },
                &nohook,
            );
        }
    }
}

// ===========================================================================
// row 54 - every getter on a stream containing every chunk
// ===========================================================================

#[test]
fn cfg54_all_getters() {
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    let rich = Extras {
        rich: true,
        ..Extras::default()
    };
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [0, 1] {
            inputs.push((
                format!("rich ct={ct} bd={bd} il={il}"),
                make_png_ex(ct, bd, il, 9, 7, 0x54_000f, rich),
            ));
        }
    }
    for &(ct, bd) in &[(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_RGB_ALPHA, 16)] {
        inputs.push((
            format!("hdr ct={ct} bd={bd}"),
            make_png_ex(
                ct,
                bd,
                0,
                9,
                7,
                0x54_0020,
                Extras {
                    hdr: true,
                    text: true,
                    ..Extras::default()
                },
            ),
        ));
    }
    inputs.push((
        "srgb".to_string(),
        make_png_ex(
            PNG_COLOR_TYPE_RGB,
            8,
            0,
            9,
            7,
            0x54_0010,
            Extras {
                srgb: true,
                text: true,
                ..Extras::default()
            },
        ),
    ));
    inputs.push((
        "iccp-rgb".to_string(),
        make_png_ex(
            PNG_COLOR_TYPE_RGB,
            8,
            0,
            9,
            7,
            0x54_0011,
            Extras {
                iccp: true,
                ..Extras::default()
            },
        ),
    ));
    inputs.push((
        "iccp-gray".to_string(),
        make_png_ex(
            PNG_COLOR_TYPE_GRAY,
            8,
            0,
            9,
            7,
            0x54_0012,
            Extras {
                iccp: true,
                ..Extras::default()
            },
        ),
    ));
    for (nm, png) in &inputs {
        diff(
            &format!("cfg54 {nm}"),
            png,
            Opt {
                full: true,
                ..Opt::default()
            },
            &|l, pp, _i, _s| unsafe {
                png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
            },
            &nohook,
        );
    }
}

// ===========================================================================
// row 55 - png_set_rows + png_read_png
// ===========================================================================

unsafe fn read_png_with_rows(l: &Library, png: &[u8], w: u32, h: u32, ct: c_int, bd: c_int) -> Snap {
    let mut s = Snap::default();
    reset_recorders();
    CURLIB.with(|c| c.set(l as *const Library));
    src_set(png);
    let pp = new_reader(l);
    let ip = png_create_info_struct(l, pp);
    assert!(!ip.is_null());
    let n = rb(w, ct, bd);
    let mut store: Vec<Vec<u8>> = (0..h.max(1)).map(|_| vec![0xccu8; n.max(1)]).collect();
    let mut rp: Vec<png_bytep> = store.iter_mut().map(|r| r.as_mut_ptr()).collect();
    png_set_rows(l, pp, ip, rp.as_mut_ptr());
    png_read_png(l, pp, ip, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
    FILE_CT.with(|c| c.set(png_get_color_type(l, pp, ip) as c_int));
    snap_full(l, pp, ip, &mut s, "after");
    let got = png_get_rows(l, pp, ip);
    s.v("rows", "same_ptr", (got == rp.as_mut_ptr()) as i64);
    for r in &store {
        s.rows.push(r.clone());
    }
    s.note(format!("src_pos={}", src_pos()));
    let mut p = pp;
    let mut i = ip;
    png_destroy_read_struct(l, &mut p, &mut i, ptr::null_mut());
    s
}

#[test]
fn cfg55_set_rows_read_png() {
    for &(ct, bd) in CT_BD {
        for il in [0, 1] {
            for &(w, h) in &[(9u32, 7u32), (1, 1), (33, 5)] {
                let png = make_png(ct, bd, il, w, h, 0x55_0013);
                let c = capture(|| unsafe {
                    read_png_with_rows(&L().c, &png, w, h, ct, bd)
                });
                let r = capture(|| unsafe {
                    read_png_with_rows(&L().rs, &png, w, h, ct, bd)
                });
                assert_snap(
                    &c,
                    &r,
                    &format!("cfg55 ct={ct} bd={bd} il={il} {w}x{h}"),
                );
            }
        }
    }
}

// ===========================================================================
// row 56 - check_for_invalid_index
// ===========================================================================

#[test]
fn cfg56_check_for_invalid_index() {
    for bd in [4, 8] {
        for il in [0, 1] {
            let png = make_png_ex(
                PNG_COLOR_TYPE_PALETTE,
                bd,
                il,
                9,
                7,
                0x56_0014,
                Extras {
                    bad_index: true,
                    ..Extras::default()
                },
            );
            for allowed in [0, 1] {
                let lbl = format!("cfg56 bd={bd} il={il} allowed={allowed}");
                diff(
                    &lbl,
                    &png,
                    Opt {
                        full: true,
                        ..Opt::default()
                    },
                    &move |l, pp, _i, _s| unsafe {
                        png_set_check_for_invalid_index(l, pp, allowed)
                    },
                    &nohook,
                );
            }
        }
    }
}

// ===========================================================================
// row 64 - read status callback
// ===========================================================================

#[test]
fn cfg64_read_status_fn() {
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [0, 1] {
            for &(w, h) in &[(9u32, 7u32), (33, 5)] {
                let png = make_png(ct, bd, il, w, h, 0x64_0015);
                for mode in [Mode::Image, Mode::RowNull, Mode::Rows] {
                    let lbl =
                        format!("cfg64 status ct={ct} bd={bd} il={il} {w}x{h} {mode:?}");
                    diff(
                        &lbl,
                        &png,
                        Opt {
                            mode,
                            ..Opt::default()
                        },
                        &nohook,
                        &|l, pp, _i, _s| unsafe {
                            png_set_read_status_fn(l, pp, Some(status_cb))
                        },
                    );
                }
            }
        }
    }
}

// ===========================================================================
// row 65 - io_state / io_chunk_type sampled in the read callback
// ===========================================================================

#[test]
fn cfg65_io_state_in_read_callback() {
    let rich = Extras {
        rich: true,
        ..Extras::default()
    };
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [0, 1] {
            for ex in [Extras::default(), rich] {
                let png = make_png_ex(ct, bd, il, 9, 7, 0x65_0016, ex);
                let lbl = format!("cfg65 ct={ct} bd={bd} il={il} rich={}", ex.rich);
                diff(
                    &lbl,
                    &png,
                    Opt {
                        io_hook: true,
                        full: true,
                        ..Opt::default()
                    },
                    &|l, pp, _i, _s| unsafe {
                        png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
                    },
                    &nohook,
                );
            }
        }
    }
}

// ===========================================================================
// row 70 - png_set_sig_bytes
// ===========================================================================

#[test]
fn cfg70_set_sig_bytes() {
    for &(ct, bd) in &[(PNG_COLOR_TYPE_GRAY, 8), (PNG_COLOR_TYPE_RGB_ALPHA, 8)] {
        for il in [0, 1] {
            let full = make_png(ct, bd, il, 9, 7, 0x70_0017);
            for n in 0..=8usize {
                let png = full[n..].to_vec();
                let lbl = format!("cfg70 n={n} ct={ct} bd={bd} il={il}");
                diff(
                    &lbl,
                    &png,
                    Opt {
                        sig_bytes: Some(n as c_int),
                        ..Opt::default()
                    },
                    &nohook,
                    &nohook,
                );
            }
        }
    }
}

// ===========================================================================
// fixture sanity: make sure the synthesised inputs really exercise what the
// rows above claim they do (otherwise a differential assertion could pass
// vacuously because both libraries reject the input).
// ===========================================================================

fn chunk_names(png: &[u8]) -> Vec<String> {
    walk(png)
        .iter()
        .map(|(_, _, n)| String::from_utf8_lossy(n).into_owned())
        .collect()
}

#[test]
fn fixture_sanity() {
    // (a) the "rich" stream really carries every chunk row 54 asks for
    let rich = make_png_ex(
        PNG_COLOR_TYPE_PALETTE,
        8,
        0,
        9,
        7,
        0x54_000f,
        Extras {
            rich: true,
            ..Extras::default()
        },
    );
    let names = chunk_names(&rich);
    for want in [
        "IHDR", "PLTE", "gAMA", "cHRM", "sBIT", "bKGD", "hIST", "tRNS", "oFFs", "pHYs", "sCAL",
        "pCAL", "tIME", "eXIf", "sPLT", "tEXt", "zTXt", "unSf", "IDAT", "IEND",
    ] {
        assert!(
            names.iter().any(|n| n == want),
            "rich fixture is missing the {want} chunk; chunks = {names:?}"
        );
    }

    // (b) the rich stream decodes cleanly and every PNG_INFO_* bit shows up
    let r = capture(|| unsafe {
        decode(
            &L().c,
            &rich,
            Opt {
                full: true,
                ..Opt::default()
            },
            &|l, pp, _i, _s| {
                png_set_keep_unknown_chunks(l, pp, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0)
            },
            &nohook,
        )
    });
    assert!(
        r.out.is_some(),
        "rich fixture failed to decode with the C library: {:?}",
        r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>()
    );
    let clog: Vec<String> = r.log.iter().map(|m| m.to_string()).collect();
    let snap = r.out.unwrap();
    // libpng 1.6.59's read chunk-position table gives hIST `pos_before =
    // PNG_HAVE_PLTE`, so a spec-conformant hIST (which must FOLLOW PLTE) is
    // always rejected as "out of place" - by the C library and therefore also
    // by the translation.  Assert that quirk instead of hIST validity.
    assert!(
        clog.iter().any(|m| m.contains("hIST: out of place")),
        "expected the hIST out-of-place warning from C, got {clog:?}"
    );
    for want in [
        "gAMA", "sBIT", "cHRM", "PLTE", "tRNS", "bKGD", "pHYs", "oFFs", "tIME", "pCAL", "sPLT",
        "sCAL", "eXIf",
    ] {
        let key = format!("info.valid_{want}");
        let v = snap
            .vals
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0);
        assert!(v != 0, "rich fixture: png_get_valid({want}) == 0");
    }
    assert!(
        snap.vals
            .iter()
            .any(|(k, v)| k == "info.unknown_n" && *v > 0),
        "rich fixture: no unknown chunks were kept"
    );

    // (b2) the PNG v3 HDR chunks really make it into the stream and back out
    let hdr = make_png_ex(
        PNG_COLOR_TYPE_RGB,
        8,
        0,
        9,
        7,
        0x54_0020,
        Extras {
            hdr: true,
            text: true,
            ..Extras::default()
        },
    );
    let hnames = chunk_names(&hdr);
    for want in ["cICP", "cLLI", "mDCV"] {
        assert!(
            hnames.iter().any(|n| n == want),
            "hdr fixture is missing the {want} chunk; chunks = {hnames:?}"
        );
    }
    let r = capture(|| unsafe {
        decode(
            &L().c,
            &hdr,
            Opt {
                full: true,
                ..Opt::default()
            },
            &nohook,
            &nohook,
        )
    });
    let snap = r.out.expect("hdr fixture failed to decode with the C library");
    for want in ["cICP", "cLLI", "mDCV"] {
        let key = format!("info.valid_{want}");
        let v = snap
            .vals
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0);
        assert!(v != 0, "hdr fixture: png_get_valid({want}) == 0");
    }

    // (c) iCCP fixtures really contain an iCCP chunk that the C library accepts
    for ct in [PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_RGB] {
        let png = make_png_ex(
            ct,
            8,
            0,
            9,
            7,
            if ct == PNG_COLOR_TYPE_GRAY {
                0x54_0012
            } else {
                0x54_0011
            },
            Extras {
                iccp: true,
                ..Extras::default()
            },
        );
        assert!(
            chunk_names(&png).iter().any(|n| n == "iCCP"),
            "iCCP fixture (ct={ct}) has no iCCP chunk: {:?}",
            chunk_names(&png)
        );
    }

    // (d) the MNG filter-method-64 stream decodes when the feature is permitted
    let mng = mng_stream(PNG_COLOR_TYPE_RGB);
    let o = Opt {
        sig_bytes: Some(8),
        ..Opt::default()
    };
    let r = capture(|| unsafe {
        decode(&L().c, &mng, o, &|l, pp, _i, _s| {
            png_permit_mng_features(l, pp, PNG_ALL_MNG_FEATURES as png_uint_32);
        }, &nohook)
    });
    assert!(
        r.out.is_some(),
        "filter-64 stream did not decode with MNG features permitted: {:?}",
        r.log.iter().map(|m| m.to_string()).collect::<Vec<_>>()
    );
    // ... and is rejected without it
    let r = capture(|| unsafe { decode(&L().c, &mng, o, &nohook, &nohook) });
    assert!(
        r.out.is_none(),
        "filter-64 stream decoded even though MNG features were not permitted"
    );

    // (e) row 43 really pauses: at least one configuration must report a
    //     non-zero byte count from png_process_data_pause(save = 0)
    let png = make_png(PNG_COLOR_TYPE_RGB, 8, 0, 17, 11, 0x43_0005);
    let r = capture(|| unsafe {
        prog_decode(
            &L().c,
            &png,
            ProgOpt {
                granule: 29,
                pause_save: Some(0),
                ..ProgOpt::default()
            },
        )
    });
    let snap = r.out.expect("paused progressive decode failed in C");
    let pause = snap
        .vals
        .iter()
        .find(|(k, _)| k == "pinfo.pause_ret")
        .map(|(_, v)| *v);
    assert_eq!(
        pause.is_some(),
        true,
        "png_process_data_pause was never called"
    );
    assert!(
        pause.unwrap() > 0,
        "png_process_data_pause(save=0) returned {:?}, expected > 0",
        pause
    );

    // (f) the out-of-range palette index fixture is really out of range
    let bad = make_png_ex(
        PNG_COLOR_TYPE_PALETTE,
        8,
        0,
        9,
        7,
        0x56_0014,
        Extras {
            bad_index: true,
            ..Extras::default()
        },
    );
    let r = capture(|| unsafe {
        decode(
            &L().c,
            &bad,
            Opt::default(),
            &|l, pp, _i, _s| png_set_check_for_invalid_index(l, pp, 1),
            &nohook,
        )
    });
    let log: Vec<String> = r.log.iter().map(|m| m.to_string()).collect();
    assert!(
        log.iter().any(|m| m.contains("palette index")),
        "expected an out-of-range palette index report from C, got {log:?}"
    );
}

/// Randomised progressive-read fuzz.
#[test]
fn cfg42_random_progressive() {
    let mut rng = Rng::new(0x1234_5678_9abc_def0);
    for case in 0..1500u32 {
        let (ct, bd) = CT_BD[rng.below(CT_BD.len() as u32) as usize];
        let il = rng.below(2) as c_int;
        let w = 1 + rng.below(34);
        let h = 1 + rng.below(20);
        let png = make_png(ct, bd, il, w, h, 0x42_2000 + case as u64);
        let granule = match rng.below(6) {
            0 => 1,
            1 => 2,
            2 => 1 + rng.below(9) as usize,
            3 => 1 + rng.below(64) as usize,
            4 => 1 + rng.below(300) as usize,
            _ => usize::MAX,
        };
        let combine = rng.below(3) == 0;
        let ih = !combine && rng.below(2) == 1;
        diff_prog(
            &format!(
                "cfg42 rnd case={case} ct={ct} bd={bd} il={il} {w}x{h} g={granule} combine={combine} ih={ih}"
            ),
            &png,
            ProgOpt {
                granule,
                combine,
                interlace_handling: ih,
                ..ProgOpt::default()
            },
        );
    }
}

/// Randomised CRC corruption: every chunk of a rich stream gets its stored CRC
/// flipped in turn, under a random `png_set_crc_action` / `png_set_benign_errors`
/// setting.  Only CRC bytes are touched, so the outcome is fully defined by the
/// CRC-action policy.
#[test]
fn cfg45_random_crc_corruption() {
    let acts = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
    ];
    let mut rng = Rng::new(0xfeed_face_0000_1);
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    ] {
        let base = make_png_ex(
            ct,
            bd,
            0,
            9,
            7,
            0x45_3000,
            Extras {
                rich: true,
                ..Extras::default()
            },
        );
        for (off, len, nm) in walk(&base) {
            if &nm == b"IEND" {
                continue;
            }
            let mut png = base.clone();
            png[off + 8 + len + 3] ^= 0x01;
            for _ in 0..6 {
                let crit = acts[rng.below(acts.len() as u32) as usize];
                let ancil = acts[rng.below(acts.len() as u32) as usize];
                let benign = rng.below(2) as c_int;
                let lbl = format!(
                    "cfg45 rnd ct={ct} bd={bd} chunk={} crit={crit} ancil={ancil} benign={benign}",
                    String::from_utf8_lossy(&nm)
                );
                diff(
                    &lbl,
                    &png,
                    Opt {
                        full: true,
                        ..Opt::default()
                    },
                    &move |l, pp, _i, _s| unsafe {
                        png_set_benign_errors(l, pp, benign);
                        png_set_crc_action(l, pp, crit, ancil);
                        png_set_keep_unknown_chunks(
                            l,
                            pp,
                            PNG_HANDLE_CHUNK_ALWAYS,
                            ptr::null(),
                            0,
                        );
                    },
                    &nohook,
                );
            }
        }
    }
}

/// Randomised reader-policy fuzz: limits, options, unknown-chunk handling,
/// user chunk callback and CRC actions all at once.
#[test]
fn cfg47_random_reader_policy() {
    let mut rng = Rng::new(0x0bad_c0de_0000_3);
    let streams: Vec<Vec<u8>> = vec![
        make_png_ex(
            PNG_COLOR_TYPE_PALETTE,
            4,
            1,
            9,
            7,
            0x47_4000,
            Extras {
                rich: true,
                ..Extras::default()
            },
        ),
        insert_chunk_after(
            &make_png(PNG_COLOR_TYPE_RGB, 8, 0, 9, 7, 0x47_4001),
            b"IHDR",
            &mk_chunk(b"prVt", &[0x5au8; 300]),
        ),
        make_png_ex(
            PNG_COLOR_TYPE_GRAY,
            16,
            0,
            17,
            3,
            0x47_4002,
            Extras {
                text: true,
                srgb: true,
                ..Extras::default()
            },
        ),
    ];
    for case in 0..1500u32 {
        let png = &streams[rng.below(streams.len() as u32) as usize];
        let wmax = [0u32, 1, 8, 9, 10, 33, 1_000_000][rng.below(7) as usize];
        let hmax = [0u32, 1, 3, 7, 8, 20, 1_000_000][rng.below(7) as usize];
        let cache = [0u32, 1, 2, 5, 20, 1000][rng.below(6) as usize];
        let mmax = [0usize, 1, 16, 64, 300, 8_000_000][rng.below(6) as usize];
        let keep = [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ][rng.below(4) as usize];
        let opt = [
            PNG_MAXIMUM_INFLATE_WINDOW,
            PNG_SKIP_sRGB_CHECK_PROFILE,
            PNG_IGNORE_ADLER32,
        ][rng.below(3) as usize];
        let onoff = [PNG_OPTION_OFF, PNG_OPTION_ON][rng.below(2) as usize];
        let ucb = rng.below(4);
        let benign = rng.below(2) as c_int;
        let lbl = format!(
            "cfg47 rnd case={case} wmax={wmax} hmax={hmax} cache={cache} mmax={mmax} keep={keep} opt={opt}/{onoff} ucb={ucb} benign={benign}"
        );
        diff(
            &lbl,
            png,
            Opt {
                full: true,
                ..Opt::default()
            },
            &move |l, pp, _i, s| unsafe {
                png_set_benign_errors(l, pp, benign);
                png_set_user_limits(l, pp, wmax, hmax);
                png_set_chunk_cache_max(l, pp, cache);
                png_set_chunk_malloc_max(l, pp, mmax);
                png_set_keep_unknown_chunks(l, pp, keep, ptr::null(), 0);
                s.v("opt", "set_option", png_set_option(l, pp, opt, onoff) as i64);
                if ucb < 3 {
                    UCHUNK_RET.with(|x| x.set(ucb as c_int - 1));
                    png_set_read_user_chunk_fn(
                        l,
                        pp,
                        0x99usize as png_voidp,
                        Some(user_chunk_cb),
                    );
                }
            },
            &nohook,
        );
    }
    UCHUNK_RET.with(|x| x.set(0));
}

//! Phase C — ERRORS.md section `## pngread.c / pngrtran.c / pngpread.c`
//! (rows 569 .. 786).
//!
//! Every case builds the exact invalid input / calling sequence, drives BOTH
//! shared objects through it and asserts the rejection is identical: same
//! return value, same error-or-not, same captured `Diag` (warnings + errors,
//! in order).
//!
//! Streams are produced two ways:
//!   * with the (already verified, see `t03_write`) sequential WRITE path and
//!     then mutated byte-wise / chunk-wise, and
//!   * assembled by hand from chunks whose IDAT holds a *stored* (BTYPE=00)
//!     deflate stream, which gives byte-level control over the zlib header,
//!     the filter bytes, the ADLER32 and the amount of compressed data.
//!
//! Rows of the section that have NO observable rejection in this build, and
//! why (each is exercised anyway through its *supported* path):
//!
//!  * 581-587 `png_read_row`'s "PNG_READ_<X>_SUPPORTED is not defined"
//!    warnings (pngread.c:313-352): pnglibconf.h defines every one of
//!    PNG_READ_INVERT/FILLER/PACKSWAP/PACK/SHIFT/BGR/SWAP_SUPPORTED, so all
//!    seven warnings are compiled out.
//!  * 596 `png_read_image`'s "Cannot read interlaced image -- interlace
//!    handler disabled" (pngread.c:647): PNG_READ_INTERLACING_SUPPORTED is on,
//!    so that whole `#else` arm is compiled out (row 595's warning IS tested).
//!  * 605-619 `png_read_png`'s fourteen "PNG_TRANSFORM_<X> not supported"
//!    app errors (pngread.c:892-1027): every corresponding
//!    PNG_READ_<X>_SUPPORTED is defined, including
//!    PNG_READ_SCALE_16_TO_8_SUPPORTED and PNG_READ_STRIP_16_TO_8_SUPPORTED.
//!  * 729 `png_set_rgb_to_gray_fixed`'s "Cannot do RGB_TO_GRAY without
//!    EXPAND_SUPPORTED" (pngrtran.c:1082): PNG_READ_EXPAND_SUPPORTED is on.
//!  * 740 `png_read_transform_info`'s "Palette is NULL in indexed image"
//!    (pngrtran.c:2086-2104): unreachable from the read path, because
//!    png_set_PLTE always png_calloc's a full 256-entry png_ptr->palette
//!    (pngset.c) -- even for the MNG zero-length PLTE, which is the only way a
//!    colour-type-3 stream can get past png_handle_PLTE with num_palette == 0.
//!    Both the zero-length-PLTE and the missing-PLTE streams are tested.
//!  * 715, 732, 733, 741-745, 750, 769-771, 782 are internal fallbacks
//!    (allocation failure inside the quantize reduction, unresolvable gamma,
//!    invalid sBIT shifts, palette indexes past num_trans, save_buffer
//!    overflow, the pass>7 clamp) with no distinct diagnostic; they are driven
//!    through the inputs that reach them (quantize matrices, gamma 0, sBIT of
//!    0/>=bit_depth, out-of-range palette indexes, 1-byte progressive feeds)
//!    and compared for full-state parity rather than for a message.
#![allow(dead_code)]

mod common;
use common::*;
use std::ptr::{null, null_mut};

// ---------------------------------------------------------------------------
// Probe + differential assertion (same pattern as t10_errors_core.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct P(i64, bool, Diag);

fn probe<F: FnOnce(&'static Api) -> i64>(api: &'static Api, f: F) -> P {
    set_current_api(api);
    diag_reset();
    let r = guard(|| f(api));
    P(r.unwrap_or(i64::MIN), r.is_some(), diag_take())
}

macro_rules! same {
    ($label:expr, $f:expr) => {{
        if std::env::var_os("PNGTRACE").is_some() {
            eprintln!("TRACE {}", $label);
        }
        let c = probe(c_api(), $f);
        let r = probe(rs_api(), $f);
        assert_eq!(c, r, "{}", $label);
        c
    }};
}

// ---------------------------------------------------------------------------
// Hashing of everything a read can produce
// ---------------------------------------------------------------------------

const FNV: u64 = 0xcbf2_9ce4_8422_2325;

fn h64(a: &mut u64, b: &[u8]) {
    for &x in b {
        *a = (*a ^ x as u64).wrapping_mul(0x100_0000_01b3);
    }
}
fn hu(a: &mut u64, v: u64) {
    h64(a, &v.to_le_bytes());
}

/// Zero the padding bits of the last byte of a sub-byte-depth row.
///
/// `png_combine_row` deliberately PRESERVES the destination row's bits beyond
/// the last pixel (`end_byte & end_mask`).  For `png_read_png` the destination
/// rows are allocated by libpng with `png_malloc` and are therefore
/// uninitialised, so those bits are indeterminate in BOTH libraries and must
/// not be compared.  (Every other driver here uses zeroed buffers, where the
/// bits are well defined and ARE compared.)
fn mask_padding(row: &mut [u8], pixel_depth: u32, width: u32, packswap: bool) {
    if pixel_depth == 0 || pixel_depth >= 8 {
        return;
    }
    let bits = pixel_depth as u64 * width as u64;
    let m = (bits & 7) as u32;
    if m == 0 {
        return;
    }
    let last = (bits / 8) as usize;
    let keep: u8 = if packswap {
        0xffu8 >> (8 - m)
    } else {
        !(0xffu8 >> m)
    };
    if last < row.len() {
        row[last] &= keep;
    }
}

unsafe fn snap(api: &'static Api, png: png_structp, info: png_infop) -> u64 {
    let mut a = FNV;
    hu(&mut a, (api.png_get_image_width)(png, info) as u64);
    hu(&mut a, (api.png_get_image_height)(png, info) as u64);
    hu(&mut a, (api.png_get_bit_depth)(png, info) as u64);
    hu(&mut a, (api.png_get_color_type)(png, info) as u64);
    hu(&mut a, (api.png_get_interlace_type)(png, info) as u64);
    hu(&mut a, (api.png_get_compression_type)(png, info) as u64);
    hu(&mut a, (api.png_get_filter_type)(png, info) as u64);
    hu(&mut a, (api.png_get_channels)(png, info) as u64);
    hu(&mut a, (api.png_get_rowbytes)(png, info) as u64);
    hu(&mut a, (api.png_get_valid)(png, info, 0xffff_ffff) as u64);
    hu(&mut a, (api.png_get_palette_max)(png, info) as i64 as u64);
    hu(&mut a, (api.png_get_rgb_to_gray_status)(png) as u64);
    a
}

// ---------------------------------------------------------------------------
// PNG stream construction / mutation
// ---------------------------------------------------------------------------

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xedb8_8320
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn adler32(d: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in d {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// One chunk, with optional overrides for the on-the-wire length and CRC.
#[derive(Clone, Debug)]
struct Ch {
    name: [u8; 4],
    data: Vec<u8>,
    len: Option<u32>,
    crc: Option<u32>,
}

impl Ch {
    fn new(name: &[u8], data: Vec<u8>) -> Ch {
        let mut n = [0u8; 4];
        n.copy_from_slice(&name[..4]);
        Ch {
            name: n,
            data,
            len: None,
            crc: None,
        }
    }
    fn is(&self, name: &[u8]) -> bool {
        self.name == name[..4]
    }
    fn emit(&self, out: &mut Vec<u8>) {
        let l = self.len.unwrap_or(self.data.len() as u32);
        out.extend_from_slice(&l.to_be_bytes());
        out.extend_from_slice(&self.name);
        out.extend_from_slice(&self.data);
        let mut crcin = self.name.to_vec();
        crcin.extend_from_slice(&self.data);
        out.extend_from_slice(&self.crc.unwrap_or(crc32(&crcin)).to_be_bytes());
    }
}

fn asm(chs: &[Ch]) -> Vec<u8> {
    let mut out = PNG_SIG.to_vec();
    for c in chs {
        c.emit(&mut out);
    }
    out
}

/// Split a well-formed stream into its chunks (signature dropped).
fn split(bytes: &[u8]) -> Vec<Ch> {
    let mut v = Vec::new();
    let mut i = 8usize;
    while i + 8 <= bytes.len() {
        let l = u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
        if i + 12 + l > bytes.len() {
            break;
        }
        v.push(Ch::new(
            &bytes[i + 4..i + 8],
            bytes[i + 8..i + 8 + l].to_vec(),
        ));
        i += 12 + l;
    }
    v
}

fn ihdr(w: u32, h: u32, bd: u8, ct: u8, comp: u8, filt: u8, il: u8) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.push(bd);
    d.push(ct);
    d.push(comp);
    d.push(filt);
    d.push(il);
    d
}

/// A zlib stream made only of *stored* deflate blocks: the compressed bytes
/// are the raw bytes, so filter bytes / lengths / ADLER32 can be controlled.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78u8, 0x01];
    if raw.is_empty() {
        out.push(0x01);
        out.extend_from_slice(&[0, 0, 0xff, 0xff]);
    } else {
        let mut i = 0usize;
        while i < raw.len() {
            let n = (raw.len() - i).min(65535);
            let last = i + n >= raw.len();
            out.push(if last { 1 } else { 0 });
            out.extend_from_slice(&(n as u16).to_le_bytes());
            out.extend_from_slice(&(!(n as u16)).to_le_bytes());
            out.extend_from_slice(&raw[i..i + n]);
            i += n;
        }
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// The raw (pre-deflate) IDAT payload of a non-interlaced image: one filter
/// byte plus `rowbytes` data bytes per row.
fn raw_image(w: u32, h: u32, pd: u32, filt: u8, fill: u8) -> Vec<u8> {
    let rb = rowbytes(pd, w);
    let mut v = Vec::with_capacity((rb + 1) * h as usize);
    for _ in 0..h {
        v.push(filt);
        for i in 0..rb {
            v.push(fill.wrapping_add(i as u8));
        }
    }
    v
}

/// A complete hand-assembled PNG (non-interlaced).
fn hand(w: u32, h: u32, bd: u8, ct: u8, filt: u8) -> Vec<Ch> {
    let pd = channels_of(ct as c_int) * bd as u32;
    let mut v = vec![Ch::new(b"IHDR", ihdr(w, h, bd, ct, 0, 0, 0))];
    if ct == 3 {
        let n = 1usize << bd;
        let mut p = Vec::new();
        for i in 0..n {
            p.push((i * 3) as u8);
            p.push((i * 5) as u8);
            p.push((i * 7) as u8);
        }
        v.push(Ch::new(b"PLTE", p));
    }
    v.push(Ch::new(
        b"IDAT",
        zlib_stored(&raw_image(w, h, pd, filt, 0x40)),
    ));
    v.push(Ch::new(b"IEND", Vec::new()));
    v
}

/// A valid PNG produced by the C *write* path.
unsafe fn made(ct: c_int, bd: c_int, w: u32, h: u32, il: c_int, anc: bool) -> Vec<u8> {
    let api = c_api();
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let (png, info) = (sess.png, sess.info);
    let pd = channels_of(ct) * bd as u32;
    let rb = rowbytes(pd, w);
    let rows: Vec<Vec<u8>> = (0..h)
        .map(|y| (0..rb).map(|x| (y * 31 + x as u32 * 7) as u8).collect())
        .collect();
    let npal = if ct == PNG_COLOR_TYPE_PALETTE {
        1usize << bd
    } else {
        0
    };
    let palette: Vec<png_color> = (0..npal)
        .map(|i| png_color {
            red: (i * 3) as u8,
            green: (i * 5) as u8,
            blue: (i * 7) as u8,
        })
        .collect();
    let key = cs("Title");
    let txt = cs("t12");
    let text = [png_text {
        compression: PNG_TEXT_COMPRESSION_NONE,
        key: key.as_ptr() as png_charp,
        text: txt.as_ptr() as png_charp,
        text_length: 3,
        itxt_length: 0,
        lang: null_mut(),
        lang_key: null_mut(),
    }];
    let sbit = png_color_8 {
        red: bd as u8,
        green: bd as u8,
        blue: bd as u8,
        gray: bd as u8,
        alpha: bd as u8,
    };
    let ok = guard(|| {
        (api.png_set_IHDR)(
            png,
            info,
            w,
            h,
            bd,
            ct,
            il,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if !palette.is_empty() {
            (api.png_set_PLTE)(png, info, palette.as_ptr(), palette.len() as c_int);
        }
        if anc {
            (api.png_set_gAMA_fixed)(png, info, 45455);
            (api.png_set_sRGB)(png, info, 0);
            (api.png_set_sBIT)(png, info, &sbit);
            (api.png_set_pHYs)(png, info, 300, 300, 1);
            (api.png_set_text)(png, info, text.as_ptr(), 1);
        }
        (api.png_write_info)(png, info);
        let mut rp: Vec<png_bytep> = rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rp.as_mut_ptr());
        (api.png_write_end)(png, info);
    })
    .is_some();
    let d = diag_take();
    assert!(ok, "reference PNG construction failed: {:?}", d);
    std::mem::take(&mut sess.sink.buf)
}

// ---------------------------------------------------------------------------
// Sequential read driver
// ---------------------------------------------------------------------------

/// Cap on the row buffers we are willing to allocate for a (possibly
/// corrupted) header.  Depends only on values that are themselves compared,
/// so both libraries always take the same branch.
const MAXBUF: u64 = 4 << 20;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ord {
    /// call png_read_info twice
    info_twice: bool,
    /// 0 none, 1 update_info, 2 start_read_image, 3 update+start,
    /// 4 start+update, 5 update twice, 6 start twice
    init: u8,
    /// call png_set_interlace_handling before reading rows
    ihand: bool,
    /// 0 read_image, 1 read_rows, 2 read_row loop, 3 read_row(NULL,NULL),
    /// 4 nothing, 5 read_rows(NULL,NULL), 6 read_rows(display only)
    rows: u8,
    end_twice: bool,
    /// read a row after png_read_end
    rows_after_end: bool,
}

impl Ord {
    fn plain() -> Ord {
        Ord {
            info_twice: false,
            init: 1,
            ihand: true,
            rows: 0,
            end_twice: false,
            rows_after_end: false,
        }
    }
}

unsafe fn seq(api: &'static Api, bytes: &[u8], o: Ord) -> i64 {
    let s = ReadSess::new(api, bytes);
    let (png, info) = (s.png, s.info);
    let mut a = FNV;
    (api.png_read_info)(png, info);
    if o.info_twice {
        (api.png_read_info)(png, info);
    }
    hu(&mut a, snap(api, png, info));
    match o.init {
        1 => (api.png_read_update_info)(png, info),
        2 => (api.png_start_read_image)(png),
        3 => {
            (api.png_read_update_info)(png, info);
            (api.png_start_read_image)(png);
        }
        4 => {
            (api.png_start_read_image)(png);
            (api.png_read_update_info)(png, info);
        }
        5 => {
            (api.png_read_update_info)(png, info);
            (api.png_read_update_info)(png, info);
        }
        6 => {
            (api.png_start_read_image)(png);
            (api.png_start_read_image)(png);
        }
        _ => {}
    }
    hu(&mut a, snap(api, png, info));
    let h = (api.png_get_image_height)(png, info);
    let rb = (api.png_get_rowbytes)(png, info);
    let np = if o.ihand {
        let n = (api.png_set_interlace_handling)(png);
        hu(&mut a, n as u64);
        n
    } else {
        1
    };
    if (h as u64) * (rb as u64 + 16) <= MAXBUF && o.rows != 4 {
        let mut buf: Vec<Vec<u8>> = (0..h as usize).map(|_| vec![0u8; rb + 16]).collect();
        match o.rows {
            0 => {
                let mut ptrs: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                (api.png_read_image)(png, ptrs.as_mut_ptr());
            }
            1 => {
                for _ in 0..np {
                    let mut ptrs: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    (api.png_read_rows)(png, ptrs.as_mut_ptr(), null_mut(), h);
                }
            }
            2 => {
                for _ in 0..np {
                    for y in 0..h as usize {
                        (api.png_read_row)(png, buf[y].as_mut_ptr(), null_mut());
                    }
                }
            }
            3 => {
                for _ in 0..np {
                    for _ in 0..h {
                        (api.png_read_row)(png, null_mut(), null_mut());
                    }
                }
            }
            5 => {
                for _ in 0..np {
                    (api.png_read_rows)(png, null_mut(), null_mut(), h);
                }
            }
            _ => {
                for _ in 0..np {
                    let mut ptrs: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    (api.png_read_rows)(png, null_mut(), ptrs.as_mut_ptr(), h);
                }
            }
        }
        for r in &buf {
            h64(&mut a, r);
        }
    }
    (api.png_read_end)(png, s.end);
    if o.end_twice {
        (api.png_read_end)(png, s.end);
    }
    if o.rows_after_end {
        let mut r = vec![0u8; rb + 16];
        (api.png_read_row)(png, r.as_mut_ptr(), null_mut());
        h64(&mut a, &r);
    }
    hu(&mut a, snap(api, png, info));
    hu(&mut a, snap(api, png, s.end));
    hu(&mut a, (api.png_get_current_row_number)(png) as u64);
    hu(&mut a, (api.png_get_current_pass_number)(png) as u64);
    a as i64
}

/// png_read_info only.
unsafe fn seq_info(api: &'static Api, bytes: &[u8]) -> i64 {
    let s = ReadSess::new(api, bytes);
    (api.png_read_info)(s.png, s.info);
    snap(api, s.png, s.info) as i64
}

fn dseq(label: &str, bytes: &[u8]) -> P {
    same!(label, |api: &'static Api| unsafe {
        seq(api, bytes, Ord::plain())
    })
}

fn dseq_o(label: &str, bytes: &[u8], o: Ord) -> P {
    same!(label, |api: &'static Api| unsafe { seq(api, bytes, o) })
}

fn dinfo(label: &str, bytes: &[u8]) -> P {
    same!(label, |api: &'static Api| unsafe { seq_info(api, bytes) })
}

// ---------------------------------------------------------------------------
// Progressive read driver
// ---------------------------------------------------------------------------

#[repr(C)]
struct Acc {
    h: u64,
    infos: u32,
    rows: u32,
    ends: u32,
    w: u32,
    pd: u32,
    il: bool,
    /// 0 start_read_image, 1 read_update_info, 2 neither
    init: u8,
    ihand: bool,
    /// call png_process_data_skip from the info callback
    skip: bool,
    /// png_process_data_pause(save) from the info callback; -1 = never
    pause: c_int,
    combine: bool,
    /// set a read transform (png_set_expand) from the info callback
    expand: bool,
    disp: Vec<Vec<u8>>,
}

impl Acc {
    fn new(init: u8) -> Acc {
        Acc {
            h: FNV,
            infos: 0,
            rows: 0,
            ends: 0,
            w: 0,
            pd: 0,
            il: false,
            init,
            ihand: true,
            skip: false,
            pause: -1,
            combine: false,
            expand: false,
            disp: Vec::new(),
        }
    }
    fn total(&self) -> i64 {
        let mut a = self.h;
        hu(&mut a, self.infos as u64);
        hu(&mut a, self.rows as u64);
        hu(&mut a, self.ends as u64);
        for d in &self.disp {
            h64(&mut a, d);
        }
        a as i64
    }
}

unsafe extern "C-unwind" fn p_info(png: png_structp, info: png_infop) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Acc;
    if p.is_null() {
        return;
    }
    let a = &mut *p;
    a.infos += 1;
    a.w = (api.png_get_image_width)(png, info);
    a.pd = (api.png_get_channels)(png, info) as u32 * (api.png_get_bit_depth)(png, info) as u32;
    a.il = (api.png_get_interlace_type)(png, info) as c_int == PNG_INTERLACE_ADAM7;
    let s = snap(api, png, info);
    hu(&mut a.h, s);
    if a.skip {
        hu(&mut a.h, (api.png_process_data_skip)(png) as u64);
    }
    if a.expand {
        (api.png_set_expand)(png);
    }
    if a.ihand {
        hu(&mut a.h, (api.png_set_interlace_handling)(png) as u64);
    }
    match a.init {
        0 => (api.png_start_read_image)(png),
        1 => (api.png_read_update_info)(png, info),
        _ => {}
    }
    if a.combine {
        let rb = rowbytes(a.pd, a.w);
        let h = (api.png_get_image_height)(png, info) as usize;
        if (h as u64) * (rb as u64 + 16) <= MAXBUF {
            a.disp = (0..h).map(|_| vec![0u8; rb + 16]).collect();
        }
    }
    if a.pause >= 0 {
        let n = (api.png_process_data_pause)(png, a.pause);
        hu(&mut a.h, n as u64);
    }
    hu(&mut a.h, snap(api, png, info));
}

unsafe extern "C-unwind" fn p_row(
    png: png_structp,
    row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Acc;
    if p.is_null() {
        return;
    }
    let a = &mut *p;
    a.rows += 1;
    hu(&mut a.h, row_num as u64);
    hu(&mut a.h, pass as i64 as u64);
    hu(&mut a.h, row.is_null() as u64);
    hu(&mut a.h, (api.png_get_current_row_number)(png) as u64);
    hu(&mut a.h, (api.png_get_current_pass_number)(png) as u64);
    if !row.is_null() && a.pd > 0 && a.ihand {
        let n = rowbytes(a.pd, a.w);
        h64(&mut a.h, std::slice::from_raw_parts(row, n));
    }
    if a.combine {
        let i = row_num as usize;
        if i < a.disp.len() {
            let dp = a.disp[i].as_mut_ptr();
            (api.png_progressive_combine_row)(png as png_const_structrp, dp, row as png_const_bytep);
        }
    }
}

unsafe extern "C-unwind" fn p_end(png: png_structp, info: png_infop) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Acc;
    if p.is_null() {
        return;
    }
    let a = &mut *p;
    a.ends += 1;
    hu(&mut a.h, snap(api, png, info));
}

/// `gran == 0` means "hand the whole buffer over at once".
unsafe fn push(api: &'static Api, bytes: &[u8], gran: usize, mut acc: Acc) -> i64 {
    let s = ReadSess::new(api, &[]);
    let mut a = Box::new(std::mem::replace(&mut acc, Acc::new(0)));
    (api.png_set_progressive_read_fn)(
        s.png,
        &mut *a as *mut Acc as png_voidp,
        Some(p_info),
        Some(p_row),
        Some(p_end),
    );
    let mut data = bytes.to_vec();
    let step = if gran == 0 { data.len().max(1) } else { gran };
    let mut pos = 0usize;
    while pos < data.len() {
        let n = step.min(data.len() - pos);
        (api.png_process_data)(s.png, s.info, data.as_mut_ptr().add(pos), n);
        pos += n;
    }
    // Drain whatever png_push_save_buffer is still holding.
    for _ in 0..4 {
        (api.png_process_data)(s.png, s.info, null_mut(), 0);
    }
    a.total()
}

fn dpush(label: &str, bytes: &[u8], gran: usize, init: u8) -> P {
    same!(label, |api: &'static Api| unsafe {
        push(api, bytes, gran, Acc::new(init))
    })
}

// ---------------------------------------------------------------------------
// Constants missing from tests/common/types.rs
// ---------------------------------------------------------------------------

const PNG_ERROR_ACTION_NONE: c_int = 1;
const PNG_ERROR_ACTION_WARN: c_int = 2;
const PNG_ERROR_ACTION_ERROR: c_int = 3;

/// The four ancillary chunks known to the reader, with their correct length.
const ANC: [&[u8]; 21] = [
    b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"sBIT", b"tRNS", b"bKGD", b"hIST", b"pHYs", b"oFFs",
    b"sCAL", b"pCAL", b"tIME", b"tEXt", b"zTXt", b"iTXt", b"sPLT", b"eXIf", b"cICP", b"cLLI",
    b"mDCV",
];

fn pattern(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| match i {
            0 => 0x41,
            1 => 0x00,
            _ => ((i * 7) % 251) as u8,
        })
        .collect()
}

// ===========================================================================
// 0. png_create_read_struct / _2 (row 569)
// ===========================================================================

#[test]
fn create_read_struct_rejections() {
    for v in [
        "1.6.59.git", "", "1", "1.6", "1.5.0", "1.7.0", "2.6.59.git", "1.6.59", "1.6.59.gi",
        "xxxxxxxxxx", "0.0.0.0",
    ] {
        let cv = cs(v);
        for two in [false, true] {
            same!(
                format!("png_create_read_struct{}({:?})", if two { "_2" } else { "" }, v),
                |api: &'static Api| unsafe {
                    let p = if two {
                        (api.png_create_read_struct_2)(
                            cv.as_ptr(),
                            null_mut(),
                            Some(cb_error),
                            Some(cb_warning),
                            null_mut(),
                            None,
                            None,
                        )
                    } else {
                        (api.png_create_read_struct)(
                            cv.as_ptr(),
                            null_mut(),
                            Some(cb_error),
                            Some(cb_warning),
                        )
                    };
                    let ok = !p.is_null();
                    if ok {
                        let mut q = p;
                        (api.png_destroy_read_struct)(&mut q, null_mut(), null_mut());
                    }
                    ok as i64
                }
            );
        }
    }
    // NULL version string, and NULL error/warning callbacks
    same!("png_create_read_struct(NULL version)", |api: &'static Api| unsafe {
        let p = (api.png_create_read_struct)(null(), null_mut(), Some(cb_error), Some(cb_warning));
        let ok = !p.is_null();
        if ok {
            let mut q = p;
            (api.png_destroy_read_struct)(&mut q, null_mut(), null_mut());
        }
        ok as i64
    });
    same!("png_create_read_struct(NULL callbacks)", |api: &'static Api| unsafe {
        let v = ver();
        let p = (api.png_create_read_struct)(v.as_ptr(), null_mut(), None, None);
        let ok = !p.is_null();
        if ok {
            let mut q = p;
            (api.png_destroy_read_struct)(&mut q, null_mut(), null_mut());
        }
        ok as i64
    });
}

// ===========================================================================
// 1. NULL-pointer and out-of-order API guards
//    rows 570, 574, 576, 592, 593, 594, 597, 601, 602, 603, 751, 752, 754,
//    755, 772, 783, 784, 785, 786
// ===========================================================================

#[test]
fn null_pointer_guards() {
    // png_read_info(NULL, NULL) / (png, NULL)
    same!("png_read_info(NULL,NULL)", |api: &'static Api| {
        unsafe { (api.png_read_info)(null_mut(), null_mut()) };
        0
    });
    same!("png_read_info(png,NULL)", |api: &'static Api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_read_info)(s.png, null_mut());
        }
        0
    });
    same!("png_read_update_info(NULL)", |api: &'static Api| {
        unsafe { (api.png_read_update_info)(null_mut(), null_mut()) };
        0
    });
    same!("png_start_read_image(NULL)", |api: &'static Api| {
        unsafe { (api.png_start_read_image)(null_mut()) };
        0
    });
    same!("png_read_row(NULL)", |api: &'static Api| {
        unsafe { (api.png_read_row)(null_mut(), null_mut(), null_mut()) };
        0
    });
    same!("png_read_rows(NULL)", |api: &'static Api| {
        unsafe { (api.png_read_rows)(null_mut(), null_mut(), null_mut(), 5) };
        0
    });
    same!("png_read_image(NULL)", |api: &'static Api| {
        unsafe { (api.png_read_image)(null_mut(), null_mut()) };
        0
    });
    same!("png_read_end(NULL)", |api: &'static Api| {
        unsafe { (api.png_read_end)(null_mut(), null_mut()) };
        0
    });
    same!("png_read_png(NULL,NULL)", |api: &'static Api| {
        unsafe { (api.png_read_png)(null_mut(), null_mut(), 0, null_mut()) };
        0
    });
    same!("png_read_png(png,NULL)", |api: &'static Api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_read_png)(s.png, null_mut(), 0, null_mut());
        }
        0
    });
    same!("png_destroy_read_struct(NULL,NULL,NULL)", |api: &'static Api| {
        unsafe { (api.png_destroy_read_struct)(null_mut(), null_mut(), null_mut()) };
        0
    });
    same!("png_destroy_read_struct(&NULL)", |api: &'static Api| {
        let mut p: png_structp = null_mut();
        unsafe { (api.png_destroy_read_struct)(&mut p, null_mut(), null_mut()) };
        p.is_null() as i64
    });
    same!("png_set_read_status_fn(NULL)", |api: &'static Api| {
        unsafe { (api.png_set_read_status_fn)(null_mut(), None) };
        0
    });
    same!("png_set_read_status_fn(png,NULL)", |api: &'static Api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            (api.png_set_read_status_fn)(s.png, None);
        }
        0
    });
    // pngpread.c NULL guards
    same!("png_process_data(NULL,NULL)", |api: &'static Api| {
        unsafe { (api.png_process_data)(null_mut(), null_mut(), null_mut(), 0) };
        0
    });
    same!("png_process_data(png,NULL)", |api: &'static Api| {
        unsafe {
            let s = ReadSess::new(api, &[]);
            let mut b = [0u8; 8];
            (api.png_process_data)(s.png, null_mut(), b.as_mut_ptr(), 8);
        }
        0
    });
    for save in [0i32, 1, -1, 2, 999, i32::MIN, i32::MAX] {
        same!(
            format!("png_process_data_pause(NULL,{})", save),
            |api: &'static Api| unsafe {
                (api.png_process_data_pause)(null_mut(), save) as i64
            }
        );
    }
    same!("png_process_some_data(NULL,NULL)", |api: &'static Api| {
        unsafe { (api.png_process_some_data)(null_mut(), null_mut()) };
        0
    });
    same!("png_push_fill_buffer(NULL)", |api: &'static Api| {
        unsafe { (api.png_push_fill_buffer)(null_mut(), null_mut(), 0) };
        0
    });
    same!("png_progressive_combine_row(NULL)", |api: &'static Api| {
        unsafe { (api.png_progressive_combine_row)(null(), null_mut(), null()) };
        0
    });
    same!("png_set_progressive_read_fn(NULL)", |api: &'static Api| {
        unsafe {
            (api.png_set_progressive_read_fn)(null_mut(), null_mut(), None, None, None)
        };
        0
    });
    same!("png_get_progressive_ptr(NULL)", |api: &'static Api| {
        let p = unsafe { (api.png_get_progressive_ptr)(null()) };
        p.is_null() as i64
    });
    // png_process_data_skip: unimplemented, always app-warns (row 754).
    // NOTE: png_process_data_skip has NO NULL guard -- png_app_warning reads
    // png_ptr->flags unconditionally (pngerror.c), so a NULL png_ptr there is
    // C undefined behaviour, not an error path; only the valid-struct call is
    // tested.
    same!("png_process_data_skip", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        let a = (api.png_process_data_skip)(s.png) as i64;
        let b = (api.png_process_data_skip)(s.png) as i64;
        a * 4 + b
    });
    // png_progressive_combine_row with a NULL new_row (row 784) and a real
    // old_row, on a struct that has read a header.
    let png = unsafe { made(PNG_COLOR_TYPE_RGB, 8, 5, 3, PNG_INTERLACE_NONE, false) };
    same!("progressive_combine_row(NULL new_row)", |api: &'static Api| unsafe {
        let mut acc = Acc::new(0);
        acc.combine = true;
        push(api, &png, 0, acc)
    });
}

/// `png_read_rows` / `png_read_row` with NULL row pointers, and the
/// out-of-order sequences (rows 575, 577, 593, 595).
#[test]
fn out_of_order_calls() {
    let mut streams: Vec<(String, Vec<u8>)> = Vec::new();
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            streams.push((
                format!("ct={} bd={} il={}", ct, bd, il),
                unsafe { made(ct, bd, 9, 5, il, true) },
            ));
        }
    }
    for (tag, b) in &streams {
        for init in 0u8..7 {
            for rows in 0u8..7 {
                for ihand in [false, true] {
                    let o = Ord {
                        info_twice: false,
                        init,
                        ihand,
                        rows,
                        end_twice: false,
                        rows_after_end: false,
                    };
                    dseq_o(
                        &format!("{} init={} rows={} ihand={}", tag, init, rows, ihand),
                        b,
                        o,
                    );
                }
            }
        }
        // png_read_info twice, png_read_end twice, a row after png_read_end
        for (it, et, rae) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, true),
        ] {
            let o = Ord {
                info_twice: it,
                init: 1,
                ihand: true,
                rows: 0,
                end_twice: et,
                rows_after_end: rae,
            };
            dseq_o(
                &format!("{} info2={} end2={} rowafterend={}", tag, it, et, rae),
                b,
                o,
            );
        }
    }
}

/// `png_read_row` / `png_read_update_info` / `png_start_read_image` /
/// `png_read_end` on a struct that never saw a header at all.
#[test]
fn row_calls_before_read_info() {
    same!("png_read_end before png_read_info", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        (api.png_read_end)(s.png, s.info);
        0
    });
    same!("png_read_update_info before png_read_info", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        (api.png_read_update_info)(s.png, s.info);
        snap(api, s.png, s.info) as i64
    });
    same!("png_start_read_image before png_read_info", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        (api.png_start_read_image)(s.png);
        snap(api, s.png, s.info) as i64
    });
    // "Invalid attempt to read row data" (row 588): row read while
    // (mode & PNG_HAVE_IDAT) == 0.
    same!("png_read_row before png_read_info", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        let mut r = [0u8; 64];
        (api.png_read_row)(s.png, r.as_mut_ptr(), null_mut());
        0
    });
    same!("png_read_rows before png_read_info", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        let mut r = vec![0u8; 64];
        let mut p = [r.as_mut_ptr()];
        (api.png_read_rows)(s.png, p.as_mut_ptr(), null_mut(), 1);
        0
    });
    same!("png_read_image before png_read_info", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        let mut r = vec![0u8; 64];
        let mut p = [r.as_mut_ptr()];
        (api.png_read_image)(s.png, p.as_mut_ptr());
        0
    });
}

// ===========================================================================
// 2. Truncated byte streams
//    rows 634 (analogue), 757-759, 765, 766, 768, 774, 776 + "Read Error"
// ===========================================================================

#[test]
fn truncation_sequential() {
    for (ct, bd, il) in [
        (PNG_COLOR_TYPE_GRAY, 1, PNG_INTERLACE_NONE),
        (PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE),
        (PNG_COLOR_TYPE_PALETTE, 4, PNG_INTERLACE_NONE),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_ADAM7),
    ] {
        let full = unsafe { made(ct, bd, 9, 5, il, true) };
        // every chunk boundary
        let mut bounds = vec![0usize, 8];
        let mut i = 8usize;
        while i + 8 <= full.len() {
            let l =
                u32::from_be_bytes([full[i], full[i + 1], full[i + 2], full[i + 3]]) as usize;
            bounds.push(i + 4);
            bounds.push(i + 8);
            bounds.push(i + 8 + l);
            i += 12 + l;
            bounds.push(i);
        }
        // plus a spread of byte offsets
        let step = (full.len() / 24).max(1);
        let mut offs: Vec<usize> = (0..full.len()).step_by(step).collect();
        offs.extend(bounds);
        offs.push(full.len());
        offs.sort_unstable();
        offs.dedup();
        for &n in &offs {
            if n > full.len() {
                continue;
            }
            dseq(
                &format!("trunc ct={} bd={} il={} at {}", ct, bd, il, n),
                &full[..n],
            );
        }
    }
}

#[test]
fn truncation_progressive() {
    for (ct, bd, il) in [
        (PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE),
        (PNG_COLOR_TYPE_PALETTE, 2, PNG_INTERLACE_NONE),
        (PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_ADAM7),
    ] {
        let full = unsafe { made(ct, bd, 7, 4, il, true) };
        let step = (full.len() / 16).max(1);
        for n in (0..=full.len()).step_by(step) {
            for gran in [0usize, 1, 5] {
                dpush(
                    &format!("push trunc ct={} bd={} il={} at {} gran={}", ct, bd, il, n, gran),
                    &full[..n],
                    gran,
                    0,
                );
            }
        }
    }
}

#[test]
fn bad_signature() {
    let good = unsafe { made(PNG_COLOR_TYPE_GRAY, 8, 4, 4, PNG_INTERLACE_NONE, false) };
    for pos in 0..8usize {
        for xor in [0xffu8, 0x01, 0x80] {
            let mut b = good.clone();
            b[pos] ^= xor;
            dseq(&format!("sig byte {} ^{:#x}", pos, xor), &b);
            for gran in [0usize, 1] {
                dpush(
                    &format!("push sig byte {} ^{:#x} gran={}", pos, xor, gran),
                    &b,
                    gran,
                    0,
                );
            }
        }
    }
    // The CR/LF mangling case: "PNG file corrupted by ASCII conversion"
    // requires the first four bytes to be intact.
    for pos in 4..8usize {
        let mut b = good.clone();
        b[pos] = b'x';
        dseq(&format!("sig ascii mangle at {}", pos), &b);
        dpush(&format!("push sig ascii mangle at {}", pos), &b, 1, 0);
    }
    // Signature bytes declared as already consumed.
    for nb in [0i32, 1, 4, 8] {
        same!(format!("sig_bytes={} then read", nb), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &good);
            (api.png_set_sig_bytes)(s.png, nb);
            (api.png_read_info)(s.png, s.info);
            snap(api, s.png, s.info) as i64
        });
    }
}

// ===========================================================================
// 3. CRC corruption + png_set_crc_action
//    rows 693-696
// ===========================================================================

#[test]
fn crc_action_matrix() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB, 8, 6, 4, PNG_INTERLACE_NONE, true) };
    let chs = split(&full);
    let actions = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
        -1,
        6,
        999,
    ];
    // Corrupt one critical (IDAT) and one ancillary (gAMA) CRC.
    for target in [b"IDAT", b"gAMA"] {
        let mut broken = chs.clone();
        for c in broken.iter_mut() {
            if c.is(target) {
                c.crc = Some(0xdead_beef);
            }
        }
        let bytes = asm(&broken);
        for &crit in &actions {
            for &anc in &actions {
                same!(
                    format!(
                        "crc_action crit={} anc={} broken={}",
                        crit,
                        anc,
                        String::from_utf8_lossy(target)
                    ),
                    |api: &'static Api| unsafe {
                        let s = ReadSess::new(api, &bytes);
                        (api.png_set_crc_action)(s.png, crit, anc);
                        (api.png_read_info)(s.png, s.info);
                        let mut a = snap(api, s.png, s.info);
                        (api.png_read_update_info)(s.png, s.info);
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let rb = (api.png_get_rowbytes)(s.png, s.info);
                        let mut buf: Vec<Vec<u8>> =
                            (0..h as usize).map(|_| vec![0u8; rb + 16]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(s.png, ptrs.as_mut_ptr());
                        (api.png_read_end)(s.png, s.end);
                        for r in &buf {
                            h64(&mut a, r);
                        }
                        a as i64
                    }
                );
            }
        }
    }
    same!("png_set_crc_action(NULL)", |api: &'static Api| {
        unsafe { (api.png_set_crc_action)(null_mut(), 3, 3) };
        0
    });
}

#[test]
fn crc_corruption_every_chunk() {
    for (ct, bd) in [(PNG_COLOR_TYPE_GRAY, 8), (PNG_COLOR_TYPE_PALETTE, 4)] {
        let full = unsafe { made(ct, bd, 6, 3, PNG_INTERLACE_NONE, true) };
        let chs = split(&full);
        for i in 0..chs.len() {
            for xor in [0xffff_ffffu32, 1] {
                let mut b = chs.clone();
                let good = {
                    let mut v = b[i].name.to_vec();
                    v.extend_from_slice(&b[i].data);
                    crc32(&v)
                };
                b[i].crc = Some(good ^ xor);
                let bytes = asm(&b);
                let nm = String::from_utf8_lossy(&chs[i].name).to_string();
                dseq(
                    &format!("crc ct={} bd={} chunk {} {} ^{:#x}", ct, bd, i, nm, xor),
                    &bytes,
                );
                dpush(
                    &format!("push crc ct={} bd={} chunk {} {} ^{:#x}", ct, bd, i, nm, xor),
                    &bytes,
                    0,
                    0,
                );
            }
        }
    }
}

// ===========================================================================
// 4. Corrupted IHDR
// ===========================================================================

#[test]
fn ihdr_field_corruption() {
    let base = hand(6, 4, 8, 2, 0);
    // widths / heights
    for &w in &[0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 1_000_001] {
        for &h in &[0u32, 1, 0x8000_0000, 1_000_001] {
            let mut b = base.clone();
            b[0].data = ihdr(w, h, 8, 2, 0, 0, 0);
            let bytes = asm(&b);
            dinfo(&format!("IHDR w={} h={}", w, h), &bytes);
            dpush(&format!("push IHDR w={} h={}", w, h), &bytes, 0, 0);
        }
    }
    // bit depth / colour type
    for bd in [0u8, 1, 2, 3, 4, 5, 7, 8, 9, 16, 17, 32, 255] {
        for ct in [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 255] {
            let mut b = base.clone();
            b[0].data = ihdr(6, 4, bd, ct, 0, 0, 0);
            let bytes = asm(&b);
            dinfo(&format!("IHDR bd={} ct={}", bd, ct), &bytes);
        }
    }
    // compression / filter / interlace method
    for comp in [0u8, 1, 2, 255] {
        for filt in [0u8, 1, 64, 65, 255] {
            for il in [0u8, 1, 2, 255] {
                let mut b = base.clone();
                b[0].data = ihdr(6, 4, 8, 2, comp, filt, il);
                let bytes = asm(&b);
                dinfo(
                    &format!("IHDR comp={} filt={} il={}", comp, filt, il),
                    &bytes,
                );
                // ... and with MNG features permitted, which legalises
                // filter method 64 (intrapixel differencing)
                same!(
                    format!("IHDR mng comp={} filt={} il={}", comp, filt, il),
                    |api: &'static Api| unsafe {
                        let s = ReadSess::new(api, &bytes);
                        (api.png_permit_mng_features)(s.png, PNG_ALL_MNG_FEATURES);
                        (api.png_read_info)(s.png, s.info);
                        snap(api, s.png, s.info) as i64
                    }
                );
            }
        }
    }
    // wrong IHDR length (the sequential and the progressive reader use
    // different code paths; the latter has its own "Invalid IHDR length")
    for l in [0usize, 1, 12, 13, 14, 20] {
        let mut b = base.clone();
        let mut d = ihdr(6, 4, 8, 2, 0, 0, 0);
        d.resize(l, 0);
        b[0].data = d;
        let bytes = asm(&b);
        dinfo(&format!("IHDR length {}", l), &bytes);
        for gran in [0usize, 1] {
            dpush(&format!("push IHDR length {} gran={}", l, gran), &bytes, gran, 0);
        }
    }
    // declared length longer than the data present
    for l in [14u32, 100, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        let mut b = base.clone();
        b[0].len = Some(l);
        let bytes = asm(&b);
        dinfo(&format!("IHDR declared length {}", l), &bytes);
        dpush(&format!("push IHDR declared length {}", l), &bytes, 0, 0);
    }
    // MNG intrapixel differencing with a colour type that is neither RGB nor
    // RGBA (rows 578, 579): png_do_read_intrapixel returns early.
    for ct in [0u8, 2, 3, 4, 6] {
        for bd in [8u8, 16] {
            if ct == 3 && bd == 16 {
                continue;
            }
            let mut b = hand(6, 4, bd, ct, 0);
            let mut d = b[0].data.clone();
            d[11] = PNG_INTRAPIXEL_DIFFERENCING as u8;
            b[0].data = d;
            let bytes = asm(&b);
            same!(
                format!("intrapixel ct={} bd={}", ct, bd),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &bytes);
                    (api.png_permit_mng_features)(s.png, PNG_ALL_MNG_FEATURES);
                    (api.png_read_info)(s.png, s.info);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    let h = (api.png_get_image_height)(s.png, s.info);
                    let rb = (api.png_get_rowbytes)(s.png, s.info);
                    let mut buf: Vec<Vec<u8>> =
                        (0..h as usize).map(|_| vec![0u8; rb + 16]).collect();
                    let mut p: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    (api.png_read_image)(s.png, p.as_mut_ptr());
                    (api.png_read_end)(s.png, s.end);
                    for r in &buf {
                        h64(&mut a, r);
                    }
                    a as i64
                }
            );
        }
    }
}

// ===========================================================================
// 5. Chunk ordering / duplication
//    rows 571, 572, 573, 599, 600, 760, 761, 762, 763
// ===========================================================================

#[test]
fn chunk_order_errors() {
    let g = hand(6, 4, 8, 2, 0); // IHDR IDAT IEND
    let p = hand(6, 4, 4, 3, 0); // IHDR PLTE IDAT IEND
    let gama = Ch::new(b"gAMA", 45455u32.to_be_bytes().to_vec());
    let text = Ch::new(b"tEXt", b"Key\0value".to_vec());

    // no IHDR at all
    let mut v: Vec<Ch> = g.iter().skip(1).cloned().collect();
    let b = asm(&v);
    dinfo("missing IHDR (IDAT first)", &b);
    dpush("push missing IHDR", &b, 0, 0);

    // an ancillary chunk before IHDR
    v = vec![gama.clone()];
    v.extend(g.iter().cloned());
    let b = asm(&v);
    dinfo("gAMA before IHDR", &b);
    dpush("push gAMA before IHDR", &b, 0, 0);

    // colour type 3 with no PLTE
    v = p.iter().filter(|c| !c.is(b"PLTE")).cloned().collect();
    let b = asm(&v);
    dinfo("ct=3 without PLTE", &b);
    dpush("push ct=3 without PLTE", &b, 0, 0);

    // PLTE after IDAT
    v = vec![p[0].clone(), p[2].clone(), p[1].clone(), p[3].clone()];
    let b = asm(&v);
    dseq("PLTE after IDAT (ct=3)", &b);
    dpush("push PLTE after IDAT (ct=3)", &b, 0, 0);
    v = vec![g[0].clone(), g[1].clone(), p[1].clone(), g[2].clone()];
    let b = asm(&v);
    dseq("PLTE after IDAT (ct=2)", &b);

    // zero-length PLTE (only legal with the MNG empty-PLTE feature)
    for ct in [2u8, 3u8] {
        let src = if ct == 3 { &p } else { &g };
        let mut w: Vec<Ch> = src.to_vec();
        if ct == 3 {
            w[1].data.clear();
        } else {
            w.insert(1, Ch::new(b"PLTE", Vec::new()));
        }
        let bytes = asm(&w);
        for mng in [0u32, PNG_ALL_MNG_FEATURES] {
            same!(
                format!("empty PLTE ct={} mng={:#x}", ct, mng),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &bytes);
                    (api.png_permit_mng_features)(s.png, mng);
                    (api.png_read_info)(s.png, s.info);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_set_expand)(s.png);
                    (api.png_read_update_info)(s.png, s.info);
                    hu(&mut a, snap(api, s.png, s.info));
                    a as i64
                }
            );
        }
    }

    // duplicated IHDR / PLTE / IEND
    for (nm, idx) in [("IHDR", 0usize), ("PLTE", 1), ("IDAT", 2), ("IEND", 3)] {
        let mut w = p.to_vec();
        let c = w[idx].clone();
        w.insert(idx + 1, c);
        let b = asm(&w);
        dseq(&format!("duplicate {}", nm), &b);
        dpush(&format!("push duplicate {}", nm), &b, 0, 0);
        // ... and the duplicate at the very end, after IEND
        let mut w2 = p.to_vec();
        let c2 = w2[idx].clone();
        w2.push(c2);
        let b2 = asm(&w2);
        dseq(&format!("{} after IEND", nm), &b2);
    }

    // IEND missing entirely
    let b = asm(&g[..g.len() - 1]);
    dseq("no IEND", &b);
    dpush("push no IEND", &b, 0, 0);

    // "Too many IDATs found": a non-IDAT chunk between two IDAT runs
    let raw = raw_image(6, 4, 24, 0, 0x40);
    let z = zlib_stored(&raw);
    let half = z.len() / 2;
    for mid in [gama.clone(), text.clone()] {
        let nm = String::from_utf8_lossy(&mid.name).to_string();
        let v = vec![
            g[0].clone(),
            Ch::new(b"IDAT", z[..half].to_vec()),
            mid.clone(),
            Ch::new(b"IDAT", z[half..].to_vec()),
            g[2].clone(),
        ];
        let b = asm(&v);
        dseq(&format!("IDAT {} IDAT", nm), &b);
        dpush(&format!("push IDAT {} IDAT", nm), &b, 0, 0);
    }
    // trailing IDAT chunks after the image data is complete: length 0 and >0,
    // with and without an intervening ancillary chunk (rows 599/600).
    for extra_len in [0usize, 1, 8] {
        for with_mid in [false, true] {
            let mut v = vec![g[0].clone(), g[1].clone()];
            if with_mid {
                v.push(text.clone());
            }
            v.push(Ch::new(b"IDAT", vec![0u8; extra_len]));
            v.push(g[2].clone());
            let b = asm(&v);
            dseq(
                &format!("trailing IDAT len={} mid={}", extra_len, with_mid),
                &b,
            );
            dpush(
                &format!("push trailing IDAT len={} mid={}", extra_len, with_mid),
                &b,
                0,
                0,
            );
            // ... and with IDAT handled as an unknown chunk, which is the
            // ".Too many IDATs found" path (row 599).
            same!(
                format!("IDAT-as-unknown len={} mid={}", extra_len, with_mid),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &b);
                    let list = b"IDAT\0".to_vec();
                    (api.png_set_keep_unknown_chunks)(
                        s.png,
                        PNG_HANDLE_CHUNK_ALWAYS,
                        list.as_ptr(),
                        1,
                    );
                    (api.png_read_info)(s.png, s.info);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_end)(s.png, s.end);
                    hu(&mut a, snap(api, s.png, s.end));
                    a as i64
                }
            );
        }
    }
    // chunks that must not follow IDAT
    for nm in ANC {
        let mut v = vec![g[0].clone(), g[1].clone()];
        v.push(Ch::new(nm, pattern(4)));
        v.push(g[2].clone());
        let b = asm(&v);
        dseq(
            &format!("{} after IDAT", String::from_utf8_lossy(nm)),
            &b,
        );
    }
    // data after IEND (progressive: process_mode is PNG_READ_DONE_MODE, so the
    // remaining input must be silently discarded -- row 756)
    let mut b = asm(&g);
    b.extend_from_slice(&[0u8; 40]);
    dseq("garbage after IEND", &b);
    for gran in [0usize, 1, 7] {
        dpush(&format!("push garbage after IEND gran={}", gran), &b, gran, 0);
    }
    let mut b2 = asm(&g);
    b2.extend_from_slice(&asm(&g)[8..]);
    dpush("push whole second image after IEND", &b2, 0, 0);
}

// ===========================================================================
// 6. Malformed chunk headers: bad names, impossible lengths
// ===========================================================================

#[test]
fn chunk_header_errors() {
    let g = hand(6, 4, 8, 2, 0);
    // non-alphabetic chunk names
    for nm in [
        &b"\x00\x00\x00\x00"[..],
        b"1234",
        b"IH R",
        b"ihdr",
        b"\xff\xff\xff\xff",
        b"gAM\x00",
        b"\x80AMA",
        b"aB{D",
        b"[EST",
        b"te\tt",
    ] {
        let mut v = vec![g[0].clone(), Ch::new(nm, pattern(4))];
        v.extend(g[1..].iter().cloned());
        let b = asm(&v);
        dseq(&format!("chunk name {:?}", nm), &b);
        dpush(&format!("push chunk name {:?}", nm), &b, 0, 0);
    }
    // lengths that exceed the remaining data / the 31-bit PNG limit
    for l in [
        5u32,
        100,
        1_000_000,
        8_000_001,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ] {
        for nm in [&b"gAMA"[..], b"tEXt", b"IDAT", b"unKn"] {
            let mut v = vec![g[0].clone()];
            let mut c = Ch::new(nm, 45455u32.to_be_bytes().to_vec());
            c.len = Some(l);
            v.push(c);
            v.extend(g[1..].iter().cloned());
            let b = asm(&v);
            dseq(
                &format!("{} declared length {}", String::from_utf8_lossy(nm), l),
                &b,
            );
            dpush(
                &format!("push {} declared length {}", String::from_utf8_lossy(nm), l),
                &b,
                0,
                0,
            );
        }
    }
}

// ===========================================================================
// 7. Every ancillary chunk at every (wrong) length
// ===========================================================================

#[test]
fn ancillary_chunk_lengths() {
    for (ct, bd) in [(PNG_COLOR_TYPE_RGB, 8), (PNG_COLOR_TYPE_PALETTE, 4)] {
        let g = hand(6, 4, bd as u8, ct as u8, 0);
        let idat = g.iter().position(|c| c.is(b"IDAT")).unwrap();
        for nm in ANC {
            for l in [0usize, 1, 3, 4, 5, 7, 9, 13, 25, 33, 300, 1100] {
                let c = Ch::new(nm, pattern(l));
                // before IDAT
                let mut v = g.to_vec();
                v.insert(idat, c.clone());
                let b = asm(&v);
                dseq(
                    &format!(
                        "ct={} {} len={} before IDAT",
                        ct,
                        String::from_utf8_lossy(nm),
                        l
                    ),
                    &b,
                );
                // after IDAT
                let mut v2 = g.to_vec();
                v2.insert(idat + 1, c.clone());
                let b2 = asm(&v2);
                dseq(
                    &format!(
                        "ct={} {} len={} after IDAT",
                        ct,
                        String::from_utf8_lossy(nm),
                        l
                    ),
                    &b2,
                );
            }
        }
    }
    // and the same through the progressive reader, at the interesting lengths
    let g = hand(5, 3, 8, 0, 0);
    let idat = g.iter().position(|c| c.is(b"IDAT")).unwrap();
    for nm in ANC {
        for l in [0usize, 4, 300] {
            let mut v = g.to_vec();
            v.insert(idat, Ch::new(nm, pattern(l)));
            let b = asm(&v);
            dpush(
                &format!("push {} len={}", String::from_utf8_lossy(nm), l),
                &b,
                0,
                0,
            );
        }
    }
}

// ===========================================================================
// 8. Corrupted zlib stream inside IDAT
//    rows 589, 767, 773-779
// ===========================================================================

#[test]
fn idat_zlib_corruption() {
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8u8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let w = 5u32;
        let h = 3u32;
        let pd = channels_of(ct) * bd as u32;
        let raw = raw_image(w, h, pd, 0, 0x30);
        let z = zlib_stored(&raw);
        let base = hand(w, h, bd, ct as u8, 0);
        let idat = base.iter().position(|c| c.is(b"IDAT")).unwrap();
        let mut variants: Vec<(String, Vec<u8>)> = Vec::new();
        // bad CMF/FLG
        for cmf in [0x00u8, 0x08, 0x18, 0x28, 0x58, 0x68, 0x79, 0xff] {
            let mut y = z.clone();
            y[0] = cmf;
            variants.push((format!("cmf={:#x}", cmf), y));
        }
        for flg in [0x00u8, 0x02, 0x20, 0xff] {
            let mut y = z.clone();
            y[1] = flg;
            variants.push((format!("flg={:#x}", flg), y));
        }
        // corrupt ADLER32
        for k in 1..5usize {
            let mut y = z.clone();
            let n = y.len();
            y[n - k] ^= 0xff;
            variants.push((format!("adler byte -{}", k), y));
        }
        // truncated deflate data
        for cut in [1usize, 2, 3, 5, 6, 10] {
            if cut < z.len() {
                variants.push((format!("cut {} bytes", cut), z[..z.len() - cut].to_vec()));
            }
        }
        // extra bytes after the zlib stream
        for extra in [1usize, 4, 32] {
            let mut y = z.clone();
            y.extend(std::iter::repeat(0x5au8).take(extra));
            variants.push((format!("extra {} bytes", extra), y));
        }
        // an entirely empty zlib stream, and one that decodes to nothing
        variants.push(("empty".into(), Vec::new()));
        variants.push(("no data".into(), zlib_stored(&[])));
        // too little / too much uncompressed data
        variants.push((
            "short raw".into(),
            zlib_stored(&raw[..raw.len().saturating_sub(3)]),
        ));
        let mut long = raw.clone();
        long.extend_from_slice(&[0u8; 9]);
        variants.push(("long raw".into(), zlib_stored(&long)));
        for (tag, zz) in &variants {
            let mut v = base.to_vec();
            v[idat] = Ch::new(b"IDAT", zz.clone());
            let b = asm(&v);
            dseq(&format!("ct={} bd={} idat {}", ct, bd, tag), &b);
            dpush(&format!("push ct={} bd={} idat {}", ct, bd, tag), &b, 0, 0);
            // ... and with the ADLER32 check switched off
            same!(
                format!("ct={} bd={} idat {} ignore-adler", ct, bd, tag),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &b);
                    (api.png_set_option)(s.png, PNG_IGNORE_ADLER32, PNG_OPTION_ON);
                    (api.png_read_info)(s.png, s.info);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    let hh = (api.png_get_image_height)(s.png, s.info);
                    let rb = (api.png_get_rowbytes)(s.png, s.info);
                    let mut buf: Vec<Vec<u8>> =
                        (0..hh as usize).map(|_| vec![0u8; rb + 16]).collect();
                    let mut p: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    (api.png_read_image)(s.png, p.as_mut_ptr());
                    (api.png_read_end)(s.png, s.end);
                    for r in &buf {
                        h64(&mut a, r);
                    }
                    a as i64
                }
            );
        }
        // split the IDAT into many pieces (including zero-length ones)
        for piece in [1usize, 2, 3, 7, 1000] {
            let mut v: Vec<Ch> = base[..idat].to_vec();
            let mut i = 0usize;
            while i < z.len() {
                let n = piece.min(z.len() - i);
                v.push(Ch::new(b"IDAT", z[i..i + n].to_vec()));
                i += n;
            }
            v.extend(base[idat + 1..].iter().cloned());
            let b = asm(&v);
            dseq(&format!("ct={} bd={} idat split {}", ct, bd, piece), &b);
            dpush(
                &format!("push ct={} bd={} idat split {}", ct, bd, piece),
                &b,
                0,
                0,
            );
        }
        // zero-length IDAT chunks interleaved (row 762)
        let mut v: Vec<Ch> = base[..idat].to_vec();
        v.push(Ch::new(b"IDAT", Vec::new()));
        v.push(Ch::new(b"IDAT", z.clone()));
        v.push(Ch::new(b"IDAT", Vec::new()));
        v.extend(base[idat + 1..].iter().cloned());
        let b = asm(&v);
        dseq(&format!("ct={} bd={} empty IDATs", ct, bd), &b);
        for gran in [0usize, 1] {
            dpush(
                &format!("push ct={} bd={} empty IDATs gran={}", ct, bd, gran),
                &b,
                gran,
                0,
            );
        }
    }
}

#[test]
fn bad_filter_bytes() {
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1u8),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for filt in [0u8, 1, 2, 3, 4, 5, 6, 7, 64, 128, 254, 255] {
            let b = asm(&hand(5, 3, bd, ct as u8, filt));
            dseq(&format!("filter {} ct={} bd={}", filt, ct, bd), &b);
            dpush(&format!("push filter {} ct={} bd={}", filt, ct, bd), &b, 0, 0);
        }
        // only the *second* row's filter byte is bad
        let w = 5u32;
        let h = 3u32;
        let pd = channels_of(ct) * bd as u32;
        let rb = rowbytes(pd, w);
        for filt in [5u8, 255] {
            let mut raw = raw_image(w, h, pd, 0, 0x30);
            raw[rb + 1] = filt;
            let mut v = hand(w, h, bd, ct as u8, 0);
            let i = v.iter().position(|c| c.is(b"IDAT")).unwrap();
            v[i] = Ch::new(b"IDAT", zlib_stored(&raw));
            let b = asm(&v);
            dseq(&format!("row1 filter {} ct={} bd={}", filt, ct, bd), &b);
            dpush(
                &format!("push row1 filter {} ct={} bd={}", filt, ct, bd),
                &b,
                0,
                0,
            );
        }
    }
}

// ===========================================================================
// 9. png_read_png
//    rows 603-619
// ===========================================================================

#[test]
fn read_png_transform_masks() {
    // NOTE: rows 605-619 ("PNG_TRANSFORM_* not supported") are UNREACHABLE in
    // this build: pnglibconf.h enables every one of PNG_READ_SCALE_16_TO_8,
    // STRIP_16_TO_8, STRIP_ALPHA, PACK, PACKSWAP, EXPAND, INVERT, SHIFT, BGR,
    // SWAP_ALPHA, SWAP, INVERT_ALPHA, GRAY_TO_RGB and EXPAND_16, so the
    // png_app_error arms in pngread.c:892-1027 are compiled out.  Each flag is
    // still driven here to prove the *supported* path agrees.
    let masks: Vec<c_int> = vec![
        PNG_TRANSFORM_IDENTITY,
        PNG_TRANSFORM_STRIP_16,
        PNG_TRANSFORM_STRIP_ALPHA,
        PNG_TRANSFORM_PACKING,
        PNG_TRANSFORM_PACKSWAP,
        PNG_TRANSFORM_EXPAND,
        PNG_TRANSFORM_INVERT_MONO,
        PNG_TRANSFORM_SHIFT,
        PNG_TRANSFORM_BGR,
        PNG_TRANSFORM_SWAP_ALPHA,
        PNG_TRANSFORM_SWAP_ENDIAN,
        PNG_TRANSFORM_INVERT_ALPHA,
        PNG_TRANSFORM_STRIP_FILLER_BEFORE,
        PNG_TRANSFORM_STRIP_FILLER_AFTER,
        PNG_TRANSFORM_GRAY_TO_RGB,
        PNG_TRANSFORM_EXPAND_16,
        PNG_TRANSFORM_SCALE_16,
        PNG_TRANSFORM_SCALE_16 | PNG_TRANSFORM_STRIP_16,
        // unknown / oversized masks
        0x1_0000,
        0x8000_0000u32 as c_int,
        0xffff,
        -1,
        999,
        i32::MIN,
        i32::MAX,
    ];
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for anc in [false, true] {
            let full = unsafe { made(ct, bd, 7, 3, PNG_INTERLACE_NONE, anc) };
            for &m in &masks {
                same!(
                    format!("read_png ct={} bd={} anc={} mask={:#x}", ct, bd, anc, m),
                    |api: &'static Api| unsafe {
                        let s = ReadSess::new(api, &full);
                        (api.png_read_png)(s.png, s.info, m, null_mut());
                        let mut a = snap(api, s.png, s.info);
                        let rp = (api.png_get_rows)(s.png, s.info);
                        let hh = (api.png_get_image_height)(s.png, s.info);
                        let rb = (api.png_get_rowbytes)(s.png, s.info);
                        hu(&mut a, rp.is_null() as u64);
                        if !rp.is_null() && (hh as u64) * (rb as u64) <= MAXBUF {
                            let pd = (api.png_get_channels)(s.png, s.info) as u32
                                * (api.png_get_bit_depth)(s.png, s.info) as u32;
                            let w = (api.png_get_image_width)(s.png, s.info);
                            let ps = (m & PNG_TRANSFORM_PACKSWAP) != 0;
                            for y in 0..hh as usize {
                                let p = *rp.add(y);
                                if !p.is_null() {
                                    let mut r = std::slice::from_raw_parts(p, rb).to_vec();
                                    mask_padding(&mut r, pd, w, ps);
                                    h64(&mut a, &r);
                                }
                            }
                        }
                        a as i64
                    }
                );
            }
            // a truncated stream through png_read_png
            for cut in [1usize, 12, full.len() / 2] {
                same!(
                    format!("read_png ct={} bd={} anc={} cut={}", ct, bd, anc, cut),
                    |api: &'static Api| unsafe {
                        let s = ReadSess::new(api, &full[..full.len() - cut]);
                        (api.png_read_png)(s.png, s.info, PNG_TRANSFORM_IDENTITY, null_mut());
                        snap(api, s.png, s.info) as i64
                    }
                );
            }
        }
    }
    // row 604: "Image is too high to process with png_read_png()" needs a
    // height above PNG_UINT_32_MAX/sizeof(png_bytep), which is only reachable
    // once the default 1,000,000-row user limit has been raised.
    // NOTE: the largest height that PASSES the check, PNG_UINT_32_MAX/8 ==
    // 536,870,911, is deliberately NOT tested: png_read_png would then go on
    // to allocate a 4 GiB row-pointer array plus one row buffer per row
    // (pngread.c:1035-1049), which exhausts memory in both libraries rather
    // than producing an error to compare.
    for h in [536_870_912u32, 536_870_913, 0x4000_0000, 0x7fff_ffff] {
        let mut v = hand(1, 1, 8, 0, 0);
        v[0].data = ihdr(1, h, 8, 0, 0, 0, 0);
        let b = asm(&v);
        same!(format!("read_png tall h={}", h), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &b);
            (api.png_set_user_limits)(s.png, 0x7fff_ffff, 0x7fff_ffff);
            (api.png_read_png)(s.png, s.info, PNG_TRANSFORM_IDENTITY, null_mut());
            snap(api, s.png, s.info) as i64
        });
    }
}

// ===========================================================================
// 10. pngrtran.c — png_rtran_ok gating for every setter
//     rows 697-704, 708, 712, 713, 716, 721-727
// ===========================================================================

/// Which read-transform setter to call.
#[derive(Clone, Copy, Debug)]
enum T {
    Background,
    Scale16,
    Strip16,
    StripAlpha,
    AlphaMode,
    Quantize,
    QuantizeNullPal,
    Gamma,
    Expand,
    PaletteToRgb,
    ExpandGray124,
    TrnsToAlpha,
    Expand16,
    GrayToRgb,
    RgbToGray,
}

const ALL_T: [T; 15] = [
    T::Background,
    T::Scale16,
    T::Strip16,
    T::StripAlpha,
    T::AlphaMode,
    T::Quantize,
    T::QuantizeNullPal,
    T::Gamma,
    T::Expand,
    T::PaletteToRgb,
    T::ExpandGray124,
    T::TrnsToAlpha,
    T::Expand16,
    T::GrayToRgb,
    T::RgbToGray,
];

unsafe fn call_t(api: &'static Api, png: png_structp, t: T) {
    let bg = png_color_16 {
        index: 0,
        red: 1,
        green: 1,
        blue: 1,
        gray: 1,
    };
    let mut pal: Vec<png_color> = (0..16)
        .map(|i| png_color {
            red: i as u8,
            green: (i * 3) as u8,
            blue: (i * 5) as u8,
        })
        .collect();
    match t {
        T::Background => (api.png_set_background_fixed)(
            png,
            &bg as *const png_color_16,
            PNG_BACKGROUND_GAMMA_SCREEN,
            0,
            100000,
        ),
        T::Scale16 => (api.png_set_scale_16)(png),
        T::Strip16 => (api.png_set_strip_16)(png),
        T::StripAlpha => (api.png_set_strip_alpha)(png),
        T::AlphaMode => (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_PNG, 100000),
        T::Quantize => {
            (api.png_set_quantize)(png, pal.as_mut_ptr(), 16, 8, null(), 1);
        }
        T::QuantizeNullPal => {
            (api.png_set_quantize)(png, null_mut(), 16, 8, null(), 1);
        }
        T::Gamma => (api.png_set_gamma_fixed)(png, 100000, 45455),
        T::Expand => (api.png_set_expand)(png),
        T::PaletteToRgb => (api.png_set_palette_to_rgb)(png),
        T::ExpandGray124 => (api.png_set_expand_gray_1_2_4_to_8)(png),
        T::TrnsToAlpha => (api.png_set_tRNS_to_alpha)(png),
        T::Expand16 => (api.png_set_expand_16)(png),
        T::GrayToRgb => (api.png_set_gray_to_rgb)(png),
        T::RgbToGray => {
            (api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, -1, -1)
        }
    }
}

#[test]
fn rtran_ok_gating() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 6, 3, PNG_INTERLACE_NONE, true) };
    for t in ALL_T {
        // (a) NULL png_ptr -> png_rtran_ok returns 0, no diagnostic possible
        same!(format!("{:?} on NULL", t), |api: &'static Api| unsafe {
            call_t(api, null_mut(), t);
            0
        });
        // (b) before png_read_info (no IHDR yet): only the need_IHDR setters
        // complain ("invalid before the PNG header has been read")
        same!(format!("{:?} before read_info", t), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            call_t(api, s.png, t);
            0
        });
        // (c) the normal place: after png_read_info
        same!(format!("{:?} after read_info", t), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            (api.png_read_info)(s.png, s.info);
            call_t(api, s.png, t);
            let mut a = snap(api, s.png, s.info);
            (api.png_read_update_info)(s.png, s.info);
            hu(&mut a, snap(api, s.png, s.info));
            a as i64
        });
        // (d) after png_read_update_info -> "invalid after
        // png_start_read_image or png_read_update_info"
        same!(format!("{:?} after update_info", t), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            (api.png_read_info)(s.png, s.info);
            (api.png_read_update_info)(s.png, s.info);
            call_t(api, s.png, t);
            snap(api, s.png, s.info) as i64
        });
        // (e) after png_start_read_image
        same!(format!("{:?} after start_read_image", t), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            (api.png_read_info)(s.png, s.info);
            (api.png_start_read_image)(s.png);
            call_t(api, s.png, t);
            snap(api, s.png, s.info) as i64
        });
        // (f) after the whole image has been read
        same!(format!("{:?} after read_end", t), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            (api.png_read_info)(s.png, s.info);
            (api.png_read_update_info)(s.png, s.info);
            let h = (api.png_get_image_height)(s.png, s.info);
            let rb = (api.png_get_rowbytes)(s.png, s.info);
            let mut buf: Vec<Vec<u8>> = (0..h as usize).map(|_| vec![0u8; rb + 16]).collect();
            let mut p: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
            (api.png_read_image)(s.png, p.as_mut_ptr());
            (api.png_read_end)(s.png, s.end);
            call_t(api, s.png, t);
            snap(api, s.png, s.info) as i64
        });
        // (g) on a WRITE struct
        same!(format!("{:?} on write struct", t), |api: &'static Api| unsafe {
            let s = WriteSess::new(api);
            call_t(api, s.png, t);
            0
        });
        // (h) on a write struct that already has an IHDR
        same!(format!("{:?} on write struct with IHDR", t), |api: &'static Api| unsafe {
            let s = WriteSess::new(api);
            (api.png_set_IHDR)(s.png, s.info, 4, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
            call_t(api, s.png, t);
            0
        });
    }
    // png_set_background_fixed with a NULL background colour (row 700)
    for before_info in [false, true] {
        same!(
            format!("set_background(NULL colour) before_info={}", before_info),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                if !before_info {
                    (api.png_read_info)(s.png, s.info);
                }
                (api.png_set_background_fixed)(
                    s.png,
                    null(),
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    0,
                    100000,
                );
                0
            }
        );
    }
}

// ===========================================================================
// 11. Gamma / alpha-mode / background value validation
//     rows 701, 705-711, 716-720, 736
// ===========================================================================

#[test]
fn gamma_value_rejections() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 5, 3, PNG_INTERLACE_NONE, true) };
    let fixed = [
        0i32,
        1,
        -1, // PNG_DEFAULT_sRGB
        -2, // PNG_GAMMA_MAC_18
        -3,
        -100000,
        999,
        1000,
        1001,
        45455,
        100000,
        10_000_000,
        10_000_001,
        i32::MAX,
        i32::MIN,
        PNG_FP_MAX,
        -PNG_FP_MAX,
    ];
    for &sg in &fixed {
        for &fg in &[0i32, 1, -1, -2, 1000, 45455, 100000, 10_000_001, i32::MIN] {
            same!(
                format!("png_set_gamma_fixed({},{})", sg, fg),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    (api.png_read_info)(s.png, s.info);
                    (api.png_set_gamma_fixed)(s.png, sg, fg);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    hu(&mut a, snap(api, s.png, s.info));
                    a as i64
                }
            );
        }
    }
    // the floating-point entry point, including values that overflow
    // png_fixed_point in convert_gamma_value (row 705)
    let dbl = [
        0.0f64,
        1.0,
        -1.0,
        -2.0,
        -3.0,
        0.45455,
        2.2,
        127.9,
        128.0,
        1e9,
        1e30,
        -1e30,
        f64::MAX,
        f64::MIN,
        21475.0,
        -21475.0,
        1e-10,
    ];
    for &sg in &dbl {
        for &fg in &[1.0f64, 0.0, -1.0, 0.45455, 1e30] {
            same!(
                format!("png_set_gamma({},{})", sg, fg),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    (api.png_read_info)(s.png, s.info);
                    (api.png_set_gamma)(s.png, sg, fg);
                    snap(api, s.png, s.info) as i64
                }
            );
        }
        same!(
            format!("png_set_alpha_mode(PNG_ALPHA_PNG,{})", sg),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_read_info)(s.png, s.info);
                (api.png_set_alpha_mode)(s.png, PNG_ALPHA_PNG, sg);
                snap(api, s.png, s.info) as i64
            }
        );
    }
}

#[test]
fn alpha_mode_rejections() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 5, 3, PNG_INTERLACE_NONE, true) };
    let modes = [
        PNG_ALPHA_PNG,
        PNG_ALPHA_STANDARD,
        PNG_ALPHA_OPTIMIZED,
        PNG_ALPHA_BROKEN,
        -1,
        4,
        5,
        999,
        i32::MIN,
        i32::MAX,
    ];
    for &m in &modes {
        for &g in &[100000i32, 0, -1, -2, 999, 45455, 10_000_001, i32::MIN] {
            same!(
                format!("png_set_alpha_mode_fixed({},{})", m, g),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    (api.png_read_info)(s.png, s.info);
                    (api.png_set_alpha_mode_fixed)(s.png, m, g);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    hu(&mut a, snap(api, s.png, s.info));
                    a as i64
                }
            );
        }
        // "conflicting calls to set alpha mode and background" (row 711):
        // png_set_background first, then a pre-multiplying alpha mode.
        for order in [0, 1] {
            same!(
                format!("alpha_mode({}) + background order={}", m, order),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    (api.png_read_info)(s.png, s.info);
                    let bg = png_color_16 {
                        index: 0,
                        red: 1,
                        green: 1,
                        blue: 1,
                        gray: 1,
                    };
                    if order == 0 {
                        (api.png_set_background_fixed)(
                            s.png,
                            &bg as *const png_color_16,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0,
                            100000,
                        );
                        (api.png_set_alpha_mode_fixed)(s.png, m, 100000);
                    } else {
                        (api.png_set_alpha_mode_fixed)(s.png, m, 100000);
                        (api.png_set_background_fixed)(
                            s.png,
                            &bg as *const png_color_16,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0,
                            100000,
                        );
                    }
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    hu(&mut a, snap(api, s.png, s.info));
                    a as i64
                }
            );
        }
        // two pre-multiplying alpha modes in a row
        same!(
            format!("alpha_mode({}) twice", m),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_read_info)(s.png, s.info);
                (api.png_set_alpha_mode_fixed)(s.png, m, 100000);
                (api.png_set_alpha_mode_fixed)(s.png, m, 100000);
                snap(api, s.png, s.info) as i64
            }
        );
    }
}

#[test]
fn background_gamma_type_rejections() {
    // row 736: "invalid background gamma type" comes out of
    // png_init_read_transformations, i.e. at png_read_update_info time.
    let codes = [
        PNG_BACKGROUND_GAMMA_UNKNOWN,
        PNG_BACKGROUND_GAMMA_SCREEN,
        PNG_BACKGROUND_GAMMA_FILE,
        PNG_BACKGROUND_GAMMA_UNIQUE,
        -1,
        4,
        5,
        999,
        i32::MIN,
        i32::MAX,
    ];
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let full = unsafe { made(ct, bd, 5, 3, PNG_INTERLACE_NONE, true) };
        for &code in &codes {
            for need in [0i32, 1, -1, 999] {
                for gamma in [100000i32, 0, -1, 45455] {
                    same!(
                        format!(
                            "background ct={} bd={} code={} need={} g={}",
                            ct, bd, code, need, gamma
                        ),
                        |api: &'static Api| unsafe {
                            let s = ReadSess::new(api, &full);
                            (api.png_read_info)(s.png, s.info);
                            // In-range components: with need_expand == 0 the
                            // value indexes the 256-entry gamma table
                            // directly (pngrtran.c:1712), so a 16-bit value
                            // there would be an out-of-bounds read in the C.
                            let bg = png_color_16 {
                                index: 1,
                                red: 1,
                                green: 1,
                                blue: 1,
                                gray: 1,
                            };
                            (api.png_set_expand)(s.png);
                            (api.png_set_background_fixed)(
                                s.png,
                                &bg as *const png_color_16,
                                code,
                                need,
                                gamma,
                            );
                            (api.png_set_gamma_fixed)(s.png, 100000, 45455);
                            let mut a = snap(api, s.png, s.info);
                            (api.png_read_update_info)(s.png, s.info);
                            hu(&mut a, snap(api, s.png, s.info));
                            a as i64
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn rgb_to_gray_rejections() {
    let actions = [
        PNG_ERROR_ACTION_NONE,
        PNG_ERROR_ACTION_WARN,
        PNG_ERROR_ACTION_ERROR,
        0,
        -1,
        4,
        999,
        i32::MIN,
        i32::MAX,
    ];
    // NOTE (C UB, dropped): (i32::MAX, i32::MAX).  The range check in
    // png_set_rgb_to_gray_fixed is `red >= 0 && green >= 0 && red + green <=
    // PNG_FP_1` (pngrtran.c:1084); `red + green` overflows a signed int, which
    // is C undefined behaviour and in practice wraps to -2, so the pair is
    // ACCEPTED and rgb_to_gray_{red,green}_coeff end up far above 32768.
    // png_do_rgb_to_gray then produces a `w` beyond the sample range and uses
    // it to index png_ptr->gamma_from_1[] out of bounds -- a read whose result
    // depends on the surrounding heap, so it is not comparable.  Pairs whose
    // sum does NOT overflow (including (100000, 1) and (50000, 50001), which
    // take the "ignoring out of range rgb_to_gray coefficients" path) are
    // tested.
    let coeffs: [(i32, i32); 12] = [
        (-1, -1),
        (-1, 50000),
        (50000, -1),
        (0, 0),
        (21260, 71520),
        (50000, 50001),
        (100000, 1),
        (100000, 100000),
        (i32::MAX, 0),
        (0, i32::MAX),
        (i32::MIN, i32::MIN),
        (100000, 0),
    ];
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    ] {
        let full = unsafe { made(ct, bd, 5, 3, PNG_INTERLACE_NONE, true) };
        for &a in &actions {
            for &(r, g) in &coeffs {
                same!(
                    format!("rgb_to_gray ct={} bd={} act={} r={} g={}", ct, bd, a, r, g),
                    |api: &'static Api| unsafe {
                        let s = ReadSess::new(api, &full);
                        (api.png_read_info)(s.png, s.info);
                        (api.png_set_rgb_to_gray_fixed)(s.png, a, r, g);
                        let mut acc = snap(api, s.png, s.info);
                        (api.png_read_update_info)(s.png, s.info);
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let rb = (api.png_get_rowbytes)(s.png, s.info);
                        let mut buf: Vec<Vec<u8>> =
                            (0..h as usize).map(|_| vec![0u8; rb + 16]).collect();
                        let mut p: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(s.png, p.as_mut_ptr());
                        (api.png_read_end)(s.png, s.end);
                        for r in &buf {
                            h64(&mut acc, r);
                        }
                        hu(&mut acc, (api.png_get_rgb_to_gray_status)(s.png) as u64);
                        acc as i64
                    }
                );
            }
            // the floating-point entry point (in-range values only; the
            // overflowing ones live in rgb_to_gray_float_overflow below)
            for &(r, g) in &[(-1.0f64, -1.0f64), (0.3, 0.6), (0.9, 0.9), (0.5, 0.5)] {
                same!(
                    format!("rgb_to_gray(f) ct={} bd={} act={} r={} g={}", ct, bd, a, r, g),
                    |api: &'static Api| unsafe {
                        let s = ReadSess::new(api, &full);
                        (api.png_read_info)(s.png, s.info);
                        (api.png_set_rgb_to_gray)(s.png, a, r, g);
                        snap(api, s.png, s.info) as i64
                    }
                );
            }
        }
    }
}

// DIVERGENCE (found here, since RESOLVED in the Rust by another change to
// src/pngrtran_a.rs while this file was being written -- see the report):
// png_set_rgb_to_gray, the FLOATING-POINT entry point, when BOTH coefficients
// overflow png_fixed_point.
//
//   function : png_set_rgb_to_gray(png_ptr, error_action, red, green)
//   inputs   : read struct after png_read_info; error_action = 1
//              (PNG_ERROR_ACTION_NONE); red = green = 1e30 (likewise 1e300,
//              f64::MAX, -1e30, 21475.0 -- any pair both outside
//              +-21474.83647)
//   C result : png_error, "fixed point overflow in rgb to gray green
//              coefficient"
//   RS result: png_error, "fixed point overflow in rgb to gray red
//              coefficient"  (before the fix)
//
// Both libraries always rejected the call fatally with the same return value;
// only the message differed.  The C wrapper (pngrtran.c:1122-1125) passes the
// two png_fixed() conversions as arguments of ONE call:
//
//     png_set_rgb_to_gray_fixed(png_ptr, error_action,
//         png_fixed(png_ptr, red,   "rgb to gray red coefficient"),
//         png_fixed(png_ptr, green, "rgb to gray green coefficient"));
//
// Argument evaluation order is unspecified in C; the reference build (gcc,
// x86-64) evaluates right-to-left, so the *green* conversion runs first and its
// png_fixed_error is the observable one.  Rust evaluates left to right, so the
// translation reported red.  src/pngrtran_a.rs now binds the two conversions to
// locals in the reference build's order; this test locks that behaviour in.
#[test]
fn rgb_to_gray_float_overflow() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB, 8, 5, 3, PNG_INTERLACE_NONE, true) };
    for &(r, g) in &[
        (1e30f64, 1e30f64),
        (1e300, 1e300),
        (-1e30, -1e30),
        (f64::MAX, f64::MAX),
        (1e30, 0.5),
        (0.5, 1e30),
        (21475.0, 21475.0),
    ] {
        same!(
            format!("rgb_to_gray(f) overflow r={} g={}", r, g),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_read_info)(s.png, s.info);
                (api.png_set_rgb_to_gray)(s.png, PNG_ERROR_ACTION_NONE, r, g);
                snap(api, s.png, s.info) as i64
            }
        );
    }
}

// ===========================================================================
// 12. User limits
// ===========================================================================

#[test]
fn user_limit_rejections() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB, 8, 16, 9, PNG_INTERLACE_NONE, true) };
    for (w, h) in [
        (0u32, 0u32),
        (1, 1),
        (15, 9),
        (16, 8),
        (16, 9),
        (17, 10),
        (0x7fff_ffff, 0x7fff_ffff),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        same!(
            format!("png_set_user_limits({},{})", w, h),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_set_user_limits)(s.png, w, h);
                (api.png_read_info)(s.png, s.info);
                let mut a = snap(api, s.png, s.info);
                (api.png_read_update_info)(s.png, s.info);
                hu(&mut a, snap(api, s.png, s.info));
                a as i64
            }
        );
        same!(
            format!("png_set_user_limits({},{}) on NULL", w, h),
            |api: &'static Api| {
                unsafe { (api.png_set_user_limits)(null_mut(), w, h) };
                0
            }
        );
    }
    // chunk cache limit: a stream with many ancillary chunks
    for n in [0u32, 1, 2, 3, 5, 20, 1000, 0xffff_ffff] {
        let g = hand(4, 2, 8, 2, 0);
        let mut v = vec![g[0].clone()];
        for i in 0..12u8 {
            v.push(Ch::new(
                b"tEXt",
                [b"Key", &[b'A' + i][..], b"\0value"].concat(),
            ));
        }
        v.extend(g[1..].iter().cloned());
        let b = asm(&v);
        same!(
            format!("png_set_chunk_cache_max({})", n),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &b);
                (api.png_set_chunk_cache_max)(s.png, n);
                (api.png_read_info)(s.png, s.info);
                let mut a = snap(api, s.png, s.info);
                let mut tp: png_textp = null_mut();
                let mut nt = 0i32;
                hu(
                    &mut a,
                    (api.png_get_text)(s.png, s.info, &mut tp, &mut nt) as u64,
                );
                hu(&mut a, nt as i64 as u64);
                a as i64
            }
        );
        same!(
            format!("png_set_chunk_cache_max({}) on NULL", n),
            |api: &'static Api| {
                unsafe { (api.png_set_chunk_cache_max)(null_mut(), n) };
                0
            }
        );
    }
    // chunk malloc limit
    for m in [0usize, 1, 2, 10, 100, 8_000_000, usize::MAX] {
        let g = hand(4, 2, 8, 2, 0);
        let mut v = vec![g[0].clone(), Ch::new(b"tEXt", pattern(200))];
        v.push(Ch::new(b"zTXt", {
            let mut d = b"Key\0\0".to_vec();
            d.extend(zlib_stored(b"hello hello hello"));
            d
        }));
        v.extend(g[1..].iter().cloned());
        let b = asm(&v);
        same!(
            format!("png_set_chunk_malloc_max({})", m),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &b);
                (api.png_set_chunk_malloc_max)(s.png, m);
                (api.png_read_info)(s.png, s.info);
                let mut a = snap(api, s.png, s.info);
                (api.png_read_end)(s.png, s.end);
                hu(&mut a, snap(api, s.png, s.end));
                a as i64
            }
        );
        same!(
            format!("png_set_chunk_malloc_max({}) on NULL", m),
            |api: &'static Api| {
                unsafe { (api.png_set_chunk_malloc_max)(null_mut(), m) };
                0
            }
        );
    }
}

// ===========================================================================
// 13. Progressive reader specifics
//     rows 751-786
// ===========================================================================

#[test]
fn progressive_misuse() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB, 8, 7, 5, PNG_INTERLACE_NONE, true) };
    let ilaced = unsafe { made(PNG_COLOR_TYPE_GRAY, 4, 9, 7, PNG_INTERLACE_ADAM7, true) };
    for (tag, bytes) in [("plain", &full), ("interlaced", &ilaced)] {
        // no row initialisation at all in the info callback (row 747:
        // "Uninitialized row" / rows never produced)
        for init in [0u8, 1, 2] {
            for gran in [0usize, 1, 3, 64] {
                dpush(
                    &format!("{} init={} gran={}", tag, init, gran),
                    bytes,
                    gran,
                    init,
                );
            }
        }
        // png_process_data_pause from the info callback, both save modes, and
        // out-of-range save values
        for save in [0i32, 1, -1, 2, 999, i32::MIN, i32::MAX] {
            for gran in [0usize, 1] {
                same!(
                    format!("{} pause save={} gran={}", tag, save, gran),
                    |api: &'static Api| unsafe {
                        let mut a = Acc::new(0);
                        a.pause = save;
                        push(api, bytes, gran, a)
                    }
                );
            }
        }
        // png_process_data_skip from the info callback
        same!(format!("{} skip in info", tag), |api: &'static Api| unsafe {
            let mut a = Acc::new(0);
            a.skip = true;
            push(api, bytes, 0, a)
        });
        // no png_set_interlace_handling
        for init in [0u8, 1] {
            same!(
                format!("{} no interlace handling init={}", tag, init),
                |api: &'static Api| unsafe {
                    let mut a = Acc::new(init);
                    a.ihand = false;
                    push(api, bytes, 0, a)
                }
            );
        }
        // png_progressive_combine_row from the row callback
        for init in [0u8, 1] {
            same!(
                format!("{} combine init={}", tag, init),
                |api: &'static Api| unsafe {
                    let mut a = Acc::new(init);
                    a.combine = true;
                    push(api, bytes, 0, a)
                }
            );
        }
    }
    // png_process_data with no callbacks installed at all
    same!("process_data without callbacks", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        let mut d = full.clone();
        (api.png_process_data)(s.png, s.info, d.as_mut_ptr(), d.len());
        snap(api, s.png, s.info) as i64
    });
    // png_process_data driven with a zero length and a NULL buffer
    same!("process_data(NULL,0) repeatedly", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &[]);
        let mut acc = Box::new(Acc::new(0));
        (api.png_set_progressive_read_fn)(
            s.png,
            &mut *acc as *mut Acc as png_voidp,
            Some(p_info),
            Some(p_row),
            Some(p_end),
        );
        for _ in 0..8 {
            (api.png_process_data)(s.png, s.info, null_mut(), 0);
        }
        acc.total()
    });
}

#[test]
fn progressive_malformed_streams() {
    // The same malformed streams as the sequential tests, fed through the
    // progressive reader one byte at a time and in one go.
    let g = hand(6, 4, 8, 2, 0);
    let pal = hand(6, 4, 4, 3, 0);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("no IHDR".into(), asm(&g[1..])));
    cases.push((
        "IHDR length 12".into(),
        asm(&{
            let mut v = g.to_vec();
            v[0].data.truncate(12);
            v
        }),
    ));
    cases.push((
        "IHDR length 14".into(),
        asm(&{
            let mut v = g.to_vec();
            v[0].data.push(0);
            v
        }),
    ));
    cases.push((
        "ct=3 no PLTE".into(),
        asm(&pal.iter().filter(|c| !c.is(b"PLTE")).cloned().collect::<Vec<_>>()),
    ));
    cases.push((
        "PLTE after IDAT".into(),
        asm(&[pal[0].clone(), pal[2].clone(), pal[1].clone(), pal[3].clone()]),
    ));
    cases.push((
        "two IDAT runs".into(),
        asm(&{
            let raw = raw_image(6, 4, 24, 0, 0x40);
            let z = zlib_stored(&raw);
            let half = z.len() / 2;
            vec![
                g[0].clone(),
                Ch::new(b"IDAT", z[..half].to_vec()),
                Ch::new(b"gAMA", 45455u32.to_be_bytes().to_vec()),
                Ch::new(b"IDAT", z[half..].to_vec()),
                g[2].clone(),
            ]
        }),
    ));
    cases.push((
        "adler broken".into(),
        asm(&{
            let mut v = g.to_vec();
            let n = v[1].data.len();
            v[1].data[n - 1] ^= 0xff;
            v
        }),
    ));
    cases.push((
        "deflate truncated".into(),
        asm(&{
            let mut v = g.to_vec();
            let n = v[1].data.len();
            v[1].data.truncate(n - 5);
            v
        }),
    ));
    cases.push((
        "deflate extra".into(),
        asm(&{
            let mut v = g.to_vec();
            v[1].data.extend_from_slice(&[0x5a; 16]);
            v
        }),
    ));
    cases.push((
        "bad filter".into(),
        asm(&hand(6, 4, 8, 2, 7)),
    ));
    cases.push((
        "IEND missing".into(),
        asm(&g[..g.len() - 1]),
    ));
    cases.push((
        "IDAT after IEND".into(),
        asm(&{
            let mut v = g.to_vec();
            let c = v[1].clone();
            v.push(c);
            v
        }),
    ));
    for (tag, b) in &cases {
        for gran in [0usize, 1, 2, 9] {
            dpush(&format!("push {} gran={}", tag, gran), b, gran, 0);
        }
        dseq(&format!("seq {}", tag), b);
    }
}

// ===========================================================================
// 13b. png_set_quantize arguments (rows 712-715)
// ===========================================================================

#[test]
fn quantize_argument_rejections() {
    // DROPPED as C undefined behaviour (not error paths):
    //
    //  * `maximum_colors < 1` together with `num_palette > maximum_colors` and
    //    `histogram == NULL`: the reduction loop
    //    `while (num_new_palette > maximum_colors)` (pngrtran.c:693) can never
    //    terminate because `num_new_palette` bottoms out at 1, and each round
    //    does `max_d += 96` (pngrtran.c:806) so the following
    //    `for (i = 0; i <= max_d; i++) if (hash[i] != NULL)` (pngrtran.c:725)
    //    runs off the end of the 769-entry `hash` array allocated at
    //    pngrtran.c:678.  Infinite loop + out-of-bounds read, in BOTH
    //    libraries.
    //  * `maximum_colors < 0` with `histogram != NULL`: the bubble sort
    //    `for (i = num_palette - 1; i >= maximum_colors; i--)` (pngrtran.c:548)
    //    indexes `quantize_sort[-1]` and below.
    //  * `num_palette < 0` or `num_palette > PNG_MAX_PALETTE_LENGTH`:
    //    `memcpy(png_ptr->palette, palette, (unsigned)num_palette * 3)`
    //    (pngrtran.c:823-824) copies into a 256-entry allocation, and for a
    //    negative value the count becomes ~4G.  Heap overflow.
    //
    // What is left below are the in-range-but-degenerate arguments.
    let full = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 5, 3, PNG_INTERLACE_NONE, true) };
    let pairs: [(i32, i32); 15] = [
        (0, 1),
        (1, 1),
        (2, 1),
        (2, 2),
        (16, 8),
        (16, 15),
        (16, 16),
        (16, 17),
        (16, 999),
        (16, i32::MAX),
        (255, 1),
        (255, 256),
        (256, 255),
        (256, 256),
        (0, i32::MAX),
    ];
    {
        for (np, max) in pairs {
            for fq in [0i32, 1, -1, 999] {
                for withhist in [false, true] {
                    same!(
                        format!(
                            "png_set_quantize(np={},max={},fq={},hist={})",
                            np, max, fq, withhist
                        ),
                        |api: &'static Api| unsafe {
                            let s = ReadSess::new(api, &full);
                            (api.png_read_info)(s.png, s.info);
                            let mut pal: Vec<png_color> = (0..256)
                                .map(|i| png_color {
                                    red: (i * 11) as u8,
                                    green: (i * 3) as u8,
                                    blue: (i * 5) as u8,
                                })
                                .collect();
                            let hist: Vec<u16> = (0..256).map(|i| (i * 7 % 997) as u16).collect();
                            (api.png_set_quantize)(
                                s.png,
                                pal.as_mut_ptr(),
                                np,
                                max,
                                if withhist { hist.as_ptr() } else { null() },
                                fq,
                            );
                            let mut a = snap(api, s.png, s.info);
                            for c in &pal {
                                hu(
                                    &mut a,
                                    c.red as u64 * 65536 + c.green as u64 * 256 + c.blue as u64,
                                );
                            }
                            a as i64
                        }
                    );
                }
            }
        }
    }
    same!("png_set_quantize(NULL png_ptr)", |api: &'static Api| unsafe {
        let mut pal = [png_color::default(); 4];
        (api.png_set_quantize)(null_mut(), pal.as_mut_ptr(), 4, 2, null(), 1);
        0
    });
    same!("png_set_quantize(NULL palette)", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &full);
        (api.png_read_info)(s.png, s.info);
        (api.png_set_quantize)(s.png, null_mut(), 4, 2, null(), 1);
        snap(api, s.png, s.info) as i64
    });
}

// ===========================================================================
// 14. Out-of-range enum values across the FFI
// ===========================================================================

#[test]
fn out_of_range_enum_values() {
    let full = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 5, 3, PNG_INTERLACE_NONE, true) };
    let wild = [-1i32, 0, 4, 5, 6, 7, 8, 999, i32::MIN, i32::MAX];
    for &v in &wild {
        // png_set_filler / png_set_add_alpha filler location
        same!(format!("png_set_filler(loc={})", v), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            (api.png_read_info)(s.png, s.info);
            (api.png_set_filler)(s.png, 0x1234, v);
            let mut a = snap(api, s.png, s.info);
            (api.png_read_update_info)(s.png, s.info);
            hu(&mut a, snap(api, s.png, s.info));
            a as i64
        });
        same!(format!("png_set_add_alpha(loc={})", v), |api: &'static Api| unsafe {
            let s = ReadSess::new(api, &full);
            (api.png_read_info)(s.png, s.info);
            (api.png_set_add_alpha)(s.png, 0x1234, v);
            snap(api, s.png, s.info) as i64
        });
        // png_set_keep_unknown_chunks keep
        same!(
            format!("png_set_keep_unknown_chunks(keep={})", v),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_set_keep_unknown_chunks)(s.png, v, null(), 0);
                (api.png_read_info)(s.png, s.info);
                snap(api, s.png, s.info) as i64
            }
        );
        // png_set_check_for_invalid_index / png_set_benign_errors
        same!(
            format!("png_set_check_for_invalid_index({})", v),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_set_check_for_invalid_index)(s.png, v);
                (api.png_read_info)(s.png, s.info);
                snap(api, s.png, s.info) as i64
            }
        );
        same!(
            format!("png_set_benign_errors({})", v),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_set_benign_errors)(s.png, v);
                (api.png_read_info)(s.png, s.info);
                snap(api, s.png, s.info) as i64
            }
        );
        // png_set_option, read side
        for o in [
            PNG_MAXIMUM_INFLATE_WINDOW,
            PNG_SKIP_sRGB_CHECK_PROFILE,
            PNG_IGNORE_ADLER32,
        ] {
            same!(
                format!("png_set_option({},{})", o, v),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    let r = (api.png_set_option)(s.png, o, v) as i64;
                    (api.png_read_info)(s.png, s.info);
                    r * 1_000_000 + snap(api, s.png, s.info) as i64
                }
            );
        }
        // png_permit_mng_features
        same!(
            format!("png_permit_mng_features({})", v),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                let r = (api.png_permit_mng_features)(s.png, v as u32) as i64;
                (api.png_read_info)(s.png, s.info);
                r * 1_000_000 + snap(api, s.png, s.info) as i64
            }
        );
        // png_set_interlace_handling before any header
        same!(
            format!("png_set_interlace_handling early ({})", v),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                let a = (api.png_set_interlace_handling)(s.png) as i64;
                (api.png_read_info)(s.png, s.info);
                let b = (api.png_set_interlace_handling)(s.png) as i64;
                a * 16 + b
            }
        );
    }
}

// ===========================================================================
// 15. The row-size invariants checked after the user transform
//     rows 590, 591 (sequential) and 780, 781 (progressive)
// ===========================================================================

use std::cell::Cell;

thread_local! {
    /// (mode, number of calls so far)
    static UT: Cell<(i32, u32)> = const { Cell::new((0, 0)) };
}

/// A read user-transform callback that lies about the row geometry.
///
/// libpng recomputes `row_info->pixel_depth` from `bit_depth * channels` right
/// after the callback returns (pngrtran.c:5165-5170), so the callback changes
/// those two fields.
unsafe extern "C-unwind" fn ut_cb(_png: png_structp, ri: png_row_infop, _row: png_bytep) {
    if ri.is_null() {
        return;
    }
    let (mode, n) = UT.get();
    UT.set((mode, n + 1));
    match mode {
        // first row already too deep -> "sequential/progressive row overflow"
        1 => {
            (*ri).bit_depth = 16;
            (*ri).channels = 4;
        }
        // the depth CHANGES after the first row -> "internal ... row size
        // calculation error".  The new depth is smaller, so nothing can
        // overrun the destination row.
        2 => {
            if n > 0 {
                (*ri).bit_depth = 1;
                (*ri).channels = 1;
            }
        }
        // zero depth
        3 => {
            (*ri).bit_depth = 0;
            (*ri).channels = 0;
        }
        _ => {}
    }
}

#[test]
fn user_transform_row_size_errors() {
    // Non-interlaced only: with a bogus pixel depth the interlace expander
    // (png_do_read_interlace) would index the row buffer using the lie.
    let g8 = unsafe { made(PNG_COLOR_TYPE_GRAY, 8, 6, 4, PNG_INTERLACE_NONE, false) };
    let rgba8 = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 6, 4, PNG_INTERLACE_NONE, false) };
    // NOTE: `png_set_user_transform_info` is deliberately only given values
    // that do NOT inflate the pixel depth beyond the image's own.  Declaring a
    // larger user pixel depth (e.g. 16x4 for a gray-8 image) makes
    // png_combine_row copy PNG_ROWBYTES(64, width) bytes out of
    // `png_ptr->row_buf`, of which only the real 6 bytes per row were ever
    // written: the rest is uninitialised png_malloc memory (pngrutil.c:4592),
    // so the copied garbage differs between the two libraries and between runs.
    // That is C undefined behaviour (indeterminate values), not an error path.
    for (tag, b, real) in [("gray8", &g8, (8i32, 1i32)), ("rgba8", &rgba8, (8, 4))] {
        for mode in [0i32, 1, 2, 3] {
            for (dep, chan) in [(0i32, 0i32), real] {
                same!(
                    format!("user transform {} mode={} info={},{}", tag, mode, dep, chan),
                    |api: &'static Api| unsafe {
                        UT.set((mode, 0));
                        let s = ReadSess::new(api, b);
                        (api.png_read_info)(s.png, s.info);
                        (api.png_set_read_user_transform_fn)(s.png, Some(ut_cb));
                        (api.png_set_user_transform_info)(s.png, null_mut(), dep, chan);
                        (api.png_read_update_info)(s.png, s.info);
                        let h = (api.png_get_image_height)(s.png, s.info);
                        let rb = (api.png_get_rowbytes)(s.png, s.info);
                        // Over-allocate: a legitimately declared deeper user
                        // pixel depth makes libpng write more than
                        // png_get_rowbytes bytes per row.
                        let mut buf: Vec<Vec<u8>> =
                            (0..h as usize).map(|_| vec![0u8; rb * 8 + 64]).collect();
                        let mut p: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(s.png, p.as_mut_ptr());
                        (api.png_read_end)(s.png, s.end);
                        let mut a = snap(api, s.png, s.info);
                        for r in &buf {
                            h64(&mut a, r);
                        }
                        hu(&mut a, UT.get().1 as u64);
                        a as i64
                    }
                );
                // ... and the same through the progressive reader
                same!(
                    format!("push user transform {} mode={} info={},{}", tag, mode, dep, chan),
                    |api: &'static Api| unsafe {
                        UT.set((mode, 0));
                        let s = ReadSess::new(api, &[]);
                        let mut acc = Box::new(Acc::new(2));
                        (api.png_set_progressive_read_fn)(
                            s.png,
                            &mut *acc as *mut Acc as png_voidp,
                            Some(pu_info),
                            Some(p_row),
                            Some(p_end),
                        );
                        UTINFO.set((dep, chan));
                        let mut data = b.to_vec();
                        (api.png_process_data)(s.png, s.info, data.as_mut_ptr(), data.len());
                        let mut t = acc.total();
                        t ^= UT.get().1 as i64;
                        t
                    }
                );
            }
        }
    }
    // DROPPED as C undefined behaviour: png_set_read_user_transform_fn(NULL,
    // fn) -- pngrtran.c:1139 does `png_ptr->transformations |=
    // PNG_USER_TRANSFORM` with no NULL check at all, so a NULL png_ptr
    // segfaults in BOTH libraries (verified: SIGSEGV).  Only the NULL
    // *callback* is an observable no-op.
    same!("png_set_read_user_transform_fn(NULL fn)", |api: &'static Api| unsafe {
        let s = ReadSess::new(api, &g8);
        (api.png_read_info)(s.png, s.info);
        (api.png_set_read_user_transform_fn)(s.png, None);
        (api.png_read_update_info)(s.png, s.info);
        let h = (api.png_get_image_height)(s.png, s.info);
        let rb = (api.png_get_rowbytes)(s.png, s.info);
        let mut buf: Vec<Vec<u8>> = (0..h as usize).map(|_| vec![0u8; rb + 16]).collect();
        let mut p: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
        (api.png_read_image)(s.png, p.as_mut_ptr());
        (api.png_read_end)(s.png, s.end);
        let mut a = snap(api, s.png, s.info);
        for r in &buf {
            h64(&mut a, r);
        }
        a as i64
    });
}

thread_local! {
    static UTINFO: Cell<(i32, i32)> = const { Cell::new((0, 0)) };
}

/// Like `p_info` but installs the lying user transform from the info callback.
unsafe extern "C-unwind" fn pu_info(png: png_structp, info: png_infop) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Acc;
    if p.is_null() {
        return;
    }
    let a = &mut *p;
    a.infos += 1;
    a.w = (api.png_get_image_width)(png, info);
    a.pd = (api.png_get_channels)(png, info) as u32 * (api.png_get_bit_depth)(png, info) as u32;
    hu(&mut a.h, snap(api, png, info));
    let (dep, chan) = UTINFO.get();
    (api.png_set_read_user_transform_fn)(png, Some(ut_cb));
    (api.png_set_user_transform_info)(png, null_mut(), dep, chan);
    (api.png_set_interlace_handling)(png);
    (api.png_start_read_image)(png);
    hu(&mut a.h, snap(api, png, info));
}

// ===========================================================================
// 16. The simplified read API
//     rows 620, 625-640, 645-657, 687-692
// ===========================================================================

unsafe fn img_state(api: &'static Api, im: &png_image, r: c_int) -> u64 {
    let mut a = FNV;
    hu(&mut a, r as i64 as u64);
    hu(&mut a, im.warning_or_error as u64);
    hu(&mut a, im.width as u64);
    hu(&mut a, im.height as u64);
    hu(&mut a, im.format as u64);
    hu(&mut a, im.flags as u64);
    hu(&mut a, im.colormap_entries as u64);
    hu(&mut a, im.version as u64);
    hu(&mut a, im.opaque.is_null() as u64);
    let msg: Vec<u8> = im
        .message
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    h64(&mut a, &msg);
    let _ = api;
    a
}

#[test]
fn simplified_read_begin_rejections() {
    let good = unsafe { made(PNG_COLOR_TYPE_RGB, 8, 5, 3, PNG_INTERLACE_NONE, false) };
    // png_image_begin_read_from_memory
    for (tag, mem, size) in [
        ("NULL memory", false, 10usize),
        ("size 0", true, 0),
        ("size 1", true, 1),
        ("size 8", true, 8),
        ("truncated", true, 20),
        ("ok", true, good.len()),
    ] {
        for version in [PNG_IMAGE_VERSION, 0, 2, 999, 0xffff_ffff] {
            same!(
                format!("begin_read_from_memory {} version={}", tag, version),
                |api: &'static Api| unsafe {
                    let mut im = png_image {
                        version,
                        ..Default::default()
                    };
                    let r = (api.png_image_begin_read_from_memory)(
                        &mut im,
                        if mem {
                            good.as_ptr() as png_const_voidp
                        } else {
                            null()
                        },
                        size,
                    );
                    let s = img_state(api, &im, r);
                    (api.png_image_free)(&mut im);
                    s as i64
                }
            );
        }
    }
    same!("begin_read_from_memory(NULL image)", |api: &'static Api| unsafe {
        (api.png_image_begin_read_from_memory)(
            null_mut(),
            good.as_ptr() as png_const_voidp,
            good.len(),
        ) as i64
    });
    // png_image_begin_read_from_stdio(NULL FILE) and from_file
    for version in [PNG_IMAGE_VERSION, 0, 999] {
        same!(
            format!("begin_read_from_stdio(NULL) version={}", version),
            |api: &'static Api| unsafe {
                let mut im = png_image {
                    version,
                    ..Default::default()
                };
                let r = (api.png_image_begin_read_from_stdio)(&mut im, null_mut());
                let s = img_state(api, &im, r);
                (api.png_image_free)(&mut im);
                s as i64
            }
        );
        same!(
            format!("begin_read_from_file(NULL name) version={}", version),
            |api: &'static Api| unsafe {
                let mut im = png_image {
                    version,
                    ..Default::default()
                };
                let r = (api.png_image_begin_read_from_file)(&mut im, null());
                let s = img_state(api, &im, r);
                (api.png_image_free)(&mut im);
                s as i64
            }
        );
        // a name that cannot be opened: png_image_error(strerror(errno))
        for name in ["", "/nonexistent/definitely/not/here.png", "/"] {
            let cn = cs(name);
            same!(
                format!("begin_read_from_file({:?}) version={}", name, version),
                |api: &'static Api| unsafe {
                    let mut im = png_image {
                        version,
                        ..Default::default()
                    };
                    let r = (api.png_image_begin_read_from_file)(&mut im, cn.as_ptr());
                    let s = img_state(api, &im, r);
                    (api.png_image_free)(&mut im);
                    s as i64
                }
            );
        }
    }
    same!("begin_read_from_stdio(NULL image)", |api: &'static Api| unsafe {
        (api.png_image_begin_read_from_stdio)(null_mut(), null_mut()) as i64
    });
    same!("begin_read_from_file(NULL image)", |api: &'static Api| unsafe {
        (api.png_image_begin_read_from_file)(null_mut(), null()) as i64
    });
    // row 620: opaque pointer not NULL on entry (begin_read called twice)
    same!("begin_read twice", |api: &'static Api| unsafe {
        let mut im = png_image::default();
        let r1 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as png_const_voidp,
            good.len(),
        );
        let mut a = img_state(api, &im, r1);
        let r2 = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as png_const_voidp,
            good.len(),
        );
        hu(&mut a, img_state(api, &im, r2));
        (api.png_image_free)(&mut im);
        a as i64
    });
    same!("png_image_free(NULL)", |api: &'static Api| unsafe {
        (api.png_image_free)(null_mut());
        0
    });
    same!("png_image_free twice", |api: &'static Api| unsafe {
        let mut im = png_image::default();
        let r = (api.png_image_begin_read_from_memory)(
            &mut im,
            good.as_ptr() as png_const_voidp,
            good.len(),
        );
        let mut a = img_state(api, &im, r);
        (api.png_image_free)(&mut im);
        hu(&mut a, img_state(api, &im, 0));
        (api.png_image_free)(&mut im);
        hu(&mut a, img_state(api, &im, 0));
        a as i64
    });
    // png_image_error on a fresh and on a live image
    for msg in ["", "boom", &"x".repeat(200)] {
        let cm = cs(msg);
        same!(format!("png_image_error({:?})", msg), |api: &'static Api| unsafe {
            let mut im = png_image::default();
            let r = (api.png_image_error)(&mut im, cm.as_ptr());
            img_state(api, &im, r) as i64
        });
    }
}

#[test]
fn simplified_read_finish_rejections() {
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let good = unsafe { made(ct, bd, 5, 3, PNG_INTERLACE_NONE, true) };
        // (a) argument validation of png_image_finish_read
        for (tag, buf, stride, cmap) in [
            ("no buffer", false, 0i32, true),
            ("stride 0", true, 0, true),
            ("stride 1", true, 1, true),
            ("stride -1", true, -1, true),
            ("stride -1000", true, -1000, true),
            ("stride huge", true, i32::MAX, true),
            ("no colormap", true, 0, false),
            ("ok", true, 0, true),
        ] {
            for version in [PNG_IMAGE_VERSION, 0, 999] {
                // NOTE: only *defined* PNG_FORMAT_* values are used.  The
                // simplified API does not validate image->format at all
                // (pngread.c:4091-4145 only tests individual flag bits), so a
                // value with undefined bits set -- e.g. 0xffffffff, which
                // claims COLORMAP and LINEAR and ASSOCIATED_ALPHA at once --
                // drives png_image_read_colormap/_direct into states their
                // internal consistency checks were never written for and
                // corrupts the heap in BOTH libraries (verified: glibc
                // "malloc(): invalid size").  That is C undefined behaviour,
                // not an error path.
                for fmt in [
                    None,
                    Some(PNG_FORMAT_GRAY),
                    Some(PNG_FORMAT_GA),
                    Some(PNG_FORMAT_AG),
                    Some(PNG_FORMAT_RGB),
                    Some(PNG_FORMAT_BGR),
                    Some(PNG_FORMAT_RGBA),
                    Some(PNG_FORMAT_ABGR),
                    Some(PNG_FORMAT_RGB_COLORMAP),
                    Some(PNG_FORMAT_RGBA_COLORMAP),
                    Some(PNG_FORMAT_LINEAR_Y),
                    Some(PNG_FORMAT_LINEAR_RGB_ALPHA),
                ] {
                    same!(
                        format!(
                            "finish_read ct={} bd={} {} version={} fmt={:?}",
                            ct, bd, tag, version, fmt
                        ),
                        |api: &'static Api| unsafe {
                            let mut im = png_image::default();
                            let r = (api.png_image_begin_read_from_memory)(
                                &mut im,
                                good.as_ptr() as png_const_voidp,
                                good.len(),
                            );
                            let mut a = img_state(api, &im, r);
                            if let Some(f) = fmt {
                                im.format = f;
                            }
                            im.version = version;
                            let mut out = vec![0u8; 5 * 3 * 8 + 4096];
                            let mut cm = vec![0u8; 4 * 256];
                            let r2 = (api.png_image_finish_read)(
                                &mut im,
                                null(),
                                if buf {
                                    out.as_mut_ptr() as *mut c_void
                                } else {
                                    null_mut()
                                },
                                stride,
                                if cmap {
                                    cm.as_mut_ptr() as *mut c_void
                                } else {
                                    null_mut()
                                },
                            );
                            hu(&mut a, img_state(api, &im, r2));
                            h64(&mut a, &out[..64]);
                            h64(&mut a, &cm);
                            (api.png_image_free)(&mut im);
                            a as i64
                        }
                    );
                }
            }
        }
        // (b) the "too few entries" color-map errors: the application is free
        // to shrink image->colormap_entries between begin_read and finish_read
        for entries in [0u32, 1, 2, 16, 215, 216, 217, 231, 244, 255, 256, 0xffff_ffff] {
            for f in [
                PNG_FORMAT_RGB_COLORMAP,
                PNG_FORMAT_RGBA_COLORMAP,
                PNG_FORMAT_BGR_COLORMAP,
                PNG_FORMAT_ABGR_COLORMAP,
            ] {
                same!(
                    format!("finish_read colormap ct={} bd={} n={} f={:#x}", ct, bd, entries, f),
                    |api: &'static Api| unsafe {
                        let mut im = png_image::default();
                        let r = (api.png_image_begin_read_from_memory)(
                            &mut im,
                            good.as_ptr() as png_const_voidp,
                            good.len(),
                        );
                        let mut a = img_state(api, &im, r);
                        im.format = f;
                        im.colormap_entries = entries;
                        let mut out = vec![0u8; 5 * 3 * 8 + 4096];
                        let mut cm = vec![0u8; 4 * 256];
                        let r2 = (api.png_image_finish_read)(
                            &mut im,
                            null(),
                            out.as_mut_ptr() as *mut c_void,
                            0,
                            cm.as_mut_ptr() as *mut c_void,
                        );
                        hu(&mut a, img_state(api, &im, r2));
                        h64(&mut a, &cm);
                        (api.png_image_free)(&mut im);
                        a as i64
                    }
                );
            }
        }
        // (c) row_stride / size overflow checks (rows 687, 689)
        for (w, h) in [
            (0x7fff_ffffu32, 1u32),
            (0x4000_0000, 1),
            (1, 0x7fff_ffff),
            (0x1_0000, 0x1_0000),
        ] {
            same!(
                format!("finish_read overflow ct={} bd={} {}x{}", ct, bd, w, h),
                |api: &'static Api| unsafe {
                    let mut im = png_image::default();
                    let r = (api.png_image_begin_read_from_memory)(
                        &mut im,
                        good.as_ptr() as png_const_voidp,
                        good.len(),
                    );
                    let mut a = img_state(api, &im, r);
                    // Overwrite the geometry the header gave us; the checks in
                    // png_image_finish_read are done on these fields before
                    // anything is allocated.
                    im.width = w;
                    im.height = h;
                    let mut out = vec![0u8; 4096];
                    let mut cm = vec![0u8; 4 * 256];
                    let r2 = (api.png_image_finish_read)(
                        &mut im,
                        null(),
                        out.as_mut_ptr() as *mut c_void,
                        0,
                        cm.as_mut_ptr() as *mut c_void,
                    );
                    hu(&mut a, img_state(api, &im, r2));
                    (api.png_image_free)(&mut im);
                    a as i64
                }
            );
        }
        // (d) finish_read without a preceding begin_read, and twice
        same!(
            format!("finish_read without begin ct={} bd={}", ct, bd),
            |api: &'static Api| unsafe {
                let mut im = png_image::default();
                let mut out = vec![0u8; 4096];
                let r = (api.png_image_finish_read)(
                    &mut im,
                    null(),
                    out.as_mut_ptr() as *mut c_void,
                    0,
                    null_mut(),
                );
                img_state(api, &im, r) as i64
            }
        );
        same!(
            format!("finish_read twice ct={} bd={}", ct, bd),
            |api: &'static Api| unsafe {
                let mut im = png_image::default();
                let r = (api.png_image_begin_read_from_memory)(
                    &mut im,
                    good.as_ptr() as png_const_voidp,
                    good.len(),
                );
                let mut a = img_state(api, &im, r);
                let mut out = vec![0u8; 5 * 3 * 8 + 4096];
                let r1 = (api.png_image_finish_read)(
                    &mut im,
                    null(),
                    out.as_mut_ptr() as *mut c_void,
                    0,
                    null_mut(),
                );
                hu(&mut a, img_state(api, &im, r1));
                let r2 = (api.png_image_finish_read)(
                    &mut im,
                    null(),
                    out.as_mut_ptr() as *mut c_void,
                    0,
                    null_mut(),
                );
                hu(&mut a, img_state(api, &im, r2));
                (api.png_image_free)(&mut im);
                a as i64
            }
        );
        same!("finish_read(NULL image)", |api: &'static Api| unsafe {
            (api.png_image_finish_read)(null_mut(), null(), null_mut(), 0, null_mut()) as i64
        });
    }
    // (e) malformed streams through the simplified API: every truncation, a
    // bad signature, a corrupt IDAT
    let good = unsafe { made(PNG_COLOR_TYPE_RGB_ALPHA, 8, 6, 4, PNG_INTERLACE_ADAM7, true) };
    let step = (good.len() / 20).max(1);
    for n in (0..=good.len()).step_by(step) {
        same!(format!("simplified truncated at {}", n), |api: &'static Api| unsafe {
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                good.as_ptr() as png_const_voidp,
                n,
            );
            let mut a = img_state(api, &im, r);
            let mut out = vec![0u8; 6 * 4 * 8 + 4096];
            let r2 = (api.png_image_finish_read)(
                &mut im,
                null(),
                out.as_mut_ptr() as *mut c_void,
                0,
                null_mut(),
            );
            hu(&mut a, img_state(api, &im, r2));
            h64(&mut a, &out[..128]);
            (api.png_image_free)(&mut im);
            a as i64
        });
    }
    for filt in [5u8, 255] {
        let b = asm(&hand(6, 4, 8, 2, filt));
        same!(format!("simplified bad filter {}", filt), |api: &'static Api| unsafe {
            let mut im = png_image::default();
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                b.as_ptr() as png_const_voidp,
                b.len(),
            );
            let mut a = img_state(api, &im, r);
            let mut out = vec![0u8; 6 * 4 * 8 + 4096];
            let r2 = (api.png_image_finish_read)(
                &mut im,
                null(),
                out.as_mut_ptr() as *mut c_void,
                0,
                null_mut(),
            );
            hu(&mut a, img_state(api, &im, r2));
            (api.png_image_free)(&mut im);
            a as i64
        });
    }
}

// ===========================================================================
// 17. Row-transform initialisation and palette-index checking
//     rows 598, 737-745, 746, 747, 748, 749, 750
// ===========================================================================

#[test]
fn transform_init_errors() {
    let g = unsafe { made(PNG_COLOR_TYPE_GRAY, 8, 5, 3, PNG_INTERLACE_NONE, false) };
    // row 746: png_do_read_transformations with png_ptr->row_buf == NULL
    // ("NULL row buffer").  row_buf is only allocated by png_read_start_row.
    for stage in [0u8, 1, 2] {
        same!(
            format!("png_do_read_transformations stage={}", stage),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &g);
                if stage >= 1 {
                    (api.png_read_info)(s.png, s.info);
                }
                if stage >= 2 {
                    (api.png_read_update_info)(s.png, s.info);
                }
                let mut ri = png_row_info {
                    width: 5,
                    rowbytes: 5,
                    color_type: PNG_COLOR_TYPE_GRAY as u8,
                    bit_depth: 8,
                    channels: 1,
                    pixel_depth: 8,
                };
                (api.png_do_read_transformations)(s.png, &mut ri);
                let mut a = snap(api, s.png, s.info);
                hu(&mut a, ri.width as u64);
                hu(&mut a, ri.rowbytes as u64);
                hu(&mut a, ri.pixel_depth as u64);
                a as i64
            }
        );
    }
    // row 747: transforms requested but neither png_start_read_image nor
    // png_read_update_info called -- only reachable through the progressive
    // reader, whose info callback is where the app must do the row init.
    for gran in [0usize, 1] {
        same!(
            format!("push transform without row init gran={}", gran),
            |api: &'static Api| unsafe {
                let mut a = Acc::new(2);
                a.expand = true;
                push(api, &g, gran, a)
            }
        );
        for init in [0u8, 1] {
            same!(
                format!("push transform with row init={} gran={}", init, gran),
                |api: &'static Api| unsafe {
                    let mut a = Acc::new(init);
                    a.expand = true;
                    push(api, &g, gran, a)
                }
            );
        }
    }
}

#[test]
fn palette_index_and_shift_errors() {
    // A colour-type-3 stream whose pixel values run past the end of a short
    // PLTE (rows 598, 745, 750).
    for bd in [1u8, 2, 4, 8] {
        for npal in [1usize, 2, 3, 4, 255, 256] {
            if npal > (1usize << bd) {
                continue;
            }
            for ntrns in [0usize, 1, 2, 255] {
                if ntrns > npal {
                    continue;
                }
                let w = 9u32;
                let h = 3u32;
                let mut v = hand(w, h, bd, 3, 0);
                // shrink the palette
                let mut p = Vec::new();
                for i in 0..npal {
                    p.push((i * 3) as u8);
                    p.push((i * 5) as u8);
                    p.push((i * 7) as u8);
                }
                let pi = v.iter().position(|c| c.is(b"PLTE")).unwrap();
                v[pi] = Ch::new(b"PLTE", p);
                if ntrns > 0 {
                    v.insert(pi + 1, Ch::new(b"tRNS", (0..ntrns).map(|i| (i * 37) as u8).collect()));
                }
                let bytes = asm(&v);
                for check in [0i32, 1] {
                    for expand in [false, true] {
                        same!(
                            format!(
                                "palette bd={} npal={} ntrns={} check={} expand={}",
                                bd, npal, ntrns, check, expand
                            ),
                            |api: &'static Api| unsafe {
                                let s = ReadSess::new(api, &bytes);
                                (api.png_set_check_for_invalid_index)(s.png, check);
                                (api.png_read_info)(s.png, s.info);
                                if expand {
                                    (api.png_set_expand)(s.png);
                                }
                                let mut a = snap(api, s.png, s.info);
                                (api.png_read_update_info)(s.png, s.info);
                                let hh = (api.png_get_image_height)(s.png, s.info);
                                let rb = (api.png_get_rowbytes)(s.png, s.info);
                                let mut buf: Vec<Vec<u8>> =
                                    (0..hh as usize).map(|_| vec![0u8; rb + 16]).collect();
                                let mut pp: Vec<png_bytep> =
                                    buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                                (api.png_read_image)(s.png, pp.as_mut_ptr());
                                (api.png_read_end)(s.png, s.end);
                                for r in &buf {
                                    h64(&mut a, r);
                                }
                                hu(&mut a, (api.png_get_palette_max)(s.png, s.info) as i64 as u64);
                                a as i64
                            }
                        );
                    }
                }
            }
        }
    }
    // Invalid sBIT-derived shifts (rows 737-743): shift == 0 or >= bit_depth
    // for one or more channels is "an error condition which is silently
    // ignored".
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let full = unsafe { made(ct, bd, 7, 3, PNG_INTERLACE_NONE, false) };
        for b in [0u8, 1, 4, 8, 15, 16, 17, 32, 255] {
            same!(
                format!("png_set_shift ct={} bd={} sBIT={}", ct, bd, b),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    (api.png_read_info)(s.png, s.info);
                    let sb = png_color_8 {
                        red: b,
                        green: b,
                        blue: b,
                        gray: b,
                        alpha: b,
                    };
                    (api.png_set_shift)(s.png, &sb);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    let hh = (api.png_get_image_height)(s.png, s.info);
                    let rb = (api.png_get_rowbytes)(s.png, s.info);
                    let mut buf: Vec<Vec<u8>> =
                        (0..hh as usize).map(|_| vec![0u8; rb + 16]).collect();
                    let mut pp: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    (api.png_read_image)(s.png, pp.as_mut_ptr());
                    (api.png_read_end)(s.png, s.end);
                    for r in &buf {
                        h64(&mut a, r);
                    }
                    a as i64
                }
            );
            // ... and the palette-shifting variant, which needs PNG_EXPAND too
            same!(
                format!("png_set_shift+expand ct={} bd={} sBIT={}", ct, bd, b),
                |api: &'static Api| unsafe {
                    let s = ReadSess::new(api, &full);
                    (api.png_read_info)(s.png, s.info);
                    let sb = png_color_8 {
                        red: b,
                        green: b,
                        blue: b,
                        gray: b,
                        alpha: b,
                    };
                    (api.png_set_expand)(s.png);
                    (api.png_set_shift)(s.png, &sb);
                    let mut a = snap(api, s.png, s.info);
                    (api.png_read_update_info)(s.png, s.info);
                    let hh = (api.png_get_image_height)(s.png, s.info);
                    let rb = (api.png_get_rowbytes)(s.png, s.info);
                    let mut buf: Vec<Vec<u8>> =
                        (0..hh as usize).map(|_| vec![0u8; rb + 16]).collect();
                    let mut pp: Vec<png_bytep> = buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                    (api.png_read_image)(s.png, pp.as_mut_ptr());
                    (api.png_read_end)(s.png, s.end);
                    for r in &buf {
                        h64(&mut a, r);
                    }
                    a as i64
                }
            );
        }
        // png_set_shift(NULL colour) and on a NULL png_ptr
        same!(
            format!("png_set_shift(NULL sBIT) ct={} bd={}", ct, bd),
            |api: &'static Api| unsafe {
                let s = ReadSess::new(api, &full);
                (api.png_read_info)(s.png, s.info);
                (api.png_set_shift)(s.png, null());
                snap(api, s.png, s.info) as i64
            }
        );
    }
}

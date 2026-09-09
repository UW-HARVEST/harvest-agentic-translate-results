//! Worker-side support: output transcript, RNG, io callbacks, PNG builders.
#![allow(dead_code)]

use crate::api::Api;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

/* ------------------------------------------------------------------ */
/* transcript                                                          */
/* ------------------------------------------------------------------ */

/// Everything the worker observes goes here, then to stdout verbatim. The
/// driver compares the C worker's stdout with the Rust worker's stdout
/// byte-for-byte, so any divergence in values, messages, ordering or
/// termination shows up as a diff.
///
/// Lines are written through immediately rather than accumulated. That matters
/// because some inputs make libpng die on a signal (a zero-width image reaches
/// an integer division by zero and takes the hardware divide fault); with a
/// buffered transcript everything observed before the fault would be lost and
/// only the exit status would be compared. Writing through means the two
/// transcripts are compared right up to the crash point.
static mut OUT: Option<std::io::BufWriter<std::io::Stdout>> = None;

fn sink() -> &'static mut std::io::BufWriter<std::io::Stdout> {
    unsafe {
        let p = &raw mut OUT;
        if (*p).is_none() {
            *p = Some(std::io::BufWriter::with_capacity(1 << 16, std::io::stdout()));
        }
        (*p).as_mut().unwrap()
    }
}

pub fn emit(s: &str) {
    use std::io::Write;
    let w = sink();
    let _ = w.write_all(s.as_bytes());
    let _ = w.write_all(b"\n");
    let _ = w.flush();
}

pub fn emitf(args: std::fmt::Arguments) {
    emit(&std::fmt::format(args));
}

#[macro_export]
macro_rules! p {
    ($($t:tt)*) => { $crate::support::emitf(format_args!($($t)*)) }
}

/// Emit a byte slice as a length plus a stable digest plus a bounded hex
/// prefix/suffix. The digest makes a difference anywhere in a large buffer
/// visible while keeping the transcript small.
pub fn emit_bytes(tag: &str, b: &[u8]) {
    let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
    let mut h2: u64 = 0x9e37_79b9_7f4a_7c15;
    for (i, x) in b.iter().enumerate() {
        h1 ^= *x as u64;
        h1 = h1.wrapping_mul(0x100_0000_01b3);
        h2 = h2
            .wrapping_add((*x as u64).wrapping_mul(i as u64 + 1))
            .rotate_left(7);
    }
    let head: String = b.iter().take(48).map(|x| format!("{:02x}", x)).collect();
    let tail: String = b
        .iter()
        .skip(b.len().saturating_sub(16))
        .map(|x| format!("{:02x}", x))
        .collect();
    emitf(format_args!(
        "{}: len={} h={:016x}{:016x} head={} tail={}",
        tag,
        b.len(),
        h1,
        h2,
        head,
        tail
    ));
}

/// Flush the transcript and terminate with `code`. Used from the error
/// callback, which must not return to libpng.
pub fn finish(code: i32) -> ! {
    use std::io::Write;
    let _ = sink().flush();
    std::process::exit(code);
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (xoshiro256**)                                    */
/* ------------------------------------------------------------------ */

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // SplitMix64 expansion so nearby seeds give unrelated streams.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }
    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.u32() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    /// Byte stream biased toward runs and small values so the adaptive PNG
    /// filters and deflate actually have structure to exploit.
    pub fn image_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        let mode = self.below(4);
        let mut cur = self.u8();
        while v.len() < n {
            match mode {
                0 => v.push(self.u8()),
                1 => {
                    let run = self.range(1, 9) as usize;
                    for _ in 0..run {
                        if v.len() < n {
                            v.push(cur)
                        }
                    }
                    cur = cur.wrapping_add(self.u8() & 7);
                }
                2 => {
                    cur = cur.wrapping_add(1);
                    v.push(cur);
                }
                _ => v.push(self.u8() & 0x0f),
            }
        }
        v.truncate(n);
        v
    }
}

/* ------------------------------------------------------------------ */
/* error / warning callbacks                                           */
/* ------------------------------------------------------------------ */

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let mut n = 0usize;
    while *p.add(n) != 0 && n < 4096 {
        n += 1;
    }
    let b = std::slice::from_raw_parts(p as *const u8, n);
    String::from_utf8_lossy(b).into_owned()
}

/// Exit status used when libpng reports a fatal error. Distinct from any
/// status the harness itself produces, so the driver can tell "both errored
/// with the same message" from "one crashed".
pub const EXIT_PNG_ERROR: i32 = 90;

pub unsafe extern "C" fn err_fn(_p: png_structp, msg: *const c_char) {
    emitf(format_args!("ERROR {}", cstr(msg)));
    finish(EXIT_PNG_ERROR);
}

pub unsafe extern "C" fn warn_fn(_p: png_structp, msg: *const c_char) {
    emitf(format_args!("WARN  {}", cstr(msg)));
}

/* ------------------------------------------------------------------ */
/* io callbacks over globals (the worker runs exactly one case)         */
/* ------------------------------------------------------------------ */

static mut WBUF: Option<Vec<u8>> = None;
static mut RBUF: Option<Vec<u8>> = None;
static mut RPOS: usize = 0;

pub fn wbuf() -> &'static mut Vec<u8> {
    unsafe {
        let p = &raw mut WBUF;
        if (*p).is_none() {
            *p = Some(Vec::new());
        }
        (*p).as_mut().unwrap()
    }
}

pub fn set_rbuf(v: Vec<u8>) {
    unsafe {
        let p = &raw mut RBUF;
        *p = Some(v);
        let q = &raw mut RPOS;
        *q = 0;
    }
}

pub fn reset_wbuf() {
    wbuf().clear();
}

pub unsafe extern "C" fn write_fn(_p: png_structp, data: *mut u8, len: usize) {
    if len > 0 && !data.is_null() {
        wbuf().extend_from_slice(std::slice::from_raw_parts(data, len));
    }
}

pub unsafe extern "C" fn flush_fn(_p: png_structp) {
    emit("FLUSH");
}

pub unsafe extern "C" fn read_fn(p: png_structp, data: *mut u8, len: usize) {
    let rb = {
        let q = &raw mut RBUF;
        (*q).as_ref().map(|v| v.as_slice()).unwrap_or(&[])
    };
    let pos = {
        let q = &raw const RPOS;
        *q
    };
    if pos + len > rb.len() {
        // Same policy in both libraries: report EOF as a png_error, which is
        // what libpng's own stdio reader does.
        emit("ERROR harness: read past end of stream");
        finish(EXIT_PNG_ERROR);
    }
    if len > 0 {
        std::ptr::copy_nonoverlapping(rb.as_ptr().add(pos), data, len);
    }
    let q = &raw mut RPOS;
    *q = pos + len;
    let _ = p;
}

/* ------------------------------------------------------------------ */
/* status / transform callbacks                                        */
/* ------------------------------------------------------------------ */

pub unsafe extern "C" fn read_status_fn(_p: png_structp, row: u32, pass: c_int) {
    emitf(format_args!("rstatus row={} pass={}", row, pass));
}

pub unsafe extern "C" fn write_status_fn(_p: png_structp, row: u32, pass: c_int) {
    emitf(format_args!("wstatus row={} pass={}", row, pass));
}

/// A deterministic user transform that mutates the row in place, so the
/// hook point, row_info contents and buffer contents are all compared.
pub unsafe extern "C" fn user_transform_fn(
    p: png_structp,
    ri: *mut png_row_info,
    row: *mut u8,
) {
    if ri.is_null() || row.is_null() {
        emit("utf: null");
        return;
    }
    let r = *ri;
    emitf(format_args!(
        "utf w={} rb={} ct={} bd={} ch={} pd={}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    ));
    let s = std::slice::from_raw_parts_mut(row, r.rowbytes);
    for (i, b) in s.iter_mut().enumerate() {
        *b = b.wrapping_add((i as u8) ^ 0x5a);
    }
    let _ = p;
}

pub unsafe extern "C" fn user_chunk_ok(_p: png_structp, c: *mut png_unknown_chunk) -> c_int {
    if !c.is_null() {
        let u = &*c;
        emitf(format_args!(
            "uchunk name={:?} size={}",
            &u.name[..4],
            u.size
        ));
    }
    1
}

pub unsafe extern "C" fn user_chunk_unhandled(_p: png_structp, c: *mut png_unknown_chunk) -> c_int {
    if !c.is_null() {
        let u = &*c;
        emitf(format_args!("uchunk0 name={:?} size={}", &u.name[..4], u.size));
    }
    0
}

pub unsafe extern "C" fn user_chunk_err(_p: png_structp, _c: *mut png_unknown_chunk) -> c_int {
    -1
}

/* ------------------------------------------------------------------ */
/* progressive read callbacks                                          */
/* ------------------------------------------------------------------ */

pub unsafe extern "C" fn prog_info_fn(p: png_structp, i: png_infop) {
    emit("prog: info");
    let _ = (p, i);
}

pub unsafe extern "C" fn prog_row_fn(_p: png_structp, row: *mut u8, num: u32, pass: c_int) {
    if row.is_null() {
        emitf(format_args!("prog: row {} pass {} <null>", num, pass));
    } else {
        emitf(format_args!("prog: row {} pass {}", num, pass));
    }
}

pub unsafe extern "C" fn prog_end_fn(_p: png_structp, _i: png_infop) {
    emit("prog: end");
}

/* ------------------------------------------------------------------ */
/* struct construction helpers                                         */
/* ------------------------------------------------------------------ */

pub struct Reader<'a> {
    pub api: &'a Api,
    pub png: png_structp,
    pub info: png_infop,
}

impl<'a> Reader<'a> {
    /// Create a read struct wired to the transcript callbacks and to `data`
    /// as the input stream.
    pub fn new(api: &'a Api, data: Vec<u8>) -> Reader<'a> {
        unsafe {
            set_rbuf(data);
            let png = (api.png_create_read_struct)(
                PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                Some(err_fn),
                Some(warn_fn),
            );
            if png.is_null() {
                emit("create_read_struct: NULL");
                finish(EXIT_PNG_ERROR);
            }
            let info = (api.png_create_info_struct)(png);
            if info.is_null() {
                emit("create_info_struct: NULL");
                finish(EXIT_PNG_ERROR);
            }
            (api.png_set_read_fn)(png, std::ptr::null_mut(), Some(read_fn));
            Reader { api, png, info }
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            let mut p = self.png;
            let mut i = self.info;
            (self.api.png_destroy_read_struct)(&mut p, &mut i, std::ptr::null_mut());
            self.png = std::ptr::null_mut();
            self.info = std::ptr::null_mut();
        }
    }
}

pub struct Writer<'a> {
    pub api: &'a Api,
    pub png: png_structp,
    pub info: png_infop,
}

impl<'a> Writer<'a> {
    pub fn new(api: &'a Api) -> Writer<'a> {
        unsafe {
            reset_wbuf();
            let png = (api.png_create_write_struct)(
                PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                Some(err_fn),
                Some(warn_fn),
            );
            if png.is_null() {
                emit("create_write_struct: NULL");
                finish(EXIT_PNG_ERROR);
            }
            let info = (api.png_create_info_struct)(png);
            if info.is_null() {
                emit("create_info_struct: NULL");
                finish(EXIT_PNG_ERROR);
            }
            (api.png_set_write_fn)(
                png,
                std::ptr::null_mut(),
                Some(write_fn),
                Some(flush_fn),
            );
            Writer { api, png, info }
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            let mut p = self.png;
            let mut i = self.info;
            (self.api.png_destroy_write_struct)(&mut p, &mut i);
            self.png = std::ptr::null_mut();
            self.info = std::ptr::null_mut();
        }
    }
}

/* ------------------------------------------------------------------ */
/* raw PNG stream construction (for error-path tests)                  */
/* ------------------------------------------------------------------ */

pub fn crc32(data: &[u8]) -> u32 {
    static mut TAB: Option<[u32; 256]> = None;
    let tab = unsafe {
        let p = &raw mut TAB;
        if (*p).is_none() {
            let mut t = [0u32; 256];
            for n in 0..256u32 {
                let mut c = n;
                for _ in 0..8 {
                    c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
                }
                t[n as usize] = c;
            }
            *p = Some(t);
        }
        (*p).as_ref().unwrap()
    };
    let mut c = 0xffff_ffffu32;
    for b in data {
        c = tab[((c ^ *b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

/// A PNG chunk with a correct CRC.
pub fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len() + 12);
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut crcbuf = name.to_vec();
    crcbuf.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crcbuf).to_be_bytes());
    v
}

/// A PNG chunk with a deliberately wrong CRC.
pub fn chunk_badcrc(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = chunk(name, data);
    let n = v.len();
    v[n - 1] ^= 0xff;
    v
}

/// A chunk whose declared length differs from the data actually supplied.
pub fn chunk_raw_len(name: &[u8; 4], declared: u32, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&declared.to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut crcbuf = name.to_vec();
    crcbuf.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crcbuf).to_be_bytes());
    v
}

pub fn ihdr_data(
    w: u32,
    h: u32,
    depth: u8,
    color: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(13);
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.push(depth);
    d.push(color);
    d.push(compression);
    d.push(filter);
    d.push(interlace);
    d
}

/// Minimal zlib stream (stored deflate blocks) so tests can build IDAT data
/// without depending on a compressor.
pub fn zlib_store(raw: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78, 0x01];
    if raw.is_empty() {
        v.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut off = 0usize;
        while off < raw.len() {
            let n = std::cmp::min(65535, raw.len() - off);
            let last = if off + n == raw.len() { 1u8 } else { 0u8 };
            v.push(last);
            v.extend_from_slice(&(n as u16).to_le_bytes());
            v.extend_from_slice(&(!(n as u16)).to_le_bytes());
            v.extend_from_slice(&raw[off..off + n]);
            off += n;
        }
    }
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for x in raw {
        a = (a + *x as u32) % 65521;
        b = (b + a) % 65521;
    }
    v.extend_from_slice(&(((b << 16) | a) as u32).to_be_bytes());
    v
}

pub fn channels_of(color: u8) -> usize {
    match color {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

pub fn rowbytes_of(w: u32, depth: u8, color: u8) -> usize {
    let bits = channels_of(color) * depth as usize * w as usize;
    (bits + 7) / 8
}

/// Uncompressed IDAT payload (filter byte 0 per row) for a non-interlaced
/// image of the given shape, filled from `rng`.
pub fn raw_rows(rng: &mut Rng, w: u32, h: u32, depth: u8, color: u8) -> Vec<u8> {
    let rb = rowbytes_of(w, depth, color);
    let mut v = Vec::with_capacity(h as usize * (rb + 1));
    for _ in 0..h {
        v.push(0u8);
        v.extend_from_slice(&rng.image_bytes(rb));
    }
    v
}

/// Assemble a complete, valid, non-interlaced PNG byte stream.
pub fn build_png(
    rng: &mut Rng,
    w: u32,
    h: u32,
    depth: u8,
    color: u8,
    extra_before_idat: &[Vec<u8>],
    extra_after_idat: &[Vec<u8>],
) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&PNG_SIG);
    v.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, depth, color, 0, 0, 0)));
    if color == 3 {
        let n = 1usize << depth.min(8);
        let mut pal = Vec::with_capacity(n * 3);
        for _ in 0..n {
            pal.push(rng.u8());
            pal.push(rng.u8());
            pal.push(rng.u8());
        }
        v.extend_from_slice(&chunk(b"PLTE", &pal));
    }
    for c in extra_before_idat {
        v.extend_from_slice(c);
    }
    let raw = raw_rows(rng, w, h, depth, color);
    v.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    for c in extra_after_idat {
        v.extend_from_slice(c);
    }
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}

/// The 15 legal `color_type` x `bit_depth` combinations (axis A1 of CONFIGS.md).
pub const A1: [(u8, u8); 15] = [
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (2, 8),
    (2, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

/// Image shapes that stress sub-byte row ends and small Adam7 passes.
pub const SHAPES: [(u32, u32); 10] = [
    (1, 1),
    (1, 7),
    (7, 1),
    (2, 3),
    (3, 5),
    (5, 5),
    (8, 8),
    (9, 4),
    (17, 3),
    (33, 6),
];

pub fn cptr(s: &'static [u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

/// Nul-terminated owned C string.
pub fn cs(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

pub fn null<T>() -> *mut T {
    std::ptr::null_mut()
}

/* ------------------------------------------------------------------ */
/* variants                                                            */
/* ------------------------------------------------------------------ */

/// A worker process runs exactly one *variant* of one case. This matters
/// because a `png_error` never returns to its caller: once one has fired the
/// process is finished. A case that has to probe several independent fatal
/// conditions declares `variants: n` and switches on `variant()`, so each
/// condition gets its own pair of processes.
static mut VARIANT: u32 = 0;

pub fn set_variant(v: u32) {
    unsafe {
        let p = &raw mut VARIANT;
        *p = v;
    }
}

pub fn variant() -> u32 {
    unsafe {
        let p = &raw const VARIANT;
        *p
    }
}

/// Pick element `variant() % len` of a slice — the idiom for "one fatal
/// condition per process".
pub fn pick<T: Copy>(items: &[T]) -> T {
    items[(variant() as usize) % items.len()]
}

pub fn vnull() -> *mut c_void {
    std::ptr::null_mut()
}

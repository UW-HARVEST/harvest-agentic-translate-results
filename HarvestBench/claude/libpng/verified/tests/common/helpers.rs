//! Reusable "drive a whole libpng operation" helpers shared by the test files.
//!
//! Everything here goes through `Api` (i.e. through the dlopen'd exports) and
//! records an event trace, so a scenario built out of these helpers can be run
//! against both libraries and compared.
#![allow(dead_code)]

use super::*;
use core::ffi::{c_int, c_void};

/* ------------------------------------------------------------------ */
/* test images                                                         */
/* ------------------------------------------------------------------ */

#[derive(Clone, Debug)]
pub struct Img {
    pub w: u32,
    pub h: u32,
    pub color_type: c_int,
    pub bit_depth: c_int,
    pub interlace: c_int,
    /// Only used when `color_type == PNG_COLOR_TYPE_PALETTE`.
    pub palette: Vec<png_color>,
    pub rows: Vec<Vec<u8>>,
}

impl Img {
    pub fn rowbytes(&self) -> usize {
        png_rowbytes(
            (self.bit_depth as usize) * channels_of(self.color_type),
            self.w as usize,
        )
    }

    /// A random image of the given shape.  Palette indices are kept in range so
    /// that the default configuration produces no warnings; the
    /// `check_for_invalid_index` tests deliberately break that.
    pub fn random(rng: &mut Rng, w: u32, h: u32, color_type: c_int, bit_depth: c_int) -> Img {
        let mut img = Img {
            w,
            h,
            color_type,
            bit_depth,
            interlace: PNG_INTERLACE_NONE,
            palette: Vec::new(),
            rows: Vec::new(),
        };
        if color_type == PNG_COLOR_TYPE_PALETTE {
            let n = 1usize << bit_depth.min(8);
            img.palette = (0..n)
                .map(|_| png_color {
                    red: rng.u8(),
                    green: rng.u8(),
                    blue: rng.u8(),
                })
                .collect();
        }
        let rb = img.rowbytes();
        img.rows = (0..h).map(|_| rng.bytes(rb)).collect();
        img
    }
}

/* ------------------------------------------------------------------ */
/* writing                                                             */
/* ------------------------------------------------------------------ */

#[derive(Clone, Default, Debug)]
pub struct WriteOpts {
    pub filter_mask: Option<c_int>,
    pub level: Option<c_int>,
    pub strategy: Option<c_int>,
    pub mem_level: Option<c_int>,
    pub window_bits: Option<c_int>,
    pub method: Option<c_int>,
    pub buffer_size: Option<usize>,
    /// Write the rows with `png_write_image` instead of row by row.
    pub bulk: bool,
    /// Call `png_write_rows` with this many rows at a time (0 = one by one).
    pub rows_at_a_time: usize,
    pub flush_every: Option<c_int>,
    pub status_fn: bool,
}

/// The result of driving a complete write.
pub struct Written {
    pub bytes: Vec<u8>,
    pub guard: Guard,
}

/// Drive a complete libpng write.  `extra` runs after `png_set_IHDR` (and after
/// `png_set_PLTE` for palette images) but before `png_write_info`, which is
/// where an application installs chunks and write transforms.
pub unsafe fn write_image(
    api: &Api,
    img: &Img,
    opts: &WriteOpts,
    extra: &mut dyn FnMut(&Api, *mut PngStruct, *mut PngInfo),
) -> Written {
    let (png, info) = new_write(api);
    (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    if opts.status_fn {
        (api.png_set_write_status_fn)(png, Some(write_status_cb));
    }
    if let Some(n) = opts.buffer_size {
        (api.png_set_compression_buffer_size)(png, n);
    }
    if let Some(v) = opts.level {
        (api.png_set_compression_level)(png, v);
    }
    if let Some(v) = opts.strategy {
        (api.png_set_compression_strategy)(png, v);
    }
    if let Some(v) = opts.mem_level {
        (api.png_set_compression_mem_level)(png, v);
    }
    if let Some(v) = opts.window_bits {
        (api.png_set_compression_window_bits)(png, v);
    }
    if let Some(v) = opts.method {
        (api.png_set_compression_method)(png, v);
    }
    if let Some(v) = opts.flush_every {
        (api.png_set_flush)(png, v);
    }

    let guard = guarded(api, png, &mut || {
        (api.png_set_IHDR)(
            png,
            info,
            img.w,
            img.h,
            img.bit_depth,
            img.color_type,
            img.interlace,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if img.color_type == PNG_COLOR_TYPE_PALETTE && !img.palette.is_empty() {
            (api.png_set_PLTE)(
                png,
                info,
                img.palette.as_ptr(),
                img.palette.len() as c_int,
            );
        }
        extra(api, png, info);
        if let Some(m) = opts.filter_mask {
            (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, m);
        }
        (api.png_write_info)(png, info);
        let passes = if img.interlace == PNG_INTERLACE_ADAM7 {
            (api.png_set_interlace_handling)(png)
        } else {
            1
        };
        log(format!("write passes={}", passes));
        if opts.bulk {
            let mut ptrs: Vec<*mut u8> = img.rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
            (api.png_write_image)(png, ptrs.as_mut_ptr());
        } else if opts.rows_at_a_time > 0 {
            for _ in 0..passes {
                let mut i = 0;
                while i < img.rows.len() {
                    let n = opts.rows_at_a_time.min(img.rows.len() - i);
                    let mut ptrs: Vec<*mut u8> = img.rows[i..i + n]
                        .iter()
                        .map(|r| r.as_ptr() as *mut u8)
                        .collect();
                    (api.png_write_rows)(png, ptrs.as_mut_ptr(), n as u32);
                    i += n;
                }
            }
        } else {
            for _ in 0..passes {
                for r in &img.rows {
                    (api.png_write_row)(png, r.as_ptr() as *mut u8);
                }
            }
        }
        (api.png_write_end)(png, info);
    });
    log(format!("write guard={:?} flushes={}", guard, tls().flushes));
    destroy_write(api, png, info);
    Written {
        bytes: std::mem::take(&mut tls().output),
        guard,
    }
}

/// A plain write with no extra chunks or transforms.
pub unsafe fn write_plain(api: &Api, img: &Img, opts: &WriteOpts) -> Written {
    write_image(api, img, opts, &mut |_, _, _| {})
}

/* ------------------------------------------------------------------ */
/* reading                                                             */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowMode {
    /// `png_read_row` once per row per pass.
    Row,
    /// `png_read_row` with a non-NULL `display_row`.
    RowDisplay,
    /// `png_read_rows` with `n` rows at a time.
    Rows(usize),
    /// `png_read_image`.
    Image,
    /// Do not read any rows at all.
    None,
}

#[derive(Clone, Debug)]
pub struct ReadOpts {
    pub rows: RowMode,
    pub update_info: bool,
    pub status_fn: bool,
    pub read_end: bool,
    /// `png_read_end(png, NULL)` instead of `png_read_end(png, info)`.
    pub end_null_info: bool,
}

impl Default for ReadOpts {
    fn default() -> Self {
        ReadOpts {
            rows: RowMode::Row,
            update_info: true,
            status_fn: false,
            read_end: true,
            end_null_info: false,
        }
    }
}

pub struct ReadResult {
    pub guard: Guard,
    pub rows: Vec<Vec<u8>>,
}

/// Log everything `png_get_*` reports about the current info state.
pub unsafe fn log_info(api: &Api, png: *mut PngStruct, info: *mut PngInfo, tag: &str) {
    let mut w = 0u32;
    let mut h = 0u32;
    let mut d = 0;
    let mut ct = 0;
    let mut il = 0;
    let mut comp = 0;
    let mut filt = 0;
    let r = (api.png_get_IHDR)(
        png, info, &mut w, &mut h, &mut d, &mut ct, &mut il, &mut comp, &mut filt,
    );
    log(format!(
        "{}: IHDR r={} {}x{} depth={} color={} il={} comp={} filter={}",
        tag, r, w, h, d, ct, il, comp, filt
    ));
    log(format!(
        "{}: rowbytes={} channels={} valid=0x{:x} palette_max={}",
        tag,
        (api.png_get_rowbytes)(png, info),
        (api.png_get_channels)(png, info),
        (api.png_get_valid)(png, info, 0xffffffff),
        (api.png_get_palette_max)(png, info)
    ));
}

/// Drive a complete libpng read.  `setup` runs after `png_read_info`, which is
/// where an application installs read transforms and inspects the info struct.
pub unsafe fn read_image(
    api: &Api,
    data: &[u8],
    opts: &ReadOpts,
    setup: &mut dyn FnMut(&Api, *mut PngStruct, *mut PngInfo),
) -> ReadResult {
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (png, info) = new_read(api);
    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
    if opts.status_fn {
        (api.png_set_read_status_fn)(png, Some(read_status_cb));
    }
    let mut out: Vec<Vec<u8>> = Vec::new();
    let guard = guarded(api, png, &mut || {
        (api.png_read_info)(png, info);
        log_info(api, png, info, "after read_info");
        setup(api, png, info);
        if opts.update_info {
            (api.png_read_update_info)(png, info);
            log_info(api, png, info, "after update_info");
        }
        let h = (api.png_get_image_height)(png, info) as usize;
        let rb = (api.png_get_rowbytes)(png, info);
        let passes = if (api.png_get_interlace_type)(png, info) as c_int == PNG_INTERLACE_ADAM7 {
            7
        } else {
            1
        };
        out = vec![vec![0u8; rb]; h];
        match opts.rows {
            RowMode::None => {}
            RowMode::Row | RowMode::RowDisplay => {
                let mut disp = vec![vec![0u8; rb]; h];
                for _ in 0..passes {
                    for y in 0..h {
                        let d = if opts.rows == RowMode::RowDisplay {
                            disp[y].as_mut_ptr()
                        } else {
                            core::ptr::null_mut()
                        };
                        (api.png_read_row)(png, out[y].as_mut_ptr(), d);
                    }
                }
                if opts.rows == RowMode::RowDisplay {
                    for (y, d) in disp.iter().enumerate() {
                        log(format!("display row {}: {:02x?}", y, d));
                    }
                }
            }
            RowMode::Rows(n) => {
                let n = n.max(1);
                for _ in 0..passes {
                    let mut y = 0;
                    while y < h {
                        let k = n.min(h - y);
                        let mut ptrs: Vec<*mut u8> =
                            out[y..y + k].iter().map(|r| r.as_ptr() as *mut u8).collect();
                        (api.png_read_rows)(
                            png,
                            ptrs.as_mut_ptr(),
                            core::ptr::null_mut(),
                            k as u32,
                        );
                        y += k;
                    }
                }
            }
            RowMode::Image => {
                let mut ptrs: Vec<*mut u8> = out.iter().map(|r| r.as_ptr() as *mut u8).collect();
                (api.png_read_image)(png, ptrs.as_mut_ptr());
            }
        }
        if opts.read_end {
            if opts.end_null_info {
                (api.png_read_end)(png, core::ptr::null_mut());
            } else {
                (api.png_read_end)(png, info);
                log_info(api, png, info, "after read_end");
            }
        }
    });
    log(format!("read guard={:?}", guard));
    for (y, r) in out.iter().enumerate() {
        log(format!("row {}: {:02x?}", y, r));
    }
    destroy_read(api, png, info);
    ReadResult { guard, rows: out }
}

pub unsafe fn read_plain(api: &Api, data: &[u8], opts: &ReadOpts) -> ReadResult {
    read_image(api, data, opts, &mut |_, _, _| {})
}

/* ------------------------------------------------------------------ */
/* building a PNG datastream by hand                                   */
/* ------------------------------------------------------------------ */

pub const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// CRC-32 as used by PNG (zlib's polynomial).
pub fn crc32(data: &[u8]) -> u32 {
    static mut TABLE: [u32; 256] = [0; 256];
    static ONCE: std::sync::Once = std::sync::Once::new();
    unsafe {
        ONCE.call_once(|| {
            for n in 0..256u32 {
                let mut c = n;
                for _ in 0..8 {
                    c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
                }
                TABLE[n as usize] = c;
            }
        });
        let mut c = 0xffffffffu32;
        for &b in data {
            c = TABLE[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
        }
        c ^ 0xffffffff
    }
}

/// A raw PNG chunk: length, type, data, CRC.
pub fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len() + 12);
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let mut crc_input = name.to_vec();
    crc_input.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    v
}

/// A chunk with a deliberately wrong CRC.
pub fn chunk_bad_crc(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = chunk(name, data);
    let n = v.len();
    v[n - 1] ^= 0xff;
    v
}

/// Split a PNG datastream into `(header, chunks..)` so tests can insert chunks.
pub fn split_chunks(png: &[u8]) -> Vec<(String, std::ops::Range<usize>)> {
    let mut out = Vec::new();
    let mut i = 8;
    while i + 8 <= png.len() {
        let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let name = String::from_utf8_lossy(&png[i + 4..i + 8]).into_owned();
        let end = i + 12 + len;
        if end > png.len() {
            break;
        }
        out.push((name, i..end));
        i = end;
    }
    out
}

/// Insert `extra` immediately before the first chunk named `before`.
pub fn insert_before(png: &[u8], before: &str, extra: &[u8]) -> Vec<u8> {
    let chunks = split_chunks(png);
    let at = chunks
        .iter()
        .find(|(n, _)| n == before)
        .map(|(_, r)| r.start)
        .unwrap_or(png.len());
    let mut v = png[..at].to_vec();
    v.extend_from_slice(extra);
    v.extend_from_slice(&png[at..]);
    v
}

pub fn insert_after_last(png: &[u8], after: &str, extra: &[u8]) -> Vec<u8> {
    let chunks = split_chunks(png);
    let at = chunks
        .iter()
        .filter(|(n, _)| n == after)
        .next_back()
        .map(|(_, r)| r.end)
        .unwrap_or(png.len());
    let mut v = png[..at].to_vec();
    v.extend_from_slice(extra);
    v.extend_from_slice(&png[at..]);
    v
}

/// A minimal valid PNG built entirely by hand (so the *reader* is the only
/// thing under test).  1x1, 8-bit gray, one IDAT.
pub fn handmade_gray1x1() -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    // zlib stream for a single row: filter 0 + one byte, stored (level 0).
    let raw = [0u8, 0x80u8];
    let mut z = vec![0x78, 0x01, 0x01, 0x02, 0x00, 0xfd, 0xff];
    z.extend_from_slice(&raw);
    let mut adler: (u32, u32) = (1, 0);
    for &b in &raw {
        adler.0 = (adler.0 + b as u32) % 65521;
        adler.1 = (adler.1 + adler.0) % 65521;
    }
    z.extend_from_slice(&(((adler.1) << 16) | adler.0).to_be_bytes());
    let mut v = SIG.to_vec();
    v.extend_from_slice(&chunk(b"IHDR", &ihdr));
    v.extend_from_slice(&chunk(b"IDAT", &z));
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}

/* ------------------------------------------------------------------ */
/* misc callbacks used by several test files                           */
/* ------------------------------------------------------------------ */

pub unsafe extern "C" fn user_transform_cb(
    _png: *mut PngStruct,
    row_info: *mut png_row_info,
    row: *mut u8,
) {
    let ri = *row_info;
    log(format!(
        "user_transform w={} rowbytes={} ct={} bd={} ch={} pd={}",
        ri.width, ri.rowbytes, ri.color_type, ri.bit_depth, ri.channels, ri.pixel_depth
    ));
    // A deterministic, in-place mangle so the effect is visible in the output.
    for i in 0..ri.rowbytes {
        *row.add(i) = (*row.add(i)).rotate_left(3) ^ (i as u8);
    }
}

pub unsafe extern "C" fn malloc_cb(png: *mut PngStruct, size: usize) -> *mut c_void {
    let t = tls();
    t.alloc_serial += 1;
    let serial = t.alloc_serial;
    let p = libc_malloc(size.max(1));
    t.allocs.push((serial as usize, size));
    log(format!("malloc #{} size={} ok={}", serial, size, !p.is_null()));
    let _ = png;
    p
}

pub unsafe extern "C" fn free_cb(_png: *mut PngStruct, p: *mut c_void) {
    tls().counter += 1;
    libc_free(p);
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

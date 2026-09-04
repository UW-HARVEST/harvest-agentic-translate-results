//! PNG container builder.
//!
//! `cp_chunk`/`cp_find` never validate CRCs, but real CRCs are cheap and keep
//! the fixtures honest. Everything else (chunk order, IDAT splitting, ancillary
//! chunks, IHDR field values, raw scanline filtering) is under the caller's
//! control because those are the axes `CONFIGS.md` enumerates.

#![allow(dead_code)]

use super::deflate;
use super::Rng;

pub const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for n in 0..256u32 {
        let mut c = n;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        t[n as usize] = c;
    }
    t
}

pub fn crc32(data: &[u8]) -> u32 {
    let t = crc_table();
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len() + 12);
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(ty);
    v.extend_from_slice(data);
    let mut crc_in = Vec::with_capacity(data.len() + 4);
    crc_in.extend_from_slice(ty);
    crc_in.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    v
}

/// A chunk with a *declared* length that may disagree with the payload, for the
/// error-path tests.
pub fn chunk_raw_len(ty: &[u8; 4], declared_len: u32, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len() + 12);
    v.extend_from_slice(&declared_len.to_be_bytes());
    v.extend_from_slice(ty);
    v.extend_from_slice(data);
    v.extend_from_slice(&[0, 0, 0, 0]);
    v
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorType {
    Grey = 0,
    Rgb = 2,
    Indexed = 3,
    GreyAlpha = 4,
    Rgba = 6,
}

impl ColorType {
    pub fn bpp(self) -> usize {
        match self {
            ColorType::Grey => 1,
            ColorType::Rgb => 3,
            ColorType::Indexed => 1,
            ColorType::GreyAlpha => 2,
            ColorType::Rgba => 4,
        }
    }
    pub const ALL: [ColorType; 5] = [
        ColorType::Grey,
        ColorType::Rgb,
        ColorType::Indexed,
        ColorType::GreyAlpha,
        ColorType::Rgba,
    ];
}

pub fn ihdr_data(w: u32, h: u32, bit_depth: u8, color_type: u8) -> Vec<u8> {
    ihdr_full(w, h, bit_depth, color_type, 0, 0, 0)
}

pub fn ihdr_full(
    w: u32,
    h: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(13);
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.push(bit_depth);
    d.push(color_type);
    d.push(compression);
    d.push(filter);
    d.push(interlace);
    d
}

// ---------------------------------------------------------------------------
// Raw scanline construction
// ---------------------------------------------------------------------------

/// Build the raw (filtered) scanline stream that `cp_unfilter` consumes:
/// `h` rows of `1 + w*bpp` bytes, row `y` prefixed with `filters[y]`.
///
/// The *stored* bytes are random; no attempt is made to make the unfiltered
/// result "meaningful". That is deliberate: `cp_unfilter` is being differentially
/// tested, so arbitrary filtered bytes exercise it harder than bytes produced by
/// a real encoder, and the C's own wrap-around arithmetic is what we compare.
pub fn raw_scanlines(rng: &mut Rng, w: usize, h: usize, bpp: usize, filters: &[u8]) -> Vec<u8> {
    assert_eq!(filters.len(), h);
    let stride = w * bpp;
    let mut v = Vec::with_capacity(h * (stride + 1));
    for y in 0..h {
        v.push(filters[y]);
        for _ in 0..stride {
            v.push(rng.u8());
        }
    }
    v
}

pub fn raw_size(w: usize, h: usize, bpp: usize) -> usize {
    h * (w * bpp + 1)
}

// ---------------------------------------------------------------------------
// Whole-file assembly
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PngSpec {
    pub w: u32,
    pub h: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub compression: u8,
    pub filter: u8,
    pub interlace: u8,
    /// zlib CMF/FLG bytes.
    pub cmf: u8,
    pub flg: u8,
    pub plte: Option<Vec<u8>>,
    pub trns: Option<Vec<u8>>,
    /// `true` ⇒ tRNS is emitted before PLTE.
    pub trns_first: bool,
    /// Ancillary chunks emitted right after IHDR.
    pub pre: Vec<Vec<u8>>,
    /// Ancillary chunks emitted between PLTE/tRNS and IDAT.
    pub mid: Vec<Vec<u8>>,
    /// Chunks emitted after the last IDAT.
    pub post: Vec<Vec<u8>>,
    /// Number of IDAT chunks the zlib stream is split across.
    pub idat_chunks: usize,
    /// The deflate stream (without the 2-byte zlib header / 4-byte adler).
    pub deflate: Vec<u8>,
    /// The raw bytes the deflate stream expands to (for the adler32).
    pub raw: Vec<u8>,
    pub with_iend: bool,
}

impl PngSpec {
    pub fn new(w: u32, h: u32, color_type: u8, deflate: Vec<u8>, raw: Vec<u8>) -> Self {
        PngSpec {
            w,
            h,
            bit_depth: 8,
            color_type,
            compression: 0,
            filter: 0,
            interlace: 0,
            cmf: 0x78,
            flg: 0x9C,
            plte: None,
            trns: None,
            trns_first: false,
            pre: Vec::new(),
            mid: Vec::new(),
            post: Vec::new(),
            idat_chunks: 1,
            deflate,
            raw,
            with_iend: true,
        }
    }

    pub fn build(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&SIG);
        v.extend_from_slice(&chunk(
            b"IHDR",
            &ihdr_full(
                self.w,
                self.h,
                self.bit_depth,
                self.color_type,
                self.compression,
                self.filter,
                self.interlace,
            ),
        ));
        for c in &self.pre {
            v.extend_from_slice(c);
        }
        let plte_chunk = self.plte.as_ref().map(|p| chunk(b"PLTE", p));
        let trns_chunk = self.trns.as_ref().map(|t| chunk(b"tRNS", t));
        if self.trns_first {
            if let Some(c) = &trns_chunk {
                v.extend_from_slice(c);
            }
            if let Some(c) = &plte_chunk {
                v.extend_from_slice(c);
            }
        } else {
            if let Some(c) = &plte_chunk {
                v.extend_from_slice(c);
            }
            if let Some(c) = &trns_chunk {
                v.extend_from_slice(c);
            }
        }
        for c in &self.mid {
            v.extend_from_slice(c);
        }
        let z = deflate::zlib_wrap(&self.deflate, &self.raw, self.cmf, self.flg);
        let n = self.idat_chunks.max(1);
        let per = (z.len() + n - 1) / n;
        let mut off = 0usize;
        let mut emitted = 0usize;
        while off < z.len() || emitted == 0 {
            let end = (off + per).min(z.len());
            v.extend_from_slice(&chunk(b"IDAT", &z[off..end]));
            off = end;
            emitted += 1;
            if emitted >= n && off >= z.len() {
                break;
            }
            if off >= z.len() {
                break;
            }
        }
        for c in &self.post {
            v.extend_from_slice(c);
        }
        if self.with_iend {
            v.extend_from_slice(&chunk(b"IEND", &[]));
        }
        v
    }
}

/// Split `z` at `cuts` random points instead of into equal pieces.
pub fn idat_chunks_at(z: &[u8], cuts: &[usize]) -> Vec<u8> {
    let mut bounds: Vec<usize> = cuts.to_vec();
    bounds.push(z.len());
    bounds.sort_unstable();
    let mut v = Vec::new();
    let mut off = 0usize;
    for b in bounds {
        let b = b.min(z.len());
        if b > off || (off == 0 && b == 0) {
            v.extend_from_slice(&chunk(b"IDAT", &z[off..b]));
            off = b;
        }
    }
    if off < z.len() {
        v.extend_from_slice(&chunk(b"IDAT", &z[off..]));
    }
    v
}

// ---------------------------------------------------------------------------
// Reference decoder — an independent model of what the C should produce
// ---------------------------------------------------------------------------

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Reimplementation of `cp_unfilter` (including its quirks: the first row uses
/// `b = c = 0` for Paeth and skips `x < bpp` entirely for Sub/Average/Paeth;
/// the `y >= 1` Sub case adds 0 for `x < bpp`).
pub fn model_unfilter(w: usize, h: usize, bpp: usize, raw: &mut [u8]) -> bool {
    let len = w * bpp;
    let mut off = 0usize;
    if h > 0 {
        let f = raw[off];
        off += 1;
        match f {
            0 | 2 => {}
            1 => {
                for x in bpp..len {
                    raw[off + x] = raw[off + x].wrapping_add(raw[off + x - bpp]);
                }
            }
            3 => {
                for x in bpp..len {
                    raw[off + x] = raw[off + x].wrapping_add(raw[off + x - bpp] / 2);
                }
            }
            4 => {
                for x in bpp..len {
                    raw[off + x] = raw[off + x].wrapping_add(paeth(raw[off + x - bpp], 0, 0));
                }
            }
            _ => return false,
        }
    }
    let mut prev = off;
    off += len;
    for _y in 1..h {
        let f = raw[off];
        off += 1;
        match f {
            0 => {}
            1 => {
                for x in bpp..len {
                    raw[off + x] = raw[off + x].wrapping_add(raw[off + x - bpp]);
                }
            }
            2 => {
                for x in 0..len {
                    raw[off + x] = raw[off + x].wrapping_add(raw[prev + x]);
                }
            }
            3 => {
                for x in 0..bpp {
                    raw[off + x] = raw[off + x].wrapping_add(raw[prev + x] / 2);
                }
                for x in bpp..len {
                    let s = (raw[off + x - bpp] as i32 + raw[prev + x] as i32) / 2;
                    raw[off + x] = raw[off + x].wrapping_add(s as u8);
                }
            }
            4 => {
                for x in 0..bpp {
                    raw[off + x] = raw[off + x].wrapping_add(raw[prev + x]);
                }
                for x in bpp..len {
                    let p = paeth(raw[off + x - bpp], raw[prev + x], raw[prev + x - bpp]);
                    raw[off + x] = raw[off + x].wrapping_add(p);
                }
            }
            _ => return false,
        }
        prev = off;
        off += len;
    }
    true
}

/// Reimplementation of `cp_convert` / `cp_depalette`, producing the `w*h*4`
/// RGBA payload the differential tests compare.
pub fn model_pixels(
    w: usize,
    h: usize,
    bpp: usize,
    color_type: u8,
    unfiltered: &[u8],
    plte: Option<&[u8]>,
    trns: Option<&[u8]>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    let stride = w * bpp + 1;
    for y in 0..h {
        let row = y * stride + 1;
        for x in 0..w {
            let s = row + x * bpp;
            if color_type == 3 {
                let c = unfiltered[s] as usize;
                let p = plte.unwrap();
                let r = p.get(c * 3).copied().unwrap_or(0);
                let g = p.get(c * 3 + 1).copied().unwrap_or(0);
                let b = p.get(c * 3 + 2).copied().unwrap_or(0);
                let a = match trns {
                    None => 255,
                    Some(t) => {
                        if c >= t.len() {
                            255
                        } else {
                            t[c]
                        }
                    }
                };
                out.extend_from_slice(&[r, g, b, a]);
            } else {
                match bpp {
                    1 => {
                        let v = unfiltered[s];
                        out.extend_from_slice(&[v, v, v, 255]);
                    }
                    2 => {
                        let v = unfiltered[s];
                        out.extend_from_slice(&[v, v, v, unfiltered[s + 1]]);
                    }
                    3 => out.extend_from_slice(&[
                        unfiltered[s],
                        unfiltered[s + 1],
                        unfiltered[s + 2],
                        255,
                    ]),
                    4 => out.extend_from_slice(&[
                        unfiltered[s],
                        unfiltered[s + 1],
                        unfiltered[s + 2],
                        unfiltered[s + 3],
                    ]),
                    _ => unreachable!(),
                }
            }
        }
    }
    out
}

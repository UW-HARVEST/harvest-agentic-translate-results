//! Helpers to synthesise PNG byte streams (valid and deliberately invalid) so
//! that the read side of both libraries can be driven with identical input.
#![allow(dead_code)]

pub const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

// ---------------------------------------------------------------------------
// CRC-32 (the PNG/zlib polynomial)
// ---------------------------------------------------------------------------

fn crc_table() -> &'static [u32; 256] {
    use std::sync::OnceLock;
    static T: OnceLock<[u32; 256]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0u32; 256];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            t[n as usize] = c;
        }
        t
    })
}

pub fn crc32(data: &[u8]) -> u32 {
    let t = crc_table();
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c = t[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

pub fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// ---------------------------------------------------------------------------
// Chunk / stream assembly
// ---------------------------------------------------------------------------

pub fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// Append a well-formed chunk (correct length + CRC).
pub fn push_chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&be32(data.len() as u32));
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    let mut crc_in = Vec::with_capacity(4 + data.len());
    crc_in.extend_from_slice(name);
    crc_in.extend_from_slice(data);
    out.extend_from_slice(&be32(crc32(&crc_in)));
}

/// Append a chunk with an arbitrary (possibly wrong) declared length and CRC.
pub fn push_chunk_raw(out: &mut Vec<u8>, len: u32, name: &[u8; 4], data: &[u8], crc: u32) {
    out.extend_from_slice(&be32(len));
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    out.extend_from_slice(&be32(crc));
}

/// Append a chunk whose CRC is deliberately corrupted.
pub fn push_chunk_bad_crc(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    let mut crc_in = Vec::with_capacity(4 + data.len());
    crc_in.extend_from_slice(name);
    crc_in.extend_from_slice(data);
    let crc = crc32(&crc_in) ^ 0xffff_ffff;
    push_chunk_raw(out, data.len() as u32, name, data, crc);
}

pub fn ihdr_data(
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(13);
    d.extend_from_slice(&be32(width));
    d.extend_from_slice(&be32(height));
    d.push(bit_depth);
    d.push(color_type);
    d.push(compression);
    d.push(filter);
    d.push(interlace);
    d
}

// ---------------------------------------------------------------------------
// A minimal "store"-mode DEFLATE encoder, so the tests need no zlib of their
// own and the IDAT payload is fully deterministic.
// ---------------------------------------------------------------------------

pub fn zlib_store(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + raw.len() / 65535 * 5 + 11);
    out.push(0x78); // CM=8, CINFO=7
    out.push(0x01); // FLEVEL=0, FCHECK so that (0x78<<8|0x01) % 31 == 0
    if raw.is_empty() {
        out.push(0x01);
        out.extend_from_slice(&[0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut i = 0usize;
        while i < raw.len() {
            let n = (raw.len() - i).min(65535);
            let last = if i + n == raw.len() { 1u8 } else { 0u8 };
            out.push(last);
            out.extend_from_slice(&(n as u16).to_le_bytes());
            out.extend_from_slice(&(!(n as u16)).to_le_bytes());
            out.extend_from_slice(&raw[i..i + n]);
            i += n;
        }
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

// ---------------------------------------------------------------------------
// Row geometry
// ---------------------------------------------------------------------------

pub fn channels_of(color_type: u8) -> u32 {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

pub fn pixel_depth(bit_depth: u8, color_type: u8) -> u32 {
    bit_depth as u32 * channels_of(color_type)
}

pub fn rowbytes(bit_depth: u8, color_type: u8, width: u32) -> usize {
    let pd = pixel_depth(bit_depth, color_type) as u64;
    (((width as u64 * pd) + 7) >> 3) as usize
}

/// Adam7 pass geometry: (xstart, ystart, xstep, ystep)
pub const ADAM7: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

pub fn pass_width(width: u32, pass: usize) -> u32 {
    let (xs, _, xt, _) = ADAM7[pass];
    if width <= xs {
        0
    } else {
        (width - xs + xt - 1) / xt
    }
}

pub fn pass_height(height: u32, pass: usize) -> u32 {
    let (_, ys, _, yt) = ADAM7[pass];
    if height <= ys {
        0
    } else {
        (height - ys + yt - 1) / yt
    }
}

/// Build the raw (filter-byte prefixed, filter type 0 = None) scan-line data
/// for a non-interlaced image whose pixel bytes come from `gen`.
pub fn raw_rows_none(
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    gen: &mut dyn FnMut(u32, usize) -> Vec<u8>,
) -> Vec<u8> {
    let rb = rowbytes(bit_depth, color_type, width);
    let mut out = Vec::with_capacity(height as usize * (rb + 1));
    for y in 0..height {
        out.push(0); // filter: None
        let mut row = gen(y, rb);
        row.resize(rb, 0);
        // zero the unused low bits of the final byte, as libpng writes them
        if rb > 0 {
            let pd = pixel_depth(bit_depth, color_type);
            if pd < 8 {
                let used = (width * pd) % 8;
                if used != 0 {
                    let mask = 0xffu8 << (8 - used);
                    let last = rb - 1;
                    row[last] &= mask;
                }
            }
        }
        out.extend_from_slice(&row);
    }
    out
}

/// Interlaced (Adam7) equivalent of `raw_rows_none`.
pub fn raw_rows_adam7(
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    gen: &mut dyn FnMut(usize, u32, usize) -> Vec<u8>,
) -> Vec<u8> {
    let mut out = Vec::new();
    let pd = pixel_depth(bit_depth, color_type);
    for pass in 0..7 {
        let pw = pass_width(width, pass);
        let ph = pass_height(height, pass);
        if pw == 0 || ph == 0 {
            continue;
        }
        let rb = rowbytes(bit_depth, color_type, pw);
        for y in 0..ph {
            out.push(0);
            let mut row = gen(pass, y, rb);
            row.resize(rb, 0);
            if pd < 8 && rb > 0 {
                let used = (pw * pd) % 8;
                if used != 0 {
                    let mask = 0xffu8 << (8 - used);
                    let last = rb - 1;
                    row[last] &= mask;
                }
            }
            out.extend_from_slice(&row);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Complete images
// ---------------------------------------------------------------------------

pub struct PngSpec {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub interlace: u8,
    /// PLTE entries (RGB triples), required for colour type 3.
    pub palette: Vec<u8>,
    /// tRNS chunk payload, if any.
    pub trns: Option<Vec<u8>>,
    /// extra chunks placed before IDAT, in order
    pub pre_idat: Vec<([u8; 4], Vec<u8>)>,
    /// extra chunks placed after IDAT, in order
    pub post_idat: Vec<([u8; 4], Vec<u8>)>,
    /// raw (unfiltered, filter-byte prefixed) image data
    pub raw: Vec<u8>,
    /// split the IDAT payload into this many chunks (>=1)
    pub idat_chunks: usize,
}

impl PngSpec {
    pub fn new(width: u32, height: u32, bit_depth: u8, color_type: u8, interlace: u8) -> Self {
        PngSpec {
            width,
            height,
            bit_depth,
            color_type,
            interlace,
            palette: Vec::new(),
            trns: None,
            pre_idat: Vec::new(),
            post_idat: Vec::new(),
            raw: Vec::new(),
            idat_chunks: 1,
        }
    }

    pub fn build(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&PNG_SIG);
        push_chunk(
            &mut out,
            b"IHDR",
            &ihdr_data(
                self.width,
                self.height,
                self.bit_depth,
                self.color_type,
                0,
                0,
                self.interlace,
            ),
        );
        if !self.palette.is_empty() {
            push_chunk(&mut out, b"PLTE", &self.palette);
        }
        if let Some(t) = &self.trns {
            push_chunk(&mut out, b"tRNS", t);
        }
        for (n, d) in &self.pre_idat {
            push_chunk(&mut out, n, d);
        }
        let z = zlib_store(&self.raw);
        let nch = self.idat_chunks.max(1);
        let per = (z.len() + nch - 1) / nch.max(1);
        if z.is_empty() {
            push_chunk(&mut out, b"IDAT", &[]);
        } else {
            let mut i = 0;
            while i < z.len() {
                let n = per.max(1).min(z.len() - i);
                push_chunk(&mut out, b"IDAT", &z[i..i + n]);
                i += n;
            }
        }
        for (n, d) in &self.post_idat {
            push_chunk(&mut out, n, d);
        }
        push_chunk(&mut out, b"IEND", &[]);
        out
    }
}

/// Deterministic, fully valid PNG of the given geometry.
pub fn make_png(
    seed: u64,
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    interlace: u8,
) -> Vec<u8> {
    let mut rng = super::Rng::new(seed);
    let mut spec = PngSpec::new(width, height, bit_depth, color_type, interlace);
    if color_type == 3 {
        let n = 1usize << bit_depth.min(8);
        let n = n.min(256);
        spec.palette = (0..n * 3).map(|_| rng.next_u8()).collect();
    }
    spec.raw = if interlace == 1 {
        let mut r2 = super::Rng::new(seed ^ 0xabcd);
        raw_rows_adam7(width, height, bit_depth, color_type, &mut |_p, _y, rb| {
            (0..rb).map(|_| r2.next_u8()).collect()
        })
    } else {
        let mut r2 = super::Rng::new(seed ^ 0xabcd);
        raw_rows_none(width, height, bit_depth, color_type, &mut |_y, rb| {
            (0..rb).map(|_| r2.next_u8()).collect()
        })
    };
    spec.build()
}

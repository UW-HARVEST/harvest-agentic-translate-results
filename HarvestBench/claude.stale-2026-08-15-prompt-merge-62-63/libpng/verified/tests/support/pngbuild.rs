//! Self-contained PNG stream construction / dissection, independent of both
//! libraries under test.  Used to feed byte-identical (valid or deliberately
//! malformed) inputs to the C and the Rust decoder.
#![allow(dead_code)]

pub const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

// ---------------------------------------------------------------------------
// checksums
// ---------------------------------------------------------------------------

pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, e) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *e = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for b in data {
        c = table[((c ^ *b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for x in data {
        a = (a + *x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// ---------------------------------------------------------------------------
// zlib stream with stored (uncompressed) deflate blocks
// ---------------------------------------------------------------------------

/// A valid zlib stream carrying `data` in stored deflate blocks.
pub fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01]; // CM=8, CINFO=7, FCHECK making 0x7801 % 31 == 0
    if data.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut off = 0usize;
        while off < data.len() {
            let n = std::cmp::min(65535, data.len() - off);
            let last = off + n >= data.len();
            out.push(if last { 1 } else { 0 });
            out.extend_from_slice(&(n as u16).to_le_bytes());
            out.extend_from_slice(&(!(n as u16)).to_le_bytes());
            out.extend_from_slice(&data[off..off + n]);
            off += n;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

// ---------------------------------------------------------------------------
// chunks
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub name: [u8; 4],
    pub data: Vec<u8>,
    /// When `Some`, this CRC is emitted verbatim instead of the correct one.
    pub crc_override: Option<u32>,
    /// When `Some`, this length is emitted verbatim instead of `data.len()`.
    pub len_override: Option<u32>,
}

impl Chunk {
    pub fn new(name: &[u8; 4], data: Vec<u8>) -> Chunk {
        Chunk {
            name: *name,
            data,
            crc_override: None,
            len_override: None,
        }
    }
    pub fn bad_crc(mut self) -> Chunk {
        let good = self.crc();
        self.crc_override = Some(good ^ 0xFFFF_FFFF);
        self
    }
    pub fn with_crc(mut self, crc: u32) -> Chunk {
        self.crc_override = Some(crc);
        self
    }
    pub fn with_len(mut self, len: u32) -> Chunk {
        self.len_override = Some(len);
        self
    }
    pub fn crc(&self) -> u32 {
        let mut buf = Vec::with_capacity(4 + self.data.len());
        buf.extend_from_slice(&self.name);
        buf.extend_from_slice(&self.data);
        crc32(&buf)
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + self.data.len());
        out.extend_from_slice(&self.len_override.unwrap_or(self.data.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.name);
        out.extend_from_slice(&self.data);
        out.extend_from_slice(&self.crc_override.unwrap_or_else(|| self.crc()).to_be_bytes());
        out
    }
}

/// Split a complete PNG datastream into its signature and chunk list.
pub fn split(png: &[u8]) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut i = 8; // skip signature
    while i + 8 <= png.len() {
        let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let name = [png[i + 4], png[i + 5], png[i + 6], png[i + 7]];
        let ds = i + 8;
        let de = std::cmp::min(ds + len, png.len());
        let data = png[ds..de].to_vec();
        out.push(Chunk::new(&name, data));
        i = de + 4;
    }
    out
}

pub fn join(chunks: &[Chunk]) -> Vec<u8> {
    let mut out = SIG.to_vec();
    for c in chunks {
        out.extend_from_slice(&c.encode());
    }
    out
}

pub fn find(chunks: &[Chunk], name: &[u8; 4]) -> Option<usize> {
    chunks.iter().position(|c| &c.name == name)
}

// ---------------------------------------------------------------------------
// image geometry helpers (mirrors PNG_ROWBYTES / the Adam7 tables)
// ---------------------------------------------------------------------------

pub fn channels(color_type: u8) -> u32 {
    match color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

pub fn pixel_bits(color_type: u8, bit_depth: u8) -> u32 {
    channels(color_type) * bit_depth as u32
}

pub fn rowbytes(color_type: u8, bit_depth: u8, width: u32) -> usize {
    let pd = pixel_bits(color_type, bit_depth) as usize;
    (pd * width as usize + 7) >> 3
}

pub const ADAM7_XSTART: [u32; 7] = [0, 4, 0, 2, 0, 1, 0];
pub const ADAM7_YSTART: [u32; 7] = [0, 0, 4, 0, 2, 0, 1];
pub const ADAM7_XSTEP: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];
pub const ADAM7_YSTEP: [u32; 7] = [8, 8, 8, 4, 4, 2, 2];

pub fn pass_width(width: u32, pass: usize) -> u32 {
    if width <= ADAM7_XSTART[pass] {
        0
    } else {
        (width - ADAM7_XSTART[pass] + ADAM7_XSTEP[pass] - 1) / ADAM7_XSTEP[pass]
    }
}

pub fn pass_height(height: u32, pass: usize) -> u32 {
    if height <= ADAM7_YSTART[pass] {
        0
    } else {
        (height - ADAM7_YSTART[pass] + ADAM7_YSTEP[pass] - 1) / ADAM7_YSTEP[pass]
    }
}

/// Total size of the raw (filter byte + row) data stream for an image.
pub fn raw_size(width: u32, height: u32, color_type: u8, bit_depth: u8, interlace: u8) -> usize {
    if interlace == 0 {
        (rowbytes(color_type, bit_depth, width) + 1) * height as usize
    } else {
        (0..7)
            .map(|p| {
                let w = pass_width(width, p);
                let h = pass_height(height, p);
                if w == 0 || h == 0 {
                    0
                } else {
                    (rowbytes(color_type, bit_depth, w) + 1) * h as usize
                }
            })
            .sum()
    }
}

// ---------------------------------------------------------------------------
// builder
// ---------------------------------------------------------------------------

pub struct Builder {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub interlace: u8,
    pub compression: u8,
    pub filter: u8,
    pub chunks: Vec<Chunk>,
}

impl Builder {
    pub fn new(width: u32, height: u32, bit_depth: u8, color_type: u8) -> Builder {
        Builder {
            width,
            height,
            bit_depth,
            color_type,
            interlace: 0,
            compression: 0,
            filter: 0,
            chunks: Vec::new(),
        }
    }

    pub fn interlace(mut self, v: u8) -> Builder {
        self.interlace = v;
        self
    }

    pub fn ihdr_bytes(&self) -> Vec<u8> {
        let mut d = Vec::with_capacity(13);
        d.extend_from_slice(&self.width.to_be_bytes());
        d.extend_from_slice(&self.height.to_be_bytes());
        d.push(self.bit_depth);
        d.push(self.color_type);
        d.push(self.compression);
        d.push(self.filter);
        d.push(self.interlace);
        d
    }

    pub fn add(mut self, name: &[u8; 4], data: Vec<u8>) -> Builder {
        self.chunks.push(Chunk::new(name, data));
        self
    }

    pub fn add_chunk(mut self, c: Chunk) -> Builder {
        self.chunks.push(c);
        self
    }

    /// Raw pre-compression data: every row filter type 0 with `fill` content.
    pub fn raw_rows(&self, seed: u64) -> Vec<u8> {
        let n = raw_size(
            self.width,
            self.height,
            self.color_type,
            self.bit_depth,
            self.interlace,
        );
        let mut out = Vec::with_capacity(n);
        let mut s = seed | 1;
        let mut next = || {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            (s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as u8
        };
        if self.interlace == 0 {
            let rb = rowbytes(self.color_type, self.bit_depth, self.width);
            for _ in 0..self.height {
                out.push(0);
                for _ in 0..rb {
                    out.push(next());
                }
            }
        } else {
            for p in 0..7 {
                let w = pass_width(self.width, p);
                let h = pass_height(self.height, p);
                if w == 0 || h == 0 {
                    continue;
                }
                let rb = rowbytes(self.color_type, self.bit_depth, w);
                for _ in 0..h {
                    out.push(0);
                    for _ in 0..rb {
                        out.push(next());
                    }
                }
            }
        }
        out
    }

    /// Assemble: signature, IHDR, the added chunks, IDAT(s), IEND.
    pub fn build(&self, idat_payload: &[u8], idat_split: usize) -> Vec<u8> {
        let mut chunks = vec![Chunk::new(b"IHDR", self.ihdr_bytes())];
        chunks.extend(self.chunks.iter().cloned());
        let z = zlib_stored(idat_payload);
        if idat_split == 0 || z.len() <= idat_split {
            chunks.push(Chunk::new(b"IDAT", z));
        } else {
            for part in z.chunks(idat_split) {
                chunks.push(Chunk::new(b"IDAT", part.to_vec()));
            }
        }
        chunks.push(Chunk::new(b"IEND", Vec::new()));
        join(&chunks)
    }

    /// Convenience: a complete, valid PNG with pseudo-random image content.
    pub fn build_valid(&self, seed: u64) -> Vec<u8> {
        let raw = self.raw_rows(seed);
        self.build(&raw, 0)
    }
}

//! A minimal DEFLATE (RFC 1951) *encoder* used to generate test vectors, plus
//! a PNG container builder. Written from the RFC so that all three block types
//! and every code-length symbol (16/17/18) can be produced on demand.

#![allow(dead_code)]

pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
pub const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
pub const PERM: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

// ---------------------------------------------------------------------------
// Bit writer (DEFLATE packs bits LSB-first; Huffman codes MSB-of-code first)
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub buf: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            buf: Vec::new(),
            acc: 0,
            n: 0,
        }
    }
    pub fn bit(&mut self, b: u32) {
        self.acc |= (b & 1) << self.n;
        self.n += 1;
        if self.n == 8 {
            self.buf.push(self.acc as u8);
            self.acc = 0;
            self.n = 0;
        }
    }
    /// Raw integer field, least-significant bit first (headers, extra bits).
    pub fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit(v >> i);
        }
    }
    /// Huffman code, most-significant bit of the code first.
    pub fn code(&mut self, c: u32, n: u32) {
        for i in (0..n).rev() {
            self.bit(c >> i);
        }
    }
    pub fn align(&mut self) {
        while self.n != 0 {
            self.bit(0);
        }
    }
    pub fn bytes(&mut self, b: &[u8]) {
        assert_eq!(self.n, 0, "bytes() requires byte alignment");
        self.buf.extend_from_slice(b);
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.buf
    }
    pub fn bit_pos(&self) -> usize {
        self.buf.len() * 8 + self.n as usize
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman
// ---------------------------------------------------------------------------

/// RFC 1951 §3.2.2 canonical code assignment.
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut bl_count = [0u32; 16];
    for &l in lens {
        assert!(l <= 15);
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next = [0u32; 16];
    let mut code = 0u32;
    for b in 1..=15usize {
        code = (code + bl_count[b - 1]) << 1;
        next[b] = code;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l > 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

/// Complete prefix code over the `used` symbols with lengths `k-1`/`k`
/// (`k = ceil(log2 n)`). Kraft sum is exactly 1, so the code is complete.
pub fn balanced_lens(used: &[usize], total: usize) -> Vec<u8> {
    let mut lens = vec![0u8; total];
    let n = used.len();
    assert!(n >= 2, "balanced_lens needs >= 2 symbols");
    let mut k = 0u32;
    while (1usize << k) < n {
        k += 1;
    }
    let m = (1usize << k) - n; // symbols that get length k-1
    for (i, &s) in used.iter().enumerate() {
        lens[s] = if i < m { (k - 1) as u8 } else { k as u8 };
    }
    lens
}

/// Frequency-driven Huffman lengths (complete code). Panics if the natural
/// depth exceeds `max_len`; callers use `balanced_lens` when that happens.
pub fn huffman_lens(freqs: &[u32], max_len: u8) -> Option<Vec<u8>> {
    let n = freqs.len();
    let used: Vec<usize> = (0..n).filter(|&i| freqs[i] > 0).collect();
    if used.is_empty() {
        return Some(vec![0u8; n]);
    }
    if used.len() == 1 {
        let mut l = vec![0u8; n];
        l[used[0]] = 1;
        return Some(l);
    }
    // nodes: (weight, depth-accumulating leaf set)
    #[derive(Clone)]
    struct Node {
        w: u64,
        leaves: Vec<usize>,
    }
    let mut heap: Vec<Node> = used
        .iter()
        .map(|&s| Node {
            w: freqs[s] as u64,
            leaves: vec![s],
        })
        .collect();
    let mut lens = vec![0u8; n];
    while heap.len() > 1 {
        // pick two smallest (stable: lowest weight, then lowest first leaf)
        heap.sort_by_key(|x| (x.w, x.leaves[0]));
        let a = heap.remove(0);
        let b = heap.remove(0);
        for &s in a.leaves.iter().chain(b.leaves.iter()) {
            lens[s] += 1;
            if lens[s] > max_len {
                return None;
            }
        }
        let mut leaves = a.leaves;
        leaves.extend(b.leaves);
        leaves.sort();
        heap.push(Node { w: a.w + b.w, leaves });
    }
    Some(lens)
}

// ---------------------------------------------------------------------------
// Fixed Huffman tables (RFC 1951 §3.2.6) — derived from the same lengths the
// C's `cp_fixed_table` holds.
// ---------------------------------------------------------------------------

pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![0u8; 288];
    for i in 0..144 {
        v[i] = 8;
    }
    for i in 144..256 {
        v[i] = 9;
    }
    for i in 256..280 {
        v[i] = 7;
    }
    for i in 280..288 {
        v[i] = 8;
    }
    v
}

pub fn fixed_dst_lens() -> Vec<u8> {
    vec![5u8; 32]
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    /// (length 3..=258, distance 1..=32768)
    Match(u32, u32),
}

pub fn len_sym(l: u32) -> (usize, u32, u32) {
    assert!((3..=258).contains(&l));
    if l == 258 {
        return (28, 0, 0);
    }
    let mut s = 0usize;
    for i in 0..29 {
        if LEN_BASE[i] <= l && (i == 28 || LEN_BASE[i + 1] > l) {
            s = i;
            break;
        }
    }
    (s, LEN_EXTRA[s], l - LEN_BASE[s])
}

pub fn dist_sym(d: u32) -> (usize, u32, u32) {
    assert!((1..=32768).contains(&d));
    let mut s = 0usize;
    for i in 0..30 {
        if DIST_BASE[i] <= d && (i == 29 || DIST_BASE[i + 1] > d) {
            s = i;
            break;
        }
    }
    (s, DIST_EXTRA[s], d - DIST_BASE[s])
}

/// Applies the token list to a growing output buffer, exactly like the
/// decompressor would, so tests know the expected plaintext.
pub fn apply(toks: &[Tok]) -> Vec<u8> {
    let mut out = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match(l, d) => {
                let start = out.len() - d as usize;
                for i in 0..l as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Block writers
// ---------------------------------------------------------------------------

/// btype = 1 (fixed Huffman).
pub fn write_fixed_block(bw: &mut BitWriter, toks: &[Tok], final_block: bool) {
    bw.bit(final_block as u32);
    bw.bits(1, 2);
    let ll = fixed_lit_lens();
    let lc = canonical(&ll);
    let dl = fixed_dst_lens();
    let dc = canonical(&dl);
    emit_tokens(bw, toks, &ll, &lc, &dl, &dc);
}

fn emit_tokens(
    bw: &mut BitWriter,
    toks: &[Tok],
    ll: &[u8],
    lc: &[u32],
    dl: &[u8],
    dc: &[u32],
) {
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let s = b as usize;
                assert!(ll[s] > 0, "literal {s} has no code");
                bw.code(lc[s], ll[s] as u32);
            }
            Tok::Match(l, d) => {
                let (ls, lx, lv) = len_sym(l);
                let s = 257 + ls;
                assert!(ll[s] > 0, "length symbol {s} has no code");
                bw.code(lc[s], ll[s] as u32);
                bw.bits(lv, lx);
                let (ds, dx, dv) = dist_sym(d);
                assert!(dl[ds] > 0, "distance symbol {ds} has no code");
                bw.code(dc[ds], dl[ds] as u32);
                bw.bits(dv, dx);
            }
        }
    }
    assert!(ll[256] > 0);
    bw.code(lc[256], ll[256] as u32);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClMode {
    /// Emit every code length as a literal 0..15 symbol (no 16/17/18).
    Literal,
    /// Full run-length encoding (uses 16, 17 and 18).
    Rle,
}

/// Code-length sequence entry: (symbol, extra value, extra bit count)
pub fn cl_sequence(all: &[u8], mode: ClMode) -> Vec<(usize, u32, u32)> {
    let mut out = Vec::new();
    if mode == ClMode::Literal {
        for &l in all {
            out.push((l as usize, 0, 0));
        }
        return out;
    }
    let n = all.len();
    let mut i = 0usize;
    while i < n {
        let v = all[i];
        let mut run = 1usize;
        while i + run < n && all[i + run] == v {
            run += 1;
        }
        if v == 0 {
            while run >= 3 {
                if run >= 11 {
                    let take = run.min(138);
                    out.push((18, (take - 11) as u32, 7));
                    run -= take;
                    i += take;
                } else {
                    let take = run.min(10);
                    out.push((17, (take - 3) as u32, 3));
                    run -= take;
                    i += take;
                }
            }
            for _ in 0..run {
                out.push((0, 0, 0));
                i += 1;
            }
        } else {
            // first occurrence is a literal, remaining can use symbol 16
            out.push((v as usize, 0, 0));
            i += 1;
            run -= 1;
            while run >= 3 {
                let take = run.min(6);
                out.push((16, (take - 3) as u32, 2));
                run -= take;
                i += take;
            }
            for _ in 0..run {
                out.push((v as usize, 0, 0));
                i += 1;
            }
        }
    }
    out
}

/// btype = 2 (dynamic Huffman). `nlen_override` forces HCLEN (must be >= the
/// natural minimum).
pub fn write_dynamic_block(
    bw: &mut BitWriter,
    toks: &[Tok],
    lit_lens: &[u8],
    dst_lens: &[u8],
    mode: ClMode,
    nlen_override: Option<usize>,
    final_block: bool,
) {
    let nlit = lit_lens.len();
    let ndst = dst_lens.len();
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));

    let mut all: Vec<u8> = Vec::with_capacity(nlit + ndst);
    all.extend_from_slice(lit_lens);
    all.extend_from_slice(dst_lens);
    let seq = cl_sequence(&all, mode);

    let mut freqs = [0u32; 19];
    for &(s, _, _) in &seq {
        freqs[s] += 1;
    }
    let cl_lens = huffman_lens(&freqs, 7).unwrap_or_else(|| {
        let used: Vec<usize> = (0..19).filter(|&i| freqs[i] > 0).collect();
        balanced_lens(&used, 19)
    });
    let cl_codes = canonical(&cl_lens);

    // HCLEN: highest index in PERM order with a non-zero length, +1 (min 4)
    let mut natural = 4usize;
    for i in 0..19 {
        if cl_lens[PERM[i]] != 0 {
            natural = natural.max(i + 1);
        }
    }
    let nlen = nlen_override.unwrap_or(natural);
    assert!((4..=19).contains(&nlen));
    assert!(nlen >= natural, "HCLEN override {nlen} drops used symbols");

    bw.bit(final_block as u32);
    bw.bits(2, 2);
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        bw.bits(cl_lens[PERM[i]] as u32, 3);
    }
    for &(s, v, nb) in &seq {
        assert!(cl_lens[s] > 0);
        bw.code(cl_codes[s], cl_lens[s] as u32);
        bw.bits(v, nb);
    }

    let lc = canonical(lit_lens);
    let dc = canonical(dst_lens);
    emit_tokens(bw, toks, lit_lens, &lc, dst_lens, &dc);
}

/// Picks a valid, complete pair of (literal/length, distance) code-length
/// tables that covers every symbol the token list uses.
pub fn lens_for(toks: &[Tok], nlit: usize, ndst: usize) -> (Vec<u8>, Vec<u8>) {
    let mut lf = vec![0u32; nlit];
    let mut df = vec![0u32; ndst];
    lf[256] = 1;
    for t in toks {
        match *t {
            Tok::Lit(b) => lf[b as usize] += 3,
            Tok::Match(l, d) => {
                lf[257 + len_sym(l).0] += 3;
                df[dist_sym(d).0] += 3;
            }
        }
    }
    let lused: Vec<usize> = (0..nlit).filter(|&i| lf[i] > 0).collect();
    let dused: Vec<usize> = (0..ndst).filter(|&i| df[i] > 0).collect();
    let ll = if lused.len() >= 2 {
        huffman_lens(&lf, 15).unwrap_or_else(|| balanced_lens(&lused, nlit))
    } else {
        // need at least 2 symbols for a complete code
        let mut u = lused.clone();
        for cand in 0..nlit {
            if u.len() >= 2 {
                break;
            }
            if !u.contains(&cand) {
                u.push(cand);
            }
        }
        u.sort();
        balanced_lens(&u, nlit)
    };
    let dl = if dused.is_empty() {
        let mut v = vec![0u8; ndst];
        v[0] = 1;
        v
    } else if dused.len() == 1 {
        let mut v = vec![0u8; ndst];
        v[dused[0]] = 1;
        v
    } else {
        huffman_lens(&df, 15).unwrap_or_else(|| balanced_lens(&dused, ndst))
    };
    (ll, dl)
}

/// Emits a dynamic-block header with an ARBITRARY code-length symbol sequence,
/// including sequences that write past the end of the C's `uint8_t lens[320]`
/// (a run-length code 16/17/18 near the end of the table). Used to exercise the
/// C's stack-frame overrun in `cp_dynamic`.
pub fn write_dynamic_header_raw(
    bw: &mut BitWriter,
    nlit: usize,
    ndst: usize,
    seq: &[(usize, u32, u32)],
    final_block: bool,
) {
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));
    let mut freqs = [0u32; 19];
    for &(s, _, _) in seq {
        freqs[s] += 1;
    }
    let cl_lens = huffman_lens(&freqs, 7).unwrap_or_else(|| {
        let used: Vec<usize> = (0..19).filter(|&i| freqs[i] > 0).collect();
        balanced_lens(&used, 19)
    });
    let cl_codes = canonical(&cl_lens);
    let mut natural = 4usize;
    for i in 0..19 {
        if cl_lens[PERM[i]] != 0 {
            natural = natural.max(i + 1);
        }
    }
    bw.bit(final_block as u32);
    bw.bits(2, 2);
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((natural - 4) as u32, 4);
    for i in 0..natural {
        bw.bits(cl_lens[PERM[i]] as u32, 3);
    }
    for &(s, v, nb) in seq {
        assert!(cl_lens[s] > 0, "CL symbol {s} has no code");
        bw.code(cl_codes[s], cl_lens[s] as u32);
        bw.bits(v, nb);
    }
}

/// btype = 0 (stored). The C only accepts a stored block whose payload runs to
/// the very end of the input (`bits_left/8 <= LEN`), i.e. it must be the last
/// block in the stream.
pub fn write_stored_block(bw: &mut BitWriter, data: &[u8], final_block: bool) {
    assert!(data.len() <= 0xFFFF);
    bw.bit(final_block as u32);
    bw.bits(0, 2);
    bw.align();
    let len = data.len() as u16;
    bw.bits(len as u32, 16);
    bw.bits((!len) as u32, 16);
    bw.bytes(data);
}

/// Fixed-Huffman literal-only stream, the simplest complete DEFLATE stream.
pub fn deflate_fixed_literals(data: &[u8]) -> Vec<u8> {
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &toks, true);
    bw.finish()
}

/// Dynamic-Huffman literal-only stream.
pub fn deflate_dynamic_literals(data: &[u8], mode: ClMode) -> Vec<u8> {
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let (ll, dl) = lens_for(&toks, 288, 32);
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, &toks, &ll, &dl, mode, None, true);
    bw.finish()
}

/// Single stored block containing `data`.
pub fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, data, true);
    bw.finish()
}

/// A greedy LZ77 compressor (window 32768, min match 3) emitting one fixed
/// Huffman block. Produces real matches, including `distance == 1` runs.
pub fn deflate_fixed_lz(data: &[u8]) -> Vec<u8> {
    let toks = lz77(data);
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &toks, true);
    bw.finish()
}

pub fn deflate_dynamic_lz(data: &[u8]) -> Vec<u8> {
    let toks = lz77(data);
    let (ll, dl) = lens_for(&toks, 288, 32);
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, &toks, &ll, &dl, ClMode::Rle, None, true);
    bw.finish()
}

pub fn lz77(data: &[u8]) -> Vec<Tok> {
    let n = data.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < n {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        let lo = i.saturating_sub(32768);
        if n - i >= 3 {
            let mut j = i;
            while j > lo {
                j -= 1;
                let maxl = (n - i).min(258);
                let mut l = 0usize;
                while l < maxl && data[j + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - j;
                    if l == maxl {
                        break;
                    }
                }
            }
        }
        if best_len >= 3 {
            out.push(Tok::Match(best_len as u32, best_dist as u32));
            i += best_len;
        } else {
            out.push(Tok::Lit(data[i]));
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// zlib wrapper (RFC 1950). The C checks CM, CINFO and FDICT but ignores the
// Adler-32 trailer entirely; it only needs `datalen >= 6`.
// ---------------------------------------------------------------------------

pub fn zlib_wrap(deflate: &[u8], cmf: u8, flg: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(deflate.len() + 6);
    v.push(cmf);
    v.push(flg);
    v.extend_from_slice(deflate);
    v.extend_from_slice(&[0, 0, 0, 0]); // Adler-32 placeholder (unchecked)
    v
}

pub fn zlib(deflate: &[u8]) -> Vec<u8> {
    zlib_wrap(deflate, 0x78, 0x9C)
}

// ---------------------------------------------------------------------------
// PNG container
// ---------------------------------------------------------------------------

pub const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

pub fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len() + 12);
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(data);
    v.extend_from_slice(&[0, 0, 0, 0]); // CRC — never validated by this C code
    v
}

pub fn ihdr_data(w: u32, h: u32, bit_depth: u8, color_type: u8, comp: u8, filt: u8, il: u8) -> Vec<u8> {
    let mut d = Vec::with_capacity(13);
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.push(bit_depth);
    d.push(color_type);
    d.push(comp);
    d.push(filt);
    d.push(il);
    d
}

pub fn bpp_for(color_type: u8) -> usize {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => panic!("unsupported colour type {color_type}"),
    }
}

/// Everything the builder can vary, mirroring the axes in CONFIGS.md.
pub struct PngSpec {
    pub w: u32,
    pub h: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub compression: u8,
    pub filter_method: u8,
    pub interlace: u8,
    /// raw scanline bytes *including* the leading filter byte of each row
    pub raw: Vec<u8>,
    pub plte: Option<Vec<u8>>,
    pub trns: Option<Vec<u8>>,
    pub idat_parts: usize,
    pub cmf: u8,
    pub flg: u8,
    pub deflate: DeflateMode,
    pub pre_idat_chunks: Vec<([u8; 4], Vec<u8>)>,
    pub iend: bool,
    /// override the zlib payload entirely (for error-path tests)
    pub raw_zlib: Option<Vec<u8>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeflateMode {
    Stored,
    FixedLiterals,
    FixedLz,
    DynamicLiteralCl,
    DynamicRle,
    DynamicLz,
}

impl PngSpec {
    pub fn new(w: u32, h: u32, color_type: u8) -> PngSpec {
        PngSpec {
            w,
            h,
            bit_depth: 8,
            color_type,
            compression: 0,
            filter_method: 0,
            interlace: 0,
            raw: Vec::new(),
            plte: None,
            trns: None,
            idat_parts: 1,
            cmf: 0x78,
            flg: 0x9C,
            deflate: DeflateMode::FixedLz,
            pre_idat_chunks: Vec::new(),
            iend: true,
            raw_zlib: None,
        }
    }

    pub fn build(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&PNG_SIG);
        v.extend_from_slice(&chunk(
            b"IHDR",
            &ihdr_data(
                self.w,
                self.h,
                self.bit_depth,
                self.color_type,
                self.compression,
                self.filter_method,
                self.interlace,
            ),
        ));
        if let Some(p) = &self.plte {
            v.extend_from_slice(&chunk(b"PLTE", p));
        }
        if let Some(t) = &self.trns {
            v.extend_from_slice(&chunk(b"tRNS", t));
        }
        for (k, d) in &self.pre_idat_chunks {
            v.extend_from_slice(&chunk(k, d));
        }
        let z = match &self.raw_zlib {
            Some(z) => z.clone(),
            None => {
                let d = match self.deflate {
                    DeflateMode::Stored => deflate_stored(&self.raw),
                    DeflateMode::FixedLiterals => deflate_fixed_literals(&self.raw),
                    DeflateMode::FixedLz => deflate_fixed_lz(&self.raw),
                    DeflateMode::DynamicLiteralCl => {
                        deflate_dynamic_literals(&self.raw, ClMode::Literal)
                    }
                    DeflateMode::DynamicRle => deflate_dynamic_literals(&self.raw, ClMode::Rle),
                    DeflateMode::DynamicLz => deflate_dynamic_lz(&self.raw),
                };
                zlib_wrap(&d, self.cmf, self.flg)
            }
        };
        let parts = self.idat_parts.max(1);
        let per = (z.len() + parts - 1) / parts;
        let mut off = 0usize;
        let mut emitted = 0usize;
        while off < z.len() || emitted == 0 {
            let end = (off + per).min(z.len());
            v.extend_from_slice(&chunk(b"IDAT", &z[off..end]));
            off = end;
            emitted += 1;
            if per == 0 {
                break;
            }
        }
        if self.iend {
            v.extend_from_slice(&chunk(b"IEND", &[]));
        }
        v
    }
}

/// Builds raw scanlines by *forward* filtering, i.e. the inverse of what
/// `cp_unfilter` does, so the decoded image is exactly `pixels`.
///
/// Note: `cp_unfilter`'s row 0 handling is **not** the PNG spec's — filter 2 is
/// a no-op and filters 1/3/4 skip the first pixel and use `0` for the missing
/// neighbours in a way that differs from the spec for filter 3 (`raw[x-bpp]/2`
/// with no `prev`). The encoder below mirrors the C exactly so round-tripping
/// is faithful.
pub fn encode_scanlines(w: usize, h: usize, bpp: usize, data: &[u8], filters: &[u8]) -> Vec<u8> {
    let stride = w * bpp;
    assert_eq!(data.len(), stride * h);
    assert_eq!(filters.len(), h);
    let mut raw = vec![0u8; (stride + 1) * h];
    // `cur` holds the already-unfiltered previous row.
    for y in 0..h {
        let f = filters[y];
        raw[y * (stride + 1)] = f;
        let cur = &data[y * stride..(y + 1) * stride];
        let prev: &[u8] = if y == 0 {
            &[]
        } else {
            &data[(y - 1) * stride..y * stride]
        };
        let mut enc = cur.to_vec();
        if y == 0 {
            match f {
                0 | 2 => {}
                1 => {
                    for x in (bpp..stride).rev() {
                        enc[x] = cur[x].wrapping_sub(cur[x - bpp]);
                    }
                }
                3 => {
                    for x in (bpp..stride).rev() {
                        enc[x] = cur[x].wrapping_sub(cur[x - bpp] / 2);
                    }
                }
                4 => {
                    for x in (bpp..stride).rev() {
                        enc[x] = cur[x].wrapping_sub(paeth(cur[x - bpp], 0, 0));
                    }
                }
                _ => panic!("bad filter"),
            }
        } else {
            match f {
                0 => {}
                1 => {
                    for x in (0..stride).rev() {
                        if x < bpp {
                            enc[x] = cur[x];
                        } else {
                            enc[x] = cur[x].wrapping_sub(cur[x - bpp]);
                        }
                    }
                }
                2 => {
                    for x in 0..stride {
                        enc[x] = cur[x].wrapping_sub(prev[x]);
                    }
                }
                3 => {
                    for x in (0..stride).rev() {
                        if x < bpp {
                            enc[x] = cur[x].wrapping_sub(prev[x] / 2);
                        } else {
                            let s = ((cur[x - bpp] as i32) + (prev[x] as i32)) / 2;
                            enc[x] = cur[x].wrapping_sub(s as u8);
                        }
                    }
                }
                4 => {
                    for x in (0..stride).rev() {
                        if x < bpp {
                            enc[x] = cur[x].wrapping_sub(prev[x]);
                        } else {
                            enc[x] =
                                cur[x].wrapping_sub(paeth(cur[x - bpp], prev[x], prev[x - bpp]));
                        }
                    }
                }
                _ => panic!("bad filter"),
            }
        }
        raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)].copy_from_slice(&enc);
    }
    raw
}

pub fn paeth(a: u8, b: u8, c: u8) -> u8 {
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

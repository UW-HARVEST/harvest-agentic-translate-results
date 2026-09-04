//! Hand-rolled DEFLATE (RFC 1951) writers.
//!
//! Written from scratch rather than taken from a compressor so that every axis
//! in `CONFIGS.md` is directly controllable: block type, code-length depth,
//! which run-length code-length symbols (16/17/18) appear, which length and
//! distance codes are used, and how blocks are chained.

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Bit writer (DEFLATE packs bits LSB-first; Huffman codes go MSB-first)
// ---------------------------------------------------------------------------

pub struct BitWriter {
    buf: Vec<u8>,
    acc: u64,
    n: u32,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            buf: Vec::new(),
            acc: 0,
            n: 0,
        }
    }
    /// `n` low bits of `v`, LSB first (used for header fields and extra bits).
    pub fn bits(&mut self, v: u32, n: u32) {
        assert!(n <= 32);
        if n == 0 {
            return;
        }
        let mask = (1u64 << n) - 1;
        self.acc |= ((v as u64) & mask) << self.n;
        self.n += n;
        while self.n >= 8 {
            self.buf.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// A Huffman code of `n` bits, emitted most-significant-bit first.
    pub fn huff(&mut self, code: u32, n: u32) {
        assert!(n >= 1 && n <= 16, "bad code length {n}");
        let mut rev = 0u32;
        for i in 0..n {
            rev |= ((code >> i) & 1) << (n - 1 - i);
        }
        self.bits(rev, n);
    }
    pub fn align(&mut self) {
        if self.n % 8 != 0 {
            let pad = 8 - self.n % 8;
            self.bits(0, pad);
        }
    }
    pub fn raw_bytes(&mut self, b: &[u8]) {
        assert_eq!(self.n, 0, "must be byte-aligned");
        self.buf.extend_from_slice(b);
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.buf
    }
    pub fn bit_len(&self) -> usize {
        self.buf.len() * 8 + self.n as usize
    }
}

// ---------------------------------------------------------------------------
// RFC 1951 tables (mirrors of cp_len_base / cp_len_extra_bits / cp_dist_*)
// ---------------------------------------------------------------------------

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
pub const CLEN_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

pub fn len_code(len: u32) -> usize {
    assert!((3..=258).contains(&len));
    let mut i = 28;
    while len < LEN_BASE[i] {
        i -= 1;
    }
    i
}

pub fn dist_code(dist: u32) -> usize {
    assert!((1..=32768).contains(&dist));
    let mut i = 29;
    while dist < DIST_BASE[i] {
        i -= 1;
    }
    i
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Tok {
    Lit(u8),
    Match { len: u32, dist: u32 },
}

/// Decode a token list the way `cp_block` does, to get the expected output.
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut o: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => o.push(b),
            Tok::Match { len, dist } => {
                let start = o.len() - dist as usize;
                for k in 0..len as usize {
                    let b = o[start + k];
                    o.push(b);
                }
            }
        }
    }
    o
}

// ---------------------------------------------------------------------------
// Stored blocks (btype = 0)
// ---------------------------------------------------------------------------

/// A single final stored block. `cp_stored` requires `bits_left/8 <= LEN` after
/// its 5-byte header, so this is the only stored shape the C accepts: exactly
/// `LEN + 5` bytes of deflate stream.
pub fn stored_block(data: &[u8], final_block: bool) -> Vec<u8> {
    assert!(data.len() <= 0xFFFF);
    let mut bw = BitWriter::new();
    bw.bits(final_block as u32, 1);
    bw.bits(0, 2);
    bw.align();
    let mut out = bw.finish();
    let len = data.len() as u16;
    out.push((len & 0xFF) as u8);
    out.push((len >> 8) as u8);
    let nlen = !len;
    out.push((nlen & 0xFF) as u8);
    out.push((nlen >> 8) as u8);
    out.extend_from_slice(data);
    out
}

// ---------------------------------------------------------------------------
// Fixed Huffman blocks (btype = 1)
// ---------------------------------------------------------------------------

pub fn fixed_lit(bw: &mut BitWriter, sym: u32) {
    match sym {
        0..=143 => bw.huff(0x30 + sym, 8),
        144..=255 => bw.huff(0x190 + (sym - 144), 9),
        256..=279 => bw.huff(sym - 256, 7),
        280..=287 => bw.huff(0xC0 + (sym - 280), 8),
        _ => panic!("bad symbol {sym}"),
    }
}

pub fn fixed_block(bw: &mut BitWriter, toks: &[Tok], final_block: bool) {
    bw.bits(final_block as u32, 1);
    bw.bits(1, 2);
    for t in toks {
        match *t {
            Tok::Lit(b) => fixed_lit(bw, b as u32),
            Tok::Match { len, dist } => {
                let lc = len_code(len);
                fixed_lit(bw, 257 + lc as u32);
                if LEN_EXTRA[lc] > 0 {
                    bw.bits(len - LEN_BASE[lc], LEN_EXTRA[lc]);
                }
                let dc = dist_code(dist);
                bw.huff(dc as u32, 5);
                if DIST_EXTRA[dc] > 0 {
                    bw.bits(dist - DIST_BASE[dc], DIST_EXTRA[dc]);
                }
            }
        }
    }
    fixed_lit(bw, 256);
}

pub fn fixed_stream(toks: &[Tok]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    fixed_block(&mut bw, toks, true);
    bw.finish()
}

// ---------------------------------------------------------------------------
// Canonical Huffman construction
// ---------------------------------------------------------------------------

/// Canonical codes from code lengths (RFC 1951 §3.2.2).
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let maxlen = *lens.iter().max().unwrap_or(&0) as usize;
    let mut bl_count = vec![0u32; maxlen + 1];
    for &l in lens {
        if l != 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; maxlen + 2];
    let mut code = 0u32;
    for b in 1..=maxlen {
        code = (code + bl_count[b - 1]) << 1;
        next[b] = code;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

/// True if the code described by `lens` is complete (Kraft sum == 1).
pub fn is_complete(lens: &[u8]) -> bool {
    let mut sum: u64 = 0;
    for &l in lens {
        if l != 0 {
            assert!(l <= 15);
            sum += 1u64 << (15 - l);
        }
    }
    sum == 1 << 15
}

/// A code `cp_build` can consume: complete, empty, or the conventional
/// single-symbol-of-length-1 form DEFLATE allows for HDIST == 1.
pub fn code_ok(lens: &[u8]) -> bool {
    let used = lens.iter().filter(|&&l| l != 0).count();
    match used {
        0 => true,
        1 => lens.iter().all(|&l| l == 0 || l == 1),
        _ => is_complete(lens),
    }
}

/// Length-limited Huffman code lengths for `freqs` (0 frequency ⇒ length 0).
/// Frequencies are flattened (`f/2+1`) until the depth fits `limit`, which keeps
/// the result a valid complete code without needing package-merge.
pub fn huff_lengths(freqs: &[u32], limit: u8) -> Vec<u8> {
    let used: Vec<usize> = (0..freqs.len()).filter(|&i| freqs[i] > 0).collect();
    assert!(
        used.len() >= 2,
        "need >= 2 used symbols to build a complete code"
    );
    let mut f: Vec<u32> = freqs.to_vec();
    loop {
        let lens = huff_lengths_raw(&f);
        if lens.iter().all(|&l| l <= limit) {
            let mut out = vec![0u8; freqs.len()];
            for i in 0..freqs.len() {
                if freqs[i] > 0 {
                    out[i] = lens[i];
                }
            }
            assert!(is_complete(&out), "constructed code is not complete");
            return out;
        }
        for x in f.iter_mut() {
            if *x > 0 {
                *x = *x / 2 + 1;
            }
        }
    }
}

fn huff_lengths_raw(freqs: &[u32]) -> Vec<u8> {
    // Node arena: leaves first, then internal nodes.
    let n = freqs.len();
    let mut weight: Vec<u64> = Vec::new();
    let mut left: Vec<i32> = Vec::new();
    let mut right: Vec<i32> = Vec::new();
    let mut live: Vec<usize> = Vec::new();
    for i in 0..n {
        weight.push(freqs[i] as u64);
        left.push(-1);
        right.push(-1);
        if freqs[i] > 0 {
            live.push(i);
        }
    }
    // Simple O(n^2) selection; alphabets here are <= 320 symbols.
    while live.len() > 1 {
        let mut a = 0usize;
        for k in 1..live.len() {
            if (weight[live[k]], live[k]) < (weight[live[a]], live[a]) {
                a = k;
            }
        }
        let ia = live.swap_remove(a);
        let mut b = 0usize;
        for k in 1..live.len() {
            if (weight[live[k]], live[k]) < (weight[live[b]], live[b]) {
                b = k;
            }
        }
        let ib = live.swap_remove(b);
        let idx = weight.len();
        weight.push(weight[ia] + weight[ib]);
        left.push(ia as i32);
        right.push(ib as i32);
        live.push(idx);
    }
    let root = live[0];
    let mut lens = vec![0u8; n];
    let mut stack = vec![(root, 0u8)];
    while let Some((node, d)) = stack.pop() {
        if left[node] < 0 {
            lens[node] = d.max(1);
        } else {
            stack.push((left[node] as usize, d + 1));
            stack.push((right[node] as usize, d + 1));
        }
    }
    lens
}

// ---------------------------------------------------------------------------
// Dynamic Huffman blocks (btype = 2)
// ---------------------------------------------------------------------------

/// One entry of the RLE-encoded code-length sequence.
#[derive(Copy, Clone, Debug)]
pub struct ClenSym {
    pub sym: u8,
    pub extra: u32,
    pub extra_bits: u32,
}

/// RLE-encode the concatenated lit/len + dist code lengths using symbols 16
/// (repeat previous 3–6), 17 (3–10 zeros) and 18 (11–138 zeros).
pub fn rle_code_lengths(all: &[u8], use_rle: bool) -> Vec<ClenSym> {
    let mut out = Vec::new();
    if !use_rle {
        for &l in all {
            out.push(ClenSym {
                sym: l,
                extra: 0,
                extra_bits: 0,
            });
        }
        return out;
    }
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        let mut run = 1usize;
        while i + run < all.len() && all[i + run] == v {
            run += 1;
        }
        if v == 0 {
            while run >= 3 {
                let take = run.min(138);
                if take >= 11 {
                    out.push(ClenSym {
                        sym: 18,
                        extra: (take - 11) as u32,
                        extra_bits: 7,
                    });
                } else {
                    out.push(ClenSym {
                        sym: 17,
                        extra: (take - 3) as u32,
                        extra_bits: 3,
                    });
                }
                run -= take;
                i += take;
            }
            for _ in 0..run {
                out.push(ClenSym {
                    sym: 0,
                    extra: 0,
                    extra_bits: 0,
                });
                i += 1;
            }
        } else {
            out.push(ClenSym {
                sym: v,
                extra: 0,
                extra_bits: 0,
            });
            i += 1;
            run -= 1;
            while run >= 3 {
                let take = run.min(6);
                out.push(ClenSym {
                    sym: 16,
                    extra: (take - 3) as u32,
                    extra_bits: 2,
                });
                run -= take;
                i += take;
            }
            for _ in 0..run {
                out.push(ClenSym {
                    sym: v,
                    extra: 0,
                    extra_bits: 0,
                });
                i += 1;
            }
        }
    }
    out
}

/// Emit a dynamic-Huffman block. `ll` must have 257..=288 entries and `dl`
/// 1..=32; both must describe complete codes.
pub fn dynamic_block(
    bw: &mut BitWriter,
    ll: &[u8],
    dl: &[u8],
    toks: &[Tok],
    final_block: bool,
    use_rle: bool,
) {
    dynamic_block_with_order(bw, ll, dl, toks, final_block, use_rle, &CLEN_ORDER)
}

/// As `dynamic_block`, but the code-length code lengths are transmitted in
/// `order` instead of the RFC's permutation — used to test a mutated
/// `cp_permutation_order` export.
pub fn dynamic_block_with_order(
    bw: &mut BitWriter,
    ll: &[u8],
    dl: &[u8],
    toks: &[Tok],
    final_block: bool,
    use_rle: bool,
    order: &[usize; 19],
) {
    assert!((257..=288).contains(&ll.len()), "hlit out of range");
    assert!((1..=32).contains(&dl.len()), "hdist out of range");
    assert!(is_complete(ll), "lit/len code incomplete");
    assert!(code_ok(dl), "dist code not usable");

    let mut all: Vec<u8> = Vec::new();
    all.extend_from_slice(ll);
    all.extend_from_slice(dl);
    let rle = rle_code_lengths(&all, use_rle);

    let mut cfreq = [0u32; 19];
    for e in &rle {
        cfreq[e.sym as usize] += 1;
    }
    // huff_lengths needs >= 2 distinct symbols in the code-length alphabet.
    if cfreq.iter().filter(|&&f| f > 0).count() < 2 {
        cfreq[if cfreq[0] > 0 { 1 } else { 0 }] += 1;
    }
    let clens = huff_lengths(&cfreq, 7);
    let ccodes = canonical(&clens);

    let mut hclen = 19usize;
    while hclen > 4 && clens[order[hclen - 1]] == 0 {
        hclen -= 1;
    }

    bw.bits(final_block as u32, 1);
    bw.bits(2, 2);
    bw.bits((ll.len() - 257) as u32, 5);
    bw.bits((dl.len() - 1) as u32, 5);
    bw.bits((hclen - 4) as u32, 4);
    for i in 0..hclen {
        bw.bits(clens[order[i]] as u32, 3);
    }
    for e in &rle {
        let s = e.sym as usize;
        bw.huff(ccodes[s], clens[s] as u32);
        if e.extra_bits > 0 {
            bw.bits(e.extra, e.extra_bits);
        }
    }

    let lcodes = canonical(ll);
    let dcodes = canonical(dl);
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let s = b as usize;
                assert!(ll[s] != 0, "literal {s} has no code");
                bw.huff(lcodes[s], ll[s] as u32);
            }
            Tok::Match { len, dist } => {
                let lc = 257 + len_code(len);
                assert!(lc < ll.len() && ll[lc] != 0, "length code {lc} has no code");
                bw.huff(lcodes[lc], ll[lc] as u32);
                let le = LEN_EXTRA[lc - 257];
                if le > 0 {
                    bw.bits(len - LEN_BASE[lc - 257], le);
                }
                let dc = dist_code(dist);
                assert!(dc < dl.len() && dl[dc] != 0, "dist code {dc} has no code");
                bw.huff(dcodes[dc], dl[dc] as u32);
                let de = DIST_EXTRA[dc];
                if de > 0 {
                    bw.bits(dist - DIST_BASE[dc], de);
                }
            }
        }
    }
    assert!(ll[256] != 0, "end-of-block has no code");
    bw.huff(lcodes[256], ll[256] as u32);
}

/// Build lit/len and dist code lengths that cover exactly the symbols used by
/// `toks` (plus end-of-block), with `depth_limit` bounding the tree depth.
pub fn tables_for(toks: &[Tok], depth_limit: u8, hlit: usize, hdist: usize) -> (Vec<u8>, Vec<u8>) {
    let hlit = hlit.clamp(257, 288);
    let hdist = hdist.clamp(1, 32);
    let mut lf = vec![0u32; hlit];
    let mut df = vec![0u32; hdist];
    lf[256] = 1;
    for t in toks {
        match *t {
            Tok::Lit(b) => lf[b as usize] += 1,
            Tok::Match { len, dist } => {
                lf[257 + len_code(len)] += 1;
                df[dist_code(dist)] += 1;
            }
        }
    }
    // A one-symbol code cannot be complete; add a filler.
    if lf.iter().filter(|&&f| f > 0).count() < 2 {
        lf[0] += 1;
    }
    let ll = huff_lengths(&lf, depth_limit);

    let dused = df.iter().filter(|&&f| f > 0).count();
    let dl = if hdist == 1 {
        // HDIST == 1 can only ever be the single-code-of-length-1 form, which
        // cp_decode handles (it asserts the leading bit is 0).
        vec![if dused > 0 { 1u8 } else { 0u8 }]
    } else if dused < 2 {
        // Pad with a filler symbol so the code is complete.
        let mut f = df.clone();
        f[0] = f[0].max(1);
        f[1] = f[1].max(1);
        huff_lengths(&f, depth_limit)
    } else {
        huff_lengths(&df, depth_limit)
    };
    (ll, dl)
}

pub fn dynamic_stream(toks: &[Tok], depth_limit: u8, hlit: usize, hdist: usize, rle: bool) -> Vec<u8> {
    let (ll, dl) = tables_for(toks, depth_limit, hlit, hdist);
    let mut bw = BitWriter::new();
    dynamic_block(&mut bw, &ll, &dl, toks, true, rle);
    bw.finish()
}

// ---------------------------------------------------------------------------
// zlib wrapper
// ---------------------------------------------------------------------------

pub fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// `cp_inflate` is handed `data + 2 .. data + datalen - 4`, so the wrapper must
/// be exactly 2 header bytes + 4 trailer bytes.
pub fn zlib_wrap(deflate: &[u8], raw: &[u8], cmf: u8, flg: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(deflate.len() + 6);
    v.push(cmf);
    v.push(flg);
    v.extend_from_slice(deflate);
    v.extend_from_slice(&adler32(raw).to_be_bytes());
    v
}

pub fn zlib(deflate: &[u8], raw: &[u8]) -> Vec<u8> {
    zlib_wrap(deflate, raw, 0x78, 0x9C)
}

// ---------------------------------------------------------------------------
// LZ77 tokenizer (hash-chain greedy matcher) — produces token streams whose
// `expand()` is exactly the input, so back-reference paths in `cp_block`
// (including the `distance == 1` memset special case) get exercised.
// ---------------------------------------------------------------------------

pub fn lz77(data: &[u8], max_dist: usize, chain_limit: usize) -> Vec<Tok> {
    const HBITS: usize = 15;
    const HSIZE: usize = 1 << HBITS;
    let mut head = vec![usize::MAX; HSIZE];
    let mut prev = vec![usize::MAX; data.len().max(1)];
    let h3 = |d: &[u8], i: usize| -> usize {
        if i + 2 >= d.len() {
            return 0;
        }
        (((d[i] as usize) << 10) ^ ((d[i + 1] as usize) << 5) ^ (d[i + 2] as usize)) & (HSIZE - 1)
    };

    let mut toks = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if i + 3 <= data.len() {
            let hh = h3(data, i);
            let mut cand = head[hh];
            let mut tries = 0usize;
            while cand != usize::MAX && tries < chain_limit {
                if i - cand > max_dist {
                    break;
                }
                let maxl = (data.len() - i).min(258);
                let mut l = 0usize;
                while l < maxl && data[cand + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - cand;
                    if l == maxl {
                        break;
                    }
                }
                cand = prev[cand];
                tries += 1;
            }
        }
        let advance = if best_len >= 3 { best_len } else { 1 };
        for k in 0..advance {
            let p = i + k;
            if p + 3 <= data.len() {
                let hh = h3(data, p);
                prev[p] = head[hh];
                head[hh] = p;
            }
        }
        if best_len >= 3 {
            toks.push(Tok::Match {
                len: best_len as u32,
                dist: best_dist as u32,
            });
        } else {
            toks.push(Tok::Lit(data[i]));
        }
        i += advance;
    }
    assert_eq!(expand(&toks), data, "lz77 round-trip broken");
    toks
}

/// Complete lit/len + dist code lengths covering the whole alphabet, so any
/// token stream can be encoded. Depth is bounded by `depth_limit`.
pub fn full_tables(depth_limit: u8) -> (Vec<u8>, Vec<u8>) {
    let lf = vec![1u32; 288];
    let df = vec![1u32; 32];
    (huff_lengths(&lf, depth_limit), huff_lengths(&df, depth_limit))
}

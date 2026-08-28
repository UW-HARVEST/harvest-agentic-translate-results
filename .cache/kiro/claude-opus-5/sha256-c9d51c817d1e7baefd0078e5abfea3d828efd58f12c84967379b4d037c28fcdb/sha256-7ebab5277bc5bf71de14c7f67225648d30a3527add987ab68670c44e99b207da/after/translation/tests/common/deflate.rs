//! Minimal DEFLATE bit-stream writer used to hand-craft inputs for
//! `cp_inflate` with exact control over block type, symbols, lengths and
//! back-distances.

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

pub struct BitWriter {
    buf: Vec<u8>,
    cur: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            buf: Vec::new(),
            cur: 0,
            nbits: 0,
        }
    }

    fn bit(&mut self, b: u32) {
        self.cur |= (b & 1) << self.nbits;
        self.nbits += 1;
        if self.nbits == 8 {
            self.buf.push(self.cur as u8);
            self.cur = 0;
            self.nbits = 0;
        }
    }

    /// DEFLATE "extra bits" and header fields: least-significant bit first.
    pub fn lsb(&mut self, val: u32, n: u32) {
        for i in 0..n {
            self.bit(val >> i);
        }
    }

    /// Huffman codes: most-significant bit of the code first.
    pub fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bit(code >> i);
        }
    }

    pub fn align(&mut self) {
        while self.nbits != 0 {
            self.bit(0);
        }
    }

    pub fn raw_byte(&mut self, b: u8) {
        assert_eq!(self.nbits, 0, "raw_byte requires byte alignment");
        self.buf.push(b);
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        if self.buf.is_empty() {
            self.buf.push(0);
        }
        self.buf
    }

    pub fn block_header(&mut self, bfinal: u32, btype: u32) {
        self.lsb(bfinal, 1);
        self.lsb(btype, 2);
    }

    // --- fixed (btype 1) Huffman code emission -----------------------------

    pub fn fixed_symbol(&mut self, sym: u32) {
        match sym {
            0..=143 => self.code(0x30 + sym, 8),
            144..=255 => self.code(0x190 + (sym - 144), 9),
            256..=279 => self.code(sym - 256, 7),
            280..=287 => self.code(0xC0 + (sym - 280), 8),
            _ => panic!("bad literal/length symbol {sym}"),
        }
    }

    pub fn fixed_literal(&mut self, byte: u8) {
        self.fixed_symbol(byte as u32);
    }

    pub fn fixed_end_of_block(&mut self) {
        self.fixed_symbol(256);
    }

    pub fn fixed_match(&mut self, length: u32, distance: u32) {
        assert!((3..=258).contains(&length));
        assert!((1..=32768).contains(&distance));
        let li = (0..29)
            .rev()
            .find(|&i| LEN_BASE[i] <= length)
            .expect("length symbol");
        assert!(length - LEN_BASE[li] < (1u32 << LEN_EXTRA[li]) || LEN_EXTRA[li] == 0);
        self.fixed_symbol(257 + li as u32);
        self.lsb(length - LEN_BASE[li], LEN_EXTRA[li]);

        let di = (0..30)
            .rev()
            .find(|&i| DIST_BASE[i] <= distance)
            .expect("distance symbol");
        // fixed distance tree: 5-bit codes, code value == symbol
        self.code(di as u32, 5);
        self.lsb(distance - DIST_BASE[di], DIST_EXTRA[di]);
    }
}

/// Reference implementation of what a DEFLATE decoder should produce for a
/// stream built out of these primitives.
#[derive(Clone, Debug)]
pub enum Item {
    Lit(u8),
    Match { length: u32, distance: u32 },
}

/// Emit one fixed-Huffman block and return the bytes it decodes to.
pub fn fixed_block(w: &mut BitWriter, bfinal: bool, items: &[Item], expect: &mut Vec<u8>) {
    w.block_header(bfinal as u32, 1);
    for it in items {
        match *it {
            Item::Lit(b) => {
                w.fixed_literal(b);
                expect.push(b);
            }
            Item::Match { length, distance } => {
                w.fixed_match(length, distance);
                let start = expect.len() - distance as usize;
                for k in 0..length as usize {
                    let b = expect[start + k];
                    expect.push(b);
                }
            }
        }
    }
    w.fixed_end_of_block();
}

/// Emit a stored (btype 0) block. `len`/`nlen` are written verbatim so callers
/// can build malformed pairs on purpose.
pub fn stored_block_raw(w: &mut BitWriter, bfinal: bool, len: u16, nlen: u16, data: &[u8]) {
    w.block_header(bfinal as u32, 0);
    w.align();
    w.raw_byte((len & 0xff) as u8);
    w.raw_byte((len >> 8) as u8);
    w.raw_byte((nlen & 0xff) as u8);
    w.raw_byte((nlen >> 8) as u8);
    for &b in data {
        w.raw_byte(b);
    }
}

pub fn stored_block(w: &mut BitWriter, bfinal: bool, data: &[u8], expect: &mut Vec<u8>) {
    let len = data.len() as u16;
    stored_block_raw(w, bfinal, len, !len, data);
    expect.extend_from_slice(data);
}

// ---------------------------------------------------------------------------
// btype 2 -- dynamic Huffman
// ---------------------------------------------------------------------------

pub const PERMUTATION: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Huffman code lengths for `freq`, limited to `max_len` bits.
/// Symbols with zero frequency get length 0.
fn huffman_lengths(freq: &[u32], max_len: u8) -> Vec<u8> {
    let n = freq.len();
    let mut work: Vec<u32> = freq.to_vec();
    for _ in 0..64 {
        let lens = huffman_lengths_unbounded(&work);
        if lens.iter().all(|&l| l <= max_len) {
            // Re-derive from the *original* nonzero set: scaling only changes
            // depths, never which symbols are used.
            let mut out = vec![0u8; n];
            for i in 0..n {
                if freq[i] != 0 {
                    out[i] = lens[i];
                }
            }
            return out;
        }
        for f in work.iter_mut() {
            if *f != 0 {
                // Halve, but never reach zero: a symbol must keep a code.
                *f = (*f >> 1).max(1);
            }
        }
    }
    panic!("could not length-limit Huffman code to {max_len} bits");
}

fn huffman_lengths_unbounded(freq: &[u32]) -> Vec<u8> {
    let n = freq.len();
    let used: Vec<usize> = (0..n).filter(|&i| freq[i] != 0).collect();
    let mut lens = vec![0u8; n];
    match used.len() {
        0 => return lens,
        1 => {
            lens[used[0]] = 1;
            return lens;
        }
        _ => {}
    }
    // Node = (weight, depth-tracking set of symbols)
    #[derive(Clone)]
    struct Node {
        w: u64,
        syms: Vec<usize>,
    }
    let mut nodes: Vec<Node> = used
        .iter()
        .map(|&s| Node {
            w: freq[s] as u64,
            syms: vec![s],
        })
        .collect();
    while nodes.len() > 1 {
        nodes.sort_by(|a, b| a.w.cmp(&b.w).then(a.syms[0].cmp(&b.syms[0])));
        let a = nodes.remove(0);
        let b = nodes.remove(0);
        for &s in a.syms.iter().chain(b.syms.iter()) {
            lens[s] += 1;
        }
        let mut syms = a.syms;
        syms.extend(b.syms);
        syms.sort();
        nodes.push(Node { w: a.w + b.w, syms });
    }
    lens
}

/// Canonical DEFLATE codes for the given lengths: `codes[sym]` is the code
/// value, to be emitted most-significant-bit first over `lens[sym]` bits.
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let maxbits = *lens.iter().max().unwrap_or(&0) as usize;
    let mut bl_count = vec![0u32; maxbits + 1];
    for &l in lens {
        if l != 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next_code = vec![0u32; maxbits + 2];
    let mut code = 0u32;
    for bits in 1..=maxbits {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    let mut codes = vec![0u32; lens.len()];
    for (sym, &l) in lens.iter().enumerate() {
        if l != 0 {
            codes[sym] = next_code[l as usize];
            next_code[l as usize] += 1;
        }
    }
    codes
}

fn len_symbol(length: u32) -> (usize, u32) {
    let i = (0..29).rev().find(|&i| LEN_BASE[i] <= length).unwrap();
    (i, length - LEN_BASE[i])
}

fn dist_symbol(distance: u32) -> (usize, u32) {
    let i = (0..30).rev().find(|&i| DIST_BASE[i] <= distance).unwrap();
    (i, distance - DIST_BASE[i])
}

/// How the code-length sequence is packed.
#[derive(Clone, Copy, PartialEq)]
pub enum ClMode {
    /// Emit every length as its own symbol (no 16/17/18).
    Literal,
    /// Use 16 (copy previous), 17 (short zero run) and 18 (long zero run).
    RunLength,
}

pub struct DynOpts {
    pub cl_mode: ClMode,
    /// Pad `HCLEN` up to this many entries (4..=19).
    pub min_nlen: usize,
    /// Emit `HDIST == 1` with a single 1-bit distance code (what zlib does for
    /// literal-only dynamic blocks). Only valid when `items` has no matches.
    pub single_dist: bool,
    /// Skew literal frequencies so the lit/len tree reaches deep code lengths.
    pub deep_tree: bool,
}

impl Default for DynOpts {
    fn default() -> Self {
        DynOpts {
            cl_mode: ClMode::RunLength,
            min_nlen: 4,
            single_dist: false,
            deep_tree: false,
        }
    }
}

/// Emit one dynamic-Huffman block; returns the bytes it decodes to (appended
/// to `expect`).
pub fn dynamic_block(
    w: &mut BitWriter,
    bfinal: bool,
    items: &[Item],
    expect: &mut Vec<u8>,
    opts: &DynOpts,
) {
    // 1. symbol frequencies
    let mut litfreq = vec![0u32; 288];
    let mut dstfreq = vec![0u32; 30];
    litfreq[256] = 1; // end of block
    for it in items {
        match *it {
            Item::Lit(b) => litfreq[b as usize] += 1,
            Item::Match { length, distance } => {
                let (li, _) = len_symbol(length);
                let (di, _) = dist_symbol(distance);
                litfreq[257 + li] += 1;
                dstfreq[di] += 1;
            }
        }
    }
    // DEFLATE needs at least one distance code.
    if dstfreq.iter().all(|&f| f == 0) {
        dstfreq[0] = 1;
    }
    // A one-symbol tree is legal but exercises a degenerate path in the
    // decoder's binary search; keep two so the stream stays unambiguous.
    if !opts.single_dist && dstfreq.iter().filter(|&&f| f != 0).count() == 1 {
        let i = dstfreq.iter().position(|&f| f != 0).unwrap();
        dstfreq[(i + 1) % 30] = 1;
    }
    if opts.single_dist {
        assert!(
            items.iter().all(|it| matches!(it, Item::Lit(_))),
            "single_dist requires a literal-only block"
        );
        dstfreq = vec![0u32; 30];
        dstfreq[0] = 1;
    }
    if litfreq.iter().filter(|&&f| f != 0).count() == 1 {
        litfreq[0] += 1;
    }
    if opts.deep_tree {
        // Fibonacci weights force a maximally unbalanced (depth-15) tree.
        let used: Vec<usize> = (0..288).filter(|&i| litfreq[i] != 0).collect();
        let (mut a, mut b) = (1u32, 1u32);
        for &s in used.iter() {
            litfreq[s] = a;
            // Cap the growth: saturating at u32::MAX would flatten the tree
            // again once the length limiter starts halving frequencies.
            let n = (a + b).min(1 << 22);
            a = b;
            b = n;
        }
    }

    let mut litlens = huffman_lengths(&litfreq, 15);
    let mut dstlens = huffman_lengths(&dstfreq, 15);

    // 2. HLIT / HDIST: trim trailing zero lengths
    let mut nlit = 288;
    while nlit > 257 && litlens[nlit - 1] == 0 {
        nlit -= 1;
    }
    let mut ndst = 30;
    while ndst > 1 && dstlens[ndst - 1] == 0 {
        ndst -= 1;
    }
    litlens.truncate(nlit);
    dstlens.truncate(ndst);

    let mut all: Vec<u8> = Vec::with_capacity(nlit + ndst);
    all.extend_from_slice(&litlens);
    all.extend_from_slice(&dstlens);

    // 3. pack the code-length sequence
    #[derive(Clone, Copy)]
    struct Cl {
        sym: usize,
        extra_bits: u32,
        extra_val: u32,
    }
    let mut seq: Vec<Cl> = Vec::new();
    match opts.cl_mode {
        ClMode::Literal => {
            for &l in &all {
                seq.push(Cl {
                    sym: l as usize,
                    extra_bits: 0,
                    extra_val: 0,
                });
            }
        }
        ClMode::RunLength => {
            let mut i = 0usize;
            while i < all.len() {
                let v = all[i];
                let mut run = 1usize;
                while i + run < all.len() && all[i + run] == v {
                    run += 1;
                }
                if v == 0 {
                    let mut left = run;
                    while left >= 11 {
                        let take = left.min(138);
                        seq.push(Cl {
                            sym: 18,
                            extra_bits: 7,
                            extra_val: (take - 11) as u32,
                        });
                        left -= take;
                    }
                    while left >= 3 {
                        let take = left.min(10);
                        seq.push(Cl {
                            sym: 17,
                            extra_bits: 3,
                            extra_val: (take - 3) as u32,
                        });
                        left -= take;
                    }
                    for _ in 0..left {
                        seq.push(Cl {
                            sym: 0,
                            extra_bits: 0,
                            extra_val: 0,
                        });
                    }
                } else {
                    // first occurrence literally, then 16-runs of the rest
                    seq.push(Cl {
                        sym: v as usize,
                        extra_bits: 0,
                        extra_val: 0,
                    });
                    let mut left = run - 1;
                    while left >= 3 {
                        let take = left.min(6);
                        seq.push(Cl {
                            sym: 16,
                            extra_bits: 2,
                            extra_val: (take - 3) as u32,
                        });
                        left -= take;
                    }
                    for _ in 0..left {
                        seq.push(Cl {
                            sym: v as usize,
                            extra_bits: 0,
                            extra_val: 0,
                        });
                    }
                }
                i += run;
            }
        }
    }

    // 4. Huffman code for the code-length alphabet (max 7 bits)
    let mut clfreq = vec![0u32; 19];
    for c in &seq {
        clfreq[c.sym] += 1;
    }
    if clfreq.iter().filter(|&&f| f != 0).count() < 2 {
        // keep the tree non-degenerate
        for s in 0..19 {
            if clfreq[s] == 0 {
                clfreq[s] = 1;
                break;
            }
        }
    }
    let cllens = huffman_lengths(&clfreq, 7);
    let clcodes = canonical_codes(&cllens);

    let mut nlen = 19;
    while nlen > opts.min_nlen.max(4) && cllens[PERMUTATION[nlen - 1]] == 0 {
        nlen -= 1;
    }

    // 5. block header
    w.block_header(bfinal as u32, 2);
    w.lsb((nlit - 257) as u32, 5);
    w.lsb((ndst - 1) as u32, 5);
    w.lsb((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.lsb(cllens[PERMUTATION[i]] as u32, 3);
    }

    // 6. the packed code-length sequence
    for c in &seq {
        w.code(clcodes[c.sym], cllens[c.sym] as u32);
        if c.extra_bits != 0 {
            w.lsb(c.extra_val, c.extra_bits);
        }
    }

    // 7. the data
    let litcodes = canonical_codes(&litlens);
    let dstcodes = canonical_codes(&dstlens);
    for it in items {
        match *it {
            Item::Lit(b) => {
                let s = b as usize;
                assert!(litlens[s] != 0, "literal {s} has no code");
                w.code(litcodes[s], litlens[s] as u32);
                expect.push(b);
            }
            Item::Match { length, distance } => {
                let (li, lextra) = len_symbol(length);
                w.code(litcodes[257 + li], litlens[257 + li] as u32);
                w.lsb(lextra, LEN_EXTRA[li]);
                let (di, dextra) = dist_symbol(distance);
                assert!(dstlens[di] != 0, "distance symbol {di} has no code");
                w.code(dstcodes[di], dstlens[di] as u32);
                w.lsb(dextra, DIST_EXTRA[di]);
                let start = expect.len() - distance as usize;
                for k in 0..length as usize {
                    let b = expect[start + k];
                    expect.push(b);
                }
            }
        }
    }
    w.code(litcodes[256], litlens[256] as u32);
}

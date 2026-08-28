//! A DEFLATE *encoder* built directly from `c_src/src/lib.c`, so that the tests
//! can drive every branch of the decoder on purpose.
//!
//! It is deliberately low level: the caller picks the literal/length/distance
//! *symbols* and their extra-bit payloads, the Huffman code lengths, `HLIT`,
//! `HDIST`, `HCLEN`, and which code-length repeat codes (16/17/18) to use.  The
//! canonical code assignment mirrors `cp_build` exactly.

#![allow(dead_code)]

use super::Rng;

// ---------------------------------------------------------------------------
// bit writer (DEFLATE bit order: LSB first, Huffman codes MSB first)
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub bytes: Vec<u8>,
    pub nbits: usize,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter { bytes: Vec::new(), nbits: 0 }
    }
    pub fn bit_pos(&self) -> usize {
        self.nbits
    }
    pub fn bits(&mut self, val: u32, n: usize) {
        for i in 0..n {
            let b = ((val >> i) & 1) as u8;
            if self.nbits % 8 == 0 {
                self.bytes.push(0);
            }
            let idx = self.nbits / 8;
            self.bytes[idx] |= b << (self.nbits % 8);
            self.nbits += 1;
        }
    }
    /// Huffman code: the most significant bit of `code` goes out first.
    pub fn huff(&mut self, code: u32, len: usize) {
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
    pub fn align_byte(&mut self) {
        while self.nbits % 8 != 0 {
            self.bits(0, 1);
        }
    }
    pub fn push_bytes(&mut self, data: &[u8]) {
        assert_eq!(self.nbits % 8, 0, "push_bytes requires byte alignment");
        self.bytes.extend_from_slice(data);
        self.nbits += 8 * data.len();
    }
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}

// ---------------------------------------------------------------------------
// canonical Huffman codes — same assignment as cp_build()
// ---------------------------------------------------------------------------

/// Returns the canonical code for every symbol (0 where `lens[i] == 0`).
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0i32; 16];
    for &l in lens {
        assert!(l < 16, "code length {l} >= 16");
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut codes = [0i32; 16];
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
    }
    let mut out = vec![0u32; lens.len()];
    for i in 0..lens.len() {
        let l = lens[i] as usize;
        if l != 0 {
            out[i] = codes[l] as u32;
            codes[l] += 1;
        }
    }
    out
}

/// `cp_build`'s return value (`first[15]`): the number of symbols with a
/// non-zero code length.
pub fn max_index(lens: &[u8]) -> i32 {
    lens.iter().filter(|&&l| l != 0).count() as i32
}

pub fn fixed_lit_lens() -> Vec<u8> {
    (0..288)
        .map(|i| match i {
            0..=143 => 8u8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        })
        .collect()
}

pub fn fixed_dist_lens() -> Vec<u8> {
    vec![5u8; 32]
}

// ---------------------------------------------------------------------------
// the decoder's length/distance tables (patchable, mirroring the globals)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Tables {
    pub len_extra: [u8; 31],
    pub len_base: [u32; 31],
    pub dist_extra: [u8; 32],
    pub dist_base: [u32; 32],
}

impl Default for Tables {
    fn default() -> Tables {
        Tables {
            len_extra: [
                0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5,
                5, 0, 0, 0,
            ],
            len_base: [
                3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83,
                99, 115, 131, 163, 195, 227, 258, 0, 0,
            ],
            dist_extra: [
                0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11,
                12, 12, 13, 13, 0, 0,
            ],
            dist_base: [
                1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769,
                1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
            ],
        }
    }
}

impl Tables {
    pub fn length_of(&self, ls: u16, lx: u32) -> u32 {
        self.len_base[(ls - 257) as usize].wrapping_add(lx)
    }
    pub fn distance_of(&self, ds: u16, dx: u32) -> u32 {
        self.dist_base[ds as usize].wrapping_add(dx)
    }
}

// ---------------------------------------------------------------------------
// tokens
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    /// literal symbol (< 256)
    Lit(u16),
    /// length symbol + extra payload, distance symbol + extra payload
    Match { ls: u16, lx: u32, ds: u16, dx: u32 },
    /// symbol 256
    End,
}

// ---------------------------------------------------------------------------
// dynamic-block description
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum ClOp {
    /// a code length 0..15 emitted directly
    Lit(u8),
    /// symbol 16: repeat the previous length 3 + extra times (extra in 0..3)
    Rep16(u32),
    /// symbol 17: 3 + extra zeros (extra in 0..7)
    Rep17(u32),
    /// symbol 18: 11 + extra zeros (extra in 0..127)
    Rep18(u32),
}

impl ClOp {
    pub fn sym(self) -> usize {
        match self {
            ClOp::Lit(l) => l as usize,
            ClOp::Rep16(_) => 16,
            ClOp::Rep17(_) => 17,
            ClOp::Rep18(_) => 18,
        }
    }
    pub fn count(self) -> usize {
        match self {
            ClOp::Lit(_) => 1,
            ClOp::Rep16(e) => 3 + e as usize,
            ClOp::Rep17(e) => 3 + e as usize,
            ClOp::Rep18(e) => 11 + e as usize,
        }
    }
}

pub const PERM: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

#[derive(Clone, Debug)]
pub struct DynSpec {
    pub hlit: usize,
    pub hdist: usize,
    pub hclen: usize,
    pub cl_lens: [u8; 19],
    pub ops: Vec<ClOp>,
    pub lit_lens: Vec<u8>,
    pub dist_lens: Vec<u8>,
}

/// Expands a list of code-length ops the way `cp_dynamic` does.
pub fn expand_ops(ops: &[ClOp]) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    for op in ops {
        match *op {
            ClOp::Lit(l) => v.push(l),
            ClOp::Rep16(e) => {
                let prev = *v.last().expect("Rep16 with no previous length");
                for _ in 0..(3 + e) {
                    v.push(prev);
                }
            }
            ClOp::Rep17(e) => {
                for _ in 0..(3 + e) {
                    v.push(0);
                }
            }
            ClOp::Rep18(e) => {
                for _ in 0..(11 + e) {
                    v.push(0);
                }
            }
        }
    }
    v
}

/// The minimal `HCLEN` that can still describe every used code-length symbol.
pub fn hclen_for(cl_lens: &[u8; 19], perm: &[u8; 19]) -> usize {
    let mut need = 4usize;
    for i in 0..19 {
        if cl_lens[perm[i] as usize] != 0 {
            need = need.max(i + 1);
        }
    }
    need
}

/// Direct (no repeat codes) encoding of a code-length vector.
pub fn ops_direct(all: &[u8]) -> Vec<ClOp> {
    all.iter().map(|&l| ClOp::Lit(l)).collect()
}

/// RLE encoding using only the repeat codes the caller allows.
pub fn ops_rle(all: &[u8], use16: bool, use17: bool, use18: bool) -> Vec<ClOp> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        let mut run = 1usize;
        while i + run < all.len() && all[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let mut left = run;
            while left > 0 {
                if use18 && left >= 11 {
                    let n = left.min(138);
                    ops.push(ClOp::Rep18((n - 11) as u32));
                    left -= n;
                } else if use17 && left >= 3 {
                    let n = left.min(10);
                    ops.push(ClOp::Rep17((n - 3) as u32));
                    left -= n;
                } else {
                    ops.push(ClOp::Lit(0));
                    left -= 1;
                }
            }
        } else {
            ops.push(ClOp::Lit(v));
            let mut left = run - 1;
            while left > 0 {
                if use16 && left >= 3 {
                    let n = left.min(6);
                    ops.push(ClOp::Rep16((n - 3) as u32));
                    left -= n;
                } else {
                    ops.push(ClOp::Lit(v));
                    left -= 1;
                }
            }
        }
        i += run;
    }
    ops
}

/// A complete canonical code over `k` leaves: `2^L - k` leaves at depth `L-1`
/// and `2*k - 2^L` at depth `L` (Kraft sum exactly 1).
pub fn balanced_lengths(k: usize) -> Vec<u8> {
    assert!(k >= 1);
    if k == 1 {
        return vec![1];
    }
    let mut l = 1usize;
    while (1usize << l) < k {
        l += 1;
    }
    let p = (1usize << l) - k;
    let mut v = vec![(l - 1) as u8; p];
    v.extend(std::iter::repeat(l as u8).take(k - p));
    v
}

/// A random *complete* set of `k` code lengths, all `<= max_len`.
pub fn random_lengths(rng: &mut Rng, k: usize, max_len: u8) -> Vec<u8> {
    assert!(k >= 1);
    if k == 1 {
        return vec![1];
    }
    let mut slots: Vec<u8> = vec![0];
    while slots.len() < k {
        let cands: Vec<usize> = (0..slots.len()).filter(|&i| slots[i] + 1 <= max_len).collect();
        if cands.is_empty() {
            return balanced_lengths(k);
        }
        let pick = cands[rng.below(cands.len())];
        let d = slots.swap_remove(pick);
        slots.push(d + 1);
        slots.push(d + 1);
    }
    assert!(slots.iter().all(|&d| d >= 1));
    slots
}

/// Build a `DynSpec` from the chosen literal/distance symbol sets.
///
/// `lit_syms` must contain 256 (end of block).  Code lengths form a complete
/// canonical code; `hlit`/`hdist` are the smallest legal values covering the
/// chosen symbols, or the caller-provided minimum.
#[allow(clippy::too_many_arguments)]
pub fn dyn_spec(
    rng: &mut Rng,
    lit_syms: &[u16],
    dist_syms: &[u16],
    min_hlit: usize,
    min_hdist: usize,
    max_len: u8,
    rle: (bool, bool, bool),
    balanced: bool,
) -> DynSpec {
    assert!(lit_syms.contains(&256));
    let mut lit_sorted: Vec<u16> = lit_syms.to_vec();
    lit_sorted.sort_unstable();
    lit_sorted.dedup();
    let mut dist_sorted: Vec<u16> = dist_syms.to_vec();
    dist_sorted.sort_unstable();
    dist_sorted.dedup();

    let hlit = min_hlit.max(257).max(lit_sorted.iter().map(|&s| s as usize + 1).max().unwrap());
    let hdist =
        min_hdist.max(1).max(dist_sorted.iter().map(|&s| s as usize + 1).max().unwrap_or(1));
    assert!(hlit <= 288 && hdist <= 32);

    let ll = if balanced {
        balanced_lengths(lit_sorted.len())
    } else {
        random_lengths(rng, lit_sorted.len(), max_len)
    };
    let dl = if dist_sorted.is_empty() {
        Vec::new()
    } else if balanced {
        balanced_lengths(dist_sorted.len())
    } else {
        random_lengths(rng, dist_sorted.len(), max_len)
    };

    let mut lit_lens = vec![0u8; hlit];
    for (i, &s) in lit_sorted.iter().enumerate() {
        lit_lens[s as usize] = ll[i];
    }
    let mut dist_lens = vec![0u8; hdist];
    for (i, &s) in dist_sorted.iter().enumerate() {
        dist_lens[s as usize] = dl[i];
    }

    let mut all = lit_lens.clone();
    all.extend_from_slice(&dist_lens);
    let ops =
        if rle == (false, false, false) { ops_direct(&all) } else { ops_rle(&all, rle.0, rle.1, rle.2) };
    debug_assert_eq!(expand_ops(&ops), all);

    let mut used: Vec<usize> = Vec::new();
    for op in &ops {
        let s = op.sym();
        if !used.contains(&s) {
            used.push(s);
        }
    }
    used.sort_unstable();
    let cl_depths =
        if balanced { balanced_lengths(used.len()) } else { random_lengths(rng, used.len(), 7) };
    let mut cl_lens = [0u8; 19];
    for (i, &s) in used.iter().enumerate() {
        cl_lens[s] = cl_depths[i];
    }
    let hclen = hclen_for(&cl_lens, &PERM);

    DynSpec { hlit, hdist, hclen, cl_lens, ops, lit_lens, dist_lens }
}

// ---------------------------------------------------------------------------
// stream builder
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Seg {
    Bytes(Vec<u8>),
    /// `input[data_off .. data_off+len]`, `len` resolved at finish time
    StoredTail { data_off: usize, len: Option<usize> },
}

#[derive(Clone, Debug)]
struct StoredInfo {
    len_field: usize,
    data_off: usize,
    seg: Option<usize>,
    override_len: Option<u16>,
}

pub struct Stream {
    pub bw: BitWriter,
    segs: Vec<Seg>,
    stored: Vec<StoredInfo>,
    /// running output, used to expand matches
    pub out: Vec<u8>,
    /// `out` is byte-exact
    pub known: bool,
    /// `out.len()` still equals the real number of bytes produced so far
    pub pos_known: bool,
    /// the stream deliberately errors out; later tokens produce nothing
    pub errored: bool,
}

impl Stream {
    pub fn new() -> Stream {
        Stream {
            bw: BitWriter::new(),
            segs: Vec::new(),
            stored: Vec::new(),
            out: Vec::new(),
            known: true,
            pos_known: true,
            errored: false,
        }
    }

    fn emit_tokens(
        &mut self,
        toks: &[Tok],
        lit_lens: &[u8],
        lit_codes: &[u32],
        dist_lens: &[u8],
        dist_codes: &[u32],
        t: &Tables,
    ) {
        for tok in toks {
            match *tok {
                Tok::Lit(s) => {
                    let s = s as usize;
                    assert!(s < 256, "Tok::Lit is for symbols < 256");
                    assert!(lit_lens[s] != 0, "literal symbol {s} not in the tree");
                    self.bw.huff(lit_codes[s], lit_lens[s] as usize);
                    if !self.errored {
                        self.out.push(s as u8);
                    }
                }
                Tok::End => {
                    assert!(lit_lens[256] != 0, "symbol 256 not in the tree");
                    self.bw.huff(lit_codes[256], lit_lens[256] as usize);
                }
                Tok::Match { ls, lx, ds, dx } => {
                    let lsi = ls as usize;
                    assert!(lsi > 256, "Tok::Match needs a length symbol > 256");
                    assert!(lit_lens[lsi] != 0, "length symbol {lsi} not in the tree");
                    self.bw.huff(lit_codes[lsi], lit_lens[lsi] as usize);
                    let nx = t.len_extra[lsi - 257] as usize;
                    assert!(nx >= 32 || lx < (1u32 << nx), "length extra payload too wide");
                    self.bw.bits(lx, nx);
                    let dsi = ds as usize;
                    assert!(dist_lens[dsi] != 0, "distance symbol {dsi} not in the tree");
                    self.bw.huff(dist_codes[dsi], dist_lens[dsi] as usize);
                    let ndx = t.dist_extra[dsi] as usize;
                    assert!(ndx >= 32 || dx < (1u32 << ndx), "distance extra payload too wide");
                    self.bw.bits(dx, ndx);

                    if self.errored {
                        continue;
                    }
                    assert!(self.pos_known, "matches after a stored block are not modelled");
                    let length = t.length_of(ls, lx) as usize;
                    let dist = t.distance_of(ds, dx) as usize;
                    if dist > self.out.len() {
                        // E4: rejected, nothing written, decoding stops
                        self.errored = true;
                    } else if dist == 0 {
                        // src == dst: every byte is copied onto itself
                        for _ in 0..length {
                            self.out.push(0);
                        }
                        if length > 0 {
                            self.known = false;
                        }
                    } else {
                        for _ in 0..length {
                            let b = self.out[self.out.len() - dist];
                            self.out.push(b);
                        }
                    }
                }
            }
        }
    }

    fn close_bytes_seg(&mut self, before: usize) {
        let seg: Vec<u8> = self.out[before.min(self.out.len())..].to_vec();
        self.segs.push(Seg::Bytes(seg));
    }

    pub fn fixed_block(&mut self, bfinal: bool, toks: &[Tok], t: &Tables) {
        let ft: Vec<u8> = fixed_lit_lens().into_iter().chain(fixed_dist_lens()).collect();
        self.fixed_block_with(bfinal, toks, t, &ft);
    }

    /// Fixed block honouring a caller-supplied (possibly patched) `cp_fixed_table`.
    pub fn fixed_block_with(&mut self, bfinal: bool, toks: &[Tok], t: &Tables, fixed_table: &[u8]) {
        self.bw.bits(bfinal as u32, 1);
        self.bw.bits(1, 2);
        let ll = fixed_table[0..288].to_vec();
        let dl = fixed_table[288..320].to_vec();
        let lc = canonical(&ll);
        let dc = canonical(&dl);
        let before = self.out.len();
        self.emit_tokens(toks, &ll, &lc, &dl, &dc, t);
        self.close_bytes_seg(before);
    }

    pub fn dynamic_block(&mut self, bfinal: bool, spec: &DynSpec, toks: &[Tok], t: &Tables) {
        self.dynamic_block_perm(bfinal, spec, toks, t, &PERM);
    }

    /// `perm` is the (possibly patched) `cp_permutation_order` table.
    pub fn dynamic_block_perm(
        &mut self,
        bfinal: bool,
        spec: &DynSpec,
        toks: &[Tok],
        t: &Tables,
        perm: &[u8; 19],
    ) {
        self.bw.bits(bfinal as u32, 1);
        self.bw.bits(2, 2);
        assert!((257..=288).contains(&spec.hlit));
        assert!((1..=32).contains(&spec.hdist));
        assert!((4..=19).contains(&spec.hclen));
        self.bw.bits((spec.hlit - 257) as u32, 5);
        self.bw.bits((spec.hdist - 1) as u32, 5);
        self.bw.bits((spec.hclen - 4) as u32, 4);
        for i in 0..spec.hclen {
            self.bw.bits(spec.cl_lens[perm[i] as usize] as u32, 3);
        }
        let cl_codes = canonical(&spec.cl_lens);
        for op in &spec.ops {
            let s = op.sym();
            assert!(spec.cl_lens[s] != 0, "code-length symbol {s} has length 0");
            self.bw.huff(cl_codes[s], spec.cl_lens[s] as usize);
            match *op {
                ClOp::Lit(_) => {}
                ClOp::Rep16(e) => self.bw.bits(e, 2),
                ClOp::Rep17(e) => self.bw.bits(e, 3),
                ClOp::Rep18(e) => self.bw.bits(e, 7),
            }
        }
        let lc = canonical(&spec.lit_lens);
        let dc = canonical(&spec.dist_lens);
        let before = self.out.len();
        self.emit_tokens(toks, &spec.lit_lens, &lc, &spec.dist_lens, &dc, t);
        self.close_bytes_seg(before);
    }

    /// A stored block whose `LEN` is patched at `finish()` time to cover
    /// everything from the data offset to the end of the input — the only shape
    /// `cp_stored` accepts (`bits_left/8 <= LEN`).  Anything appended after this
    /// call therefore becomes part of the copied data, because the C does **not**
    /// advance the bit reader past a stored block.
    pub fn stored_block(&mut self, bfinal: bool, data: &[u8]) {
        self.bw.bits(bfinal as u32, 1);
        self.bw.bits(0, 2);
        self.bw.align_byte();
        let len_field = self.bw.byte_len();
        self.bw.push_bytes(&[0, 0, 0, 0]);
        let data_off = self.bw.byte_len();
        self.bw.push_bytes(data);
        let seg = self.segs.len();
        self.segs.push(Seg::StoredTail { data_off, len: None });
        self.stored.push(StoredInfo { len_field, data_off, seg: Some(seg), override_len: None });
        self.pos_known = false;
    }

    /// A stored block with explicit LEN/NLEN.  `contributes` says whether the
    /// block is expected to succeed (and therefore append `LEN` bytes to `out`).
    pub fn stored_block_raw(
        &mut self,
        bfinal: bool,
        len: u16,
        nlen: u16,
        data: &[u8],
        contributes: bool,
    ) {
        self.bw.bits(bfinal as u32, 1);
        self.bw.bits(0, 2);
        self.bw.align_byte();
        let len_field = self.bw.byte_len();
        self.bw.push_bytes(&len.to_le_bytes());
        self.bw.push_bytes(&nlen.to_le_bytes());
        let data_off = self.bw.byte_len();
        self.bw.push_bytes(data);
        let seg = if contributes {
            let s = self.segs.len();
            self.segs.push(Seg::StoredTail { data_off, len: Some(len as usize) });
            self.pos_known = false;
            Some(s)
        } else {
            self.errored = true;
            None
        };
        self.stored.push(StoredInfo { len_field, data_off, seg, override_len: Some(len) });
    }

    /// Raw bit access for hand-crafted streams.
    pub fn raw_bits(&mut self, val: u32, n: usize) {
        self.bw.bits(val, n);
    }

    /// Finish the stream and return `(input bytes, expected output or None,
    /// expected output length)`.
    ///
    /// `first_bytes` is what `pinflate` will compute from the input pointer's
    /// alignment; with `pad_last_word` the input is padded so `last_bytes == 0`,
    /// which keeps `cp_ptr`'s `words + word_index` arithmetic in sync with the
    /// real byte position (needed by stored blocks).
    pub fn finish(
        mut self,
        first_bytes: usize,
        pad_last_word: bool,
        pad: u8,
    ) -> (Vec<u8>, Option<Vec<u8>>, usize) {
        self.bw.align_byte();
        if pad_last_word {
            while self.bw.byte_len() % 4 != first_bytes % 4 {
                self.bw.push_bytes(&[pad]);
            }
        }
        let mut input = self.bw.bytes.clone();

        for s in &self.stored {
            if s.override_len.is_none() {
                let l = input.len() - s.data_off;
                assert!(l <= 65535, "stored block longer than 65535 bytes");
                let lb = (l as u16).to_le_bytes();
                let nb = (!(l as u16)).to_le_bytes();
                input[s.len_field] = lb[0];
                input[s.len_field + 1] = lb[1];
                input[s.len_field + 2] = nb[0];
                input[s.len_field + 3] = nb[1];
                if let Some(i) = s.seg {
                    if let Seg::StoredTail { len, .. } = &mut self.segs[i] {
                        *len = Some(l);
                    }
                }
            }
        }

        let mut out: Vec<u8> = Vec::new();
        let mut known = self.known;
        for seg in &self.segs {
            match seg {
                Seg::Bytes(b) => out.extend_from_slice(b),
                Seg::StoredTail { data_off, len } => {
                    let l = len.unwrap();
                    if data_off + l <= input.len() {
                        out.extend_from_slice(&input[*data_off..*data_off + l]);
                    } else {
                        // the C reads past the end of the caller's input buffer
                        for _ in 0..l {
                            out.push(0);
                        }
                        known = false;
                    }
                }
            }
        }
        let n = out.len();
        (input, if known { Some(out) } else { None }, n)
    }
}

/// `first_bytes` for a given input-pointer alignment (`pinflate` computes
/// `((in + 3) & ~3) - in`).
pub fn first_bytes_for(align_mod4: usize) -> usize {
    (4 - align_mod4 % 4) % 4
}

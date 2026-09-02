//! A DEFLATE *encoder* used to drive the decoder under test.
//!
//! It is deliberately written from RFC 1951 and from `c_src/src/lib.c`'s own
//! canonical-code construction (`cp_build`), so that the streams it produces
//! exercise precisely the branches enumerated in `CONFIGS.md`:
//! stored / fixed / dynamic blocks, every length- and distance-extra-bit width,
//! `dist == 1` (memset path) vs `dist > 1`, overlapping copies, code-length
//! symbols 16/17/18, and code bit lengths on both sides of `cp_build`'s
//! `len <= 9` lookup cutoff.

#![allow(dead_code)]

use super::rng::Rng;

// ---------------------------------------------------------------------------
// RFC 1951 tables (independent copies; the exported C tables are compared
// against these in the C01 row)
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

/// `cp_permutation_order` from the C source.
pub const PERMUTATION: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// The 320-entry `cp_fixed_table`.
pub fn fixed_table() -> Vec<u8> {
    let mut t = vec![0u8; 288 + 32];
    for i in 0..144 {
        t[i] = 8;
    }
    for i in 144..256 {
        t[i] = 9;
    }
    for i in 256..280 {
        t[i] = 7;
    }
    for i in 280..288 {
        t[i] = 8;
    }
    for i in 288..320 {
        t[i] = 5;
    }
    t
}

// ---------------------------------------------------------------------------
// Bit writer (LSB-first stream, Huffman codes MSB-first, as per RFC 1951)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct BitWriter {
    buf: Vec<u8>,
    cur: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }

    /// Write `n` bits of `val`, least-significant bit first.
    pub fn bits(&mut self, val: u32, n: u32) {
        assert!(n <= 32);
        for i in 0..n {
            let b = (val >> i) & 1;
            self.cur |= b << self.nbits;
            self.nbits += 1;
            if self.nbits == 8 {
                self.buf.push(self.cur as u8);
                self.cur = 0;
                self.nbits = 0;
            }
        }
    }

    /// Write a Huffman code, most-significant bit first.
    pub fn code(&mut self, code: u32, len: u32) {
        assert!(len >= 1 && len <= 15, "bad code length {len}");
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }

    /// Total number of bits written so far (including the partial byte).
    pub fn bit_len(&self) -> usize {
        self.buf.len() * 8 + self.nbits as usize
    }

    pub fn align(&mut self) {
        if self.nbits != 0 {
            self.buf.push(self.cur as u8);
            self.cur = 0;
            self.nbits = 0;
        }
    }

    pub fn raw_bytes(&mut self, b: &[u8]) {
        assert_eq!(self.nbits, 0, "raw_bytes requires byte alignment");
        self.buf.extend_from_slice(b);
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.buf
    }
}

// ---------------------------------------------------------------------------
// Canonical Huffman codes — mirrors `cp_build` in the C source exactly
// ---------------------------------------------------------------------------

/// Assign canonical codes for `lens` the same way `cp_build` does.
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0u32; 16];
    for &l in lens {
        assert!(l < 16, "code length {l} >= 16 would trip cp_build's assert");
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut codes = [0u32; 16];
    for n in 1..16usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
    }
    let mut next = codes;
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

/// True when `sum(2^-len) == 1` over the non-zero lengths, i.e. the prefix code
/// is *complete*. `cp_decode`'s `assert` only holds for complete codes.
pub fn is_complete(lens: &[u8]) -> bool {
    let mut kraft: u64 = 0; // in units of 2^-15
    for &l in lens {
        if l != 0 {
            kraft += 1u64 << (15 - l as u32);
        }
    }
    kraft == 1 << 15
}

/// Build a *complete* set of code lengths for `n` symbols, max depth `max_len`.
///
/// Starts from the single-leaf tree and repeatedly splits a random leaf, which
/// preserves the Kraft sum at exactly 1.
pub fn random_complete_lengths(rng: &mut Rng, n: usize, max_len: u8) -> Vec<u8> {
    assert!(n >= 2);
    let mut leaves: Vec<u8> = vec![0];
    while leaves.len() < n {
        let candidates: Vec<usize> = (0..leaves.len())
            .filter(|&i| leaves[i] < max_len)
            .collect();
        assert!(
            !candidates.is_empty(),
            "cannot fit {n} symbols within depth {max_len}"
        );
        let i = candidates[rng.below(candidates.len())];
        leaves[i] += 1;
        let d = leaves[i];
        leaves.push(d);
    }
    leaves
}

/// A complete, near-balanced code for `n` symbols (all lengths k or k+1).
pub fn balanced_complete_lengths(n: usize) -> Vec<u8> {
    assert!(n >= 2);
    let k = (usize::BITS - 1 - n.leading_zeros()) as u8; // floor(log2(n))
    let short = (1usize << (k + 1)) - n; // this many get length k
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        v.push(if i < short { k } else { k + 1 });
    }
    v
}

/// A maximally skewed complete code: lengths 1,2,3,...,n-1,n-1.
pub fn skewed_complete_lengths(n: usize) -> Vec<u8> {
    assert!(n >= 2 && n <= 16);
    let mut v: Vec<u8> = (1..n as u8).collect();
    v.push((n - 1) as u8);
    v
}

// ---------------------------------------------------------------------------
// Program: what the decompressed stream should contain
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Lit(u8),
    /// Length/distance pair; the encoder picks the symbols.
    Match { len: u32, dist: u32 },
    /// Explicit symbol control (used for the extra-bit-width sweeps and for
    /// deliberately odd encodings such as length 258 via symbol 284).
    Raw {
        lsym: u16,
        lextra: u32,
        dsym: u16,
        dextra: u32,
    },
}

/// Resolve `Op` into (length symbol, extra, distance symbol, extra).
pub fn resolve(op: Op) -> Option<(u16, u32, u16, u32)> {
    match op {
        Op::Lit(_) => None,
        Op::Raw {
            lsym,
            lextra,
            dsym,
            dextra,
        } => Some((lsym, lextra, dsym, dextra)),
        Op::Match { len, dist } => {
            let mut li = 28usize;
            loop {
                if len >= LEN_BASE[li] && len - LEN_BASE[li] < (1 << LEN_EXTRA[li]) {
                    break;
                }
                assert!(li > 0, "length {len} not encodable");
                li -= 1;
            }
            let mut di = 29usize;
            loop {
                if dist >= DIST_BASE[di] && dist - DIST_BASE[di] < (1 << DIST_EXTRA[di]) {
                    break;
                }
                assert!(di > 0, "distance {dist} not encodable");
                di -= 1;
            }
            Some((
                (257 + li) as u16,
                len - LEN_BASE[li],
                di as u16,
                dist - DIST_BASE[di],
            ))
        }
    }
}

/// The bytes a program expands to (the reference output).
pub fn expand(ops: &[Op]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for &op in ops {
        match op {
            Op::Lit(b) => out.push(b),
            _ => {
                let (lsym, lextra, dsym, dextra) = resolve(op).unwrap();
                let len = LEN_BASE[lsym as usize - 257] + lextra;
                let dist = DIST_BASE[dsym as usize] + dextra;
                assert!(dist as usize <= out.len(), "distance {dist} before start");
                for _ in 0..len {
                    let b = out[out.len() - dist as usize];
                    out.push(b);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Block emitters
// ---------------------------------------------------------------------------

/// btype 0 — stored (uncompressed) block.
pub fn emit_stored(w: &mut BitWriter, bfinal: bool, payload: &[u8]) {
    emit_stored_lens(w, bfinal, payload, payload.len() as u16, !(payload.len() as u16));
}

/// btype 0 with explicit LEN/NLEN (for the E1 error row).
pub fn emit_stored_lens(
    w: &mut BitWriter,
    bfinal: bool,
    payload: &[u8],
    len: u16,
    nlen: u16,
) {
    w.bits(bfinal as u32, 1);
    w.bits(0, 2);
    w.align();
    w.bits(len as u32, 16);
    w.bits(nlen as u32, 16);
    w.raw_bytes(payload);
}

/// btype 1 — fixed Huffman block. `table` is the 320-entry fixed length table
/// (normally `fixed_table()`; a mutated copy is used by the C06 row).
pub fn emit_fixed_with(w: &mut BitWriter, bfinal: bool, ops: &[Op], table: &[u8]) {
    assert_eq!(table.len(), 288 + 32);
    let lit_codes = canonical_codes(&table[..288]);
    let dst_codes = canonical_codes(&table[288..]);
    w.bits(bfinal as u32, 1);
    w.bits(1, 2);
    emit_ops(w, ops, &table[..288], &lit_codes, &table[288..], &dst_codes);
    w.code(lit_codes[256], table[256] as u32); // end of block
}

pub fn emit_fixed(w: &mut BitWriter, bfinal: bool, ops: &[Op]) {
    emit_fixed_with(w, bfinal, ops, &fixed_table())
}

fn emit_ops(
    w: &mut BitWriter,
    ops: &[Op],
    lit_lens: &[u8],
    lit_codes: &[u32],
    dst_lens: &[u8],
    dst_codes: &[u32],
) {
    for &op in ops {
        match op {
            Op::Lit(b) => {
                let s = b as usize;
                assert!(lit_lens[s] != 0, "literal {s} has no code");
                w.code(lit_codes[s], lit_lens[s] as u32);
            }
            _ => {
                let (lsym, lextra, dsym, dextra) = resolve(op).unwrap();
                let ls = lsym as usize;
                assert!(lit_lens[ls] != 0, "length symbol {ls} has no code");
                w.code(lit_codes[ls], lit_lens[ls] as u32);
                w.bits(lextra, LEN_EXTRA[ls - 257]);
                let ds = dsym as usize;
                assert!(dst_lens[ds] != 0, "distance symbol {ds} has no code");
                w.code(dst_codes[ds], dst_lens[ds] as u32);
                let de = if ds < 30 { DIST_EXTRA[ds] } else { 0 };
                w.bits(dextra, de);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// btype 2 — dynamic Huffman block
// ---------------------------------------------------------------------------

/// Which code-length repeat codes the code-length emitter is allowed to use.
#[derive(Clone, Copy, Default, Debug)]
pub struct RepeatOpts {
    pub use16: bool,
    pub use17: bool,
    pub use18: bool,
}

impl RepeatOpts {
    pub fn none() -> RepeatOpts {
        RepeatOpts::default()
    }
    pub fn all() -> RepeatOpts {
        RepeatOpts {
            use16: true,
            use17: true,
            use18: true,
        }
    }
}

/// A fully specified dynamic block.
pub struct Dynamic {
    /// exactly `nlit` entries, `nlit` in 257..=288
    pub lit_lens: Vec<u8>,
    /// exactly `ndst` entries, `ndst` in 1..=32
    pub dst_lens: Vec<u8>,
    pub repeats: RepeatOpts,
    /// forced HCLEN+4 (must be >= the highest permutation index used)
    pub force_nlen: Option<usize>,
}

/// Encode the concatenated (lit_lens ++ dst_lens) sequence into code-length
/// alphabet symbols with their extra bits.
fn code_length_symbols(seq: &[u8], r: RepeatOpts) -> Vec<(u8, u32, u32)> {
    let mut out: Vec<(u8, u32, u32)> = Vec::new(); // (symbol, extra_val, extra_bits)
    let mut i = 0usize;
    while i < seq.len() {
        let v = seq[i];
        let mut run = 1usize;
        while i + run < seq.len() && seq[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let mut left = run;
            while left > 0 {
                if r.use18 && left >= 11 {
                    let n = left.min(138);
                    out.push((18, (n - 11) as u32, 7));
                    left -= n;
                } else if r.use17 && left >= 3 {
                    let n = left.min(10);
                    out.push((17, (n - 3) as u32, 3));
                    left -= n;
                } else {
                    out.push((0, 0, 0));
                    left -= 1;
                }
            }
        } else {
            // The first occurrence must always be literal (symbol 16 copies the
            // *previous* length, and at position 0 the C reads lens[-1]).
            out.push((v, 0, 0));
            let mut left = run - 1;
            while left > 0 {
                if r.use16 && left >= 3 && !out.is_empty() {
                    let n = left.min(6);
                    out.push((16, (n - 3) as u32, 2));
                    left -= n;
                } else {
                    out.push((v, 0, 0));
                    left -= 1;
                }
            }
        }
        i += run;
    }
    out
}

pub fn emit_dynamic(w: &mut BitWriter, bfinal: bool, d: &Dynamic, ops: &[Op]) {
    let perm: Vec<usize> = PERMUTATION.to_vec();
    emit_dynamic_with_permutation(w, bfinal, d, ops, &perm)
}

/// `emit_dynamic` against an arbitrary code-length permutation (row C03, which
/// mutates the exported `cp_permutation_order`).
pub fn emit_dynamic_with_permutation(
    w: &mut BitWriter,
    bfinal: bool,
    d: &Dynamic,
    ops: &[Op],
    perm: &[usize],
) {
    let (lit_codes, dst_codes) = emit_dynamic_header_with_permutation(w, bfinal, d, perm);
    emit_ops(w, ops, &d.lit_lens, &lit_codes, &d.dst_lens, &dst_codes);
    assert!(d.lit_lens[256] != 0, "end-of-block symbol needs a code");
    w.code(lit_codes[256], d.lit_lens[256] as u32);
}

/// Emit only a dynamic block's header (block type, HLIT/HDIST/HCLEN, the
/// code-length code and the coded lit/dist lengths). Used by the malformed-tree
/// tests, which need to follow the header with hand-written bits and must be
/// able to declare code lengths that no valid encoder would produce.
pub fn emit_dynamic_header_only(
    w: &mut BitWriter,
    bfinal: bool,
    d: &Dynamic,
) -> (Vec<u32>, Vec<u32>) {
    let perm: Vec<usize> = PERMUTATION.to_vec();
    emit_dynamic_header_with_permutation(w, bfinal, d, &perm)
}

pub fn emit_dynamic_header_with_permutation(
    w: &mut BitWriter,
    bfinal: bool,
    d: &Dynamic,
    perm: &[usize],
) -> (Vec<u32>, Vec<u32>) {
    assert_eq!(perm.len(), 19);
    let nlit = d.lit_lens.len();
    let ndst = d.dst_lens.len();
    assert!((257..=288).contains(&nlit), "nlit {nlit} out of range");
    assert!((1..=32).contains(&ndst), "ndst {ndst} out of range");

    let mut seq: Vec<u8> = Vec::with_capacity(nlit + ndst);
    seq.extend_from_slice(&d.lit_lens);
    seq.extend_from_slice(&d.dst_lens);

    let cl = code_length_symbols(&seq, d.repeats);

    // Build a complete code over the code-length alphabet symbols actually used.
    let mut used = [false; 19];
    for &(s, _, _) in &cl {
        used[s as usize] = true;
    }
    let used_syms: Vec<usize> = (0..19).filter(|&i| used[i]).collect();
    let mut cl_lens = [0u8; 19];
    if used_syms.len() == 1 {
        // A single-symbol code is incomplete; add a spare so the code stays
        // complete (the spare symbol is never emitted).
        let spare = (0..19).find(|i| !used[*i]).unwrap();
        cl_lens[used_syms[0]] = 1;
        cl_lens[spare] = 1;
    } else {
        let lens = balanced_complete_lengths(used_syms.len());
        for (k, &s) in used_syms.iter().enumerate() {
            cl_lens[s] = lens[k].max(1).min(7);
        }
        assert!(is_complete(&cl_lens), "code-length code not complete");
    }
    assert!(cl_lens.iter().all(|&l| l <= 7));

    // HCLEN: number of code lengths transmitted, in permutation order.
    let mut nlen = 4usize;
    for (i, &pi) in perm.iter().enumerate() {
        if cl_lens[pi] != 0 {
            nlen = nlen.max(i + 1);
        }
    }
    if let Some(f) = d.force_nlen {
        assert!(f >= nlen, "force_nlen {f} too small (need {nlen})");
        assert!((4..=19).contains(&f));
        nlen = f;
    }

    let cl_codes = canonical_codes(&cl_lens);

    w.bits(bfinal as u32, 1);
    w.bits(2, 2);
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.bits(cl_lens[perm[i]] as u32, 3);
    }
    for &(s, ev, eb) in &cl {
        w.code(cl_codes[s as usize], cl_lens[s as usize] as u32);
        if eb > 0 {
            w.bits(ev, eb);
        }
    }

    (
        canonical_codes(&d.lit_lens),
        canonical_codes(&d.dst_lens),
    )
}

/// Build a `Dynamic` whose alphabet covers every symbol used by `ops`
/// (plus 256), with code lengths of the requested shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Balanced,
    Skewed,
    Random,
    /// All code lengths <= 9 (fills `cp_build`'s lookup table for every symbol).
    Shallow,
    /// Force some code lengths into 10..15 (tree-only path in `cp_build`).
    Deep,
}

pub fn dynamic_for(
    rng: &mut Rng,
    ops: &[Op],
    shape: Shape,
    repeats: RepeatOpts,
    nlit: usize,
    ndst: usize,
) -> Dynamic {
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));

    let mut lit_used = vec![false; nlit];
    let mut dst_used = vec![false; ndst];
    lit_used[256] = true;
    for &op in ops {
        match op {
            Op::Lit(b) => lit_used[b as usize] = true,
            _ => {
                let (lsym, _, dsym, _) = resolve(op).unwrap();
                assert!((lsym as usize) < nlit, "length symbol {lsym} >= nlit {nlit}");
                assert!((dsym as usize) < ndst, "distance symbol {dsym} >= ndst {ndst}");
                lit_used[lsym as usize] = true;
                dst_used[dsym as usize] = true;
            }
        }
    }

    let lit_lens = lens_for(rng, &lit_used, shape, 15);
    let n_dst_used = dst_used.iter().filter(|x| **x).count();
    let dst_lens = if n_dst_used == 0 {
        vec![0u8; ndst]
    } else if n_dst_used == 1 && ndst == 1 {
        // Degenerate single-code distance tree (`ndst == 1` row C35).
        vec![1u8]
    } else {
        if n_dst_used == 1 {
            // Pad with a spare so the code is complete.
            let spare = (0..ndst).find(|&i| !dst_used[i]).unwrap();
            dst_used[spare] = true;
        }
        lens_for(rng, &dst_used, shape, 15)
    };

    Dynamic {
        lit_lens,
        dst_lens,
        repeats,
        force_nlen: None,
    }
}

fn lens_for(rng: &mut Rng, used: &[bool], shape: Shape, cap: u8) -> Vec<u8> {
    let idx: Vec<usize> = (0..used.len()).filter(|&i| used[i]).collect();
    let n = idx.len();
    let mut out = vec![0u8; used.len()];
    if n == 0 {
        return out;
    }
    if n == 1 {
        // Complete a 1-symbol alphabet by giving a spare symbol length 1 too.
        let spare = (0..used.len()).find(|&i| !used[i]).unwrap();
        out[idx[0]] = 1;
        out[spare] = 1;
        return out;
    }
    let lens = match shape {
        Shape::Balanced => balanced_complete_lengths(n),
        Shape::Skewed => {
            if n <= 16 {
                skewed_complete_lengths(n)
            } else {
                balanced_complete_lengths(n)
            }
        }
        Shape::Random => random_complete_lengths(rng, n, cap),
        Shape::Shallow => {
            if n <= 512 {
                let mut l = random_complete_lengths(rng, n, 9);
                l.truncate(n);
                l
            } else {
                balanced_complete_lengths(n)
            }
        }
        Shape::Deep => {
            // Skewed as far as depth 15 allows, then balanced for the rest.
            if n <= 16 {
                skewed_complete_lengths(n)
            } else {
                // Split: 15 symbols on a unary spine, the rest balanced in the
                // deepest subtree.
                let mut l: Vec<u8> = Vec::new();
                let spine = 13usize;
                for d in 1..=spine {
                    l.push(d as u8);
                }
                let rest = n - spine;
                let sub = balanced_complete_lengths(rest.max(2));
                let extra = sub.iter().map(|&x| x + spine as u8).collect::<Vec<u8>>();
                l.extend(extra.into_iter().take(rest));
                if !is_complete(&l) || l.iter().any(|&x| x > 15) {
                    balanced_complete_lengths(n)
                } else {
                    l
                }
            }
        }
    };
    assert_eq!(lens.len(), n);
    for (k, &i) in idx.iter().enumerate() {
        out[i] = lens[k];
    }
    assert!(is_complete(&out), "incomplete code for shape {shape:?}");
    out
}

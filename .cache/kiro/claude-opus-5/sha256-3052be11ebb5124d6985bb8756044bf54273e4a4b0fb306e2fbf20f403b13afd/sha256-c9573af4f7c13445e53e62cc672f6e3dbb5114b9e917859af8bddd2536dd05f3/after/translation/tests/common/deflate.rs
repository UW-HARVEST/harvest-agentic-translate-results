//! A small, fully controllable DEFLATE *encoder* used to synthesise the valid
//! streams demanded by `CONFIGS.md`. It is deliberately explicit (no automatic
//! choices) so that each row of the configuration table can be targeted:
//! block type, HLIT/HDIST/HCLEN, which code-length opcodes appear, code length
//! ranges, length/distance symbol classes and copy strategies are all caller
//! controlled.

#![allow(dead_code)]

use super::Rng;

// ---------------------------------------------------------------------------
// Bit writer (DEFLATE order: integers LSB-first, Huffman codes MSB-first)
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub buf: Vec<u8>,
    pub nbits: usize,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            buf: Vec::new(),
            nbits: 0,
        }
    }
    pub fn bit(&mut self, b: u32) {
        let byte = self.nbits / 8;
        if byte >= self.buf.len() {
            self.buf.push(0);
        }
        self.buf[byte] |= ((b & 1) as u8) << (self.nbits % 8);
        self.nbits += 1;
    }
    /// `n` bits of `v`, least-significant first (DEFLATE integer order).
    pub fn push(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit(v >> i);
        }
    }
    /// A Huffman code of `n` bits, most-significant first (DEFLATE code order).
    pub fn code(&mut self, c: u32, n: u32) {
        for i in (0..n).rev() {
            self.bit(c >> i);
        }
    }
    pub fn align_byte(&mut self) {
        while self.nbits % 8 != 0 {
            self.bit(0);
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.buf.clone()
    }
}

// ---------------------------------------------------------------------------
// DEFLATE static tables (mirrors of cp_len_base / cp_dist_base etc.)
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

/// Length symbol index (0..=28, i.e. DEFLATE symbol 257+i) for a match length.
pub fn len_sym(len: u32) -> (usize, u32, u32) {
    assert!((3..=258).contains(&len), "bad match length {len}");
    let mut i = 28;
    while i > 0 && LEN_BASE[i] > len {
        i -= 1;
    }
    let extra = LEN_EXTRA[i];
    let val = len - LEN_BASE[i];
    assert!(extra == 0 || val < (1 << extra), "len {len} sym {i}");
    (i, extra, val)
}

pub fn dist_sym(dist: u32) -> (usize, u32, u32) {
    assert!((1..=32768).contains(&dist), "bad distance {dist}");
    let mut i = 29;
    while i > 0 && DIST_BASE[i] > dist {
        i -= 1;
    }
    (i, DIST_EXTRA[i], dist - DIST_BASE[i])
}

/// The fixed literal/length code (`cp_fixed_table[0..288]`).
pub fn fixed_lit_code(sym: u32) -> (u32, u32) {
    match sym {
        0..=143 => (0x30 + sym, 8),
        144..=255 => (0x190 + (sym - 144), 9),
        256..=279 => (sym - 256, 7),
        280..=287 => (0xC0 + (sym - 280), 8),
        _ => panic!("bad literal/length symbol {sym}"),
    }
}

/// The fixed literal/length code as a length vector, i.e. `cp_fixed_table`.
pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![8u8; 288];
    for i in 144..256 {
        v[i] = 9;
    }
    for i in 256..280 {
        v[i] = 7;
    }
    v
}
pub fn fixed_dst_lens() -> Vec<u8> {
    vec![5u8; 32]
}

/// `btype = 1` but with a caller-supplied length table, for exercising a
/// consumer-mutated `cp_fixed_table` (CONFIGS.md C40).
pub fn emit_fixed_with_lens(
    w: &mut BitWriter,
    bfinal: bool,
    toks: &[Tok],
    lit_lens: &[u8],
    dst_lens: &[u8],
) {
    let lit_codes = canonical(lit_lens);
    let dst_codes = canonical(dst_lens);
    w.push(bfinal as u32, 1);
    w.push(1, 2);
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let s = b as usize;
                assert!(lit_lens[s] != 0);
                w.code(lit_codes[s], lit_lens[s] as u32);
            }
            Tok::Match { len, dist } => {
                let (ls, lx, lv) = len_sym(len);
                let s = 257 + ls;
                assert!(lit_lens[s] != 0);
                w.code(lit_codes[s], lit_lens[s] as u32);
                w.push(lv, lx);
                let (ds, dx, dv) = dist_sym(dist);
                assert!(dst_lens[ds] != 0);
                w.code(dst_codes[ds], dst_lens[ds] as u32);
                w.push(dv, dx);
            }
        }
    }
    assert!(lit_lens[256] != 0);
    w.code(lit_codes[256], lit_lens[256] as u32);
}

/// `btype = 1` header plus an arbitrary literal/length symbol number, so that
/// the length-symbol tail 285..287 (whose `cp_len_base` entries are 258, 0, 0)
/// can be driven directly.
pub fn emit_fixed_raw_sym(w: &mut BitWriter, sym: u32) {
    let (c, n) = fixed_lit_code(sym);
    w.code(c, n);
}

pub fn emit_fixed_raw_dist(w: &mut BitWriter, dsym: u32, extra_bits: u32, extra_val: u32) {
    w.code(dsym, 5);
    w.push(extra_val, extra_bits);
}

/// Canonical code values for a code-length vector (RFC1951 §3.2.2).
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let max = *lens.iter().max().unwrap_or(&0) as usize;
    let mut bl_count = vec![0u32; max + 1];
    for &l in lens {
        if l != 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; max + 2];
    let mut code = 0u32;
    for bits in 1..=max {
        code = (code + bl_count[bits - 1]) << 1;
        next[bits] = code;
    }
    lens.iter()
        .map(|&l| {
            if l == 0 {
                0
            } else {
                let c = next[l as usize];
                next[l as usize] += 1;
                c
            }
        })
        .collect()
}

/// True when the code described by `lens` is *complete* (Kraft sum == 1) or is
/// the single-code degenerate case, which `cp_build`/`cp_decode` also accept.
pub fn is_complete(lens: &[u8]) -> bool {
    let used: Vec<u8> = lens.iter().copied().filter(|&l| l != 0).collect();
    if used.is_empty() {
        return true;
    }
    if used.len() == 1 {
        return used[0] == 1;
    }
    let mut sum: u64 = 0;
    for &l in &used {
        sum += 1u64 << (15 - l);
    }
    sum == 1u64 << 15
}

/// Huffman code lengths from symbol frequencies, capped at `max_len`.
///
/// Uses plain Huffman first; if that exceeds `max_len` it falls back to uniform
/// weights (which for <= 288 symbols can never exceed depth 9). Deterministic.
pub fn huff_lengths(freqs: &[u32], max_len: u8) -> Vec<u8> {
    let n = freqs.len();
    let used: Vec<usize> = (0..n).filter(|&i| freqs[i] > 0).collect();
    let mut lens = vec![0u8; n];
    match used.len() {
        0 => return lens,
        1 => {
            lens[used[0]] = 1;
            return lens;
        }
        _ => {}
    }
    for attempt in 0..2 {
        let w: Vec<u64> = if attempt == 0 {
            used.iter().map(|&i| freqs[i] as u64).collect()
        } else {
            used.iter().map(|_| 1u64).collect()
        };
        if let Some(d) = huff_depths(&w) {
            if *d.iter().max().unwrap() <= max_len as u32 {
                for (k, &i) in used.iter().enumerate() {
                    lens[i] = d[k] as u8;
                }
                assert!(is_complete(&lens), "generated code is not complete");
                return lens;
            }
        }
    }
    panic!("cannot build a code of depth <= {max_len} for {} symbols", used.len());
}

/// Depths of a Huffman tree over `weights` (deterministic; ties broken by
/// insertion order, so the result is reproducible).
fn huff_depths(weights: &[u64]) -> Option<Vec<u32>> {
    let n = weights.len();
    if n < 2 {
        return None;
    }
    // node: (weight, left, right); leaves are 0..n
    let mut wt: Vec<u64> = weights.to_vec();
    let mut kids: Vec<Option<(usize, usize)>> = vec![None; n];
    let mut alive: Vec<usize> = (0..n).collect();
    while alive.len() > 1 {
        // two smallest
        alive.sort_by_key(|&i| (wt[i], i));
        let a = alive.remove(0);
        let b = alive.remove(0);
        let idx = wt.len();
        wt.push(wt[a] + wt[b]);
        kids.push(Some((a, b)));
        alive.push(idx);
    }
    let root = alive[0];
    let mut depth = vec![0u32; wt.len()];
    let mut stack = vec![(root, 0u32)];
    while let Some((node, d)) = stack.pop() {
        depth[node] = d;
        if let Some((a, b)) = kids[node] {
            stack.push((a, d + 1));
            stack.push((b, d + 1));
        }
    }
    Some(depth[..n].to_vec())
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    Match { len: u32, dist: u32 },
}

/// Expand a token list to the bytes a correct inflater must produce.
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut o: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => o.push(b),
            Tok::Match { len, dist } => {
                assert!(dist as usize <= o.len(), "match distance exceeds history");
                for _ in 0..len {
                    let b = o[o.len() - dist as usize];
                    o.push(b);
                }
            }
        }
    }
    o
}

// ---------------------------------------------------------------------------
// Block emitters
// ---------------------------------------------------------------------------

/// `btype = 0`. Note: the C's `cp_stored` never consumes the payload from the
/// bit reader, and rejects any input with more whole bytes left than `LEN`
/// (ERRORS.md E2), so a stored block must normally be last in the input.
pub fn emit_stored(w: &mut BitWriter, bfinal: bool, payload: &[u8]) {
    w.push(bfinal as u32, 1);
    w.push(0, 2);
    w.align_byte();
    let len = payload.len() as u32;
    w.push(len & 0xFFFF, 16);
    w.push((!len) & 0xFFFF, 16);
    for &b in payload {
        w.push(b as u32, 8);
    }
}

/// `btype = 0` with caller-chosen LEN/NLEN (for the E1/E2 error rows).
pub fn emit_stored_raw(w: &mut BitWriter, bfinal: bool, len: u16, nlen: u16, payload: &[u8]) {
    w.push(bfinal as u32, 1);
    w.push(0, 2);
    w.align_byte();
    w.push(len as u32, 16);
    w.push(nlen as u32, 16);
    for &b in payload {
        w.push(b as u32, 8);
    }
}

/// `btype = 1` (fixed Huffman).
pub fn emit_fixed(w: &mut BitWriter, bfinal: bool, toks: &[Tok]) {
    w.push(bfinal as u32, 1);
    w.push(1, 2);
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let (c, n) = fixed_lit_code(b as u32);
                w.code(c, n);
            }
            Tok::Match { len, dist } => {
                let (ls, lx, lv) = len_sym(len);
                let (c, n) = fixed_lit_code(257 + ls as u32);
                w.code(c, n);
                w.push(lv, lx);
                let (ds, dx, dv) = dist_sym(dist);
                w.code(ds as u32, 5);
                w.push(dv, dx);
            }
        }
    }
    let (c, n) = fixed_lit_code(256);
    w.code(c, n);
}

/// Knobs for a dynamic (`btype = 2`) block.
#[derive(Clone)]
pub struct DynOpts {
    /// force HLIT to at least this many literal/length codes (257..=288)
    pub min_hlit: usize,
    /// force HDIST to at least this many distance codes (1..=32)
    pub min_hdist: usize,
    /// emit all 19 code-length lengths instead of trimming trailing zeros
    pub full_hclen: bool,
    /// force HCLEN to exactly this many entries (must cover all used CL symbols)
    pub force_hclen: Option<usize>,
    /// use uniform symbol weights (shallow codes, all lengths <= 9) instead of
    /// frequency-weighted ones (which can reach 10..15)
    pub uniform_weights: bool,
    /// disable the 16/17/18 run-length opcodes in the code-length stream
    pub no_rle: bool,
    /// order in which the 19 code-length lengths are written; must equal the
    /// contents of the (writable) `cp_permutation_order` export at call time
    pub perm: [usize; 19],
}

impl Default for DynOpts {
    fn default() -> DynOpts {
        DynOpts {
            min_hlit: 257,
            min_hdist: 1,
            full_hclen: false,
            force_hclen: None,
            uniform_weights: false,
            no_rle: false,
            perm: PERM,
        }
    }
}

pub const PERM: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Encoded code-length stream item.
#[derive(Clone, Copy, Debug)]
struct ClItem {
    sym: usize,
    extra_bits: u32,
    extra_val: u32,
}

fn rle_lengths(v: &[u8], no_rle: bool) -> Vec<ClItem> {
    let mut out = Vec::new();
    if no_rle {
        for &x in v {
            out.push(ClItem {
                sym: x as usize,
                extra_bits: 0,
                extra_val: 0,
            });
        }
        return out;
    }
    let mut i = 0usize;
    while i < v.len() {
        let cur = v[i];
        let mut run = 1usize;
        while i + run < v.len() && v[i + run] == cur {
            run += 1;
        }
        i += run;
        if cur == 0 {
            while run >= 11 {
                let n = run.min(138);
                out.push(ClItem {
                    sym: 18,
                    extra_bits: 7,
                    extra_val: (n - 11) as u32,
                });
                run -= n;
            }
            while run >= 3 {
                let n = run.min(10);
                out.push(ClItem {
                    sym: 17,
                    extra_bits: 3,
                    extra_val: (n - 3) as u32,
                });
                run -= n;
            }
            for _ in 0..run {
                out.push(ClItem {
                    sym: 0,
                    extra_bits: 0,
                    extra_val: 0,
                });
            }
        } else {
            out.push(ClItem {
                sym: cur as usize,
                extra_bits: 0,
                extra_val: 0,
            });
            run -= 1;
            while run >= 3 {
                let n = run.min(6);
                out.push(ClItem {
                    sym: 16,
                    extra_bits: 2,
                    extra_val: (n - 3) as u32,
                });
                run -= n;
            }
            for _ in 0..run {
                out.push(ClItem {
                    sym: cur as usize,
                    extra_bits: 0,
                    extra_val: 0,
                });
            }
        }
    }
    out
}

/// What `emit_dynamic` actually produced — used by tests to assert that a row
/// really did hit the configuration it claims (e.g. "a length in 10..15").
pub struct DynInfo {
    pub hlit: usize,
    pub hdist: usize,
    pub hclen: usize,
    pub lit_lens: Vec<u8>,
    pub dst_lens: Vec<u8>,
    pub cl_syms_used: Vec<usize>,
    pub max_lit_len: u8,
}

pub fn emit_dynamic(w: &mut BitWriter, bfinal: bool, toks: &[Tok], o: &DynOpts) -> DynInfo {
    // --- frequencies -------------------------------------------------------
    let mut lfreq = vec![0u32; 288];
    let mut dfreq = vec![0u32; 32];
    for t in toks {
        match *t {
            Tok::Lit(b) => lfreq[b as usize] += 1,
            Tok::Match { len, dist } => {
                lfreq[257 + len_sym(len).0] += 1;
                dfreq[dist_sym(dist).0] += 1;
            }
        }
    }
    lfreq[256] += 1; // end of block
    // Pad so HLIT/HDIST reach the requested minima: give the boundary symbol a
    // code even though it is never emitted.
    let hlit = o.min_hlit.max(
        (0..288)
            .rev()
            .find(|&i| lfreq[i] > 0)
            .map(|i| i + 1)
            .unwrap_or(257)
            .max(257),
    );
    assert!((257..=288).contains(&hlit));
    if lfreq[hlit - 1] == 0 {
        lfreq[hlit - 1] = 1;
    }
    let hdist = o.min_hdist.max(
        (0..32)
            .rev()
            .find(|&i| dfreq[i] > 0)
            .map(|i| i + 1)
            .unwrap_or(1)
            .max(1),
    );
    assert!((1..=32).contains(&hdist));
    if dfreq[hdist - 1] == 0 {
        dfreq[hdist - 1] = 1;
    }

    let mk = |f: &[u32]| -> Vec<u8> {
        if o.uniform_weights {
            let uf: Vec<u32> = f.iter().map(|&x| (x > 0) as u32).collect();
            huff_lengths(&uf, 15)
        } else {
            huff_lengths(f, 15)
        }
    };
    let lit_lens = mk(&lfreq[..hlit]);
    let dst_lens = mk(&dfreq[..hdist]);
    let max_lit_len = *lit_lens.iter().max().unwrap();

    // --- code-length stream ------------------------------------------------
    let mut all: Vec<u8> = Vec::new();
    all.extend_from_slice(&lit_lens);
    all.extend_from_slice(&dst_lens);
    let items = rle_lengths(&all, o.no_rle);

    let mut clfreq = vec![0u32; 19];
    for it in &items {
        clfreq[it.sym] += 1;
    }
    let cl_lens = huff_lengths(&clfreq, 7);
    let cl_codes = canonical(&cl_lens);

    let mut hclen = if o.full_hclen {
        19
    } else {
        let mut k = 19;
        while k > 4 && cl_lens[o.perm[k - 1]] == 0 {
            k -= 1;
        }
        k
    };
    if let Some(f) = o.force_hclen {
        // only legal if every used CL symbol is inside the first `f` entries
        let needed = (0..19).filter(|&i| cl_lens[o.perm[i]] != 0).max().unwrap() + 1;
        assert!(f >= needed, "force_hclen={f} < needed={needed}");
        hclen = f;
    }
    assert!((4..=19).contains(&hclen));

    // --- emit --------------------------------------------------------------
    w.push(bfinal as u32, 1);
    w.push(2, 2);
    w.push((hlit - 257) as u32, 5);
    w.push((hdist - 1) as u32, 5);
    w.push((hclen - 4) as u32, 4);
    for i in 0..hclen {
        w.push(cl_lens[o.perm[i]] as u32, 3);
    }
    for it in &items {
        w.code(cl_codes[it.sym], cl_lens[it.sym] as u32);
        if it.extra_bits > 0 {
            w.push(it.extra_val, it.extra_bits);
        }
    }

    let lit_codes = canonical(&lit_lens);
    let dst_codes = canonical(&dst_lens);
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let s = b as usize;
                assert!(lit_lens[s] != 0, "literal {s} has no code");
                w.code(lit_codes[s], lit_lens[s] as u32);
            }
            Tok::Match { len, dist } => {
                let (ls, lx, lv) = len_sym(len);
                let s = 257 + ls;
                assert!(lit_lens[s] != 0);
                w.code(lit_codes[s], lit_lens[s] as u32);
                w.push(lv, lx);
                let (ds, dx, dv) = dist_sym(dist);
                assert!(dst_lens[ds] != 0, "distance sym {ds} has no code");
                w.code(dst_codes[ds], dst_lens[ds] as u32);
                w.push(dv, dx);
            }
        }
    }
    assert!(lit_lens[256] != 0);
    w.code(lit_codes[256], lit_lens[256] as u32);

    DynInfo {
        hlit,
        hdist,
        hclen,
        lit_lens,
        dst_lens,
        cl_syms_used: (0..19).filter(|&i| clfreq[i] > 0).collect(),
        max_lit_len,
    }
}

// ---------------------------------------------------------------------------
// Random token stream generation
// ---------------------------------------------------------------------------

/// Random, always *valid* token stream: matches never reach behind the start of
/// the output and lengths/distances stay in the DEFLATE ranges.
pub fn random_tokens(rng: &mut Rng, n: usize, max_dist: u32, lit_alphabet: &[u8]) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut produced: u32 = 0;
    for _ in 0..n {
        if produced >= 1 && rng.below(100) < 35 {
            let dist = rng.range(1, produced.min(max_dist).max(1));
            let len = rng.range(3, 258);
            toks.push(Tok::Match { len, dist });
            produced += len;
        } else {
            let b = lit_alphabet[rng.below(lit_alphabet.len() as u32) as usize];
            toks.push(Tok::Lit(b));
            produced += 1;
        }
    }
    toks
}

pub fn all_bytes() -> Vec<u8> {
    (0..=255u8).collect()
}
pub fn low_bytes() -> Vec<u8> {
    (0..=143u8).collect()
}
pub fn high_bytes() -> Vec<u8> {
    (144..=255u8).collect()
}

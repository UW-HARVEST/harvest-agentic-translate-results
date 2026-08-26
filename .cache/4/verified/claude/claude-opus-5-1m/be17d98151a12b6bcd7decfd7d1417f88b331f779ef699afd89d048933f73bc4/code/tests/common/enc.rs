//! A small DEFLATE *encoder*, used to drive `pinflate` down chosen code paths.
//!
//! Compressing with a real library would only produce whatever streams that
//! library happens to like. The rows in `CONFIGS.md` name specific branches of
//! `c_src/src/lib.c` (`backwards_distance == 1`, code-length symbol 18,
//! `ndst == 1`, code lengths > 9, `nlen == 4`, …), so the tests build the
//! streams themselves and can guarantee each branch is taken.

#![allow(dead_code)]

use super::{canonical_codes, fixed_lit_lens, BitWriter, Rng};

/// `cp_len_base[0..29]`
pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
/// `cp_len_extra_bits[0..29]`
pub const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// `cp_dist_base[0..30]`
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
/// `cp_dist_extra_bits[0..30]`
pub const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

pub fn len_sym(len: u32) -> usize {
    assert!((3..=258).contains(&len), "length {len} out of range");
    if len == 258 {
        return 28;
    }
    let mut s = 0;
    for i in 0..28 {
        if LEN_BASE[i] <= len {
            s = i;
        }
    }
    s
}

pub fn dist_sym(d: u32) -> usize {
    assert!((1..=32768).contains(&d), "distance {d} out of range");
    let mut s = 0;
    for i in 0..30 {
        if DIST_BASE[i] <= d {
            s = i;
        }
    }
    s
}

/// One emitted DEFLATE item.
#[derive(Copy, Clone, Debug)]
pub enum Item {
    Lit(u8),
    /// back-reference: copy `len` bytes from `dist` bytes back
    Match { len: u32, dist: u32 },
}

impl Item {
    pub fn expand(items: &[Item]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        for it in items {
            match *it {
                Item::Lit(b) => out.push(b),
                Item::Match { len, dist } => {
                    let d = dist as usize;
                    assert!(d <= out.len(), "match distance {d} exceeds {} bytes", out.len());
                    for _ in 0..len {
                        let b = out[out.len() - d];
                        out.push(b);
                    }
                }
            }
        }
        out
    }
}

/// Classic Huffman code-length assignment (repeated merge of the two smallest
/// weights). Symbols with weight 0 get length 0. A single used symbol gets
/// length 1, which is what DEFLATE encoders emit for `ndst == 1`.
pub fn huffman_lens(weights: &[u64]) -> Vec<u8> {
    let n = weights.len();
    let used: Vec<usize> = (0..n).filter(|&i| weights[i] > 0).collect();
    let mut lens = vec![0u8; n];
    if used.is_empty() {
        return lens;
    }
    if used.len() == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    // nodes: (weight, depth-accumulating leaf set)
    let mut nodes: Vec<(u64, Vec<usize>)> = used.iter().map(|&i| (weights[i], vec![i])).collect();
    while nodes.len() > 1 {
        nodes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].cmp(&b.1[0])));
        let (w0, l0) = nodes.remove(0);
        let (w1, l1) = nodes.remove(0);
        for &i in l0.iter().chain(l1.iter()) {
            lens[i] += 1;
        }
        let mut merged = l0;
        merged.extend(l1);
        merged.sort_unstable();
        nodes.push((w0 + w1, merged));
    }
    for &i in &used {
        assert!(lens[i] <= 15, "code length {} exceeds 15", lens[i]);
    }
    lens
}

/// Kraft sum check: `== 1 << 15` means a complete code.
pub fn kraft(lens: &[u8]) -> u32 {
    lens.iter()
        .filter(|&&l| l != 0)
        .map(|&l| 1u32 << (15 - l))
        .sum()
}

/// A complete, near-balanced code over the symbols with a non-zero weight.
/// With `m` used symbols, `2r` get length `k + 1` and `m - 2r` get length `k`,
/// where `k = floor(log2 m)` and `r = m - 2^k`; the Kraft sum is exactly 1.
pub fn balanced_lens(weights: &[u64]) -> Vec<u8> {
    let used: Vec<usize> = (0..weights.len()).filter(|&i| weights[i] > 0).collect();
    let mut lens = vec![0u8; weights.len()];
    let m = used.len();
    if m == 0 {
        return lens;
    }
    if m == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    let k = (usize::BITS - 1 - m.leading_zeros()) as usize; // floor(log2 m)
    let r = m - (1 << k);
    for (j, &s) in used.iter().enumerate() {
        lens[s] = if j < 2 * r { (k + 1) as u8 } else { k as u8 };
    }
    lens
}

/// Depth-limited code lengths.
///
/// DEFLATE's dynamic header writes each code-length code length into a **3-bit**
/// field, so the 19-symbol code-length alphabet may not exceed depth 7, and the
/// literal/distance alphabets may not exceed 15. Plain Huffman easily exceeds
/// both, so the weight range is halved until the tree fits, falling back to a
/// balanced code.
pub fn lens_with_limit(weights: &[u64], limit: u8) -> Vec<u8> {
    let mut w = weights.to_vec();
    for _ in 0..64 {
        let l = huffman_lens(&w);
        if l.iter().all(|&x| x <= limit) {
            return l;
        }
        if w.iter().filter(|&&x| x > 0).all(|&x| x == 1) {
            break;
        }
        for x in w.iter_mut() {
            if *x > 1 {
                *x = (*x + 1) / 2;
            }
        }
    }
    let l = balanced_lens(&w);
    assert!(
        l.iter().all(|&x| x <= limit),
        "cannot fit {} symbols into depth {limit}",
        w.iter().filter(|&&x| x > 0).count()
    );
    l
}

// ---------------------------------------------------------------------------
// Fixed-Huffman blocks (btype == 1)
// ---------------------------------------------------------------------------

pub fn fixed_block(w: &mut BitWriter, items: &[Item], bfinal: bool) {
    w.bit(bfinal as u32);
    w.bits(1, 2); // btype = 1
    let lit_lens = fixed_lit_lens();
    let lit_codes = canonical_codes(&lit_lens);
    let dist_lens = vec![5u8; 32];
    let dist_codes = canonical_codes(&dist_lens);
    emit_items(w, items, &lit_lens, &lit_codes, &dist_lens, &dist_codes);
}

fn emit_items(
    w: &mut BitWriter,
    items: &[Item],
    lit_lens: &[u8],
    lit_codes: &[u32],
    dist_lens: &[u8],
    dist_codes: &[u32],
) {
    for it in items {
        match *it {
            Item::Lit(b) => {
                let s = b as usize;
                assert!(lit_lens[s] != 0, "literal {s} has no code");
                w.code(lit_codes[s], lit_lens[s] as usize);
            }
            Item::Match { len, dist } => {
                let ls = len_sym(len);
                let s = 257 + ls;
                assert!(lit_lens[s] != 0, "length symbol {s} has no code");
                w.code(lit_codes[s], lit_lens[s] as usize);
                w.bits(len - LEN_BASE[ls], LEN_EXTRA[ls] as usize);
                let ds = dist_sym(dist);
                assert!(dist_lens[ds] != 0, "distance symbol {ds} has no code");
                w.code(dist_codes[ds], dist_lens[ds] as usize);
                w.bits(dist - DIST_BASE[ds], DIST_EXTRA[ds] as usize);
            }
        }
    }
    assert!(lit_lens[256] != 0, "end-of-block symbol has no code");
    w.code(lit_codes[256], lit_lens[256] as usize);
}

// ---------------------------------------------------------------------------
// Dynamic-Huffman blocks (btype == 2)
// ---------------------------------------------------------------------------

/// `cp_permutation_order`
pub const PERM: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// How to encode the code-length sequence of a dynamic header.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ClMode {
    /// only symbols 0..15 -- exercises `cp_dynamic`'s `default:` arm
    Raw,
    /// use 16/17/18 wherever they apply -- exercises all three run arms
    Rle,
}

/// Run-length-encodes `seq` (the concatenated literal and distance code
/// lengths) into `(symbol, extra_value, extra_bits)` triples.
pub fn encode_cl(seq: &[u8], mode: ClMode) -> Vec<(usize, u32, usize)> {
    let mut out = Vec::new();
    if mode == ClMode::Raw {
        for &v in seq {
            out.push((v as usize, 0, 0));
        }
        return out;
    }
    let mut i = 0;
    while i < seq.len() {
        let v = seq[i];
        let mut run = 1;
        while i + run < seq.len() && seq[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let mut left = run;
            while left >= 11 {
                let take = left.min(138);
                out.push((18, (take - 11) as u32, 7));
                left -= take;
            }
            while left >= 3 {
                let take = left.min(10);
                out.push((17, (take - 3) as u32, 3));
                left -= take;
            }
            for _ in 0..left {
                out.push((0, 0, 0));
            }
        } else {
            out.push((v as usize, 0, 0));
            let mut left = run - 1;
            while left >= 3 {
                let take = left.min(6);
                out.push((16, (take - 3) as u32, 2));
                left -= take;
            }
            for _ in 0..left {
                out.push((v as usize, 0, 0));
            }
        }
        i += run;
    }
    out
}

pub struct DynSpec {
    pub lit_lens: Vec<u8>,
    pub dist_lens: Vec<u8>,
    pub cl_mode: ClMode,
    /// force `nlen` (4..=19); `None` = the minimum that carries all used
    /// code-length symbols
    pub force_nlen: Option<usize>,
    /// force `nlit` (257..=288) / `ndst` (1..=32); `None` = derived from the
    /// length vectors
    pub force_nlit: Option<usize>,
    pub force_ndst: Option<usize>,
}

impl DynSpec {
    pub fn new(lit_lens: Vec<u8>, dist_lens: Vec<u8>) -> DynSpec {
        DynSpec {
            lit_lens,
            dist_lens,
            cl_mode: ClMode::Rle,
            force_nlen: None,
            force_nlit: None,
            force_ndst: None,
        }
    }
}

pub fn dynamic_block(w: &mut BitWriter, spec: &DynSpec, items: &[Item], bfinal: bool) {
    let nlit = spec.force_nlit.unwrap_or_else(|| {
        let last = (0..spec.lit_lens.len())
            .rev()
            .find(|&i| spec.lit_lens[i] != 0)
            .unwrap_or(256);
        (last + 1).max(257)
    });
    let ndst = spec.force_ndst.unwrap_or_else(|| {
        let last = (0..spec.dist_lens.len())
            .rev()
            .find(|&i| spec.dist_lens[i] != 0);
        match last {
            Some(l) => l + 1,
            None => 1,
        }
    });
    assert!((257..=288).contains(&nlit), "nlit {nlit} out of range");
    assert!((1..=32).contains(&ndst), "ndst {ndst} out of range");

    let mut lit_lens = spec.lit_lens.clone();
    lit_lens.resize(288, 0);
    let mut dist_lens = spec.dist_lens.clone();
    dist_lens.resize(32, 0);

    let mut seq: Vec<u8> = lit_lens[..nlit].to_vec();
    seq.extend_from_slice(&dist_lens[..ndst]);
    let cl = encode_cl(&seq, spec.cl_mode);

    let mut freq = vec![0u64; 19];
    for &(s, _, _) in &cl {
        freq[s] += 1;
    }
    // the 3-bit header field caps code-length code lengths at 7
    let cl_lens = lens_with_limit(&freq, 7);
    assert!(
        cl_lens.iter().all(|&l| l <= 7),
        "code-length code length exceeds the 3-bit header field: {cl_lens:?}"
    );
    assert_eq!(
        kraft(&cl_lens),
        1 << 15,
        "code-length code is incomplete: {cl_lens:?}"
    );
    let cl_codes = canonical_codes(&cl_lens);

    let nlen = spec.force_nlen.unwrap_or_else(|| {
        let mut n = 4;
        for i in 0..19 {
            if cl_lens[PERM[i]] != 0 {
                n = n.max(i + 1);
            }
        }
        n
    });
    assert!((4..=19).contains(&nlen), "nlen {nlen} out of range");
    for i in nlen..19 {
        assert_eq!(
            cl_lens[PERM[i]], 0,
            "nlen={nlen} drops code-length symbol {}",
            PERM[i]
        );
    }

    w.bit(bfinal as u32);
    w.bits(2, 2); // btype = 2
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.bits(cl_lens[PERM[i]] as u32, 3);
    }
    for &(s, extra, nbits) in &cl {
        assert!(cl_lens[s] != 0, "code-length symbol {s} has no code");
        w.code(cl_codes[s], cl_lens[s] as usize);
        w.bits(extra, nbits);
    }

    assert!(
        lit_lens.iter().all(|&l| l <= 15),
        "literal code length exceeds 15"
    );
    assert!(
        dist_lens.iter().all(|&l| l <= 15),
        "distance code length exceeds 15"
    );
    let lit_codes = canonical_codes(&lit_lens);
    let dist_codes = canonical_codes(&dist_lens);
    emit_items(w, items, &lit_lens, &lit_codes, &dist_lens, &dist_codes);
}

/// Derives complete literal/length and distance code-length vectors from the
/// symbols `items` actually uses, so `dynamic_block` never has to emit a symbol
/// without a code.
pub fn lens_for(items: &[Item], extra_lit_weight: &[(usize, u64)]) -> (Vec<u8>, Vec<u8>) {
    let mut lw = vec![0u64; 288];
    let mut dw = vec![0u64; 32];
    lw[256] = 1; // end-of-block always present
    for it in items {
        match *it {
            Item::Lit(b) => lw[b as usize] += 3,
            Item::Match { len, dist } => {
                lw[257 + len_sym(len)] += 3;
                dw[dist_sym(dist)] += 3;
            }
        }
    }
    for &(s, wgt) in extra_lit_weight {
        lw[s] += wgt;
    }
    let lit = lens_with_limit(&lw, 15);
    let dist = if dw.iter().all(|&x| x == 0) {
        // no matches: still need at least one distance code in the header
        let mut d = vec![0u8; 32];
        d[0] = 1;
        d
    } else {
        lens_with_limit(&dw, 15)
    };
    (lit, dist)
}

/// A random item stream whose matches are always in range.
pub fn random_items(rng: &mut Rng, n: usize, alphabet: usize, max_dist: u32) -> Vec<Item> {
    let mut items = Vec::new();
    let mut produced: u32 = 0;
    for _ in 0..n {
        if produced >= 3 && rng.below(3) == 0 {
            let dist = 1 + rng.below((produced.min(max_dist)) as usize) as u32;
            let len = 3 + rng.below(60) as u32;
            items.push(Item::Match { len, dist });
            produced += len;
        } else {
            let b = (rng.below(alphabet.max(1))) as u8;
            items.push(Item::Lit(b));
            produced += 1;
        }
    }
    items
}

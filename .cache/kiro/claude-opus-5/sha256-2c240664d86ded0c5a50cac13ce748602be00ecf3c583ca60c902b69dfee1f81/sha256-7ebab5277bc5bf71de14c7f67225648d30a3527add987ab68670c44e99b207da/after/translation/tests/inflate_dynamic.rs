//! Level 1b: hand-built *dynamic* Huffman blocks, to reach corners of
//! `cp_dynamic` / `cp_build` / `cp_decode` that zlib never emits: minimal
//! HLIT/HDIST, single-symbol trees, 15-bit codes, and every code-length symbol
//! (including the 16/17/18 repeat forms).

mod common;

use common::*;

const PERM: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

struct W {
    bytes: Vec<u8>,
    nbits: u32,
}

impl W {
    fn new() -> W {
        W {
            bytes: Vec::new(),
            nbits: 0,
        }
    }
    fn bits(&mut self, val: u32, n: u32) {
        for i in 0..n {
            if self.nbits % 8 == 0 {
                self.bytes.push(0);
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= (((val >> i) & 1) as u8) << (self.nbits % 8);
            self.nbits += 1;
        }
    }
    fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
}

/// Kraft sum scaled by 2^40 (lengths are <= 15 so this cannot overflow).
fn kraft_sum(lens: &[u8]) -> u64 {
    lens.iter()
        .filter(|&&l| l != 0)
        .map(|&l| 1u64 << (40 - l as u32))
        .sum()
}

fn kraft_ok(lens: &[u8]) -> bool {
    kraft_sum(lens) <= (1u64 << 40)
}

/// Canonical Huffman code assignment, identical to what `cp_build` derives.
fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0u32; 16];
    for &l in lens {
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0u32;
    for l in 1..16 {
        code = (code + counts[l - 1]) << 1;
        next[l] = code;
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

/// Run-length encodes a code-length vector using the 16/17/18 symbols where
/// `use_runs` is set, otherwise emits every length literally.
fn encode_lens(all: &[u8], use_runs: bool) -> Vec<(u8, u32, u32)> {
    // (code-length symbol, extra value, extra bit count)
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let v = all[i];
        let mut run = 1usize;
        while i + run < all.len() && all[i + run] == v {
            run += 1;
        }
        if use_runs && v == 0 && run >= 11 {
            let take = run.min(138);
            out.push((18u8, (take - 11) as u32, 7));
            i += take;
        } else if use_runs && v == 0 && run >= 3 {
            let take = run.min(10);
            out.push((17u8, (take - 3) as u32, 3));
            i += take;
        } else if use_runs && v != 0 && run >= 4 && i > 0 {
            out.push((v, 0, 0));
            let mut left = run - 1;
            i += 1;
            while left >= 3 {
                let take = left.min(6);
                out.push((16u8, (take - 3) as u32, 2));
                left -= take;
                i += take;
            }
            for _ in 0..left {
                out.push((v, 0, 0));
                i += 1;
            }
        } else {
            out.push((v, 0, 0));
            i += 1;
        }
    }
    out
}

/// Symbols to emit inside the block: literals, back references, end-of-block.
enum Sym {
    Lit(u32),
    Ref(u32, u32),
}

/// Emits one final dynamic block.
fn dynamic_block(
    lit_lens: &[u8],
    dist_lens: &[u8],
    syms: &[Sym],
    use_runs: bool,
) -> (Vec<u8>, usize) {
    assert!((257..=288).contains(&lit_lens.len()));
    assert!((1..=32).contains(&dist_lens.len()));
    assert!(
        kraft_sum(lit_lens) == (1u64 << 40),
        "literal tree is not complete (kraft = {}/2^40)",
        kraft_sum(lit_lens)
    );
    assert!(kraft_ok(dist_lens), "distance tree over-subscribed");

    let mut all: Vec<u8> = Vec::new();
    all.extend_from_slice(lit_lens);
    all.extend_from_slice(dist_lens);
    let encoded = encode_lens(&all, use_runs);

    // code lengths for the code-length alphabet: uniform weights keep the tree
    // shallow (<= 5 bits for 19 symbols) and, crucially, complete.
    let mut cl_counts = [0usize; 19];
    for &(sym, _, _) in &encoded {
        cl_counts[sym as usize] = 1;
    }
    let cl_lens = huffman_lengths(&cl_counts, 7);
    assert!(kraft_ok(&cl_lens), "code-length tree over-subscribed");
    let cl_codes = canonical(&cl_lens);

    let mut hclen = 19usize;
    while hclen > 4 && cl_lens[PERM[hclen - 1]] == 0 {
        hclen -= 1;
    }

    let mut w = W::new();
    w.bits(1, 1); // BFINAL
    w.bits(2, 2); // BTYPE = 10 (dynamic)
    w.bits((lit_lens.len() - 257) as u32, 5);
    w.bits((dist_lens.len() - 1) as u32, 5);
    w.bits((hclen - 4) as u32, 4);
    for i in 0..hclen {
        w.bits(cl_lens[PERM[i]] as u32, 3);
    }
    for &(sym, extra, nextra) in &encoded {
        w.code(cl_codes[sym as usize], cl_lens[sym as usize] as u32);
        if nextra > 0 {
            w.bits(extra, nextra);
        }
    }

    let lit_codes = canonical(lit_lens);
    let dist_codes = canonical(dist_lens);
    let mut outlen = 0usize;
    for s in syms {
        match *s {
            Sym::Lit(v) => {
                assert!(lit_lens[v as usize] != 0, "literal {v} has no code");
                w.code(lit_codes[v as usize], lit_lens[v as usize] as u32);
                outlen += 1;
            }
            Sym::Ref(len, dist) => {
                let li = (0..29).rev().find(|&i| LEN_BASE[i] <= len).unwrap();
                let ls = 257 + li;
                assert!(lit_lens[ls] != 0, "length symbol {ls} has no code");
                w.code(lit_codes[ls], lit_lens[ls] as u32);
                w.bits(len - LEN_BASE[li], LEN_EXTRA[li]);
                let di = (0..30).rev().find(|&i| DIST_BASE[i] <= dist).unwrap();
                assert!(di < dist_lens.len() && dist_lens[di] != 0, "dist sym {di} unusable");
                w.code(dist_codes[di], dist_lens[di] as u32);
                w.bits(dist - DIST_BASE[di], DIST_EXTRA[di]);
                outlen += len as usize;
            }
        }
    }
    // end of block
    assert!(lit_lens[256] != 0, "EOB has no code");
    w.code(lit_codes[256], lit_lens[256] as u32);
    (w.bytes, outlen)
}

/// Package-merge-free Huffman length computation good enough for the small
/// alphabets used here: repeatedly merge the two lowest-weight nodes.
fn huffman_lengths(counts: &[usize], max_len: u8) -> Vec<u8> {
    let n = counts.len();
    let used: Vec<usize> = (0..n).filter(|&i| counts[i] > 0).collect();
    let mut lens = vec![0u8; n];
    if used.is_empty() {
        return lens;
    }
    if used.len() == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    // nodes: (weight, symbols-in-subtree)
    let mut nodes: Vec<(usize, Vec<usize>)> =
        used.iter().map(|&i| (counts[i], vec![i])).collect();
    while nodes.len() > 1 {
        nodes.sort_by_key(|x| (x.0, x.1[0]));
        let a = nodes.remove(0);
        let b = nodes.remove(0);
        for &s in a.1.iter().chain(b.1.iter()) {
            lens[s] += 1;
        }
        let mut syms = a.1;
        syms.extend(b.1);
        nodes.push((a.0 + b.0, syms));
    }
    // Huffman trees are always complete, so no clamping should ever be needed.
    assert!(
        lens.iter().all(|&l| l <= max_len),
        "code-length tree deeper than {max_len} bits"
    );
    lens
}

fn compare(label: &str, stream: &[u8], out_bytes: usize) {
    let p = pair();
    for align in 0..4 {
        let input = AlignedInput::new(stream, align);
        let c = run_inflate_backed(&p.c, &input, out_bytes, out_bytes + 4096);
        let r = run_inflate_backed(&p.rs, &input, out_bytes, out_bytes + 4096);
        assert_inflate_eq(&format!("{label} align={align}"), &c, &r);
    }
}

/// Minimal dynamic block: HLIT = 257, HDIST = 1, single-symbol trees.
#[test]
fn minimal_dynamic_block() {
    for use_runs in [false, true] {
        let mut lit = vec![0u8; 257];
        lit[256] = 1;
        lit[0] = 1;
        let dist = vec![1u8; 1];
        let syms: Vec<Sym> = (0..10).map(|_| Sym::Lit(0)).collect();
        let (s, n) = dynamic_block(&lit, &dist, &syms, use_runs);
        compare(&format!("minimal_runs{use_runs}"), &s, n);
        compare(&format!("minimal_runs{use_runs}_slack"), &s, n + 100);
        if n > 1 {
            compare(&format!("minimal_runs{use_runs}_short"), &s, n - 1);
        }
    }
}

/// A complete literal tree in which `2^l` symbols share the code length `l`:
/// `2^l - 2` literals plus the end-of-block symbol 256 and the length symbol
/// 257. Everything else is unused, which also produces long runs of zero code
/// lengths for the `cp_dynamic` 17/18 symbols to encode.
fn flat_lit_lens(l: u8) -> Vec<u8> {
    let literals = (1usize << l) - 2;
    assert!(literals <= 256);
    let mut lens = vec![0u8; 288];
    for i in 0..literals {
        lens[i] = l;
    }
    lens[256] = l;
    lens[257] = l;
    lens
}

/// A complete distance tree with `2^l` codes of length `l`.
fn flat_dist_lens(l: u8) -> Vec<u8> {
    let n = 1usize << l;
    assert!(n <= 32);
    vec![l; n]
}

/// A tree spanning every code length from 1 to 15, so `cp_decode`'s binary
/// search runs to full depth and `cp_build` fills slots for lengths beyond the
/// 9-bit fast-lookup range.
fn deep_lit_lens() -> Vec<u8> {
    let mut lens = vec![0u8; 288];
    for i in 0..13usize {
        lens[i] = (i + 1) as u8; // lengths 1..13
    }
    lens[13] = 14;
    lens[256] = 15;
    lens[257] = 15;
    lens
}

/// A tree where every literal has the same length as the fixed table, plus a
/// tree needing 15-bit codes.
#[test]
fn wide_and_deep_trees() {
    // 1) the fixed-table shape, but transmitted as a dynamic block
    let mut lit = vec![9u8; 288];
    for i in 0..144 {
        lit[i] = 8;
    }
    for i in 256..280 {
        lit[i] = 7;
    }
    for i in 280..288 {
        lit[i] = 8;
    }
    let dist = flat_dist_lens(5);
    let mut syms: Vec<Sym> = (0..=255u32).map(Sym::Lit).collect();
    syms.push(Sym::Ref(3, 1));
    syms.push(Sym::Ref(258, 300));
    for use_runs in [false, true] {
        let (s, n) = dynamic_block(&lit, &dist, &syms, use_runs);
        compare(&format!("wide_runs{use_runs}"), &s, n);
        compare(&format!("wide_runs{use_runs}_slack"), &s, n + 200);
        compare(&format!("wide_runs{use_runs}_short"), &s, n - 1);
    }

    // 2) codes from 1 to 15 bits long
    let lit2 = deep_lit_lens();
    let dist2 = flat_dist_lens(1);
    let mut syms2: Vec<Sym> = Vec::new();
    for rep in 0..3 {
        for i in 0..14u32 {
            syms2.push(Sym::Lit(i));
        }
        if rep > 0 {
            syms2.push(Sym::Ref(3, 2));
        }
    }
    for use_runs in [false, true] {
        let (s, n) = dynamic_block(&lit2, &dist2, &syms2, use_runs);
        compare(&format!("deep_runs{use_runs}"), &s, n);
        compare(&format!("deep_runs{use_runs}_slack"), &s, n + 64);
        compare(&format!("deep_runs{use_runs}_short"), &s, n - 1);
    }

    // 3) every flat width, which sweeps the zero-run lengths the code-length
    //    alphabet has to encode (2 zeros for l=8 up to 254 for l=1)
    for l in 1..=8u8 {
        let lit = flat_lit_lens(l);
        let dist = flat_dist_lens(if l > 5 { 5 } else { l });
        let literals = (1u32 << l) - 2;
        let mut syms: Vec<Sym> = (0..literals.min(20)).map(Sym::Lit).collect();
        if literals >= 3 {
            syms.push(Sym::Ref(3, 2));
        }
        for use_runs in [false, true] {
            let (s, n) = dynamic_block(&lit, &dist, &syms, use_runs);
            compare(&format!("flat{l}_runs{use_runs}"), &s, n);
        }
    }
}

/// Exercises the code-length repeat symbols 16, 17 and 18. `flat_lit_lens`
/// gives long zero runs; `dist_lens` of varying width gives short ones, and the
/// repeated equal lengths make the encoder emit symbol 16.
#[test]
fn code_length_repeat_symbols() {
    for l in 1..=8u8 {
        for dl in 0..=5u8 {
            let lit = flat_lit_lens(l);
            let mut dist = flat_dist_lens(dl);
            // pad the distance table with unused (zero-length) entries so the
            // 17/18 zero-run symbols have to cross the lit/dist boundary too
            for extra in [0usize, 3, 11, 31 - dist.len().min(31)] {
                let mut d = dist.clone();
                for _ in 0..extra {
                    if d.len() < 32 {
                        d.push(0);
                    }
                }
                let literals = (1u32 << l) - 2;
                let mut syms: Vec<Sym> = (0..literals.min(8)).map(Sym::Lit).collect();
                if literals >= 3 && d.iter().any(|&x| x != 0) {
                    syms.push(Sym::Ref(3, 1));
                }
                let (s, n) = dynamic_block(&lit, &d, &syms, true);
                compare(&format!("cl_l{l}_dl{dl}_pad{extra}"), &s, n);
            }
            dist.clear();
        }
    }
}

/// Every length symbol (257..285) and every usable distance symbol.
#[test]
fn all_length_and_distance_symbols() {
    let lit: Vec<u8> = {
        let mut counts = vec![0usize; 288];
        counts[256] = 1;
        counts[0] = 1;
        for i in 257..286 {
            counts[i] = 1;
        }
        let mut l = huffman_lengths(&counts, 15);
        l.resize(288, 0);
        l
    };
    let dist = vec![5u8; 32];
    for li in 0..29usize {
        for &di in &[0usize, 1, 4, 8, 15, 22, 29] {
            let length = LEN_BASE[li];
            let dist_v = DIST_BASE[di];
            // need `dist_v` bytes of history first
            let mut syms: Vec<Sym> = Vec::new();
            for _ in 0..dist_v {
                syms.push(Sym::Lit(0));
            }
            syms.push(Sym::Ref(length, dist_v));
            let (s, n) = dynamic_block(&lit, &dist, &syms, true);
            compare(&format!("lensym{li}_distsym{di}"), &s, n);
            // and the maximum value of the extra-bit field
            let max_len = length + ((1u32 << LEN_EXTRA[li]) - 1);
            if LEN_EXTRA[li] > 0 && li < 28 {
                let mut syms: Vec<Sym> = Vec::new();
                for _ in 0..dist_v {
                    syms.push(Sym::Lit(0));
                }
                syms.push(Sym::Ref(max_len, dist_v + ((1 << DIST_EXTRA[di]) - 1)));
                let (s, n) = dynamic_block(&lit, &dist, &syms, true);
                compare(&format!("lensym{li}max_distsym{di}max"), &s, n);
            }
        }
    }
}

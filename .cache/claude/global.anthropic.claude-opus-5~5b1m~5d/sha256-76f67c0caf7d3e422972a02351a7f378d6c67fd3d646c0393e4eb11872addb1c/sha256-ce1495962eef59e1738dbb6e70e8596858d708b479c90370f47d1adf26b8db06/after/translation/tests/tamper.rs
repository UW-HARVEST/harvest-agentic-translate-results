//! CONFIGS.md rows 41–44: tampering with the exported writable data symbols.
//!
//! `dlopen` returns the same mapping for every `Library::new` in a process, so
//! these globals are shared by every test in the binary.  They therefore live in
//! their own test binary, driven by a SINGLE `#[test]`, so no other test can
//! observe a half-tampered table.

mod common;

use common::*;

fn read_u8s(p: &Pair, sym: &[u8], n: usize) -> (Vec<u8>, Vec<u8>) {
    let a: *mut u8 = p.c.data(sym);
    let b: *mut u8 = p.rs.data(sym);
    unsafe {
        (
            std::slice::from_raw_parts(a, n).to_vec(),
            std::slice::from_raw_parts(b, n).to_vec(),
        )
    }
}

#[test]
fn tamper_all_globals() {
    g08_tamper_fixed_table();
    g09_tamper_len_tables();
    g10_tamper_dist_tables();
    g11_tamper_permutation_order();
}

/// Saves a data symbol's contents in both libraries and restores them on drop,
/// so a failed assertion cannot leak a tampered table into the next scenario.
struct Saved<T: Copy> {
    c: *mut T,
    rs: *mut T,
    n: usize,
    old: Vec<T>,
}

impl<T: Copy> Saved<T> {
    fn new(p: &Pair, sym: &[u8], n: usize) -> Saved<T> {
        let c: *mut T = p.c.data(sym);
        let rs: *mut T = p.rs.data(sym);
        let old = unsafe { std::slice::from_raw_parts(c, n).to_vec() };
        Saved { c, rs, n, old }
    }
    fn write(&self, v: &[T]) {
        assert_eq!(v.len(), self.n);
        unsafe {
            std::ptr::copy_nonoverlapping(v.as_ptr(), self.c, self.n);
            std::ptr::copy_nonoverlapping(v.as_ptr(), self.rs, self.n);
        }
    }
}

impl<T: Copy> Drop for Saved<T> {
    fn drop(&mut self) {
        unsafe {
            std::ptr::copy_nonoverlapping(self.old.as_ptr(), self.c, self.n);
            std::ptr::copy_nonoverlapping(self.old.as_ptr(), self.rs, self.n);
        }
    }
}

/// CONFIGS row 41: swap `cp_fixed_table` for a different, still-valid
/// (`len < 16`) assignment and decode a BTYPE=1 block through it.
fn g08_tamper_fixed_table() {
    let p = pair();
    // A complete tree over all 288 literal/length symbols with a different
    // shape: 32 codes of length 8 and 256 of length 9
    // (Kraft = 32/256 + 256/512 = 1).
    let mut lit = vec![9u8; 288];
    for i in 0..32 {
        lit[i] = 8;
    }
    let dst = vec![5u8; 32];

    let mut table = Vec::with_capacity(320);
    table.extend_from_slice(&lit);
    table.extend_from_slice(&dst);

    let saved = Saved::<u8>::new(&p, b"cp_fixed_table\0", 320);
    saved.write(&table);

    let lit_h = Huff::new(lit.clone());
    let dst_h = Huff::new(dst.clone());

    let mut rng = Rng::new(SEED ^ 0xF1);
    for case in 0..64 {
        let n = rng.below(40) as usize;
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
        let mut w = BitWriter::new();
        w.bit(1);
        w.bits_lsb(1, 2);
        write_items(&mut w, &lit_h, &dst_h, &items);
        let mut stream = w.bytes.clone();
        stream.extend_from_slice(&[0u8; 4]);
        let mut expect = Vec::new();
        expand(&items, &mut expect);
        diff_inflate_expect(&p, &stream, &expect, &format!("g08/case{case}"));
    }

    // also with matches, so the tampered length/distance codes are used
    for case in 0..48 {
        let prefix = rng.range(1, 20) as usize;
        let mut items: Vec<Item> = (0..prefix).map(|_| Item::Lit(rng.u8())).collect();
        let dist = rng.range(1, prefix as u32);
        let len = rng.range(3, 40);
        items.push(Item::Match(len, dist));
        let mut w = BitWriter::new();
        w.bit(1);
        w.bits_lsb(1, 2);
        write_items(&mut w, &lit_h, &dst_h, &items);
        let mut stream = w.bytes.clone();
        stream.extend_from_slice(&[0u8; 4]);
        let mut expect = Vec::new();
        expand(&items, &mut expect);
        diff_inflate_expect(&p, &stream, &expect, &format!("g08b/case{case}"));
    }

    drop(saved);

    // and the tables really were restored, identically in both libraries
    let (c, r) = read_u8s(&p, b"cp_fixed_table\0", 320);
    assert_eq!(c, r);
    assert_eq!(&c[..144], &[8u8; 144][..]);
}

/// CONFIGS row 42: tamper `cp_len_base` / `cp_len_extra_bits`.
fn g09_tamper_len_tables() {
    let p = pair();
    let saved_base = Saved::<u32>::new(&p, b"cp_len_base\0", 31);
    let saved_extra = Saved::<u8>::new(&p, b"cp_len_extra_bits\0", 31);

    // Every length symbol becomes "base 4 + 1 extra bit" => length 4 or 5.
    saved_base.write(&vec![4u32; 31]);
    saved_extra.write(&vec![1u8; 31]);

    let lit_h = Huff::new(fixed_lit_lens());
    let dst_h = Huff::new(fixed_dist_lens());

    let mut rng = Rng::new(SEED ^ 0xF2);
    for case in 0..48 {
        let mut items: Vec<Item> = (0..8).map(|_| Item::Lit(rng.u8())).collect();
        let len_idx = rng.below(29) as usize;
        let len_extra = rng.below(2);
        let dist_idx = rng.below(3) as usize; // distances 1..4, all 0 extra bits
        items.push(Item::RawMatch { len_idx, len_extra, dist_idx, dist_extra: 0 });

        let mut w = BitWriter::new();
        w.bit(1);
        w.bits_lsb(1, 2);
        write_items_tables(&mut w, &lit_h, &dst_h, &items, &[1u8; 31], &DIST_EXTRA32);
        let mut stream = w.bytes.clone();
        stream.extend_from_slice(&[0u8; 4]);

        // model the tampered tables
        let mut expect: Vec<u8> = Vec::new();
        for it in &items {
            match *it {
                Item::Lit(b) => expect.push(b),
                Item::RawMatch { len_extra, dist_idx, .. } => {
                    let length = 4 + len_extra;
                    let dist = DIST_BASE[dist_idx];
                    let start = expect.len() - dist as usize;
                    for k in 0..length as usize {
                        let b = expect[start + k];
                        expect.push(b);
                    }
                }
                _ => unreachable!(),
            }
        }
        diff_inflate_expect(&p, &stream, &expect, &format!("g09/case{case}"));
    }
}

/// CONFIGS row 43: tamper `cp_dist_base` / `cp_dist_extra_bits`.
fn g10_tamper_dist_tables() {
    let p = pair();
    let saved_base = Saved::<u32>::new(&p, b"cp_dist_base\0", 32);
    let saved_extra = Saved::<u8>::new(&p, b"cp_dist_extra_bits\0", 32);

    // Every distance symbol becomes "base 2 + 1 extra bit" => distance 2 or 3.
    saved_base.write(&vec![2u32; 32]);
    saved_extra.write(&vec![1u8; 32]);

    let lit_h = Huff::new(fixed_lit_lens());
    let dst_h = Huff::new(fixed_dist_lens());

    let mut rng = Rng::new(SEED ^ 0xF3);
    for case in 0..48 {
        let mut items: Vec<Item> = (0..8).map(|_| Item::Lit(rng.u8())).collect();
        let len_idx = rng.below(8) as usize; // 0 extra bits => length 3..10
        let dist_idx = rng.below(30) as usize;
        let dist_extra = rng.below(2);
        items.push(Item::RawMatch { len_idx, len_extra: 0, dist_idx, dist_extra });

        let mut w = BitWriter::new();
        w.bit(1);
        w.bits_lsb(1, 2);
        write_items_tables(&mut w, &lit_h, &dst_h, &items, &LEN_EXTRA31, &[1u8; 32]);
        let mut stream = w.bytes.clone();
        stream.extend_from_slice(&[0u8; 4]);

        let mut expect: Vec<u8> = Vec::new();
        for it in &items {
            match *it {
                Item::Lit(b) => expect.push(b),
                Item::RawMatch { len_idx, dist_extra, .. } => {
                    let length = LEN_BASE[len_idx];
                    let dist = 2 + dist_extra;
                    let start = expect.len() - dist as usize;
                    for k in 0..length as usize {
                        let b = expect[start + k];
                        expect.push(b);
                    }
                }
                _ => unreachable!(),
            }
        }
        diff_inflate_expect(&p, &stream, &expect, &format!("g10/case{case}"));
    }
}

/// CONFIGS row 44: permute `cp_permutation_order` (still a permutation of
/// `0..=18`) and encode the dynamic header with the *same* permutation.
fn g11_tamper_permutation_order() {
    let p = pair();
    let saved = Saved::<u8>::new(&p, b"cp_permutation_order\0", 19);

    // reversed order
    let mut perm_u8 = [0u8; 19];
    for i in 0..19 {
        perm_u8[i] = (18 - i) as u8;
    }
    saved.write(&perm_u8);
    let mut perm = [0usize; 19];
    for i in 0..19 {
        perm[i] = perm_u8[i] as usize;
    }

    let mut rng = Rng::new(SEED ^ 0xF4);
    for case in 0..32 {
        let n = rng.range(1, 30) as usize;
        let data: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        let mut used: Vec<usize> = data.iter().map(|&b| b as usize).collect();
        used.push(256);
        let lit_lens = balanced_lens(257, &used);
        let dst_lens = vec![1u8; 1];
        let cl = cl_stream_literal(&lit_lens, &dst_lens);
        let (cl_lens, _) = cl_lens_for(&cl);
        // with a reversed permutation, nlen must cover the largest position of
        // any used code-length symbol in the *tampered* order
        let mut nlen = 4usize;
        for (pos, &q) in perm.iter().enumerate() {
            if cl_lens[q] != 0 {
                nlen = nlen.max(pos + 1);
            }
        }
        let items: Vec<Item> = data.iter().map(|&b| Item::Lit(b)).collect();

        let mut w = BitWriter::new();
        write_dynamic_block(
            &mut w, true, &lit_lens, &dst_lens, &cl, &cl_lens, nlen, &perm, &items,
        );
        let mut stream = w.bytes.clone();
        stream.extend_from_slice(&[0u8; 4]);
        diff_inflate_expect(&p, &stream, &data, &format!("g11/case{case}"));
    }
}

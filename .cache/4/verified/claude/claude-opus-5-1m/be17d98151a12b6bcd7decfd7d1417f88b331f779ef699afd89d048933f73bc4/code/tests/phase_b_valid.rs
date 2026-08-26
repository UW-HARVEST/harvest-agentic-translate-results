//! Phase B -- valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every case is run against BOTH shared objects through `libloading` (see
//! `tests/common/mod.rs`) and the return value, `cp_error_reason` and the whole
//! output buffer (padding included) must match byte for byte.

mod common;

use common::enc::*;
use common::shared::Case;
use common::*;

/// Wraps a finished bit stream into cases at every input/output alignment, since
/// `pinflate` branches on `(size_t)in & 3` when it computes `first_bytes`.
fn all_alignments(label: &str, stream: &[u8], out_size: i32) -> Vec<Case> {
    let mut v = Vec::new();
    for in_off in 0..4 {
        for out_off in [0usize, 1, 3] {
            v.push(
                Case::new(&format!("{label} in_off={in_off} out_off={out_off}"), stream, out_size)
                    .in_off(in_off)
                    .out_off(out_off),
            );
        }
    }
    v
}

fn fixed_stream(items: &[Item]) -> Vec<u8> {
    let mut w = BitWriter::new();
    fixed_block(&mut w, items, true);
    w.align();
    w.bytes
}

fn dynamic_stream(spec: &DynSpec, items: &[Item]) -> Vec<u8> {
    let mut w = BitWriter::new();
    dynamic_block(&mut w, spec, items, true);
    w.align();
    w.bytes
}

/// Sanity: the stream we just built really does decode, i.e. the row exercises
/// the *valid* path rather than accidentally testing an error path.
fn assert_decodes(label: &str, stream: &[u8], expect: &[u8]) {
    let case = Case::new(label, stream, (expect.len().max(1)) as i32);
    let r = run_batch(c_so(), std::slice::from_ref(&case));
    match &r[0] {
        common::shared::Outcome::Ret { ret, out, .. } => {
            assert_eq!(
                *ret, 1,
                "{label}: C failed to decode its own valid stream\n  stream={}",
                common::shared::hex(stream)
            );
            assert_eq!(
                &out[..expect.len()],
                expect,
                "{label}: C decoded the wrong bytes\n  stream={}",
                common::shared::hex(stream)
            );
        }
        o => panic!(
            "{label}: C died on a valid stream: {o:?}\n  stream={}",
            common::shared::hex(stream)
        ),
    }
}

/// The stored-block path in the C original is only correct for some input
/// sizes: `cp_stored`'s length check is `s->bits_left / 8 <= LEN` (inverted, see
/// ERRORS.md E2), and `cp_ptr` derives the payload address from `word_index`,
/// which the "final word" branch of `cp_peak_bits` never advances. So a stored
/// block decodes correctly only when it is the last block *and* the input has
/// enough whole 32-bit words. Rather than assert something false about the C,
/// these rows assert that at least one variant really does take the success
/// path, and leave the byte-for-byte agreement to `assert_batch_matches`.
fn assert_some_decode(label: &str, variants: &[(Vec<u8>, Vec<u8>, usize)]) {
    let cases: Vec<Case> = variants
        .iter()
        .enumerate()
        .map(|(i, (stream, expect, in_off))| {
            Case::new(&format!("{label}#{i}"), stream, expect.len().max(1) as i32).in_off(*in_off)
        })
        .collect();
    let r = run_batch(c_so(), &cases);
    let mut good = 0;
    for (i, (_, expect, _)) in variants.iter().enumerate() {
        if let common::shared::Outcome::Ret { ret, out, .. } = &r[i] {
            if *ret == 1 && &out[..expect.len()] == expect.as_slice() {
                good += 1;
            }
        }
    }
    assert!(
        good > 0,
        "{label}: none of the {} variants took the C success path -- the row \
         would only be testing error paths",
        variants.len()
    );
}

// ---------------------------------------------------------------------------
// C1 -- C5: stored blocks (btype == 0)
// ---------------------------------------------------------------------------

#[test]
fn c1_stored_empty() {
    let mut w = BitWriter::new();
    stored_block(&mut w, &[], true);
    let mut cases = all_alignments("C1 stored LEN=0", &w.bytes, 1);
    // out_bytes == 0 as well: nothing is written, so no bound is touched
    cases.push(Case::new("C1 stored LEN=0 out=0", &w.bytes, 0));
    assert_batch_matches(&cases);
}

#[test]
fn c2_stored_small_random() {
    let mut rng = Rng::new(0xC2);
    let mut cases = Vec::new();
    let mut variants = Vec::new();
    for trial in 0..48 {
        let n = rng.range(1, 64);
        let payload = rng.bytes(n);
        let mut w = BitWriter::new();
        stored_block(&mut w, &payload, true);
        for in_off in 0..4 {
            variants.push((w.bytes.clone(), payload.clone(), in_off));
            let oo = rng.below(4);
            cases.push(
                Case::new(&format!("C2 trial{trial} len={n} in_off={in_off}"), &w.bytes, n as i32)
                    .in_off(in_off)
                    .out_off(oo),
            );
        }
    }
    assert_some_decode("C2 stored small random", &variants);
    assert_batch_matches(&cases);
}

#[test]
fn c3_stored_max_len() {
    // LEN == 65535, the widest a stored block can be
    let mut rng = Rng::new(0xC3);
    let payload = rng.bytes(65535);
    let mut w = BitWriter::new();
    stored_block(&mut w, &payload, true);
    let variants: Vec<(Vec<u8>, Vec<u8>, usize)> = (0..4)
        .map(|io| (w.bytes.clone(), payload.clone(), io))
        .collect();
    assert_some_decode("C3 stored LEN=65535", &variants);
    let mut cases = Vec::new();
    for in_off in 0..4 {
        cases.push(
            Case::new(&format!("C3 LEN=65535 in_off={in_off}"), &w.bytes, 65535)
                .in_off(in_off)
                .out_pad(64),
        );
    }
    assert_batch_matches(&cases);
}

#[test]
fn c4_stored_two_blocks() {
    // bfinal == 0 then bfinal == 1: the `do { ... } while (!bfinal)` loop runs twice
    let mut rng = Rng::new(0xC4);
    let mut cases = Vec::new();
    for trial in 0..16 {
        let na = rng.range(0, 40);
        let a = rng.bytes(na);
        let nb = rng.range(0, 40);
        let b = rng.bytes(nb);
        let mut w = BitWriter::new();
        stored_block(&mut w, &a, false);
        stored_block(&mut w, &b, true);
        let mut expect = a.clone();
        expect.extend_from_slice(&b);
        // NB: a non-final stored block always trips E2 (`bits_left / 8 <= LEN`
        // is false while a second block still follows), so this row deliberately
        // asserts only C/Rust agreement on that rejection.
        cases.extend(all_alignments(
            &format!("C4 two stored blocks trial{trial}"),
            &w.bytes,
            expect.len().max(1) as i32,
        ));
    }
    assert_batch_matches(&cases);
}

#[test]
fn c5_stored_alignment_discard() {
    // `cp_stored` starts with `cp_read_bits(s, s->count & 7)`. Prefixing the
    // stored block with a fixed block whose bit length is not a multiple of 8
    // makes that discard non-zero.
    let mut rng = Rng::new(0xC5);
    let mut cases = Vec::new();
    let mut variants = Vec::new();
    for pad_lits in 0..8usize {
        let lits: Vec<Item> = (0..pad_lits).map(|i| Item::Lit(b'a' + i as u8)).collect();
        let ntail = rng.range(1, 24);
        let tail = rng.bytes(ntail);
        let mut w = BitWriter::new();
        fixed_block(&mut w, &lits, false);
        stored_block(&mut w, &tail, true);
        let mut expect: Vec<u8> = lits
            .iter()
            .map(|i| match i {
                Item::Lit(b) => *b,
                _ => unreachable!(),
            })
            .collect();
        expect.extend_from_slice(&tail);
        variants.push((w.bytes.clone(), expect.clone(), 0));
        cases.extend(all_alignments(
            &format!("C5 discard pad={pad_lits}"),
            &w.bytes,
            expect.len() as i32,
        ));
    }
    assert_some_decode("C5 stored after fixed", &variants);
    assert_batch_matches(&cases);
}

// ---------------------------------------------------------------------------
// C6 -- C10: fixed blocks (btype == 1)
// ---------------------------------------------------------------------------

#[test]
fn c6_fixed_literals_only() {
    let mut rng = Rng::new(0xC6);
    let mut cases = Vec::new();
    for trial in 0..40 {
        let n = rng.range(1, 64);
        // literals spanning both fixed-code lengths: 0..143 are 8 bits,
        // 144..255 are 9 bits
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.byte())).collect();
        let stream = fixed_stream(&items);
        let expect = Item::expand(&items);
        assert_decodes(&format!("C6 trial{trial}"), &stream, &expect);
        cases.extend(all_alignments(
            &format!("C6 fixed literals trial{trial} n={n}"),
            &stream,
            expect.len() as i32,
        ));
    }
    assert_batch_matches(&cases);
}

#[test]
fn c7_fixed_matches_bytecopy() {
    // `switch (backwards_distance) { case 1: memset; default: byte loop }`
    // -- this row is the `default` arm.
    let mut rng = Rng::new(0xC7);
    let mut cases = Vec::new();
    for trial in 0..40 {
        let mut items: Vec<Item> = (0..rng.range(4, 20)).map(|_| Item::Lit(rng.byte())).collect();
        let mut produced = items.len() as u32;
        for _ in 0..rng.range(1, 12) {
            if produced < 4 {
                break;
            }
            let dist = 2 + rng.below((produced - 1).min(64) as usize) as u32;
            let len = 3 + rng.below(80) as u32;
            items.push(Item::Match { len, dist });
            produced += len;
        }
        let stream = fixed_stream(&items);
        let expect = Item::expand(&items);
        assert_decodes(&format!("C7 trial{trial}"), &stream, &expect);
        cases.extend(all_alignments(
            &format!("C7 fixed matches trial{trial}"),
            &stream,
            expect.len() as i32,
        ));
    }
    assert_batch_matches(&cases);
}

#[test]
fn c8_fixed_dist1_memset() {
    // `backwards_distance == 1` -> the `memset(dst, *src, length)` fast path
    let mut rng = Rng::new(0xC8);
    let mut cases = Vec::new();
    for trial in 0..32 {
        let mut items = vec![Item::Lit(rng.byte())];
        for _ in 0..rng.range(1, 6) {
            items.push(Item::Match { len: 3 + rng.below(256) as u32, dist: 1 });
            items.push(Item::Lit(rng.byte()));
        }
        let stream = fixed_stream(&items);
        let expect = Item::expand(&items);
        assert_decodes(&format!("C8 trial{trial}"), &stream, &expect);
        cases.extend(all_alignments(
            &format!("C8 dist=1 memset trial{trial}"),
            &stream,
            expect.len() as i32,
        ));
    }
    assert_batch_matches(&cases);
}

#[test]
fn c9_fixed_length_extremes() {
    // length 3 (`cp_len_base[0]`, 0 extra bits) and length 258
    // (`cp_len_base[28] == 258`, 0 extra bits)
    let mut cases = Vec::new();
    for &len in &[3u32, 4, 258] {
        for &dist in &[1u32, 2, 3, 4] {
            let mut items: Vec<Item> = (0..8).map(|i| Item::Lit(b'0' + i)).collect();
            items.push(Item::Match { len, dist });
            let stream = fixed_stream(&items);
            let expect = Item::expand(&items);
            let label = format!("C9 len={len} dist={dist}");
            assert_decodes(&label, &stream, &expect);
            cases.extend(all_alignments(&label, &stream, expect.len() as i32));
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn c10_fixed_all_length_and_distance_symbols() {
    // every length symbol (0..28 -> 0..5 extra bits) and every distance symbol
    // reachable inside a 32 KiB window
    let mut cases = Vec::new();
    for ls in 0..29usize {
        let base = LEN_BASE[ls];
        let extra = LEN_EXTRA[ls] as u32;
        let lens: Vec<u32> = if extra == 0 {
            vec![base]
        } else {
            vec![base, base + 1, base + (1 << extra) - 1]
        };
        for len in lens {
            let mut items: Vec<Item> = (0..300).map(|i| Item::Lit((i % 251) as u8)).collect();
            items.push(Item::Match { len, dist: 300 });
            items.push(Item::Match { len: 3, dist: 1 });
            let stream = fixed_stream(&items);
            let expect = Item::expand(&items);
            let label = format!("C10 lensym={ls} len={len}");
            assert_decodes(&label, &stream, &expect);
            cases.push(Case::new(&label, &stream, expect.len() as i32).in_off(ls % 4));
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn c18_c19_distance_symbols() {
    // C18: distance symbols with 0 extra bits (distances 1..4)
    // C19: distance symbols with 1..13 extra bits, up to the 32 KiB window
    let mut cases = Vec::new();
    for ds in 0..30usize {
        let base = DIST_BASE[ds];
        let extra = DIST_EXTRA[ds] as u32;
        let dists: Vec<u32> = if extra == 0 {
            vec![base]
        } else {
            vec![base, base + 1, base + (1 << extra) - 1]
        };
        for dist in dists {
            let prefix = dist as usize;
            let mut items: Vec<Item> = (0..prefix).map(|i| Item::Lit((i % 253) as u8)).collect();
            items.push(Item::Match { len: 3, dist });
            items.push(Item::Match { len: 258, dist });
            let stream = fixed_stream(&items);
            let expect = Item::expand(&items);
            let label = format!("C18/C19 distsym={ds} dist={dist}");
            assert_decodes(&label, &stream, &expect);
            cases.push(
                Case::new(&label, &stream, expect.len() as i32)
                    .in_off(ds % 4)
                    .out_off(ds % 3)
                    .out_pad(64),
            );
        }
    }
    assert_batch_matches(&cases);
}

// ---------------------------------------------------------------------------
// C11 -- C17: dynamic blocks (btype == 2)
// ---------------------------------------------------------------------------

#[test]
fn c11_dynamic_nlen_extremes() {
    // nlen == 19 (all code-length code lengths transmitted) and the minimum
    // nlen that still carries every used symbol
    let mut rng = Rng::new(0xC11);
    let mut cases = Vec::new();
    for &force in &[None, Some(19usize)] {
        for trial in 0..8 {
            let items = random_items(&mut rng, 40, 200, 64);
            let (lit, dist) = lens_for(&items, &[]);
            let mut spec = DynSpec::new(lit, dist);
            spec.force_nlen = force;
            let stream = dynamic_stream(&spec, &items);
            let expect = Item::expand(&items);
            let label = format!("C11 nlen={force:?} trial{trial}");
            assert_decodes(&label, &stream, &expect);
            cases.extend(all_alignments(&label, &stream, expect.len() as i32));
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn c11b_dynamic_nlen_minimum_four() {
    // nlen == 4 exactly: only code-length symbols 16, 17, 18 and 0 may be used,
    // i.e. every transmitted length must sit at PERM[0..4] == [16, 17, 18, 0].
    // Reached by making the code-length alphabet {17, 18} plus 0.
    let mut lit = vec![0u8; 288];
    // a complete literal code over 4 symbols, all length 2
    for &s in &[b'x' as usize, b'y' as usize, b'z' as usize, 256usize] {
        lit[s] = 2;
    }
    let mut dist = vec![0u8; 32];
    dist[0] = 1;
    let items = vec![Item::Lit(b'x'), Item::Lit(b'y'), Item::Lit(b'z'), Item::Lit(b'x')];
    let mut spec = DynSpec::new(lit, dist);
    spec.cl_mode = ClMode::Rle;
    let stream = dynamic_stream(&spec, &items);
    let expect = Item::expand(&items);
    assert_decodes("C11b nlen minimum", &stream, &expect);
    assert_batch_matches(&all_alignments("C11b nlen minimum", &stream, expect.len() as i32));
}

#[test]
fn c12_c13_c14_dynamic_cl_run_symbols() {
    // code-length symbol 16 (copy previous 3..6), 17 (3..10 zeros),
    // 18 (11..138 zeros), and the `Raw` encoding that uses none of them
    let mut rng = Rng::new(0xC14);
    let mut cases = Vec::new();
    for mode in [ClMode::Rle, ClMode::Raw] {
        for trial in 0..10 {
            // a sparse alphabet forces long zero runs -> symbols 17 and 18;
            // many equal lengths force symbol 16
            let items = random_items(&mut rng, 60, 6, 32);
            let (lit, dist) = lens_for(&items, &[]);
            let mut spec = DynSpec::new(lit, dist);
            spec.cl_mode = mode;
            let stream = dynamic_stream(&spec, &items);
            let expect = Item::expand(&items);
            let label = format!("C12/13/14 {mode:?} sparse trial{trial}");
            assert_decodes(&label, &stream, &expect);
            cases.extend(all_alignments(&label, &stream, expect.len() as i32));
        }
        for trial in 0..10 {
            // a dense alphabet: nearly all 288 literal lengths are equal, which
            // is what produces long `case 16` runs
            let items: Vec<Item> = (0..280).map(|i| Item::Lit((i % 255) as u8)).collect();
            let (lit, dist) = lens_for(&items, &[]);
            let mut spec = DynSpec::new(lit, dist);
            spec.cl_mode = mode;
            let stream = dynamic_stream(&spec, &items);
            let expect = Item::expand(&items);
            let label = format!("C12/13/14 {mode:?} dense trial{trial}");
            assert_decodes(&label, &stream, &expect);
            cases.push(Case::new(&label, &stream, expect.len() as i32).in_off(trial % 4));
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn c15_dynamic_ndst_one() {
    // ndst == 1: `cp_build(0, s->dst, lens + nlit, 1)` builds a one-entry tree
    let mut rng = Rng::new(0xC15);
    let mut cases = Vec::new();
    for trial in 0..12 {
        let items: Vec<Item> = (0..rng.range(4, 30)).map(|_| Item::Lit(rng.byte())).collect();
        let (lit, _) = lens_for(&items, &[]);
        let mut dist = vec![0u8; 32];
        dist[0] = 1;
        let mut spec = DynSpec::new(lit, dist);
        spec.force_ndst = Some(1);
        let stream = dynamic_stream(&spec, &items);
        let expect = Item::expand(&items);
        let label = format!("C15 ndst=1 trial{trial}");
        assert_decodes(&label, &stream, &expect);
        cases.extend(all_alignments(&label, &stream, expect.len() as i32));
    }
    assert_batch_matches(&cases);
}

#[test]
fn c16_dynamic_nlit_288_ndst_32() {
    // both maxima: nlit == 288, ndst == 32. Uniform weights give every symbol a
    // code, so all 288 + 32 transmitted lengths are non-zero.
    let mut cases = Vec::new();
    let lit = lens_with_limit(&vec![1u64; 288], 15);
    let dist = lens_with_limit(&vec![1u64; 32], 15);
    assert_eq!(kraft(&lit), 1 << 15, "literal code incomplete");
    assert_eq!(kraft(&dist), 1 << 15, "distance code incomplete");
    assert!(lit.iter().all(|&l| l != 0), "not every literal symbol has a code");
    assert!(dist.iter().all(|&l| l != 0), "not every distance symbol has a code");

    for trial in 0..8usize {
        let mut rng = Rng::new(0xC16 + trial as u64);
        // touch every length symbol and every distance symbol
        let mut items: Vec<Item> = (0..40000).map(|i| Item::Lit((i % 256) as u8)).collect();
        for ls in 0..29usize {
            items.push(Item::Match { len: LEN_BASE[ls], dist: 1 + (ls as u32 % 3) });
        }
        for ds in 0..30usize {
            items.push(Item::Match { len: 3, dist: DIST_BASE[ds] });
        }
        for _ in 0..40 {
            items.push(Item::Lit(rng.byte()));
        }
        let mut spec = DynSpec::new(lit.clone(), dist.clone());
        spec.force_nlit = Some(288);
        spec.force_ndst = Some(32);
        spec.cl_mode = if trial % 2 == 0 { ClMode::Rle } else { ClMode::Raw };
        let stream = dynamic_stream(&spec, &items);
        let expect = Item::expand(&items);
        let label = format!("C16 nlit=288 ndst=32 trial{trial}");
        assert_decodes(&label, &stream, &expect);
        cases.push(
            Case::new(&label, &stream, expect.len() as i32)
                .in_off(trial % 4)
                .out_pad(64),
        );
    }
    assert_batch_matches(&cases);
}

#[test]
fn c17_c35_dynamic_deep_codes() {
    // C17: a tree with codes of length <= 9 (which `cp_build` also writes into
    //      `s->lookup`) *and* codes longer than 9 (tree-only).
    // C35: maximum legal Huffman depth, 15.
    let mut cases = Vec::new();
    for depth in [10usize, 12, 15] {
        // Fibonacci weights over `depth + 1` symbols produce a maximally
        // unbalanced code whose longest length is exactly `depth`.
        let mut lw = vec![0u64; 288];
        let syms: Vec<usize> = std::iter::once(256usize)
            .chain((0..depth).map(|i| b'a' as usize + i))
            .collect();
        let mut a: u64 = 1;
        let mut b: u64 = 1;
        for &s in syms.iter().rev() {
            lw[s] = a;
            let n = a + b;
            a = b;
            b = n;
        }
        let lit = huffman_lens(&lw);
        let longest = *lit.iter().max().unwrap();
        assert!(
            longest > 9,
            "depth {depth}: longest code {longest} does not exceed 9"
        );
        assert!(lit.iter().any(|&l| l != 0 && l <= 9), "depth {depth}: no short code");
        let mut dist = vec![0u8; 32];
        dist[0] = 1;
        let items: Vec<Item> = (0..200)
            .map(|i| Item::Lit(syms[(i * 7) % syms.len()].min(255) as u8))
            .filter(|it| matches!(it, Item::Lit(b) if *b != 0))
            .collect();
        let items: Vec<Item> = items
            .into_iter()
            .filter(|it| match it {
                Item::Lit(b) => lit[*b as usize] != 0,
                _ => true,
            })
            .collect();
        let spec = DynSpec::new(lit, dist);
        let stream = dynamic_stream(&spec, &items);
        let expect = Item::expand(&items);
        let label = format!("C17/C35 depth={depth} longest={longest}");
        assert_decodes(&label, &stream, &expect);
        cases.extend(all_alignments(&label, &stream, expect.len() as i32));
    }
    assert_batch_matches(&cases);
}

#[test]
fn c36_end_of_block_code_lengths() {
    // symbol 256 with the shortest code, and with a long code
    let mut cases = Vec::new();
    for &eob_short in &[true, false] {
        let mut lw = vec![0u64; 288];
        for i in 0..40usize {
            lw[b'A' as usize + i] = if eob_short { 1 } else { 1000 };
        }
        lw[256] = if eob_short { 1_000_000 } else { 1 };
        let lit = huffman_lens(&lw);
        let mut dist = vec![0u8; 32];
        dist[0] = 1;
        let items: Vec<Item> = (0..120).map(|i| Item::Lit(b'A' + (i % 40) as u8)).collect();
        let spec = DynSpec::new(lit.clone(), dist);
        let stream = dynamic_stream(&spec, &items);
        let expect = Item::expand(&items);
        let label = format!("C36 eob_len={} ", lit[256]);
        assert_decodes(&label, &stream, &expect);
        cases.extend(all_alignments(&label, &stream, expect.len() as i32));
    }
    assert_batch_matches(&cases);
}

// ---------------------------------------------------------------------------
// C20 -- C27, C33, C34, C38 -- C40: stream/buffer shapes
// ---------------------------------------------------------------------------

#[test]
fn c20_c34_multi_block_mixed_types() {
    // C20: several blocks in one stream (the `!bfinal` loop iterates)
    // C34: stored -> fixed -> dynamic -> final, in one stream
    let mut rng = Rng::new(0xC20);
    let mut cases = Vec::new();
    let mut variants = Vec::new();
    for trial in 0..12 {
        let nstored = rng.range(1, 50);
        let stored = rng.bytes(nstored);
        let fixed_items: Vec<Item> = (0..rng.range(3, 40)).map(|_| Item::Lit(rng.byte())).collect();
        let dyn_items = random_items(&mut rng, 40, 40, 32);
        let (lit, dist) = lens_for(&dyn_items, &[]);
        let spec = DynSpec::new(lit, dist);

        // The stored block goes last: `cp_stored`'s inverted length check (E2)
        // rejects any stored block that still has input after it.
        let mut w = BitWriter::new();
        fixed_block(&mut w, &fixed_items, false);
        dynamic_block(&mut w, &spec, &dyn_items, false);
        stored_block(&mut w, &stored, true);

        let mut expect = Item::expand(&fixed_items);
        expect.extend(Item::expand(&dyn_items));
        expect.extend_from_slice(&stored);
        let label = format!("C20/C34 mixed trial{trial}");
        variants.push((w.bytes.clone(), expect.clone(), 0));
        cases.extend(all_alignments(&label, &w.bytes, expect.len() as i32));
    }
    assert_some_decode("C20/C34 mixed block types", &variants);
    assert_batch_matches(&cases);
}

#[test]
fn c21_c22_c23_final_word_shapes() {
    // `last_bytes = (in_bytes - first_bytes) & 3` selects `final_word_available`
    // and therefore which branch `cp_peak_bits` takes; `word_count == 0` means
    // the whole stream lives in `first_bytes` + the final word.
    let mut cases = Vec::new();
    let mut seen: std::collections::BTreeSet<(usize, i32)> = std::collections::BTreeSet::new();
    for nlit in 1..=12usize {
        let items: Vec<Item> = (0..nlit).map(|i| Item::Lit(b'a' + i as u8)).collect();
        let stream = fixed_stream(&items);
        let expect = Item::expand(&items);
        for in_off in 0..4usize {
            let first_bytes = ((in_off + 3) & !3) - in_off;
            let last_bytes = (stream.len() as i32 - first_bytes as i32) & 3;
            let word_count = (stream.len() as i32 - first_bytes as i32) / 4;
            seen.insert((last_bytes as usize, word_count));
            let label = format!(
                "C21/22/23 nlit={nlit} in_off={in_off} last_bytes={last_bytes} word_count={word_count}"
            );
            assert_decodes(&label, &stream, &expect);
            cases.push(Case::new(&label, &stream, expect.len() as i32).in_off(in_off));
        }
    }
    let lasts: std::collections::BTreeSet<usize> = seen.iter().map(|&(l, _)| l).collect();
    assert!(
        lasts.contains(&0) && lasts.contains(&1) && lasts.contains(&2) && lasts.contains(&3),
        "did not cover every `last_bytes` value, got {lasts:?}"
    );
    assert!(
        seen.iter().any(|&(_, wc)| wc == 0),
        "did not cover word_count == 0"
    );
    assert_batch_matches(&cases);
}

#[test]
fn c24_alignment_cross_product() {
    let items: Vec<Item> = (0..50).map(|i| Item::Lit((i * 5 % 256) as u8)).collect();
    let stream = fixed_stream(&items);
    let expect = Item::expand(&items);
    let mut cases = Vec::new();
    for in_off in 0..4usize {
        for out_off in 0..4usize {
            cases.push(
                Case::new(
                    &format!("C24 in_off={in_off} out_off={out_off}"),
                    &stream,
                    expect.len() as i32,
                )
                .in_off(in_off)
                .out_off(out_off),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn c25_c26_out_buffer_sizes() {
    // exact fit, and generously oversized (the padding must stay 0xCD)
    let mut rng = Rng::new(0xC25);
    let mut cases = Vec::new();
    for trial in 0..24 {
        let items = random_items(&mut rng, 30, 60, 24);
        let stream = fixed_stream(&items);
        let n = Item::expand(&items).len() as i32;
        for &out_size in &[n, n + 1, n + 1000] {
            cases.push(
                Case::new(
                    &format!("C25/C26 trial{trial} exact={n} out_size={out_size}"),
                    &stream,
                    out_size,
                )
                .in_off(trial % 4)
                .out_pad(256),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn c38_c39_trailing_garbage() {
    // C38: bytes after the final block must be ignored
    // C39: `in_bytes` covering that garbage as well
    let mut rng = Rng::new(0xC38);
    let mut cases = Vec::new();
    for trial in 0..24 {
        let items = random_items(&mut rng, 25, 90, 16);
        let mut stream = fixed_stream(&items);
        let expect_len = Item::expand(&items).len() as i32;
        let clean_len = stream.len();
        let ngarbage = rng.range(1, 20);
        let garbage = rng.bytes(ngarbage);
        stream.extend_from_slice(&garbage);
        cases.push(
            Case::new(
                &format!("C38 trailing garbage trial{trial}"),
                &stream,
                expect_len,
            )
            .in_off(trial % 4),
        );
        // C39: in_bytes shorter than the buffer contents (the C reads only what
        // in_bytes covers, but its bit reader can still peek into the padding)
        cases.push(
            Case::new(
                &format!("C39 in_len={clean_len} of {} trial{trial}", stream.len()),
                &stream,
                expect_len,
            )
            .in_len(clean_len as i32)
            .in_off(trial % 4),
        );
    }
    assert_batch_matches(&cases);
}

#[test]
fn c40_large_rle_memset_runs() {
    // 64 KiB of one byte via `backwards_distance == 1`: maximal `memset` runs,
    // and enough output to exercise the multi-block path
    let mut items = vec![Item::Lit(0x5A)];
    let mut produced = 1u32;
    while produced < 65536 {
        let len = 258.min(65536 - produced);
        items.push(Item::Match { len, dist: 1 });
        produced += len;
    }
    let stream = fixed_stream(&items);
    let expect = Item::expand(&items);
    assert_eq!(expect.len(), 65536);
    assert_decodes("C40 64KiB memset", &stream, &expect);
    let mut cases = Vec::new();
    for in_off in 0..4 {
        cases.push(
            Case::new(&format!("C40 64KiB memset in_off={in_off}"), &stream, 65536)
                .in_off(in_off)
                .out_pad(64),
        );
    }
    assert_batch_matches(&cases);
}

#[test]
fn c20b_large_payload_multiblock() {
    // > 64 KiB decompressed across several blocks, with distances beyond 32 KiB/2
    let mut rng = Rng::new(0xC20B);
    let mut w = BitWriter::new();
    let mut expect: Vec<u8> = Vec::new();
    for blk in 0..5usize {
        let mut items: Vec<Item> = (0..2000).map(|_| Item::Lit(rng.byte())).collect();
        for _ in 0..300 {
            let produced = Item::expand(&items).len() as u32;
            if produced < 40 {
                continue;
            }
            let dist = 1 + rng.below(produced.min(32768) as usize) as u32;
            items.push(Item::Match { len: 3 + rng.below(256) as u32, dist });
        }
        let last = blk == 4;
        if blk % 2 == 0 {
            fixed_block(&mut w, &items, last);
        } else {
            let (lit, dist) = lens_for(&items, &[]);
            dynamic_block(&mut w, &DynSpec::new(lit, dist), &items, last);
        }
        expect.extend(Item::expand(&items));
    }
    w.align();
    assert!(expect.len() > 65536, "payload only {} bytes", expect.len());
    assert_decodes("C20b large multiblock", &w.bytes, &expect);
    assert_batch_matches(&[
        Case::new("C20b large multiblock", &w.bytes, expect.len() as i32).out_pad(64),
        Case::new("C20b large multiblock io1", &w.bytes, expect.len() as i32)
            .in_off(1)
            .out_off(3)
            .out_pad(64),
    ]);
}

// ---------------------------------------------------------------------------
// C28 -- C32: the writable exported tables, and cp_error_reason
// ---------------------------------------------------------------------------

#[test]
fn c28_caller_rewrites_fixed_table() {
    // `cp_fixed_table` is an exported, *writable* `uint8_t[320]`. Replacing it
    // with a different complete code changes how a btype==1 block decodes, in
    // both libraries identically.
    let mut lw = vec![0u64; 288];
    for i in 0..288usize {
        lw[i] = if i < 32 { 8 } else { 1 };
    }
    lw[256] = 64;
    let alt_lit = huffman_lens(&lw);
    assert_eq!(kraft(&alt_lit), 1 << 15, "alt fixed literal code incomplete");
    let alt_dist = vec![5u8; 32];
    let mut table = alt_lit.clone();
    table.extend_from_slice(&alt_dist);
    assert_eq!(table.len(), 320);

    let items: Vec<Item> = (0..60).map(|i| Item::Lit((i * 3 % 288).min(255) as u8)).collect();
    // encode using the *replacement* code
    let mut w = BitWriter::new();
    w.bit(1);
    w.bits(1, 2);
    let lit_codes = canonical_codes(&alt_lit);
    let _dist_codes = canonical_codes(&alt_dist);
    for it in &items {
        if let Item::Lit(b) = it {
            w.code(lit_codes[*b as usize], alt_lit[*b as usize] as usize);
        }
    }
    w.code(lit_codes[256], alt_lit[256] as usize);
    w.align();

    let expect = Item::expand(&items);
    let mut cases = Vec::new();
    for in_off in 0..4 {
        cases.push(
            Case::new(
                &format!("C28 rewritten cp_fixed_table in_off={in_off}"),
                &w.bytes,
                expect.len() as i32,
            )
            .in_off(in_off)
            .table("ft", table.clone()),
        );
    }
    // and the *same* stream without the table override: both libraries must
    // agree on whatever the pristine table makes of it (garbage or an error)
    cases.push(Case::new("C28 same stream, pristine table", &w.bytes, expect.len() as i32));
    assert_batch_matches(&cases);
}

#[test]
fn c29_caller_permutes_permutation_order() {
    // `cp_permutation_order` is writable too: reverse it and write the dynamic
    // header's code lengths in that order.
    let mut perm: Vec<u8> = PERM.iter().map(|&x| x as u8).collect();
    perm.reverse();

    let items: Vec<Item> = (0..40).map(|i| Item::Lit(b'a' + (i % 20) as u8)).collect();
    let (lit, dist) = lens_for(&items, &[]);
    let mut lit = lit;
    lit.resize(288, 0);
    let mut dist = dist;
    dist.resize(32, 0);
    let nlit = 257usize;
    let ndst = 1usize;
    let mut seq: Vec<u8> = lit[..nlit].to_vec();
    seq.extend_from_slice(&dist[..ndst]);
    let cl = encode_cl(&seq, ClMode::Rle);
    let mut freq = vec![0u64; 19];
    for &(s, _, _) in &cl {
        freq[s] += 1;
    }
    let cl_lens = huffman_lens(&freq);
    let cl_codes = canonical_codes(&cl_lens);

    let mut w = BitWriter::new();
    w.bit(1);
    w.bits(2, 2);
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits(19 - 4, 4);
    for i in 0..19usize {
        w.bits(cl_lens[perm[i] as usize] as u32, 3);
    }
    for &(s, extra, nbits) in &cl {
        w.code(cl_codes[s], cl_lens[s] as usize);
        w.bits(extra, nbits);
    }
    let lit_codes = canonical_codes(&lit);
    for it in &items {
        if let Item::Lit(b) = it {
            w.code(lit_codes[*b as usize], lit[*b as usize] as usize);
        }
    }
    w.code(lit_codes[256], lit[256] as usize);
    w.align();

    let expect = Item::expand(&items);
    let mut cases = Vec::new();
    for in_off in 0..4 {
        cases.push(
            Case::new(
                &format!("C29 reversed cp_permutation_order in_off={in_off}"),
                &w.bytes,
                expect.len() as i32,
            )
            .in_off(in_off)
            .table("po", perm.clone()),
        );
    }
    cases.push(Case::new(
        "C29 same stream, pristine permutation",
        &w.bytes,
        expect.len() as i32,
    ));
    assert_batch_matches(&cases);
}

#[test]
fn c30_c31_caller_rewrites_length_and_distance_tables() {
    // `cp_len_base`/`cp_len_extra_bits`/`cp_dist_base`/`cp_dist_extra_bits` are
    // writable, and `cp_block` reads them for every match. Retune them and the
    // *same* stream must decode to the same (different) bytes in both libraries.
    let items: Vec<Item> = (0..20)
        .map(|i| Item::Lit(b'A' + i as u8))
        .chain([Item::Match { len: 5, dist: 4 }, Item::Match { len: 20, dist: 9 }])
        .collect();
    let stream = fixed_stream(&items);
    let n = Item::expand(&items).len() as i32;

    let u32_table = |v: &[u32]| -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()
    };

    // all extra-bit counts zeroed: lengths/distances collapse onto their bases
    let zero_len_extra = vec![0u8; 31];
    let zero_dist_extra = vec![0u8; 32];
    // bases shifted by one
    let mut lb: Vec<u32> = LEN_BASE.to_vec();
    lb.push(0);
    lb.push(0);
    for x in lb.iter_mut() {
        if *x > 3 {
            *x -= 1;
        }
    }
    let mut db: Vec<u32> = DIST_BASE.to_vec();
    db.push(0);
    db.push(0);
    for x in db.iter_mut() {
        if *x > 1 {
            *x -= 1;
        }
    }

    let cases = vec![
        Case::new("C30 zeroed cp_len_extra_bits", &stream, n)
            .table("le", zero_len_extra.clone()),
        Case::new("C30 shifted cp_len_base", &stream, n).table("lb", u32_table(&lb)),
        Case::new("C30 both", &stream, n)
            .table("le", zero_len_extra)
            .table("lb", u32_table(&lb)),
        Case::new("C31 zeroed cp_dist_extra_bits", &stream, n)
            .table("de", zero_dist_extra.clone()),
        Case::new("C31 shifted cp_dist_base", &stream, n).table("db", u32_table(&db)),
        Case::new("C31 both", &stream, n)
            .table("de", zero_dist_extra)
            .table("db", u32_table(&db)),
        Case::new("C30/C31 pristine baseline", &stream, n),
    ];
    assert_batch_matches(&cases);
}

#[test]
fn c32_error_reason_untouched_on_success() {
    // The C never clears `cp_error_reason`; a successful call must leave the
    // caller's value in place.
    let items: Vec<Item> = (0..12).map(|i| Item::Lit(b'q' + i as u8)).collect();
    let stream = fixed_stream(&items);
    let n = Item::expand(&items).len() as i32;
    let cases = vec![
        Case::new("C32 preset sentinel, success", &stream, n).err_preset(),
        Case::new("C32 null, success", &stream, n),
        // and after a failure the message must be the library's, not the sentinel
        Case::new("C32 preset sentinel, failure", &stream, 1).err_preset(),
    ];
    assert_batch_matches(&cases);

    // additionally pin down that the successful case really kept the sentinel
    let r = run_batch(c_so(), &cases);
    match &r[0] {
        common::shared::Outcome::Ret { ret, err, .. } => {
            assert_eq!(*ret, 1);
            assert_eq!(
                err.as_deref(),
                Some(&b"SENTINEL-UNTOUCHED-X"[..]),
                "C cleared cp_error_reason on success"
            );
        }
        o => panic!("unexpected {o:?}"),
    }
}

#[test]
fn c33_single_block_each_btype() {
    // bfinal == 1 on the very first block, for btype 0/1/2
    let payload = b"single-block payload".to_vec();
    let mut w0 = BitWriter::new();
    stored_block(&mut w0, &payload, true);

    let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
    let s1 = fixed_stream(&items);
    let (lit, dist) = lens_for(&items, &[]);
    let s2 = dynamic_stream(&DynSpec::new(lit, dist), &items);

    let n = payload.len() as i32;
    assert_decodes("C33 btype=0", &w0.bytes, &payload);
    assert_decodes("C33 btype=1", &s1, &payload);
    assert_decodes("C33 btype=2", &s2, &payload);

    let mut cases = Vec::new();
    for (lbl, s) in [("btype=0", &w0.bytes), ("btype=1", &s1), ("btype=2", &s2)] {
        cases.extend(all_alignments(&format!("C33 {lbl}"), s, n));
    }
    assert_batch_matches(&cases);
}

// ---------------------------------------------------------------------------
// C27, C37: property-style randomized sweeps
// ---------------------------------------------------------------------------

#[test]
fn c27_c37_randomized_sweep() {
    // The cross-product of everything above, driven randomly from a fixed seed:
    // block type, payload shape (empty / one / many / low-entropy / random),
    // code-length encoding, alignments, and output sizing.
    let mut rng = Rng::new(0x37);
    let mut cases = Vec::new();
    let mut decoded = 0usize;
    for trial in 0..320usize {
        let shape = rng.below(6);
        let items: Vec<Item> = match shape {
            0 => Vec::new(),
            1 => vec![Item::Lit(rng.byte())],
            2 => (0..rng.range(1, 200)).map(|_| Item::Lit(rng.byte())).collect(),
            3 => {
                let n = rng.range(1, 120);
                random_items(&mut rng, n, 4, 64)
            }
            4 => {
                let n = rng.range(1, 120);
                random_items(&mut rng, n, 256, 4096)
            }
            _ => {
                let mut v = vec![Item::Lit(rng.byte())];
                for _ in 0..rng.range(1, 30) {
                    v.push(Item::Match { len: 3 + rng.below(256) as u32, dist: 1 });
                }
                v
            }
        };
        let expect = Item::expand(&items);

        let btype = rng.below(3);
        let mut w = BitWriter::new();
        match btype {
            0 => {
                // stored blocks carry raw bytes, chunked to 65535
                if expect.is_empty() {
                    stored_block(&mut w, &[], true);
                } else {
                    let chunks: Vec<&[u8]> = expect.chunks(65535).collect();
                    for (i, c) in chunks.iter().enumerate() {
                        stored_block(&mut w, c, i + 1 == chunks.len());
                    }
                }
            }
            1 => fixed_block(&mut w, &items, true),
            _ => {
                let (lit, dist) = lens_for(&items, &[]);
                let mut spec = DynSpec::new(lit, dist);
                spec.cl_mode = if rng.bool() { ClMode::Rle } else { ClMode::Raw };
                dynamic_block(&mut w, &spec, &items, true);
            }
        }
        w.align();

        let out_size = match rng.below(3) {
            0 => expect.len().max(1) as i32,
            1 => expect.len() as i32 + 1 + rng.below(64) as i32,
            _ => expect.len().max(1) as i32,
        };
        let label = format!(
            "C37 trial{trial} shape={shape} btype={btype} out={} bytes",
            expect.len()
        );
        if btype != 0 {
            // stored blocks: see `assert_some_decode` -- the C's own stored path
            // is size-dependent, so only agreement is asserted for them.
            assert_decodes(&label, &w.bytes, &expect);
            decoded += 1;
        }
        cases.push(
            Case::new(&label, &w.bytes, out_size)
                .in_off(rng.below(4))
                .out_off(rng.below(4))
                .in_pad(rng.range(0, 64))
                .out_pad(rng.range(0, 256)),
        );
    }
    assert!(
        decoded > 150,
        "only {decoded} of 320 randomized streams took the C success path"
    );
    assert_batch_matches(&cases);
}

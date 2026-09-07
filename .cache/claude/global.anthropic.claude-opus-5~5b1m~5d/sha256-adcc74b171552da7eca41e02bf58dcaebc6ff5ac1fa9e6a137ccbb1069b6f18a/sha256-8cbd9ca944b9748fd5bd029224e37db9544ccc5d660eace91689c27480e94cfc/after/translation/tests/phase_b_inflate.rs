//! Phase B — valid-path differential tests for `cp_inflate`.
//!
//! One test per row of `CONFIGS.md` §`cp_inflate` (rows 16..34).  Every case is
//! run through both `.so`s' exported `cp_inflate` in a forked child, and the
//! return value, the *whole* output buffer and `cp_error_reason` are compared.
//! Happy-path rows additionally assert that the decode really succeeded and
//! produced the expected plaintext, so "both sides fail identically" can never
//! masquerade as a pass.

mod common;

use common::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const SEED: u64 = 0x5EED_1234;

fn rng(row: u64) -> StdRng {
    StdRng::seed_from_u64(SEED ^ (row << 40) ^ 0xA5A5)
}

fn run(cases: Vec<Case>, expected: Vec<Vec<u8>>) -> BatchReport {
    assert_eq!(cases.len(), expected.len());
    // The C build the translation targets (NDEBUG) ...
    let rep = diff_inflate_batch(pair_release(), &cases);
    rep.assert_all_decoded(&cases, &expected);
    // ... and the assert-enabled build, which must agree on valid streams.
    let rep2 = diff_inflate_batch(pair_asserts(), &cases);
    rep2.assert_all_decoded(&cases, &expected);
    rep
}

/// Differential comparison only, with no claim about *what* the right answer
/// is.  Used for the inputs on which the C library is quirky but internally
/// consistent (see the notes on `cp_stored` in `CONFIGS.md` rows 16/27/31):
/// the requirement is still that C and Rust agree bit for bit, and that the C
/// library survives.
fn run_diff_only(cases: Vec<Case>) -> BatchReport {
    let rep = diff_inflate_batch(pair_release(), &cases);
    assert!(rep.c_died.is_empty(), "C died: {:?}", rep.c_died);
    assert_eq!(rep.compared, cases.len());
    let rep2 = diff_inflate_batch(pair_asserts(), &cases);
    assert!(rep2.c_died.is_empty(), "C(assert) died: {:?}", rep2.c_died);
    rep
}

fn fixed_stream(items: &[Item]) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, items);
    w.finish()
}

fn lits(data: &[u8]) -> Vec<Item> {
    data.iter().map(|&b| Item::Lit(b)).collect()
}

// ---------------------------------------------------------------------- row 16
// A stored block is `1 byte of header + 4 bytes of LEN/NLEN + LEN bytes`, so
// the total input is `LEN + 5`.  `cp_ptr()` derives the copy source from
// `words + word_index - count/8`, which is only correct while the bit buffer
// has been refilled from `s->words`; once `cp_peak_bits` has fallen back to
// `s->final_word` (which does *not* advance `word_index`) the pointer is off by
// `last_bytes`.  So the C library only copies the right bytes when
// `(LEN + 5) % 4 == 0`, i.e. `LEN % 4 == 3`.  Both variants are tested:
// the aligned ones against the true plaintext, the rest differentially.
#[test]
fn row16_stored_block() {
    let mut r = rng(16);
    let (mut aligned, mut aexp) = (Vec::new(), Vec::new());
    let mut other = Vec::new();
    for _ in 0..160 {
        let n: usize = r.gen_range(1..=200);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        let mut w = BitWriter::new();
        write_stored_block(&mut w, true, &payload);
        let s = w.finish();
        assert_eq!(s.len(), n + 5);
        let c = Case::new(format!("stored n={n}"), s, n as i32);
        if n % 4 == 3 {
            aligned.push(c);
            aexp.push(payload);
        } else {
            other.push(c);
        }
    }
    assert!(aligned.len() >= 20 && other.len() >= 60);
    run(aligned, aexp);
    run_diff_only(other);
}

// ---------------------------------------------------------------------- row 17
#[test]
fn row17_stored_block_len_zero() {
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, &[]);
    let cases = vec![Case::new("stored LEN=0", w.finish(), 0).out_cap(32)];
    run(cases, vec![vec![]]);
}

// ---------------------------------------------------------------------- row 18
#[test]
fn row18_fixed_literals_only() {
    let mut r = rng(18);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    let all: Vec<u8> = (0..=255u8).collect();
    cases.push(Case::new(
        "fixed all-256-literals",
        fixed_stream(&lits(&all)),
        256,
    ));
    exp.push(all);
    for _ in 0..150 {
        let n = r.gen_range(1..=300);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        cases.push(Case::new(
            format!("fixed lits n={n}"),
            fixed_stream(&lits(&payload)),
            payload.len() as i32,
        ));
        exp.push(payload);
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 19
// distance == 1 -> the memset() fast path, exhaustive over every length.
#[test]
fn row19_distance_one_memset_path() {
    let mut r = rng(19);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for length in 3u32..=258 {
        let items = vec![Item::Lit(r.gen::<u8>()), Item::Match(length, 1)];
        let plain = expand(&items, &[]);
        cases.push(Case::new(
            format!("dist=1 len={length}"),
            fixed_stream(&items),
            plain.len() as i32,
        ));
        exp.push(plain);
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 20
// distance 2..64 -> the byte-at-a-time (possibly overlapping) copy.
#[test]
fn row20_distance_byte_copy_path() {
    let mut r = rng(20);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for dist in 2u32..=64 {
        for _ in 0..4 {
            let length = r.gen_range(3u32..=258);
            let seed: Vec<u8> = (0..dist).map(|_| r.gen::<u8>()).collect();
            let mut items = lits(&seed);
            items.push(Item::Match(length, dist));
            let plain = expand(&items, &[]);
            cases.push(Case::new(
                format!("dist={dist} len={length}"),
                fixed_stream(&items),
                plain.len() as i32,
            ));
            exp.push(plain);
        }
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 21
// every cp_len_extra_bits class, exhaustive over the 29 length symbols.
#[test]
fn row21_all_length_symbols() {
    let mut r = rng(21);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    let mut classes = std::collections::BTreeSet::new();
    for ls in 0usize..29 {
        classes.insert(LEN_EXTRA[ls]);
        let nextra = LEN_EXTRA[ls];
        let variants: Vec<u32> = if nextra == 0 {
            vec![LEN_BASE[ls]]
        } else {
            vec![
                LEN_BASE[ls],
                LEN_BASE[ls] + (1 << nextra) - 1,
                LEN_BASE[ls] + r.gen_range(0..(1u32 << nextra)),
            ]
        };
        for length in variants {
            // 258 is both `LEN_BASE[27] + 31` and `LEN_BASE[28]`; the canonical
            // encoder picks symbol 28, so only assert for the unambiguous ones.
            if length != 258 {
                assert_eq!(len_symbol(length), ls);
            }
            let dist = length.max(3);
            let seed: Vec<u8> = (0..dist).map(|_| r.gen::<u8>()).collect();
            let mut items = lits(&seed);
            items.push(Item::Match(length, dist));
            let plain = expand(&items, &[]);
            cases.push(Case::new(
                format!("lensym={ls} len={length} dist={dist}"),
                fixed_stream(&items),
                plain.len() as i32,
            ));
            exp.push(plain);
        }
    }
    assert_eq!(
        classes,
        (0u32..=5).collect(),
        "should cover every extra-bit class 0..5"
    );
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 22
// every cp_dist_extra_bits class, exhaustive over the 30 distance symbols.
#[test]
fn row22_all_distance_symbols() {
    let mut r = rng(22);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    let mut classes = std::collections::BTreeSet::new();
    for ds in 0usize..30 {
        classes.insert(DIST_EXTRA[ds]);
        let nextra = DIST_EXTRA[ds];
        let variants: Vec<u32> = if nextra == 0 {
            vec![DIST_BASE[ds]]
        } else {
            vec![
                DIST_BASE[ds],
                DIST_BASE[ds] + (1 << nextra) - 1,
                DIST_BASE[ds] + r.gen_range(0..(1u32 << nextra)),
            ]
        };
        for dist in variants {
            assert_eq!(dist_symbol(dist), ds);
            let seed: Vec<u8> = (0..dist).map(|_| r.gen::<u8>()).collect();
            let length = r.gen_range(3u32..=258);
            let mut items = lits(&seed);
            items.push(Item::Match(length, dist));
            let plain = expand(&items, &[]);
            let n = plain.len() as i32;
            cases.push(Case::new(
                format!("distsym={ds} dist={dist} len={length}"),
                fixed_stream(&items),
                n,
            ));
            exp.push(plain);
        }
    }
    assert_eq!(
        classes,
        (0u32..=13).collect(),
        "should cover every extra-bit class 0..13"
    );
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 23
// literals >= 144 only -> the 9-bit half of the fixed table; plus the 8-bit
// tail symbols 280..285 used as length codes.
#[test]
fn row23_fixed_nine_bit_literals() {
    let mut r = rng(23);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for _ in 0..60 {
        let n = r.gen_range(1..=200);
        let payload: Vec<u8> = (0..n).map(|_| r.gen_range(144..=255u8)).collect();
        cases.push(Case::new(
            format!("fixed 9-bit lits n={n}"),
            fixed_stream(&lits(&payload)),
            payload.len() as i32,
        ));
        exp.push(payload);
    }
    for ls in 23usize..=28 {
        let length = LEN_BASE[ls];
        let seed: Vec<u8> = (0..length).map(|_| r.gen::<u8>()).collect();
        let mut items = lits(&seed);
        items.push(Item::Match(length, seed.len() as u32));
        let plain = expand(&items, &[]);
        cases.push(Case::new(
            format!("fixed litsym={} (len {length})", 257 + ls),
            fixed_stream(&items),
            plain.len() as i32,
        ));
        exp.push(plain);
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 24
// real dynamic-Huffman streams from miniz.
#[test]
fn row24_dynamic_real_streams() {
    let mut r = rng(24);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for _ in 0..80 {
        let n = r.gen_range(1..=4096);
        let payload: Vec<u8> = (0..n)
            .map(|i| {
                if r.gen_bool(0.5) {
                    r.gen::<u8>()
                } else {
                    b"abcdefghij"[(i % 10) as usize]
                }
            })
            .collect();
        let s = deflate_raw(&payload, 9);
        cases.push(Case::new(
            format!("miniz-9 n={n} ({} bytes)", s.len()),
            s,
            payload.len() as i32,
        ));
        exp.push(payload);
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 25
// hand-built dynamic header at the minimum extremes: HLIT=257, HDIST=1 and the
// smallest HCLEN that can still describe a usable code-length alphabet.
//
// With HCLEN == 5 only CLEN_ORDER[0..5] == {16,17,18,0,8} may carry a length,
// so the literal table has to be built exclusively out of the values 8 and 0:
// 256 symbols at depth 8 is exactly a complete tree.
#[test]
fn row25_dynamic_header_minimum() {
    let mut r = rng(25);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());

    let hlit = 257usize;
    let hdist = 1usize;
    let mut lit_lens = vec![8u8; hlit];
    lit_lens[255] = 0; // drop literal 255 so exactly 256 leaves remain
    assert!(is_complete(&lit_lens));
    let dst_lens = vec![0u8; hdist]; // no distance codes (we emit no matches)

    let all: Vec<u8> = lit_lens.iter().chain(dst_lens.iter()).copied().collect();
    let clen_items = rle_code_lengths(&all);
    for &(sym, _) in &clen_items {
        assert!(
            CLEN_ORDER.iter().position(|&o| o == sym).unwrap() < 5,
            "symbol {sym} needs HCLEN > 5"
        );
    }
    // a complete 2-bit tree over the four symbols {16,17,18,0} plus 8 needs
    // exactly 4 leaves; use {0, 8, 16, 17}.
    let mut clen_lens = [0u8; 19];
    for s in [0usize, 8, 16, 17] {
        clen_lens[s] = 2;
    }
    assert!(is_complete(&clen_lens));
    let enc = HuffEnc::new(lit_lens.clone(), dst_lens.clone());

    for _ in 0..25 {
        let n = r.gen_range(1..=64);
        let payload: Vec<u8> = (0..n).map(|_| r.gen_range(0..=254u8)).collect();
        let mut w = BitWriter::new();
        write_dynamic_block(
            &mut w,
            true,
            hlit,
            hdist,
            5,
            &clen_lens,
            &clen_items,
            &enc,
            &lits(&payload),
        );
        cases.push(Case::new(
            format!("dyn HLIT=257 HDIST=1 HCLEN=5 n={n}"),
            w.finish(),
            payload.len() as i32,
        ));
        exp.push(payload);
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 26
// hand-built dynamic header at the maximum extremes: HLIT=288, HDIST=32,
// HCLEN=19, code-length symbols 16/17/18 all used, and code words of length
// 10..15 (which `cp_build` deliberately leaves out of `s->lookup`).
#[test]
fn row26_dynamic_header_maximum_deep_tree() {
    let mut r = rng(26);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());

    let lit_lens = lit_lens_deep(); // 288 entries, depths 2, 3..15, 9
    let mut dst_lens = dst_lens_deep(); // 16 entries, depths 1..15
    dst_lens.extend(std::iter::repeat(0u8).take(16)); // declare 32, use 16
    assert_eq!(dst_lens.len(), 32);
    assert!(is_complete(&lit_lens) && is_complete(&dst_lens));
    assert!(lit_lens.iter().any(|&l| l >= 10), "need codes deeper than 9");

    let all: Vec<u8> = lit_lens.iter().chain(dst_lens.iter()).copied().collect();
    let clen_items = rle_code_lengths(&all);
    let used: std::collections::BTreeSet<usize> = clen_items.iter().map(|&(s, _)| s).collect();
    for want in [16usize, 17, 18] {
        assert!(
            used.contains(&want),
            "row 26 must exercise code-length symbol {want}; used={used:?}"
        );
    }
    // complete alphabet over all 19 code-length symbols: 13 at depth 4, 6 at 5
    let flat = complete_lengths(19);
    let mut clen_lens = [0u8; 19];
    clen_lens.copy_from_slice(&flat);
    assert!(is_complete(&clen_lens));

    let enc = HuffEnc::new(lit_lens.clone(), dst_lens.clone());
    for _ in 0..25 {
        let n = r.gen_range(1..=80);
        let mut items = lits(&(0..n).map(|_| r.gen::<u8>()).collect::<Vec<u8>>());
        // add matches so the deep length codes (257..269 => lengths 3..19) and
        // the deep distance codes are actually decoded
        let base = items.len() as u32;
        if base >= 20 {
            items.push(Item::Match(r.gen_range(3u32..=19), r.gen_range(1u32..=20)));
            items.push(Item::Match(r.gen_range(3u32..=19), 1));
        }
        let plain = expand(&items, &[]);
        let mut w = BitWriter::new();
        write_dynamic_block(
            &mut w,
            true,
            lit_lens.len(),
            dst_lens.len(),
            19,
            &clen_lens,
            &clen_items,
            &enc,
            &items,
        );
        cases.push(Case::new(
            format!("dyn HLIT=288 HDIST=32 HCLEN=19 deep n={n}"),
            w.finish(),
            plain.len() as i32,
        ));
        exp.push(plain);
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 27
// BFINAL chains of 2..5 blocks mixing BTYPE 1 and 2, optionally ending in a
// stored block.  A stored block can only be *last*: `cp_stored` rejects any
// stream where more than LEN bytes of input remain (ERRORS.md row 4).
#[test]
fn row27_multi_block_chains() {
    let mut r = rng(27);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    let mut stored_chains: Vec<Case> = Vec::new();
    let dyn_lit = lit_lens_all_literals(257);
    let dyn_dst = dst_lens_complete(2);
    for _ in 0..60 {
        let nblocks = r.gen_range(2..=5);
        let mut w = BitWriter::new();
        let mut plain: Vec<u8> = Vec::new();
        let mut kinds = Vec::new();
        for b in 0..nblocks {
            let last = b == nblocks - 1;
            let n = r.gen_range(1..=64);
            let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
            let kind = if last { r.gen_range(0..3) } else { 1 + r.gen_range(0..2) };
            kinds.push(kind);
            match kind {
                0 => write_stored_block(&mut w, last, &payload),
                1 => write_fixed_block(&mut w, last, &lits(&payload)),
                _ => dynamic_block_auto(&mut w, last, &dyn_lit, &dyn_dst, &lits(&payload)),
            }
            plain.extend_from_slice(&payload);
        }
        let s = w.finish();
        let c = Case::new(
            format!("chain {kinds:?} ({} bytes)", s.len()),
            s,
            plain.len() as i32,
        );
        // Chains ending in a stored block inherit `cp_stored`'s misaligned
        // source pointer (see row 16), so those are compared differentially
        // only; the pure fixed/dynamic chains must decode exactly.
        if kinds.contains(&0) {
            stored_chains.push(c);
        } else {
            cases.push(c);
            exp.push(plain);
        }
    }
    assert!(!cases.is_empty() && !stored_chains.is_empty());
    run(cases, exp);
    run_diff_only(stored_chains);
}

// ---------------------------------------------------------------------- row 28
// all 16 (first_bytes, last_bytes) alignment/tail combinations, exhaustive.
#[test]
fn row28_input_alignment_and_tail() {
    let mut r = rng(28);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    let mut seen = std::collections::BTreeSet::new();
    for misalign in 0usize..4 {
        for extra in 0usize..8 {
            let payload: Vec<u8> = (0..(4 + extra)).map(|_| r.gen::<u8>()).collect();
            let s = fixed_stream(&lits(&payload));
            // the harness places `in` at (16-aligned + misalign), so the C code
            // computes first_bytes = (4 - misalign) % 4
            let first_bytes = (4 - misalign) % 4;
            let last_bytes = (s.len() - first_bytes) % 4;
            seen.insert((first_bytes, last_bytes));
            cases.push(
                Case::new(
                    format!(
                        "align first_bytes={first_bytes} last_bytes={last_bytes} inlen={}",
                        s.len()
                    ),
                    s,
                    payload.len() as i32,
                )
                .misaligned(misalign),
            );
            exp.push(payload);
        }
    }
    assert_eq!(
        seen.len(),
        16,
        "expected all 16 (first_bytes, last_bytes) combos, got {seen:?}"
    );
    let rep = run(cases.clone(), exp);
    assert_eq!(rep.compared, cases.len());
}

// ---------------------------------------------------------------------- row 29
// word_count == 0: everything comes from `bits` + `final_word`.
#[test]
fn row29_word_count_zero() {
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for misalign in 0usize..4 {
        for lit in [0u8, 1, 0x41, 0xFF] {
            let s = fixed_stream(&[Item::Lit(lit)]);
            assert!(s.len() <= 4, "stream should be tiny, got {}", s.len());
            cases.push(
                Case::new(
                    format!("tiny misalign={misalign} lit={lit:#04x} len={}", s.len()),
                    s,
                    1,
                )
                .misaligned(misalign),
            );
            exp.push(vec![lit]);
        }
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 30
// out_bytes slack.
#[test]
fn row30_out_buffer_slack() {
    let mut r = rng(30);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for _ in 0..30 {
        let n = r.gen_range(1..=200usize);
        let payload: Vec<u8> = (0..n).map(|_| r.gen::<u8>()).collect();
        for s in [deflate_raw(&payload, 9), fixed_stream(&lits(&payload))] {
            for slack in [0usize, 1, 7, n] {
                let ob = (n + slack) as i32;
                cases.push(
                    Case::new(format!("slack={slack} n={n}"), s.clone(), ob)
                        .out_cap(ob as usize + 32),
                );
                exp.push(payload.clone());
            }
        }
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 31
// large payloads across miniz compression levels (level 0 => stored blocks).
#[test]
fn row31_large_payloads_all_levels() {
    let mut r = rng(31);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for &n in &[8usize * 1024, 20 * 1024, 64 * 1024] {
        let payload: Vec<u8> = (0..n)
            .map(|i| {
                if r.gen_bool(0.3) {
                    r.gen::<u8>()
                } else {
                    b"the quick brown fox jumps over the lazy dog "[i % 44]
                }
            })
            .collect();
        for level in [1u32, 6, 9] {
            let s = deflate_raw(&payload, level);
            cases.push(Case::new(
                format!("miniz-{level} n={n} ({} bytes)", s.len()),
                s,
                n as i32,
            ));
            exp.push(payload.clone());
        }
        // Level 0 emits a *chain* of stored blocks.  `cp_stored` rejects any
        // stored block that is not the last thing in the stream
        // (`bits_left/8 <= LEN`, ERRORS.md row 4), so the C library refuses
        // these — deterministically, and the Rust side must refuse identically.
        let s0 = deflate_raw(&payload, 0);
        let c0 = vec![Case::new(format!("miniz-0 n={n}"), s0, n as i32)];
        let rep0 = run_diff_only(c0.clone());
        if n > 0xFFFF {
            // more than one stored block => rejected
            assert_eq!(
                rep0.c[0].ret, 0,
                "[{}] level-0 chain should be rejected",
                c0[0].label
            );
            assert_eq!(
                rep0.c[0].err_str(),
                "Stored block extends beyond end of input stream."
            );
        } else {
            // exactly one stored block => accepted (payload correctness is
            // subject to the `cp_ptr` alignment quirk of row 16)
            assert_eq!(rep0.c[0].ret, 1, "[{}] single stored block", c0[0].label);
        }
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 32
// one repeated byte: maximal distance==1 memset runs / length==258 chains.
#[test]
fn row32_repeated_byte_runs() {
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for n in [1usize, 2, 3, 259, 4096, 64 * 1024] {
        let payload = vec![0x5Au8; n];
        for level in [1u32, 9] {
            cases.push(Case::new(
                format!("run n={n} level={level}"),
                deflate_raw(&payload, level),
                n as i32,
            ));
            exp.push(payload.clone());
        }
        if n >= 259 {
            let mut items = vec![Item::Lit(0x5A)];
            let mut have = 1usize;
            while have < n {
                let take = (n - have).min(258);
                if take < 3 {
                    for _ in 0..take {
                        items.push(Item::Lit(0x5A));
                    }
                    have = n;
                } else {
                    items.push(Item::Match(take as u32, 1));
                    have += take;
                }
            }
            let plain = expand(&items, &[]);
            assert_eq!(plain.len(), n);
            cases.push(Case::new(
                format!("hand dist1 chain n={n}"),
                fixed_stream(&items),
                n as i32,
            ));
            exp.push(plain);
        }
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 33
// 32 KiB back-references (distance symbols 28 and 29).
#[test]
fn row33_far_back_references() {
    let mut r = rng(33);
    let (mut cases, mut exp) = (Vec::new(), Vec::new());
    for &dist in &[16385u32, 20000, 24577, 30000, 32768] {
        let seed: Vec<u8> = (0..dist).map(|_| r.gen::<u8>()).collect();
        for &length in &[3u32, 100, 258] {
            let mut items = lits(&seed);
            items.push(Item::Match(length, dist));
            let plain = expand(&items, &[]);
            cases.push(Case::new(
                format!("dist={dist} len={length} sym={}", dist_symbol(dist)),
                fixed_stream(&items),
                plain.len() as i32,
            ));
            exp.push(plain);
        }
    }
    run(cases, exp);
}

// ---------------------------------------------------------------------- row 34
// the composed pipeline: cp_inflate then unfilter, PNG style.
#[test]
fn row34_inflate_then_unfilter_pipeline() {
    let mut r = rng(34);
    let mut cases = Vec::new();
    let mut plains = Vec::new();
    let mut geom = Vec::new();
    for _ in 0..40 {
        let bpp = [1i32, 2, 3, 4][r.gen_range(0..4)];
        let w = r.gen_range(1..=64i32);
        let h = r.gen_range(1..=16i32);
        let stride = 1 + (w * bpp) as usize;
        let mut filtered: Vec<u8> = (0..h as usize * stride).map(|_| r.gen::<u8>()).collect();
        for y in 0..h as usize {
            filtered[y * stride] = r.gen_range(0..=4u8);
        }
        cases.push(Case::new(
            format!("png w={w} h={h} bpp={bpp}"),
            deflate_raw(&filtered, 9),
            filtered.len() as i32,
        ));
        plains.push(filtered);
        geom.push((w, h, bpp));
    }
    let rep = run(cases.clone(), plains.clone());
    // second stage: unfilter the *inflated* bytes on both sides
    for (i, &(w, h, bpp)) in geom.iter().enumerate() {
        let inflated = &rep.c[i].out[..plains[i].len()];
        assert_eq!(inflated, &plains[i][..]);
        diff_unfilter(
            pair_release(),
            &format!("pipeline w={w} h={h} bpp={bpp}"),
            w,
            h,
            bpp,
            inflated,
        );
        diff_unfilter(
            pair_asserts(),
            &format!("pipeline(assert) w={w} h={h} bpp={bpp}"),
            w,
            h,
            bpp,
            inflated,
        );
    }
}

//! Phase B — rows I25…I29 of `CONFIGS.md`: the six exported *data* objects are
//! `static mut`/non-`const` in both libraries and are read on **every** call, so
//! mutating them through `dlsym` is part of the public configuration surface.
//!
//! Every mutation is applied **inside the forked child**, so each call sees a
//! private copy (fork COW) and the parent's tables stay pristine.

mod common;

use common::deflate::*;
use common::{expect_output, InflateHarness, Rng};

/// I25 — `cp_len_base[0]` 3 → 5: a length symbol 257 must now decode as a
/// 5-byte match in both libraries.
#[test]
fn i25_mutate_len_base() {
    let h = InflateHarness::new("i25", 1 << 16, 1 << 14);
    let mut rng = Rng::new(0x5025);
    for newbase in [3u32, 4, 5, 9, 64] {
        for it in 0..10 {
            let pre: Vec<u8> = (0..8).map(|_| rng.u8()).collect();
            let mut items: Vec<Item> = pre.iter().map(|&b| Item::Lit(b as u16)).collect();
            items.push(Item::RawMatch(257, 0, 0, 0)); // length symbol 257, distance 1
            let mut bw = BitWriter::new();
            emit_fixed_block(&mut bw, true, &items);
            let stream = bw.finish();

            let ctx = format!("I25 cp_len_base[0]={newbase} #{it}");
            let o = h.call_with_setup(&ctx, &stream, 0, 4096, &|lib| unsafe {
                *lib.cp_len_base = newbase;
            });
            // reference: 8 literals then `newbase` copies of the last byte
            let mut expected = pre.clone();
            for _ in 0..newbase {
                let v = *expected.last().unwrap();
                expected.push(v);
            }
            expect_output(&ctx, &o, &expected, 4096);
        }
    }
}

/// I26 — `cp_dist_base[0]` 1 → 2 and `cp_dist_extra_bits[0]` 0 → 2: both the
/// distance value and the number of bits consumed change.
#[test]
fn i26_mutate_dist_base_and_extra_bits() {
    let h = InflateHarness::new("i26", 1 << 16, 1 << 14);
    let pre: Vec<u8> = b"abcdefgh".to_vec();
    let mut items: Vec<Item> = pre.iter().map(|&b| Item::Lit(b as u16)).collect();
    items.push(Item::RawMatch(257, 0, 0, 0));
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, true, &items);
    let stream = bw.finish();

    for newdist in [1u32, 2, 3, 8] {
        let ctx = format!("I26 cp_dist_base[0]={newdist}");
        let o = h.call_with_setup(&ctx, &stream, 0, 4096, &|lib| unsafe {
            *lib.cp_dist_base = newdist;
        });
        let mut expected = pre.clone();
        for _ in 0..3 {
            let v = expected[expected.len() - newdist as usize];
            expected.push(v);
        }
        expect_output(&ctx, &o, &expected, 4096);
    }

    // extra-bit widths change how many bits are consumed; the result is then
    // stream-dependent, so only C-vs-Rust identity is asserted.
    for extra in [0u8, 1, 2, 5, 13] {
        let ctx = format!("I26 cp_dist_extra_bits[0]={extra}");
        h.call_with_setup(&ctx, &stream, 0, 4096, &|lib| unsafe {
            *lib.cp_dist_extra_bits = extra;
        });
        let ctx = format!("I26 cp_len_extra_bits[0]={extra}");
        h.call_with_setup(&ctx, &stream, 0, 4096, &|lib| unsafe {
            *lib.cp_len_extra_bits = extra;
        });
    }
}

/// I28 — `cp_fixed_table` replaced by a *different but still complete* canonical
/// length assignment; a stream encoded with the new table must decode.
#[test]
fn i28_mutate_fixed_table() {
    let h = InflateHarness::new("i28", 1 << 16, 1 << 14);
    let mut rng = Rng::new(0x5028);

    // literals 0..=255 -> 9 bits (256/512), symbols 256..=287 -> 6 bits (32/64):
    // Kraft = 256/512 + 32/64 = 1, i.e. a complete code.
    let mut newtab = [0u8; 320];
    for i in 0..256 {
        newtab[i] = 9;
    }
    for i in 256..288 {
        newtab[i] = 6;
    }
    for i in 288..320 {
        newtab[i] = 5;
    }
    let lit = HuffEnc::new(newtab[..288].to_vec());
    let dist = HuffEnc::new(newtab[288..].to_vec());
    assert_eq!(lit.kraft(), 1 << 15);
    assert_eq!(dist.kraft(), 1 << 15);

    for it in 0..20 {
        let n = rng.range(0, 60) as usize;
        let payload = rng.bytes(n);
        let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b as u16)).collect();
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(1, 2); // BTYPE = static
        emit_items(&mut bw, &lit, &dist, &items);
        let mut stream = bw.finish();
        stream.extend([0u8; 4]);

        let ctx = format!("I28 mutated cp_fixed_table #{it}");
        let o = h.call_with_setup(&ctx, &stream, 0, 4096, &|lib| unsafe {
            std::ptr::copy_nonoverlapping(newtab.as_ptr(), lib.cp_fixed_table, 320);
        });
        expect_output(&ctx, &o, &payload, 4096);
    }

    // Sanity: the *same* stream must fail with the pristine table (otherwise the
    // test would pass even if the global were ignored).
    let items: Vec<Item> = b"hello".iter().map(|&b| Item::Lit(b as u16)).collect();
    let mut bw = BitWriter::new();
    bw.bits(1, 1);
    bw.bits(1, 2);
    emit_items(&mut bw, &lit, &dist, &items);
    let mut stream = bw.finish();
    stream.extend([0u8; 4]);
    let with = h.call_with_setup("I28 with mutation", &stream, 0, 4096, &|lib| unsafe {
        std::ptr::copy_nonoverlapping(newtab.as_ptr(), lib.cp_fixed_table, 320);
    });
    let without = h.call("I28 without mutation", &stream, 0, 4096);
    assert_ne!(
        (with.ret, with.out.clone()),
        (without.ret, without.out.clone()),
        "mutating cp_fixed_table had no observable effect — the global is not live"
    );
}

/// I29 — `cp_permutation_order` reversed (still a permutation of 0…18), with the
/// dynamic header's code lengths written in the new order.
#[test]
fn i29_mutate_permutation_order() {
    let h = InflateHarness::new("i29", 1 << 16, 1 << 14);
    let mut rng = Rng::new(0x5029);

    let mut newperm = PERM;
    newperm.reverse();

    for it in 0..20 {
        let nlit = 257usize;
        let ndst = 1usize;
        let mut used = vec![256usize];
        while used.len() < 6 {
            let s = rng.below(200) as usize;
            if !used.contains(&s) {
                used.push(s);
            }
        }
        used.sort_unstable();
        let litlens = lengths_for(nlit, &used);
        let dstlens = lengths_for(ndst, &[0]);
        let items: Vec<Item> = (0..30)
            .map(|_| {
                let mut s = used[rng.below(used.len() as u32) as usize];
                if s == 256 {
                    s = used[0];
                }
                Item::Lit(s as u16)
            })
            .filter(|it| matches!(it, Item::Lit(v) if *v != 256))
            .collect();

        let mut bw = BitWriter::new();
        let (lit, dst) = emit_dynamic_header_with(
            &mut bw,
            true,
            &litlens,
            &dstlens,
            ClMode::Repeats,
            Some(19),
            &newperm,
        );
        emit_items(&mut bw, &lit, &dst, &items);
        let mut stream = bw.finish();
        stream.extend([0u8; 4]);
        let expected = expected_output(&items);

        let ctx = format!("I29 reversed cp_permutation_order #{it}");
        let o = h.call_with_setup(&ctx, &stream, 0, 4096, &|lib| unsafe {
            for (i, &v) in newperm.iter().enumerate() {
                *lib.cp_permutation_order.add(i) = v as u8;
            }
        });
        expect_output(&ctx, &o, &expected, 4096);
    }
}

/// Reading the globals back out through `dlsym` after a mutating call must show
/// the parent's copies untouched (proves the fork isolation the rows rely on).
#[test]
fn mutations_are_isolated_to_the_child() {
    let h = InflateHarness::new("iso", 1 << 16, 1 << 14);
    let stream = {
        let mut bw = BitWriter::new();
        emit_fixed_block(&mut bw, true, &[Item::Lit(65)]);
        bw.finish()
    };
    h.call_with_setup("iso", &stream, 0, 4096, &|lib| unsafe {
        *lib.cp_len_base = 999;
        *lib.cp_fixed_table = 3;
    });
    let (c, r) = common::libs();
    unsafe {
        assert_eq!(*c.cp_len_base, 3);
        assert_eq!(*r.cp_len_base, 3);
        assert_eq!(*c.cp_fixed_table, 8);
        assert_eq!(*r.cp_fixed_table, 8);
    }
}

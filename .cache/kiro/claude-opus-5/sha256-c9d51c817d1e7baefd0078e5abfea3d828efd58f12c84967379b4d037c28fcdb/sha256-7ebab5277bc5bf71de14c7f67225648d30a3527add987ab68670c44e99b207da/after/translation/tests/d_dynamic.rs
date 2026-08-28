//! Targeted coverage of `cp_dynamic` / `cp_build` / `cp_decode` using
//! hand-built btype-2 blocks, so the code-length run symbols 16/17/18, the
//! HCLEN padding and a wide range of HLIT/HDIST values are all hit
//! deterministically rather than at zlib's discretion.

mod common;

use common::deflate::{dynamic_block, ClMode, DynOpts, Item};
use common::{libs, Aligned, Rng};
use std::ffi::c_void;
use std::os::raw::c_int;

fn run_both(input: &[u8], out_bytes: usize, label: &str) -> (c_int, Vec<u8>) {
    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let slack = 32;
    let mut last = (0, Vec::new());
    for in_off in 0..4usize {
        let mut res = Vec::new();
        for lib in [&l.c, &l.rust] {
            lib.clear_error_reason();
            let inbuf = Aligned::from_slice_at(input, in_off);
            let outbuf = Aligned::new(out_bytes + slack);
            let ret = unsafe {
                (lib.cp_inflate)(
                    inbuf.at(in_off) as *mut c_void,
                    input.len() as c_int,
                    outbuf.ptr() as *mut c_void,
                    out_bytes as c_int,
                )
            };
            res.push((ret, outbuf.as_slice().to_vec(), lib.error_reason()));
        }
        assert_eq!(res[0].0, res[1].0, "{label} (align {in_off}): return differs");
        assert_eq!(
            res[0].1,
            res[1].1,
            "{label} (align {in_off}): output differs\n{}",
            common::hexdiff(&res[0].1, &res[1].1)
        );
        assert_eq!(
            res[0].2.as_deref().map(String::from_utf8_lossy),
            res[1].2.as_deref().map(String::from_utf8_lossy),
            "{label} (align {in_off}): cp_error_reason differs"
        );
        last = (res[0].0, res[0].1.clone());
    }
    last
}

fn check_dyn(items: &[Item], opts: &DynOpts, label: &str) {
    let mut w = common::deflate::BitWriter::new();
    let mut expect = Vec::new();
    dynamic_block(&mut w, true, items, &mut expect, opts);
    let input = w.finish();
    let (ret, out) = run_both(&input, expect.len().max(1), label);
    assert_eq!(ret, 1, "{label}: C rejected a well-formed dynamic block");
    if &out[..expect.len()] != &expect[..] {
        let at = (0..expect.len()).find(|&i| out[i] != expect[i]);
        panic!("{label}: plaintext differs at {at:?} (len {})", expect.len());
    }
}

#[test]
fn dynamic_literal_code_lengths() {
    // ClMode::Literal exercises only the `default:` arm of cp_dynamic's switch.
    let mut rng = Rng::new(0xd1_0000_0000_0001);
    for n in [1usize, 2, 3, 10, 50, 300, 1000] {
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
        check_dyn(
            &items,
            &DynOpts {
                cl_mode: ClMode::Literal,
                min_nlen: 4,
                single_dist: false,
                deep_tree: false,
            },
            &format!("dyn literal cl, {n} literals"),
        );
    }
}

#[test]
fn dynamic_run_length_code_lengths() {
    // ClMode::RunLength drives symbols 16 / 17 / 18.
    let mut rng = Rng::new(0xd2_0000_0000_0001);
    for n in [1usize, 2, 3, 10, 50, 300, 2000] {
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
        check_dyn(
            &items,
            &DynOpts::default(),
            &format!("dyn rle cl, {n} literals"),
        );
    }
}

#[test]
fn dynamic_narrow_alphabets() {
    // Few distinct literals -> long zero runs in the code-length stream, i.e.
    // heavy use of symbol 18 (and 17 for the short tails).
    for alphabet in [2usize, 3, 4, 5, 8, 16, 33, 64, 129, 255] {
        let items: Vec<Item> = (0..600)
            .map(|i| Item::Lit(((i * 37) % alphabet) as u8))
            .collect();
        for mode in [ClMode::Literal, ClMode::RunLength] {
            check_dyn(
                &items,
                &DynOpts {
                    cl_mode: mode,
                    min_nlen: 4,
                    single_dist: false,
                    deep_tree: false,
                },
                &format!("dyn alphabet {alphabet} mode {}", mode == ClMode::RunLength),
            );
        }
    }
}

#[test]
fn dynamic_hclen_padding() {
    // HCLEN is 4 + read_bits(4), so 4..=19 entries; padding it forces the
    // permutation-order loop to run to its maximum length.
    let mut rng = Rng::new(0xd3_0000_0000_0001);
    let items: Vec<Item> = (0..400).map(|_| Item::Lit(rng.u8() & 0x3f)).collect();
    for min_nlen in 4..=19usize {
        check_dyn(
            &items,
            &DynOpts {
                cl_mode: ClMode::RunLength,
                min_nlen,
                single_dist: false,
                deep_tree: false,
            },
            &format!("dyn HCLEN >= {min_nlen}"),
        );
    }
}

#[test]
fn dynamic_with_matches() {
    let mut rng = Rng::new(0xd4_0000_0000_0001);
    for distance in [1u32, 2, 3, 5, 16, 100, 255, 256, 1000, 4096, 20000] {
        let mut items: Vec<Item> = (0..distance.max(1) as usize)
            .map(|_| Item::Lit(rng.u8()))
            .collect();
        for length in [3u32, 10, 100, 258] {
            items.push(Item::Match { length, distance });
        }
        items.push(Item::Lit(0x7f));
        check_dyn(
            &items,
            &DynOpts::default(),
            &format!("dyn matches dist {distance}"),
        );
    }
}

#[test]
fn dynamic_all_length_and_distance_symbols() {
    // One block per (length symbol, distance symbol) family so every
    // extra-bits width in cp_len_extra_bits / cp_dist_extra_bits is used with
    // a dynamically built tree.
    let mut rng = Rng::new(0xd5_0000_0000_0001);
    for di in 0..30usize {
        let base = common::deflate::DIST_BASE[di];
        let extra = common::deflate::DIST_EXTRA[di];
        for distance in [base, base + ((1u32 << extra) - 1)] {
            let mut items: Vec<Item> =
                (0..distance as usize).map(|_| Item::Lit(rng.u8())).collect();
            for li in 0..29usize {
                let lb = common::deflate::LEN_BASE[li];
                let lx = common::deflate::LEN_EXTRA[li];
                for length in [lb, (lb + ((1u32 << lx) - 1)).min(258)] {
                    items.push(Item::Match { length, distance });
                }
            }
            check_dyn(
                &items,
                &DynOpts::default(),
                &format!("dyn dist sym {di} dist {distance} x all len syms"),
            );
        }
    }
}

#[test]
fn dynamic_multi_block_streams() {
    let mut rng = Rng::new(0xd6_0000_0000_0001);
    for nblocks in 2..=4usize {
        let mut w = common::deflate::BitWriter::new();
        let mut expect = Vec::new();
        for b in 0..nblocks {
            let n = 20 + rng.below(200) as usize;
            let mut items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
            items.push(Item::Match {
                length: 5,
                distance: 3,
            });
            let opts = if b % 2 == 0 {
                DynOpts::default()
            } else {
                DynOpts {
                    cl_mode: ClMode::Literal,
                    min_nlen: 19,
                    single_dist: false,
                    deep_tree: false,
                }
            };
            dynamic_block(&mut w, b + 1 == nblocks, &items, &mut expect, &opts);
        }
        let input = w.finish();
        let label = format!("dyn {nblocks} blocks");
        let (ret, out) = run_both(&input, expect.len(), &label);
        assert_eq!(ret, 1, "{label}: rejected");
        assert_eq!(&out[..expect.len()], &expect[..], "{label}: plaintext");
    }
}

#[test]
fn dynamic_mixed_block_types_in_one_stream() {
    // fixed -> dynamic -> fixed, all in one stream.
    let mut rng = Rng::new(0xd7_0000_0000_0001);
    let mut w = common::deflate::BitWriter::new();
    let mut expect = Vec::new();
    let a: Vec<Item> = (0..60).map(|_| Item::Lit(rng.u8())).collect();
    common::deflate::fixed_block(&mut w, false, &a, &mut expect);
    let b: Vec<Item> = (0..120)
        .map(|_| Item::Lit(rng.u8() & 0x1f))
        .chain([Item::Match {
            length: 20,
            distance: 7,
        }])
        .collect();
    dynamic_block(&mut w, false, &b, &mut expect, &DynOpts::default());
    let c: Vec<Item> = (0..30)
        .map(|_| Item::Lit(rng.u8()))
        .chain([Item::Match {
            length: 258,
            distance: 1,
        }])
        .collect();
    common::deflate::fixed_block(&mut w, true, &c, &mut expect);
    let input = w.finish();
    let (ret, out) = run_both(&input, expect.len(), "mixed block types");
    assert_eq!(ret, 1);
    assert_eq!(&out[..expect.len()], &expect[..]);
}

#[test]
fn dynamic_out_buffer_pressure() {
    // Same stream truncated at every output size, so cp_block's two bounds
    // checks fire at many different points inside a dynamic block.
    let mut rng = Rng::new(0xd8_0000_0000_0001);
    let mut items: Vec<Item> = (0..40).map(|_| Item::Lit(rng.u8())).collect();
    items.push(Item::Match {
        length: 60,
        distance: 8,
    });
    items.extend((0..10).map(|_| Item::Lit(rng.u8())));

    let mut w = common::deflate::BitWriter::new();
    let mut expect = Vec::new();
    dynamic_block(&mut w, true, &items, &mut expect, &DynOpts::default());
    let input = w.finish();
    for out_bytes in 0..=expect.len() {
        run_both(&input, out_bytes, &format!("dyn out_bytes={out_bytes}"));
    }
}

#[test]
fn dynamic_single_distance_code() {
    // HDIST == 1 with one 1-bit distance code: a one-entry tree, which makes
    // cp_build return max_index == 1 and cp_decode binary-search a single slot.
    let mut rng = Rng::new(0xd9_0000_0000_0001);
    for n in [1usize, 5, 100, 700] {
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
        for mode in [ClMode::Literal, ClMode::RunLength] {
            check_dyn(
                &items,
                &DynOpts {
                    cl_mode: mode,
                    min_nlen: 4,
                    single_dist: true,
                    deep_tree: false,
                },
                &format!("dyn single dist code, {n} literals"),
            );
        }
    }
}

#[test]
fn dynamic_deep_code_lengths() {
    // Push the literal tree toward the 15-bit maximum so cp_build's
    // `len <= 9` lookup shortcut is skipped for many symbols and cp_decode
    // has to walk deeper into the sorted tree.
    for nsyms in [4usize, 8, 12, 16, 20, 24, 30, 40] {
        let items: Vec<Item> = (0..(nsyms * 40))
            .map(|i| Item::Lit((i % nsyms) as u8))
            .collect();
        check_dyn(
            &items,
            &DynOpts {
                cl_mode: ClMode::RunLength,
                min_nlen: 4,
                single_dist: false,
                deep_tree: true,
            },
            &format!("dyn deep tree, {nsyms} symbols"),
        );
    }
}

#[test]
fn stored_block_after_another_block() {
    // cp_stored re-aligns with `cp_read_bits(s, s->count & 7)`, so reaching it
    // from a non-byte-aligned position must still work.
    let mut rng = Rng::new(0xda_0000_0000_0001);
    for nlits in 0..24usize {
        let mut w = common::deflate::BitWriter::new();
        let mut expect = Vec::new();
        let a: Vec<Item> = (0..nlits).map(|_| Item::Lit(rng.u8())).collect();
        common::deflate::fixed_block(&mut w, false, &a, &mut expect);
        let payload: Vec<u8> = (0..17u8).collect();
        common::deflate::stored_block(&mut w, true, &payload, &mut expect);
        let input = w.finish();
        let label = format!("fixed({nlits}) then stored");
        let (ret, out) = run_both(&input, expect.len().max(1), &label);
        assert_eq!(ret, 1, "{label}: rejected");
        assert_eq!(&out[..expect.len()], &expect[..], "{label}: plaintext");
    }

    // ...and from a dynamic block, whose bit length is not a multiple of 8.
    for nlits in [10usize, 11, 12, 13, 14, 15, 16, 17] {
        let mut w = common::deflate::BitWriter::new();
        let mut expect = Vec::new();
        let a: Vec<Item> = (0..nlits).map(|_| Item::Lit(rng.u8() & 0x0f)).collect();
        dynamic_block(&mut w, false, &a, &mut expect, &DynOpts::default());
        let payload: Vec<u8> = (0..9u8).map(|i| i * 3).collect();
        common::deflate::stored_block(&mut w, true, &payload, &mut expect);
        let input = w.finish();
        let label = format!("dynamic({nlits}) then stored");
        let (ret, out) = run_both(&input, expect.len().max(1), &label);
        assert_eq!(ret, 1, "{label}: rejected");
        assert_eq!(&out[..expect.len()], &expect[..], "{label}: plaintext");
    }
}

#[test]
fn two_stored_blocks_rejected_identically() {
    // The second stored block leaves trailing input, so `bits_left / 8 <= LEN`
    // fails for the first one. Whatever the C code reports, Rust must match.
    let mut w = common::deflate::BitWriter::new();
    let mut expect = Vec::new();
    common::deflate::stored_block(&mut w, false, &[1, 2, 3, 4], &mut expect);
    common::deflate::stored_block(&mut w, true, &[5, 6, 7, 8], &mut expect);
    let input = w.finish();
    let (ret, _) = run_both(&input, 64, "two stored blocks");
    assert_eq!(ret, 0, "expected the C code to reject a non-final stored block");
}

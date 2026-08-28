//! Broad randomised differential fuzzing of `cp_inflate`.
//!
//! Streams are assembled from randomly chosen well-formed blocks (stored,
//! fixed, dynamic) with random literals, match lengths and back-distances, and
//! replayed against both libraries at every input alignment and a range of
//! output-buffer sizes.
//!
//! Streams are kept well-formed on purpose: the default C build compiles with
//! `assert()` enabled, so a corrupt stream aborts inside the C library instead
//! of returning an error we could compare.

mod common;

use common::deflate::{dynamic_block, fixed_block, stored_block, BitWriter, ClMode, DynOpts, Item};
use common::{libs, Aligned, Rng};
use std::ffi::c_void;
use std::os::raw::c_int;

fn diff(input: &[u8], out_bytes: usize, label: &str) -> c_int {
    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let slack = 24;
    let mut first_ret = None;
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
        assert_eq!(
            res[0].0, res[1].0,
            "{label} (align {in_off}): return {} vs {}",
            res[0].0, res[1].0
        );
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
        first_ret = Some(res[0].0);
    }
    first_ret.unwrap()
}

/// Build a random well-formed stream.
/// Returns (bytes, expected plaintext, contains_stored_block).
///
/// `allow_stored` matters because `cp_stored` copies `LEN` bytes into the output
/// buffer with no bounds check at all, so a stored block combined with an
/// undersized output buffer corrupts the heap inside the C library. Callers that
/// shrink the output buffer must pass `false`.
fn random_stream(rng: &mut Rng, allow_stored: bool) -> (Vec<u8>, Vec<u8>, bool) {
    let mut w = BitWriter::new();
    let mut expect: Vec<u8> = Vec::new();
    let nblocks = 1 + rng.below(4) as usize;
    let mut has_stored = false;

    for b in 0..nblocks {
        let last = b + 1 == nblocks;
        // A stored block must terminate the stream for this decoder, so only
        // pick it for the final block.
        let kind = if last && allow_stored {
            rng.below(3)
        } else {
            1 + rng.below(2)
        };

        let nitems = 1 + rng.below(120) as usize;
        let mut items: Vec<Item> = Vec::with_capacity(nitems);
        let mut produced = expect.len();
        for _ in 0..nitems {
            if produced > 0 && rng.below(3) == 0 {
                let distance = 1 + rng.below(produced.min(32768) as u64) as u32;
                let length = 3 + rng.below(256) as u32;
                items.push(Item::Match { length, distance });
                produced += length as usize;
            } else {
                items.push(Item::Lit(rng.u8()));
                produced += 1;
            }
        }

        match kind {
            0 => {
                let n = rng.below(300) as usize;
                let data: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
                stored_block(&mut w, true, &data, &mut expect);
                has_stored = true;
            }
            1 => fixed_block(&mut w, last, &items, &mut expect),
            _ => {
                let opts = DynOpts {
                    cl_mode: if rng.below(2) == 0 {
                        ClMode::Literal
                    } else {
                        ClMode::RunLength
                    },
                    min_nlen: 4 + rng.below(16) as usize,
                    single_dist: false,
                    deep_tree: rng.below(4) == 0,
                };
                dynamic_block(&mut w, last, &items, &mut expect, &opts);
            }
        }
    }
    (w.finish(), expect, has_stored)
}

#[test]
fn fuzz_random_streams() {
    let mut rng = Rng::new(0xf0_0d_1234_5678);
    for iter in 0..1500 {
        let (input, expect, _) = random_stream(&mut rng, true);
        let label = format!("fuzz stream {iter} (in {} bytes)", input.len());
        let _ = diff(&input, expect.len().max(1), &label);
    }
}

#[test]
fn fuzz_random_streams_truncated_output() {
    // Same streams, but with an output buffer too small at a random point, so
    // cp_block's bounds checks fire in the middle of arbitrary block types.
    let mut rng = Rng::new(0xf1_0d_8765_4321);
    for iter in 0..800 {
        let (input, expect, has_stored) = random_stream(&mut rng, false);
        assert!(!has_stored);
        let n = expect.len().max(1);
        let out_bytes = rng.below(n as u64 + 1) as usize;
        diff(
            &input,
            out_bytes,
            &format!("fuzz truncated {iter} out_bytes={out_bytes}/{n}"),
        );
    }
}

#[test]
fn fuzz_zlib_round_trip() {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut rng = Rng::new(0xf2_0d_dead_beef);
    for iter in 0..400 {
        // Mixtures of runs, random noise and a small alphabet exercise all of
        // zlib's block-type decisions.
        let n = rng.below(20_000) as usize;
        let mut data = Vec::with_capacity(n);
        while data.len() < n {
            match rng.below(4) {
                0 => {
                    let run = 1 + rng.below(300) as usize;
                    let byte = rng.u8();
                    data.extend(std::iter::repeat(byte).take(run.min(n - data.len())));
                }
                1 => {
                    let run = 1 + rng.below(200) as usize;
                    for _ in 0..run.min(n - data.len()) {
                        data.push(rng.u8());
                    }
                }
                2 => {
                    let run = 1 + rng.below(400) as usize;
                    for i in 0..run.min(n - data.len()) {
                        data.push((i % 7) as u8 + b'a');
                    }
                }
                _ => {
                    let take = (1 + rng.below(500) as usize).min(n - data.len());
                    let src = data.len().saturating_sub(1 + rng.below(64) as usize);
                    for k in 0..take {
                        let b = if data.is_empty() {
                            0
                        } else {
                            data[(src + k) % data.len()]
                        };
                        data.push(b);
                    }
                }
            }
        }
        data.truncate(n);

        let level = rng.below(10) as u32;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
        e.write_all(&data).unwrap();
        let input = e.finish().unwrap();

        let label = format!("zlib fuzz {iter} level {level} plain {n}");
        let ret = diff(&input, data.len().max(1), &label);
        let _ = ret;
    }
}

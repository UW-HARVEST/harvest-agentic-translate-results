//! `cp_inflate` -- exported by the C `.so`, therefore part of the ABI the Rust
//! `.so` must reproduce.
//!
//! The default C build has `assert()` live (no `NDEBUG`), so a malformed stream
//! aborts the process inside the C library rather than returning. Inputs here
//! are therefore well-formed DEFLATE, plus the specific failure paths the C code
//! reports through `cp_error_reason` without tripping an assertion.

mod common;

use common::deflate::{fixed_block, stored_block, stored_block_raw, BitWriter, Item};
use common::{libs, Aligned, Rng};
use std::ffi::c_void;
use std::os::raw::c_int;

struct Outcome {
    ret: c_int,
    out: Vec<u8>,
    err: Option<Vec<u8>>,
}

/// Call `cp_inflate` on one library with the input placed at `in_off` bytes
/// past a 16-byte-aligned base (so `first_bytes` is exercised for every
/// alignment) and return everything observable.
fn run(lib: &common::Lib, input: &[u8], in_off: usize, out_bytes: usize, out_slack: usize) -> Outcome {
    lib.clear_error_reason();
    let inbuf = Aligned::from_slice_at(input, in_off);
    let outbuf = Aligned::new(out_bytes + out_slack);
    let ret = unsafe {
        (lib.cp_inflate)(
            inbuf.at(in_off) as *mut c_void,
            input.len() as c_int,
            outbuf.ptr() as *mut c_void,
            out_bytes as c_int,
        )
    };
    Outcome {
        ret,
        out: outbuf.as_slice().to_vec(),
        err: lib.error_reason(),
    }
}

/// Differential check across all four input alignments.
/// Returns the C library's return code and output buffer.
fn check(input: &[u8], out_bytes: usize, label: &str) -> (c_int, Vec<u8>) {
    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let slack = 32;
    let mut last = (0, Vec::new());
    for in_off in 0..4usize {
        let c = run(&l.c, input, in_off, out_bytes, slack);
        let r = run(&l.rust, input, in_off, out_bytes, slack);
        assert_eq!(
            c.ret, r.ret,
            "{label} (align {in_off}): return {} vs {}, c_err={:?} rust_err={:?}",
            c.ret,
            r.ret,
            c.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            r.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        );
        assert_eq!(
            c.out,
            r.out,
            "{label} (align {in_off}): output differs\n{}",
            common::hexdiff(&c.out, &r.out)
        );
        assert_eq!(
            c.err.as_deref().map(String::from_utf8_lossy),
            r.err.as_deref().map(String::from_utf8_lossy),
            "{label} (align {in_off}): cp_error_reason differs"
        );
        last = (c.ret, c.out);
    }
    last
}

/// Check, and additionally assert the decoded prefix equals `expect`.
fn check_ok(input: &[u8], expect: &[u8], label: &str) {
    let (ret, out) = check(input, expect.len().max(1), label);
    assert_eq!(ret, 1, "{label}: C library rejected a well-formed stream");
    assert_plaintext(&out, expect, label);
}

fn assert_plaintext(out: &[u8], expect: &[u8], label: &str) {
    if &out[..expect.len()] != expect {
        let at = (0..expect.len()).find(|&i| out[i] != expect[i]);
        panic!(
            "{label}: decoded plaintext differs from the reference encoder \
             (len {}, first difference at {:?})",
            expect.len(),
            at
        );
    }
}

// ---------------------------------------------------------------------------
// btype 1 -- fixed Huffman
// ---------------------------------------------------------------------------

#[test]
fn fixed_literals_all_byte_values() {
    // Sweep every literal so both the 7/8/9-bit fixed code lengths are used.
    for chunk in 0..8usize {
        let mut w = BitWriter::new();
        let mut expect = Vec::new();
        let items: Vec<Item> = (0..32u16)
            .map(|i| Item::Lit((chunk * 32 + i as usize) as u8))
            .collect();
        fixed_block(&mut w, true, &items, &mut expect);
        let input = w.finish();
        check_ok(&input, &expect, &format!("fixed literals chunk {chunk}"));
    }

    let mut w = BitWriter::new();
    let mut expect = Vec::new();
    let items: Vec<Item> = (0..=255u8).map(Item::Lit).collect();
    fixed_block(&mut w, true, &items, &mut expect);
    check_ok(&w.finish(), &expect, "fixed literals 0..=255");
}

#[test]
fn fixed_matches_all_length_symbols() {
    // Every length symbol (257..=285) with its minimum and maximum extra-bit
    // value, at a distance large enough to stay non-overlapping.
    for li in 0..29usize {
        let base = common::deflate::LEN_BASE[li];
        let extra = common::deflate::LEN_EXTRA[li];
        let maxlen = (base + ((1u32 << extra) - 1)).min(258);
        for length in [base, maxlen] {
            let mut w = BitWriter::new();
            let mut expect = Vec::new();
            let mut items: Vec<Item> = (0..258u32).map(|i| Item::Lit((i * 7 + 3) as u8)).collect();
            items.push(Item::Match {
                length,
                distance: 258,
            });
            fixed_block(&mut w, true, &items, &mut expect);
            check_ok(
                &w.finish(),
                &expect,
                &format!("fixed len sym {} len {length}", 257 + li),
            );
        }
    }
}

#[test]
fn fixed_matches_all_distance_symbols() {
    for di in 0..30usize {
        let base = common::deflate::DIST_BASE[di];
        let extra = common::deflate::DIST_EXTRA[di];
        let maxdist = base + ((1u32 << extra) - 1);
        for distance in [base, maxdist] {
            // need at least `distance` bytes of history
            let mut w = BitWriter::new();
            let mut expect = Vec::new();
            let mut items: Vec<Item> = (0..distance)
                .map(|i| Item::Lit((i.wrapping_mul(31).wrapping_add(7)) as u8))
                .collect();
            items.push(Item::Match {
                length: 3,
                distance,
            });
            items.push(Item::Match {
                length: 258,
                distance,
            });
            fixed_block(&mut w, true, &items, &mut expect);
            check_ok(
                &w.finish(),
                &expect,
                &format!("fixed dist sym {di} dist {distance}"),
            );
        }
    }
}

#[test]
fn fixed_overlapping_and_memset_copies() {
    // distance == 1 takes the `memset` fast path; 2..=8 take the overlapping
    // byte-by-byte loop, whose semantics differ from a plain memcpy.
    for distance in 1..=8u32 {
        for length in [3u32, 4, 7, 8, 9, 16, 17, 31, 32, 33, 100, 257, 258] {
            let mut w = BitWriter::new();
            let mut expect = Vec::new();
            let mut items: Vec<Item> = (0..distance).map(|i| Item::Lit(0xA0 | i as u8)).collect();
            items.push(Item::Match { length, distance });
            items.push(Item::Lit(0x5A));
            items.push(Item::Match {
                length: 3,
                distance: 1,
            });
            fixed_block(&mut w, true, &items, &mut expect);
            check_ok(
                &w.finish(),
                &expect,
                &format!("overlap dist {distance} len {length}"),
            );
        }
    }
}

#[test]
fn fixed_multiple_blocks() {
    let mut rng = Rng::new(0x51de_1234_5678_0001);
    for nblocks in 1..=5usize {
        let mut w = BitWriter::new();
        let mut expect = Vec::new();
        for b in 0..nblocks {
            let n = 1 + rng.below(40) as usize;
            let mut items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
            if !expect.is_empty() || n >= 4 {
                items.push(Item::Match {
                    length: 3,
                    distance: 1,
                });
            }
            fixed_block(&mut w, b + 1 == nblocks, &items, &mut expect);
        }
        check_ok(&w.finish(), &expect, &format!("{nblocks} fixed blocks"));
    }
}

// ---------------------------------------------------------------------------
// btype 0 -- stored
// ---------------------------------------------------------------------------

#[test]
fn stored_block_final_only() {
    let mut rng = Rng::new(0x5107_ed00_0000_0001);
    // The C code demands `bits_left / 8 <= LEN`, i.e. a stored block must be
    // the last thing in the stream.
    for n in [0usize, 1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 64, 255, 1000] {
        let data: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        let mut w = BitWriter::new();
        let mut expect = Vec::new();
        stored_block(&mut w, true, &data, &mut expect);
        check(&w.finish(), expect.len().max(1), &format!("stored {n} bytes"));
    }
}

#[test]
fn stored_block_len_nlen_mismatch() {
    let data: Vec<u8> = (0..16u8).collect();
    let mut w = BitWriter::new();
    stored_block_raw(&mut w, true, 16, 0x1234, &data);
    let _ = check(&w.finish(), 64, "stored LEN/NLEN mismatch");

    // ...and confirm the reported reason really is the LEN/NLEN one.
    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let mut w = BitWriter::new();
    stored_block_raw(&mut w, true, 16, 0x1234, &data);
    let input = w.finish();
    let c = run(&l.c, &input, 0, 64, 16);
    assert_eq!(c.ret, 0);
    assert_eq!(
        String::from_utf8_lossy(c.err.as_deref().unwrap()),
        "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."
    );
}

#[test]
fn stored_block_extends_beyond_stream() {
    // Trailing bytes after the stored payload make `bits_left / 8 > LEN`.
    let data: Vec<u8> = (0..8u8).collect();
    for trailing in 1..=6usize {
        let mut w = BitWriter::new();
        stored_block_raw(&mut w, true, 8, !8u16, &data);
        let mut input = w.finish();
        input.extend(std::iter::repeat(0xAAu8).take(trailing));
        check(&input, 64, &format!("stored + {trailing} trailing bytes"));
    }
}

// ---------------------------------------------------------------------------
// btype 2 -- dynamic Huffman (produced by zlib via flate2)
// ---------------------------------------------------------------------------

fn raw_deflate(data: &[u8], level: u32) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn corpus() -> Vec<(&'static str, Vec<u8>)> {
    let mut rng = Rng::new(0xc0ffee_1234_5678);
    let mut v: Vec<(&'static str, Vec<u8>)> = Vec::new();
    v.push(("empty", Vec::new()));
    v.push(("one byte", vec![0x42]));
    v.push(("short text", b"hello hello hello world".to_vec()));
    v.push((
        "lorem",
        b"the quick brown fox jumps over the lazy dog. \
          the quick brown fox jumps over the lazy dog. \
          THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG!!"
            .to_vec(),
    ));
    v.push(("all zeros 64k", vec![0u8; 65536]));
    v.push(("all 0xff 4k", vec![0xffu8; 4096]));
    v.push((
        "counting 100k",
        (0..100_000usize).map(|i| (i % 251) as u8).collect(),
    ));
    v.push((
        "random 32k",
        (0..32_768usize).map(|_| rng.u8()).collect::<Vec<u8>>(),
    ));
    v.push((
        "sparse",
        (0..50_000usize)
            .map(|i| if i % 977 == 0 { 0xAB } else { 0 })
            .collect(),
    ));
    v.push((
        "skewed alphabet",
        (0..80_000usize)
            .map(|i| if i % 13 == 0 { b'z' } else { b'a' })
            .collect(),
    ));
    v.push((
        "two symbol",
        (0..5000usize).map(|i| if i % 2 == 0 { 0 } else { 255 }).collect(),
    ));
    v
}

#[test]
fn zlib_streams_all_levels() {
    for (name, data) in corpus() {
        for level in [1u32, 2, 4, 6, 9] {
            let input = raw_deflate(&data, level);
            let label = format!("zlib level {level} / {name}");
            let (ret, out) = check(&input, data.len().max(1), &label);
            // Not every zlib stream is accepted by this decoder (e.g. stored
            // blocks that are not the last thing in the stream); the point of
            // the test is that C and Rust agree, which `check` asserted.
            if ret == 1 {
                assert_plaintext(&out, &data, &label);
            }
        }
    }
}

#[test]
fn zlib_level_zero_stored_streams() {
    // Level 0 emits stored blocks; whatever the C code decides (success for a
    // lone final block, failure otherwise) the Rust side must agree.
    for (name, data) in corpus() {
        let input = raw_deflate(&data, 0);
        check(
            &input,
            data.len().max(1) + 8,
            &format!("zlib level 0 / {name}"),
        );
    }
}

#[test]
fn zlib_streams_every_input_length() {
    // Sweeps short lengths so the tail handling (`last_bytes` / `final_word`)
    // is hit for every `in_bytes % 4` and every `first_bytes`.
    let mut rng = Rng::new(0x1010_2020_3030_4041);
    for n in 0..300usize {
        let data: Vec<u8> = (0..n).map(|_| (rng.u8() & 0x0f) + b'a').collect();
        for level in [1u32, 6, 9] {
            let input = raw_deflate(&data, level);
            let label = format!("len {n} level {level}");
            let (ret, out) = check(&input, data.len().max(1), &label);
            // Not every zlib stream is accepted by this decoder (e.g. stored
            // blocks that are not the last thing in the stream); the point of
            // the test is that C and Rust agree, which `check` asserted.
            if ret == 1 {
                assert_plaintext(&out, &data, &label);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// error paths inside cp_block
// ---------------------------------------------------------------------------

#[test]
fn out_buffer_too_small_for_symbol() {
    let mut w = BitWriter::new();
    let mut expect = Vec::new();
    let items: Vec<Item> = (0..40u8).map(|i| Item::Lit(i.wrapping_mul(9))).collect();
    fixed_block(&mut w, true, &items, &mut expect);
    let input = w.finish();
    for out_bytes in 0..40usize {
        check(
            &input,
            out_bytes,
            &format!("literal overflow out_bytes={out_bytes}"),
        );
    }

    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let c = run(&l.c, &input, 0, 5, 32);
    assert_eq!(c.ret, 0);
    assert_eq!(
        String::from_utf8_lossy(c.err.as_deref().unwrap()),
        "Attempted to overwrite out buffer while outputting a symbol."
    );
}

#[test]
fn out_buffer_too_small_for_string() {
    let mut w = BitWriter::new();
    let mut expect = Vec::new();
    let items = vec![
        Item::Lit(1),
        Item::Lit(2),
        Item::Lit(3),
        Item::Lit(4),
        Item::Match {
            length: 100,
            distance: 4,
        },
    ];
    fixed_block(&mut w, true, &items, &mut expect);
    let input = w.finish();
    for out_bytes in 5..104usize {
        check(&input, out_bytes, &format!("string overflow out={out_bytes}"));
    }

    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let c = run(&l.c, &input, 0, 10, 128);
    assert_eq!(c.ret, 0);
    assert_eq!(
        String::from_utf8_lossy(c.err.as_deref().unwrap()),
        "Attempted to overwrite out buffer while outputting a string."
    );
}

#[test]
fn invalid_backwards_distance() {
    // A match that reaches before the start of the out buffer.
    let mut w = BitWriter::new();
    // Craft manually: two literals then a match with distance 8 (> history).
    w.block_header(1, 1);
    w.fixed_literal(b'x');
    w.fixed_literal(b'y');
    w.fixed_match(3, 8);
    w.fixed_end_of_block();

    let input = w.finish();
    check(&input, 64, "backwards distance underflow");

    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let c = run(&l.c, &input, 0, 64, 32);
    assert_eq!(c.ret, 0);
    assert_eq!(
        String::from_utf8_lossy(c.err.as_deref().unwrap()),
        "Attempted to write before out buffer (invalid backwards distance)."
    );
}

#[test]
fn unknown_block_type() {
    // bfinal = 1, btype = 3
    for extra in [1usize, 2, 3, 4, 8, 16] {
        let mut input = vec![0u8; extra];
        input[0] = 0b0000_0111;
        check(&input, 32, &format!("btype 3, {extra} input bytes"));
    }

    let l = libs();
    let _g = common::ERR_LOCK.lock().unwrap();
    let input = vec![0b0000_0111u8, 0, 0, 0, 0, 0, 0, 0];
    let c = run(&l.c, &input, 0, 32, 32);
    assert_eq!(c.ret, 0);
    assert_eq!(
        String::from_utf8_lossy(c.err.as_deref().unwrap()),
        "Detected unknown block type within input stream."
    );
}

#[test]
fn out_buffer_exactly_sized() {
    // `s->out + 1 <= s->out_end` means an exactly-sized buffer must still work.
    let mut rng = Rng::new(0x4578_6163_7400_0001);
    for n in 1..200usize {
        let data: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        let input = raw_deflate(&data, 6);
        let label = format!("exact out {n}");
        let (ret, out) = check(&input, n, &label);
        assert_eq!(ret, 1, "{label}: rejected");
        assert_plaintext(&out, &data, &label);
    }
}

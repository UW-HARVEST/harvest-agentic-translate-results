//! Level 1: the bit reader and `cp_stored`.
//!
//! Stored blocks are the shortest path through `pinflate`, so they isolate the
//! initial bit-buffer priming (`first_bytes`, `final_word`, `bits_left`),
//! `cp_read_bits`, the byte-alignment skip and `cp_ptr`'s pointer arithmetic.
//!
//! Note the C code's guard `s->bits_left / 8 <= LEN`: a stored block only
//! decodes when nothing follows it in the input, so these streams carry no
//! padding.

mod harness;

use harness::deflate::*;
use harness::Differ;

fn stored_stream(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter::new();
    write_stored_block(&mut w, true, data);
    w.finish()
}

#[test]
fn stored_blocks_various_lengths_and_alignments() {
    let mut d = Differ::new();
    for len in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 64, 100] {
        let data: Vec<u8> = (0..len)
            .map(|i| (i as u8).wrapping_mul(37).wrapping_add(11))
            .collect();
        let stream = stored_stream(&data);
        for offset in 0..8usize {
            d.check_ret1(
                &format!("stored len={len} off={offset}"),
                &stream,
                offset,
                data.len().max(1) + 8,
            );
        }
    }
    d.finish("stored blocks");
}

/// Anchor test: when the input ends on a word boundary (`last_bytes == 0`) the
/// `final_word` path never runs, `count` stays exact and `cp_ptr` lands on the
/// real payload -- so the C code produces the textbook inflate output. This
/// pins down that the harness really is decoding, not just agreeing on garbage.
#[test]
fn stored_block_word_aligned_produces_real_payload() {
    let mut d = Differ::new();
    let mut anchored = 0usize;
    for len in 0..40usize {
        let data: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let stream = stored_stream(&data);
        // offset 0 keeps the buffer word-aligned (Vec data is at least 8-byte
        // aligned), so first_bytes == 0 and last_bytes == stream.len() % 4.
        if stream.len() % 4 != 0 {
            continue;
        }
        anchored += 1;
        d.check_ok(
            &format!("aligned stored len={len}"),
            &stream,
            0,
            data.len() + 8,
            &data,
        );
    }
    assert!(anchored >= 8, "expected several word-aligned cases");
    d.finish("word-aligned stored blocks");
}

#[test]
fn stored_block_output_buffer_sizes() {
    // `cp_stored` performs no bounds check against `out_end`, so it overruns
    // small output buffers. Both sides must overrun identically.
    let mut d = Differ::new();
    let data: Vec<u8> = (0..40u8).collect();
    let stream = stored_stream(&data);
    for out_bytes in [0usize, 1, 8, 39, 40, 41, 128] {
        for offset in 0..4usize {
            d.check(
                &format!("stored out_bytes={out_bytes} off={offset}"),
                &stream,
                offset,
                out_bytes,
            );
        }
    }
    d.finish("stored output sizes");
}

#[test]
fn stored_block_len_nlen_mismatch() {
    let mut d = Differ::new();
    for (len_field, nlen_field) in [
        (4u32, 0u32),
        (4, 0xFFFF),
        (0, 0),
        (10, 0xFFF4),
        (0xFFFF, 0),
        (1, 0xFFFE),
    ] {
        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(0, 2);
        w.align();
        w.bits(len_field, 16);
        w.bits(nlen_field, 16);
        for i in 0..8u32 {
            w.bits(i, 8);
        }
        let stream = w.finish();
        d.check(
            &format!("stored LEN={len_field:#x} NLEN={nlen_field:#x}"),
            &stream,
            0,
            64,
        );
    }
    d.finish("stored LEN/NLEN");
}

#[test]
fn stored_block_length_beyond_input() {
    // Exercises the "Stored block extends beyond end of input stream." guard
    // by declaring a LEN smaller than the bytes that actually follow.
    let mut d = Differ::new();
    for declared in [0usize, 1, 2, 4, 8] {
        for trailing in [0usize, 1, 4, 9, 20] {
            let mut w = BitWriter::new();
            w.bits(1, 1);
            w.bits(0, 2);
            w.align();
            w.bits(declared as u32, 16);
            w.bits(!(declared as u32) & 0xFFFF, 16);
            for i in 0..trailing {
                w.bits((i as u32) & 0xFF, 8);
            }
            let stream = w.finish();
            d.check(
                &format!("stored declared={declared} trailing={trailing}"),
                &stream,
                0,
                64,
            );
        }
    }
    d.finish("stored length guard");
}

#[test]
fn multiple_stored_blocks() {
    // Only the final block may be a stored one (see the guard above), so this
    // chains a fixed literal block in front of it.
    let mut d = Differ::new();
    let tail: Vec<u8> = (0..12u8).map(|i| i ^ 0x5A).collect();
    for prefix in [
        vec![],
        vec![Token::Lit(b'x')],
        vec![Token::Lit(b'a'), Token::Lit(b'b'), Token::Lit(b'c')],
    ] {
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, false, &prefix);
        write_stored_block(&mut w, true, &tail);
        let stream = w.finish();
        let mut expected = expand(&prefix);
        expected.extend_from_slice(&tail);
        for offset in 0..4usize {
            d.check_ret1(
                &format!("fixed+stored prefix={} off={offset}", prefix.len()),
                &stream,
                offset,
                expected.len() + 16,
            );
        }
    }
    d.finish("chained blocks");
}

#[test]
fn truncated_and_tiny_inputs() {
    // Deliberately malformed / short inputs. Many of these trip an assertion
    // in both implementations; the harness only requires that C and Rust agree
    // on *whether* they die and on anything they produced before doing so.
    let mut d = Differ::new();
    let stream = stored_stream(&(0..24u8).collect::<Vec<u8>>());
    for take in 0..stream.len() {
        d.check(
            &format!("truncated to {take}"),
            &stream[..take],
            0,
            64,
        );
    }
    for bytes in [
        vec![],
        vec![0x00],
        vec![0x01],
        vec![0x02],
        vec![0x03],
        vec![0x06],
        vec![0x07],
        vec![0xFF],
        vec![0x00, 0x00],
        vec![0x03, 0x00],
    ] {
        d.check(&format!("tiny {bytes:02x?}"), &bytes, 0, 64);
    }
    d.finish("truncated inputs");
}

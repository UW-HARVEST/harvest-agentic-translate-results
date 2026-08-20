//! Phase C -- error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each row constructs the exact invalid input/condition, runs it against BOTH
//! shared objects through `libloading`, and asserts they reject it the *same
//! way* -- the same `pinflate` return value and the same `cp_error_reason`
//! string for the explicit rejections, and the same fatal signal for the
//! `assert()` rows (the reference C library is built without `-DNDEBUG`, so a
//! failed assert is a caller-visible `abort()`; see `ERRORS.md`).

mod common;

use common::enc::*;
use common::shared::{hex, Case, Outcome};
use common::*;

const ERR_LEN_NLEN: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const ERR_STORED_BEYOND: &str = "Stored block extends beyond end of input stream.";
const ERR_OUT_SYMBOL: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const ERR_BACK_DIST: &str = "Attempted to write before out buffer (invalid backwards distance).";
const ERR_OUT_STRING: &str = "Attempted to overwrite out buffer while outputting a string.";
const ERR_UNKNOWN_BLOCK: &str = "Detected unknown block type within input stream.";

/// A stored block written by hand so LEN and NLEN can be set independently.
fn stored_raw(len_field: u16, nlen_field: u16, payload: &[u8], bfinal: bool) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.bit(bfinal as u32);
    w.bits(0, 2);
    w.align();
    w.bits(len_field as u32, 16);
    w.bits(nlen_field as u32, 16);
    for b in payload {
        w.bits(*b as u32, 8);
    }
    w.bytes
}

fn fixed_stream(items: &[Item]) -> Vec<u8> {
    let mut w = BitWriter::new();
    fixed_block(&mut w, items, true);
    w.align();
    w.bytes
}

/// A fixed block that emits a raw (length symbol, distance symbol) pair without
/// checking that the back-reference is in range -- needed for E4/E25/E26/E27.
fn fixed_raw_match(len_sym_idx: usize, len_extra: u32, dist_sym_idx: usize, dist_extra: u32) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.bit(1);
    w.bits(1, 2);
    let lit_lens = fixed_lit_lens();
    let lit_codes = canonical_codes(&lit_lens);
    let dist_lens = vec![5u8; 32];
    let dist_codes = canonical_codes(&dist_lens);
    let s = 257 + len_sym_idx;
    w.code(lit_codes[s], lit_lens[s] as usize);
    if len_sym_idx < 29 {
        w.bits(len_extra, LEN_EXTRA[len_sym_idx] as usize);
    }
    w.code(dist_codes[dist_sym_idx], dist_lens[dist_sym_idx] as usize);
    if dist_sym_idx < 30 {
        w.bits(dist_extra, DIST_EXTRA[dist_sym_idx] as usize);
    }
    w.code(lit_codes[256], lit_lens[256] as usize);
    w.align();
    // plenty of trailing input so the reader never runs dry before the check
    w.bytes.extend_from_slice(&[0u8; 32]);
    w.bytes
}

// ===========================================================================
// A. Explicit rejections
// ===========================================================================

#[test]
fn e1_stored_len_nlen_not_complementary() {
    // lib.c:176 -- LEN != (uint16_t)~NLEN
    let payload = b"hello";
    for bad in [0u16, 1, 0xFFFF, 5, 0xFFFB ^ 1] {
        let stream = stored_raw(5, bad, payload, true);
        let good = 5u16 == !bad;
        if good {
            continue;
        }
        assert_error_row(
            Case::new(&format!("E1 LEN=5 NLEN={bad:#06x}"), &stream, 64),
            0,
            Some(ERR_LEN_NLEN),
        );
    }
    // and randomized: any non-complementary pair must be rejected identically
    let mut rng = Rng::new(0xE1);
    let mut cases = Vec::new();
    for i in 0..64 {
        let len = rng.next_u64() as u16;
        let mut nlen = rng.next_u64() as u16;
        if len == !nlen {
            nlen ^= 1;
        }
        let n = rng.range(0, 24);
        let payload = rng.bytes(n);
        let stream = stored_raw(len, nlen, &payload, true);
        cases.push(Case::new(&format!("E1 rand{i} LEN={len} NLEN={nlen}"), &stream, 64));
    }
    assert_batch_matches(&cases);
}

#[test]
fn e2_stored_block_extends_beyond_input() {
    // lib.c:185 -- !(s->bits_left / 8 <= (int)LEN); reached whenever *more*
    // input remains than LEN says, e.g. a stored block that is not last.
    let payload = b"abcdefghijklmnop";
    // LEN = 1 but 16 payload bytes follow
    let stream = stored_raw(1, !1u16, payload, true);
    assert_error_row(
        Case::new("E2 LEN=1 with 16 bytes of input", &stream, 64),
        0,
        Some(ERR_STORED_BEYOND),
    );
    // a non-final stored block always trips it: another block still follows
    let mut w = BitWriter::new();
    stored_block(&mut w, b"first", false);
    stored_block(&mut w, b"second", true);
    assert_error_row(
        Case::new("E2 non-final stored block", &w.bytes, 64),
        0,
        Some(ERR_STORED_BEYOND),
    );
    let mut rng = Rng::new(0xE2);
    let mut cases = Vec::new();
    for i in 0..48 {
        let n = rng.range(8, 60);
        let payload = rng.bytes(n);
        let len = rng.range(0, n.saturating_sub(1)) as u16;
        let stream = stored_raw(len, !len, &payload, true);
        cases.push(Case::new(&format!("E2 rand{i} LEN={len} of {n}"), &stream, 4096));
    }
    assert_batch_matches(&cases);
}

#[test]
fn e3_out_buffer_full_on_literal() {
    // lib.c:260 -- !(s->out + 1 <= s->out_end)
    let items: Vec<Item> = (0..6).map(|i| Item::Lit(b'a' + i)).collect();
    let stream = fixed_stream(&items);
    assert_error_row(
        Case::new("E3 out_bytes=0", &stream, 0),
        0,
        Some(ERR_OUT_SYMBOL),
    );
    // and one byte short of what the block produces
    for short in 1..6i32 {
        assert_error_row(
            Case::new(&format!("E3 out_bytes={short} need 6"), &stream, short),
            0,
            Some(ERR_OUT_SYMBOL),
        );
    }
    // out_bytes < 0 -> out_end < out, so the very first literal is rejected
    assert_error_row(
        Case::new("E3 out_bytes=-1", &stream, -1),
        0,
        Some(ERR_OUT_SYMBOL),
    );
    // NULL output pointer with out_bytes == 0: the bound check fires before any
    // dereference (ERRORS.md E19)
    assert_error_row(
        Case::new("E3/E19 out=NULL out_bytes=0", &stream, 0).null_out(),
        0,
        Some(ERR_OUT_SYMBOL),
    );
}

#[test]
fn e4_backwards_distance_before_buffer_start() {
    // lib.c:279 -- !(s->out - backwards_distance >= s->begin)
    // A match as the very first item: nothing has been written yet.
    for (dsym, dextra) in [(0usize, 0u32), (1, 0), (3, 0), (5, 1), (9, 7)] {
        let stream = fixed_raw_match(0, 0, dsym, dextra);
        assert_error_row(
            Case::new(&format!("E4 first item is a match, distsym={dsym}"), &stream, 4096),
            0,
            Some(ERR_BACK_DIST),
        );
    }
    // a match whose distance exceeds everything produced so far
    let mut w = BitWriter::new();
    w.bit(1);
    w.bits(1, 2);
    let lit_lens = fixed_lit_lens();
    let lit_codes = canonical_codes(&lit_lens);
    let dist_lens = vec![5u8; 32];
    let dist_codes = canonical_codes(&dist_lens);
    for b in b"abcd" {
        w.code(lit_codes[*b as usize], lit_lens[*b as usize] as usize);
    }
    w.code(lit_codes[257 + 0], lit_lens[257] as usize); // length 3
    let ds = dist_sym(100);
    w.code(dist_codes[ds], dist_lens[ds] as usize);
    w.bits(100 - DIST_BASE[ds], DIST_EXTRA[ds] as usize);
    w.code(lit_codes[256], lit_lens[256] as usize);
    w.align();
    w.bytes.extend_from_slice(&[0u8; 32]);
    assert_error_row(
        Case::new("E4 dist=100 after 4 bytes", &w.bytes, 4096),
        0,
        Some(ERR_BACK_DIST),
    );
}

#[test]
fn e5_match_overruns_out_buffer() {
    // lib.c:288 -- !(s->out + length <= s->out_end)
    let mut items: Vec<Item> = (0..8).map(|i| Item::Lit(b'A' + i)).collect();
    items.push(Item::Match { len: 100, dist: 4 });
    let stream = fixed_stream(&items);
    // room for the 8 literals but not the 100-byte copy
    for out_size in [8i32, 9, 50, 107] {
        assert_error_row(
            Case::new(&format!("E5 out_bytes={out_size} need 108"), &stream, out_size),
            0,
            Some(ERR_OUT_STRING),
        );
    }
    // exactly enough must succeed -- proves the boundary is at the right place
    let cases = [Case::new("E5 out_bytes=108 exact", &stream, 108)];
    assert_batch_matches(&cases);
    let r = run_batch(c_so(), &cases);
    match &r[0] {
        Outcome::Ret { ret, .. } => assert_eq!(*ret, 1, "exact fit should succeed"),
        o => panic!("unexpected {o:?}"),
    }
    // dist == 1 takes the memset arm and must be bounded the same way
    let mut items2 = vec![Item::Lit(b'z')];
    items2.push(Item::Match { len: 200, dist: 1 });
    let stream2 = fixed_stream(&items2);
    for out_size in [1i32, 2, 100, 200] {
        assert_error_row(
            Case::new(&format!("E5 memset arm out_bytes={out_size}"), &stream2, out_size),
            0,
            Some(ERR_OUT_STRING),
        );
    }
}

#[test]
fn e6_unknown_block_type() {
    // lib.c:362 -- btype == 3
    for bfinal in [0u32, 1] {
        let mut w = BitWriter::new();
        w.bit(bfinal);
        w.bits(3, 2);
        w.align();
        w.bytes.extend_from_slice(&[0u8; 8]);
        assert_error_row(
            Case::new(&format!("E6 btype=3 bfinal={bfinal}"), &w.bytes, 64),
            0,
            Some(ERR_UNKNOWN_BLOCK),
        );
    }
    // btype == 3 after a valid block
    let mut w = BitWriter::new();
    fixed_block(&mut w, &[Item::Lit(b'x')], false);
    w.bit(1);
    w.bits(3, 2);
    w.align();
    w.bytes.extend_from_slice(&[0u8; 8]);
    assert_error_row(
        Case::new("E6 btype=3 as second block", &w.bytes, 64),
        0,
        Some(ERR_UNKNOWN_BLOCK),
    );

    // The block type is the C's only enum-like field; every one of its four
    // values is covered here and in Phase B, including the out-of-range one.
    let mut cases = Vec::new();
    for btype in 0..4u32 {
        let mut w = BitWriter::new();
        w.bit(1);
        w.bits(btype, 2);
        w.align();
        w.bytes.extend_from_slice(&[0xFFu8; 16]);
        cases.push(Case::new(&format!("E6 btype={btype} sweep"), &w.bytes, 64));
    }
    assert_batch_matches(&cases);
}

// ===========================================================================
// B. Aborting assert()s
// ===========================================================================

#[test]
fn e7_cp_ptr_not_byte_aligned() {
    // lib.c:95 -- assert(!(s->bits_left & 7))
    //
    // Constructed (see verify/construct_e7.py): a btype==1 block whose
    // `cp_peak_bits` takes the "final word" branch at a bit position that is not
    // byte aligned, which breaks the reader's `count == -consumed (mod 8)`
    // invariant, followed by a btype==0 block whose LEN/NLEN happen to be
    // complements so `cp_ptr` is actually reached.
    //   bits: bfinal=0 btype=1 | EOB(7 zero bits) | bfinal=1 btype=0
    //         | LEN = 0xFFF8 | NLEN = 0x0007
    let stream = [0x02u8, 0x04, 0xFF, 0xFF];
    assert_assert_row(
        Case::new("E7 cp_ptr misaligned", &stream, 4096).in_off(2),
        95,
        "cp_ptr",
        "!(s->bits_left & 7)",
    );
}

#[test]
fn e8_peak_bits_word_index_invariant() {
    // lib.c:104 -- assert(s->word_index <= s->word_count)
    //
    // Provably unreachable: the assert sits inside `if (s->word_index <
    // s->word_count)`, immediately after a single `++`, so `word_index <=
    // word_count` always holds. Reproduced in the Rust port for completeness.
    // What *is* testable is the branch that guards it, so this row hammers the
    // word-refill path across every input length mod 4 and every alignment.
    let mut rng = Rng::new(0xE8);
    let mut cases = Vec::new();
    for n in 1..=64usize {
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.byte())).collect();
        let stream = fixed_stream(&items);
        for in_off in 0..4 {
            cases.push(
                Case::new(
                    &format!("E8 refill n={n} len={} in_off={in_off}", stream.len()),
                    &stream,
                    n as i32,
                )
                .in_off(in_off),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e9_consume_bits_not_enough_buffered() {
    // lib.c:115 -- assert(s->count >= num_bits_to_read)
    for (label, data, out, io, oo) in [
        ("E9 truncated btype=1", vec![0x03u8], 64usize, 0usize, 0usize),
        ("E9 truncated btype=2", vec![0x05u8, 0x00], 64, 0, 0),
        ("E9 0xecff", vec![0xECu8, 0xFF], 64, 0, 0),
        (
            "E9 fuzz witness",
            hex_to_vec("fdff9511a0d9d5df99226100"),
            16,
            3,
            1,
        ),
    ] {
        assert_assert_row(
            Case::new(label, &data, out as i32).in_off(io).out_off(oo),
            115,
            "cp_consume_bits",
            "s->count >= num_bits_to_read",
        );
    }
}

#[test]
fn e10_read_bits_num_bits_over_32() {
    // lib.c:123 -- assert(num_bits_to_read <= 32)
    //
    // `num_bits_to_read` only ever comes from a constant, from `s->count & 7`,
    // or from `cp_len_extra_bits[]` / `cp_dist_extra_bits[]`. Those two tables
    // are *exported and writable*, so a caller can legitimately put a value
    // above 32 in them -- that is the trigger.
    let mut items: Vec<Item> = (0..8).map(|i| Item::Lit(b'a' + i)).collect();
    items.push(Item::Match { len: 3, dist: 4 }); // length symbol 257, index 0
    let stream = fixed_stream(&items);

    for bad in [33u8, 40, 64, 255] {
        let mut le = vec![0u8; 31];
        le.copy_from_slice(&LEN_EXTRA_31);
        le[0] = bad;
        assert_assert_row(
            Case::new(
                &format!("E10 cp_len_extra_bits[0]={bad}"),
                &stream,
                4096,
            )
            .table("le", le),
            123,
            "cp_read_bits",
            "num_bits_to_read <= 32",
        );
    }
    // and through the distance table
    let mut items2: Vec<Item> = (0..8).map(|i| Item::Lit(b'a' + i)).collect();
    items2.push(Item::Match { len: 5, dist: 5 }); // distance symbol 4
    let stream2 = fixed_stream(&items2);
    for bad in [33u8, 100] {
        let mut de = DIST_EXTRA_32.to_vec();
        de[4] = bad;
        assert_assert_row(
            Case::new(
                &format!("E10 cp_dist_extra_bits[4]={bad}"),
                &stream2,
                4096,
            )
            .table("de", de),
            123,
            "cp_read_bits",
            "num_bits_to_read <= 32",
        );
    }
}

#[test]
fn e11_read_bits_num_bits_negative() {
    // lib.c:124 -- assert(num_bits_to_read >= 0)
    //
    // Provably unreachable: every argument `cp_read_bits` is ever called with is
    // either a non-negative literal, `s->count & 7` (a bitwise AND with 7, so
    // 0..7 even for a negative `count` in two's complement), or a `uint8_t`
    // table entry promoted to `int` (0..255). None can be negative.
    // The closest reachable state is a *large* table entry, which trips E10
    // instead -- asserted here so the ordering of the two checks is pinned down.
    let mut items: Vec<Item> = (0..4).map(|i| Item::Lit(b'a' + i)).collect();
    items.push(Item::Match { len: 3, dist: 2 });
    let stream = fixed_stream(&items);
    let mut le = LEN_EXTRA_31.to_vec();
    le[0] = 0xFF; // 255 as an int: > 32, never < 0
    assert_assert_row(
        Case::new("E11 table entry 0xFF hits the <=32 check, not the >=0 one", &stream, 4096)
            .table("le", le),
        123,
        "cp_read_bits",
        "num_bits_to_read <= 32",
    );
    // a negative `count` still yields a non-negative `count & 7`
    let mut cases = Vec::new();
    for io in 0..4 {
        for n in [1usize, 2, 3, 4, 5] {
            let d = vec![0x00u8; n]; // btype=0 -> cp_read_bits(s, s->count & 7)
            cases.push(Case::new(&format!("E11 count&7 io={io} n={n}"), &d, 64).in_off(io));
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e12_read_bits_input_exhausted() {
    // lib.c:125 -- assert(s->bits_left > 0)
    for (label, data, io) in [
        ("E12 empty input", vec![], 0usize),
        ("E12 empty input off1", vec![], 1),
        ("E12 empty input off2", vec![], 2),
        ("E12 empty input off3", vec![], 3),
        ("E12 single 0x05", vec![0x05u8], 0),
        ("E12 single 0x00", vec![0x00u8], 0),
        ("E12 single 0x0d", vec![0x0Du8], 0),
        ("E12 single 0xed", vec![0xEDu8], 0),
        ("E12 single 0xcb off1", vec![0xCBu8], 1),
    ] {
        assert_assert_row(
            Case::new(label, &data, 4096).in_off(io),
            125,
            "cp_read_bits",
            "s->bits_left > 0",
        );
    }
    // The exact boundary: `bits_left == 0` (not merely negative) at a
    // `cp_read_bits` entry. A non-final stored block with LEN == 0 and
    // in_bytes == 5 consumes precisely all 40 bits, then loops for another
    // block header. Verified against the instrumented C build:
    //   READ n=1 cnt=8 bl=0 ...
    assert_assert_row(
        Case::new(
            "E12 bits_left == 0 exactly",
            &[0x00u8, 0x00, 0x00, 0xFF, 0xFF],
            64,
        ),
        125,
        "cp_read_bits",
        "s->bits_left > 0",
    );
    // and the same shape at other input lengths, so `> 0` vs `>= 0` is pinned
    let mut boundary = Vec::new();
    for extra in 0..6usize {
        let mut d = vec![0x00u8, 0x00, 0x00, 0xFF, 0xFF];
        d.extend(std::iter::repeat(0u8).take(extra));
        for io in 0..4usize {
            boundary.push(
                Case::new(&format!("E12 boundary extra={extra} io={io}"), &d, 64).in_off(io),
            );
        }
    }
    assert_batch_matches(&boundary);

    // in_bytes == 0 with a non-empty buffer, and in_bytes < 0
    let stream = fixed_stream(&[Item::Lit(b'x')]);
    assert_assert_row(
        Case::new("E12/E20 in_bytes=0", &stream, 4096).in_len(0),
        125,
        "cp_read_bits",
        "s->bits_left > 0",
    );
    for neg in [-1i32, -8, -1000, i32::MIN] {
        assert_assert_row(
            Case::new(&format!("E12/E20 in_bytes={neg}"), &stream, 4096).in_len(neg),
            125,
            "cp_read_bits",
            "s->bits_left > 0",
        );
    }
}

#[test]
fn e13_read_bits_count_over_64() {
    // lib.c:126 -- assert(s->count <= 64)
    //
    // Provably unreachable. `count` only grows by 32 (a whole word) or, once,
    // by `bits_left` in `cp_peak_bits`' final-word branch. A refill only happens
    // when `count < num_bits_to_read`, and at the final-word branch
    // `bits_left == last_bytes * 8 + count` with `last_bytes <= 3`, so
    // afterwards `count == 2 * count_before + last_bytes * 8`. With
    // `count_before <= num_bits_to_read - 1 <= 31` (32 is the largest
    // `num_bits_to_read` that survives E10) the next `cp_read_bits` entry sees
    // at most `2 * 31 + 24 - 32 == 54`.
    //
    // This row drives `count` as high as it can go -- `num_bits_to_read == 32`
    // via the writable extra-bits table, with `last_bytes == 3` -- and requires
    // C and Rust to agree.
    let mut items: Vec<Item> = (0..4).map(|i| Item::Lit(b'a' + i)).collect();
    items.push(Item::Match { len: 3, dist: 2 });
    let base = fixed_stream(&items);
    let mut cases = Vec::new();
    for extra in 0..8usize {
        let mut stream = base.clone();
        stream.extend(std::iter::repeat(0u8).take(extra));
        for io in 0..4 {
            let mut le = LEN_EXTRA_31.to_vec();
            le[0] = 32; // the maximum that passes E10
            cases.push(
                Case::new(
                    &format!("E13 num_bits=32 pad={extra} io={io}"),
                    &stream,
                    4096,
                )
                .in_off(io)
                .table("le", le),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e14_read_bits_would_overflow() {
    // lib.c:127 -- assert(!cp_would_overflow(s, num_bits_to_read))
    for (label, data, out, io) in [
        ("E14 fuzz witness", hex_to_vec("02003ea80a"), 64usize, 0usize),
        ("E14 witness 4dd13f", hex_to_vec("4dd13f"), 4096, 0),
        ("E14 witness 259305", hex_to_vec("259305"), 4096, 0),
        ("E14 witness 2c10c2 io1", hex_to_vec("2c10c2"), 4096, 1),
        ("E14 witness 59d01b6c io1", hex_to_vec("59d01b6c"), 4096, 1),
        ("E14 witness 14592f io3", hex_to_vec("14592f"), 4096, 3),
        ("E14 witness b51421", hex_to_vec("b51421"), 4096, 0),
    ] {
        assert_assert_row(
            Case::new(label, &data, out as i32).in_off(io),
            127,
            "cp_read_bits",
            "!cp_would_overflow(s, num_bits_to_read)",
        );
    }
    // Neighbouring truncations trip *different* asserts; pinning them down
    // proves the ordering of the five checks in cp_read_bits is preserved.
    assert_assert_row(
        Case::new("E14 neighbour 0105 -> E9", &[0x01u8, 0x05], 64),
        115,
        "cp_consume_bits",
        "s->count >= num_bits_to_read",
    );
    assert_assert_row(
        Case::new("E14 neighbour 010500 -> E12", &[0x01u8, 0x05, 0x00], 64),
        125,
        "cp_read_bits",
        "s->bits_left > 0",
    );
}

#[test]
fn e15_cp_build_code_length_over_15() {
    // lib.c:154 -- assert(len < 16)
    //
    // Constructed: a btype==1 block first (so `cp_build(0, s->dst, ...)` leaves
    // `s->dst[31] == 0xF80001F5`), then a btype==2 block whose 19 code-length
    // code lengths are all zero. `cp_build` then returns 0, so `cp_decode`
    // searches an empty tree, reads `tree[-1]` -- which *is* `s->dst[31]` --
    // and decodes symbol 31. `cp_dynamic`'s `default:` arm stores 31 into
    // `lens[]`, and the next `cp_build` trips the assert. The all-ones tail is
    // required so `cp_decode`'s own prefix assert (E16) passes each time.
    let mut bits: Vec<u32> = Vec::new();
    bits.extend([0, 1, 0]); // bfinal=0, btype=1
    bits.extend([0; 7]); // end-of-block (fixed code 0, 7 bits)
    bits.extend([1, 0, 1]); // bfinal=1, btype=2
    bits.extend([0; 5]); // nlit = 257
    bits.extend([0; 5]); // ndst = 1
    bits.extend([0; 4]); // nlen = 4
    bits.extend([0; 12]); // four 3-bit code lengths, all zero
    assert_eq!(bits.len(), 39);
    let total_bits = 180 * 8;
    while bits.len() < total_bits {
        bits.push(1);
    }
    let mut w = BitWriter::new();
    for b in bits {
        w.bit(b);
    }
    assert_eq!(w.bytes.len(), 180);
    assert_eq!(&w.bytes[..5], &[0x02, 0x14, 0x00, 0x00, 0x80]);
    assert_assert_row(
        Case::new("E15 code length 31 in lens[]", &w.bytes, 4096),
        154,
        "cp_build",
        "len < 16",
    );
}

#[test]
fn e16_cp_decode_prefix_mismatch() {
    // lib.c:217 -- assert((search >> len) == (key >> len))
    for (label, data, out, io, oo) in [
        (
            "E16 fuzz witness",
            hex_to_vec("fdff09d1f5cf3c42ee743b4dc7324663"),
            16usize,
            2usize,
            1usize,
        ),
        (
            "E16 incomplete dynamic tree",
            hex_to_vec("fdff4b9926b298eb8ef38cba51eea435847e9201063dd22640b54ded01"),
            4096,
            0,
            0,
        ),
    ] {
        let case = Case::new(label, &data, out as i32).in_off(io).out_off(oo);
        // both libraries must react identically; the first is a known SIGABRT
        assert_batch_matches(std::slice::from_ref(&case));
    }
    assert_assert_row(
        Case::new(
            "E16 fuzz witness",
            &hex_to_vec("fdff09d1f5cf3c42ee743b4dc7324663"),
            16,
        )
        .in_off(2)
        .out_off(1),
        217,
        "cp_decode",
        "(search >> len) == (key >> len)",
    );

    // An *incomplete* Huffman code is the general trigger: build a literal tree
    // over 3 symbols with lengths 1, 2, 3 (Kraft sum 7/8) and then feed the one
    // bit pattern that no code covers.
    let mut lit = vec![0u8; 288];
    lit[b'a' as usize] = 1;
    lit[b'b' as usize] = 2;
    lit[256] = 3;
    assert_eq!(kraft(&lit), (1 << 15) - (1 << 12), "expected an incomplete code");
    let mut dist = vec![0u8; 32];
    dist[0] = 1;
    let mut w = BitWriter::new();
    let spec = DynSpec::new(lit.clone(), dist);
    dynamic_block(&mut w, &spec, &[Item::Lit(b'a')], true);
    // overwrite the tail with all-ones so the unmatched prefix 111 is read
    w.align();
    let hdr = w.bytes.len();
    w.bytes.extend_from_slice(&[0xFFu8; 32]);
    let case = Case::new(
        &format!("E16 incomplete code, {hdr}-byte header"),
        &w.bytes,
        4096,
    );
    assert_batch_matches(std::slice::from_ref(&case));
}

// ===========================================================================
// C. Boundary / degenerate inputs that are not rejected
// ===========================================================================

#[test]
fn e17_e18_null_pointers() {
    // E18: in == NULL. `(size_t)NULL + 3 & ~3` gives first_bytes == 0 and
    // s->words == NULL; with in_bytes == 0 the E12 assert fires first, with
    // in_bytes > 0 the reader dereferences NULL.
    assert_assert_row(
        Case::new("E18 in=NULL in_bytes=0", &[], 4096).null_in().in_len(0),
        125,
        "cp_read_bits",
        "s->bits_left > 0",
    );
    for n in [1i32, 4, 8] {
        let case = Case::new(&format!("E18 in=NULL in_bytes={n}"), &[], 4096)
            .null_in()
            .in_len(n);
        assert_batch_matches(std::slice::from_ref(&case));
        let r = run_batch(c_so(), std::slice::from_ref(&case));
        match &r[0] {
            Outcome::Signal { sig, .. } => assert!(
                *sig == 11 || *sig == 6,
                "E18 in=NULL in_bytes={n}: expected SIGSEGV or SIGABRT, got {sig}"
            ),
            o => panic!("E18 in=NULL in_bytes={n}: expected a fatal signal, got {o:?}"),
        }
    }
    // E17 (calloc failure) cannot be provoked portably; both implementations
    // dereference the result unchecked, so they fail the same way. Documented
    // in ERRORS.md.
}

#[test]
fn e19_e22_out_pointer_and_size_boundaries() {
    let stream = fixed_stream(&[Item::Lit(b'q'), Item::Lit(b'r')]);
    let mut cases = vec![
        Case::new("E19 out=NULL out_bytes=0", &stream, 0).null_out(),
        Case::new("E22 out_bytes=-1", &stream, -1),
        Case::new("E22 out_bytes=i32::MIN", &stream, i32::MIN),
        Case::new("E22 out_bytes=1 need 2", &stream, 1),
        Case::new("E22 out_bytes=2 exact", &stream, 2),
        Case::new("E22 out_bytes=3", &stream, 3),
    ];
    // NULL out with out_bytes < 0: out_end < out, so the check still fires
    cases.push(Case::new("E19 out=NULL out_bytes=-1", &stream, -1).null_out());
    assert_batch_matches(&cases);
    // pin the error codes down for the well-defined ones
    assert_error_row(
        Case::new("E19 out=NULL out_bytes=0", &stream, 0).null_out(),
        0,
        Some(ERR_OUT_SYMBOL),
    );
    assert_error_row(
        Case::new("E22 out_bytes=i32::MIN", &stream, i32::MIN),
        0,
        Some(ERR_OUT_SYMBOL),
    );
}

#[test]
fn e20_e21_in_bytes_boundaries() {
    // E21: in_bytes < first_bytes -- word_count goes negative, last_bytes is
    // `(negative & 3)`, and the pre-load loop reads past in_bytes. Both
    // libraries must read the same padding bytes, which the harness guarantees.
    let mut cases = Vec::new();
    for in_off in 1..4usize {
        for n in 0..6i32 {
            let data = vec![0x01u8, 0x02, 0x00, 0xFD, 0xFF, 0x41, 0x42, 0x43];
            cases.push(
                Case::new(&format!("E21 in_off={in_off} in_bytes={n}"), &data, 64)
                    .in_off(in_off)
                    .in_len(n),
            );
        }
    }
    // E20: negative in_bytes at every alignment
    for in_off in 0..4usize {
        for n in [-1i32, -3, -7, -32] {
            let data = vec![0x01u8, 0x05, 0x00, 0xFA, 0xFF, b'h', b'e', b'l', b'l', b'o'];
            cases.push(
                Case::new(&format!("E20 in_off={in_off} in_bytes={n}"), &data, 64)
                    .in_off(in_off)
                    .in_len(n),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e23_stored_len_exceeds_remaining_input() {
    // lib.c:193 -- memcpy reads past the end of the input buffer, because E2's
    // check is inverted and cannot stop it. The harness pads the input with a
    // fixed pattern so both libraries read the same bytes.
    let mut cases = Vec::new();
    for len in [8u16, 64, 300, 4096, 0xFFFF] {
        for io in 0..4usize {
            let stream = stored_raw(len, !len, b"ab", true);
            cases.push(
                Case::new(&format!("E23 LEN={len} with 2 payload bytes io={io}"), &stream, 0x20000)
                    .in_off(io)
                    .in_pad(0x20000)
                    .out_pad(4096),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e24_stored_len_exceeds_out_bytes() {
    // lib.c:193 -- there is no output bound check in cp_stored at all, so the
    // memcpy writes past out_bytes. The output buffer is over-allocated so the
    // overrun is observable (and compared) instead of fatal.
    let mut cases = Vec::new();
    for (len, out_size) in [(64u16, 1i32), (64, 0), (256, 16), (1024, 100), (4096, 1)] {
        let payload: Vec<u8> = (0..len as usize).map(|i| (i % 251) as u8).collect();
        let stream = stored_raw(len, !len, &payload, true);
        cases.push(
            Case::new(
                &format!("E24 LEN={len} out_bytes={out_size}"),
                &stream,
                out_size,
            )
            .out_pad(0x10000),
        );
    }
    assert_batch_matches(&cases);
}

#[test]
fn e25_zero_length_match_makes_no_progress() {
    // lib.c:272 -- cp_len_base[29] == cp_len_base[30] == 0 with 0 extra bits, so
    // length symbols 286 and 287 produce `length == 0`: `s->out` does not
    // advance, `memset(dst, *src, 0)` copies nothing, and **no check fails** --
    // the C accepts it. It is not a hang, because the symbol still consumes
    // bits; both libraries must return 1 with identical output.
    for lsym in [29usize, 30] {
        let mut w = BitWriter::new();
        w.bit(1);
        w.bits(1, 2);
        let lit_lens = fixed_lit_lens();
        let lit_codes = canonical_codes(&lit_lens);
        let dist_lens = vec![5u8; 32];
        let dist_codes = canonical_codes(&dist_lens);
        for b in b"abcdefgh" {
            w.code(lit_codes[*b as usize], lit_lens[*b as usize] as usize);
        }
        let s = 257 + lsym;
        w.code(lit_codes[s], lit_lens[s] as usize); // no extra bits
        w.code(dist_codes[0], dist_lens[0] as usize); // distance 1
        w.code(lit_codes[256], lit_lens[256] as usize);
        w.align();
        w.bytes.extend_from_slice(&[0u8; 64]);
        let case = Case::new(&format!("E25 length symbol {s} -> length 0"), &w.bytes, 4096);
        assert_error_row(case, 1, None);
        // and at every alignment / output size
        let mut cases = Vec::new();
        for io in 0..4usize {
            for out in [8i32, 9, 4096] {
                cases.push(
                    Case::new(&format!("E25 lensym={s} io={io} out={out}"), &w.bytes, out)
                        .in_off(io),
                );
            }
        }
        assert_batch_matches(&cases);
    }
}

#[test]
fn e26_e27_table_index_boundaries() {
    // The last in-range entries of each table: cp_len_extra_bits[30] /
    // cp_len_base[30] (length symbol 287) and cp_dist_extra_bits[31] /
    // cp_dist_base[31] (distance symbol 31). Distance symbols 30 and 31 have
    // base 0, so `backwards_distance == 0`: `out - 0 >= begin` passes and the
    // copy is a self-copy.
    let mut cases = Vec::new();
    for dsym in [29usize, 30, 31] {
        for lsym in [28usize, 29, 30] {
            let mut w = BitWriter::new();
            w.bit(1);
            w.bits(1, 2);
            let lit_lens = fixed_lit_lens();
            let lit_codes = canonical_codes(&lit_lens);
            let dist_lens = vec![5u8; 32];
            let dist_codes = canonical_codes(&dist_lens);
            for b in b"0123456789" {
                w.code(lit_codes[*b as usize], lit_lens[*b as usize] as usize);
            }
            let s = 257 + lsym;
            w.code(lit_codes[s], lit_lens[s] as usize);
            if lsym < 29 {
                w.bits(0, LEN_EXTRA[lsym] as usize);
            }
            w.code(dist_codes[dsym], dist_lens[dsym] as usize);
            if dsym < 30 {
                w.bits(0, DIST_EXTRA[dsym] as usize);
            }
            w.code(lit_codes[256], lit_lens[256] as usize);
            w.align();
            w.bytes.extend_from_slice(&[0u8; 64]);
            cases.push(Case::new(
                &format!("E26/E27 lensym={} distsym={dsym}", 257 + lsym),
                &w.bytes,
                4096,
            ));
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e28_e29_empty_tree_and_lens_minus_one() {
    // E28: cp_decode with hi == 0 reads tree[-1]; #[repr(C)] makes that the same
    //      field in both libraries.
    // E29: cp_dynamic's `case 16` with n == 0 reads lens[-1], which gcc places
    //      on the most significant byte of the spilled `s` pointer (always 0 on
    //      x86-64). The Rust port models the frame, so it reads 0 too.
    //
    // One stream covers both: a btype==2 header whose code-length alphabet is
    // {0, 16}, with symbol 16 emitted first (n == 0), which fills lens[] with
    // lens[-1] and then zeros -- leaving an empty literal tree for cp_block.
    let mut w = BitWriter::new();
    w.bit(1); // bfinal
    w.bits(2, 2); // btype = 2
    w.bits(0, 5); // nlit = 257
    w.bits(0, 5); // ndst = 1
    w.bits(0, 4); // nlen = 4  -> PERM[0..4] = 16, 17, 18, 0
    w.bits(1, 3); // len(sym 16) = 1
    w.bits(0, 3); // len(sym 17) = 0
    w.bits(0, 3); // len(sym 18) = 0
    w.bits(1, 3); // len(sym 0)  = 1
    // canonical codes over {0:1, 16:1}: symbol 0 -> 0, symbol 16 -> 1
    w.code(1, 1); // symbol 16
    w.bits(0, 2); // repeat 3 times  -> lens[0..3] = lens[-1]
    for _ in 0..(257 + 1 - 3) {
        w.code(0, 1); // symbol 0
    }
    w.align();
    w.bytes.extend_from_slice(&[0u8; 64]);
    let case = Case::new("E28/E29 empty tree + lens[-1]", &w.bytes, 4096);
    assert_batch_matches(std::slice::from_ref(&case));

    // and the same shape at every input alignment
    let mut cases = Vec::new();
    for io in 0..4usize {
        for out in [0i32, 1, 64, 4096] {
            cases.push(
                Case::new(&format!("E28/E29 io={io} out={out}"), &w.bytes, out).in_off(io),
            );
        }
    }
    assert_batch_matches(&cases);
}

#[test]
fn e30_lens_overrun_wedges_the_loop() {
    // lib.c:231 -- a run-length code that pushes `n` past `lens[288 + 32]`
    // overwrites cp_dynamic's own locals (`nlit`, `ndst`, the run counter and
    // `n`), which makes the C library loop forever. The Rust port models the
    // gcc -O0 frame layout, so it wedges identically (SIGALRM from both).
    assert_signal_row(
        Case::new(
            "E30 lens[] overrun",
            &hex_to_vec(
                "fdff4b9926b298eb8ef38cba51eea435847e9201063dd22640b54ded00",
            ),
            4096,
        )
        .in_off(3),
        14,
    );

    // randomized: maximal nlit/ndst headers followed by random bits, which is
    // the shape that overruns `lens[]`
    let mut rng = Rng::new(0xE30);
    let mut cases = Vec::new();
    for i in 0..96 {
        let mut w = BitWriter::new();
        w.bit(1);
        w.bits(2, 2);
        w.bits(31, 5); // nlit = 288
        w.bits(31, 5); // ndst = 32
        w.bits(15, 4); // nlen = 19
        for _ in 0..19 {
            w.bits(rng.below(8) as u32, 3);
        }
        for _ in 0..rng.range(16, 400) {
            w.bit(rng.below(2) as u32);
        }
        w.align();
        cases.push(
            Case::new(&format!("E30 rand{i}"), &w.bytes, 4096)
                .in_off(rng.below(4))
                .in_pad(1024),
        );
    }
    assert_batch_matches(&cases);
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn hex_to_vec(s: &str) -> Vec<u8> {
    common::shared::unhex(s)
}

/// `cp_len_extra_bits` as shipped (31 entries).
const LEN_EXTRA_31: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
/// `cp_dist_extra_bits` as shipped (32 entries).
const DIST_EXTRA_32: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

/// Sanity: the tables the tests hand back must be exactly what the libraries
/// shipped, otherwise a "table override" row would silently change two things.
#[test]
fn tables_match_the_shipped_values() {
    let c = common::shared::Lib::open(c_so().to_str().unwrap());
    let r = common::shared::Lib::open(rust_so().to_str().unwrap());
    for spec in common::shared::TABLES {
        let cb = c.table_bytes(spec.key);
        let rb = r.table_bytes(spec.key);
        assert_eq!(
            cb,
            rb,
            "{} differs between the libraries:\n  C    = {}\n  Rust = {}",
            spec.symbol,
            hex(&cb),
            hex(&rb)
        );
    }
    assert_eq!(c.table_bytes("le"), LEN_EXTRA_31.to_vec());
    assert_eq!(c.table_bytes("de"), DIST_EXTRA_32.to_vec());
}

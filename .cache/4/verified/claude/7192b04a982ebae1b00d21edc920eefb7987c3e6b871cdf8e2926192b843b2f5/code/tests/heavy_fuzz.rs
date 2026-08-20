//! Cross-cutting differential fuzzing over the FULL input surface at once
//! (multiple independent seeds), so that combinations not hand-enumerated in
//! `CONFIGS.md` / `ERRORS.md` still get compared.

mod common;

use common::*;

const SEEDS: &[u64] = &[
    0x0000_0000_0000_0001,
    0xDEAD_BEEF_1234_5678,
    0x5EED_1234_ABCD_0001,
    0x9E37_79B9_7F4A_7C15,
    0xFFFF_FFFF_FFFF_FFFF,
    0x1357_9BDF_0246_8ACE,
];

/// Alphabet biased toward the interesting bytes but including plain junk.
const ALPHABET: &[u8] = b"0123456789+-.eE0123456789.eE,]}: \t\0\"aAxX/\\[{\x80\xff";

fn one_round(seed: u64, iters: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let n = rng.below(48);
        let content: Vec<u8> = (0..n)
            .map(|_| {
                if rng.below(8) == 0 {
                    rng.byte() // any byte at all
                } else {
                    *rng.pick(ALPHABET)
                }
            })
            .collect();
        let clen = content.len();

        // `length` never exceeds the real allocation, so the C's reads stay in
        // bounds; `offset` is allowed to run past `length`.
        let length = match rng.below(8) {
            0 => 0,
            1 => clen,
            2 => clen,
            _ => rng.below(clen + 1),
        };
        let offset = match rng.below(8) {
            0 => 0,
            1 => length,
            2 => length + 1 + rng.below(4),
            3 => usize::MAX - rng.below(4),
            4 => 1usize << 63,
            _ => rng.below(clen + 1),
        };
        let mut case = Case::new(&content)
            .length(length)
            .offset(offset)
            .depth(rng.next_u64() as usize)
            .item_state(rng.next_u64() as i32, rng.next_u64() as i32, rng.next_u64());

        // Occasionally exercise the NULL axes (only where the C is safe).
        match rng.below(32) {
            0 => case = case.buffer_null(),
            1 => case = case.content_null(),
            2 => case = case.buffer_null().item_null(),
            3 => case = case.content_null().item_null(),
            _ => {}
        }
        assert_same(&case);
    }
}

#[test]
fn fuzz_all_axes_multi_seed() {
    for &s in SEEDS {
        one_round(s, 20_000);
    }
}

/// Fuzz with a HUGE `length` but a guaranteed in-buffer stop byte, so the
/// `offset + index < length` arithmetic is exercised without an OOB read.
#[test]
fn fuzz_huge_length_with_stop_byte() {
    for &s in SEEDS {
        let mut rng = Rng::new(s ^ 0x0000_0000_0000_0BEE);
        for _ in 0..5_000 {
            let n = rng.below(24);
            let mut content: Vec<u8> = (0..n).map(|_| *rng.pick(b"0123456789+-.eE")).collect();
            content.push(*rng.pick(b",]} \0a"));
            content.extend_from_slice(b"junkjunkjunk");
            let stop = n; // index of the stop byte
            let length = *rng.pick(&[
                usize::MAX,
                usize::MAX - 1,
                1usize << 63,
                1usize << 40,
                content.len() + 1_000_000,
            ]);
            let offset = rng.below(stop + 1);
            let case = Case::new(&content).length(length).offset(offset);
            let out = assert_same(&case);
            assert!(out.buf_offset <= stop.max(offset), "{}", case.label());
        }
    }
}

/// Fuzz whole sequences of calls sharing one `parse_buffer`.
#[test]
fn fuzz_sequences() {
    fn skip(buf: &mut ParseBuffer, content: &[u8]) {
        // Always make progress so a sequence cannot livelock.
        if buf.offset < buf.length && buf.offset < content.len() {
            let c = content[buf.offset];
            if !b"0123456789+-.eE".contains(&c) {
                buf.offset += 1;
            }
        }
    }
    for &s in SEEDS {
        let mut rng = Rng::new(s ^ 0x0000_0000_0000_5E17);
        for _ in 0..2_000 {
            let n = rng.below(40);
            let doc: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
            assert_same_sequence(&doc, 12, skip);
        }
    }
}

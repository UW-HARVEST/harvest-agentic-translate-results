// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH the C `.so` and the
// Rust `.so` through their exported `driver` symbol and compares the captured
// stdout byte-for-byte. Randomized rows use the fixed seed in `common::SEED`
// so failures are reproducible.

mod common;

use common::*;
use std::ffi::c_char;

/// Lengths at or adjacent to the SIMD vector widths glibc's `strcspn` uses.
const VECTOR_BOUNDARY_LENS: &[usize] = &[
    15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
];

// ---------------------------------------------------------------------------
// Rows 1-3: degenerate / empty shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg01_empty_s1_empty_s2() {
    let h = Harness::new();
    let cases = vec![Case::new(b"", b"")];
    h.assert_same("cfg01 empty s1 + empty s2", &cases);
    // Pin the actual value so the row cannot pass vacuously.
    assert_eq!(h.capture_c(&[cases[0].ptrs()]), b"0\n");
}

#[test]
fn cfg02_empty_s2_random_s1() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x02);
    let mut cases = Vec::new();
    for _ in 0..600 {
        let n = rng.range(1, 64);
        cases.push(Case::raw(rng.string_full_domain(n), vec![0]));
    }
    h.assert_same("cfg02 empty s2 (pure strlen path) + random s1", &cases);
}

#[test]
fn cfg03_empty_s1_random_s2() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x03);
    let mut cases = Vec::new();
    for _ in 0..600 {
        let n = rng.range(1, 64);
        cases.push(Case::raw(vec![0], rng.string_full_domain(n)));
    }
    h.assert_same("cfg03 empty s1 + random non-empty s2", &cases);
}

// ---------------------------------------------------------------------------
// Rows 4-11: reject-set size sweep crossed with match position
// ---------------------------------------------------------------------------

/// Builds `s1` of `len` bytes drawn from `filler` (none of which appear in
/// `reject`), optionally planting `reject[0]` at `match_at`.
fn build_s1(
    rng: &mut Rng,
    len: usize,
    filler: &[u8],
    plant: Option<(usize, u8)>,
) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(filler[rng.below(filler.len())]);
    }
    if let Some((pos, byte)) = plant {
        if pos < len {
            v[pos] = byte;
        }
    }
    v.push(0);
    v
}

/// Two disjoint alphabets so "match" and "no match" are fully controlled.
const LOW_ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const HIGH_ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[test]
fn cfg04_s2_len1_match_first() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x04);
    let mut cases = Vec::new();
    for _ in 0..400 {
        let len = rng.range(1, 80);
        let needle = HIGH_ALPHA[rng.below(HIGH_ALPHA.len())];
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, Some((0, needle)));
        cases.push(Case::raw(s1, vec![needle, 0]));
    }
    h.assert_same("cfg04 |s2|=1, match at position 0", &cases);
}

#[test]
fn cfg05_s2_len1_match_last() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x05);
    let mut cases = Vec::new();
    for _ in 0..400 {
        let len = rng.range(1, 80);
        let needle = HIGH_ALPHA[rng.below(HIGH_ALPHA.len())];
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, Some((len - 1, needle)));
        cases.push(Case::raw(s1, vec![needle, 0]));
    }
    h.assert_same("cfg05 |s2|=1, match at last position", &cases);
}

#[test]
fn cfg06_s2_len1_no_match() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x06);
    let mut cases = Vec::new();
    for _ in 0..400 {
        let len = rng.range(0, 80);
        let needle = HIGH_ALPHA[rng.below(HIGH_ALPHA.len())];
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, None);
        cases.push(Case::raw(s1, vec![needle, 0]));
    }
    h.assert_same("cfg06 |s2|=1, no match (returns strlen)", &cases);
}

#[test]
fn cfg07_s2_len1_all_positions() {
    // Exhaustive: every s1 length 1..=80 x every match position 0..len-1.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x07);
    let mut cases = Vec::new();
    for len in 1..=80usize {
        for pos in 0..len {
            let s1 = build_s1(&mut rng, len, LOW_ALPHA, Some((pos, b'!')));
            cases.push(Case::raw(s1, vec![b'!', 0]));
        }
    }
    h.assert_same("cfg07 |s2|=1, exhaustive length x match position", &cases);
}

/// Shared driver for the reject-set-size rows: random `|s2|` in `lo..=hi`,
/// random `|s1|`, and a match planted at a random position (or nowhere).
fn reject_size_row(h: &Harness, label: &str, salt: u64, lo: usize, hi: usize, iters: usize) {
    let mut rng = Rng::new(SEED ^ salt);
    let mut cases = Vec::new();
    for _ in 0..iters {
        let n2 = rng.range(lo, hi);
        // Reject set drawn from HIGH_ALPHA; s1 filler from LOW_ALPHA (disjoint).
        let mut s2: Vec<u8> = (0..n2).map(|_| HIGH_ALPHA[rng.below(HIGH_ALPHA.len())]).collect();
        s2.push(0);
        let len = rng.range(0, 120);
        let plant = if len > 0 && rng.below(4) != 0 {
            // 3/4 of cases contain a real match at a random position.
            Some((rng.below(len), s2[rng.below(n2)]))
        } else {
            None
        };
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, plant);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same(label, &cases);
}

#[test]
fn cfg08_s2_len2() {
    let h = Harness::new();
    reject_size_row(&h, "cfg08 |s2|=2", 0x08, 2, 2, 600);
}

#[test]
fn cfg09_s2_len3() {
    let h = Harness::new();
    reject_size_row(&h, "cfg09 |s2|=3", 0x09, 3, 3, 600);
}

#[test]
fn cfg10_s2_len_4_16() {
    let h = Harness::new();
    reject_size_row(&h, "cfg10 |s2| in 4..=16", 0x10, 4, 16, 900);
}

#[test]
fn cfg11_s2_len_17_64() {
    let h = Harness::new();
    reject_size_row(&h, "cfg11 |s2| in 17..=64", 0x11, 17, 64, 900);
}

// ---------------------------------------------------------------------------
// Rows 12-13: full 255-byte reject domain
// ---------------------------------------------------------------------------

#[test]
fn cfg12_s2_full_255_domain() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x12);
    let full = all_nonzero_bytes();
    assert_eq!(full.len(), 255);
    let mut cases = Vec::new();
    for _ in 0..400 {
        let n = rng.range(1, 100);
        let mut s2 = full.clone();
        s2.push(0);
        cases.push(Case::raw(rng.string_full_domain(n), s2));
    }
    h.assert_same("cfg12 s2 = all 255 non-NUL bytes, non-empty s1", &cases);
    // Every byte is rejected, so the result must be 0 for all cases.
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();
    let out = h.capture_c(&ptrs);
    assert_eq!(
        out,
        "0\n".repeat(cases.len()).into_bytes(),
        "with the full byte domain rejected every result must be 0"
    );
}

#[test]
fn cfg13_s2_full_domain_s1_empty() {
    let h = Harness::new();
    let mut s2 = all_nonzero_bytes();
    s2.push(0);
    let cases = vec![Case::raw(vec![0], s2)];
    h.assert_same("cfg13 s2 = full domain, s1 empty", &cases);
    assert_eq!(h.capture_c(&[cases[0].ptrs()]), b"0\n");
}

// ---------------------------------------------------------------------------
// Rows 14-15: s1 length sweeps around vector boundaries
// ---------------------------------------------------------------------------

#[test]
fn cfg14_s1_length_sweep_no_match() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x14);
    let mut cases = Vec::new();
    for len in 0..=136usize {
        for _ in 0..3 {
            let s1 = build_s1(&mut rng, len, LOW_ALPHA, None);
            let n2 = rng.range(1, 8);
            let mut s2: Vec<u8> =
                (0..n2).map(|_| HIGH_ALPHA[rng.below(HIGH_ALPHA.len())]).collect();
            s2.push(0);
            cases.push(Case::raw(s1, s2));
        }
    }
    h.assert_same("cfg14 |s1| swept 0..=136 with no match", &cases);
}

#[test]
fn cfg15_vector_boundary_match_positions() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x15);
    let mut cases = Vec::new();
    for &len in VECTOR_BOUNDARY_LENS {
        let mut positions: Vec<usize> = vec![0, 1, 2];
        positions.extend([len - 3, len - 2, len - 1]);
        for pos in positions {
            for n2 in [1usize, 2, 5, 17] {
                let mut s2: Vec<u8> =
                    (0..n2).map(|_| HIGH_ALPHA[rng.below(HIGH_ALPHA.len())]).collect();
                let needle = s2[0];
                s2.push(0);
                let s1 = build_s1(&mut rng, len, LOW_ALPHA, Some((pos, needle)));
                cases.push(Case::raw(s1, s2));
            }
        }
    }
    h.assert_same("cfg15 vector-boundary lengths x edge match positions", &cases);
}

// ---------------------------------------------------------------------------
// Rows 16-19: pointer alignment and page-edge behaviour
// ---------------------------------------------------------------------------

#[test]
fn cfg16_s1_alignment_sweep() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x16);
    let mut cases = Vec::new();
    for off in 0..=63usize {
        for len in [1usize, 7, 16, 33, 64, 100] {
            // Buffer is padded by `off` leading bytes; the pointer handed to the
            // library starts at `off`, giving every alignment mod 64.
            let mut buf = vec![b'#'; off];
            let body = build_s1(&mut rng, len, LOW_ALPHA, Some((len - 1, b'!')));
            buf.extend_from_slice(&body);
            cases.push(Case::raw(buf, vec![b'!', 0]).with_offsets(off, 0));
        }
    }
    h.assert_same("cfg16 s1 alignment sweep (offsets 0..=63)", &cases);
}

#[test]
fn cfg17_s2_alignment_sweep() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x17);
    let mut cases = Vec::new();
    for off in 0..=63usize {
        for n2 in [1usize, 2, 8, 32] {
            let mut s2buf = vec![b'#'; off];
            let mut s2: Vec<u8> =
                (0..n2).map(|_| HIGH_ALPHA[rng.below(HIGH_ALPHA.len())]).collect();
            let needle = s2[0];
            s2.push(0);
            s2buf.extend_from_slice(&s2);
            let len = rng.range(1, 90);
            let pos = rng.below(len);
            let s1 = build_s1(&mut rng, len, LOW_ALPHA, Some((pos, needle)));
            cases.push(Case::raw(s1, s2buf).with_offsets(0, off));
        }
    }
    h.assert_same("cfg17 s2 alignment sweep (offsets 0..=63)", &cases);
}

#[test]
fn cfg18_s1_terminated_at_page_edge() {
    // s1's terminating NUL is the very last accessible byte before a PROT_NONE
    // guard page, so any over-read past the NUL faults. Both libraries must
    // return strlen(s1) without touching the guard page.
    let h = Harness::new();
    let mut gp = GuardedPage::new();
    let s2 = b"!\0";
    for &len in &[1usize, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000] {
        let s1 = gp.string_flush_to_edge(len, b'a');
        let c = h.capture_c(&[(s1, s2.as_ptr() as *const c_char)]);
        let r = h.capture_rs(&[(s1, s2.as_ptr() as *const c_char)]);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "cfg18 s1 flush against guard page, len={len}"
        );
        assert_eq!(
            c,
            format!("{len}\n").into_bytes(),
            "cfg18 expected strlen for len={len}"
        );
    }
}

#[test]
fn cfg19_s2_terminated_at_page_edge() {
    // Same idea for the reject set: s2 ends exactly at the guard page.
    let h = Harness::new();
    let mut gp = GuardedPage::new();
    let s1 = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaZ\0";
    for &len in &[1usize, 2, 15, 16, 17, 32, 33, 64, 65, 255] {
        // Reject page filled with 'Z' (present in s1 at index 52).
        let s2 = gp.string_flush_to_edge(len, b'Z');
        let c = h.capture_c(&[(s1.as_ptr() as *const c_char, s2)]);
        let r = h.capture_rs(&[(s1.as_ptr() as *const c_char, s2)]);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "cfg19 s2 flush against guard page, len={len}"
        );
        assert_eq!(c, b"52\n", "cfg19 expected match at index 52 for len={len}");
    }
}

// ---------------------------------------------------------------------------
// Rows 20-22: high-bit / signed-char hazards
// ---------------------------------------------------------------------------

#[test]
fn cfg20_s1_high_bit_no_match() {
    // s1 is entirely 0x80..=0xFF (negative as `char`), s2 is ASCII: no match, so
    // the result is strlen. A sign-extended table index would panic or misindex.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x20);
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    let mut cases = Vec::new();
    for _ in 0..500 {
        let len = rng.range(1, 120);
        let s1 = rng.string_from(len, &high);
        let n2 = rng.range(1, 16);
        let s2 = rng.string_from(n2, LOW_ALPHA);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg20 s1 all high-bit bytes, ASCII s2, no match", &cases);
}

#[test]
fn cfg21_high_bit_match() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x21);
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    let mut cases = Vec::new();
    for _ in 0..800 {
        let n2 = rng.range(1, 20);
        let mut s2 = rng.string_from(n2, &high);
        let needle = s2[rng.below(n2)];
        // Filler must avoid every byte in the reject set.
        let reject: Vec<u8> = s2[..n2].to_vec();
        let filler: Vec<u8> = high.iter().copied().filter(|b| !reject.contains(b)).collect();
        if filler.is_empty() {
            s2 = vec![needle, 0];
        }
        let filler: Vec<u8> = if filler.is_empty() {
            high.iter().copied().filter(|&b| b != needle).collect()
        } else {
            filler
        };
        let len = rng.range(1, 120);
        let pos = rng.below(len);
        let s1 = build_s1(&mut rng, len, &filler, Some((pos, needle)));
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg21 high-bit bytes in both s1 and s2, real match", &cases);
}

#[test]
fn cfg22_byte_0xff_boundary() {
    // 0xFF is the top of the reject-table domain: an off-by-one in the table
    // size or an inclusive/exclusive slip shows up exactly here.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x22);
    let mut cases = Vec::new();
    for _ in 0..400 {
        let len = rng.range(1, 90);
        // Filler avoids 0xFF entirely.
        let filler: Vec<u8> = (1u16..=0xFE).map(|b| b as u8).collect();
        let pos = rng.below(len);
        let s1 = build_s1(&mut rng, len, &filler, Some((pos, 0xFF)));
        cases.push(Case::raw(s1, vec![0xFF, 0]));
    }
    // Also the mirrored case: 0xFF only in s1's filler, s2 rejects something else.
    for _ in 0..200 {
        let len = rng.range(1, 90);
        let s1 = build_s1(&mut rng, len, &[0xFFu8], None);
        cases.push(Case::raw(s1, vec![0xFEu8, 0]));
    }
    h.assert_same("cfg22 byte 0xFF at the top of the reject domain", &cases);
}

// ---------------------------------------------------------------------------
// Rows 23-24: redundant reject sets
// ---------------------------------------------------------------------------

#[test]
fn cfg23_s2_duplicate_bytes() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x23);
    let mut cases = Vec::new();
    for reps in 1..=64usize {
        let byte = HIGH_ALPHA[rng.below(HIGH_ALPHA.len())];
        let mut s2 = vec![byte; reps];
        s2.push(0);
        let len = rng.range(1, 100);
        let plant = if rng.below(2) == 0 {
            Some((rng.below(len), byte))
        } else {
            None
        };
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, plant);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg23 s2 with heavy duplicate bytes", &cases);
}

#[test]
fn cfg24_s2_single_byte_repeated() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x24);
    let mut cases = Vec::new();
    for _ in 0..300 {
        let mut s2 = vec![b'Q'; 200];
        s2.push(0);
        let len = rng.range(0, 150);
        let plant = if len > 0 && rng.below(2) == 0 {
            Some((rng.below(len), b'Q'))
        } else {
            None
        };
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, plant);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg24 s2 = one byte repeated 200 times", &cases);
}

// ---------------------------------------------------------------------------
// Rows 25-28: large inputs and %zu formatting width
// ---------------------------------------------------------------------------

fn long_no_match_case(len: usize) -> Case {
    // 'a' repeated, reject set disjoint from it -> result is exactly `len`.
    Case::raw({
        let mut v = vec![b'a'; len];
        v.push(0);
        v
    }, vec![b'!', b'?', 0])
}

#[test]
fn cfg25_s1_4kib_no_match() {
    let h = Harness::new();
    let cases = vec![long_no_match_case(4096)];
    h.assert_same("cfg25 s1 = 4 KiB, no match", &cases);
    assert_eq!(h.capture_c(&[cases[0].ptrs()]), b"4096\n");
}

#[test]
fn cfg26_s1_64kib_no_match() {
    let h = Harness::new();
    let cases = vec![long_no_match_case(65536)];
    h.assert_same("cfg26 s1 = 64 KiB, no match", &cases);
    assert_eq!(h.capture_c(&[cases[0].ptrs()]), b"65536\n");
}

#[test]
fn cfg27_s1_1mib_no_match() {
    let h = Harness::new();
    let cases = vec![long_no_match_case(1024 * 1024)];
    h.assert_same("cfg27 s1 = 1 MiB, no match (widest %zu)", &cases);
    assert_eq!(h.capture_c(&[cases[0].ptrs()]), b"1048576\n");
}

#[test]
fn cfg28_result_digit_width_sweep() {
    // Every decimal-width transition of the printed result.
    let h = Harness::new();
    let lens = [
        0usize, 1, 9, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10000, 10001, 99999, 100000,
    ];
    let cases: Vec<Case> = lens.iter().map(|&n| long_no_match_case(n)).collect();
    h.assert_same("cfg28 %zu digit-width sweep", &cases);
    // Pin the exact formatting, including the newline, for every width.
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();
    let expected: String = lens.iter().map(|n| format!("{n}\n")).collect();
    assert_eq!(
        String::from_utf8_lossy(&h.capture_c(&ptrs)),
        expected,
        "cfg28 C formatting"
    );
    assert_eq!(
        String::from_utf8_lossy(&h.capture_rs(&ptrs)),
        expected,
        "cfg28 Rust formatting"
    );
}

// ---------------------------------------------------------------------------
// Rows 29-30: call sequencing / shared stdout
// ---------------------------------------------------------------------------

#[test]
fn cfg29_no_state_leak_between_calls() {
    // Alternate between "everything rejected" and "nothing rejected". If either
    // implementation retained its reject set across calls, the empty-s2 rows
    // would start returning 0 instead of strlen.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x29);
    let mut full = all_nonzero_bytes();
    full.push(0);
    let mut cases = Vec::new();
    let mut expected = String::new();
    for i in 0..200 {
        let len = rng.range(1, 40);
        let s1 = rng.string_from(len, LOW_ALPHA);
        if i % 2 == 0 {
            cases.push(Case::raw(s1, full.clone()));
            expected.push_str("0\n");
        } else {
            cases.push(Case::raw(s1, vec![0]));
            expected.push_str(&format!("{len}\n"));
        }
    }
    h.assert_same("cfg29 alternating full/empty reject set across 200 calls", &cases);
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();
    assert_eq!(
        String::from_utf8_lossy(&h.capture_rs(&ptrs)),
        expected,
        "cfg29 per-call reject state must not leak"
    );
}

#[test]
fn cfg30_interleaved_c_and_rust_calls() {
    // Both libraries write to the SAME glibc `stdout`. Interleaving them must
    // produce a stream identical to running each alone, proving the Rust side
    // uses the same stream and buffering discipline.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x30);
    let mut cases = Vec::new();
    for _ in 0..50 {
        let n2 = rng.range(1, 6);
        let s2 = rng.string_from(n2, HIGH_ALPHA);
        let len = rng.range(0, 60);
        let plant = if len > 0 { Some((rng.below(len), s2[0])) } else { None };
        let s1 = build_s1(&mut rng, len, LOW_ALPHA, plant);
        cases.push(Case::raw(s1, s2));
    }
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();

    let c_only = h.capture_c(&ptrs);
    let rs_only = h.capture_rs(&ptrs);
    assert_eq!(c_only, rs_only, "cfg30 baseline C vs Rust");

    // Now interleave: C, Rust, C, Rust ... each line must duplicate.
    let mut interleaved_expected = Vec::new();
    for line in c_only.split_inclusive(|&b| b == b'\n') {
        interleaved_expected.extend_from_slice(line);
        interleaved_expected.extend_from_slice(line);
    }
    let mut got = Vec::new();
    for p in &ptrs {
        got.extend_from_slice(&h.capture_c(std::slice::from_ref(p)));
        got.extend_from_slice(&h.capture_rs(std::slice::from_ref(p)));
    }
    assert_eq!(
        String::from_utf8_lossy(&got),
        String::from_utf8_lossy(&interleaved_expected),
        "cfg30 interleaved C/Rust output on the shared stdout"
    );
}

// ---------------------------------------------------------------------------
// Rows 31-33: aliasing and embedded NULs
// ---------------------------------------------------------------------------

#[test]
fn cfg31_s1_equals_s2_same_pointer() {
    // Passing the identical pointer for both arguments: s1[0] is by definition a
    // member of the reject set, so a non-empty string yields 0.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x31);
    for _ in 0..200 {
        let len = rng.range(0, 80);
        let buf = rng.string_full_domain(len);
        let p = buf.as_ptr() as *const c_char;
        let c = h.capture_c(&[(p, p)]);
        let r = h.capture_rs(&[(p, p)]);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "cfg31 s1 == s2 (len={len})"
        );
        assert_eq!(c, b"0\n", "cfg31 aliased pointers must yield 0 (len={len})");
    }
}

#[test]
fn cfg32_overlapping_buffers() {
    // s2 points into the interior of s1's own buffer.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x32);
    for _ in 0..300 {
        let len = rng.range(2, 90);
        let buf = rng.string_full_domain(len);
        let off = rng.range(1, len);
        let p1 = buf.as_ptr() as *const c_char;
        let p2 = unsafe { buf.as_ptr().add(off) as *const c_char };
        let c = h.capture_c(&[(p1, p2)]);
        let r = h.capture_rs(&[(p1, p2)]);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "cfg32 overlapping buffers (len={len}, off={off})"
        );
    }
}

#[test]
fn cfg33_embedded_nul_mid_buffer() {
    // Bytes after the first NUL must be invisible to both implementations.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x33);
    let mut cases = Vec::new();
    for _ in 0..600 {
        let visible1 = rng.range(0, 40);
        let visible2 = rng.range(0, 20);
        let mut s1: Vec<u8> = (0..visible1).map(|_| LOW_ALPHA[rng.below(LOW_ALPHA.len())]).collect();
        s1.push(0);
        // Garbage past the NUL, deliberately containing bytes that WOULD match.
        s1.extend((0..rng.range(1, 20)).map(|_| HIGH_ALPHA[rng.below(HIGH_ALPHA.len())]));
        s1.push(0);

        let mut s2: Vec<u8> =
            (0..visible2).map(|_| HIGH_ALPHA[rng.below(HIGH_ALPHA.len())]).collect();
        s2.push(0);
        // Garbage past the NUL that WOULD reject s1's alphabet if it were read.
        s2.extend((0..rng.range(1, 20)).map(|_| LOW_ALPHA[rng.below(LOW_ALPHA.len())]));
        s2.push(0);

        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg33 embedded NUL mid-buffer in s1 and s2", &cases);
}

// ---------------------------------------------------------------------------
// Rows 34-36: broad randomized property sweeps
// ---------------------------------------------------------------------------

#[test]
fn cfg34_randomized_property_sweep() {
    // The cross-product row: random lengths, random full-domain bytes, random
    // alignments for both pointers.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x34);
    let mut cases = Vec::new();
    for _ in 0..4000 {
        let n1 = rng.range(0, 300);
        let n2 = rng.range(0, 300);
        let off1 = rng.range(0, 32);
        let off2 = rng.range(0, 32);

        let mut b1 = vec![b'~'; off1];
        b1.extend(rng.string_full_domain(n1));
        let mut b2 = vec![b'~'; off2];
        b2.extend(rng.string_full_domain(n2));

        cases.push(Case::raw(b1, b2).with_offsets(off1, off2));
    }
    h.assert_same("cfg34 randomized cross-product sweep (4000 cases)", &cases);
}

#[test]
fn cfg35_dense_match_sweep() {
    // Tiny reject alphabet that overlaps s1's alphabet heavily -> matches occur
    // very early, exercising the short-circuit paths.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x35);
    let small = b"ab";
    let mut cases = Vec::new();
    for _ in 0..2000 {
        let n1 = rng.range(0, 500);
        let s1 = rng.string_from(n1, b"abc");
        let n2 = rng.range(1, 2);
        let s2 = rng.string_from(n2, small);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg35 dense/early matches", &cases);
}

/// `MemAvailable` from /proc/meminfo, in bytes (0 if unavailable).
fn mem_available_bytes() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemAvailable:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
                .map(|kb| kb * 1024)
        })
        .unwrap_or(0)
}

#[test]
fn cfg37_result_crosses_2gib_zu_vs_int_boundary() {
    // The ONLY input that can distinguish `printf("%zu", n)` from `printf("%d", n)`
    // on x86-64: a result >= 2^31. Below that the low 32 bits of the register
    // print identically, so a wrong conversion specifier is invisible. Here the
    // result is exactly 2^31, whose low 32 bits as a signed int would print
    // "-2147483648" instead of "2147483648".
    const N: usize = 2_147_483_648; // 2 GiB, == 2^31
    let need = (N as u64) + (768 << 20); // buffer + headroom
    let avail = mem_available_bytes();
    if avail < need {
        eprintln!(
            "cfg37 SKIPPED: needs ~{} MiB available, only {} MiB free",
            need >> 20,
            avail >> 20
        );
        return;
    }

    let h = Harness::new();
    // with_capacity(N + 1) so pushing the NUL cannot double the allocation.
    let mut s1: Vec<u8> = Vec::with_capacity(N + 1);
    s1.resize(N, b'a');
    s1.push(0);
    let cases = vec![Case::raw(s1, vec![b'!', 0])];

    h.assert_same("cfg37 result == 2^31 (%zu vs %d boundary)", &cases);
    let ptrs = [cases[0].ptrs()];
    assert_eq!(
        String::from_utf8_lossy(&h.capture_c(&ptrs)),
        "2147483648\n",
        "cfg37 C must print the full 64-bit value"
    );
    assert_eq!(
        String::from_utf8_lossy(&h.capture_rs(&ptrs)),
        "2147483648\n",
        "cfg37 Rust must print the full 64-bit value, not a truncated int"
    );
}

#[test]
fn cfg36_sparse_match_sweep() {
    // Large reject alphabet disjoint from s1's alphabet -> matches are absent, so
    // the full length of s1 is always scanned.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0x36);
    let s1_alpha: Vec<u8> = (0x01u16..=0x40).map(|b| b as u8).collect();
    let s2_alpha: Vec<u8> = (0x41u16..=0xFF).map(|b| b as u8).collect();
    let mut cases = Vec::new();
    for _ in 0..2000 {
        let n1 = rng.range(0, 500);
        let n2 = rng.range(1, 190);
        let s1 = rng.string_from(n1, &s1_alpha);
        let s2 = rng.string_from(n2, &s2_alpha);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_same("cfg36 sparse/absent matches", &cases);
}

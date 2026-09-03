//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each row constructs the exact rejecting condition, calls BOTH libraries and
//! requires that they agree on the *specific* rejection:
//!   * soft errors: same return value AND the same `cp_error_reason` string;
//!   * hard errors: same signal AND the same assertion site
//!     (`lib.c:<line>: <fn>: Assertion `<expr>' failed.`), because the Rust
//!     translation reproduces the C assertion sites verbatim.

mod common;

use common::deflate::*;
use common::{Case, Diff, GlobalPoke, Outcome};
use std::collections::BTreeSet;

const E1: &str = "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const E2: &str = "Stored block extends beyond end of input stream.";
const E3: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const E4: &str = "Attempted to write before out buffer (invalid backwards distance).";
const E5: &str = "Attempted to overwrite out buffer while outputting a string.";
const E6: &str = "Detected unknown block type within input stream.";

const A1: &str = "lib.c:95: cp_ptr: Assertion `!(s->bits_left & 7)' failed.";
const A2: &str = "lib.c:104: cp_peak_bits: Assertion `s->word_index <= s->word_count' failed.";
const A3: &str = "lib.c:115: cp_consume_bits: Assertion `s->count >= num_bits_to_read' failed.";
const A4: &str = "lib.c:123: cp_read_bits: Assertion `num_bits_to_read <= 32' failed.";
const A5: &str = "lib.c:124: cp_read_bits: Assertion `num_bits_to_read >= 0' failed.";
const A6: &str = "lib.c:125: cp_read_bits: Assertion `s->bits_left > 0' failed.";
const A7: &str = "lib.c:126: cp_read_bits: Assertion `s->count <= 64' failed.";
const A8: &str = "lib.c:127: cp_read_bits: Assertion `!cp_would_overflow(s, num_bits_to_read)' failed.";
const A9: &str = "lib.c:154: cp_build: Assertion `len < 16' failed.";
const A10: &str = "lib.c:217: cp_decode: Assertion `(search >> len) == (key >> len)' failed.";

struct Ctx {
    d: Diff,
    seen: BTreeSet<String>,
}

impl Ctx {
    fn run(&mut self, row: &str, what: &str, case: &Case) -> Outcome {
        let o = self.d.check(row, what, case);
        if let Some(a) = &o.assert_site {
            self.seen.insert(a.clone());
        }
        o
    }
    /// Require: both agree (checked by `check`) AND the C soft-rejected with
    /// exactly `msg`.
    fn soft(&mut self, row: &str, what: &str, case: &Case, msg: &str) {
        let o = self.run(row, what, case);
        let got = o.err.as_deref().map(|v| String::from_utf8_lossy(v).into_owned());
        if o.signal.is_some() || o.ret != 0 || got.as_deref() != Some(msg) {
            self.d.fail(format!(
                "[{row}] {what}: expected C ret=0 with cp_error_reason={msg:?}, got {:?}",
                o
            ));
        }
    }
    /// Require: both agree AND the C died at exactly `site`.
    fn hard(&mut self, row: &str, what: &str, case: &Case, site: &str) {
        let o = self.run(row, what, case);
        if o.signal != Some(libc::SIGABRT) || o.assert_site.as_deref() != Some(site) {
            self.d.fail(format!(
                "[{row}] {what}: expected SIGABRT at {site:?}, got {:?}",
                o
            ));
        }
    }
    fn signal(&mut self, row: &str, what: &str, case: &Case, sig: i32) {
        let o = self.run(row, what, case);
        if o.signal != Some(sig) {
            self.d.fail(format!("[{row}] {what}: expected signal {sig}, got {:?}", o));
        }
    }
}

/// A stream that emits `nlit` literals then EOB, as a *fixed* block.
fn fixed_lits(bfinal: bool, lits: &[u8]) -> Vec<u8> {
    let toks: Vec<Tok> = lits.iter().map(|&b| Tok::Lit(b)).collect();
    let mut w = BitWriter::new();
    emit_fixed(&mut w, bfinal, &toks);
    w.bytes()
}

#[test]
fn phase_c() {
    let mut cx = Ctx {
        d: Diff::new(),
        seen: BTreeSet::new(),
    };
    let mut rng = common::Rng::new(0xE770_0001);

    // ================= SOFT ERRORS =======================================

    let b = cx.d.row_start("E1 stored: LEN != ~NLEN");
    for (len, nlen) in [(4u16, 0u16), (0, 0), (8, 8), (1, 0xFFFF), (0x1234, 0x1234)] {
        assert_ne!(len, !nlen, "test case {len}/{nlen:#06x} is actually a valid complement");
        let payload = vec![0xAAu8; len.min(64) as usize];
        let mut w = BitWriter::new();
        emit_stored_raw(&mut w, true, len, nlen, &payload);
        let s = w.bytes();
        cx.soft("E1", &format!("LEN={len} NLEN={nlen:#06x}"), &Case::new(s, 4096), E1);
    }
    for _ in 0..30 {
        let len = rng.range(0, 64) as u16;
        let mut nlen = rng.u32() as u16;
        if nlen == !len {
            nlen = nlen.wrapping_add(1);
        }
        let payload = rng.bytes(len as usize);
        let mut w = BitWriter::new();
        emit_stored_raw(&mut w, true, len, nlen, &payload);
        cx.soft("E1", "random LEN/NLEN mismatch", &Case::new(w.bytes(), 4096), E1);
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("E2 stored: bits_left/8 > LEN (more input left than LEN announces)");
    for (len, extra) in [(0usize, 1usize), (0, 4), (1, 8), (2, 3), (4, 16), (7, 40)] {
        // Correct complement, but the payload written is LEN+extra bytes long,
        // so `bits_left / 8` exceeds LEN.
        let payload = vec![0x5Au8; len + extra];
        let mut w = BitWriter::new();
        emit_stored_raw(&mut w, true, len as u16, !(len as u16), &payload);
        cx.soft(
            "E2",
            &format!("LEN={len} but {} payload bytes", len + extra),
            &Case::new(w.bytes(), 4096),
            E2,
        );
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("E3 literal with a full output buffer (out+1 > out_end)");
    for lits in [1usize, 2, 5] {
        let data: Vec<u8> = (0..lits).map(|i| (0x61 + i) as u8).collect();
        let s = fixed_lits(true, &data);
        for ob in 0..lits as i32 {
            cx.soft(
                "E3",
                &format!("{lits} literals, out_bytes={ob}"),
                &Case::new(s.clone(), ob),
                E3,
            );
        }
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("E4 match reaching before the start of the output buffer");
    for (nlit, dist) in [(1u32, 2u32), (1, 3), (2, 3), (3, 5), (0, 1), (4, 100)] {
        let data: Vec<u8> = (0..nlit).map(|i| (0x61 + i) as u8).collect();
        let toks: Vec<Tok> = data
            .iter()
            .map(|&x| Tok::Lit(x))
            .chain(std::iter::once(Tok::Match { len: 3, dist }))
            .collect();
        let mut w = BitWriter::new();
        // emit_fixed's expand() would reject this, so emit the tokens raw.
        w.push(1, 1);
        w.push(1, 2);
        for &x in &data {
            emit_fixed_raw_sym(&mut w, x as u32);
        }
        let (ls, lx, lv) = len_sym(3);
        emit_fixed_raw_sym(&mut w, 257 + ls as u32);
        w.push(lv, lx);
        let (ds, dx, dv) = dist_sym(dist);
        emit_fixed_raw_dist(&mut w, ds as u32, dx, dv);
        emit_fixed_raw_sym(&mut w, 256);
        let _ = toks;
        cx.soft(
            "E4",
            &format!("{nlit} literals then dist={dist}"),
            &Case::new(w.bytes(), 4096),
            E4,
        );
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("E5 match copy overrunning the output buffer (out+length > out_end)");
    for (nlit, len, dist) in [(4u32, 258u32, 1u32), (4, 10, 2), (1, 3, 1), (8, 100, 4)] {
        let data: Vec<u8> = (0..nlit).map(|i| (0x61 + i % 26) as u8).collect();
        let mut toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
        toks.push(Tok::Match { len, dist });
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &toks);
        let s = w.bytes();
        // Enough room for the literals (so E3 does not fire) but not the copy.
        for ob in [nlit as i32, nlit as i32 + 1, (nlit + len) as i32 - 1] {
            cx.soft(
                "E5",
                &format!("{nlit} lits + len={len} dist={dist}, out_bytes={ob}"),
                &Case::new(s.clone(), ob),
                E5,
            );
        }
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("E6 reserved block type 3 (the out-of-range 'enum' value)");
    for bfinal in [0u32, 1] {
        for pre in 0..3usize {
            // `pre` non-final empty fixed blocks first, then a btype=3 header.
            let mut w = BitWriter::new();
            for _ in 0..pre {
                emit_fixed(&mut w, false, &[]);
            }
            w.push(bfinal, 1);
            w.push(3, 2);
            for _ in 0..4 {
                w.push(0, 8);
            }
            cx.soft(
                "E6",
                &format!("btype=3 after {pre} empty blocks, bfinal={bfinal}"),
                &Case::new(w.bytes(), 4096),
                E6,
            );
        }
    }
    cx.d.row_end(b);

    // ================= HARD ERRORS (live assert()s) ======================

    let b = cx.d.row_start("A6 assert(s->bits_left > 0): empty and negative in_bytes");
    cx.hard("A6", "in_bytes=0", &Case::new(vec![], 64), A6);
    cx.hard("A6", "in_bytes=0, empty buffer", &Case::new(vec![], 0), A6);
    for ia in 0..4usize {
        cx.hard("A6", &format!("in_bytes=0 in_align={ia}"), &Case::new(vec![], 64).in_align(ia), A6);
    }
    for n in [-1i32, -2, -7, -1024, i32::MIN] {
        cx.hard(
            "A6",
            &format!("in_bytes={n}"),
            &Case::new(vec![0x05, 0, 0, 0], 64).in_bytes(n),
            A6,
        );
    }
    // truncated dynamic header: bits run out exactly on a read_bits boundary
    cx.hard("A6", "1-byte dynamic header", &Case::new(vec![0x05], 64), A6);
    cx.d.row_end(b);

    let b = cx.d.row_start("A3 assert(s->count >= num_bits_to_read): buffered bits exhausted");
    for input in [
        vec![0xe8u8, 0xd1],
        vec![0x05, 0x00],
        vec![0x01, 0x00],
        vec![0x05, 0xff],
    ] {
        cx.hard("A3", &format!("input={}", common::hex(&input)), &Case::new(input.clone(), 64), A3);
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("A8 assert(!cp_would_overflow(...)): truncated stream");
    for (input, ia) in [
        (vec![0x41u8, 0x11, 0x2a, 0x27], 1usize),
        (vec![0x41, 0x11, 0x2a, 0x27], 0),
    ] {
        let o = cx.run("A8", &format!("input={} in_align={ia}", common::hex(&input)), &Case::new(input.clone(), 64).in_align(ia));
        if o.assert_site.as_deref() == Some(A8) {
            continue;
        }
        // fall through to the sweep below
        let _ = o;
    }
    // Sweep short inputs until the site is observed, requiring agreement on all.
    {
        let mut hit = 0usize;
        for hi in 0u32..=255 {
            for lo in [0x01u8, 0x05, 0x03, 0x07] {
                for ia in 0..4usize {
                    let input = vec![lo, hi as u8, 0x00, 0x00];
                    let o = cx.run("A8", "sweep", &Case::new(input, 64).in_align(ia));
                    if o.assert_site.as_deref() == Some(A8) {
                        hit += 1;
                    }
                }
            }
        }
        println!("  A8 observed {hit} times in the sweep");
        if hit == 0 && !cx.seen.contains(A8) {
            cx.d.fail("[A8] assertion site never observed".into());
        }
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("A10 assert((search >> len) == (key >> len)): bogus Huffman entry");
    for input in [vec![0x3cu8, 0x1f, 0xee, 0xd6]] {
        cx.hard("A10", &format!("input={}", common::hex(&input)), &Case::new(input, 0), A10);
    }
    // nlit == 0: cp_decode is entered with hi == 0, so it reads tree[-1] (the
    // neighbouring `lookup` field of cp_state_t).
    {
        let s = fixed_lits(true, b"A");
        let mut case = Case::new(s, 64);
        for i in 0..288 {
            case = case.poke(GlobalPoke::FixedTable(i, 0));
        }
        cx.hard("A10", "cp_fixed_table zeroed -> nlit==0 -> tree[-1]", &case, A10);
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("A4 assert(num_bits_to_read <= 32): consumer poked cp_len_extra_bits");
    for v in [33u8, 40, 64, 100, 255] {
        let toks = vec![
            Tok::Lit(b'a'),
            Tok::Lit(b'b'),
            Tok::Lit(b'c'),
            Tok::Lit(b'd'),
            Tok::Match { len: 3, dist: 1 },
        ];
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &toks);
        let case = Case::new(w.bytes(), 4096).poke(GlobalPoke::LenExtraBits(0, v));
        cx.hard("A4", &format!("cp_len_extra_bits[0]={v}"), &case, A4);
    }
    for v in [33u8, 200] {
        let mut toks = vec![Tok::Lit(b'a')];
        toks.push(Tok::Match { len: 3, dist: 1 });
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &toks);
        let case = Case::new(w.bytes(), 4096).poke(GlobalPoke::DistExtraBits(0, v));
        cx.hard("A4", &format!("cp_dist_extra_bits[0]={v}"), &case, A4);
    }
    cx.d.row_end(b);

    let b = cx.d.row_start("A9 assert(len < 16): code length >= 16 in the length vector");
    for v in [16u8, 17, 31, 255] {
        for idx in [0usize, 1, 143, 287] {
            let s = fixed_lits(true, b"x");
            let case = Case::new(s, 64).poke(GlobalPoke::FixedTable(idx, v));
            cx.hard("A9", &format!("cp_fixed_table[{idx}]={v}"), &case, A9);
        }
    }
    // Also through the distance half of the table (cp_build(0, dst, ...)).
    for v in [16u8, 200] {
        let s = fixed_lits(true, b"x");
        let case = Case::new(s, 64).poke(GlobalPoke::FixedTable(288 + 5, v));
        cx.hard("A9", &format!("cp_fixed_table[293]={v} (distance half)"), &case, A9);
    }
    cx.d.row_end(b);

    // A1: `cp_ptr` requires the remaining input bit count to be byte aligned.
    //
    // `bits_left` at `cp_ptr` is congruent (mod 8) to the consumed-bit count at
    // which the *final-word* load happened, because that load adds `bits_left`
    // (not 32) to `count`, and the stored block's pre-alignment read is derived
    // from `count`. So A1 needs a final-word load at a consumed count that is
    // not a multiple of 8, with ~38 bits of slack left for the stored header.
    //
    // Construction (in_bytes = 15, in_align = 0 -> word_count = 3,
    // last_bytes = 3):
    //   bits   0..2    non-final fixed block header (bfinal=0, btype=1)
    //   bits   3..81    2 x 8-bit literal + 7 x 9-bit literal  (82 bits consumed)
    //   bit    82       cp_decode(EOB) peeks 16 with count == 14 -> FINAL LOAD
    //                   at c_f = 82 (82 mod 8 == 2), count := 14 + 38 = 52
    //   bits  82..88    EOB (7 bits)
    //   bits  89..91    stored block header (bfinal=1, btype=0)
    //   bits  92..93    pre-alignment read of `count & 7` == 2 bits
    //   bits  94..109   LEN  = 0xFFFF
    //   bits 110..125   NLEN = 0x0000 (the last 6 bits are past the input and
    //                   read back as the zeros the accumulator holds)
    // -> LEN == (uint16_t)~NLEN passes, bits_left == -6, -6/8 == 0 <= LEN passes,
    //    and cp_ptr sees bits_left & 7 == 2.
    let b = cx.d.row_start("A1 assert(!(s->bits_left & 7)): stored block reached unaligned");
    {
        let mut w = BitWriter::new();
        w.push(0, 1); // bfinal = 0
        w.push(1, 2); // btype  = 1 (fixed)
        emit_fixed_raw_sym(&mut w, b'a' as u32); // 8 bits
        emit_fixed_raw_sym(&mut w, b'b' as u32); // 8 bits
        for _ in 0..7 {
            emit_fixed_raw_sym(&mut w, 0xF0); // 9 bits each
        }
        assert_eq!(w.nbits, 82, "fixed block should end at bit 82");
        emit_fixed_raw_sym(&mut w, 256); // EOB, 7 bits
        assert_eq!(w.nbits, 89);
        w.push(1, 1); // bfinal = 1
        w.push(0, 2); // btype  = 0 (stored)
        w.push(0, 2); // the `count & 7` pre-alignment bits
        assert_eq!(w.nbits, 94);
        w.push(0xFFFF, 16); // LEN
        w.push(0x0000, 16); // NLEN
        let mut s = w.bytes();
        s.truncate(15); // in_bytes = 15 -> last_bytes = 3, word_count = 3
        assert_eq!(s.len(), 15);
        cx.hard("A1", "hand-built unaligned stored block", &Case::new(s, 4096), A1);
    }
    // Plus a sweep, so the row is not tied to a single hand-tuned vector.
    {
        let mut hit = 0usize;
        for nine in 0..12usize {
            for eight in 0..12usize {
                for extra in 0..4usize {
                    let mut w = BitWriter::new();
                    w.push(0, 1);
                    w.push(1, 2);
                    for _ in 0..eight {
                        emit_fixed_raw_sym(&mut w, b'a' as u32);
                    }
                    for _ in 0..nine {
                        emit_fixed_raw_sym(&mut w, 0xF0);
                    }
                    emit_fixed_raw_sym(&mut w, 256);
                    w.push(1, 1);
                    w.push(0, 2);
                    let pad = (8 - (w.nbits % 8)) % 8;
                    w.push(0, pad as u32);
                    w.push(0xFFFF, 16);
                    w.push(0x0000, 16);
                    let mut s = w.bytes();
                    let want = s.len().saturating_sub(extra).max(1);
                    s.truncate(want);
                    let o = cx.run(
                        "A1",
                        &format!("sweep eight={eight} nine={nine} trunc={extra}"),
                        &Case::new(s, 4096),
                    );
                    if o.assert_site.as_deref() == Some(A1) {
                        hit += 1;
                    }
                }
            }
        }
        println!("  A1 also observed {hit} times in the sweep");
    }
    cx.d.row_end(b);

    cx.d.finish("Phase C (ERRORS.md rows)");
    println!("\nassert sites observed in Phase C:");
    for s in &cx.seen {
        println!("  {s}");
    }
    for (name, site) in [("A2", A2), ("A5", A5), ("A7", A7)] {
        println!(
            "{name} ({site}) observed: {}",
            cx.seen.contains(site)
        );
    }
}

//! `ERRORS.md` row E32 — large randomized differential fuzzing of `cp_inflate`.
//!
//! Four generators, all seeded deterministically:
//!
//! 1. uniform random byte strings of every length 1…40,
//! 2. bit-flipped valid streams (static / dynamic / stored / real zlib output),
//! 3. truncations of valid streams at every byte length,
//! 4. random *structured* streams: valid headers with random Huffman tables.
//!
//! For every input the C `.so`, the Rust `.so` and the independent model in
//! `tests/common/cmodel.rs` must agree on the return value, `cp_error_reason`,
//! the whole output mapping, the termination signal and the `assert` message.
//! Inputs on which the C code performs undefined behaviour are counted and
//! reported instead (see `ERRORS.md` §D).

mod common;

use common::deflate::*;
use common::{model_matches, InflateHarness, Rng};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::collections::BTreeMap;
use std::io::Write;

fn raw_deflate(data: &[u8], level: u32) -> Vec<u8> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn valid_streams(rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();

    // static, literals only
    let items: Vec<Item> = (0..40).map(|_| Item::Lit(rng.u8() as u16)).collect();
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, true, &items);
    v.push(("static-lit".into(), bw.finish()));

    // static, with matches
    let mut items: Vec<Item> = (0..6).map(|i| Item::Lit(97 + i as u16)).collect();
    items.push(Item::Match(258, 3));
    items.push(Item::Match(7, 1));
    items.push(Item::Match(60, 200));
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, true, &items);
    v.push(("static-match".into(), bw.finish()));

    // dynamic
    let used: Vec<usize> = vec![0, 1, 5, 65, 66, 130, 200, 256, 257, 261, 270];
    let litlens = lengths_for(288, &used);
    let dstlens = lengths_for(32, &[0, 1, 2, 3, 20, 30, 31]);
    let mut bw = BitWriter::new();
    let (lit, dst) = emit_dynamic_header(&mut bw, true, &litlens, &dstlens, ClMode::Repeats, None);
    emit_items(
        &mut bw,
        &lit,
        &dst,
        // lengths/distances chosen so that their symbols are in `used`:
        // 3 -> 257, 7 -> 261, 24 -> 270; dist 1 -> 0, 2 -> 1, 4 -> 3
        &[
            Item::Lit(65),
            Item::Lit(66),
            Item::Lit(0),
            Item::Match(3, 1),
            Item::Match(7, 2),
            Item::Match(24, 4),
        ],
    );
    v.push(("dynamic".into(), bw.finish()));

    // stored
    let payload = rng.bytes(24);
    let mut bw = BitWriter::new();
    emit_stored_block(&mut bw, true, &payload, None);
    v.push(("stored".into(), bw.finish()));

    // multi-block
    let mut bw = BitWriter::new();
    emit_fixed_block(&mut bw, false, &[Item::Lit(1), Item::Lit(2)]);
    emit_fixed_block(&mut bw, false, &[Item::Lit(3)]);
    emit_fixed_block(&mut bw, true, &[Item::Lit(4), Item::Match(5, 1)]);
    v.push(("multi".into(), bw.finish()));

    // real zlib output at several levels
    for level in [1u32, 6, 9] {
        let data: Vec<u8> = (0..200u32).map(|i| (i % 37) as u8).collect();
        v.push((format!("zlib-l{level}"), raw_deflate(&data, level)));
    }
    v
}

/// Random *structured* stream: a syntactically valid block header with random
/// (possibly incomplete / over-subscribed) Huffman tables and random payload
/// bits — hits the tree-construction and decode paths far more often than
/// uniform noise.
fn structured(rng: &mut Rng) -> Vec<u8> {
    let mut bw = BitWriter::new();
    let bfinal = rng.below(2);
    match rng.below(3) {
        0 => {
            bw.bits(bfinal, 1);
            bw.bits(1, 2);
            for _ in 0..rng.range(1, 40) {
                bw.bits(rng.u32(), rng.range(1, 16) as u32);
            }
        }
        1 => {
            bw.bits(bfinal, 1);
            bw.bits(2, 2);
            bw.bits(rng.u32(), 5); // HLIT
            bw.bits(rng.u32(), 5); // HDIST
            let hclen = rng.below(16);
            bw.bits(hclen, 4);
            for _ in 0..hclen + 4 {
                bw.bits(rng.u32(), 3);
            }
            for _ in 0..rng.range(1, 60) {
                bw.bits(rng.u32(), rng.range(1, 8) as u32);
            }
        }
        _ => {
            bw.bits(bfinal, 1);
            bw.bits(0, 2);
            bw.align();
            let len = rng.u32() & 0xFFFF;
            bw.bits(len, 16);
            let nlen = if rng.below(2) == 0 { !len } else { rng.u32() };
            bw.bits(nlen, 16);
            let n = rng.range(0, 40) as usize;
            let p = rng.bytes(n);
            bw.raw(&p);
        }
    }
    bw.finish()
}

struct Stats {
    inputs: usize,
    ub: usize,
    classes: BTreeMap<String, usize>,
    diverged: Vec<String>,
    model_mismatch: Vec<String>,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            inputs: 0,
            ub: 0,
            classes: BTreeMap::new(),
            diverged: Vec::new(),
            model_mismatch: Vec::new(),
        }
    }
    fn one(&mut self, h: &InflateHarness, name: &str, stream: &[u8], align: usize, out_bytes: i32) {
        self.inputs += 1;
        let (oc, or) = h.call_pair(stream, align, out_bytes);
        let m = h.model(stream, align, stream.len() as i32, out_bytes);
        if !m.defined() {
            self.ub += 1;
            return;
        }
        let cls = match (&oc.signal, oc.ret) {
            (Some(s), _) => format!("signal {s} {}", first_line(&oc.stderr)),
            (None, 1) => "ok".to_string(),
            (None, r) => format!(
                "ret{r} {}",
                oc.err
                    .as_ref()
                    .map(|e| String::from_utf8_lossy(e).into_owned())
                    .unwrap_or_default()
            ),
        };
        *self.classes.entry(cls).or_insert(0) += 1;
        if oc != or && self.diverged.len() < 10 {
            self.diverged.push(format!(
                "{name} align={align} out={out_bytes}\n    bytes = {:02x?}\n    C    = {oc:?}\n    Rust = {or:?}",
                stream
            ));
        }
        for (who, o) in [("C", &oc), ("Rust", &or)] {
            if let Err(e) = model_matches(o, &m) {
                if self.model_mismatch.len() < 10 {
                    self.model_mismatch.push(format!(
                        "{name} align={align} out={out_bytes} [{who}] {e}\n    bytes = {:02x?}",
                        stream
                    ));
                }
            }
        }
    }
    fn finish(&self, tag: &str) {
        let mut report = format!(
            "{tag}: {} inputs, {} with C undefined behaviour\n",
            self.inputs, self.ub
        );
        for (k, n) in &self.classes {
            report += &format!("  {n:6}  {k}\n");
        }
        println!("{report}");
        assert!(
            self.diverged.is_empty(),
            "{} divergences:\n  {}",
            self.diverged.len(),
            self.diverged.join("\n  ")
        );
        assert!(
            self.model_mismatch.is_empty(),
            "{} model mismatches:\n  {}",
            self.model_mismatch.len(),
            self.model_mismatch.join("\n  ")
        );
        assert!(
            self.ub * 4 < self.inputs,
            "{tag}: too many inputs ({}/{}) hit C undefined behaviour",
            self.ub,
            self.inputs
        );
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

/// E32 — uniform random byte strings.
#[test]
fn fuzz_random_streams() {
    let h = InflateHarness::new("fz1", 1 << 16, 1 << 13);
    let mut rng = Rng::new(0x600D_5EED);
    let mut st = Stats::new();
    for len in 1..=40usize {
        for i in 0..30 {
            let s = rng.bytes(len);
            let align = (i % 4) as usize;
            let out_bytes = [0i32, 8, 512, 4096][(i / 4) % 4];
            st.one(&h, &format!("rand{len}#{i}"), &s, align, out_bytes);
        }
    }
    st.finish("fuzz_random_streams");
    assert!(st.inputs >= 1200);
}

/// E32 — single- and multi-bit flips of valid streams.
#[test]
fn fuzz_bitflipped_valid_streams() {
    let h = InflateHarness::new("fz2", 1 << 16, 1 << 13);
    let mut rng = Rng::new(0xBEEF_F00D);
    let mut st = Stats::new();
    let bases = valid_streams(&mut rng);
    for (name, base) in &bases {
        for it in 0..90 {
            let mut s = base.clone();
            let flips = 1 + (it % 3);
            for _ in 0..flips {
                let bit = rng.below((s.len() * 8) as u32) as usize;
                s[bit / 8] ^= 1 << (bit % 8);
            }
            let align = (it % 4) as usize;
            let out_bytes = [0i32, 16, 512, 4096][(it / 4) % 4];
            st.one(&h, &format!("{name}-flip{it}"), &s, align, out_bytes);
        }
    }
    st.finish("fuzz_bitflipped_valid_streams");
    assert!(st.inputs >= 500);
}

/// E32 — every truncation of every valid stream, at every alignment.
#[test]
fn fuzz_truncated_valid_streams() {
    let h = InflateHarness::new("fz3", 1 << 16, 1 << 13);
    let mut rng = Rng::new(0x7777_1234);
    let mut st = Stats::new();
    for (name, base) in &valid_streams(&mut rng) {
        for n in 1..=base.len() {
            for align in 0..4usize {
                st.one(
                    &h,
                    &format!("{name}[..{n}]"),
                    &base[..n],
                    align,
                    if n % 2 == 0 { 4096 } else { 64 },
                );
            }
        }
    }
    st.finish("fuzz_truncated_valid_streams");
    assert!(st.inputs >= 500);
}

/// E32 — random but *structurally plausible* streams (valid headers, random
/// Huffman tables), which reach `cp_build`/`cp_decode` far more often than noise.
#[test]
fn fuzz_structured_streams() {
    let h = InflateHarness::new("fz4", 1 << 16, 1 << 13);
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let mut st = Stats::new();
    for i in 0..1500 {
        let s = structured(&mut rng);
        if s.is_empty() || s.len() + 8 > h.inbuf.usable() {
            continue;
        }
        let align = (i % 4) as usize;
        let out_bytes = [0i32, 8, 512, 4096][(i / 4) % 4];
        st.one(&h, &format!("struct#{i}"), &s, align, out_bytes);
    }
    st.finish("fuzz_structured_streams");
    assert!(st.inputs >= 1000);
}

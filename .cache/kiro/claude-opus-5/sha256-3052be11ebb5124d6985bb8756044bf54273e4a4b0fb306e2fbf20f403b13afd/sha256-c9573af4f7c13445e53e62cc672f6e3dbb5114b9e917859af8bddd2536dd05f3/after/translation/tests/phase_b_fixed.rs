//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every row drives BOTH `.so`s through `pinflate` with many randomized inputs
//! (fixed seed) and compares the return value, the whole output window plus
//! slack, `cp_error_reason` and the process wait status.
//!
//! Each row additionally asserts NON-VACUITY: the C library must really have
//! accepted the stream (`ret == 1`) and produced the byte sequence the encoder
//! intended. Without that, a row where both libraries abort identically would
//! silently "pass".

mod common;

use common::deflate::*;
use common::{Case, Diff};

const SEED: u64 = 0x5EED_1234;

/// Run one valid stream and require the C to accept it with exactly `expect`.
fn valid(d: &mut Diff, row: &str, what: &str, stream: Vec<u8>, expect: &[u8], case: Case) {
    let c = d.check(row, what, &case);
    let n = expect.len();
    let ok = c.signal.is_none() && c.ret == 1 && c.out.len() >= n && &c.out[..n] == expect;
    if !ok {
        d.fail(format!(
            "[{row}] {what}: VACUOUS — the C library did not accept the stream.\n    \
             stream={}\n    expected out[{n}]={}\n    C: {:?}",
            common::hex(&stream),
            common::hex(expect),
            c
        ));
    }
    // Nothing may be written past out_bytes.
    if case.out_bytes >= 0 {
        let ob = case.out_bytes as usize;
        if c.out.len() > ob && c.out[ob..].iter().any(|&b| b != case.out_fill) {
            d.fail(format!(
                "[{row}] {what}: C wrote past out_bytes={ob}: {}",
                common::hex(&c.out[ob..])
            ));
        }
    }
}

fn fixed_case(toks: &[Tok]) -> (Vec<u8>, Vec<u8>) {
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, toks);
    (w.bytes(), expand(toks))
}

/// Build a long history cheaply: one literal then RLE runs of distance 1.
fn history(n: u32, seed: u8) -> Vec<Tok> {
    let mut t = vec![Tok::Lit(seed)];
    let mut have = 1u32;
    while have < n {
        let l = (n - have).min(258).max(3);
        t.push(Tok::Match { len: l, dist: 1 });
        have += l;
    }
    t
}

#[test]
fn phase_b() {
    let mut d = Diff::new();
    let mut rng = common::Rng::new(SEED);

    // ---------------- C1: fixed, 8-bit literal class (0..143) -------------
    let b = d.row_start("C1 fixed / literals 0..143 (8-bit codes)");
    for _ in 0..40 {
        let n = rng.range(1, 200) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(rng.below(144) as u8))
            .collect();
        let (s, e) = fixed_case(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C1", "8-bit literals", s, &e, c);
    }
    d.row_end(b);

    // ---------------- C2: fixed, 9-bit literal class (144..255) -----------
    let b = d.row_start("C2 fixed / literals 144..255 (9-bit codes)");
    for _ in 0..40 {
        let n = rng.range(1, 200) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(144 + rng.below(112) as u8))
            .collect();
        let (s, e) = fixed_case(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C2", "9-bit literals", s, &e, c);
    }
    d.row_end(b);

    // ---------------- C3: fixed, both literal classes mixed ---------------
    let b = d.row_start("C3 fixed / literals 0..255 mixed");
    for _ in 0..60 {
        let n = rng.range(1, 400) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let (s, e) = fixed_case(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C3", "mixed literals", s, &e, c);
    }
    d.row_end(b);

    // ---------------- C4: all four `in` alignments ------------------------
    let b = d.row_start("C4 fixed x in_align 0,1,2,3 (first_bytes 0..3)");
    for _ in 0..20 {
        let n = rng.range(1, 120) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let (s, e) = fixed_case(&toks);
        for a in 0..4usize {
            let c = Case::new(s.clone(), e.len() as i32).in_align(a);
            valid(&mut d, "C4", &format!("in_align={a}"), s.clone(), &e, c);
        }
    }
    d.row_end(b);

    // ---------------- C5: all four last_bytes residues --------------------
    // Trailing padding bytes are legal: `bfinal` ends the loop, so the extra
    // bytes only change bits_left / word_count / final_word.
    let b = d.row_start("C5 fixed x last_bytes 0,1,2,3 (final_word_available both)");
    for _ in 0..15 {
        let n = rng.range(1, 120) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let (s0, e) = fixed_case(&toks);
        for a in 0..4usize {
            for pad in 0..4usize {
                let mut s = s0.clone();
                for _ in 0..pad {
                    s.push(rng.byte());
                }
                let residue = (s.len() - ((4 - a) & 3)) & 3;
                let c = Case::new(s.clone(), e.len() as i32).in_align(a);
                valid(
                    &mut d,
                    "C5",
                    &format!("in_align={a} pad={pad} last_bytes~{residue}"),
                    s,
                    &e,
                    c,
                );
            }
        }
    }
    d.row_end(b);

    // ---------------- C6: word_count == 0 (final-word path only) ----------
    let b = d.row_start("C6 fixed / word_count == 0 (whole stream in prefix+final word)");
    {
        // empty fixed block: 3 header bits + 7-bit EOB = 10 bits = 2 bytes
        let (s, e) = fixed_case(&[]);
        assert_eq!(s.len(), 2, "empty fixed block should be 2 bytes");
        for a in 0..4usize {
            let c = Case::new(s.clone(), e.len() as i32).in_align(a);
            valid(&mut d, "C6", &format!("2-byte stream in_align={a}"), s.clone(), &e, c);
        }
        // one literal: 3 + 8 + 7 = 18 bits = 3 bytes
        for lit in [0u8, 0x41, 0xFF] {
            let (s, e) = fixed_case(&[Tok::Lit(lit)]);
            assert_eq!(s.len(), 3);
            for a in 0..4usize {
                let c = Case::new(s.clone(), e.len() as i32).in_align(a);
                valid(
                    &mut d,
                    "C6",
                    &format!("3-byte stream lit={lit:#04x} in_align={a}"),
                    s.clone(),
                    &e,
                    c,
                );
            }
        }
    }
    d.row_end(b);

    // ---------------- C7: word_count == 1 and > 1 -------------------------
    let b = d.row_start("C7 fixed / word_count == 1 and > 1 (cp_peak_bits refill)");
    for target_words in [1usize, 2, 3, 8, 64] {
        // literal count chosen so the stream spans roughly `target_words` words
        let n = (target_words * 4).saturating_sub(2).max(1);
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let (s, e) = fixed_case(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(
            &mut d,
            "C7",
            &format!("~{target_words} words ({} bytes)", s.len()),
            s,
            &e,
            c,
        );
    }
    d.row_end(b);

    // ---------------- C8/C9/C10: every length symbol & extra-bit class ----
    let b = d.row_start("C8/C9/C10 fixed / all 29 length symbols x extra-bit values");
    for ls in 0..29usize {
        let base = LEN_BASE[ls];
        let extra = LEN_EXTRA[ls];
        let span = 1u32 << extra;
        let vals: Vec<u32> = if span <= 4 {
            (0..span).collect()
        } else {
            let mut v = vec![0, span - 1];
            for _ in 0..3 {
                v.push(rng.below(span));
            }
            v
        };
        for v in vals {
            let len = base + v;
            if len > 258 {
                continue;
            }
            for dist in [1u32, 2, 3, len.max(1), len / 2 + 1] {
                let mut toks = vec![Tok::Lit(0x5A)];
                let mut have = 1u32;
                while have < dist {
                    toks.push(Tok::Lit((have & 0xFF) as u8));
                    have += 1;
                }
                toks.push(Tok::Match { len, dist });
                let (s, e) = fixed_case(&toks);
                let c = Case::new(s.clone(), e.len() as i32);
                valid(
                    &mut d,
                    "C8/C9/C10",
                    &format!("lensym {} (base {base} extra {extra}) len={len} dist={dist}", 257 + ls),
                    s,
                    &e,
                    c,
                );
            }
        }
    }
    d.row_end(b);

    // ---------------- C11: distance == 1 -> memset RLE branch -------------
    let b = d.row_start("C11 fixed / distance == 1 (memset branch), lengths 3..258");
    for len in [3u32, 4, 7, 8, 15, 16, 17, 100, 257, 258] {
        for seed in [0x00u8, 0x7F, 0xFF] {
            let toks = vec![Tok::Lit(seed), Tok::Match { len, dist: 1 }];
            let (s, e) = fixed_case(&toks);
            let c = Case::new(s.clone(), e.len() as i32);
            valid(&mut d, "C11", &format!("memset len={len} byte={seed:#04x}"), s, &e, c);
        }
    }
    for _ in 0..30 {
        let len = rng.range(3, 258);
        let toks = vec![Tok::Lit(rng.byte()), Tok::Match { len, dist: 1 }];
        let (s, e) = fixed_case(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C11", &format!("memset random len={len}"), s, &e, c);
    }
    d.row_end(b);

    // ---------------- C12: 1 < distance < length -> overlapping copy ------
    let b = d.row_start("C12 fixed / 1 < distance < length (self-overlapping byte copy)");
    for dist in [2u32, 3, 5, 8, 17, 64] {
        for len in [dist + 1, dist * 2 + 1, 258] {
            if !(3..=258).contains(&len) {
                continue;
            }
            let mut toks: Vec<Tok> = (0..dist).map(|i| Tok::Lit((0x61 + i % 26) as u8)).collect();
            toks.push(Tok::Match { len, dist });
            let (s, e) = fixed_case(&toks);
            let c = Case::new(s.clone(), e.len() as i32);
            valid(&mut d, "C12", &format!("overlap dist={dist} len={len}"), s, &e, c);
        }
    }
    for _ in 0..40 {
        let dist = rng.range(2, 40);
        let len = rng.range(dist + 1, 258);
        let mut toks: Vec<Tok> = (0..dist).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(Tok::Match { len, dist });
        let (s, e) = fixed_case(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C12", &format!("random overlap dist={dist} len={len}"), s, &e, c);
    }
    d.row_end(b);

    // ---------------- C13/C14: every distance symbol ----------------------
    let b = d.row_start("C13/C14 fixed / all 30 distance symbols x extra-bit values");
    for ds in 0..30usize {
        let base = DIST_BASE[ds];
        let extra = DIST_EXTRA[ds];
        let span = 1u32 << extra;
        let vals: Vec<u32> = if span <= 2 {
            (0..span).collect()
        } else {
            vec![0, span - 1, rng.below(span)]
        };
        for v in vals {
            let dist = base + v;
            if dist > 32768 {
                continue;
            }
            let mut toks = history(dist, 0x42);
            toks.push(Tok::Match { len: 3, dist });
            toks.push(Tok::Match { len: 258, dist });
            let (s, e) = fixed_case(&toks);
            let c = Case::new(s.clone(), e.len() as i32);
            valid(
                &mut d,
                "C13/C14",
                &format!("distsym {ds} (base {base} extra {extra}) dist={dist} hist={}", e.len()),
                s,
                &e,
                c,
            );
        }
    }
    d.row_end(b);

    // ---------------- C15: length symbols 285..287 ------------------------
    // 285 is `len_base 258 / 0 extra`; 286 and 287 exist in cp_fixed_table but
    // index cp_len_base[29] / [30], which are 0 -> a zero-length copy.
    let b = d.row_start("C15 fixed / literal-length symbols 285,286,287 (8-bit table tail)");
    for sym in [285u32, 286, 287] {
        for dsym in [0u32, 1, 3] {
            let mut w = BitWriter::new();
            w.push(1, 1);
            w.push(1, 2);
            // 4 bytes of history so any distance up to 4 is in range
            for i in 0..4u32 {
                emit_fixed_raw_sym(&mut w, 0x61 + i);
            }
            emit_fixed_raw_sym(&mut w, sym);
            if sym == 285 {
                // 0 extra bits
            }
            emit_fixed_raw_dist(&mut w, dsym, DIST_EXTRA[dsym as usize], 0);
            emit_fixed_raw_sym(&mut w, 256);
            let s = w.bytes();
            let expect_extra = if sym == 285 { 258usize } else { 0 };
            let out_bytes = (4 + expect_extra) as i32;
            let c = d.check(
                "C15",
                &format!("sym={sym} dsym={dsym}"),
                &Case::new(s.clone(), out_bytes),
            );
            // Non-vacuity: the C must accept and produce 'abcd' + the copy.
            let mut exp = b"abcd".to_vec();
            if sym == 285 {
                let dist = DIST_BASE[dsym as usize] as usize;
                for _ in 0..258 {
                    let byte = exp[exp.len() - dist];
                    exp.push(byte);
                }
            }
            if c.signal.is_some() || c.ret != 1 || &c.out[..exp.len()] != &exp[..] {
                d.fail(format!(
                    "[C15] sym={sym} dsym={dsym}: VACUOUS/unexpected C behaviour: {:?} (expected ret=1 out={})",
                    c,
                    common::hex(&exp)
                ));
            }
        }
    }
    d.row_end(b);

    d.finish("Phase B (part 1: fixed blocks C1-C15)");
}

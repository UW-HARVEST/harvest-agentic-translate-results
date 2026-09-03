//! Phase B part 3 — multi-block sequences (C32-C35), output-buffer shapes
//! (C36-C37), consumer-mutated writable exports (C38-C40) and the randomized
//! valid-stream sweep (C41).

mod common;

use common::deflate::*;
use common::{Case, Diff, GlobalPoke};

const SEED: u64 = 0x5EED_1234;

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
    // Nothing may be written after the decoded data, up to and past out_bytes.
    if let Some(bad) = c.out[n.min(c.out.len())..]
        .iter()
        .position(|&b| b != case.out_fill)
    {
        d.fail(format!(
            "[{row}] {what}: byte {} past the decoded data was modified (0x{:02x})",
            n + bad,
            c.out[n + bad]
        ));
    }
    c.out.len();
}

/// Random token stream that only uses history already produced.
fn rand_toks(rng: &mut common::Rng, count: usize, already: u32) -> (Vec<Tok>, u32) {
    let mut toks = Vec::new();
    let mut have = already;
    for _ in 0..count {
        if have >= 4 && rng.below(100) < 30 {
            let dist = rng.range(1, have.min(4096));
            let len = rng.range(3, 258);
            toks.push(Tok::Match { len, dist });
            have += len;
        } else {
            toks.push(Tok::Lit(rng.byte()));
            have += 1;
        }
    }
    (toks, have)
}

#[test]
fn phase_b_multiblock_globals() {
    let mut d = Diff::new();
    let mut rng = common::Rng::new(SEED ^ 0xC0FFEE);

    // ---------------- C32: two fixed blocks -------------------------------
    let b = d.row_start("C32 two blocks, btype pair (1,1)");
    for _ in 0..30 {
        let (t1, have) = { let k = rng.range(1, 40) as usize; rand_toks(&mut rng, k, 0) };
        let (t2, _) = { let k = rng.range(1, 40) as usize; rand_toks(&mut rng, k, have) };
        let mut w = BitWriter::new();
        emit_fixed(&mut w, false, &t1);
        emit_fixed(&mut w, true, &t2);
        let mut all = t1.clone();
        all.extend_from_slice(&t2);
        let e = expand(&all);
        let s = w.bytes();
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C32", "fixed + fixed", s, &e, c);
    }
    d.row_end(b);

    // ---------------- C33: mixed fixed/dynamic pairs ----------------------
    let b = d.row_start("C33 two blocks, btype pairs (1,2) and (2,1)");
    for order in 0..2 {
        for _ in 0..20 {
            let (t1, have) = { let k = rng.range(1, 40) as usize; rand_toks(&mut rng, k, 0) };
            let (t2, _) = { let k = rng.range(1, 40) as usize; rand_toks(&mut rng, k, have) };
            let mut w = BitWriter::new();
            if order == 0 {
                emit_fixed(&mut w, false, &t1);
                emit_dynamic(&mut w, true, &t2, &DynOpts::default());
            } else {
                emit_dynamic(&mut w, false, &t1, &DynOpts::default());
                emit_fixed(&mut w, true, &t2);
            }
            let mut all = t1.clone();
            all.extend_from_slice(&t2);
            let e = expand(&all);
            let s = w.bytes();
            let c = Case::new(s.clone(), e.len() as i32);
            valid(
                &mut d,
                "C33",
                if order == 0 { "fixed + dynamic" } else { "dynamic + fixed" },
                s,
                &e,
                c,
            );
        }
    }
    d.row_end(b);

    // ---------------- C34: 3..8 blocks, random types ----------------------
    // Each block re-runs cp_build and overwrites s->lit / s->dst / s->lookup.
    let b = d.row_start("C34 3..8 blocks, random btype in {1,2} (cp_build re-run per block)");
    for _ in 0..30 {
        let nb = rng.range(3, 8) as usize;
        let mut w = BitWriter::new();
        let mut all: Vec<Tok> = Vec::new();
        let mut have = 0u32;
        let mut types = String::new();
        for i in 0..nb {
            let (t, h) = { let k = rng.range(0, 30) as usize; rand_toks(&mut rng, k, have) };
            have = h;
            let last = i == nb - 1;
            if rng.below(2) == 0 {
                emit_fixed(&mut w, last, &t);
                types.push('1');
            } else {
                emit_dynamic(&mut w, last, &t, &DynOpts::default());
                types.push('2');
            }
            all.extend_from_slice(&t);
        }
        let e = expand(&all);
        let s = w.bytes();
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C34", &format!("{nb} blocks types={types}"), s, &e, c);
    }
    d.row_end(b);

    // ---------------- C35: empty blocks (EOB only) ------------------------
    let b = d.row_start("C35 blocks containing only the end-of-block symbol");
    for nb in 1..=6usize {
        for kind in 0..3 {
            let mut w = BitWriter::new();
            for i in 0..nb {
                let last = i == nb - 1;
                match kind {
                    0 => emit_fixed(&mut w, last, &[]),
                    1 => {
                        emit_dynamic(&mut w, last, &[], &DynOpts::default());
                    }
                    _ => {
                        if i % 2 == 0 {
                            emit_fixed(&mut w, last, &[])
                        } else {
                            emit_dynamic(&mut w, last, &[], &DynOpts::default());
                        }
                    }
                }
            }
            let s = w.bytes();
            for ob in [0i32, 1, 32] {
                let c = Case::new(s.clone(), ob);
                valid(&mut d, "C35", &format!("{nb} empty blocks kind={kind} out_bytes={ob}"), s.clone(), &[], c);
            }
        }
    }
    d.row_end(b);

    // ---------------- C36: out_bytes exactly the decoded size -------------
    let b = d.row_start("C36 out_bytes == decoded size (tightest accepting bound)");
    for _ in 0..60 {
        let (t, _) = { let k = rng.range(1, 60) as usize; rand_toks(&mut rng, k, 0) };
        let e = expand(&t);
        let mut w = BitWriter::new();
        if rng.below(2) == 0 {
            emit_fixed(&mut w, true, &t);
        } else {
            emit_dynamic(&mut w, true, &t, &DynOpts::default());
        }
        let s = w.bytes();
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C36", &format!("exact fit {} bytes", e.len()), s, &e, c);
    }
    d.row_end(b);

    // ---------------- C37: out_bytes much larger than needed --------------
    let b = d.row_start("C37 out_bytes >> decoded size (tail of out must stay untouched)");
    for _ in 0..60 {
        let (t, _) = { let k = rng.range(1, 60) as usize; rand_toks(&mut rng, k, 0) };
        let e = expand(&t);
        let mut w = BitWriter::new();
        if rng.below(2) == 0 {
            emit_fixed(&mut w, true, &t);
        } else {
            emit_dynamic(&mut w, true, &t, &DynOpts::default());
        }
        let s = w.bytes();
        let slack = rng.range(1, 4096) as i32;
        for oa in 0..4usize {
            let c = Case::new(s.clone(), e.len() as i32 + slack)
                .out_align(oa)
                .out_fill(0xA7);
            valid(
                &mut d,
                "C37",
                &format!("decoded {} out_bytes {} out_align {oa}", e.len(), e.len() as i32 + slack),
                s.clone(),
                &e,
                c,
            );
        }
    }
    d.row_end(b);

    // ---------------- C38: consumer-mutated cp_len_base / cp_dist_base ----
    // Proves the Rust decoder really reads the exported (writable) tables
    // rather than baked-in constants.
    let b = d.row_start("C38 poked cp_len_base / cp_dist_base (still-valid values)");
    for &(new_len_base, new_dist_base) in &[(7u32, 2u32), (5, 3), (12, 4), (200, 1), (3, 4)] {
        // history "abcd", then length symbol 257 (0 extra bits) and distance
        // symbol 0 (0 extra bits), so the decoded length/distance come straight
        // from the poked table entries.
        let mut w = BitWriter::new();
        w.push(1, 1);
        w.push(1, 2);
        for i in 0..4u32 {
            emit_fixed_raw_sym(&mut w, 0x61 + i);
        }
        emit_fixed_raw_sym(&mut w, 257);
        emit_fixed_raw_dist(&mut w, 0, 0, 0);
        emit_fixed_raw_sym(&mut w, 256);
        let s = w.bytes();

        let mut exp = b"abcd".to_vec();
        for _ in 0..new_len_base {
            let x = exp[exp.len() - new_dist_base as usize];
            exp.push(x);
        }
        let case = Case::new(s.clone(), exp.len() as i32)
            .poke(GlobalPoke::LenBase(0, new_len_base))
            .poke(GlobalPoke::DistBase(0, new_dist_base));
        valid(
            &mut d,
            "C38",
            &format!("cp_len_base[0]={new_len_base} cp_dist_base[0]={new_dist_base}"),
            s.clone(),
            &exp,
            case,
        );
        // The poke must actually change the result, otherwise the row proves
        // nothing about where the table is read from.
        if new_len_base != 3 || new_dist_base != 1 {
            let base_case = Case::new(s.clone(), exp.len() as i32);
            let unpoked = common::run(&d.arena, &d.pair.c, &base_case);
            if unpoked.out[..exp.len()] == exp[..] {
                d.fail(format!(
                    "[C38] poke ({new_len_base},{new_dist_base}) had no observable effect"
                ));
            }
        }
    }
    d.row_end(b);

    // ---------------- C39: consumer-mutated cp_permutation_order ----------
    let b = d.row_start("C39 poked cp_permutation_order (different valid permutation)");
    {
        // A rotation of the default order: still a permutation of 0..18.
        let mut alt = PERM;
        alt.rotate_left(5);
        for _ in 0..12 {
            let alphabet: Vec<u8> = (0..rng.range(2, 40)).map(|_| rng.byte()).collect();
            let n = rng.range(10, 200) as usize;
            let toks: Vec<Tok> = (0..n)
                .map(|_| Tok::Lit(alphabet[rng.below(alphabet.len() as u32) as usize]))
                .collect();
            let o = DynOpts {
                perm: alt,
                full_hclen: true, // all 19 entries, so any permutation is legal
                ..DynOpts::default()
            };
            let mut w = BitWriter::new();
            emit_dynamic(&mut w, true, &toks, &o);
            let s = w.bytes();
            let e = expand(&toks);
            let mut case = Case::new(s.clone(), e.len() as i32);
            for (i, &p) in alt.iter().enumerate() {
                case = case.poke(GlobalPoke::PermutationOrder(i, p as u8));
            }
            valid(&mut d, "C39", "rotated permutation order", s.clone(), &e, case);

            // Without the poke the same stream must NOT decode to `e`.
            let unpoked = common::run(&d.arena, &d.pair.c, &Case::new(s.clone(), e.len() as i32));
            if unpoked.signal.is_none() && unpoked.ret == 1 && unpoked.out[..e.len()] == e[..] {
                d.fail("[C39] permutation poke had no observable effect".into());
            }
        }
    }
    d.row_end(b);

    // ---------------- C40: consumer-mutated cp_fixed_table ---------------
    let b = d.row_start("C40 poked cp_fixed_table (alternative complete 288/32 assignment)");
    {
        // 256 x 9 bits + 32 x 6 bits = 256/512 + 32/64 = 1.0 -> complete.
        let mut lit_lens = vec![9u8; 288];
        for i in 256..288 {
            lit_lens[i] = 6;
        }
        let dst_lens = vec![5u8; 32];
        assert!(is_complete(&lit_lens) && is_complete(&dst_lens));
        for _ in 0..15 {
            let (t, _) = { let k = rng.range(1, 50) as usize; rand_toks(&mut rng, k, 0) };
            let e = expand(&t);
            let mut w = BitWriter::new();
            emit_fixed_with_lens(&mut w, true, &t, &lit_lens, &dst_lens);
            let s = w.bytes();
            let mut case = Case::new(s.clone(), e.len() as i32);
            for i in 0..288 {
                case = case.poke(GlobalPoke::FixedTable(i, lit_lens[i]));
            }
            valid(&mut d, "C40", "alternative fixed table", s.clone(), &e, case);

            let unpoked = common::run(&d.arena, &d.pair.c, &Case::new(s.clone(), e.len() as i32));
            if unpoked.signal.is_none() && unpoked.ret == 1 && unpoked.out[..e.len()] == e[..] {
                d.fail("[C40] fixed-table poke had no observable effect".into());
            }
        }
    }
    d.row_end(b);

    // ---------------- C41: randomized valid-stream sweep ------------------
    let b = d.row_start("C41 randomized well-formed streams (blocks x types x alignments x sizes)");
    for _ in 0..400 {
        let nb = rng.range(1, 4) as usize;
        let mut w = BitWriter::new();
        let mut all: Vec<Tok> = Vec::new();
        let mut have = 0u32;
        for i in 0..nb {
            let (t, h) = { let k = rng.range(0, 40) as usize; rand_toks(&mut rng, k, have) };
            have = h;
            let last = i == nb - 1;
            if rng.below(2) == 0 {
                emit_fixed(&mut w, last, &t);
            } else {
                let o = DynOpts {
                    min_hlit: if rng.below(3) == 0 { 288 } else { 257 },
                    min_hdist: if rng.below(3) == 0 { 32 } else { 1 },
                    full_hclen: rng.below(2) == 0,
                    uniform_weights: rng.below(2) == 0,
                    no_rle: rng.below(4) == 0,
                    ..DynOpts::default()
                };
                emit_dynamic(&mut w, last, &t, &o);
            }
            all.extend_from_slice(&t);
        }
        let mut s = w.bytes();
        for _ in 0..rng.below(4) {
            s.push(rng.byte());
        }
        let e = expand(&all);
        let slack = rng.below(64) as i32;
        let case = Case::new(s.clone(), e.len() as i32 + slack)
            .in_align(rng.below(4) as usize)
            .out_align(rng.below(4) as usize)
            .out_fill(rng.byte());
        valid(&mut d, "C41", &format!("{nb} blocks, {} out bytes", e.len()), s, &e, case);
    }
    d.row_end(b);

    d.finish("Phase B (part 3: C32-C41)");
}

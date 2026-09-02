//! Phase B rows C01..C06 (exported writable tables) and C39 (real zlib streams).
//!
//! The table rows mutate process-wide globals inside both `.so`s, so they live
//! in their own test binary (own process) and run in a single `#[test]` so no
//! other test observes the mutated state.

mod common;

use common::deflate::*;
use common::rng::{Rng, SEED};
use common::*;

// ===========================================================================
// C01 / C02 — initial contents of the exported data objects
// ===========================================================================

fn c01_c02_initial_table_contents() {
    let p = load_pair();

    // C02: cp_error_reason starts NULL in both.
    assert_eq!(p.c.error(), None, "C02: C cp_error_reason not initially NULL");
    assert_eq!(
        p.rust.error(),
        None,
        "C02: Rust cp_error_reason not initially NULL"
    );

    // C01: byte-identical tables, and identical to the RFC 1951 values.
    assert_eq!(
        p.c.fixed_table(),
        p.rust.fixed_table(),
        "C01: cp_fixed_table differs"
    );
    assert_eq!(p.c.fixed_table(), fixed_table(), "C01: cp_fixed_table wrong");

    assert_eq!(
        p.c.permutation_order(),
        p.rust.permutation_order(),
        "C01: cp_permutation_order differs"
    );
    assert_eq!(
        p.c.permutation_order(),
        PERMUTATION.iter().map(|&x| x as u8).collect::<Vec<u8>>()
    );

    let mut lex = LEN_EXTRA.iter().map(|&x| x as u8).collect::<Vec<u8>>();
    lex.extend_from_slice(&[0, 0]);
    assert_eq!(
        p.c.len_extra_bits(),
        p.rust.len_extra_bits(),
        "C01: cp_len_extra_bits differs"
    );
    assert_eq!(p.c.len_extra_bits(), lex);

    let mut lb = LEN_BASE.to_vec();
    lb.extend_from_slice(&[0, 0]);
    assert_eq!(p.c.len_base(), p.rust.len_base(), "C01: cp_len_base differs");
    assert_eq!(p.c.len_base(), lb);

    let mut dex = DIST_EXTRA.iter().map(|&x| x as u8).collect::<Vec<u8>>();
    dex.extend_from_slice(&[0, 0]);
    assert_eq!(
        p.c.dist_extra_bits(),
        p.rust.dist_extra_bits(),
        "C01: cp_dist_extra_bits differs"
    );
    assert_eq!(p.c.dist_extra_bits(), dex);

    let mut db = DIST_BASE.to_vec();
    db.extend_from_slice(&[0, 0]);
    assert_eq!(
        p.c.dist_base(),
        p.rust.dist_base(),
        "C01: cp_dist_base differs"
    );
    assert_eq!(p.c.dist_base(), db);
}

// ===========================================================================
// C03..C06 — mutate the exported tables identically in both libraries
// ===========================================================================

struct Saved {
    fixed: Vec<u8>,
    perm: Vec<u8>,
    lex: Vec<u8>,
    lb: Vec<u32>,
    dex: Vec<u8>,
    db: Vec<u32>,
}

fn save(im: &Impl) -> Saved {
    Saved {
        fixed: im.fixed_table(),
        perm: im.permutation_order(),
        lex: im.len_extra_bits(),
        lb: im.len_base(),
        dex: im.dist_extra_bits(),
        db: im.dist_base(),
    }
}

fn restore(im: &Impl, s: &Saved) {
    im.set_fixed_table(&s.fixed);
    im.set_permutation_order(&s.perm);
    im.set_len_extra_bits(&s.lex);
    im.set_len_base(&s.lb);
    im.set_dist_extra_bits(&s.dex);
    im.set_dist_base(&s.db);
}

fn lits(bytes: &[u8]) -> Vec<Op> {
    bytes.iter().map(|&b| Op::Lit(b)).collect()
}

#[test]
fn c01_to_c06_tables() {
    c01_c02_initial_table_contents();
    let p = load_pair();
    let saved_c = save(&p.c);
    let saved_r = save(&p.rust);
    let mut rng = Rng::new(SEED ^ 0xC0FFEE);

    // -------------------------------------------------------------------
    // C03 — mutate cp_permutation_order and encode with the same order.
    // -------------------------------------------------------------------
    for rot in 1..19usize {
        let mut perm: Vec<u8> = PERMUTATION.iter().map(|&x| x as u8).collect();
        perm.rotate_left(rot);
        p.c.set_permutation_order(&perm);
        p.rust.set_permutation_order(&perm);
        assert_eq!(p.c.permutation_order(), p.rust.permutation_order());

        let alpha: Vec<u8> = (0..=255u8).collect();
        let n = 1 + rng.below(80);
        let mut ops: Vec<Op> = Vec::new();
        for _ in 0..n {
            ops.push(Op::Lit(alpha[rng.below(alpha.len())]));
        }
        let d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::all(), 257, 1);
        let perm_usize: Vec<usize> = perm.iter().map(|&x| x as usize).collect();
        let mut w = BitWriter::new();
        emit_dynamic_with_permutation(&mut w, true, &d, &ops, &perm_usize);
        let stream = w.finish();
        let expect = expand(&ops);
        let out_bytes = expect.len() + 8;
        let c = run_inflate(&p.c, &stream, 0, out_bytes, None);
        assert_eq!(c.ret, 1, "[C03 rot={rot}] C rejected: {c:?}");
        assert_eq!(&c.out[..expect.len()], &expect[..], "[C03 rot={rot}] self-check");
        diff_inflate(&p, &format!("C03 rot={rot}"), &stream, 0, out_bytes);
    }
    restore(&p.c, &saved_c);
    restore(&p.rust, &saved_r);

    // -------------------------------------------------------------------
    // C04 — mutate cp_len_base / cp_len_extra_bits.
    // -------------------------------------------------------------------
    for iter in 0..8usize {
        let mut lb = saved_c.lb.clone();
        let mut lex = saved_c.lex.clone();
        // Shift every base and shuffle a few extra-bit widths (still <= 5 so the
        // encoder's bit accounting stays inside cp_read_bits' limits).
        for i in 0..29 {
            lb[i] = lb[i] + (iter as u32 % 4) + 1;
        }
        for i in 0..29 {
            lex[i] = ((lex[i] as usize + iter) % 6) as u8;
        }
        p.c.set_len_base(&lb);
        p.rust.set_len_base(&lb);
        p.c.set_len_extra_bits(&lex);
        p.rust.set_len_extra_bits(&lex);
        assert_eq!(p.c.len_base(), p.rust.len_base());
        assert_eq!(p.c.len_extra_bits(), p.rust.len_extra_bits());

        // The stream is encoded with the *standard* widths, so the decoder now
        // reads a different number of bits: the two implementations must still
        // agree bit-for-bit (whatever they produce).
        let mut ops = lits(&rng.bytes(64));
        ops.push(Op::Match { len: 8, dist: 4 });
        ops.push(Op::Match { len: 20, dist: 1 });
        ops.extend(lits(&rng.bytes(4)));
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &ops);
        let stream = w.finish();
        diff_inflate(&p, &format!("C04 iter={iter}"), &stream, 0, 4096);
    }
    restore(&p.c, &saved_c);
    restore(&p.rust, &saved_r);

    // -------------------------------------------------------------------
    // C05 — mutate cp_dist_base / cp_dist_extra_bits.
    // -------------------------------------------------------------------
    for iter in 0..8usize {
        let mut db = saved_c.db.clone();
        let mut dex = saved_c.dex.clone();
        for i in 0..30 {
            db[i] = db[i].saturating_sub(iter as u32).max(1);
        }
        for i in 0..30 {
            dex[i] = ((dex[i] as usize + iter) % 14) as u8;
        }
        p.c.set_dist_base(&db);
        p.rust.set_dist_base(&db);
        p.c.set_dist_extra_bits(&dex);
        p.rust.set_dist_extra_bits(&dex);
        assert_eq!(p.c.dist_base(), p.rust.dist_base());
        assert_eq!(p.c.dist_extra_bits(), p.rust.dist_extra_bits());

        let mut ops = lits(&rng.bytes(200));
        ops.push(Op::Match { len: 6, dist: 40 });
        ops.push(Op::Match { len: 12, dist: 3 });
        ops.extend(lits(&rng.bytes(4)));
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &ops);
        let stream = w.finish();
        diff_inflate(&p, &format!("C05 iter={iter}"), &stream, 0, 8192);
    }
    restore(&p.c, &saved_c);
    restore(&p.rust, &saved_r);

    // -------------------------------------------------------------------
    // C06 — mutate cp_fixed_table (a permutation keeps the code complete).
    // -------------------------------------------------------------------
    let mut mutated = saved_c.fixed.clone();
    mutated[..288].reverse();
    assert!(mutated[256] != 0);
    for iter in 0..16usize {
        p.c.set_fixed_table(&mutated);
        p.rust.set_fixed_table(&mutated);
        assert_eq!(p.c.fixed_table(), p.rust.fixed_table());

        let alpha: Vec<u8> = (0..=255u8).collect();
        let n = 1 + rng.below(120);
        let mut ops: Vec<Op> = Vec::new();
        let mut outlen = 0usize;
        for _ in 0..n {
            if outlen >= 8 && rng.below(3) == 0 {
                let dist = 1 + rng.below(outlen.min(200));
                let mut dsym = 29usize;
                while DIST_BASE[dsym] as usize > dist {
                    dsym -= 1;
                }
                let dextra = dist as u32 - DIST_BASE[dsym];
                let lsym = 257 + rng.below(20) as u16;
                let eb = LEN_EXTRA[lsym as usize - 257];
                let lextra = if eb == 0 { 0 } else { rng.below(1 << eb) as u32 };
                ops.push(Op::Raw {
                    lsym,
                    lextra,
                    dsym: dsym as u16,
                    dextra,
                });
                outlen += (LEN_BASE[lsym as usize - 257] + lextra) as usize;
            } else {
                ops.push(Op::Lit(alpha[rng.below(alpha.len())]));
                outlen += 1;
            }
        }
        let expect = expand(&ops);
        let mut w = BitWriter::new();
        emit_fixed_with(&mut w, true, &ops, &mutated);
        let stream = w.finish();
        let out_bytes = expect.len() + 8;
        let c = run_inflate(&p.c, &stream, 0, out_bytes, None);
        assert_eq!(c.ret, 1, "[C06 iter={iter}] C rejected: {c:?}");
        assert_eq!(&c.out[..expect.len()], &expect[..], "[C06 iter={iter}] self-check");
        diff_inflate(&p, &format!("C06 iter={iter}"), &stream, 0, out_bytes);
    }
    restore(&p.c, &saved_c);
    restore(&p.rust, &saved_r);

    assert_eq!(p.c.fixed_table(), p.rust.fixed_table());
    assert_eq!(p.c.fixed_table(), fixed_table());
}

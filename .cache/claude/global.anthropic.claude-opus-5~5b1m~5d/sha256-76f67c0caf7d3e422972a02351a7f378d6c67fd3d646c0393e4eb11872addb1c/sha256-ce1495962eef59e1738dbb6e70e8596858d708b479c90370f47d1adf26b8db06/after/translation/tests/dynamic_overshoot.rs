//! `cp_dynamic`'s run-length loop can write past its local `uint8_t
//! lens[288 + 32]`.
//!
//! The loop condition is `n < nlit + ndst`, but a single code-length symbol 16 /
//! 17 / 18 writes 3..=138 entries, so the last run can overshoot by up to 137
//! bytes.  In the C those bytes land on the *other locals* of `cp_dynamic`'s
//! stack frame.  `objdump -d` on the reference object pins the -O0 frame down
//! exactly (offsets relative to `&lens[0]`, i.e. `%rbp-0x180`):
//!
//! | lens index | C object            | still used after the loop? |
//! |------------|---------------------|----------------------------|
//! | -8 ..   -1 | spilled `s` pointer | yes (`lens[-1]` is its top byte, always 0) |
//! |  0 .. 319  | `lens` itself       | yes                        |
//! | 320 .. 338 | `lenlens[19]`       | no                         |
//! | 348 .. 351 | `sym`               | no (reassigned each pass)  |
//! | 352 .. 355 | `nlen`              | no                         |
//! | 356 .. 359 | `ndst`              | **yes**                    |
//! | 360 .. 363 | `nlit`              | **yes**                    |
//! | 364 .. 375 | the three run counters | only inside their own run |
//! | 376 .. 379 | `n`                 | **yes** (the loop variable) |
//! | 380 .. 383 | the HCLEN loop's `i`| no                         |
//! | 384 .. 391 | saved `%rbp`        | on return                  |
//! | 392 .. 399 | return address      | on return                  |
//!
//! Every overshoot that stops before index 384 is fully determined, so the Rust
//! must reproduce it.  Overshoots that reach 384 smash the frame pointer /
//! return address; see the note at the end of `ERRORS.md`.

mod common;

use common::fork::*;
use common::*;

const NLIT: usize = 288;
const NDST: usize = 32;

/// Code-length symbol used to overshoot.
#[derive(Clone, Copy, Debug)]
enum Tail {
    /// symbol 17, 3..=10 zeros
    Zeros17(u32),
    /// symbol 18, 11..=138 zeros
    Zeros18(u32),
    /// symbol 16, 3..=6 copies of the previous entry
    Repeat16(u32),
}

/// Build a BTYPE=2 stream whose code-length program writes `lens[0..319]`
/// literally and then overshoots with `tail`.
fn overshoot_stream(tail: Tail, items: &[Item]) -> Vec<u8> {
    // Literal/length tree over all 288 symbols; distance tree over 0..=30 with
    // `dst_lens[31] == 0`, because `lens[319]` is overwritten by the tail.
    let lit_lens = balanced_lens(NLIT, &(0..NLIT).collect::<Vec<_>>());
    let mut dst_lens = balanced_lens(NDST, &(0..NDST - 1).collect::<Vec<_>>());
    dst_lens[NDST - 1] = 0;
    assert_eq!(lit_lens.len(), NLIT);
    assert_eq!(dst_lens.len(), NDST);

    let mut all = lit_lens.clone();
    all.extend_from_slice(&dst_lens);
    assert_eq!(all.len(), 320);
    assert_eq!(all[319], 0);

    // literal code-length symbols for indices 0..=318, then the overshooting tail
    let mut cl: Vec<ClSym> = all[..319].iter().map(|&v| ClSym::Lit(v)).collect();
    match tail {
        Tail::Zeros17(k) => {
            assert!((3..=10).contains(&k));
            cl.push(ClSym::Rep17(k));
        }
        Tail::Zeros18(k) => {
            assert!((11..=138).contains(&k));
            cl.push(ClSym::Rep18(k));
        }
        Tail::Repeat16(k) => {
            assert!((3..=6).contains(&k));
            cl.push(ClSym::Rep16(k));
        }
    }

    // code lengths for the code-length alphabet
    let (cl_lens, nlen) = {
        let mut used = [false; 19];
        for s in &cl {
            let idx = match *s {
                ClSym::Lit(v) => v as usize,
                ClSym::Rep16(_) => 16,
                ClSym::Rep17(_) => 17,
                ClSym::Rep18(_) => 18,
            };
            used[idx] = true;
        }
        let syms: Vec<usize> = (0..19).filter(|&i| used[i]).collect();
        let mut lens = [0u8; 19];
        let k = syms.len();
        let mut depth = 0u32;
        while (1usize << depth) < k {
            depth += 1;
        }
        if depth == 0 {
            lens[syms[0]] = 1;
        } else {
            let short = (1usize << depth) - k;
            for (i, &s) in syms.iter().enumerate() {
                lens[s] = if i < short { (depth - 1) as u8 } else { depth as u8 };
            }
        }
        let mut n = 4usize;
        for (pos, &pp) in PERMUTATION_ORDER.iter().enumerate() {
            if used[pp] {
                n = n.max(pos + 1);
            }
        }
        (lens, n)
    };

    let mut w = BitWriter::new();
    w.bit(1); // BFINAL
    w.bits_lsb(2, 2); // BTYPE = 2
    w.bits_lsb((NLIT - 257) as u32, 5);
    w.bits_lsb((NDST - 1) as u32, 5);
    w.bits_lsb((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.bits_lsb(cl_lens[PERMUTATION_ORDER[i]] as u32, 3);
    }
    let clh = Huff::new(cl_lens.to_vec());
    for s in &cl {
        match *s {
            ClSym::Lit(v) => clh.put(&mut w, v as usize),
            ClSym::Rep16(k) => {
                clh.put(&mut w, 16);
                w.bits_lsb(k - 3, 2);
            }
            ClSym::Rep17(k) => {
                clh.put(&mut w, 17);
                w.bits_lsb(k - 3, 3);
            }
            ClSym::Rep18(k) => {
                clh.put(&mut w, 18);
                w.bits_lsb(k - 11, 7);
            }
        }
    }

    // payload, encoded with the tree the decoder will actually have built
    let lit = Huff::new(lit_lens);
    let dst = Huff::new(dst_lens);
    write_items(&mut w, &lit, &dst, items);

    let mut stream = w.bytes;
    stream.extend_from_slice(&[0u8; 8]);
    stream
}

/// `n` after the tail, i.e. one past the last `lens` index written.
fn n_end(tail: Tail) -> usize {
    319 + match tail {
        Tail::Zeros17(k) | Tail::Zeros18(k) | Tail::Repeat16(k) => k as usize,
    }
}

/// Payload kinds, chosen so that each corrupted local is actually observable:
///   * literals only            -> observes `nlit` / `n` corruption
///   * literals + a match       -> observes `ndst` corruption too, because the
///                                 distance tree is only consulted for a match
fn payloads(rng: &mut Rng) -> Vec<Vec<Item>> {
    let mut v = Vec::new();
    let n = rng.range(1, 12) as usize;
    v.push((0..n).map(|_| Item::Lit(rng.u8())).collect());
    let prefix = rng.range(4, 20) as usize;
    let mut with_match: Vec<Item> = (0..prefix).map(|_| Item::Lit(rng.u8())).collect();
    let dist = rng.range(1, prefix as u32);
    with_match.push(Item::Match(rng.range(3, 30), dist));
    v.push(with_match);
    // distance 1 (memset arm) as well
    let mut d1: Vec<Item> = (0..3).map(|_| Item::Lit(rng.u8())).collect();
    d1.push(Item::Match(9, 1));
    v.push(d1);
    v
}

/// Run one overshoot shape through the child-process harness, since the C may
/// legitimately abort (a corrupted `nlit` leaves an empty literal tree).
fn check(tail: Tail, label: &str) {
    no_core_dumps();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xD00D ^ n_end(tail) as u64);
    let mut outcomes: std::collections::BTreeSet<String> = Default::default();
    for case in 0..1 {
        for (pi, items) in payloads(&mut rng).into_iter().enumerate() {
            let stream = overshoot_stream(tail, &items);
            for off in 0..2usize {
                let o = diff_fork_full(
                    &p,
                    &stream,
                    off,
                    stream.len() as i32,
                    4096,
                    512,
                    &format!("{label}/{case}/p{pi}/off{off}"),
                );
                outcomes.insert(match (o.signal, &o.assertion) {
                    (Some(6), Some(a)) => format!("SIGABRT {}", assertion_expr(a)),
                    (Some(14), _) => "SIGALRM (both spin forever)".to_string(),
                    (Some(sig), _) => format!("signal {sig}"),
                    (None, _) => {
                        // only the *kind* of outcome, not the buffer hashes
                        let rc = o
                            .summary
                            .as_deref()
                            .and_then(|t| t.split_whitespace().next())
                            .unwrap_or("rc=?")
                            .to_string();
                        format!("returned {rc}")
                    }
                });
            }
        }
    }
    eprintln!("{label}: n_end={} -> {outcomes:?}", n_end(tail));
}

/// Overshoot that stops inside `lenlens` — semantically invisible, but it must
/// still not change the result.
#[test]
fn ov01_into_lenlens() {
    for k in [3u32, 5, 10] {
        check(Tail::Zeros17(k), &format!("ov01/z17/{k}"));
    }
    for k in [3u32, 4, 6] {
        check(Tail::Repeat16(k), &format!("ov01/r16/{k}"));
    }
    for k in [11u32, 15, 19] {
        check(Tail::Zeros18(k), &format!("ov01/z18/{k}"));
    }
}

/// Overshoot reaching `sym` / `nlen` — also not read after the loop.
#[test]
fn ov02_into_sym_and_nlen() {
    for k in [30u32, 33, 34, 36, 37] {
        check(Tail::Zeros18(k), &format!("ov02/z18/{k}"));
    }
}

/// Overshoot that zeroes `ndst`, which IS used afterwards
/// (`cp_build(0, s->dst, lens + nlit, ndst)`).
#[test]
fn ov03_corrupts_ndst() {
    for k in [38u32, 39, 40, 41] {
        check(Tail::Zeros18(k), &format!("ov03/z18/{k}"));
    }
}

/// Overshoot that zeroes `nlit`, which IS used afterwards
/// (`cp_build(s, s->lit, lens, nlit)` and `lens + nlit`).
#[test]
fn ov04_corrupts_nlit() {
    for k in [42u32, 43, 44, 45] {
        check(Tail::Zeros18(k), &format!("ov04/z18/{k}"));
    }
}

/// Overshoot reaching the three run counters.
#[test]
fn ov05_into_run_counters() {
    for k in [46u32, 50, 54, 57] {
        check(Tail::Zeros18(k), &format!("ov05/z18/{k}"));
    }
}

/// Overshoot that rewrites the loop variable `n` itself, so the loop restarts
/// from a lower index.
#[test]
fn ov06_corrupts_n() {
    for k in [58u32, 59, 60, 61] {
        check(Tail::Zeros18(k), &format!("ov06/z18/{k}"));
    }
}

/// Overshoot reaching the HCLEN loop counter `i` (dead after the loop).
#[test]
fn ov07_into_hclen_counter() {
    for k in [62u32, 63, 64, 65] {
        check(Tail::Zeros18(k), &format!("ov07/z18/{k}"));
    }
}

/// An overshoot long enough to reach the saved frame pointer (`lens[384]`) is
/// **unreachable**: `lens[364..368]` is the symbol-18 run counter, so the run
/// zeroes its own counter, the following `--i` takes it negative and the loop
/// never terminates.  `lens[376..380]` is `n`, which the loop then resets to
/// ~257 on every pass, so `n` cycles in `257..=376` and never reaches 384.
///
/// Both implementations therefore spin forever and are killed by the same
/// `SIGALRM` budget.
#[test]
fn ov08_runaway_never_reaches_saved_rbp() {
    for k in [66u32, 74, 82, 100, 120, 138] {
        check(Tail::Zeros18(k), &format!("ov08/z18/{k}"));
    }
}

/// A clean BTYPE=2 stream (no overshoot), used to probe the *other* unchecked
/// index in `cp_dynamic`: `lenlens[cp_permutation_order[i]]`.
fn plain_dynamic_stream(items: &[Item]) -> Vec<u8> {
    let lit_lens = balanced_lens(NLIT, &(0..NLIT).collect::<Vec<_>>());
    let dst_lens = balanced_lens(NDST, &(0..NDST).collect::<Vec<_>>());
    let cl = cl_stream_literal(&lit_lens, &dst_lens);
    let (cl_lens, nlen) = cl_lens_for(&cl);
    let mut w = BitWriter::new();
    write_dynamic_block(
        &mut w, true, &lit_lens, &dst_lens, &cl, &cl_lens, nlen, &PERMUTATION_ORDER, items,
    );
    let mut stream = w.bytes;
    stream.extend_from_slice(&[0u8; 8]);
    stream
}

/// `cp_dynamic` writes `lenlens[cp_permutation_order[i]]` for `i < nlen` with no
/// range check on the table entry.  `cp_permutation_order` is an exported,
/// writable global, so an entry `> 18` makes the C write past its 19-byte local
/// `lenlens` — which sits at `%rbp-0x40`, so slots `19..=63` land on
/// `cp_dynamic`'s own locals (`sym`, `nlen`, `ndst`, `nlit`, the run counters,
/// `n`, `i`) and are fully determined.
///
/// Slots `>= 64` reach the saved `%rbp` and the caller's frame; those are noted
/// as out of scope in `ERRORS.md` and deliberately not exercised.
#[test]
fn ov09_permutation_order_out_of_range() {
    no_core_dumps();
    let p = pair();
    let pc: *mut u8 = p.c.data(b"cp_permutation_order\0");
    let pr: *mut u8 = p.rs.data(b"cp_permutation_order\0");
    let old = unsafe { std::slice::from_raw_parts(pc, 19).to_vec() };

    let mut rng = Rng::new(SEED ^ 0x9E97);
    let mut outcomes: std::collections::BTreeSet<String> = Default::default();
    for slot in [19u8, 20, 27, 28, 31, 32, 36, 40, 44, 48, 55, 60, 63] {
        for pos in [0usize, 3, 8] {
            let mut perm = old.clone();
            perm[pos] = slot;
            unsafe {
                std::ptr::copy_nonoverlapping(perm.as_ptr(), pc, 19);
                std::ptr::copy_nonoverlapping(perm.as_ptr(), pr, 19);
            }
            let items: Vec<Item> = (0..rng.range(1, 8)).map(|_| Item::Lit(rng.u8())).collect();
            let stream = plain_dynamic_stream(&items);
            let o = diff_fork_full(
                &p,
                &stream,
                0,
                stream.len() as i32,
                4096,
                256,
                &format!("ov09/slot{slot}/pos{pos}"),
            );
            outcomes.insert(match (o.signal, &o.assertion) {
                (Some(6), Some(a)) => format!("SIGABRT {}", assertion_expr(a)),
                (Some(14), _) => "SIGALRM (both spin forever)".to_string(),
                (Some(sig), _) => format!("signal {sig}"),
                (None, _) => "returned".to_string(),
            });
        }
    }
    unsafe {
        std::ptr::copy_nonoverlapping(old.as_ptr(), pc, 19);
        std::ptr::copy_nonoverlapping(old.as_ptr(), pr, 19);
    }
    eprintln!("ov09: {outcomes:?}");

    // the tables really were restored, and a plain stream works again
    let items: Vec<Item> = (0..5).map(|i| Item::Lit(i as u8 * 31)).collect();
    let stream = plain_dynamic_stream(&items);
    let mut expect = Vec::new();
    expand(&items, &mut expect);
    diff_inflate_expect(&p, &stream, &expect, "ov09/restored");
}

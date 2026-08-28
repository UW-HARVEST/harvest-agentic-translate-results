//! Out-of-bounds reads of the exported tables.
//!
//! `cp_block` indexes `cp_len_extra_bits` / `cp_len_base` with `symbol - 257`
//! and `cp_dist_extra_bits` / `cp_dist_base` with `distance_symbol`, neither of
//! which is range-checked.  Both can exceed the array bounds, in which case the
//! C reads whatever the linker put next in `.data`.  The Rust translation must
//! reproduce the C's `.data` image, not its own.
//!
//! Reachability (derived from the C):
//!   * `cp_decode` binary-searches `tree[0 .. hi)`.  `search` is always
//!     `>= 0xFFFF` and `tree[0]` is always `<= (287 << 4) | 15 = 0x120F`, so
//!     `lo` can only stay `0` when `hi == 0` — an *empty* tree.
//!   * A dynamic block may legally declare `HDIST` distance codes and then give
//!     all of them code length 0, so `cp_build` returns `0` and `s->ndst == 0`.
//!   * `cp_decode(s, s->dst, 0)` then reads `s->dst[-1]`, which is
//!     `s->lit[287]` — a *well formed* entry whenever the literal tree uses all
//!     288 symbols.  Its symbol field is `287`, so `cp_dist_extra_bits[287]` and
//!     `cp_dist_base[287]` are read, 255 and 255 entries past the end.

mod common;

use common::*;

/// Build the exact stream described in the module docs.
///
/// Returns `(stream, n_literals, length_of_match)`.
fn empty_dist_tree_stream(nlits: usize, len_idx: usize, len_extra: u32) -> (Vec<u8>, usize, u32) {
    // literal/length tree over *all* 288 symbols, so slot 287 is populated
    let lit_lens = balanced_lens(288, &(0..288).collect::<Vec<_>>());
    let lit = Huff::new(lit_lens.clone());
    // canonical assignment must put symbol 287 in the last slot with the
    // all-ones code of the maximum length
    let max_len = *lit_lens.iter().max().unwrap();
    assert_eq!(lit_lens[287], max_len);
    let code287 = lit.codes[287];
    assert_eq!(code287, (1u32 << max_len) - 1, "symbol 287 must own the all-ones code");

    // one declared distance code, given length 0 => cp_build returns 0
    let dst_lens = vec![0u8; 1];

    let mut all = lit_lens.clone();
    all.extend_from_slice(&dst_lens);
    let cl: Vec<ClSym> = all.iter().map(|&v| ClSym::Lit(v)).collect();
    let (cl_lens, nlen) = cl_lens_for(&cl);

    let mut w = BitWriter::new();
    w.bit(1); // BFINAL
    w.bits_lsb(2, 2); // BTYPE = 2 (dynamic)
    w.bits_lsb((288 - 257) as u32, 5); // HLIT
    w.bits_lsb((1 - 1) as u32, 5); // HDIST
    w.bits_lsb((nlen - 4) as u32, 4); // HCLEN
    for i in 0..nlen {
        w.bits_lsb(cl_lens[PERMUTATION_ORDER[i]] as u32, 3);
    }
    let clh = Huff::new(cl_lens.to_vec());
    for s in &cl {
        match *s {
            ClSym::Lit(v) => clh.put(&mut w, v as usize),
            _ => unreachable!(),
        }
    }

    // payload: `nlits` literals, then a length symbol, then the bits of
    // lit[287]'s code where the distance code would normally go.
    for i in 0..nlits {
        lit.put(&mut w, (i & 0xFF) as usize);
    }
    lit.put(&mut w, 257 + len_idx);
    w.bits_lsb(len_extra, LEN_EXTRA[len_idx] as u32);
    w.code(code287, max_len as u32); // consumed by cp_decode(s->dst, 0)
    lit.put(&mut w, 256); // end of block

    let mut stream = w.bytes;
    stream.extend_from_slice(&[0u8; 8]);
    (stream, nlits, LEN_BASE[len_idx] + len_extra)
}

/// The C reads `cp_dist_extra_bits[287]` / `cp_dist_base[287]`; whatever bytes
/// those land on, the Rust must read the same ones.
#[test]
fn oob01_empty_distance_tree_reads_past_dist_tables() {
    let p = pair();
    for (len_idx, len_extra) in [(0usize, 0u32), (3, 0), (7, 0), (12, 2), (20, 9)] {
        for nlits in [1usize, 4, 9] {
            let (stream, n, mlen) = empty_dist_tree_stream(nlits, len_idx, len_extra);
            for out_slack in [0usize, 8] {
                let out_len = n + mlen as usize + out_slack;
                let (rc, _) = diff_inflate(
                    &p,
                    &stream,
                    0,
                    out_len,
                    0,
                    &format!("oob01/li{len_idx}/x{len_extra}/n{nlits}/s{out_slack}"),
                );
                // The C's `cp_dist_extra_bits[287]` and `cp_dist_base[287]` both
                // land on zero bytes (past `.bss`, inside the last mapped page),
                // so `backwards_distance == 0`: the bound checks pass and the
                // copy loop writes each byte onto itself.
                assert_eq!(rc, 1, "oob01: C succeeds with backwards_distance == 0");
            }
        }
    }
}

/// Same shape, but drive every input alignment and a range of output sizes so
/// the (mis)behaviour is compared broadly rather than at one point.
#[test]
fn oob02_empty_distance_tree_matrix() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x0B0B);
    for case in 0..80 {
        let len_idx = rng.below(29) as usize;
        let nx = LEN_EXTRA[len_idx] as u32;
        let len_extra = if nx == 0 { 0 } else { rng.below(1u32 << nx) };
        let nlits = rng.range(1, 16) as usize;
        let (stream, n, mlen) = empty_dist_tree_stream(nlits, len_idx, len_extra);
        let out_len = n + mlen as usize + rng.below(16) as usize;
        for off in 0..4usize {
            diff_inflate(&p, &stream, off, out_len, 0, &format!("oob02/{case}/off{off}"));
        }
    }
}

/// A *too small* output buffer with the same stream, so the length check in
/// `cp_block` fires while `backwards_distance` is still the out-of-bounds value.
#[test]
fn oob03_empty_distance_tree_out_too_small() {
    let p = pair();
    for (len_idx, len_extra) in [(5usize, 0u32), (16, 3)] {
        let (stream, n, mlen) = empty_dist_tree_stream(6, len_idx, len_extra);
        let out_len = n + (mlen as usize) - 1;
        let (rc, _) = diff_inflate(&p, &stream, 0, out_len, 0, "oob03");
        assert_eq!(rc, 0);
        assert_eq!(
            p.c.error_reason().map(|v| String::from_utf8_lossy(&v).into_owned()),
            Some("Attempted to overwrite out buffer while outputting a string.".to_string())
        );
    }
}

/// `cp_build`'s first loop is `for (n = 0; n < sym_count; n++) counts[lens[n]]++;`
/// with `int counts[16]`, so a code length `>= 16` increments *past* the array
/// before the `assert(len < 16)` in the second loop ever runs.  The Rust aborts
/// at the counting loop instead; this test proves the two are observationally
/// identical (same signal, same diagnostic) across the whole `uint8_t` range and
/// at several positions in the table.
#[test]
fn oob04_code_length_ge_16_sweep() {
    use common::fork::*;
    no_core_dumps();
    let p = pair();
    let tc: *mut u8 = p.c.data(b"cp_fixed_table\0");
    let tr: *mut u8 = p.rs.data(b"cp_fixed_table\0");
    let old = unsafe { std::slice::from_raw_parts(tc, 320).to_vec() };

    // BFINAL=1, BTYPE=1 -> cp_fixed -> cp_build over the tampered table
    let mut stream = vec![0x03u8];
    stream.extend_from_slice(&[0u8; 15]);

    let mut outcomes: std::collections::BTreeSet<String> = Default::default();
    for &bad in &[16u8, 17, 18, 20, 24, 31, 32, 40, 64, 100, 127, 128, 200, 254, 255] {
        for &pos in &[0usize, 1, 143, 144, 255, 287, 288, 300, 319] {
            let mut t = old.clone();
            t[pos] = bad;
            unsafe {
                std::ptr::copy_nonoverlapping(t.as_ptr(), tc, 320);
                std::ptr::copy_nonoverlapping(t.as_ptr(), tr, 320);
            }
            let o = diff_fork_full(
                &p,
                &stream,
                0,
                stream.len() as i32,
                4096,
                64,
                &format!("oob04/bad{bad}/pos{pos}"),
            );
            outcomes.insert(match (o.signal, &o.assertion) {
                (Some(6), Some(a)) => format!("SIGABRT {}", assertion_expr(a)),
                (Some(sig), _) => format!("signal {sig}"),
                (None, _) => "returned".to_string(),
            });
        }
    }
    unsafe {
        std::ptr::copy_nonoverlapping(old.as_ptr(), tc, 320);
        std::ptr::copy_nonoverlapping(old.as_ptr(), tr, 320);
    }
    eprintln!("oob04: {outcomes:?}");
    assert!(
        outcomes.iter().any(|o| o.contains("len < 16")),
        "expected the `len < 16` assertion to be reached, got {outcomes:?}"
    );
}

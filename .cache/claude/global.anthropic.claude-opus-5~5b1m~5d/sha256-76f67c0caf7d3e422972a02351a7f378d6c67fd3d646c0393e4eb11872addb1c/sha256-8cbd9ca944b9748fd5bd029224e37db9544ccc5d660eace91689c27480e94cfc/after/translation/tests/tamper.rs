//! `CONFIGS.md` rows 35 and 39 — the exported tables are *mutable* globals that
//! the algorithm reads on every call, so a consumer permuting them is a valid
//! configuration. Run in a dedicated test binary (its own process) so the
//! mutation cannot leak into the other test files.

mod common;

use common::deflate::*;
use common::*;
use std::ffi::c_void;

const SEED: u64 = 0x7A_9BE1;

fn run(lib: &Lib, input: &[u8], out_len: usize) -> (i32, Vec<u8>) {
    let mut inbuf = AlignedBuf::new(input, 0);
    let mut out = vec![0xCDu8; out_len + 64];
    let ret = unsafe {
        (lib.cp_inflate())(
            inbuf.ptr() as *mut c_void,
            input.len() as i32,
            out.as_mut_ptr() as *mut c_void,
            out_len as i32,
        )
    };
    (ret, out)
}

// --------------------------------------------------------------------------
// row 35 — a permuted (but still complete) cp_fixed_table
// --------------------------------------------------------------------------

#[test]
fn row35_permuted_fixed_table() {
    let p = pair();
    let rng = Rng::new(SEED ^ 35);

    // reverse the 288 literal/length code lengths: the multiset is unchanged, so
    // the code stays complete, but every symbol gets a different code
    let original = p.c.data_u8(b"cp_fixed_table", 320);
    assert_eq!(original, p.rs.data_u8(b"cp_fixed_table", 320));
    let mut permuted = original.clone();
    permuted[..288].reverse();
    assert!(kraft_is_complete(&permuted[..288]));

    for lib in [&p.c, &p.rs] {
        let base = lib.data_ptr_u8(b"cp_fixed_table");
        unsafe {
            std::ptr::copy_nonoverlapping(permuted.as_ptr(), base, 320);
        }
    }

    let litcodes = canonical_codes(&permuted[..288]);
    let distcodes = canonical_codes(&permuted[288..]);

    for i in 0..200 {
        let n = rng.range(1, 60);
        let data = rng.bytes(n);
        let mut toks: Vec<Tok> = data.iter().copied().map(Tok::Lit).collect();
        if n >= 4 {
            toks.push(Tok::Match { len: rng.range(3, 40) as u32, dist: rng.range(1, n) as u32 });
        }
        let expected = expand(&toks);

        let mut w = BitWriter::new();
        w.bits(1, 1); // BFINAL
        w.bits(1, 2); // BTYPE = 01 (fixed) -> uses cp_fixed_table
        for t in &toks {
            match *t {
                Tok::Lit(b) => {
                    let (c, l) = litcodes[b as usize];
                    w.code(c, l);
                }
                Tok::Match { len, dist } => {
                    let (ls, lex, lval) = encode_length(len);
                    let (c, l) = litcodes[ls as usize];
                    w.code(c, l);
                    w.bits(lval, lex);
                    let (ds, dex, dval) = encode_distance(dist);
                    let (c, l) = distcodes[ds as usize];
                    w.code(c, l);
                    w.bits(dval, dex);
                }
            }
        }
        let (c, l) = litcodes[256];
        w.code(c, l);
        let stream = w.finish();

        let (rc, oc) = run(&p.c, &stream, expected.len());
        let (rr, or) = run(&p.rs, &stream, expected.len());
        assert_eq!(rc, rr, "row35 #{i}: return differs");
        assert_eq!(oc, or, "row35 #{i}: output differs");
        assert_eq!(p.c.error_reason(), p.rs.error_reason(), "row35 #{i}: reason differs");
        assert_eq!(rc, 1, "row35 #{i}: expected success");
        assert_eq!(&oc[..expected.len()], &expected[..], "row35 #{i}: payload");
    }

    // restore, and prove the untampered table still works
    for lib in [&p.c, &p.rs] {
        let base = lib.data_ptr_u8(b"cp_fixed_table");
        unsafe {
            std::ptr::copy_nonoverlapping(original.as_ptr(), base, 320);
        }
    }
    let data = b"restored".to_vec();
    let stream = fixed_literal_stream(&data);
    let (rc, oc) = run(&p.c, &stream, data.len());
    let (rr, or) = run(&p.rs, &stream, data.len());
    assert_eq!((rc, &oc), (rr, &or));
    assert_eq!(&oc[..data.len()], &data[..]);
}

// --------------------------------------------------------------------------
// row 39 — a permuted cp_permutation_order (dynamic header transmission order)
// --------------------------------------------------------------------------

#[test]
fn row39_permuted_permutation_order() {
    let p = pair();
    let rng = Rng::new(SEED ^ 39);

    let original = p.c.data_u8(b"cp_permutation_order", 19);
    assert_eq!(original, p.rs.data_u8(b"cp_permutation_order", 19));

    // reverse the order in which the 19 code-length code lengths arrive
    let mut permuted = original.clone();
    permuted.reverse();
    for lib in [&p.c, &p.rs] {
        let base = lib.data_ptr_u8(b"cp_permutation_order");
        unsafe {
            std::ptr::copy_nonoverlapping(permuted.as_ptr(), base, 19);
        }
    }

    for i in 0..80 {
        let n = rng.range(1, 80);
        let toks: Vec<Tok> = rng
            .bytes_alphabet(n, rng.range(2, 30))
            .into_iter()
            .map(Tok::Lit)
            .collect();
        let spec = dyn_spec_for(&toks, balanced_lengths, true);
        let expected = expand(&toks);

        // Build the dynamic block, but transmit the code-length code lengths in
        // the *permuted* order and with HCLEN == 19 so every slot is sent.
        let mut seq = spec.litlens.clone();
        seq.extend_from_slice(&spec.distlens);
        let items = rle_code_lengths(&seq, spec.rle);
        let mut used = [false; 19];
        for &(s, _, _) in &items {
            used[s as usize] = true;
        }
        let cl_idx: Vec<usize> = (0..19).filter(|&k| used[k]).collect();
        let cl_assigned = balanced_lengths(cl_idx.len());
        let mut cl_lens = [0u8; 19];
        for (k, &idx) in cl_idx.iter().enumerate() {
            cl_lens[idx] = cl_assigned[k];
        }

        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(2, 2);
        w.bits((spec.litlens.len() - 257) as u32, 5);
        w.bits((spec.distlens.len() - 1) as u32, 5);
        w.bits((19 - 4) as u32, 4);
        for k in 0..19 {
            w.bits(cl_lens[permuted[k] as usize] as u32, 3);
        }
        let cl_codes = canonical_codes(&cl_lens);
        for &(sym, ex, val) in &items {
            let (c, l) = cl_codes[sym as usize];
            w.code(c, l);
            w.bits(val, ex);
        }
        let litc = canonical_codes(&spec.litlens);
        for t in &toks {
            match *t {
                Tok::Lit(b) => {
                    let (c, l) = litc[b as usize];
                    w.code(c, l);
                }
                Tok::Match { .. } => unreachable!(),
            }
        }
        let (c, l) = litc[256];
        w.code(c, l);
        let stream = w.finish();

        let (rc, oc) = run(&p.c, &stream, expected.len());
        let (rr, or) = run(&p.rs, &stream, expected.len());
        assert_eq!(rc, rr, "row39 #{i}: return differs");
        assert_eq!(oc, or, "row39 #{i}: output differs");
        assert_eq!(rc, 1, "row39 #{i}: expected success");
        assert_eq!(&oc[..expected.len()], &expected[..], "row39 #{i}: payload");
    }

    for lib in [&p.c, &p.rs] {
        let base = lib.data_ptr_u8(b"cp_permutation_order");
        unsafe {
            std::ptr::copy_nonoverlapping(original.as_ptr(), base, 19);
        }
    }
}

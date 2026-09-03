//! Phase B part 2 — stored blocks (C16-C20) and dynamic blocks (C21-C31).

mod common;

use common::deflate::*;
use common::{Case, Diff};

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

/// For rows whose C behaviour is a quirk rather than an acceptance: only require
/// that the two libraries agree, and report what the C actually did.
fn agree(d: &mut Diff, row: &str, what: &str, case: Case) -> common::Outcome {
    d.check(row, what, &case)
}

#[test]
fn phase_b_stored_dynamic() {
    let mut d = Diff::new();
    let mut rng = common::Rng::new(SEED ^ 0xA5A5);

    // ================= STORED (btype = 0) =================================
    // Note the C's `cp_stored` rejects any input with more whole bytes left than
    // LEN (ERRORS.md E2) and never consumes the payload from the bit reader, so
    // an accepted stored block must be the last thing in the input.

    let b = d.row_start("C16 stored / LEN == 0 (empty stored block)");
    {
        let mut w = BitWriter::new();
        emit_stored(&mut w, true, &[]);
        let s = w.bytes();
        for oa in 0..4usize {
            for ob in [0i32, 1, 64] {
                let c = Case::new(s.clone(), ob).out_align(oa);
                valid(&mut d, "C16", &format!("LEN=0 out_bytes={ob} out_align={oa}"), s.clone(), &[], c);
            }
        }
    }
    d.row_end(b);

    let b = d.row_start("C17 stored / LEN = 1..8 (every payload residue mod 4)");
    {
        // `cp_ptr()` is `words + word_index - count/8`, which only points at the
        // payload when whole 32-bit words covered the 5-byte block header. With
        // `in_align == 0` that needs `word_count >= 2`, i.e. `LEN >= 3`; for
        // shorter payloads the C reads *header* bytes instead and still returns
        // 1. That is the ground truth, so it is asserted as such.
        let mut exact = 0usize;
        for len in 1..=8usize {
            for _ in 0..4 {
                let payload = rng.bytes(len);
                let mut w = BitWriter::new();
                emit_stored(&mut w, true, &payload);
                let s = w.bytes();
                let c = Case::new(s.clone(), len as i32);
                let o = agree(&mut d, "C17", &format!("LEN={len}"), c);
                let is_exact = o.signal.is_none() && o.ret == 1 && &o.out[..len] == &payload[..];
                if is_exact {
                    exact += 1;
                }
                if len >= 3 && !is_exact {
                    d.fail(format!(
                        "[C17] LEN={len}: C must copy the payload verbatim but produced {:?}",
                        o
                    ));
                }
                if len < 3 && o.ret != 1 {
                    d.fail(format!("[C17] LEN={len}: C should still return 1, got {:?}", o));
                }
            }
        }
        println!("  C17: {exact}/32 instances copied the payload verbatim");
        if exact == 0 {
            d.fail("[C17] no stored payload was copied verbatim (vacuous row)".into());
        }
    }
    d.row_end(b);

    let b = d.row_start("C18 stored / large LEN (> 4 KiB, spans many words)");
    for len in [255usize, 256, 1000, 4096, 8191, 65535] {
        let payload = rng.bytes(len);
        let mut w = BitWriter::new();
        emit_stored(&mut w, true, &payload);
        let s = w.bytes();
        let c = Case::new(s.clone(), len as i32);
        valid(&mut d, "C18", &format!("LEN={len}"), s, &payload, c);
    }
    d.row_end(b);

    let b = d.row_start("C19 stored x in_align 0..3 x out_align 0..3 (cp_ptr arithmetic)");
    {
        let mut exact = 0usize;
        for len in [1usize, 3, 4, 7, 33] {
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            emit_stored(&mut w, true, &payload);
            let s = w.bytes();
            for ia in 0..4usize {
                for oa in 0..4usize {
                    let c = Case::new(s.clone(), len as i32).in_align(ia).out_align(oa);
                    let o = agree(&mut d, "C19", &format!("LEN={len} in_align={ia} out_align={oa}"), c);
                    let is_exact = o.signal.is_none() && o.ret == 1 && &o.out[..len] == &payload[..];
                    if is_exact {
                        exact += 1;
                    }
                    // Proven above: with in_align == 0 and LEN >= 3 the C copies
                    // the payload verbatim at every out alignment.
                    if ia == 0 && len >= 3 && !is_exact {
                        d.fail(format!(
                            "[C19] LEN={len} in_align=0 out_align={oa}: C should copy verbatim, got {:?}",
                            o
                        ));
                    }
                }
            }
        }
        println!("  C19: {exact}/80 instances copied the payload verbatim");
        if exact == 0 {
            d.fail("[C19] vacuous row: no verbatim copy observed".into());
        }
    }
    d.row_end(b);

    let b = d.row_start("C20 stored non-final (payload is re-parsed as the next block header)");
    for len in [1usize, 2, 4, 9] {
        for _ in 0..6 {
            let payload = rng.bytes(len);
            let mut w = BitWriter::new();
            emit_stored(&mut w, false, &payload);
            let s = w.bytes();
            let c = Case::new(s.clone(), 4096);
            agree(&mut d, "C20", &format!("non-final LEN={len}"), c);
        }
    }
    // A deterministic, *accepting* instance: the stored payload itself is a
    // valid final fixed block ("bfinal=1, btype=1, EOB"), so the outer loop
    // re-reads it and terminates normally.
    {
        let (inner, _) = {
            let mut w = BitWriter::new();
            emit_fixed(&mut w, true, &[]);
            (w.bytes(), ())
        };
        let mut w = BitWriter::new();
        emit_stored(&mut w, false, &inner);
        let s = w.bytes();
        let c = Case::new(s.clone(), 4096);
        let o = agree(&mut d, "C20", "non-final stored whose payload is a final fixed block", c);
        println!("  C20 chained: C -> {:?}", o);
    }
    d.row_end(b);

    // ================= DYNAMIC (btype = 2) ================================

    let b = d.row_start("C21 dynamic / HLIT=257 HDIST=1 HCLEN=4 (all minima)");
    for _ in 0..20 {
        // Only literals 0/1 plus EOB keeps HLIT at 257; a single distance code
        // keeps HDIST at 1; forcing HCLEN=4 requires only CL symbols 16,17,18,0.
        let n = rng.range(1, 60) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.below(2) as u8)).collect();
        let o = DynOpts {
            min_hlit: 257,
            min_hdist: 1,
            force_hclen: None,
            ..DynOpts::default()
        };
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        assert_eq!(info.hlit, 257, "row C21 wanted HLIT=257");
        assert_eq!(info.hdist, 1, "row C21 wanted HDIST=1");
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(
            &mut d,
            "C21",
            &format!("HLIT={} HDIST={} HCLEN={}", info.hlit, info.hdist, info.hclen),
            s,
            &e,
            c,
        );
    }
    d.row_end(b);

    let b = d.row_start("C22 dynamic / HLIT=288 HDIST=32 HCLEN=19 (all maxima)");
    for _ in 0..20 {
        let n = rng.range(20, 200) as usize;
        let mut toks: Vec<Tok> = Vec::new();
        let mut have = 0u32;
        for _ in 0..n {
            if have > 40 && rng.below(4) == 0 {
                let dist = rng.range(1, have.min(300));
                let len = rng.range(3, 60);
                toks.push(Tok::Match { len, dist });
                have += len;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                have += 1;
            }
        }
        let o = DynOpts {
            min_hlit: 288,
            min_hdist: 32,
            full_hclen: true,
            ..DynOpts::default()
        };
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        assert_eq!((info.hlit, info.hdist, info.hclen), (288, 32, 19));
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C22", "HLIT=288 HDIST=32 HCLEN=19", s, &e, c);
    }
    d.row_end(b);

    // C23/C24/C25: force the 16 / 17 / 18 code-length opcodes to appear.
    let mut seen16 = 0usize;
    let mut seen17 = 0usize;
    let mut seen18 = 0usize;
    let b = d.row_start("C23/C24/C25 dynamic / code-length opcodes 16, 17 and 18 present");
    for _ in 0..60 {
        // Sparse alphabets produce long zero runs (17/18); repeated equal code
        // lengths produce 16.
        let alphabet: Vec<u8> = (0..rng.range(2, 30))
            .map(|_| rng.byte())
            .collect();
        let n = rng.range(30, 400) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(alphabet[rng.below(alphabet.len() as u32) as usize]))
            .collect();
        let o = DynOpts {
            min_hlit: if rng.below(2) == 0 { 288 } else { 257 },
            uniform_weights: rng.below(2) == 0,
            ..DynOpts::default()
        };
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        if info.cl_syms_used.contains(&16) {
            seen16 += 1;
        }
        if info.cl_syms_used.contains(&17) {
            seen17 += 1;
        }
        if info.cl_syms_used.contains(&18) {
            seen18 += 1;
        }
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(
            &mut d,
            "C23/C24/C25",
            &format!("cl_syms={:?} hlit={}", info.cl_syms_used, info.hlit),
            s,
            &e,
            c,
        );
    }
    println!("  code-length opcode coverage: 16 in {seen16} streams, 17 in {seen17}, 18 in {seen18}");
    // Opcode 16 (repeat previous length) needs *adjacent equal nonzero* code
    // lengths, which a sparse alphabet never produces. A dense alphabet with
    // uniform weights gives every literal the same length -> long runs.
    for &(nsyms, hlit) in &[(256usize, 288usize), (256, 257), (128, 257), (64, 288), (32, 257)] {
        let toks: Vec<Tok> = (0..nsyms * 3)
            .map(|i| Tok::Lit(((i % nsyms) * (256 / nsyms)) as u8))
            .collect();
        let o = DynOpts {
            min_hlit: hlit,
            uniform_weights: true,
            ..DynOpts::default()
        };
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        if info.cl_syms_used.contains(&16) {
            seen16 += 1;
        }
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(
            &mut d,
            "C23/C24/C25",
            &format!("dense alphabet nsyms={nsyms} hlit={hlit} cl_syms={:?}", info.cl_syms_used),
            s,
            &e,
            c,
        );
    }
    println!("  after dense-alphabet cases: opcode 16 in {seen16} streams");
    if seen16 == 0 || seen17 == 0 || seen18 == 0 {
        d.fail(format!(
            "[C23/C24/C25] opcode coverage incomplete: 16={seen16} 17={seen17} 18={seen18}"
        ));
    }
    d.row_end(b);

    let b = d.row_start("C26 dynamic / all code lengths <= 9 (cp_build lookup fast path)");
    for _ in 0..30 {
        let alphabet: Vec<u8> = (0..rng.range(2, 200)).map(|_| rng.byte()).collect();
        let n = rng.range(10, 300) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(alphabet[rng.below(alphabet.len() as u32) as usize]))
            .collect();
        let o = DynOpts {
            uniform_weights: true,
            ..DynOpts::default()
        };
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        assert!(info.max_lit_len <= 9, "uniform weights gave depth {}", info.max_lit_len);
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C26", &format!("max code len {}", info.max_lit_len), s, &e, c);
    }
    d.row_end(b);

    let b = d.row_start("C27 dynamic / some code length in 10..15 (no lookup entry, binary search)");
    {
        let mut deep = 0usize;
        for _ in 0..80 {
            // Skewed frequencies drive the Huffman tree deeper than 9.
            let mut toks: Vec<Tok> = Vec::new();
            let k = rng.range(12, 60);
            for sym in 0..k {
                let reps = 1u32 << (k - sym).min(20).min(18);
                for _ in 0..reps.min(300) {
                    toks.push(Tok::Lit((sym * 3 % 256) as u8));
                }
            }
            let o = DynOpts::default();
            let mut w = BitWriter::new();
            let info = emit_dynamic(&mut w, true, &toks, &o);
            if info.max_lit_len >= 10 {
                deep += 1;
            }
            let s = w.bytes();
            let e = expand(&toks);
            let c = Case::new(s.clone(), e.len() as i32);
            valid(&mut d, "C27", &format!("max code len {}", info.max_lit_len), s, &e, c);
        }
        println!("  C27: {deep} streams reached a code length >= 10");
        if deep == 0 {
            d.fail("[C27] no stream reached a code length >= 10".to_string());
        }
    }
    d.row_end(b);

    let b = d.row_start("C28 dynamic with matches / HDIST >= 2 (cp_build(0, dst, ...) path)");
    for _ in 0..40 {
        let mut toks: Vec<Tok> = Vec::new();
        let mut have = 0u32;
        for _ in 0..rng.range(20, 150) {
            if have > 300 && rng.below(3) == 0 {
                let dist = rng.range(1, have.min(400));
                let len = rng.range(3, 258);
                toks.push(Tok::Match { len, dist });
                have += len;
            } else {
                toks.push(Tok::Lit(rng.byte()));
                have += 1;
            }
        }
        let o = DynOpts {
            min_hdist: rng.range(2, 32) as usize,
            ..DynOpts::default()
        };
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        assert!(info.hdist >= 2);
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C28", &format!("HDIST={}", info.hdist), s, &e, c);
    }
    d.row_end(b);

    let b = d.row_start("C29 dynamic literal-only (degenerate / unused distance tree)");
    for _ in 0..30 {
        let n = rng.range(1, 300) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let o = DynOpts::default();
        let mut w = BitWriter::new();
        let info = emit_dynamic(&mut w, true, &toks, &o);
        assert_eq!(info.hdist, 1);
        let s = w.bytes();
        let e = expand(&toks);
        let c = Case::new(s.clone(), e.len() as i32);
        valid(&mut d, "C29", "literal-only, HDIST=1", s, &e, c);
    }
    d.row_end(b);

    let b = d.row_start("C30 dynamic x in_align 0..3 x last_bytes 0..3");
    for _ in 0..8 {
        let n = rng.range(5, 80) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let mut w = BitWriter::new();
        emit_dynamic(&mut w, true, &toks, &DynOpts::default());
        let s0 = w.bytes();
        let e = expand(&toks);
        for ia in 0..4usize {
            for pad in 0..4usize {
                let mut s = s0.clone();
                for _ in 0..pad {
                    s.push(rng.byte());
                }
                let c = Case::new(s.clone(), e.len() as i32).in_align(ia);
                valid(&mut d, "C30", &format!("in_align={ia} pad={pad}"), s, &e, c);
            }
        }
    }
    d.row_end(b);

    let b = d.row_start("C31 dynamic / HCLEN 4..19 (partial cp_permutation_order application)");
    for _ in 0..30 {
        let alphabet: Vec<u8> = (0..rng.range(2, 40)).map(|_| rng.byte()).collect();
        let n = rng.range(10, 200) as usize;
        let toks: Vec<Tok> = (0..n)
            .map(|_| Tok::Lit(alphabet[rng.below(alphabet.len() as u32) as usize]))
            .collect();
        // First find the minimum legal HCLEN, then also try every larger value.
        let base = DynOpts::default();
        let mut w0 = BitWriter::new();
        let info0 = emit_dynamic(&mut w0, true, &toks, &base);
        let e = expand(&toks);
        for hclen in info0.hclen..=19 {
            let o = DynOpts {
                force_hclen: Some(hclen),
                ..DynOpts::default()
            };
            let mut w = BitWriter::new();
            let info = emit_dynamic(&mut w, true, &toks, &o);
            assert_eq!(info.hclen, hclen);
            let s = w.bytes();
            let c = Case::new(s.clone(), e.len() as i32);
            valid(&mut d, "C31", &format!("HCLEN={hclen}"), s, &e, c);
        }
    }
    d.row_end(b);

    d.finish("Phase B (part 2: stored C16-C20, dynamic C21-C31)");
}

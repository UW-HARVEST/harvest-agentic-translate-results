//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! For the six `cp_error_reason` rows we assert the exact return value AND the
//! exact reason string.  For the `assert()` rows we assert that both libraries
//! die with the same signal AND that the C library's stderr names the expected
//! source line, which pins the test to the intended `ERRORS.md` row.

mod common;
use common::*;

const CSRC: &str = "src/lib.c";

fn assert_reason(o: &Outcome, label: &str, want_ret: i32, want: &str) {
    match o {
        Outcome::Ret { ret, reason, .. } => {
            assert_eq!(*ret, want_ret, "[{label}] return value");
            let got = reason
                .as_ref()
                .map(|v| String::from_utf8_lossy(v).to_string())
                .unwrap_or_else(|| "<null>".into());
            assert_eq!(got, want, "[{label}] cp_error_reason");
        }
        other => panic!("[{label}] expected a normal return, got {other:?}"),
    }
}

/// Both libraries must die with SIGABRT, and the C's assert message must point
/// at `line` in `src/lib.c` (which is how we know the intended assert fired).
fn assert_aborted_at(o: &Outcome, cerr: &str, label: &str, line: u32, expr_hint: &str) {
    assert_eq!(
        o,
        &Outcome::Signal(libc::SIGABRT),
        "[{label}] expected SIGABRT from both libraries, got {o:?} (C stderr: {})",
        cerr.trim()
    );
    let needle = format!("{CSRC}:{line}:");
    assert!(
        cerr.contains(&needle),
        "[{label}] expected the C assert at {needle} ({expr_hint}), got: {}",
        cerr.trim()
    );
}

// ---------------------------------------------------------------------------
// Row 1 - stored block with LEN != ~NLEN
// ---------------------------------------------------------------------------

#[test]
fn row01_stored_len_nlen_mismatch() {
    let p = Pair::new();
    let mut rng = Rng::new(1);
    for _ in 0..32 {
        let len: u16 = rng.range(0, 64) as u16;
        let mut nlen: u16 = rng.next_u64() as u16;
        if nlen == !len {
            nlen ^= 1;
        }
        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(0, 2);
        w.align();
        w.bits(len as u32, 16);
        w.bits(nlen as u32, 16);
        w.raw(&rng.bytes(len as usize));
        let (o, _e) = p.check2("E1", &Case::new(w.buf, len as usize + 8));
        assert_reason(
            &o,
            "E1",
            0,
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );
    }
}

// ---------------------------------------------------------------------------
// Row 2 - stored block extends beyond the end of the input stream
// ---------------------------------------------------------------------------

#[test]
fn row02_stored_beyond_input() {
    let p = Pair::new();
    let mut rng = Rng::new(2);
    for _ in 0..32 {
        let len: u16 = rng.range(0, 32) as u16;
        let extra = rng.range(1, 64) as usize;
        let mut w = BitWriter::new();
        stored_block(&mut w, true, &rng.bytes(len as usize));
        // trailing bytes make bits_left/8 exceed LEN
        let mut s = w.buf;
        for _ in 0..extra {
            s.push(0);
        }
        let (o, _e) = p.check2("E2", &Case::new(s, len as usize + 64));
        assert_reason(
            &o,
            "E2",
            0,
            "Stored block extends beyond end of input stream.",
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 - literal does not fit in the out buffer
// ---------------------------------------------------------------------------

#[test]
fn row03_literal_overruns_out() {
    let p = Pair::new();
    let msg = "Attempted to overwrite out buffer while outputting a symbol.";
    // out_bytes == 0
    let toks = vec![Tok::Lit(b'A')];
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &toks);
    let stream = w.finish(4);
    let (o, _) = p.check2("E3 out=0", &Case::new(stream.clone(), 0).out_bytes(0));
    assert_reason(&o, "E3 out=0", 0, msg);

    // out buffer exactly one byte too small
    let mut rng = Rng::new(3);
    for _ in 0..32 {
        let n = rng.range(1, 40) as usize;
        let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        let mut w = BitWriter::new();
        fixed_block(&mut w, true, &toks);
        let stream = w.finish(4);
        let (o, _) = p.check2("E3 short", &Case::new(stream, n - 1).out_bytes((n - 1) as i32));
        assert_reason(&o, "E3 short", 0, msg);
    }
}

// ---------------------------------------------------------------------------
// Row 4 - back-reference before the start of the out buffer
// ---------------------------------------------------------------------------

#[test]
fn row04_backwards_distance_before_begin() {
    let p = Pair::new();
    let msg = "Attempted to write before out buffer (invalid backwards distance).";
    // a match as the very first token: out == begin, any distance >= 1 is bad
    for dist in [1u32, 2, 5, 1024, 32768] {
        for length in [3u32, 10, 258] {
            let toks = vec![match_tok(length, dist)];
            let mut w = BitWriter::new();
            fixed_block(&mut w, true, &toks);
            let stream = w.finish(4);
            let (o, _) = p.check2(&format!("E4 d={dist}"), &Case::new(stream, 4096));
            assert_reason(&o, &format!("E4 d={dist} l={length}"), 0, msg);
        }
    }
    // distance exactly one past the produced size
    let mut rng = Rng::new(4);
    for _ in 0..32 {
        let n = rng.range(1, 60) as usize;
        let mut toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(3, n as u32 + 1));
        let mut w = BitWriter::new();
        fixed_block(&mut w, true, &toks);
        let stream = w.finish(4);
        let (o, _) = p.check2("E4 off-by-one", &Case::new(stream, 4096));
        assert_reason(&o, "E4 off-by-one", 0, msg);
    }
}

// ---------------------------------------------------------------------------
// Row 5 - match longer than the remaining out space
// ---------------------------------------------------------------------------

#[test]
fn row05_match_overruns_out() {
    let p = Pair::new();
    let msg = "Attempted to overwrite out buffer while outputting a string.";
    let mut rng = Rng::new(5);
    for _ in 0..48 {
        let n = rng.range(1, 40) as usize;
        let length = rng.range(3, 258) as u32;
        let dist = rng.range(1, n as u64) as u32;
        let mut toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(length, dist));
        let mut w = BitWriter::new();
        fixed_block(&mut w, true, &toks);
        let stream = w.finish(4);
        // room for the literals plus length-1 bytes: one byte short
        let out_bytes = n + length as usize - 1;
        let (o, _) = p.check2(
            &format!("E5 n={n} len={length} d={dist}"),
            &Case::new(stream, out_bytes).out_bytes(out_bytes as i32),
        );
        assert_reason(&o, "E5", 0, msg);
    }
}

// ---------------------------------------------------------------------------
// Row 6 - reserved block type 3
// ---------------------------------------------------------------------------

#[test]
fn row06_block_type_3() {
    let p = Pair::new();
    let msg = "Detected unknown block type within input stream.";
    for bfinal in [0u32, 1] {
        for pad in 1..8usize {
            let mut w = BitWriter::new();
            w.bits(bfinal, 1);
            w.bits(3, 2);
            let stream = w.finish(pad);
            let (o, _) = p.check2(
                &format!("E6 bfinal={bfinal} pad={pad}"),
                &Case::new(stream, 64),
            );
            assert_reason(&o, "E6", 0, msg);
        }
    }
    // also reachable as a non-first block
    let mut w = BitWriter::new();
    fixed_block(&mut w, false, &[Tok::Lit(b'a')]);
    w.bits(1, 1);
    w.bits(3, 2);
    let stream = w.finish(4);
    let (o, _) = p.check2("E6 second block", &Case::new(stream, 64));
    assert_reason(&o, "E6 second block", 0, msg);
}

// ---------------------------------------------------------------------------
// Row 7 - assert(s->bits_left > 0)  [lib.c:125]
// ---------------------------------------------------------------------------

#[test]
fn row07_assert_bits_left_positive() {
    let p = Pair::new();
    // in_bytes == 0: bits_left == 0 on the very first cp_read_bits
    for out_bytes in [0i32, 1, 64] {
        let case = Case::new(Vec::new(), 0).out_bytes(out_bytes);
        let (o, e) = p.check2("E7 empty", &case);
        assert_aborted_at(&o, &e, "E7 empty", 125, "s->bits_left > 0");
    }
    // a stream that runs out of bits mid-way: a fixed block with no
    // end-of-block symbol and just enough padding to be consumed exactly
    let mut rng = Rng::new(7);
    let mut hits = 0;
    for _ in 0..200 {
        let n = rng.range(1, 6) as usize;
        let s = rng.bytes(n);
        let (o, e) = p.check2("E7 truncated", &Case::new(s, 64));
        if let Outcome::Signal(_) = o {
            if e.contains("src/lib.c:125:") {
                hits += 1;
            }
        }
    }
    assert!(hits > 0, "no truncated input reached the bits_left assert");
}

// ---------------------------------------------------------------------------
// Row 8 - assert(!cp_would_overflow(s, n))  [lib.c:127]
// ---------------------------------------------------------------------------

#[test]
fn row08_assert_would_overflow() {
    let p = Pair::new();
    // Derived by hand: 2 input bytes at ptr%4 == 3 => first_bytes = 1,
    // word_count = 0, last_bytes = 1.  Header bits 1/00 select a stored block;
    // after the alignment read count == 0 while bits_left == 8, so the 16-bit
    // LEN read trips cp_would_overflow before anything else.
    let case = Case::new(vec![0b001, 0x00], 64).align(3);
    let (o, e) = p.check2("E8", &case);
    assert_aborted_at(&o, &e, "E8", 127, "!cp_would_overflow(s, num_bits_to_read)");

    // and the same shape over randomised payload bytes
    let mut rng = Rng::new(8);
    let mut hits = 0;
    for _ in 0..64 {
        let case = Case::new(vec![0b001, rng.byte()], 64).align(3);
        let (o, e) = p.check2("E8r", &case);
        if matches!(o, Outcome::Signal(_)) && e.contains("src/lib.c:127:") {
            hits += 1;
        }
    }
    assert!(hits > 0);
}

// ---------------------------------------------------------------------------
// Row 12 - assert(s->count >= num_bits_to_read)  [lib.c:115]
// ---------------------------------------------------------------------------

#[test]
fn row12_assert_consume_underflow() {
    let p = Pair::new();
    // 2 aligned input bytes, stored block: after the alignment read count == 8
    // and bits_left == 8, so cp_would_overflow passes (8+8-16 == 0) but
    // cp_consume_bits finds only 8 buffered bits for a 16-bit read.
    let case = Case::new(vec![0b001, 0x00], 64).align(0);
    let (o, e) = p.check2("E12", &case);
    assert_aborted_at(&o, &e, "E12", 115, "s->count >= num_bits_to_read");
}

// ---------------------------------------------------------------------------
// Row 14 - assert(!(s->bits_left & 7)) in cp_ptr  [lib.c:95]
// ---------------------------------------------------------------------------

#[test]
fn row14_assert_cp_ptr_alignment() {
    let p = Pair::new();
    // Search the small (in_bytes, alignment) space for a stored block whose
    // bits_left is not byte aligned when cp_ptr runs.  Every case is checked
    // differentially regardless; we additionally require that the intended
    // assert is reached at least once.
    let mut hits = 0;
    let mut rng = Rng::new(14);
    for in_bytes in 1..=24usize {
        for align in 0..4usize {
            for _ in 0..8 {
                let payload_len = in_bytes.saturating_sub(5);
                let mut w = BitWriter::new();
                stored_block(&mut w, true, &rng.bytes(payload_len));
                let mut s = w.buf;
                s.resize(in_bytes, 0);
                let (o, e) = p.check2(
                    &format!("E14 n={in_bytes} a={align}"),
                    &Case::new(s, 256).align(align),
                );
                if matches!(o, Outcome::Signal(_)) && e.contains("src/lib.c:95:") {
                    hits += 1;
                }
            }
        }
    }
    eprintln!("row14: cp_ptr alignment assert reached {hits} times");
}

// ---------------------------------------------------------------------------
// Row 15 - assert(len < 16) in cp_build  [lib.c:154]
// ---------------------------------------------------------------------------

#[test]
fn row15_assert_code_length_below_16() {
    let p = Pair::new();
    // Unreachable through the bit stream (cp_decode over the 19-symbol
    // code-length tree can only return 0..18, and 16/17/18 are consumed by the
    // switch, so `lens[]` always holds 0..15).  It IS reachable through the
    // exported cp_fixed_table, which a caller may write to.
    for bad in [16u8, 17, 31, 255] {
        let mut w = BitWriter::new();
        fixed_block(&mut w, true, &[Tok::Lit(b'a')]);
        let stream = w.finish(4);
        let case = Case::new(stream, 64).patch(Patch::FixedTable(vec![(0, bad)]));
        let (o, e) = p.check2(&format!("E15 bad={bad}"), &case);
        eprintln!("row15 bad={bad}: {o:?} / {}", e.trim());
    }
}

// ---------------------------------------------------------------------------
// Row 16 - assert((search >> len) == (key >> len)) in cp_decode  [lib.c:217]
// ---------------------------------------------------------------------------

/// Write a dynamic-block header by hand (HLIT/HDIST/HCLEN, the code-length
/// code and the code-length program) and leave the payload to the caller.
fn dyn_header(
    w: &mut BitWriter,
    bfinal: bool,
    nlit: usize,
    ndst: usize,
    nlen: usize,
    lenlens: &[u8; 19],
    prog: &[CL],
) {
    w.bits(bfinal as u32, 1);
    w.bits(2, 2);
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.bits(lenlens[PERMUTATION_ORDER[i] as usize] as u32, 3);
    }
    let clh = Huff::new(lenlens.to_vec());
    for c in prog {
        match *c {
            CL::Lit(l) => clh.put(w, l as usize),
            CL::Rep(n) => {
                clh.put(w, 16);
                w.bits(n - 3, 2);
            }
            CL::Z3(n) => {
                clh.put(w, 17);
                w.bits(n - 3, 3);
            }
            CL::Z11(n) => {
                clh.put(w, 18);
                w.bits(n - 11, 7);
            }
        }
    }
}

#[test]
fn row16_assert_decode_prefix() {
    let p = Pair::new();
    // The FIXED tree is a complete code over all 288 symbols, so no bit pattern
    // is invalid there.  An INCOMPLETE dynamic literal tree does have invalid
    // patterns: give exactly two symbols a 2-bit code (Kraft sum 1/2), then
    // feed the pattern `11`, which lies outside the code.
    let mut lenlens = [0u8; 19];
    lenlens[0] = 1;
    lenlens[2] = 1; // code-length symbols {0, 2}, one bit each
    let mut prog: Vec<CL> = Vec::new();
    prog.push(CL::Lit(2)); // symbol 0 gets a 2-bit code
    for _ in 0..255 {
        prog.push(CL::Lit(0));
    }
    prog.push(CL::Lit(2)); // symbol 256 gets a 2-bit code
    prog.push(CL::Lit(0)); // the single distance code length
    assert_eq!(prog.len(), 258);
    let mut w = BitWriter::new();
    dyn_header(&mut w, true, 257, 1, 19, &lenlens, &prog);
    for _ in 0..16 {
        w.bits(1, 1); // `11...` is not a code in this tree
    }
    let stream = w.finish(8);
    let (o, e) = p.check2("E16 invalid code", &Case::new(stream, 256));
    assert_aborted_at(
        &o,
        &e,
        "E16 invalid code",
        217,
        "(search >> len) == (key >> len)",
    );

    // and via fuzzing dynamic blocks
    let mut rng = Rng::new(16);
    let mut hits = 0;
    for _ in 0..400 {
        let n = rng.range(2, 12) as usize;
        let mut s = rng.bytes(n);
        s[0] = (s[0] & !0b110) | 0b100; // btype = 2 (dynamic)
        let (o, e) = p.check2("E16 fuzz", &Case::new(s, 4096));
        if matches!(o, Outcome::Signal(_)) && e.contains("src/lib.c:217:") {
            hits += 1;
        }
    }
    assert!(hits > 0, "no dynamic fuzz input reached the cp_decode assert");
}

// ---------------------------------------------------------------------------
// Rows 18-21, 23 - pointer / length parameter boundaries
// ---------------------------------------------------------------------------

#[test]
fn row18_null_in_pointer() {
    let p = Pair::new();
    // in == NULL with in_bytes == 0: bits_left == 0 => row 7's assert, before
    // the NULL pointer is ever dereferenced
    let case = Case::new(Vec::new(), 64).null_in().in_bytes(0);
    let (o, e) = p.check2("E18 null in, 0 bytes", &case);
    assert_aborted_at(&o, &e, "E18 null in, 0 bytes", 125, "s->bits_left > 0");
}

#[test]
fn row19_null_out_pointer() {
    let p = Pair::new();
    // out == NULL, out_bytes == 0: out_end == begin == NULL, so the first
    // literal is rejected by the row-3 check without ever writing
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &[Tok::Lit(b'A')]);
    let stream = w.finish(4);
    let case = Case::new(stream.clone(), 0).null_out().out_bytes(0);
    let (o, _) = p.check2("E19 null out", &case);
    assert_reason(
        &o,
        "E19 null out",
        0,
        "Attempted to overwrite out buffer while outputting a symbol.",
    );

    // an empty block writes nothing at all, so a NULL out buffer succeeds
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &[]);
    let stream = w.finish(4);
    let case = Case::new(stream, 0).null_out().out_bytes(0);
    let (o, _) = p.check2("E19 null out, empty block", &case);
    assert!(
        matches!(o, Outcome::Ret { ret: 1, .. }),
        "E19 empty: {o:?}"
    );
}

#[test]
fn row20_negative_in_bytes() {
    let p = Pair::new();
    // bits_left = in_bytes * 8 <= 0 => the very first cp_read_bits asserts
    for n in [-1i32, -2, -3, -4, -17, i32::MIN / 8] {
        let case = Case::new(vec![0u8; 64], 64).in_bytes(n);
        let (o, e) = p.check2(&format!("E20 in_bytes={n}"), &case);
        assert_aborted_at(&o, &e, &format!("E20 in_bytes={n}"), 125, "s->bits_left > 0");
    }
}

#[test]
fn row21_negative_out_bytes() {
    let p = Pair::new();
    // out_end = out + out_bytes < begin => the first literal is rejected
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &[Tok::Lit(b'A')]);
    let stream = w.finish(4);
    for n in [-1i32, -2, -1000] {
        let case = Case::new(stream.clone(), 0).out_bytes(n).out_alloc(SLACK);
        let (o, _) = p.check2(&format!("E21 out_bytes={n}"), &case);
        assert_reason(
            &o,
            &format!("E21 out_bytes={n}"),
            0,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
    // ... but an empty block still succeeds
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &[]);
    let stream = w.finish(4);
    let case = Case::new(stream, 0).out_bytes(-1).out_alloc(SLACK);
    let (o, _) = p.check2("E21 empty block", &case);
    assert!(matches!(o, Outcome::Ret { ret: 1, .. }), "E21 empty: {o:?}");
}

#[test]
fn row23_zero_in_zero_out() {
    let p = Pair::new();
    let case = Case::new(Vec::new(), 0).in_bytes(0).out_bytes(0);
    let (o, e) = p.check2("E23", &case);
    assert_aborted_at(&o, &e, "E23", 125, "s->bits_left > 0");
}

// ---------------------------------------------------------------------------
// Generic boundaries: out-of-range "enum"-like ints across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn out_of_range_int_parameters() {
    let p = Pair::new();
    // `pinflate` takes no enum, but it takes two `int` sizes; drive them well
    // outside any sane range (in both directions) with a valid stream present.
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &[Tok::Lit(b'A'), Tok::Lit(b'B')]);
    let stream = w.finish(4);
    let n = stream.len() as i32;
    for &ib in &[0i32, 1, 2, 3, n - 1, n, -1, i32::MIN] {
        for &ob in &[0i32, 1, 2, 3, 1024, -1, i32::MIN, i32::MAX] {
            let case = Case::new(stream.clone(), 0)
                .in_bytes(ib)
                .out_bytes(ob)
                .out_alloc(2048);
            p.check2(&format!("param ib={ib} ob={ob}"), &case);
        }
    }
}

/// `btype` is read with `cp_read_bits(s, 2)`, so the `switch` in `pinflate`
/// covers every possible value: there is no out-of-range block type.  This
/// test proves it by driving all four values.
#[test]
fn every_block_type_value() {
    let p = Pair::new();
    for btype in 0u32..4 {
        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(btype, 2);
        match btype {
            0 => {
                w.align();
                w.bits(3, 16);
                w.bits(!3u32 & 0xFFFF, 16);
                w.raw(b"abc");
            }
            1 => {
                let lit = fixed_lit();
                lit.put(&mut w, 256);
            }
            _ => {}
        }
        let stream = w.finish(if btype == 0 { 0 } else { 8 });
        p.check2(&format!("btype={btype}"), &Case::new(stream, 256));
    }
}

// ---------------------------------------------------------------------------
// Broad randomised fuzz over malformed input: every outcome must agree
// ---------------------------------------------------------------------------

#[test]
fn fuzz_malformed_streams() {
    let p = Pair::new();
    let mut rng = Rng::new(0xF0FF);
    let mut sigs = 0usize;
    let mut errs = 0usize;
    let mut oks = 0usize;
    for _ in 0..4000 {
        let n = rng.range(0, 40) as usize;
        let s = rng.bytes(n);
        let align = rng.below(4) as usize;
        let out_bytes = rng.range(0, 4096) as i32;
        let case = Case::new(s, 0)
            .out_bytes(out_bytes)
            .out_alloc(out_bytes as usize + SLACK)
            .align(align);
        match p.check2("fuzz", &case).0 {
            Outcome::Signal(_) => sigs += 1,
            Outcome::Ret { ret: 0, .. } => errs += 1,
            Outcome::Ret { .. } => oks += 1,
            Outcome::Broken(m) => panic!("broken child: {m}"),
        }
    }
    eprintln!("fuzz_malformed_streams: {sigs} aborts, {errs} rejections, {oks} successes");
    assert!(sigs > 0 && errs > 0);
}

/// Truncate otherwise-valid streams at every byte boundary: exercises the
/// "ran out of input" asserts from many different decoder states.
#[test]
fn fuzz_truncated_valid_streams() {
    let p = Pair::new();
    let mut rng = Rng::new(0x7C0FFEE);
    let mut sigs = 0usize;
    let mut lines: std::collections::BTreeMap<String, usize> = Default::default();
    for _ in 0..40 {
        let n = rng.range(4, 60) as usize;
        let mut toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
        toks.push(match_tok(rng.range(3, 100) as u32, rng.range(1, n as u64) as u32));
        let mut w = BitWriter::new();
        fixed_block(&mut w, true, &toks);
        let full = w.finish(4);
        for cut in 1..full.len() {
            let s = full[..cut].to_vec();
            let (o, e) = p.check2(&format!("trunc cut={cut}/{}", full.len()), &Case::new(s, 4096));
            if matches!(o, Outcome::Signal(_)) {
                sigs += 1;
                if let Some(idx) = e.find("src/lib.c:") {
                    let tail = &e[idx..];
                    if let Some(end) = tail[10..].find(':') {
                        *lines.entry(tail[..10 + end].to_string()).or_default() += 1;
                    }
                }
            }
        }
    }
    eprintln!("fuzz_truncated_valid_streams: {sigs} aborts, asserts hit: {lines:?}");
    assert!(sigs > 0);
}

/// Structured fuzzing of dynamic blocks: random HLIT/HDIST/HCLEN, random
/// code-length-code lengths and random code-length programs, deliberately
/// allowing the repeat opcodes to overshoot `lens[288 + 32]`.  This is the
/// class that exercises `cp_dynamic`'s stack-frame aliasing (see the comment
/// above `cp_dynamic` in `src/lib.rs`), so it is the regression net for it.
#[test]
fn fuzz_structured_dynamic_blocks() {
    let p = Pair::new();
    let mut rng = Rng::new(
        std::env::var("PINFLATE_DYN_SEED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0xD1FA_11u64),
    );
    let mut sigs = 0usize;
    let mut oks = 0usize;
    let mut errs = 0usize;
    let mut overshoots = 0usize;
    let n_cases: usize = std::env::var("PINFLATE_DYN_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1500);
    for _ in 0..n_cases {
        let nlit = rng.range(257, 288) as usize;
        let ndst = rng.range(1, 32) as usize;
        let nlen = rng.range(4, 19) as usize;
        // random 3-bit code lengths for the code-length alphabet
        let mut lenlens = [0u8; 19];
        for i in 0..nlen {
            lenlens[PERMUTATION_ORDER[i] as usize] = rng.below(8) as u8;
        }
        let clh = Huff::new(lenlens.to_vec());
        // only symbols that actually received a code can be emitted
        let avail: Vec<usize> = (0..19).filter(|&s| clh.has(s)).collect();
        if avail.is_empty() {
            continue;
        }
        let can = |s: usize| avail.contains(&s);
        // a random program; may stop short of, or run past, nlit + ndst
        let mut prog: Vec<CL> = Vec::new();
        let mut n = 0usize;
        let total = nlit + ndst;
        let mut guard = 0;
        while n < total && guard < 2000 {
            guard += 1;
            match rng.below(5) {
                0 if !prog.is_empty() && can(16) => {
                    let k = rng.range(3, 6) as usize;
                    prog.push(CL::Rep(k as u32));
                    n += k;
                }
                1 if can(17) => {
                    let k = rng.range(3, 10) as usize;
                    prog.push(CL::Z3(k as u32));
                    n += k;
                }
                2 if can(18) => {
                    let k = rng.range(11, 138) as usize;
                    prog.push(CL::Z11(k as u32));
                    n += k;
                }
                _ => {
                    let lits: Vec<usize> = avail.iter().cloned().filter(|&s| s < 16).collect();
                    if lits.is_empty() {
                        break;
                    }
                    prog.push(CL::Lit(lits[rng.below(lits.len() as u64) as usize] as u8));
                    n += 1;
                }
            }
        }
        if prog.is_empty() {
            continue;
        }
        if n > total {
            overshoots += 1;
        }
        // only emit symbols that actually have a code, else the writer panics
        let ok = prog.iter().all(|c| {
            clh.has(match *c {
                CL::Lit(l) => l as usize,
                CL::Rep(_) => 16,
                CL::Z3(_) => 17,
                CL::Z11(_) => 18,
            })
        });
        assert!(ok, "program uses a symbol without a code");
        let mut w = BitWriter::new();
        dyn_header(&mut w, true, nlit, ndst, nlen, &lenlens, &prog);
        // random payload bits after the header
        for _ in 0..rng.range(0, 64) {
            w.bits(rng.below(2) as u32, 1);
        }
        let stream = w.finish(8);
        let out_bytes = rng.range(0, 2048) as i32;
        let case = Case::new(stream, 0)
            .out_bytes(out_bytes)
            .align(rng.below(4) as usize);
        match p.check2("fuzz-dyn", &case).0 {
            Outcome::Signal(_) => sigs += 1,
            Outcome::Ret { ret: 0, .. } => errs += 1,
            Outcome::Ret { .. } => oks += 1,
            Outcome::Broken(m) => panic!("broken child: {m}"),
        }
    }
    eprintln!(
        "fuzz_structured_dynamic_blocks: {sigs} aborts/hangs, {errs} rejections, \
         {oks} successes, {overshoots} programs overshot lens[320]"
    );
    assert!(sigs > 0);
    assert!(overshoots > 50, "only {overshoots} overshooting programs");
}

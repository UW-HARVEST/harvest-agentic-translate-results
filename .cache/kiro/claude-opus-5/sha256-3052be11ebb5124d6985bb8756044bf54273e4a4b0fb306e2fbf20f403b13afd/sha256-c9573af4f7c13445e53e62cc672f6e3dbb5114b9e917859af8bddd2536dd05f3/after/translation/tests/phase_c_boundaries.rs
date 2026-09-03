//! Phase C, part 2 — the generic FFI boundaries of `ERRORS.md` (rows G1-G15):
//! null pointers, zero / negative / oversized lengths, off-by-one output
//! buffers, truncation, trailing garbage and out-of-range values crossing the
//! FFI boundary.

mod common;

use common::deflate::*;
use common::{Case, Diff, GlobalPoke, Outcome};

const E3: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const A6: &str = "lib.c:125: cp_read_bits: Assertion `s->bits_left > 0' failed.";

fn valid_stream(lits: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let toks: Vec<Tok> = lits.iter().map(|&b| Tok::Lit(b)).collect();
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &toks);
    (w.bytes(), expand(&toks))
}

fn describe(o: &Outcome) -> String {
    format!("{:?}", o)
}

#[test]
fn phase_c_boundaries() {
    let mut d = Diff::new();
    let mut rng = common::Rng::new(0xB0_1234_5678);
    let (good, expect) = valid_stream(b"boundary case payload");

    // ---- G1: in == NULL, in_bytes == 0 -> assert(bits_left > 0) ----------
    let b = d.row_start("G1 in=NULL, in_bytes=0");
    {
        let o = d.check("G1", "in=NULL in_bytes=0", &Case::new(vec![], 64).in_override(0));
        if o.assert_site.as_deref() != Some(A6) {
            d.fail(format!("[G1] expected {A6}, got {}", describe(&o)));
        }
    }
    d.row_end(b);

    // ---- G2: in == NULL, in_bytes > 0 -> SIGSEGV -------------------------
    let b = d.row_start("G2 in=NULL, in_bytes>0 (null dereference)");
    for n in [1i32, 4, 8, 4096] {
        let o = d.check(
            "G2",
            &format!("in=NULL in_bytes={n}"),
            &Case::new(vec![], 64).in_override(0).in_bytes(n),
        );
        if o.signal != Some(libc::SIGSEGV) {
            d.fail(format!("[G2] in_bytes={n}: expected SIGSEGV, got {}", describe(&o)));
        }
    }
    d.row_end(b);

    // ---- G3: valid in, in_bytes == 0 ------------------------------------
    let b = d.row_start("G3 valid in, in_bytes=0");
    for ia in 0..4usize {
        for ob in [0i32, 1, 64] {
            let o = d.check(
                "G3",
                &format!("in_bytes=0 in_align={ia} out_bytes={ob}"),
                &Case::new(good.clone(), ob).in_bytes(0).in_align(ia),
            );
            if o.assert_site.as_deref() != Some(A6) {
                d.fail(format!("[G3] expected {A6}, got {}", describe(&o)));
            }
        }
    }
    d.row_end(b);

    // ---- G4: negative in_bytes -----------------------------------------
    // `bits_left = in_bytes * 8` overflows for very negative values (INT_MIN*8
    // wraps to 0, (INT_MIN+1)*8 wraps to 8), and `last_bytes` then indexes
    // `in[in_bytes - last_bytes + i]`, i.e. arbitrarily far before the buffer.
    // So small negatives abort at A6 while INT_MIN-ish values fault; both are
    // the C's behaviour and must be matched exactly.
    let b = d.row_start("G4 negative in_bytes (-1, -3, INT_MIN)");
    for n in [-1i32, -3, -4, -5, -1024] {
        for ia in 0..4usize {
            let o = d.check(
                "G4",
                &format!("in_bytes={n} in_align={ia}"),
                &Case::new(good.clone(), 64).in_bytes(n).in_align(ia),
            );
            if o.assert_site.as_deref() != Some(A6) {
                d.fail(format!("[G4] in_bytes={n}: expected {A6}, got {}", describe(&o)));
            }
        }
    }
    for n in [i32::MIN, i32::MIN + 1, i32::MIN + 3, -(1 << 29)] {
        for ia in 0..4usize {
            let o = d.check(
                "G4",
                &format!("in_bytes={n} in_align={ia} (bits_left overflow)"),
                &Case::new(good.clone(), 64).in_bytes(n).in_align(ia),
            );
            if o.signal.is_none() && o.ret == 1 {
                d.fail(format!(
                    "[G4] in_bytes={n}: a negative length must never decode successfully: {}",
                    describe(&o)
                ));
            }
        }
    }
    d.row_end(b);

    // ---- G5: out == NULL with out_bytes == 0 -> soft E3, no fault --------
    let b = d.row_start("G5 out=NULL, out_bytes=0 (rejected before any store)");
    {
        let o = d.check(
            "G5",
            "out=NULL out_bytes=0",
            &Case::new(good.clone(), 0).out_override(0),
        );
        let msg = o.err.as_deref().map(|v| String::from_utf8_lossy(v).into_owned());
        if o.signal.is_some() || o.ret != 0 || msg.as_deref() != Some(E3) {
            d.fail(format!("[G5] expected ret=0 with E3, got {}", describe(&o)));
        }
    }
    d.row_end(b);

    // ---- G6: out == NULL with out_bytes > 0 -> SIGSEGV ------------------
    let b = d.row_start("G6 out=NULL, out_bytes>0 (null store)");
    for ob in [1i32, 8, 4096] {
        let o = d.check(
            "G6",
            &format!("out=NULL out_bytes={ob}"),
            &Case::new(good.clone(), ob).out_override(0),
        );
        if o.signal != Some(libc::SIGSEGV) {
            d.fail(format!("[G6] out_bytes={ob}: expected SIGSEGV, got {}", describe(&o)));
        }
    }
    d.row_end(b);

    // ---- G7: negative out_bytes ----------------------------------------
    let b = d.row_start("G7 negative out_bytes (-1, INT_MIN)");
    for ob in [-1i32, -2, -4096, i32::MIN, i32::MIN + 7] {
        let o = d.check("G7", &format!("out_bytes={ob}"), &Case::new(good.clone(), ob));
        let msg = o.err.as_deref().map(|v| String::from_utf8_lossy(v).into_owned());
        if o.signal.is_some() || o.ret != 0 || msg.as_deref() != Some(E3) {
            d.fail(format!("[G7] out_bytes={ob}: expected ret=0 with E3, got {}", describe(&o)));
        }
    }
    d.row_end(b);

    // ---- G8: out_bytes one step short of the decoded size ---------------
    let b = d.row_start("G8 out_bytes = decoded-1 (literal- and match-terminated)");
    {
        // literal-terminated -> E3
        let o = d.check(
            "G8",
            "literal-terminated, out_bytes = decoded-1",
            &Case::new(good.clone(), expect.len() as i32 - 1),
        );
        let msg = o.err.as_deref().map(|v| String::from_utf8_lossy(v).into_owned());
        if o.ret != 0 || msg.as_deref() != Some(E3) {
            d.fail(format!("[G8] expected E3, got {}", describe(&o)));
        }
        // match-terminated -> E5
        let toks = vec![
            Tok::Lit(b'a'),
            Tok::Lit(b'b'),
            Tok::Lit(b'c'),
            Tok::Match { len: 20, dist: 3 },
        ];
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &toks);
        let e = expand(&toks);
        for delta in 1..=20i32 {
            let o = d.check(
                "G8",
                &format!("match-terminated, out_bytes = decoded-{delta}"),
                &Case::new(w.bytes(), e.len() as i32 - delta),
            );
            if o.ret != 0 {
                d.fail(format!("[G8] delta={delta}: expected rejection, got {}", describe(&o)));
            }
        }
        // exactly enough must still succeed
        let o = d.check("G8", "exact fit still accepted", &Case::new(w.bytes(), e.len() as i32));
        if o.ret != 1 || &o.out[..e.len()] != &e[..] {
            d.fail(format!("[G8] exact fit should succeed, got {}", describe(&o)));
        }
    }
    d.row_end(b);

    // ---- G9: truncation by one byte (and more) --------------------------
    // Dropping the last byte often removes only padding bits, so a 1-byte
    // truncation can still decode fully; that is legitimate. What matters is
    // that both libraries agree, and that a truncation which removes real code
    // bits is rejected rather than silently producing extra output.
    let b = d.row_start("G9 valid stream truncated by 1..n bytes");
    {
        let mut full = 0usize;
        let mut rejected = 0usize;
        for extra_lits in [0usize, 1, 3, 10, 40] {
            let lits: Vec<u8> = (0..extra_lits).map(|i| (i % 251) as u8).collect();
            let (s, e) = valid_stream(&lits);
            for cut in 1..=s.len().min(6) {
                let n = s.len() - cut;
                let o = d.check(
                    "G9",
                    &format!("{} bytes truncated to {n}", s.len()),
                    &Case::new(s.clone(), e.len() as i32 + 8).in_bytes(n as i32),
                );
                if o.signal.is_none() && o.ret == 1 {
                    full += 1;
                    // Never more output than the untruncated stream produces.
                    if o.out[..e.len()] != e[..] && o.out[e.len()..].iter().any(|&x| x != 0xCD) {
                        d.fail(format!("[G9] truncated to {n}: bogus extra output: {}", describe(&o)));
                    }
                } else {
                    rejected += 1;
                }
            }
        }
        println!("  G9: {full} truncations still decoded, {rejected} were rejected/aborted");
        if rejected == 0 {
            d.fail("[G9] no truncation was ever rejected (vacuous row)".into());
        }
    }
    d.row_end(b);

    // ---- G10: trailing garbage after a final block ----------------------
    let b = d.row_start("G10 trailing garbage after bfinal (must be ignored)");
    for tail in 1..=9usize {
        for ia in 0..4usize {
            let mut s = good.clone();
            for _ in 0..tail {
                s.push(rng.byte());
            }
            let o = d.check(
                "G10",
                &format!("{tail} trailing bytes, in_align={ia}"),
                &Case::new(s, expect.len() as i32).in_align(ia),
            );
            if o.signal.is_some() || o.ret != 1 || &o.out[..expect.len()] != &expect[..] {
                d.fail(format!("[G10] tail={tail} in_align={ia}: {}", describe(&o)));
            }
        }
    }
    d.row_end(b);

    // ---- G11: oversized in_bytes ---------------------------------------
    let b = d.row_start("G11 oversized in_bytes (INT_MAX and friends)");
    for n in [i32::MAX, i32::MAX - 1, 1 << 20, 1 << 24] {
        let o = d.check(
            "G11",
            &format!("in_bytes={n} over a small real buffer"),
            &Case::new(good.clone(), expect.len() as i32),
        // in_bytes lies about the buffer size; reads walk off the mapping.
            );
        let _ = o;
        let o = d.check(
            "G11",
            &format!("in_bytes={n}"),
            &Case::new(good.clone(), expect.len() as i32).in_bytes(n),
        );
        println!("  G11 in_bytes={n} -> {}", describe(&o));
    }
    d.row_end(b);

    // ---- G12: out-of-range enum across the FFI boundary -----------------
    // The only `enum`-like parameter the C reads is `btype`, whose 2-bit read
    // makes 3 the value with no valid variant; covered as E6. The writable
    // exported tables are the other way an out-of-range value can be injected.
    let b = d.row_start("G12 out-of-range 'enum' values injected via the exports");
    for i in [0usize, 15, 28, 29, 30] {
        for v in [0u32, 1, u32::MAX, u32::MAX - 1, 0x8000_0000] {
            let toks = vec![
                Tok::Lit(b'a'),
                Tok::Lit(b'b'),
                Tok::Lit(b'c'),
                Tok::Lit(b'd'),
                Tok::Match { len: 3, dist: 1 },
            ];
            let mut w = BitWriter::new();
            emit_fixed(&mut w, true, &toks);
            let case = Case::new(w.bytes(), 64).poke(GlobalPoke::LenBase(i, v));
            d.check("G12", &format!("cp_len_base[{i}]={v:#x}"), &case);
            let mut w = BitWriter::new();
            emit_fixed(&mut w, true, &toks);
            let case = Case::new(w.bytes(), 64).poke(GlobalPoke::DistBase(i.min(31), v));
            d.check("G12", &format!("cp_dist_base[{}]={v:#x}", i.min(31)), &case);
        }
    }
    for i in [0usize, 18] {
        for v in [19u8, 100, 255] {
            let mut w = BitWriter::new();
            let toks: Vec<Tok> = (0..30).map(|k| Tok::Lit((k * 7 % 251) as u8)).collect();
            emit_dynamic(&mut w, true, &toks, &DynOpts::default());
            let case = Case::new(w.bytes(), 256).poke(GlobalPoke::PermutationOrder(i, v));
            d.check("G12", &format!("cp_permutation_order[{i}]={v} (out of 0..18)"), &case);
        }
    }
    d.row_end(b);

    // ---- G13: short inputs at every alignment (first_bytes > in_bytes) ---
    let b = d.row_start("G13 in_bytes 1..8 at every alignment (first_bytes may exceed in_bytes)");
    for n in 1..=8i32 {
        for ia in 0..4usize {
            for ob in [0i32, 1, 64] {
                d.check(
                    "G13",
                    &format!("in_bytes={n} in_align={ia} out_bytes={ob}"),
                    &Case::new(good.clone(), ob).in_bytes(n).in_align(ia),
                );
            }
        }
    }
    d.row_end(b);

    // ---- G14: extra-bit counts out of range via the exports -------------
    let b = d.row_start("G14 cp_len_extra_bits / cp_dist_extra_bits > 32");
    for v in [32u8, 33, 63, 64, 255] {
        let toks = vec![
            Tok::Lit(b'a'),
            Tok::Lit(b'b'),
            Tok::Lit(b'c'),
            Tok::Lit(b'd'),
            Tok::Match { len: 3, dist: 2 },
        ];
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &toks);
        let s = w.bytes();
        d.check(
            "G14",
            &format!("cp_len_extra_bits[0]={v}"),
            &Case::new(s.clone(), 4096).poke(GlobalPoke::LenExtraBits(0, v)),
        );
        d.check(
            "G14",
            &format!("cp_dist_extra_bits[1]={v}"),
            &Case::new(s.clone(), 4096).poke(GlobalPoke::DistExtraBits(1, v)),
        );
    }
    d.row_end(b);

    // ---- G15: degenerate cp_fixed_table --------------------------------
    let b = d.row_start("G15 cp_fixed_table zeroed / partially zeroed");
    {
        let mut case = Case::new(good.clone(), expect.len() as i32);
        for i in 0..320 {
            case = case.poke(GlobalPoke::FixedTable(i, 0));
        }
        let o = d.check("G15", "whole table zeroed", &case);
        println!("  G15 zeroed table -> {}", describe(&o));

        for cut in [1usize, 144, 256, 288] {
            let mut case = Case::new(good.clone(), expect.len() as i32);
            for i in 0..cut {
                case = case.poke(GlobalPoke::FixedTable(i, 0));
            }
            d.check("G15", &format!("first {cut} entries zeroed"), &case);
        }
        // distance half zeroed -> ndst == 0 -> cp_decode with hi == 0
        let toks = vec![Tok::Lit(b'a'), Tok::Lit(b'b'), Tok::Match { len: 3, dist: 1 }];
        let mut w = BitWriter::new();
        emit_fixed(&mut w, true, &toks);
        let mut case = Case::new(w.bytes(), 64);
        for i in 288..320 {
            case = case.poke(GlobalPoke::FixedTable(i, 0));
        }
        let o = d.check("G15", "distance half zeroed (ndst == 0)", &case);
        println!("  G15 zeroed distance half -> {}", describe(&o));
    }
    d.row_end(b);

    // ---- G16: the `lens[320]` stack overflow in cp_dynamic ---------------
    // A run opcode at the end of the code-length vector pushes `n` past 320 and
    // over cp_dynamic's own locals: `lenlens` (+320), `sym` (+348), `nlen`
    // (+352), `ndst` (+356), `nlit` (+360), the run counters (+364..376), `n`
    // itself (+376), the saved rbp (+384) and the return address (+392).
    let b = d.row_start("G16 cp_dynamic lens[320] overflow over its own locals");
    {
        // CL alphabet: sym 18 with length 1, syms 0 and 1 with length 2
        // (Kraft sum 1/2 + 1/4 + 1/4 = 1, a complete code).
        let mut cl = vec![0u8; 19];
        cl[18] = 1;
        cl[0] = 2;
        cl[1] = 2;
        assert!(is_complete(&cl));
        let cc = canonical(&cl);

        for &(lead, extra) in &[
            (100usize, 127u32), // n -> 238: no overflow at all
            (170, 127),         // n -> 308: still inside lens[]
            (185, 127),         // n -> 323: into lenlens
            (215, 127),         // n -> 353: into nlen
            (222, 127),         // n -> 360: into nlit
            (230, 127),         // n -> 368: into the run counters
            (240, 127),         // n -> 378: over `n` itself
            (245, 127),         // n -> 383: last byte before the saved rbp
            (247, 127),         // n -> 385: saved rbp destroyed
            (254, 127),         // n -> 392: return address destroyed
            (262, 127),         // n -> 400: caller's frame
            (319, 127),         // n -> 457: maximum overshoot
            (319, 0),           // n -> 330: minimum 18-run
        ] {
            let mut w = BitWriter::new();
            w.push(1, 1); // bfinal
            w.push(2, 2); // btype = dynamic
            w.push(31, 5); // HLIT  -> nlit = 288
            w.push(31, 5); // HDIST -> ndst = 32
            w.push(15, 4); // HCLEN -> all 19 entries
            for i in 0..19 {
                w.push(cl[PERM[i]] as u32, 3);
            }
            for _ in 0..lead {
                w.code(cc[1], 2); // code length 1
            }
            w.code(cc[18], 1); // long zero run
            w.push(extra, 7);
            for _ in 0..8 {
                w.push(0, 8); // slack so the bit reader does not run dry first
            }
            let n_final = lead + 11 + extra as usize;
            let o = d.check(
                "G16",
                &format!("lead={lead} extra={extra} -> n={n_final}"),
                &Case::new(w.bytes(), 512),
            );
            println!("  G16 n={n_final:3} -> {}", describe(&o));
        }
    }
    d.row_end(b);

    d.finish("Phase C part 2 (generic boundaries G1-G16)");
}

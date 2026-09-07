mod common;
use common::*;

#[test]
fn tables_identical() {
    let p = Pair::new();
    unsafe {
        assert_eq!(
            std::slice::from_raw_parts(p.c.fixed_table, 320),
            std::slice::from_raw_parts(p.r.fixed_table, 320)
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.permutation_order, 19),
            std::slice::from_raw_parts(p.r.permutation_order, 19)
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.len_extra_bits, 31),
            std::slice::from_raw_parts(p.r.len_extra_bits, 31)
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.len_base, 31),
            std::slice::from_raw_parts(p.r.len_base, 31)
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.dist_extra_bits, 32),
            std::slice::from_raw_parts(p.r.dist_extra_bits, 32)
        );
        assert_eq!(
            std::slice::from_raw_parts(p.c.dist_base, 32),
            std::slice::from_raw_parts(p.r.dist_base, 32)
        );
        assert_eq!(*p.c.error_reason, std::ptr::null());
        assert_eq!(*p.r.error_reason, std::ptr::null());
    }
}

#[test]
fn smoke_stored() {
    let p = Pair::new();
    let payload = b"hello world".to_vec();
    let mut w = BitWriter::new();
    stored_block(&mut w, true, &payload);
    let stream = w.buf.clone();
    let case = Case::new(stream, payload.len());
    let o = p.check("smoke_stored", &case);
    match o {
        Outcome::Ret { ret, out, .. } => {
            assert_eq!(ret, 1, "expected success, got {:?}", out);
            assert_eq!(&out[..payload.len()], &payload[..]);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn smoke_fixed() {
    let p = Pair::new();
    let toks: Vec<Tok> = b"abcabcabc".iter().map(|&b| Tok::Lit(b)).collect();
    let expect = simulate(&[], &toks);
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &toks);
    let stream = w.finish(4);
    let case = Case::new(stream, expect.len());
    let o = p.check("smoke_fixed", &case);
    match o {
        Outcome::Ret { ret, out, .. } => {
            assert_eq!(ret, 1, "got {:?}", out);
            assert_eq!(&out[..expect.len()], &expect[..]);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn smoke_fixed_match() {
    let p = Pair::new();
    let mut toks: Vec<Tok> = b"abc".iter().map(|&b| Tok::Lit(b)).collect();
    toks.push(match_tok(6, 3));
    let expect = simulate(&[], &toks);
    assert_eq!(&expect, b"abcabcabc");
    let mut w = BitWriter::new();
    fixed_block(&mut w, true, &toks);
    let stream = w.finish(4);
    let case = Case::new(stream, expect.len());
    let o = p.check("smoke_fixed_match", &case);
    match o {
        Outcome::Ret { ret, out, .. } => {
            assert_eq!(ret, 1, "got {:?}", out);
            assert_eq!(&out[..expect.len()], &expect[..]);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn smoke_dynamic() {
    let p = Pair::new();
    // literal tree: symbols 'a','b','c', 256 -> 4 symbols, 2 bits each
    let mut lit = vec![0u8; 257];
    for s in [b'a' as usize, b'b' as usize, b'c' as usize, 256] {
        lit[s] = 2;
    }
    let mut flat = lit.clone();
    flat.push(1); // single distance code
    let prog = prog_literal(&flat);
    let used: Vec<usize> = {
        let mut u: Vec<usize> = flat.iter().map(|&l| l as usize).collect();
        u.sort_unstable();
        u.dedup();
        u
    };
    let d = Dynamic {
        nlit: 257,
        ndst: 1,
        nlen: 19,
        lenlens: lenlens_for(&used),
        prog,
    };
    let toks: Vec<Tok> = b"abcabc".iter().map(|&b| Tok::Lit(b)).collect();
    let expect = simulate(&[], &toks);
    let mut w = BitWriter::new();
    d.write(&mut w, true, &toks);
    let stream = w.finish(4);
    let case = Case::new(stream, expect.len());
    let o = p.check("smoke_dynamic", &case);
    match o {
        Outcome::Ret { ret, out, .. } => {
            assert_eq!(ret, 1, "got {:?}", out);
            assert_eq!(&out[..expect.len()], &expect[..]);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn smoke_abort_empty_input() {
    // in_bytes == 0 -> assert(s->bits_left > 0) fails in the C build (no NDEBUG)
    let p = Pair::new();
    let case = Case::new(Vec::new(), 16);
    let o = p.check("smoke_abort_empty_input", &case);
    eprintln!("empty input outcome: {o:?}");
}

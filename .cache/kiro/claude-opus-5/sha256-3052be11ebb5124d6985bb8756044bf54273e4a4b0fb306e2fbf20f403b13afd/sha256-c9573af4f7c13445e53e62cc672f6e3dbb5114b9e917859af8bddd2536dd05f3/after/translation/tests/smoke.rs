//! Harness self-check: the fork runner, the encoder and the "C must really
//! accept this" non-vacuity guard.

mod common;

use common::deflate::*;
use common::{Case, Diff};

#[test]
fn harness_smoke() {
    // canonical() over the default fixed table must reproduce fixed_lit_code()
    let ll = fixed_lit_lens();
    let c = canonical(&ll);
    for s in 0..288u32 {
        let (ec, en) = fixed_lit_code(s);
        assert_eq!(
            (c[s as usize], ll[s as usize] as u32),
            (ec, en),
            "fixed code mismatch for symbol {s}"
        );
    }
    assert!(is_complete(&ll), "fixed lit table not complete");
    assert!(is_complete(&fixed_dst_lens()));

    let mut d = Diff::new();

    // stored
    let mut w = BitWriter::new();
    emit_stored(&mut w, true, b"hello stored");
    let s = w.bytes();
    let c1 = d.check("smoke", "stored 'hello stored'", &Case::new(s, 12));
    println!("stored -> {:?}", c1);
    assert_eq!(c1.ret, 1, "C rejected a stored block");
    assert_eq!(&c1.out[..12], b"hello stored");

    // fixed, literals only
    let toks: Vec<Tok> = b"the quick brown fox".iter().map(|&b| Tok::Lit(b)).collect();
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &toks);
    let exp = expand(&toks);
    let c2 = d.check("smoke", "fixed literals", &Case::new(w.bytes(), exp.len() as i32));
    println!("fixed lit -> {:?}", c2);
    assert_eq!(c2.ret, 1, "C rejected a fixed block");
    assert_eq!(&c2.out[..exp.len()], &exp[..]);

    // fixed with a match
    let toks = vec![
        Tok::Lit(b'a'),
        Tok::Lit(b'b'),
        Tok::Lit(b'c'),
        Tok::Match { len: 5, dist: 3 },
        Tok::Match { len: 258, dist: 1 },
    ];
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &toks);
    let exp = expand(&toks);
    let c3 = d.check("smoke", "fixed match", &Case::new(w.bytes(), exp.len() as i32));
    println!("fixed match -> ret={} len={}", c3.ret, exp.len());
    assert_eq!(c3.ret, 1);
    assert_eq!(&c3.out[..exp.len()], &exp[..]);

    // dynamic
    let payload: Vec<u8> = (0..200u32).map(|i| (i * 7 % 251) as u8).collect();
    let toks: Vec<Tok> = payload.iter().map(|&b| Tok::Lit(b)).collect();
    let mut w = BitWriter::new();
    let info = emit_dynamic(&mut w, true, &toks, &DynOpts::default());
    let exp = expand(&toks);
    let c4 = d.check("smoke", "dynamic literals", &Case::new(w.bytes(), exp.len() as i32));
    println!(
        "dynamic -> ret={} hlit={} hdist={} hclen={} maxlen={} cl_syms={:?}",
        c4.ret, info.hlit, info.hdist, info.hclen, info.max_lit_len, info.cl_syms_used
    );
    assert_eq!(c4.ret, 1, "C rejected a dynamic block: {:?}", c4);
    assert_eq!(&c4.out[..exp.len()], &exp[..]);

    // an abort case: empty input must kill both with SIGABRT
    let c5 = d.check("smoke", "in_bytes=0", &Case::new(vec![], 16));
    println!("empty input -> {:?}", c5);
    assert_eq!(c5.signal, Some(libc::SIGABRT), "C did not abort on empty input");

    d.finish("smoke");
}

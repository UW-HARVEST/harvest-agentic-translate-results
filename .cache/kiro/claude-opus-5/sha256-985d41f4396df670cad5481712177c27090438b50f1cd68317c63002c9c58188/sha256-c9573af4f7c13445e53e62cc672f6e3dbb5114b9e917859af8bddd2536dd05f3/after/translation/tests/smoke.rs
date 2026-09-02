//! Smoke test: validates the harness and the hand-rolled DEFLATE encoder
//! before the full Phase B/C suites rely on them.

mod common;

use common::deflate::*;
use common::*;

#[test]
fn smoke_stored() {
    let data: Vec<u8> = (0..37u8).collect();
    let mut d = Deflate::new();
    d.stored(true, &data);
    let stream = d.finish();
    let out = diff_inflate(
        InflateCase::new(&stream, data.len() + 8),
        CBuild::AsBuilt,
        "smoke_stored",
    );
    assert_eq!(out.ret, 1, "stored block should inflate, err={:?}", out.err);
    assert_eq!(&out.out[..data.len()], &data[..]);
}

#[test]
fn smoke_fixed_literals() {
    let data: Vec<u8> = (0..200u8).map(|i| i.wrapping_mul(7)).collect();
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let mut d = Deflate::new();
    d.fixed(true, &toks);
    let stream = d.finish();
    let out = diff_inflate(
        InflateCase::new(&stream, data.len() + 8),
        CBuild::AsBuilt,
        "smoke_fixed_literals",
    );
    assert_eq!(out.ret, 1, "fixed block failed: {:?}", out.err);
    assert_eq!(&out.out[..data.len()], &data[..]);
}

#[test]
fn smoke_fixed_match() {
    let toks = vec![
        Tok::Lit(b'a'),
        Tok::Lit(b'b'),
        Tok::Lit(b'c'),
        Tok::Match(9, 3),
        Tok::Lit(b'!'),
    ];
    let expected = expand(&toks);
    assert_eq!(&expected, b"abcabcabcabc!");
    let mut d = Deflate::new();
    d.fixed(true, &toks);
    let stream = d.finish();
    let out = diff_inflate(
        InflateCase::new(&stream, expected.len() + 8),
        CBuild::AsBuilt,
        "smoke_fixed_match",
    );
    assert_eq!(out.ret, 1, "fixed match failed: {:?}", out.err);
    assert_eq!(&out.out[..expected.len()], &expected[..]);
}

#[test]
fn smoke_dynamic() {
    let mut rng = Rng::new(1);
    let toks = rand_literals(&mut rng, 300);
    let expected = expand(&toks);
    let lit_lens = lit_lens_for(&toks, 288);
    let dist_lens = dist_lens_for(&toks, 32);
    let mut d = Deflate::new();
    d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
    let stream = d.finish();
    let out = diff_inflate(
        InflateCase::new(&stream, expected.len() + 8),
        CBuild::AsBuilt,
        "smoke_dynamic",
    );
    assert_eq!(out.ret, 1, "dynamic block failed: {:?}", out.err);
    assert_eq!(&out.out[..expected.len()], &expected[..]);
}

#[test]
fn smoke_unfilter() {
    let (w, h, bpp) = (5i32, 4i32, 3i32);
    let stride = (w * bpp + 1) as usize;
    let mut raw = vec![0u8; stride * h as usize];
    let mut rng = Rng::new(2);
    for y in 0..h as usize {
        raw[y * stride] = (y % 5) as u8;
        for x in 1..stride {
            raw[y * stride + x] = rng.byte();
        }
    }
    let r = diff_unfilter(w, h, bpp, &raw, CBuild::AsBuilt, "smoke_unfilter");
    assert_eq!(r.ret, 1);
}

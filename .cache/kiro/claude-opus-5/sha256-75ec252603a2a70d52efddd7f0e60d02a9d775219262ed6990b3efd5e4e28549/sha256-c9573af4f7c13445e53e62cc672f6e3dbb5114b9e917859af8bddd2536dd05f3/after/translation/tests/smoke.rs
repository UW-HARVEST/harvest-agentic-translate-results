mod common;

use common::deflate::{self, Tok};
use common::png::{self, ColorType, PngSpec};
use common::*;

/// Sanity: both libraries load, expose the symbols, and a minimal 1x1 grey PNG
/// decodes identically. Also checks the harness's own DEFLATE writers against
/// the independent reference model.
#[test]
fn smoke_stored_1x1_grey() {
    let mut rng = Rng::new(SEED);
    let w = 1usize;
    let h = 1usize;
    let bpp = 1usize;
    let raw = png::raw_scanlines(&mut rng, w, h, bpp, &[0]);
    let def = deflate::stored_block(&raw, true);
    let spec = PngSpec::new(w as u32, h as u32, 0, def, raw.clone());
    let file = spec.build();

    let (c, r) = call_load_png(&file);
    eprintln!("C  : {}", c.head());
    eprintln!("RS : {}", r.head());
    assert_same("smoke 1x1 grey stored", &c, &r);
    assert!(!c.pix_null, "C failed to decode: {}", c.err_str());

    let mut u = raw.clone();
    assert!(png::model_unfilter(w, h, bpp, &mut u));
    let expect = png::model_pixels(w, h, bpp, 0, &u, None, None);
    assert_eq!(c.payload, expect, "C output disagrees with reference model");
}

#[test]
fn smoke_fixed_huffman_inflate() {
    let mut rng = Rng::new(SEED ^ 1);
    let data = rng.bytes(200);
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let def = deflate::fixed_stream(&toks);
    let (c, r) = call_inflate(&def, def.len() as i32, data.len() as i32);
    eprintln!("C  : {}", c.head());
    eprintln!("RS : {}", r.head());
    assert_same("smoke fixed inflate", &c, &r);
    assert_eq!(c.ret, 1, "C cp_inflate failed: {}", c.err_str());
    assert_eq!(c.payload, data, "C output disagrees with reference model");
}

#[test]
fn smoke_dynamic_huffman_inflate() {
    let mut rng = Rng::new(SEED ^ 2);
    let data = rng.bytes(500);
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let def = deflate::dynamic_stream(&toks, 15, 288, 2, true);
    let (c, r) = call_inflate(&def, def.len() as i32, data.len() as i32);
    eprintln!("C  : {}", c.head());
    eprintln!("RS : {}", r.head());
    assert_same("smoke dynamic inflate", &c, &r);
    assert_eq!(c.ret, 1, "C cp_inflate failed: {}", c.err_str());
    assert_eq!(c.payload, data, "C output disagrees with reference model");
}

#[test]
fn smoke_all_color_types() {
    let mut rng = Rng::new(SEED ^ 3);
    for ct in ColorType::ALL {
        let (w, h) = (5usize, 4usize);
        let bpp = ct.bpp();
        let filters = vec![0u8; h];
        let raw = png::raw_scanlines(&mut rng, w, h, bpp, &filters);
        let def = deflate::stored_block(&raw, true);
        let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw.clone());
        if ct == ColorType::Indexed {
            spec.plte = Some(rng.bytes(256 * 3));
        }
        let file = spec.build();
        let (c, r) = call_load_png(&file);
        assert_same(&format!("smoke color_type={}", ct as u8), &c, &r);
        assert!(!c.pix_null, "ct={} failed: {}", ct as u8, c.err_str());

        let mut u = raw.clone();
        assert!(png::model_unfilter(w, h, bpp, &mut u));
        let expect = png::model_pixels(
            w,
            h,
            bpp,
            ct as u8,
            &u,
            spec.plte.as_deref(),
            spec.trns.as_deref(),
        );
        assert_eq!(c.payload, expect, "ct={} model mismatch", ct as u8);
    }
}

#[test]
fn smoke_assert_is_observable() {
    // cp_inflate with in_bytes == 0 ⇒ bits_left == 0 ⇒ assert(s->bits_left > 0)
    // ⇒ SIGABRT. Both implementations must die the same way.
    let (c, r) = call_inflate(&[0u8; 8], 0, 16);
    eprintln!("C  : {}", c.head());
    eprintln!("RS : {}", r.head());
    assert_eq!(c.signal, Some(6), "expected C to SIGABRT");
    assert_same("in_bytes == 0", &c, &r);
}

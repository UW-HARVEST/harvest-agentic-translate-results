mod common;

use common::deflate::*;
use common::*;

#[test]
fn smoke_symbols_load() {
    let p = load_pair();
    assert_eq!(p.c.snapshot_tables(), p.rs.snapshot_tables(), "table parity");
}

#[test]
fn smoke_stored() {
    let p = load_pair();
    let payload = b"hello stored world".to_vec();
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, true, &payload);
    let stream = bw.finish();
    let r = diff_inflate(&p, "smoke_stored", &stream, 0, payload.len() as i32);
    assert_eq!(r.ret, 1, "C rejected a well-formed stored block: {:?}", r.reason);
    assert_eq!(&r.out[..payload.len()], &payload[..]);
}

#[test]
fn smoke_fixed_literals() {
    let p = load_pair();
    let data: Vec<u8> = (0u8..=255).collect();
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, true, &toks);
    let stream = bw.finish();
    let r = diff_inflate(&p, "smoke_fixed", &stream, 0, data.len() as i32);
    assert_eq!(r.ret, 1, "C rejected fixed block: {:?}", r.reason);
    assert_eq!(&r.out[..data.len()], &data[..]);
}

#[test]
fn smoke_fixed_match() {
    let p = load_pair();
    let toks = vec![
        Tok::Lit(b'a'),
        Tok::Lit(b'b'),
        Tok::Match { len: 10, dist: 2 },
        Tok::Match { len: 258, dist: 1 },
    ];
    let expect = apply_tokens(&toks);
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, true, &toks);
    let stream = bw.finish();
    let r = diff_inflate(&p, "smoke_fixed_match", &stream, 0, expect.len() as i32);
    assert_eq!(r.ret, 1, "C rejected: {:?}", r.reason);
    assert_eq!(&r.out[..expect.len()], &expect[..]);
}

#[test]
fn smoke_dynamic() {
    let p = load_pair();
    let toks = vec![
        Tok::Lit(1),
        Tok::Lit(2),
        Tok::Lit(3),
        Tok::Match { len: 7, dist: 3 },
    ];
    let expect = apply_tokens(&toks);
    let spec = dynamic_spec_for(&toks, TreeShape::Balanced, RepeatMode::All, 288, 30, None);
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, true, &spec, &toks);
    let stream = bw.finish();
    let r = diff_inflate(&p, "smoke_dynamic", &stream, 0, expect.len() as i32);
    assert_eq!(r.ret, 1, "C rejected dynamic block: {:?}", r.reason);
    assert_eq!(&r.out[..expect.len()], &expect[..]);
}

#[test]
fn smoke_flate2() {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let p = load_pair();
    let data: Vec<u8> = (0..5000u32).map(|i| (i % 37) as u8).collect();
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(6));
    e.write_all(&data).unwrap();
    let stream = e.finish().unwrap();
    let r = diff_inflate(&p, "smoke_flate2", &stream, 0, data.len() as i32);
    assert_eq!(r.ret, 1, "C rejected zlib stream: {:?}", r.reason);
    assert_eq!(&r.out[..data.len()], &data[..]);
}

#[test]
fn smoke_convert_pix() {
    let p = load_pair();
    let (w, h, bpp) = (5i32, 3i32, 4i32);
    let src: Vec<u8> = (0..(h as usize * (1 + w as usize * bpp as usize)))
        .map(|i| (i * 7 + 1) as u8)
        .collect();
    diff_convert_pix(&p, "smoke_convert_pix", bpp, w, h, &src, (w * h) as usize, 0);
}

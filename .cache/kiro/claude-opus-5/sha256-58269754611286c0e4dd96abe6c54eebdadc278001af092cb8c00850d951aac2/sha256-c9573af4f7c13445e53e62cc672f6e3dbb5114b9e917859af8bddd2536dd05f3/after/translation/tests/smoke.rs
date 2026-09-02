mod common;

use common::deflate::*;
use common::rng::{Rng, SEED};
use common::*;

#[test]
fn smoke_tables_match() {
    let p = load_pair();
    assert_eq!(p.c.fixed_table(), p.rust.fixed_table());
    assert_eq!(p.c.fixed_table(), fixed_table());
    assert_eq!(p.c.permutation_order(), p.rust.permutation_order());
    assert_eq!(p.c.len_base(), p.rust.len_base());
    assert_eq!(p.c.dist_base(), p.rust.dist_base());
    println!("tables ok");
}

#[test]
fn smoke_stored() {
    let p = load_pair();
    for len in [0usize, 1, 3, 4, 5, 16, 300] {
        let mut rng = Rng::new(SEED + len as u64);
        let payload = rng.bytes(len);
        let mut w = BitWriter::new();
        emit_stored(&mut w, true, &payload);
        let stream = w.finish();
        let c = run_inflate(&p.c, &stream, 0, len.max(1) + 8, None);
        let r = run_inflate(&p.rust, &stream, 0, len.max(1) + 8, None);
        println!("LEN={len} C: {c:?}");
        println!("LEN={len} R: {r:?}");
        assert_eq!(c.ret, r.ret);
        assert_eq!(c.err, r.err);
        assert_eq!(c.out, r.out);
    }
}

#[test]
fn smoke_fixed() {
    let p = load_pair();
    let ops: Vec<Op> = b"hello hello hello world"
        .iter()
        .map(|&b| Op::Lit(b))
        .collect();
    let expect = expand(&ops);
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &ops);
    let stream = w.finish();
    let c = run_inflate(&p.c, &stream, 0, expect.len() + 4, None);
    let r = run_inflate(&p.rust, &stream, 0, expect.len() + 4, None);
    println!("fixed C: {c:?}");
    println!("fixed R: {r:?}");
    assert_eq!(&c.out[..expect.len()], &expect[..], "encoder self-check");
    assert_eq!(c.ret, r.ret);
    assert_eq!(c.err, r.err);
    assert_eq!(c.out, r.out);
}

#[test]
fn smoke_fixed_match() {
    let p = load_pair();
    let mut ops: Vec<Op> = b"abcdefgh".iter().map(|&b| Op::Lit(b)).collect();
    ops.push(Op::Match { len: 5, dist: 8 });
    ops.push(Op::Match { len: 10, dist: 1 });
    ops.push(Op::Match { len: 7, dist: 3 });
    let expect = expand(&ops);
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &ops);
    let stream = w.finish();
    let c = run_inflate(&p.c, &stream, 0, expect.len() + 4, None);
    let r = run_inflate(&p.rust, &stream, 0, expect.len() + 4, None);
    println!("match C: {c:?}");
    println!("match R: {r:?}");
    assert_eq!(&c.out[..expect.len()], &expect[..], "encoder self-check");
    assert_eq!(c.ret, r.ret);
    assert_eq!(c.out, r.out);
}

#[test]
fn smoke_dynamic() {
    let p = load_pair();
    let mut rng = Rng::new(SEED);
    let mut ops: Vec<Op> = b"the quick brown fox".iter().map(|&b| Op::Lit(b)).collect();
    ops.push(Op::Match { len: 6, dist: 10 });
    let expect = expand(&ops);
    let d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::none(), 288, 30);
    let mut w = BitWriter::new();
    emit_dynamic(&mut w, true, &d, &ops);
    let stream = w.finish();
    let c = run_inflate(&p.c, &stream, 0, expect.len() + 4, None);
    let r = run_inflate(&p.rust, &stream, 0, expect.len() + 4, None);
    println!("dyn C: {c:?}");
    println!("dyn R: {r:?}");
    assert_eq!(&c.out[..expect.len()], &expect[..], "encoder self-check");
    assert_eq!(c.ret, r.ret);
    assert_eq!(c.out, r.out);
}

#[test]
fn smoke_convert_pix() {
    let p = load_pair();
    let mut rng = Rng::new(SEED);
    for bpp in 1..=4 {
        let (w, h) = (5, 3);
        let src = rng.bytes(h * (1 + w * bpp as usize) + 8);
        diff_convert_pix(&p, "smoke", bpp, w as i32, h as i32, &src, w * h);
    }
}

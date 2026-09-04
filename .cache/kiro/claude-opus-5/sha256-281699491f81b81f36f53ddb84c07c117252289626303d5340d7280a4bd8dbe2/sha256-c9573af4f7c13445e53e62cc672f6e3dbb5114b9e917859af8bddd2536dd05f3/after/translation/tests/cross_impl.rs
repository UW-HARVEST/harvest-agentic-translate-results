//! CONFIGS.md rows 107,108 — cross-implementation interoperability.
//!
//! Data compressed by one `.so` must decompress byte-identically through the
//! other, in both directions, for every module.
#![allow(non_snake_case)]

mod common;
use common::frame::*;
use common::*;
use libloading::Library;

type FnCompress4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnHC5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnDecSafe = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;

fn block_compress(lib: &Library, src: &[u8]) -> Vec<u8> {
    unsafe {
        let f = sym::<FnCompress4>(lib, "LZ4_compress_default");
        let bound = compress_bound(src.len() as i32).max(1);
        let mut d = vec![0u8; bound as usize];
        let n = f(src.as_ptr(), d.as_mut_ptr(), src.len() as i32, bound);
        assert!(n > 0, "compress failed for len {}", src.len());
        d.truncate(n as usize);
        d
    }
}

fn hc_compress(lib: &Library, src: &[u8], lvl: i32) -> Vec<u8> {
    unsafe {
        let f = sym::<FnHC5>(lib, "LZ4_compress_HC");
        let bound = compress_bound(src.len() as i32).max(1);
        let mut d = vec![0u8; bound as usize];
        let n = f(src.as_ptr(), d.as_mut_ptr(), src.len() as i32, bound, lvl);
        assert!(n > 0);
        d.truncate(n as usize);
        d
    }
}

fn block_decompress(lib: &Library, comp: &[u8], out_len: usize) -> (i32, Vec<u8>) {
    unsafe {
        let f = sym::<FnDecSafe>(lib, "LZ4_decompress_safe");
        let mut o = vec![0u8; out_len + 16];
        let n = f(
            comp.as_ptr(),
            o.as_mut_ptr(),
            comp.len() as i32,
            (out_len + 16) as i32,
        );
        o.truncate(if n > 0 { n as usize } else { 0 });
        (n, o)
    }
}

#[test]
fn cross_block_api() {
    let mut rng = Rng::new(0xC0DE_0001);
    let i = impls();
    for &shape in ALL_SHAPES.iter() {
        for &len in [1usize, 4, 100, 65535, 65536, 200000].iter() {
            let src = mkdata(shape, len, &mut rng);
            let c = block_compress(&i.c, &src);
            let r = block_compress(&i.r, &src);
            assert_eq!(c, r, "block bytes differ {shape:?} len={len}");
            for comp in [&c, &r] {
                for lib in [&i.c, &i.r] {
                    let (n, o) = block_decompress(lib, comp, len);
                    assert_eq!(n as usize, len);
                    assert_eq!(&o[..], &src[..]);
                }
            }
        }
    }
}

#[test]
fn cross_hc_api() {
    let mut rng = Rng::new(0xC0DE_0002);
    let i = impls();
    for &shape in ALL_SHAPES.iter() {
        for &len in [1usize, 100, 65536, 150000].iter() {
            for &lvl in [1i32, 3, 9, 10, 12].iter() {
                let src = mkdata(shape, len, &mut rng);
                let c = hc_compress(&i.c, &src, lvl);
                let r = hc_compress(&i.r, &src, lvl);
                assert_eq!(c, r, "HC bytes differ {shape:?} len={len} lvl={lvl}");
                for comp in [&c, &r] {
                    for lib in [&i.c, &i.r] {
                        let (n, o) = block_decompress(lib, comp, len);
                        assert_eq!(n as usize, len);
                        assert_eq!(&o[..], &src[..]);
                    }
                }
            }
        }
    }
}

#[test]
fn cross_frame_api() {
    let mut rng = Rng::new(0xC0DE_0003);
    let i = impls();
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(4) {
        for &len in [0usize, 1, 1000, 70000, 200000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let fc = compress_frame(&i.c, &src, Some(p), 0).frame;
            let fr = compress_frame(&i.r, &src, Some(p), 0).frame;
            assert_eq!(fc, fr, "frame bytes differ prefs#{pi} len={len}");
            for frame in [&fc, &fr] {
                for lib in [&i.c, &i.r] {
                    for sc in [0usize, 1, 4096] {
                        let d = decompress_frame(lib, frame, len, sc, 0, None, None, false);
                        assert_eq!(&d.out[..], &src[..], "prefs#{pi} len={len} sc={sc}");
                    }
                }
            }
        }
    }
}

#[test]
fn cross_frame_streaming() {
    let mut rng = Rng::new(0xC0DE_0004);
    let i = impls();
    let prefs = pref_matrix();
    let dict = mkdata(Shape::Textish, 8192, &mut rng);
    for (pi, p) in prefs.iter().enumerate().step_by(6) {
        let src = mkdata(Shape::Textish, 180000, &mut rng);
        for mode in 0..3 {
            let begin = match mode {
                0 => BeginMode::Plain,
                1 => BeginMode::UsingDict(&dict),
                _ => BeginMode::UsingCDict(&dict),
            };
            let plan = StreamPlan {
                begin,
                prefs: Some(*p),
                copts: None,
                steps: (0..30).map(|_| (7777usize, UpdKind::Compressed, false)).collect(),
            };
            let fc = compress_stream(&i.c, &src, &plan).frame;
            let fr = compress_stream(&i.r, &src, &plan).frame;
            assert_eq!(fc, fr, "stream bytes differ prefs#{pi} mode={mode}");
            let d = if mode == 0 { None } else { Some(&dict[..]) };
            for frame in [&fc, &fr] {
                for lib in [&i.c, &i.r] {
                    let out = decompress_frame(lib, frame, src.len(), 4096, 0, None, d, false);
                    assert_eq!(&out.out[..], &src[..], "prefs#{pi} mode={mode}");
                }
            }
        }
    }
}

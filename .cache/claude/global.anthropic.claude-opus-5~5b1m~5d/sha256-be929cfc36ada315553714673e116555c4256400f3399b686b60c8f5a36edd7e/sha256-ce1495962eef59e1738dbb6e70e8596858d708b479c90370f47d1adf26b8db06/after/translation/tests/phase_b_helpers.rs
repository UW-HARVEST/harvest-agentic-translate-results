//! Phase B rows B55-B58: `strkey`, `intput`, and the composed binary pipeline.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

fn k32(v: i32) -> [u8; 4] {
    v.to_ne_bytes()
}

fn mk_elem(key: &[u8], elemsize: usize, fill: &[u8]) -> Vec<u8> {
    let mut e = vec![0u8; elemsize];
    e[..key.len()].copy_from_slice(key);
    let n = (elemsize - key.len()).min(fill.len());
    e[key.len()..key.len() + n].copy_from_slice(&fill[..n]);
    e
}

/// B55 — `strkey` over the whole interesting int range
#[test]
fn cfg_b55_strkey() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut fixed: Vec<i32> = vec![
            0,
            1,
            -1,
            2,
            9,
            10,
            11,
            99,
            100,
            -99,
            -100,
            999,
            1000,
            -1000,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
        ];
        let mut rng = Rng::new(55);
        for _ in 0..200 {
            fixed.push(rng.i32());
        }
        for n in fixed {
            let pc = (c.strkey)(n as c_int);
            let sc = read_cstr(pc);
            let pr = (r.strkey)(n as c_int);
            let sr = read_cstr(pr);
            let want = format!("test_{n}").into_bytes();
            assert_eq!(sc, want, "B55 C strkey({n})");
            assert_eq!(sr, want, "B55 RUST strkey({n})");
        }
    });
}

/// B56 — two consecutive `strkey` calls share the same static buffer
#[test]
fn cfg_b56_strkey_static_buffer() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let c1 = (c.strkey)(1234);
        let c2 = (c.strkey)(-7);
        assert_eq!(c1, c2, "B56 C returns the same static buffer");
        assert_eq!(read_cstr(c1), b"test_-7".to_vec(), "B56 C clobbered");
        let r1 = (r.strkey)(1234);
        let r2 = (r.strkey)(-7);
        assert_eq!(r1, r2, "B56 RUST returns the same static buffer");
        assert_eq!(read_cstr(r1), b"test_-7".to_vec(), "B56 RUST clobbered");
        // long value then short value: the short one must be NUL terminated in
        // the middle of the previous, longer, digits
        for &(a, b) in &[
            (i32::MIN, 0i32),
            (1234567890, 1),
            (-1234567890, -1),
            (999999999, 9),
        ] {
            (c.strkey)(a);
            let sc = read_cstr((c.strkey)(b));
            (r.strkey)(a);
            let sr = read_cstr((r.strkey)(b));
            assert_eq!(sc, format!("test_{b}").into_bytes(), "B56 C {a}→{b}");
            assert_eq!(sr, format!("test_{b}").into_bytes(), "B56 RUST {a}→{b}");
        }
    });
}

/// B57 — `intput` for every `num ∉ {9, 11}` must return normally, and must
/// advance the global hash seed identically in both libraries (it builds one
/// fresh table internally).
#[test]
fn cfg_b57_intput_ok() {
    let mut nums: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        3,
        7,
        8,
        10,
        12,
        13,
        100,
        -100,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    let mut rng = Rng::new(57);
    while nums.len() < 216 {
        let n = rng.i32();
        if n != 9 && n != 11 {
            nums.push(n);
        }
    }
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for n in nums {
            (c.intput)(n as c_int);
            (r.intput)(n as c_int);
            // the seed must have advanced in lock-step: build a table and compare
            let mut mc = Map::new(c, Shape::binary(8, 4));
            let mut mr = Map::new(r, Shape::binary(8, 4));
            let mut kc = k32(5);
            let mut kr = k32(5);
            let e = mk_elem(&k32(5), 8, &k32(5));
            mc.put_struct(kc.as_mut_ptr() as *mut c_void, &e, HM_BINARY);
            mr.put_struct(kr.as_mut_ptr() as *mut c_void, &e, HM_BINARY);
            assert_eq!(
                mc.snapshot(),
                mr.snapshot(),
                "B57 seed diverged after intput({n})"
            );
            mc.free();
            mr.free();
        }
    });
}

/// B58 — composed binary pipeline over several `rand_seed` values
#[test]
fn cfg_b58_binary_pipeline() {
    for &seed in &[
        0usize,
        1,
        0x3141_5926,
        0xdead_beef,
        usize::MAX,
        0x1234_5678_9abc_def0,
    ] {
        with_libs(seed, |c, r| unsafe {
            let mut rng = Rng::new(58 ^ seed as u64);
            let mut p = Pair::new(c, r, Shape::binary(16, 8));
            let keys: Vec<[u8; 8]> = (0..90u64).map(|i| (i * 0x9E37_79B9).to_ne_bytes()).collect();
            for (i, k) in keys.iter().enumerate().take(60) {
                let e = mk_elem(k, 16, &(i as u64).to_ne_bytes());
                p.put_struct(k, &e, HM_BINARY, &format!("B58 s={seed:#x} put {i}"));
            }
            for i in (0..60).step_by(3) {
                p.del(&keys[i], HM_BINARY, &format!("B58 s={seed:#x} del {i}"));
            }
            for (i, k) in keys.iter().enumerate().skip(60) {
                let e = mk_elem(k, 16, &(rng.next_u64()).to_ne_bytes());
                p.put_struct(k, &e, HM_BINARY, &format!("B58 s={seed:#x} put2 {i}"));
            }
            for (i, k) in keys.iter().enumerate() {
                p.geti(k, HM_BINARY, &format!("B58 s={seed:#x} get {i}"));
            }
            p.defaults(&[0xEEu8; 16], &format!("B58 s={seed:#x} defaults"));
            p.free(&format!("B58 s={seed:#x} free"));
        });
    }
}

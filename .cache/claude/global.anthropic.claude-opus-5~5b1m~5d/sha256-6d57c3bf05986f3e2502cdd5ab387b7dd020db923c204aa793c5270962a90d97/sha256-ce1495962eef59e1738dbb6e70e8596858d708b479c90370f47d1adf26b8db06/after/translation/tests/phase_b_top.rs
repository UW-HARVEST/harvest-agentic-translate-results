//! Phase B — CONFIGS.md rows 48..50: the two non-`stbds_` entry points,
//! `strkey` (static buffer + `sprintf`) and `helxo` (the `lib.h` API, which
//! prints to stdout).

mod common;

use common::*;
use std::ffi::c_int;

unsafe fn read_cstr(p: *const std::ffi::c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let b = *(p.offset(i) as *const u8);
        if b == 0 {
            return v;
        }
        v.push(b);
        i += 1;
        assert!(i < 4096);
    }
}

// row 48 -----------------------------------------------------------------------
#[test]
fn cfg_48_strkey() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0x48);
    unsafe {
        let mut ns: Vec<c_int> = vec![
            0,
            1,
            -1,
            9,
            10,
            99,
            100,
            999,
            1000,
            c_int::MAX,
            c_int::MIN,
            c_int::MIN + 1,
            -999_999_999,
        ];
        for _ in 0..200 {
            ns.push(rng.next_u64() as u32 as i32);
        }
        for n in ns {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            let sc = read_cstr(pc);
            let sr = read_cstr(pr);
            assert_eq!(
                sc,
                sr,
                "strkey({}) C={:?} Rust={:?}",
                n,
                String::from_utf8_lossy(&sc),
                String::from_utf8_lossy(&sr)
            );
            assert_eq!(sc, format!("test_{}", n).into_bytes());
            // the returned pointer must be the same static buffer every time
            assert_eq!(pc, (c.strkey)(n));
            assert_eq!(pr, (r.strkey)(n));
        }
        // the buffer must keep the *last* value (static storage, no per-call copy)
        let p = (c.strkey)(1234);
        let q = (r.strkey)(1234);
        (c.strkey)(-77);
        (r.strkey)(-77);
        assert_eq!(read_cstr(p), b"test_-77".to_vec());
        assert_eq!(read_cstr(q), b"test_-77".to_vec());
    }
}

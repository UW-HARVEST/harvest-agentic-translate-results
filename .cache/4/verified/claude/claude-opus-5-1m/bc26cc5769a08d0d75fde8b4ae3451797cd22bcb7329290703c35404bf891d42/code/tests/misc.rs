//! Phase B — CONFIGS.md row 45: `strkey`.

mod common;
use common::*;

#[test]
fn strkey_matrix() {
    let _g = lock();
    let (c, r) = pair();
    let mut cases: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        -10,
        99,
        100,
        -100,
        999,
        1000,
        12345,
        -12345,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    let mut rng = Rng::new(0x4501);
    for _ in 0..200 {
        cases.push(rng.next_u32() as i32);
    }
    unsafe {
        // the returned pointer must be the library's own static buffer, i.e.
        // stable across calls
        let p0c = (c.strkey)(0);
        let p0r = (r.strkey)(0);
        for &n in &cases {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_eq!(pc, p0c, "C strkey must return the same static buffer");
            assert_eq!(pr, p0r, "RUST strkey must return the same static buffer");
            let sc = cstr(pc).expect("C returned NULL");
            let sr = cstr(pr).expect("RUST returned NULL");
            assert_eq!(sc, sr, "strkey({n}) diverged");
            assert_eq!(
                sc,
                format!("test_{n}").into_bytes(),
                "strkey({n}) wrong content"
            );
        }
        // the two libraries must NOT share the buffer (each has its own static)
        assert_ne!(p0c, p0r, "the two libraries must have separate static buffers");
        // trailing garbage: a shorter number must not leave the previous digits
        let _ = (c.strkey)(1234567);
        let _ = (r.strkey)(1234567);
        let a = cstr((c.strkey)(7)).unwrap();
        let b = cstr((r.strkey)(7)).unwrap();
        assert_eq!(a, b);
        assert_eq!(a, b"test_7".to_vec());
    }
}

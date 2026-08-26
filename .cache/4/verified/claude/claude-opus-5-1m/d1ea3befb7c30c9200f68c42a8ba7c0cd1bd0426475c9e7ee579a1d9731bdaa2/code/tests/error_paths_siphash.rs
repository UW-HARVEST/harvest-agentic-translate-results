//! Phase C — `ERRORS.md` row 11: `siphash`'s `int init` boundary values.
//!
//! `siphash` writes to fd 1, so (like `differential_siphash.rs`) this binary
//! contains exactly one `#[test]`, guaranteeing nothing else writes to fd 1 while
//! a capture window is open.

mod common;

use common::{diff_siphash, impls, Rng};
use std::ffi::c_void;

fn expected_text(init: i32) -> String {
    let (c, _) = impls();
    let mut mem = [0u8; 64];
    let mut z: i32 = init;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }
    let mut s = String::new();
    for i in 0..64usize {
        let h = unsafe { (c.hash_bytes)(mem.as_mut_ptr() as *mut c_void, i, 0) };
        s.push_str("  { ");
        for j in 0..8usize {
            s.push_str(&format!("0x{:02x}, ", ((h >> (j * 8)) & 255) as u8));
        }
        s.push_str(" },\n");
    }
    s
}

#[test]
fn row11_siphash_int_extremes() {
    // `init` is a plain `int`, not an enum: every one of the 2^32 values is a
    // valid input and none is rejected. These are the boundary / one-step-past
    // values plus a deterministic random spread.
    let mut boundaries: Vec<i32> = vec![
        0,
        1,
        -1,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 63,
        i32::MIN + 64,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 63,
        i32::MAX - 64,
        127,
        128,
        129,
        255,
        256,
        -127,
        -128,
        -129,
        -255,
        -256,
        0x7fff,
        -0x8000,
        0x00ff_ff00,
        -0x00ff_ff00,
    ];
    let mut rng = Rng::new(0xE011);
    for _ in 0..32 {
        boundaries.push(rng.next_i32());
    }

    for init in boundaries {
        let got = diff_siphash(init);
        let text = String::from_utf8(got).expect("output must be ASCII");
        assert_eq!(
            text.lines().count(),
            64,
            "row11 init={init}: expected 64 lines"
        );
        assert_eq!(
            text,
            expected_text(init),
            "row11 init={init}: captured output disagrees with an independent \
             recomputation via the C stbds_hash_bytes"
        );
        eprintln!("  [ERRORS.md row 11] init={init} ok");
    }
}

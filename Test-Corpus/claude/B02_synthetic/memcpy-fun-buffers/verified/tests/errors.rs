//! Phase C — error-path differential tests.
//!
//! One test (or one clearly-labelled block) per row of ERRORS.md. Each
//! constructs the exact invalid input the C code checks for, calls BOTH shared
//! objects through their exported symbols, and asserts they return the SAME
//! error code / sentinel AND write the same bytes to stderr.

mod common;

use common::*;
use core::ffi::c_int;
use core::ptr::{null, null_mut};

/// Rows 1–39 all concern the library functions; rows 40–54 concern `main` and
/// live in `tests/so_main_diff.rs` (they need a whole stdin stream).

// ============================================================ rows 1–3 =====
// validate_buffer

#[test]
fn row01_validate_buffer_null() {
    let (c, r) = both();
    let co = observe(None, || unsafe { (c.validate_buffer)(null()) });
    let ro = observe(None, || unsafe { (r.validate_buffer)(null()) });
    same("ERRORS row1: validate_buffer(NULL)", &co, &ro);
    assert_eq!(co.ret, 0, "C must return false");
    assert_eq!(co.stderr, b"Error: NULL buffer\n".to_vec());
}

#[test]
fn row02_validate_buffer_length_over_maximum() {
    let (c, r) = both();
    for len in [
        257usize,
        258,
        300,
        512,
        1000,
        65536,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ] {
        let mut b = BufferT::patterned(0x42);
        b.length = len;
        let cb = b;
        let rb = b;
        let co = observe(None, || unsafe { (c.validate_buffer)(&cb) });
        let ro = observe(None, || unsafe { (r.validate_buffer)(&rb) });
        same(&format!("ERRORS row2: length={}", len), &co, &ro);
        assert_eq!(co.ret, 0);
        assert_eq!(
            co.stderr,
            format!("Error: Buffer length {} exceeds maximum 256\n", len).into_bytes()
        );
    }
    // exactly 256 must be accepted (boundary, not an error)
    let mut b = Rng::new(2).buffer_len(256);
    b.checksum = checksum(&b.data);
    let cb = b;
    let rb = b;
    let co = observe(None, || unsafe { (c.validate_buffer)(&cb) });
    let ro = observe(None, || unsafe { (r.validate_buffer)(&rb) });
    same("ERRORS row2: length=256 boundary", &co, &ro);
    assert_eq!(co.ret, 1);
    assert!(co.stderr.is_empty());
}

#[test]
fn row03_validate_buffer_checksum_mismatch_warns_but_succeeds() {
    let (c, r) = both();
    let mut rng = Rng::new(3);
    for _ in 0..60 {
        let mut b = rng.buffer(0, 256);
        let expected = checksum(&b.data[..b.length]);
        b.checksum = expected.wrapping_add(1 + rng.next_u32() % 1000);
        let cb = b;
        let rb = b;
        let co = observe(None, || unsafe { (c.validate_buffer)(&cb) });
        let ro = observe(None, || unsafe { (r.validate_buffer)(&rb) });
        same("ERRORS row3", &co, &ro);
        assert_eq!(co.ret, 1, "mismatch is only a warning, still returns true");
        assert_eq!(
            co.stderr,
            format!(
                "Warning: Checksum mismatch. Expected {}, got {}\n",
                expected, b.checksum
            )
            .into_bytes()
        );
    }
}

// ============================================================ rows 4–7 =====
// init_buffer_array / free_buffer_array

#[test]
fn row04_init_buffer_array_non_positive_capacity() {
    let (c, r) = both();
    for cap in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        let co = observe(None, || unsafe {
            let a = (c.init_buffer_array)(cap);
            a.is_null()
        });
        let ro = observe(None, || unsafe {
            let a = (r.init_buffer_array)(cap);
            a.is_null()
        });
        same(&format!("ERRORS row4: cap={}", cap), &co, &ro);
        assert!(co.ret, "must return NULL");
        assert_eq!(
            co.stderr,
            format!("Error: Invalid capacity {}\n", cap).into_bytes()
        );
    }
    // capacity 1 is the smallest accepted value (boundary)
    let co = observe(None, || unsafe {
        let a = (c.init_buffer_array)(1);
        let ok = !a.is_null();
        (c.free_buffer_array)(a);
        ok
    });
    assert!(co.ret);
    assert!(co.stderr.is_empty());
}

#[test]
fn row06_init_buffer_array_storage_allocation_failure() {
    // 272 bytes * INT_MAX ≈ 584 GB: the second malloc is expected to fail, which
    // takes the `Failed to allocate buffer storage` branch (and frees `arr`).
    // If the platform happens to satisfy the request both sides simply succeed;
    // either way they must agree.
    let (c, r) = both();
    for cap in [i32::MAX, i32::MAX - 1, 0x4000_0000, 0x2000_0000] {
        let co = observe(None, || unsafe {
            let a = (c.init_buffer_array)(cap);
            let null = a.is_null();
            if !null {
                (c.free_buffer_array)(a);
            }
            null
        });
        let ro = observe(None, || unsafe {
            let a = (r.init_buffer_array)(cap);
            let null = a.is_null();
            if !null {
                (r.free_buffer_array)(a);
            }
            null
        });
        same(&format!("ERRORS row6: cap={}", cap), &co, &ro);
        if co.ret {
            assert_eq!(
                co.stderr,
                b"Error: Failed to allocate buffer storage\n".to_vec()
            );
        }
    }
}

#[test]
fn row07_free_buffer_array_null_is_a_noop() {
    let (c, r) = both();
    let co = observe(None, || unsafe { (c.free_buffer_array)(null_mut()) });
    let ro = observe(None, || unsafe { (r.free_buffer_array)(null_mut()) });
    same("ERRORS row7: free_buffer_array(NULL)", &co, &ro);
    assert!(co.stderr.is_empty() && co.stdout.is_empty());
    // and twice in a row, to be sure nothing is remembered
    let co = observe(None, || unsafe {
        (c.free_buffer_array)(null_mut());
        (c.free_buffer_array)(null_mut());
    });
    let ro = observe(None, || unsafe {
        (r.free_buffer_array)(null_mut());
        (r.free_buffer_array)(null_mut());
    });
    same("ERRORS row7: twice", &co, &ro);
}

// =========================================================== rows 8–11 =====
// buffer_copy

#[test]
fn row08_09_10_buffer_copy_null_pointers() {
    let (c, r) = both();
    let good = Rng::new(8).buffer(0, 100);
    // (src_null, dst_null) for all three failing combinations
    for (sn, dn) in [(true, false), (false, true), (true, true)] {
        let mut cb = good;
        let mut rb = good;
        let mut cd = BufferT::patterned(0x81);
        let mut rd = BufferT::patterned(0x81);
        let co = observe(None, || unsafe {
            let s = if sn { null() } else { &cb as *const BufferT };
            let d = if dn { null_mut() } else { &mut cd as *mut BufferT };
            (c.buffer_copy)(s, d) as i64
        });
        let ro = observe(None, || unsafe {
            let s = if sn { null() } else { &rb as *const BufferT };
            let d = if dn { null_mut() } else { &mut rd as *mut BufferT };
            (r.buffer_copy)(s, d) as i64
        });
        same(
            &format!("ERRORS rows8-10: src_null={} dst_null={}", sn, dn),
            &co,
            &ro,
        );
        assert_eq!(co.ret, -1);
        assert_eq!(co.stderr, b"Error: NULL pointer in buffer_copy\n".to_vec());
        // untouched
        let _ = (&mut cb, &mut rb);
        same_buf("ERRORS rows8-10 dst untouched", &cd, &rd);
    }
}

#[test]
fn row11_buffer_copy_rejects_oversized_source() {
    let (c, r) = both();
    for len in [257usize, 512, usize::MAX] {
        let mut s = BufferT::patterned(0x11);
        s.length = len;
        diff_bufs(
            &format!("ERRORS row11: len={}", len),
            || {
                let mut s2 = BufferT::patterned(0x11);
                s2.length = len;
                vec![s2, BufferT::patterned(0x12)]
            },
            |api, p| unsafe { (api.buffer_copy)(p, p.add(1)) as i64 },
            true,
        );
        let cs = s;
        let mut cd = BufferT::patterned(0x12);
        let co = observe(None, || unsafe { (c.buffer_copy)(&cs, &mut cd) as i64 });
        let mut rd = BufferT::patterned(0x12);
        let ro = observe(None, || unsafe { (r.buffer_copy)(&s, &mut rd) as i64 });
        same(&format!("ERRORS row11/direct len={}", len), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Buffer length {} exceeds maximum 256\n", len).into_bytes(),
            "only validate_buffer's message, no second one"
        );
    }
}

// ========================================================== rows 12–13 =====
// buffer_reverse

#[test]
fn row12_buffer_reverse_null() {
    let (c, r) = both();
    let co = observe(None, || unsafe { (c.buffer_reverse)(null_mut()) as i64 });
    let ro = observe(None, || unsafe { (r.buffer_reverse)(null_mut()) as i64 });
    same("ERRORS row12", &co, &ro);
    assert_eq!(co.ret, -1);
    assert_eq!(co.stderr, b"Error: NULL buffer in reverse\n".to_vec());
}

#[test]
fn row13_buffer_reverse_empty_leaves_checksum_alone() {
    let mut rng = Rng::new(13);
    for _ in 0..40 {
        let seed = rng.next_u64();
        diff_bufs(
            "ERRORS row13",
            || {
                let mut g = Rng::new(seed);
                let mut b = g.buffer_len(0);
                b.checksum = g.next_u32(); // deliberately not 0
                vec![b]
            },
            |api, p| unsafe { (api.buffer_reverse)(p) as i64 },
            true,
        );
    }
}

// ========================================================== rows 14–15 =====
// buffer_merge

#[test]
fn row14_buffer_merge_null_pointers() {
    let (c, r) = both();
    let good = Rng::new(14).buffer(0, 50);
    for mask in 1u8..8 {
        let (n1, n2, nd) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
        let co = observe(None, || unsafe {
            let a = good;
            let b = good;
            let mut d = BufferT::patterned(0x14);
            let p1 = if n1 { null() } else { &a as *const BufferT };
            let p2 = if n2 { null() } else { &b as *const BufferT };
            let pd = if nd { null_mut() } else { &mut d as *mut BufferT };
            (c.buffer_merge)(p1, p2, pd) as i64
        });
        let ro = observe(None, || unsafe {
            let a = good;
            let b = good;
            let mut d = BufferT::patterned(0x14);
            let p1 = if n1 { null() } else { &a as *const BufferT };
            let p2 = if n2 { null() } else { &b as *const BufferT };
            let pd = if nd { null_mut() } else { &mut d as *mut BufferT };
            (r.buffer_merge)(p1, p2, pd) as i64
        });
        same(&format!("ERRORS row14: mask={:03b}", mask), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(co.stderr, b"Error: NULL pointer in buffer_merge\n".to_vec());
    }
}

#[test]
fn row15_buffer_merge_combined_length_over_maximum() {
    let (c, r) = both();
    let cases: &[(usize, usize)] = &[
        (256, 1),
        (1, 256),
        (256, 256),
        (200, 200),
        (129, 128),
        (128, 129),
        (257, 0),
        (0, 257),
        (255, 2),
        (300, 300),
        (1000, 1),
    ];
    for &(l1, l2) in cases {
        diff_bufs(
            &format!("ERRORS row15: {}+{}", l1, l2),
            || {
                let mut a = BufferT::patterned(0xA1);
                a.length = l1;
                let mut b = BufferT::patterned(0xA2);
                b.length = l2;
                vec![a, b, BufferT::patterned(0xA3)]
            },
            |api, p| unsafe { (api.buffer_merge)(p, p.add(1), p.add(2)) as i64 },
            true,
        );
        // and check the exact message
        let mut a = BufferT::patterned(0xA1);
        a.length = l1;
        let mut b = BufferT::patterned(0xA2);
        b.length = l2;
        let mut d = BufferT::patterned(0xA3);
        let co = observe(None, || unsafe { (c.buffer_merge)(&a, &b, &mut d) as i64 });
        let mut d2 = BufferT::patterned(0xA3);
        let ro = observe(None, || unsafe { (r.buffer_merge)(&a, &b, &mut d2) as i64 });
        same("ERRORS row15/msg", &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Merged length {} exceeds maximum\n", l1 + l2).into_bytes()
        );
    }
    // 256 + 0 and 0 + 256 are exactly at the limit and must succeed
    for &(l1, l2) in &[(256usize, 0usize), (0, 256), (128, 128)] {
        let mut a = Rng::new(15).buffer_len(l1);
        a.checksum = checksum(&a.data[..l1]);
        let mut b = Rng::new(16).buffer_len(l2);
        b.checksum = checksum(&b.data[..l2]);
        let mut d = BufferT::patterned(0xA4);
        let co = observe(None, || unsafe { (c.buffer_merge)(&a, &b, &mut d) as i64 });
        assert_eq!(co.ret, 0, "{}+{} is the boundary, must succeed", l1, l2);
        assert!(co.stderr.is_empty());
    }
}

// ========================================================== rows 16–17 =====
// buffer_split

#[test]
fn row16_buffer_split_null_pointers() {
    let (c, r) = both();
    let good = Rng::new(17).buffer(0, 50);
    for mask in 1u8..8 {
        let (ns, n1, n2) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
        let co = observe(None, || unsafe {
            let s = good;
            let mut d1 = BufferT::patterned(0x16);
            let mut d2 = BufferT::patterned(0x17);
            let ps = if ns { null() } else { &s as *const BufferT };
            let p1 = if n1 { null_mut() } else { &mut d1 as *mut BufferT };
            let p2 = if n2 { null_mut() } else { &mut d2 as *mut BufferT };
            (c.buffer_split)(ps, 3, p1, p2) as i64
        });
        let ro = observe(None, || unsafe {
            let s = good;
            let mut d1 = BufferT::patterned(0x16);
            let mut d2 = BufferT::patterned(0x17);
            let ps = if ns { null() } else { &s as *const BufferT };
            let p1 = if n1 { null_mut() } else { &mut d1 as *mut BufferT };
            let p2 = if n2 { null_mut() } else { &mut d2 as *mut BufferT };
            (r.buffer_split)(ps, 3, p1, p2) as i64
        });
        same(&format!("ERRORS row16: mask={:03b}", mask), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(co.stderr, b"Error: NULL pointer in buffer_split\n".to_vec());
    }
}

#[test]
fn row17_buffer_split_position_past_length() {
    let (c, r) = both();
    let mut rng = Rng::new(0x17);
    let mut cases: Vec<(usize, usize)> = Vec::new();
    for len in [0usize, 1, 2, 10, 255, 256] {
        cases.push((len, len + 1));
        cases.push((len, len + 2));
        cases.push((len, 257));
        cases.push((len, usize::MAX)); // what `(size_t)-1` looks like
        cases.push((len, usize::MAX - 1));
        cases.push((len, 1 << 40));
    }
    for _ in 0..40 {
        let len = rng.below(257);
        cases.push((len, len + 1 + rng.below(1000)));
    }
    for &(len, pos) in &cases {
        diff_bufs(
            &format!("ERRORS row17: len={} pos={}", len, pos),
            || {
                let mut g = Rng::new(len as u64 ^ pos as u64);
                let s = g.buffer_len(len);
                vec![s, BufferT::patterned(0x18), BufferT::patterned(0x19)]
            },
            move |api, p| unsafe { (api.buffer_split)(p, pos, p.add(1), p.add(2)) as i64 },
            true,
        );
        let s = Rng::new(len as u64 ^ pos as u64).buffer_len(len);
        let mut d1 = BufferT::patterned(0x18);
        let mut d2 = BufferT::patterned(0x19);
        let co = observe(None, || unsafe {
            (c.buffer_split)(&s, pos, &mut d1, &mut d2) as i64
        });
        let mut e1 = BufferT::patterned(0x18);
        let mut e2 = BufferT::patterned(0x19);
        let ro = observe(None, || unsafe {
            (r.buffer_split)(&s, pos, &mut e1, &mut e2) as i64
        });
        same("ERRORS row17/msg", &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Split position {} exceeds length {}\n", pos, len).into_bytes()
        );
    }
}

// ========================================================== rows 18–19 =====
// buffer_interleave

#[test]
fn row18_buffer_interleave_null_pointers() {
    let (c, r) = both();
    let good = Rng::new(18).buffer(0, 50);
    for mask in 1u8..8 {
        let (n1, n2, nd) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
        let co = observe(None, || unsafe {
            let a = good;
            let b = good;
            let mut d = BufferT::patterned(0x1A);
            let p1 = if n1 { null() } else { &a as *const BufferT };
            let p2 = if n2 { null() } else { &b as *const BufferT };
            let pd = if nd { null_mut() } else { &mut d as *mut BufferT };
            (c.buffer_interleave)(p1, p2, pd) as i64
        });
        let ro = observe(None, || unsafe {
            let a = good;
            let b = good;
            let mut d = BufferT::patterned(0x1A);
            let p1 = if n1 { null() } else { &a as *const BufferT };
            let p2 = if n2 { null() } else { &b as *const BufferT };
            let pd = if nd { null_mut() } else { &mut d as *mut BufferT };
            (r.buffer_interleave)(p1, p2, pd) as i64
        });
        same(&format!("ERRORS row18: mask={:03b}", mask), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            b"Error: NULL pointer in buffer_interleave\n".to_vec()
        );
    }
}

#[test]
fn row19_buffer_interleave_combined_length_over_maximum() {
    let (c, r) = both();
    for &(l1, l2) in &[
        (256usize, 1usize),
        (1, 256),
        (256, 256),
        (200, 200),
        (129, 128),
        (257, 0),
        (0, 257),
        (1000, 1000),
    ] {
        diff_bufs(
            &format!("ERRORS row19: {}+{}", l1, l2),
            || {
                let mut a = BufferT::patterned(0xB1);
                a.length = l1;
                let mut b = BufferT::patterned(0xB2);
                b.length = l2;
                vec![a, b, BufferT::patterned(0xB3)]
            },
            |api, p| unsafe { (api.buffer_interleave)(p, p.add(1), p.add(2)) as i64 },
            true,
        );
        let mut a = BufferT::patterned(0xB1);
        a.length = l1;
        let mut b = BufferT::patterned(0xB2);
        b.length = l2;
        let mut d = BufferT::patterned(0xB3);
        let co = observe(None, || unsafe {
            (c.buffer_interleave)(&a, &b, &mut d) as i64
        });
        let mut d2 = BufferT::patterned(0xB3);
        let ro = observe(None, || unsafe {
            (r.buffer_interleave)(&a, &b, &mut d2) as i64
        });
        same("ERRORS row19/msg", &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            b"Error: Interleaved length exceeds maximum\n".to_vec(),
            "the interleave message carries no numbers"
        );
    }
}

// ========================================================== rows 20–23 =====
// buffer_rotate

#[test]
fn row20_buffer_rotate_null() {
    let (c, r) = both();
    for pos in [0i32, 1, -1, i32::MIN, i32::MAX] {
        let co = observe(None, || unsafe { (c.buffer_rotate)(null_mut(), pos) as i64 });
        let ro = observe(None, || unsafe { (r.buffer_rotate)(null_mut(), pos) as i64 });
        same(&format!("ERRORS row20: pos={}", pos), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(co.stderr, b"Error: NULL buffer in rotate\n".to_vec());
    }
}

#[test]
fn row21_22_23_buffer_rotate_early_returns_and_negative_normalisation() {
    let mut rng = Rng::new(0x21);
    // row 21: length == 0 for a wide range of `positions`
    for pos in [0i32, 1, -1, 255, -255, i32::MIN, i32::MAX] {
        let seed = rng.next_u64();
        diff_bufs(
            &format!("ERRORS row21: len=0 pos={}", pos),
            || {
                let mut g = Rng::new(seed);
                let mut b = g.buffer_len(0);
                b.checksum = g.next_u32();
                vec![b]
            },
            move |api, p| unsafe { (api.buffer_rotate)(p, pos) as i64 },
            true,
        );
    }
    // row 22: positions == 0 for a wide range of lengths
    for len in [0usize, 1, 2, 3, 128, 255, 256] {
        let seed = rng.next_u64();
        diff_bufs(
            &format!("ERRORS row22: len={} pos=0", len),
            || {
                let mut g = Rng::new(seed);
                let mut b = g.buffer_len(len);
                b.checksum = g.next_u32(); // must survive untouched
                vec![b]
            },
            |api, p| unsafe { (api.buffer_rotate)(p, 0) as i64 },
            true,
        );
    }
    // row 23: negative normalisation
    for _ in 0..120 {
        let len = 1 + rng.below(256);
        let pos = -(rng.range(1, 2_000_000_000) as i32);
        let seed = rng.next_u64();
        diff_bufs(
            "ERRORS row23",
            || vec![Rng::new(seed).buffer_len(len)],
            move |api, p| unsafe { (api.buffer_rotate)(p, pos) as i64 },
            true,
        );
    }
}

// ============================================================== row 24 =====
// buffer_conditional_copy

#[test]
fn row24_conditional_copy_null_pointers() {
    let (c, r) = both();
    let good = Rng::new(24).buffer(0, 50);
    for (sn, dn) in [(true, false), (false, true), (true, true)] {
        for cm in [0u8, 1, 2, 0xFF] {
            let co = observe(None, || unsafe {
                let s = good;
                let mut d = BufferT::patterned(0x24);
                let ps = if sn { null() } else { &s as *const BufferT };
                let pd = if dn { null_mut() } else { &mut d as *mut BufferT };
                (c.buffer_conditional_copy)(ps, pd, 7, cm) as i64
            });
            let ro = observe(None, || unsafe {
                let s = good;
                let mut d = BufferT::patterned(0x24);
                let ps = if sn { null() } else { &s as *const BufferT };
                let pd = if dn { null_mut() } else { &mut d as *mut BufferT };
                (r.buffer_conditional_copy)(ps, pd, 7, cm) as i64
            });
            same(
                &format!("ERRORS row24: s_null={} d_null={} cm={}", sn, dn, cm),
                &co,
                &ro,
            );
            assert_eq!(co.ret, -1);
            assert_eq!(
                co.stderr,
                b"Error: NULL pointer in conditional_copy\n".to_vec()
            );
        }
    }
}

// ========================================================== rows 25–26 =====
// buffer_copy_strided

#[test]
fn row25_copy_strided_null_pointers() {
    let (c, r) = both();
    let good = Rng::new(25).buffer(0, 50);
    for (sn, dn) in [(true, false), (false, true), (true, true)] {
        // The NULL check happens BEFORE the stride check, so an invalid stride
        // must still produce the NULL message.
        for stride in [1i32, 0, -1, i32::MIN] {
            let co = observe(None, || unsafe {
                let s = good;
                let mut d = BufferT::patterned(0x25);
                let ps = if sn { null() } else { &s as *const BufferT };
                let pd = if dn { null_mut() } else { &mut d as *mut BufferT };
                (c.buffer_copy_strided)(ps, pd, stride) as i64
            });
            let ro = observe(None, || unsafe {
                let s = good;
                let mut d = BufferT::patterned(0x25);
                let ps = if sn { null() } else { &s as *const BufferT };
                let pd = if dn { null_mut() } else { &mut d as *mut BufferT };
                (r.buffer_copy_strided)(ps, pd, stride) as i64
            });
            same(
                &format!("ERRORS row25: s_null={} d_null={} stride={}", sn, dn, stride),
                &co,
                &ro,
            );
            assert_eq!(co.ret, -1);
            assert_eq!(co.stderr, b"Error: NULL pointer in copy_strided\n".to_vec());
        }
    }
}

#[test]
fn row26_copy_strided_invalid_stride() {
    let (c, r) = both();
    let mut rng = Rng::new(26);
    let mut strides: Vec<i32> = vec![0, -1, -2, -100, i32::MIN, i32::MIN + 1];
    for _ in 0..20 {
        strides.push(-(rng.range(1, 2_000_000_000) as i32));
    }
    for &stride in &strides {
        for len in [0usize, 1, 10, 256] {
            diff_bufs(
                &format!("ERRORS row26: stride={} len={}", stride, len),
                || {
                    let mut g = Rng::new(len as u64);
                    let s = g.buffer_len(len);
                    vec![s, BufferT::patterned(0x26)]
                },
                move |api, p| unsafe { (api.buffer_copy_strided)(p, p.add(1), stride) as i64 },
                true,
            );
        }
        let s = Rng::new(1).buffer_len(4);
        let mut d = BufferT::patterned(0x26);
        let co = observe(None, || unsafe {
            (c.buffer_copy_strided)(&s, &mut d, stride) as i64
        });
        let mut d2 = BufferT::patterned(0x26);
        let ro = observe(None, || unsafe {
            (r.buffer_copy_strided)(&s, &mut d2, stride) as i64
        });
        same("ERRORS row26/msg", &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Invalid stride {}\n", stride).into_bytes()
        );
    }
    // stride == 1 is the smallest valid value (boundary)
    let s = Rng::new(2).buffer_len(4);
    let mut d = BufferT::patterned(0x27);
    let co = observe(None, || unsafe {
        (c.buffer_copy_strided)(&s, &mut d, 1) as i64
    });
    assert_eq!(co.ret, 0);
    assert!(co.stderr.is_empty());
}

// ========================================================== rows 27–33 =====
// process_buffer_array

/// Run `process_buffer_array` on an array built by the library itself.
fn proc_obs(
    api: &Api,
    arr_null: bool,
    cap: i32,
    count: i32,
    op: c_int,
    param: c_int,
    bufs: &[BufferT],
) -> Obs<i64> {
    observe(None, || unsafe {
        if arr_null {
            return (api.process_buffer_array)(null_mut(), op, param) as i64;
        }
        let a = (api.init_buffer_array)(cap);
        assert!(!a.is_null());
        let st = (*a).buffers;
        for (i, b) in bufs.iter().enumerate() {
            *st.add(i) = *b;
        }
        (*a).count = count;
        let rc = (api.process_buffer_array)(a, op, param) as i64;
        (api.free_buffer_array)(a);
        rc
    })
}

#[test]
fn row27_process_buffer_array_null() {
    let (c, r) = both();
    for op in [0i32, 1, 2, 3, 4, 5, 6, 7, -1, i32::MIN, i32::MAX] {
        let co = proc_obs(c, true, 0, 0, op, 0, &[]);
        let ro = proc_obs(r, true, 0, 0, op, 0, &[]);
        same(&format!("ERRORS row27: op={}", op), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(co.stderr, b"Error: Invalid buffer array\n".to_vec());
    }
}

#[test]
fn row28_process_buffer_array_count_zero() {
    let (c, r) = both();
    let bufs = [Rng::new(28).buffer(0, 50)];
    for op in [0i32, 1, 2, 3, 4, 5, 6, 7, -1, i32::MIN, i32::MAX] {
        let co = proc_obs(c, false, 2, 0, op, 3, &bufs);
        let ro = proc_obs(r, false, 2, 0, op, 3, &bufs);
        same(&format!("ERRORS row28: op={}", op), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            b"Error: Invalid buffer array\n".to_vec(),
            "count == 0 is rejected before the switch, whatever the op"
        );
    }
}

#[test]
fn row29_process_buffer_array_merge_needs_two() {
    let (c, r) = both();
    let bufs = [Rng::new(29).buffer(0, 50), Rng::new(30).buffer(0, 50)];
    for count in [1i32, -1, -2, i32::MIN] {
        let co = proc_obs(c, false, 2, count, OP_MERGE, 0, &bufs);
        let ro = proc_obs(r, false, 2, count, OP_MERGE, 0, &bufs);
        same(&format!("ERRORS row29: count={}", count), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            b"Error: Need at least 2 buffers for merge\n".to_vec()
        );
    }
}

#[test]
fn row30_process_buffer_array_unknown_operation() {
    // OP_SPLIT (3) and OP_INTERLEAVE (4) are *valid enumerators* that the switch
    // does not handle, so they take the `default:` branch — as does any integer
    // with no corresponding enumerator (a C enum accepts any int across FFI).
    let (c, r) = both();
    let bufs = [Rng::new(31).buffer(0, 50), Rng::new(32).buffer(0, 50)];
    let ops: [i32; 16] = [
        3,
        4,
        7,
        8,
        9,
        42,
        -1,
        -2,
        -7,
        100,
        1000,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        0x7FFF_FFFF,
    ];
    for op in ops {
        let co = proc_obs(c, false, 2, 2, op, 3, &bufs);
        let ro = proc_obs(r, false, 2, 2, op, 3, &bufs);
        same(&format!("ERRORS row30: op={}", op), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Unknown operation {}\n", op).into_bytes()
        );
    }
}

#[test]
fn row31_32_33_process_buffer_array_inner_failures() {
    let (c, r) = both();

    // row 31: OP_COPY where buffers[0].length > 256 makes the inner
    // buffer_copy -> validate_buffer fail.
    for len in [257usize, 512, usize::MAX] {
        let mut b0 = BufferT::patterned(0x31);
        b0.length = len;
        let bufs = [b0, Rng::new(33).buffer(0, 10), Rng::new(34).buffer(0, 10)];
        let co = proc_obs(c, false, 3, 3, OP_COPY, 0, &bufs);
        let ro = proc_obs(r, false, 3, 3, OP_COPY, 0, &bufs);
        same(&format!("ERRORS row31: len={}", len), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Buffer length {} exceeds maximum 256\n", len).into_bytes()
        );
    }

    // row 32: OP_MERGE where a pair's combined length exceeds the maximum.
    for &(l1, l2) in &[(200usize, 200usize), (256, 1), (129, 128)] {
        let mut a = BufferT::patterned(0x32);
        a.length = l1;
        let mut b = BufferT::patterned(0x33);
        b.length = l2;
        let bufs = [a, b];
        let co = proc_obs(c, false, 2, 2, OP_MERGE, 0, &bufs);
        let ro = proc_obs(r, false, 2, 2, OP_MERGE, 0, &bufs);
        same(&format!("ERRORS row32: {}+{}", l1, l2), &co, &ro);
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Merged length {} exceeds maximum\n", l1 + l2).into_bytes()
        );
    }

    // row 33: OP_CHECKSUM where one buffer's length exceeds the maximum
    // (validate_buffer returns false -> -1).  The buffer *before* it produces a
    // warning first, so the whole stderr stream is compared.
    for bad_index in 0..3usize {
        let mut bufs: Vec<BufferT> = (0..3)
            .map(|i| {
                let mut b = Rng::new(40 + i as u64).buffer(1, 20);
                b.checksum ^= 0xFF; // make every one warn
                b
            })
            .collect();
        bufs[bad_index].length = 257;
        let co = proc_obs(c, false, 3, 3, OP_CHECKSUM, 0, &bufs);
        let ro = proc_obs(r, false, 3, 3, OP_CHECKSUM, 0, &bufs);
        same(&format!("ERRORS row33: bad={}", bad_index), &co, &ro);
        assert_eq!(co.ret, -1);
    }
}

// ========================================================== rows 34–38 =====
// read_buffer

#[test]
fn row34_read_buffer_null_does_not_consume_stdin() {
    let (c, r) = both();
    let path = observe_stdin_path();
    let input = b"2 11 22 3 1 2 3\n";

    let co = observe(Some(input), || unsafe {
        c_freopen_stdin(&path);
        let rc1 = (c.read_buffer)(null_mut()) as i64;
        // The NULL check happens before any scanf, so the stream is intact.
        let mut b = BufferT::patterned(0x34);
        let rc2 = (c.read_buffer)(&mut b) as i64;
        (rc1, rc2, b.length, b.checksum, b.data[..b.length.min(256)].to_vec())
    });
    let ro = observe(Some(input), || unsafe {
        (r.reset_stdin.unwrap())();
        let rc1 = (r.read_buffer)(null_mut()) as i64;
        let mut b = BufferT::patterned(0x34);
        let rc2 = (r.read_buffer)(&mut b) as i64;
        (rc1, rc2, b.length, b.checksum, b.data[..b.length.min(256)].to_vec())
    });
    same("ERRORS row34", &co, &ro);
    assert_eq!(co.ret.0, -1);
    assert_eq!(co.ret.1, 0);
    assert_eq!(co.ret.2, 2, "the first token must still be available");
    assert_eq!(co.stderr, b"Error: NULL buffer in read_buffer\n".to_vec());
}

/// Read `calls` buffers from `input` in both libraries and compare everything.
#[track_caller]
fn diff_reads(what: &str, input: &[u8], calls: usize) {
    let (c, r) = both();
    let path = observe_stdin_path();
    let go = |api: &Api, reset: &dyn Fn()| {
        reset();
        let mut v = Vec::new();
        for _ in 0..calls {
            let mut b = BufferT::patterned(0x77);
            let rc = unsafe { (api.read_buffer)(&mut b) } as i64;
            v.push((rc, b.length, b.checksum, b.data.to_vec()));
        }
        v
    };
    let co = observe(Some(input), || go(c, &|| c_freopen_stdin(&path)));
    let ro = observe(Some(input), || {
        go(r, &|| unsafe { (r.reset_stdin.unwrap())() })
    });
    same(
        &format!("{} input={:?}", what, String::from_utf8_lossy(input)),
        &co,
        &ro,
    );
}

#[test]
fn row35_read_buffer_length_scan_failure() {
    // EOF and every kind of matching failure.
    for input in [
        &b""[..],
        b" ",
        b"\n",
        b"\t\n \r",
        b"x",
        b" x 5",
        b"abc",
        b"-",
        b"+",
        b"-x",
        b"+x",
        b".",
        b".5",
        b"e",
        b"0x",
        b"--3",
        b"++3",
        b"-+3",
        b"/3",
        b":",
        b"\x00",
        b"\xff",
    ] {
        diff_reads("ERRORS row35", input, 1);
    }
    // Verify the message once explicitly.
    let (c, _r) = both();
    let path = observe_stdin_path();
    let co = observe(Some(b"x"), || unsafe {
        c_freopen_stdin(&path);
        let mut b = BufferT::patterned(0x35);
        (c.read_buffer)(&mut b) as i64
    });
    assert_eq!(co.ret, -1);
    assert_eq!(co.stderr, b"Error: Failed to read buffer length\n".to_vec());
}

#[test]
fn row36_37_read_buffer_length_out_of_range() {
    let (c, _r) = both();
    let path = observe_stdin_path();
    let mut lengths: Vec<String> = Vec::new();
    for l in [
        -1i64,
        -2,
        -100,
        -256,
        -257,
        257,
        258,
        300,
        1000,
        65536,
        2147483647,
        -2147483648,
    ] {
        lengths.push(l.to_string());
    }
    // scanf-overflow forms, whose truncated `int` values are what gets checked
    for s in [
        "2147483648",
        "4294967296",
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775808",
    ] {
        lengths.push(s.to_string());
    }
    for l in &lengths {
        let input = format!("{} 1 2 3\n", l).into_bytes();
        diff_reads("ERRORS row36/37", &input, 1);
    }
    // exact message for the two canonical cases
    for (tok, shown) in [("-1", "-1"), ("257", "257")] {
        let input = format!("{} 1 2\n", tok).into_bytes();
        let co = observe(Some(&input), || unsafe {
            c_freopen_stdin(&path);
            let mut b = BufferT::patterned(0x36);
            (c.read_buffer)(&mut b) as i64
        });
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Invalid buffer length {}\n", shown).into_bytes()
        );
    }
    // 0 and 256 are the accepted boundaries
    for tok in ["0", "256"] {
        let n: usize = tok.parse().unwrap();
        let mut input = tok.to_string();
        for i in 0..n {
            input.push_str(&format!(" {}", i % 256));
        }
        input.push('\n');
        let co = observe(Some(input.as_bytes()), || unsafe {
            c_freopen_stdin(&path);
            let mut b = BufferT::patterned(0x37);
            ((c.read_buffer)(&mut b) as i64, b.length)
        });
        assert_eq!(co.ret, (0, n), "length {} is a valid boundary", n);
        assert!(co.stderr.is_empty());
    }
}

#[test]
fn row38_read_buffer_byte_scan_failure() {
    // Truncated streams: the error names the index of the byte that failed.
    for len in [1usize, 2, 3, 5, 10, 256] {
        for provided in 0..len.min(4) {
            let mut input = len.to_string();
            for i in 0..provided {
                input.push_str(&format!(" {}", i));
            }
            input.push('\n');
            diff_reads("ERRORS row38/short", input.as_bytes(), 1);
        }
        // junk token instead of a number
        for junk in ["x", "-", "+", ".", "abc", "0x1"] {
            let input = format!("{} 1 {} 3 4 5\n", len, junk).into_bytes();
            diff_reads("ERRORS row38/junk", &input, 1);
        }
    }
    // exact message
    let (c, _r) = both();
    let path = observe_stdin_path();
    for (input, idx) in [
        (&b"3 1 2"[..], 2usize),
        (&b"3"[..], 0),
        (&b"3 1"[..], 1),
        // "1 2 3" fill indices 0..2, so the failing element is index 3
        (&b"5 1 2 3 x"[..], 3),
    ] {
        let co = observe(Some(input), || unsafe {
            c_freopen_stdin(&path);
            let mut b = BufferT::patterned(0x38);
            (c.read_buffer)(&mut b) as i64
        });
        assert_eq!(co.ret, -1);
        assert_eq!(
            co.stderr,
            format!("Error: Failed to read byte {}\n", idx).into_bytes(),
            "input {:?}",
            String::from_utf8_lossy(input)
        );
    }
}

// ============================================================== row 39 =====
// write_buffer

#[test]
fn row39_write_buffer_null() {
    let (c, r) = both();
    let co = observe(None, || unsafe { (c.write_buffer)(null()) });
    let ro = observe(None, || unsafe { (r.write_buffer)(null()) });
    same("ERRORS row39", &co, &ro);
    assert!(co.stdout.is_empty(), "nothing may be written to stdout");
    assert_eq!(co.stderr, b"Error: NULL buffer in write_buffer\n".to_vec());
    // twice, and interleaved with a valid call
    let good = Rng::new(39).buffer(1, 10);
    let co = observe(None, || unsafe {
        (c.write_buffer)(null());
        (c.write_buffer)(&good);
        (c.write_buffer)(null());
    });
    let ro = observe(None, || unsafe {
        (r.write_buffer)(null());
        (r.write_buffer)(&good);
        (r.write_buffer)(null());
    });
    same("ERRORS row39/interleaved", &co, &ro);
}

// ================================================= generic FFI boundaries ===

#[test]
fn g2_calculate_checksum_null_with_zero_length() {
    let (c, r) = both();
    let co = observe(None, || unsafe { (c.calculate_checksum)(null(), 0) });
    let ro = observe(None, || unsafe { (r.calculate_checksum)(null(), 0) });
    same("G2: calculate_checksum(NULL, 0)", &co, &ro);
    assert_eq!(co.ret, 0);
    assert!(co.stderr.is_empty());
}

#[test]
fn g7_out_of_range_c_bool_across_ffi() {
    // A C `_Bool` parameter can receive any byte pattern across the ABI.
    let mut rng = Rng::new(0xB7);
    for cm in [0u8, 1, 2, 3, 0x7F, 0x80, 0xFE, 0xFF] {
        for _ in 0..6 {
            let seed = rng.next_u64();
            let len = 1 + rng.below(40);
            let pat = rng.u8();
            diff_bufs(
                &format!("G7: copy_matching={}", cm),
                || {
                    let mut g = Rng::new(seed);
                    let s = g.buffer_len(len);
                    vec![s, BufferT::patterned(0x7B)]
                },
                move |api, p| unsafe {
                    (api.buffer_conditional_copy)(p, p.add(1), pat, cm) as i64
                },
                true,
            );
        }
    }
}

#[test]
fn g8_int_min_for_every_int_parameter() {
    let (c, r) = both();
    // capacity
    let co = observe(None, || unsafe { (c.init_buffer_array)(i32::MIN).is_null() });
    let ro = observe(None, || unsafe { (r.init_buffer_array)(i32::MIN).is_null() });
    same("G8: init_buffer_array(INT_MIN)", &co, &ro);
    // stride
    diff_bufs(
        "G8: buffer_copy_strided(INT_MIN)",
        || vec![Rng::new(1).buffer_len(10), BufferT::patterned(0x8A)],
        |api, p| unsafe { (api.buffer_copy_strided)(p, p.add(1), i32::MIN) as i64 },
        true,
    );
    // positions
    for len in [1usize, 2, 3, 255, 256] {
        diff_bufs(
            "G8: buffer_rotate(INT_MIN)",
            || vec![Rng::new(len as u64).buffer_len(len)],
            |api, p| unsafe { (api.buffer_rotate)(p, i32::MIN) as i64 },
            true,
        );
    }
    // op and param of process_buffer_array
    let bufs = [Rng::new(2).buffer(1, 20), Rng::new(3).buffer(1, 20)];
    for op in [OP_ROTATE, i32::MIN] {
        let co = proc_obs(c, false, 2, 2, op, i32::MIN, &bufs);
        let ro = proc_obs(r, false, 2, 2, op, i32::MIN, &bufs);
        same(&format!("G8: process(op={}, param=INT_MIN)", op), &co, &ro);
    }
}

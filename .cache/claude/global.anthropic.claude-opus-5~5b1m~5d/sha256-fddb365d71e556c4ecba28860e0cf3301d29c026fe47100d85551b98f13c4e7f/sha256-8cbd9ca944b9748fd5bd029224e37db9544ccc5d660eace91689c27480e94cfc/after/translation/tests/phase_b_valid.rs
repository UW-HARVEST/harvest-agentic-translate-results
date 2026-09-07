//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through the exported C-ABI symbols of BOTH shared objects
//! (loaded with `libloading`); return values *and* captured stdout are compared
//! byte-for-byte.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int};

/// Runs `create_state` on both libs, snapshots the resulting state, destroys it
/// with the *same* library that created it, and compares everything.
fn diff_create(initial_val: c_int, capacity: c_int, read_buffer: bool) {
    let l = libs();
    let _g = lock();

    let ((c_snap, c_null), c_out) = capture_stdout(|| unsafe {
        let f = l.c.create_state();
        let p = f(initial_val, capacity);
        let s = snapshot(p, read_buffer);
        let d = l.c.destroy_state();
        d(p);
        (s, p.is_null())
    });
    let ((r_snap, r_null), r_out) = capture_stdout(|| unsafe {
        let f = l.rust.create_state();
        let p = f(initial_val, capacity);
        let s = snapshot(p, read_buffer);
        let d = l.rust.destroy_state();
        d(p);
        (s, p.is_null())
    });

    assert_eq!(
        c_null, r_null,
        "create_state({initial_val}, {capacity}) nullness differs"
    );
    assert_eq!(
        c_snap, r_snap,
        "create_state({initial_val}, {capacity}) state differs"
    );
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "create_state({initial_val}, {capacity}) stdout differs"
    );
}

// --- Row 1 -----------------------------------------------------------------
#[test]
fn row01_create_state_cap128_random_initial() {
    let mut rng = Rng::new(0xC0FFEE01);
    for _ in 0..1000 {
        diff_create(rng.next_i32(), 128, true);
    }
}

// --- Row 2 -----------------------------------------------------------------
#[test]
fn row02_create_state_cap128_interesting_initial() {
    for &v in INTERESTING_I32 {
        diff_create(v, 128, true);
    }
}

// --- Row 3 -----------------------------------------------------------------
#[test]
fn row03_create_state_truncating_capacity() {
    for cap in 1..=17 {
        for &v in &[0, 12345, -12345, i32::MAX, i32::MIN, 1078530011] {
            diff_create(v, cap, true);
        }
    }
}

// --- Row 4 -----------------------------------------------------------------
#[test]
fn row04_create_state_capacity_zero() {
    // buffer contents are uninitialised for capacity 0, so compare fields only
    for &v in &[0, 1, -1, i32::MAX, i32::MIN] {
        diff_create(v, 0, false);
    }
}

// --- Row 5 -----------------------------------------------------------------
#[test]
fn row05_create_state_large_capacity() {
    let mut rng = Rng::new(0xC0FFEE05);
    for &cap in &[18, 32, 64, 256, 1024, 65536] {
        for _ in 0..40 {
            diff_create(rng.next_i32(), cap, true);
        }
    }
}

// ---------------------------------------------------------------------------
// update_flags
// ---------------------------------------------------------------------------

/// Creates a state with each library, applies the given `params` sequence via
/// `update_flags`, and compares flags word + stdout.
fn diff_update_flags(initial_val: c_int, params: &[c_int]) {
    let l = libs();
    let _g = lock();

    let run = |imp: &Impl| unsafe {
        let create = imp.create_state();
        let update = imp.update_flags();
        let destroy = imp.destroy_state();
        let p = create(initial_val, 128);
        assert!(!p.is_null());
        for &param in params {
            update(p, param);
        }
        let s = snapshot(p, true);
        destroy(p);
        s
    };

    let (c_snap, c_out) = capture_stdout(|| run(&l.c));
    let (r_snap, r_out) = capture_stdout(|| run(&l.rust));

    assert_eq!(
        c_snap, r_snap,
        "update_flags(initial={initial_val}, params={params:?}) state differs\n\
         C flags=0x{:08x} Rust flags=0x{:08x}",
        c_snap.flags, r_snap.flags
    );
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "update_flags(initial={initial_val}, params={params:?}) stdout differs"
    );
}

// --- Row 6 -----------------------------------------------------------------
#[test]
fn row06_update_flags_full_low_6_bits() {
    for param in 0..64 {
        diff_update_flags(42, &[param]);
    }
}

// --- Row 7 -----------------------------------------------------------------
#[test]
fn row07_update_flags_random_i32() {
    let mut rng = Rng::new(0xC0FFEE07);
    for _ in 0..1000 {
        let init = rng.next_i32();
        diff_update_flags(init, &[rng.next_i32()]);
    }
}

// --- Row 8 -----------------------------------------------------------------
#[test]
fn row08_update_flags_counter_wrap() {
    for &n in &[1usize, 2, 30, 31, 32, 33, 64, 65] {
        let mut rng = Rng::new(0xC0FFEE08 + n as u64);
        let params: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        diff_update_flags(7, &params);
    }
}

// --- Row 9 -----------------------------------------------------------------
#[test]
fn row09_update_flags_boundary_params() {
    for &param in INTERESTING_I32 {
        diff_update_flags(0, &[param]);
        diff_update_flags(0, &[param, param]);
    }
}

// ---------------------------------------------------------------------------
// process_buffer
// ---------------------------------------------------------------------------

// --- Row 10 ----------------------------------------------------------------
#[test]
fn row10_process_buffer_created_state_all_bytes() {
    let l = libs();
    for &init in &[0, 1, -1, 1234567890, -987654321, i32::MAX, i32::MIN] {
        for b in 0u16..=255 {
            let target = b as u8 as c_char;
            let _g = lock();
            let run = |imp: &Impl| unsafe {
                let create = imp.create_state();
                let pb = imp.process_buffer();
                let destroy = imp.destroy_state();
                let p = create(init, 128);
                assert!(!p.is_null());
                let n = pb(p, target);
                destroy(p);
                n
            };
            let (c_n, c_out) = capture_stdout(|| run(&l.c));
            let (r_n, r_out) = capture_stdout(|| run(&l.rust));
            assert_eq!(c_n, r_n, "process_buffer(init={init}, target={b}) return");
            assert_eq!(
                show(&c_out),
                show(&r_out),
                "process_buffer(init={init}, target={b}) stdout"
            );
        }
    }
}

/// `process_buffer` on a synthetic state whose buffer holds exactly `content`.
fn diff_process_buffer_synthetic(flags: u32, data: u32, capacity: c_int, content: &[u8], target: u8) {
    let l = libs();
    let _g = lock();

    let c_state = SyntheticState::new(flags, data, capacity, content);
    let r_state = SyntheticState::new(flags, data, capacity, content);

    let (c_n, c_out) = capture_stdout(|| unsafe {
        let f = l.c.process_buffer();
        f(c_state.ptr, target as c_char)
    });
    let (r_n, r_out) = capture_stdout(|| unsafe {
        let f = l.rust.process_buffer();
        f(r_state.ptr, target as c_char)
    });

    assert_eq!(
        c_n,
        r_n,
        "process_buffer(buffer={:?}, target={target}) return differs",
        String::from_utf8_lossy(content)
    );
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "process_buffer(buffer={:?}, target={target}) stdout differs",
        String::from_utf8_lossy(content)
    );
    // the function must not mutate the state
    let cs = unsafe { snapshot(c_state.ptr, true) };
    let rs = unsafe { snapshot(r_state.ptr, true) };
    assert_eq!(cs.flags, rs.flags);
    assert_eq!(cs.data, rs.data);
    assert_eq!(cs.buffer, rs.buffer);
}

// --- Row 11 ----------------------------------------------------------------
#[test]
fn row11_process_buffer_random_ascii() {
    let mut rng = Rng::new(0xC0FFEE11);
    for _ in 0..2000 {
        let len = rng.below(65) as usize;
        // printable ASCII, weighted towards digits so matches are common
        let content: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(2) == 0 {
                    b'0' + rng.byte() % 10
                } else {
                    0x20 + rng.byte() % 0x5F
                }
            })
            .collect();
        let target = if rng.below(3) == 0 {
            rng.byte()
        } else {
            b'0' + rng.byte() % 10
        };
        diff_process_buffer_synthetic(rng.next_u32(), rng.next_u32(), len as c_int, &content, target);
    }
}

// --- Row 12 ----------------------------------------------------------------
#[test]
fn row12_process_buffer_random_high_bytes() {
    let mut rng = Rng::new(0xC0FFEE12);
    for _ in 0..2000 {
        let len = rng.below(48) as usize;
        // any non-NUL byte, including 0x80..0xFF (negative `char`)
        let content: Vec<u8> = (0..len).map(|_| 1 + rng.byte() % 255).collect();
        let target = rng.byte(); // full 0..=255, incl. 0 and high bytes
        diff_process_buffer_synthetic(0, 0, len as c_int, &content, target);
    }
}

// --- Row 13 ----------------------------------------------------------------
#[test]
fn row13_process_buffer_buffer_shapes() {
    let shapes: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"5".to_vec(),
        vec![b'x'; 2],
        vec![b'x'; 31],
        vec![b'x'; 64],
        vec![0xFF; 5],
        vec![0x80; 5],
        b"5aaaa".to_vec(),  // match only at first byte
        b"aaaa5".to_vec(),  // match only at last byte
        b"5aaa5".to_vec(),  // both ends
        b"55555".to_vec(),  // every byte
        vec![b'z'; 300],    // long
        {
            let mut v = vec![b'q'; 300];
            v[299] = b'7';
            v
        },
        {
            let mut v = vec![b'q'; 300];
            v[0] = b'7';
            v
        },
    ];
    for content in &shapes {
        for &target in &[0u8, b'a', b'x', b'5', b'7', b'z', b'q', 0x80, 0xFF, 1] {
            diff_process_buffer_synthetic(0x7B05, 0, content.len() as c_int, content, target);
        }
    }
}

// ---------------------------------------------------------------------------
// confuse_types
// ---------------------------------------------------------------------------

fn diff_confuse_types(initial_val: c_int, ops: &[c_int]) {
    let l = libs();
    let _g = lock();

    let run = |imp: &Impl| unsafe {
        let create = imp.create_state();
        let ct = imp.confuse_types();
        let destroy = imp.destroy_state();
        let p = create(initial_val, 128);
        assert!(!p.is_null());
        let results: Vec<c_int> = ops.iter().map(|&op| ct(p, op)).collect();
        let s = snapshot(p, true);
        destroy(p);
        (results, s)
    };

    let ((c_res, c_snap), c_out) = capture_stdout(|| run(&l.c));
    let ((r_res, r_snap), r_out) = capture_stdout(|| run(&l.rust));

    assert_eq!(
        c_res, r_res,
        "confuse_types(init=0x{initial_val:08x}, ops={ops:?}) results differ"
    );
    assert_eq!(
        c_snap, r_snap,
        "confuse_types(init=0x{initial_val:08x}, ops={ops:?}) state differs"
    );
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "confuse_types(init=0x{initial_val:08x}, ops={ops:?}) stdout differs"
    );
}

// --- Row 14 ----------------------------------------------------------------
#[test]
fn row14_confuse_types_op0() {
    let mut rng = Rng::new(0xC0FFEE14);
    for &v in INTERESTING_I32 {
        diff_confuse_types(v, &[0]);
    }
    for _ in 0..500 {
        diff_confuse_types(rng.next_i32(), &[0]);
    }
}

// --- Row 15 ----------------------------------------------------------------
#[test]
fn row15_confuse_types_op1_random() {
    let mut rng = Rng::new(0xC0FFEE15);
    for _ in 0..3000 {
        diff_confuse_types(rng.next_i32(), &[1]);
    }
}

// --- Row 16 ----------------------------------------------------------------
#[test]
fn row16_confuse_types_op1_float_bit_patterns() {
    let mut pats: Vec<i32> = INTERESTING_I32.to_vec();
    // exponent sweep: every exponent, a few mantissas, both signs
    for exp in 0u32..=255 {
        for &mant in &[0u32, 1, 0x400000, 0x7FFFFF] {
            for sign in [0u32, 1] {
                pats.push(((sign << 31) | (exp << 23) | mant) as i32);
            }
        }
    }
    for p in pats {
        diff_confuse_types(p, &[1]);
    }
}

// --- Row 17 ----------------------------------------------------------------
#[test]
fn row17_confuse_types_op2_random() {
    let mut rng = Rng::new(0xC0FFEE17);
    for &v in INTERESTING_I32 {
        diff_confuse_types(v, &[2]);
    }
    for _ in 0..1000 {
        diff_confuse_types(rng.next_i32(), &[2]);
    }
}

// --- Row 18 ----------------------------------------------------------------
#[test]
fn row18_confuse_types_op3_random() {
    let mut rng = Rng::new(0xC0FFEE18);
    for &v in INTERESTING_I32 {
        diff_confuse_types(v, &[3]);
    }
    for _ in 0..1000 {
        diff_confuse_types(rng.next_i32(), &[3]);
    }
    // both low bytes negative -> negative sum
    for hi in [0x00u32, 0xFF] {
        for a in [0x80u32, 0xFF, 0x81, 0xC0] {
            for b in [0x80u32, 0xFF, 0x81, 0xC0] {
                diff_confuse_types(((hi << 24) | (hi << 16) | (b << 8) | a) as i32, &[3]);
            }
        }
    }
}

// --- Row 19 ----------------------------------------------------------------
#[test]
fn row19_confuse_types_ordered_pairs() {
    let mut rng = Rng::new(0xC0FFEE19);
    for a in 0..4 {
        for b in 0..4 {
            for &v in &[0, 1, -1, 1078530011, i32::MAX, i32::MIN, 0x4EFFFFFF] {
                diff_confuse_types(v, &[a, b]);
            }
            for _ in 0..20 {
                diff_confuse_types(rng.next_i32(), &[a, b]);
            }
        }
    }
    // longer sequences
    for _ in 0..200 {
        let n = 1 + rng.below(6) as usize;
        let ops: Vec<c_int> = (0..n).map(|_| rng.below(4) as c_int).collect();
        diff_confuse_types(rng.next_i32(), &ops);
    }
}

// --- Row 20 ----------------------------------------------------------------
#[test]
fn row20_cross_library_destroy() {
    let l = libs();
    let mut rng = Rng::new(0xC0FFEE20);
    for _ in 0..200 {
        let init = rng.next_i32();
        let _g = lock();
        // C creates, Rust destroys
        let ((c_snap, r_snap), out) = capture_stdout(|| unsafe {
            let cp = l.c.create_state()(init, 128);
            let rp = l.rust.create_state()(init, 128);
            let cs = snapshot(cp, true);
            let rs = snapshot(rp, true);
            l.rust.destroy_state()(cp); // Rust frees a C-allocated state
            l.c.destroy_state()(rp); // C frees a Rust-allocated state
            (cs, rs)
        });
        assert_eq!(c_snap, r_snap, "cross-library state mismatch (init={init})");
        // Two create_state calls emit no output at all on success.
        assert_eq!(out.len(), 0, "unexpected output: {}", show(&out));
    }
}

// --- Row 21 ----------------------------------------------------------------
#[test]
fn row21_full_pipeline_low_level() {
    let l = libs();
    let mut rng = Rng::new(0xC0FFEE21);
    for _ in 0..2000 {
        let init = rng.next_i32();
        let n_updates = rng.below(5) as usize;
        let params: Vec<c_int> = (0..n_updates).map(|_| rng.next_i32()).collect();
        let target = if rng.below(2) == 0 {
            b'0' + rng.byte() % 10
        } else {
            rng.byte()
        };
        let op = (rng.next_i32() % 7) - 3; // includes out-of-range selectors
        let cap = *[128, 64, 17, 16, 8, 1024].get(rng.below(6) as usize).unwrap();

        let _g = lock();
        let run = |imp: &Impl| unsafe {
            let p = imp.create_state()(init, cap);
            if p.is_null() {
                return (true, 0, 0, StateSnapshot {
                    is_null: true,
                    flags: 0,
                    data: 0,
                    capacity: 0,
                    buffer_is_null: true,
                    buffer: vec![],
                });
            }
            let uf = imp.update_flags();
            for &param in &params {
                uf(p, param);
            }
            let found = imp.process_buffer()(p, target as c_char);
            let ct = imp.confuse_types()(p, op);
            let s = snapshot(p, true);
            imp.destroy_state()(p);
            (false, found, ct, s)
        };
        let (c_r, c_out) = capture_stdout(|| run(&l.c));
        let (r_r, r_out) = capture_stdout(|| run(&l.rust));
        assert_eq!(
            c_r, r_r,
            "pipeline(init={init}, params={params:?}, target={target}, op={op}, cap={cap}) differs"
        );
        assert_eq!(
            show(&c_out),
            show(&r_out),
            "pipeline(init={init}, params={params:?}, target={target}, op={op}, cap={cap}) stdout differs"
        );
    }
}

// ---------------------------------------------------------------------------
// confusion (the public one-shot wrapper)
// ---------------------------------------------------------------------------

fn diff_confusion(a: c_int, b: c_int, c: c_int, d: c_int) {
    let l = libs();
    let _g = lock();
    let (c_ret, c_out) = capture_stdout(|| unsafe { l.c.confusion()(a, b, c, d) });
    let (r_ret, r_out) = capture_stdout(|| unsafe { l.rust.confusion()(a, b, c, d) });
    assert_eq!(c_ret, r_ret, "confusion({a},{b},{c},{d}) return differs");
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "confusion({a},{b},{c},{d}) stdout differs"
    );
}

// --- Row 22 ----------------------------------------------------------------
#[test]
fn row22_confusion_selector_cross() {
    let mut rng = Rng::new(0xC0FFEE22);
    for p3 in 0..10 {
        for p4 in 0..4 {
            for _ in 0..10 {
                diff_confusion(rng.next_i32(), rng.next_i32(), p3, p4);
            }
        }
    }
}

// --- Row 23 ----------------------------------------------------------------
#[test]
fn row23_confusion_negative_selectors() {
    let mut rng = Rng::new(0xC0FFEE23);
    for p3 in -10..=0 {
        for p4 in -4..=0 {
            for _ in 0..8 {
                diff_confusion(rng.next_i32(), rng.next_i32(), p3, p4);
            }
        }
    }
    diff_confusion(0, 0, i32::MIN, i32::MIN);
    diff_confusion(0, 0, i32::MAX, i32::MAX);
}

// --- Row 24 ----------------------------------------------------------------
#[test]
fn row24_confusion_random_all_params() {
    let mut rng = Rng::new(0xC0FFEE24);
    for _ in 0..5000 {
        diff_confusion(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

// --- Row 25 ----------------------------------------------------------------
#[test]
fn row25_confusion_extremes() {
    let vals = [0, 1, -1, i32::MAX, i32::MIN, 1078530011, 0x4EFFFFFF, 0x7FC00000u32 as i32];
    for &a in &vals {
        for &b in &vals {
            diff_confusion(a, b, 3, 1);
            diff_confusion(3, 1, a, b);
            diff_confusion(a, 1, b, 2);
            diff_confusion(1, a, 2, b);
        }
    }
}

// --- Row A (layout parity) -------------------------------------------------
#[test]
fn phase_a_struct_layout_parity() {
    assert_eq!(std::mem::size_of::<ProcessState>(), 24);
    assert_eq!(std::mem::align_of::<ProcessState>(), 8);
    let l = libs();
    let _g = lock();
    // A freshly created state must have the fully-determined bit-field word
    // flag1=1, flag2=0, flag3=1, counter=0, mode=3, status=15, reserved=0.
    for imp in [&l.c, &l.rust] {
        let (snap, _) = capture_stdout(|| unsafe {
            let p = imp.create_state()(-42, 128);
            let s = snapshot(p, true);
            imp.destroy_state()(p);
            s
        });
        assert_eq!(field(snap.flags, FLAG1_SHIFT, 1), 1, "{}", imp.name);
        assert_eq!(field(snap.flags, FLAG2_SHIFT, 1), 0, "{}", imp.name);
        assert_eq!(field(snap.flags, FLAG3_SHIFT, 1), 1, "{}", imp.name);
        assert_eq!(field(snap.flags, COUNTER_SHIFT, 5), 0, "{}", imp.name);
        assert_eq!(field(snap.flags, MODE_SHIFT, 3), 3, "{}", imp.name);
        assert_eq!(field(snap.flags, STATUS_SHIFT, 5), 15, "{}", imp.name);
        assert_eq!(field(snap.flags, RESERVED_SHIFT, 16), 0, "{}", imp.name);
        assert_eq!(snap.data, (-42i32) as u32, "{}", imp.name);
        assert_eq!(snap.capacity, 128, "{}", imp.name);
        assert_eq!(snap.buffer, b"State:-42:Mode:3", "{}", imp.name);
    }
}

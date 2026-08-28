//! Differential tests: every function is invoked through the exported symbols
//! of both shared objects (never called directly), and the return value, the
//! resulting `ProcessState` contents and the exact stdout bytes are compared.
//!
//! Uses a custom (`harness = false`) runner so that the whole suite is strictly
//! sequential: capturing stdout means redirecting fd 1, which is process-wide.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Input corpora
// ---------------------------------------------------------------------------

fn int_corpus() -> Vec<c_int> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        4,
        5,
        7,
        8,
        9,
        10,
        -10,
        15,
        16,
        31,
        32,
        42,
        -42,
        99,
        100,
        127,
        128,
        255,
        256,
        1000,
        -1000,
        65535,
        65536,
        1078530011,
        -1078530011,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    // Deterministic pseudo-random fill.
    let mut s: u64 = 0x1234_5678_9abc_def0;
    for _ in 0..48 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        v.push((s >> 32) as u32 as i32);
    }
    v
}

/// Bit patterns for the `TypeConfusion` union: interesting `int`, `float` and
/// byte-level values.
fn u32_corpus() -> Vec<u32> {
    let mut v = vec![
        0x0000_0000,
        0x0000_0001,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0x4042_8F5C, // ~3.04
        0x4048_F5C3, // ~3.14
        0x4042_8F5B, // 1078530011 (the value case 0 writes)
        0x3F80_0000, // 1.0f
        0xBF80_0000, // -1.0f
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        0x7FC0_0000, // quiet NaN
        0xFFC0_0000, // -quiet NaN
        0x7F80_0001, // signalling NaN
        0xFF80_0001, // -signalling NaN
        0x0000_0001, // smallest denormal
        0x8000_0001, // -smallest denormal
        0x007F_FFFF, // largest denormal
        0x7F7F_FFFF, // FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
        0x4B00_0000, // 8388608.0f
        0x4F00_0000, // 2147483648.0f
        0xCF00_0000, // -2147483648.0f
        0x4EFF_FFFF, // 2147483520.0f (largest float < INT_MAX)
        0xCEFF_FFFF,
        0x4C00_0000, // 3.3554432e7
        0x3C23_D70A, // 0.01f  -> *100 == 1.0f
        0x3D4C_CCCD, // 0.05f
        0x4179_9999, // 15.6f
        0x8081_8283,
        0x7F7E_7D7C,
        0x0080_00FF,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_FF00,
        0x00FF_0000,
        0xFF00_0000,
    ];
    let mut s: u64 = 0xdead_beef_cafe_1234;
    for _ in 0..40 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        v.push((s >> 32) as u32);
    }
    v
}

fn flags_corpus() -> Vec<u32> {
    let mut v = vec![
        0x0000_0000,
        0xFFFF_FFFF,
        0x0000_7FFD, // the value create_state leaves behind
        0x0000_00F8, // counter = 31
        0x0000_0008, // counter = 1
        0x0000_0700, // mode = 7
        0x0000_F800, // status = 31
        0xFFFF_0000, // reserved = 0xFFFF
        0x0000_0007,
        0x1234_5678,
        0x8765_4321,
    ];
    let mut s: u64 = 0x0f0f_0f0f_a5a5_a5a5;
    for _ in 0..24 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        v.push((s >> 32) as u32);
    }
    v
}

fn buffer_corpus() -> Vec<Vec<u8>> {
    vec![
        b"".to_vec(),
        b"0".to_vec(),
        b"a".to_vec(),
        b"00000".to_vec(),
        b"State:42:Mode:3".to_vec(),
        b"State:0:Mode:3".to_vec(),
        b"State:-2147483648:Mode:3".to_vec(),
        b"0123456789".to_vec(),
        b"9876543210".to_vec(),
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec(),
        b"abc0abc0abc0".to_vec(),
        b"zzz0".to_vec(),
        b"0zzz".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0x80, 0x81, 0xFF, 0xFE, 0x41, 0x80],
        vec![0xFF; 16],
        vec![0x01, 0x02, 0x03, 0x7F, 0x80, 0xC0, 0x30, 0x30],
        (1u8..=64u8).collect::<Vec<u8>>(),
    ]
}

// ---------------------------------------------------------------------------
// 1. create_state / destroy_state
// ---------------------------------------------------------------------------

fn create_state_matches() {
    let p = pair();
    let capacities: [c_int; 10] = [1, 2, 3, 5, 8, 16, 17, 128, 4096, 65536];

    for &cap in &capacities {
        for &init in &int_corpus() {
            let (c_snap, c_out) = capture(|| unsafe {
                let s = (p.c.create_state)(init, cap);
                let snap = snapshot(s);
                (p.c.destroy_state)(s);
                snap
            });
            let (r_snap, r_out) = capture(|| unsafe {
                let s = (p.rs.create_state)(init, cap);
                let snap = snapshot(s);
                (p.rs.destroy_state)(s);
                snap
            });

            let ctx = format!("create_state({init}, {cap})");
            assert_eq!(c_snap, r_snap, "state mismatch for {ctx}");
            assert_stdout_eq(&ctx, &c_out, &r_out);
            // Sanity: the allocation should have succeeded here.
            assert!(c_snap.is_some(), "unexpected NULL from C for {ctx}");
        }
    }
}

/// `capacity == 0`: `malloc(0)` succeeds, `snprintf` writes nothing.  The
/// buffer bytes are then out of bounds, so only the pointer-ness and stdout
/// are compared.
fn create_state_zero_capacity() {
    let p = pair();
    for &init in &[0, 1, -1, 42, i32::MAX, i32::MIN] {
        let (c_res, c_out) = capture(|| unsafe {
            let s = (p.c.create_state)(init, 0);
            let r = if s.is_null() {
                None
            } else {
                Some(((*s).flags, (*s).data, (*s).capacity, (*s).buffer.is_null()))
            };
            (p.c.destroy_state)(s);
            r
        });
        let (r_res, r_out) = capture(|| unsafe {
            let s = (p.rs.create_state)(init, 0);
            let r = if s.is_null() {
                None
            } else {
                Some(((*s).flags, (*s).data, (*s).capacity, (*s).buffer.is_null()))
            };
            (p.rs.destroy_state)(s);
            r
        });
        let ctx = format!("create_state({init}, 0)");
        assert_eq!(c_res, r_res, "{ctx}");
        assert_stdout_eq(&ctx, &c_out, &r_out);
    }
}

/// A negative capacity makes `malloc((size_t)negative)` fail, exercising the
/// "Failed to allocate buffer" branch.
fn create_state_buffer_alloc_failure() {
    let p = pair();
    for &cap in &[-1i32, -2, i32::MIN, -4096] {
        let (c_null, c_out) = capture(|| unsafe {
            let s = (p.c.create_state)(7, cap);
            let n = s.is_null();
            (p.c.destroy_state)(s);
            n
        });
        let (r_null, r_out) = capture(|| unsafe {
            let s = (p.rs.create_state)(7, cap);
            let n = s.is_null();
            (p.rs.destroy_state)(s);
            n
        });
        let ctx = format!("create_state(7, {cap})");
        assert_eq!(c_null, r_null, "{ctx}");
        assert_stdout_eq(&ctx, &c_out, &r_out);
    }
}

fn destroy_state_null_is_noop() {
    let p = pair();
    let ((), c_out) = capture(|| unsafe { (p.c.destroy_state)(std::ptr::null_mut()) });
    let ((), r_out) = capture(|| unsafe { (p.rs.destroy_state)(std::ptr::null_mut()) });
    assert_stdout_eq("destroy_state(NULL)", &c_out, &r_out);
    assert!(c_out.is_empty(), "destroy_state(NULL) should be silent");
}

/// A state whose `buffer` is NULL must be freed without touching it, and a
/// state allocated by one library must be destroyable by the other (both use
/// the same libc allocator).
fn destroy_state_cross_and_null_buffer() {
    let p = pair();
    unsafe {
        // NULL buffer.
        for imp in [&p.c, &p.rs] {
            let s = common::OwnedState::new(0x1234, 0x5678, 16, None);
            let raw = s.ptr;
            std::mem::forget(s); // ownership handed to destroy_state
            let ((), out) = capture(|| (imp.destroy_state)(raw));
            assert!(out.is_empty(), "{} destroy_state printed output", imp.name);
        }
        // Cross-library create/destroy.
        let (a, _) = capture(|| (p.c.create_state)(5, 32));
        let ((), _) = capture(|| (p.rs.destroy_state)(a));
        let (b, _) = capture(|| (p.rs.create_state)(5, 32));
        let ((), _) = capture(|| (p.c.destroy_state)(b));
    }
}

// ---------------------------------------------------------------------------
// 2. process_buffer
// ---------------------------------------------------------------------------

fn process_buffer_null_state() {
    let p = pair();
    for &t in &[0i8, b'0' as i8, -1i8] {
        let (c_r, c_out) = capture(|| unsafe {
            (p.c.process_buffer)(std::ptr::null_mut(), t as c_char)
        });
        let (r_r, r_out) = capture(|| unsafe {
            (p.rs.process_buffer)(std::ptr::null_mut(), t as c_char)
        });
        let ctx = format!("process_buffer(NULL, {t})");
        assert_eq!(c_r, r_r, "{ctx}");
        assert_stdout_eq(&ctx, &c_out, &r_out);
    }
}

fn process_buffer_null_buffer() {
    let p = pair();
    let sc = OwnedState::new(0x7FFD, 0, 16, None);
    let sr = OwnedState::new(0x7FFD, 0, 16, None);
    let (c_r, c_out) = capture(|| unsafe { (p.c.process_buffer)(sc.ptr, b'0' as c_char) });
    let (r_r, r_out) = capture(|| unsafe { (p.rs.process_buffer)(sr.ptr, b'0' as c_char) });
    assert_eq!(c_r, r_r);
    assert_stdout_eq("process_buffer(state{buffer=NULL})", &c_out, &r_out);
}

fn process_buffer_matches() {
    let p = pair();
    let targets: Vec<i8> = {
        let mut t: Vec<i8> = (b'0'..=b'9').map(|b| b as i8).collect();
        t.extend_from_slice(&[
            0i8, 1, b'a' as i8, b'z' as i8, b'S' as i8, b':' as i8, b'M' as i8, 0x7F, -1, -2,
            -0x80, 0x41, 0x03,
        ]);
        t
    };

    for buf in buffer_corpus() {
        for &t in &targets {
            let sc = OwnedState::new(0x7FFD, 0x1234_5678, 128, Some((&buf, 128, 0xAB)));
            let sr = OwnedState::new(0x7FFD, 0x1234_5678, 128, Some((&buf, 128, 0xAB)));

            let (c_r, c_out) = capture(|| unsafe { (p.c.process_buffer)(sc.ptr, t as c_char) });
            let (r_r, r_out) = capture(|| unsafe { (p.rs.process_buffer)(sr.ptr, t as c_char) });

            let ctx = format!("process_buffer({:?}, target={})", show(&buf), t);
            assert_eq!(c_r, r_r, "return mismatch for {ctx}");
            assert_stdout_eq(&ctx, &c_out, &r_out);
            // The state itself must be untouched.
            assert_eq!(snapshot(sc.ptr), snapshot(sr.ptr), "state mutated: {ctx}");
        }
    }
}

// ---------------------------------------------------------------------------
// 3. update_flags
// ---------------------------------------------------------------------------

fn update_flags_null_state() {
    let p = pair();
    for &param in &[0, 1, -1, i32::MIN, i32::MAX] {
        let ((), c_out) = capture(|| unsafe { (p.c.update_flags)(std::ptr::null_mut(), param) });
        let ((), r_out) = capture(|| unsafe { (p.rs.update_flags)(std::ptr::null_mut(), param) });
        assert_stdout_eq(&format!("update_flags(NULL, {param})"), &c_out, &r_out);
        assert!(c_out.is_empty());
    }
}

fn update_flags_matches() {
    let p = pair();
    for &flags in &flags_corpus() {
        for &param in &int_corpus() {
            let sc = OwnedState::new(flags, 0xAAAA_5555, 128, Some((b"State:1:Mode:3", 128, 0)));
            let sr = OwnedState::new(flags, 0xAAAA_5555, 128, Some((b"State:1:Mode:3", 128, 0)));

            let ((), c_out) = capture(|| unsafe { (p.c.update_flags)(sc.ptr, param) });
            let ((), r_out) = capture(|| unsafe { (p.rs.update_flags)(sr.ptr, param) });

            let ctx = format!("update_flags(flags={flags:#010x}, {param})");
            assert_eq!(snapshot(sc.ptr), snapshot(sr.ptr), "state mismatch for {ctx}");
            assert_stdout_eq(&ctx, &c_out, &r_out);
        }
    }
}

/// Repeated calls exercise the 5-bit counter wrap-around.
fn update_flags_counter_wraps() {
    let p = pair();
    let sc = OwnedState::new(0x7FFD, 0, 128, Some((b"x", 8, 0)));
    let sr = OwnedState::new(0x7FFD, 0, 128, Some((b"x", 8, 0)));
    for i in 0..70i32 {
        let ((), c_out) = capture(|| unsafe { (p.c.update_flags)(sc.ptr, i) });
        let ((), r_out) = capture(|| unsafe { (p.rs.update_flags)(sr.ptr, i) });
        let ctx = format!("update_flags iteration {i}");
        assert_eq!(snapshot(sc.ptr), snapshot(sr.ptr), "{ctx}");
        assert_stdout_eq(&ctx, &c_out, &r_out);
    }
}

// ---------------------------------------------------------------------------
// 4. confuse_types
// ---------------------------------------------------------------------------

fn confuse_types_null_state() {
    let p = pair();
    for op in -5..=5 {
        let (c_r, c_out) = capture(|| unsafe { (p.c.confuse_types)(std::ptr::null_mut(), op) });
        let (r_r, r_out) = capture(|| unsafe { (p.rs.confuse_types)(std::ptr::null_mut(), op) });
        assert_eq!(c_r, r_r, "confuse_types(NULL, {op})");
        assert_stdout_eq(&format!("confuse_types(NULL, {op})"), &c_out, &r_out);
    }
}

fn confuse_types_matches() {
    let p = pair();
    let ops: Vec<c_int> = vec![-4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 100, i32::MIN, i32::MAX];

    for &data in &u32_corpus() {
        for &op in &ops {
            let sc = OwnedState::new(0x7FFD, data, 128, Some((b"State:1:Mode:3", 128, 0)));
            let sr = OwnedState::new(0x7FFD, data, 128, Some((b"State:1:Mode:3", 128, 0)));

            let (c_r, c_out) = capture(|| unsafe { (p.c.confuse_types)(sc.ptr, op) });
            let (r_r, r_out) = capture(|| unsafe { (p.rs.confuse_types)(sr.ptr, op) });

            let ctx = format!("confuse_types(data={data:#010x}, op={op})");
            assert_eq!(c_r, r_r, "return mismatch for {ctx}");
            assert_eq!(snapshot(sc.ptr), snapshot(sr.ptr), "state mismatch for {ctx}");
            assert_stdout_eq(&ctx, &c_out, &r_out);
        }
    }
}

// ---------------------------------------------------------------------------
// 5. confusion (top level)
// ---------------------------------------------------------------------------

fn check_confusion(p: &Pair, a: c_int, b: c_int, c: c_int, d: c_int) {
    let (c_r, c_out) = capture(|| unsafe { (p.c.confusion)(a, b, c, d) });
    let (r_r, r_out) = capture(|| unsafe { (p.rs.confusion)(a, b, c, d) });
    let ctx = format!("confusion({a}, {b}, {c}, {d})");
    assert_eq!(c_r, r_r, "return mismatch for {ctx}");
    assert_stdout_eq(&ctx, &c_out, &r_out);
}

fn confusion_small_grid() {
    let p = pair();
    let vals: [c_int; 8] = [0, 1, 2, 3, 4, 7, -1, -3];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    check_confusion(p, a, b, c, d);
                }
            }
        }
    }
}

fn confusion_edge_values() {
    let p = pair();
    let vals: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        11,
        -9,
        -10,
        -11,
        1078530011,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        255,
        256,
        -255,
        65536,
        -65536,
        1000000007,
        -1000000007,
    ];
    for &a in &vals {
        for &b in &vals {
            check_confusion(p, a, b, 3, 1);
        }
    }
    for &c in &vals {
        for &d in &vals {
            check_confusion(p, 1078530011, 0b1011, c, d);
        }
    }
}

fn confusion_random() {
    let p = pair();
    let mut s: u64 = 0xfeed_face_0bad_c0de;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (s >> 32) as u32 as c_int
    };
    for _ in 0..500 {
        let a = next();
        let b = next();
        let c = next();
        let d = next();
        check_confusion(p, a, b, c, d);
    }
}

/// `param4 % 4 == 1` reads the union as a float, so sweep `param1` over the
/// bit patterns that make that read interesting.
fn confusion_float_reinterpretation() {
    let p = pair();
    for &bits in &u32_corpus() {
        let a = bits as c_int;
        for d in [1, 5, -3, 9] {
            check_confusion(p, a, 0, 0, d);
        }
        check_confusion(p, a, 0x7FFF_FFFF, 7, 2);
        check_confusion(p, a, -1, -7, 3);
    }
}

// ---------------------------------------------------------------------------
// 6. High-volume batched sweeps
//
// One fd-1 capture per implementation for a whole batch of calls, which makes
// tens of thousands of comparisons cheap while still being byte-exact.
// ---------------------------------------------------------------------------

fn sweep_u32(seed: u64, n: usize) -> Vec<u32> {
    // Systematic coverage of every float exponent with a few mantissas and
    // both signs, then a pseudo-random tail.
    let mut v = Vec::with_capacity(n + 256 * 12);
    let mantissas: [u32; 6] = [0x00_0000, 0x00_0001, 0x40_0000, 0x7F_FFFF, 0x2A_AAAA, 0x12_3456];
    for exp in 0u32..256 {
        for &m in &mantissas {
            for sign in 0u32..2 {
                v.push((sign << 31) | (exp << 23) | m);
            }
        }
    }
    let mut s = seed;
    for _ in 0..n {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        v.push((s >> 32) as u32);
    }
    v
}

/// Every `confuse_types` operation against a very large set of union bit
/// patterns.  This is the main stress test for the float reinterpretation,
/// the `(int)(float * 100)` truncation and the signed-`char` byte reads.
fn confuse_types_sweep() {
    let p = pair();
    let vals = sweep_u32(0xa1b2_c3d4_e5f6_0718, 20_000);

    for op in [0i32, 1, 2, 3, -1, 4] {
        let run = |imp: &Impl| {
            let s = OwnedState::new(0x7FFD, 0, 128, Some((b"State:1:Mode:3", 128, 0)));
            capture(|| {
                let mut rets = Vec::with_capacity(vals.len());
                let mut datas = Vec::with_capacity(vals.len());
                for &v in &vals {
                    unsafe {
                        (*s.ptr).data = v;
                        rets.push((imp.confuse_types)(s.ptr, op));
                        datas.push((*s.ptr).data);
                    }
                }
                (rets, datas)
            })
        };
        let ((c_rets, c_datas), c_out) = run(&p.c);
        let ((r_rets, r_datas), r_out) = run(&p.rs);

        assert_eq!(
            c_rets.len(),
            r_rets.len(),
            "confuse_types sweep length (op={op})"
        );
        for (i, (&cv, &rv)) in c_rets.iter().zip(r_rets.iter()).enumerate() {
            assert_eq!(
                cv, rv,
                "confuse_types(data={:#010x}, op={op}) -> C {cv} vs Rust {rv}",
                vals[i]
            );
        }
        for (i, (&cv, &rv)) in c_datas.iter().zip(r_datas.iter()).enumerate() {
            assert_eq!(
                cv, rv,
                "confuse_types(data={:#010x}, op={op}) left data C {cv:#010x} vs Rust {rv:#010x}",
                vals[i]
            );
        }
        assert_stdout_eq(&format!("confuse_types sweep op={op}"), &c_out, &r_out);
    }
}

/// `update_flags` over a large product of starting bit patterns and params.
fn update_flags_sweep() {
    let p = pair();
    let mut cases: Vec<(u32, c_int)> = Vec::new();
    let mut s: u64 = 0x5150_2050_1050_0500;
    for _ in 0..20_000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let flags = (s >> 32) as u32;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let param = (s >> 32) as u32 as c_int;
        cases.push((flags, param));
    }
    for f in 0u32..64 {
        for prm in -40i32..40 {
            cases.push((f, prm));
        }
    }

    let run = |imp: &Impl| {
        let st = OwnedState::new(0, 0, 128, Some((b"x", 8, 0)));
        capture(|| {
            let mut out = Vec::with_capacity(cases.len());
            for &(flags, param) in &cases {
                unsafe {
                    (*st.ptr).flags = flags;
                    (imp.update_flags)(st.ptr, param);
                    out.push((*st.ptr).flags);
                }
            }
            out
        })
    };
    let (c_flags, c_out) = run(&p.c);
    let (r_flags, r_out) = run(&p.rs);

    for (i, (&cv, &rv)) in c_flags.iter().zip(r_flags.iter()).enumerate() {
        assert_eq!(
            cv, rv,
            "update_flags(flags={:#010x}, {}) -> C {cv:#010x} vs Rust {rv:#010x}",
            cases[i].0, cases[i].1
        );
    }
    assert_eq!(c_flags.len(), r_flags.len());
    assert_stdout_eq("update_flags sweep", &c_out, &r_out);
}

/// `process_buffer` over pseudo-random byte buffers and every possible target
/// byte value.
fn process_buffer_sweep() {
    let p = pair();
    let mut bufs: Vec<Vec<u8>> = buffer_corpus();
    let mut s: u64 = 0x0bad_f00d_1337_2468;
    let mut rnd = move || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (s >> 32) as u32
    };
    for _ in 0..64 {
        let len = (rnd() % 40) as usize;
        let mut b = Vec::with_capacity(len);
        for _ in 0..len {
            // Never zero: it would terminate the string early.
            b.push(((rnd() % 255) + 1) as u8);
        }
        bufs.push(b);
    }
    // A buffer containing every non-zero byte value.
    bufs.push((1u8..=255u8).collect());

    let targets: Vec<i8> = (0u16..256).map(|b| b as u8 as i8).collect();

    let run = |imp: &Impl| {
        capture(|| {
            let mut out = Vec::with_capacity(bufs.len() * targets.len());
            for b in &bufs {
                let st = OwnedState::new(0x7FFD, 0x1234_5678, 128, Some((b, 300, 0xAB)));
                for &t in &targets {
                    unsafe { out.push((imp.process_buffer)(st.ptr, t as c_char)) };
                }
            }
            out
        })
    };
    let (c_ret, c_out) = run(&p.c);
    let (r_ret, r_out) = run(&p.rs);

    assert_eq!(c_ret.len(), r_ret.len());
    for (i, (&cv, &rv)) in c_ret.iter().zip(r_ret.iter()).enumerate() {
        let bi = i / targets.len();
        let ti = i % targets.len();
        assert_eq!(
            cv, rv,
            "process_buffer(buf#{bi} {:?}, target={}) -> C {cv} vs Rust {rv}",
            show(&bufs[bi]), targets[ti]
        );
    }
    assert_stdout_eq("process_buffer sweep", &c_out, &r_out);
}

/// `create_state` over a large set of (initial_val, capacity) pairs.
fn create_state_sweep() {
    let p = pair();
    let mut cases: Vec<(c_int, c_int)> = Vec::new();
    let mut s: u64 = 0x7777_1111_2222_3333;
    for _ in 0..4000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let init = (s >> 32) as u32 as c_int;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let cap = ((s >> 32) as u32 % 40 + 1) as c_int;
        cases.push((init, cap));
    }
    for &init in &int_corpus() {
        for cap in 1..30 {
            cases.push((init, cap));
        }
    }

    let run = |imp: &Impl| {
        capture(|| {
            let mut out = Vec::with_capacity(cases.len());
            for &(init, cap) in &cases {
                unsafe {
                    let st = (imp.create_state)(init, cap);
                    out.push(snapshot(st));
                    (imp.destroy_state)(st);
                }
            }
            out
        })
    };
    let (c_snaps, c_out) = run(&p.c);
    let (r_snaps, r_out) = run(&p.rs);

    assert_eq!(c_snaps.len(), r_snaps.len());
    for (i, (cv, rv)) in c_snaps.iter().zip(r_snaps.iter()).enumerate() {
        assert_eq!(cv, rv, "create_state{:?}", cases[i]);
    }
    assert_stdout_eq("create_state sweep", &c_out, &r_out);
}

/// The top-level entry point over a large pseudo-random input space, plus a
/// grid biased towards the interesting `% 10` and `% 4` residues.
fn confusion_sweep() {
    let p = pair();
    let mut cases: Vec<(c_int, c_int, c_int, c_int)> = Vec::new();
    let mut s: u64 = 0x2468_ace0_1357_9bdf;
    let mut rnd = move || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (s >> 32) as u32 as c_int
    };
    for _ in 0..4000 {
        cases.push((rnd(), rnd(), rnd(), rnd()));
    }
    // Small-magnitude grid: exercises every (param3 % 10, param4 % 4) pair and
    // every `mode` / flag combination.
    for a in [0, 1, -1, 1078530011, i32::MAX, i32::MIN] {
        for b in -16..16 {
            for c in -11..11 {
                for d in -4..4 {
                    cases.push((a, b, c, d));
                }
            }
        }
    }

    let run = |imp: &Impl| {
        capture(|| {
            let mut out = Vec::with_capacity(cases.len());
            for &(a, b, c, d) in &cases {
                unsafe { out.push((imp.confusion)(a, b, c, d)) };
            }
            out
        })
    };
    let (c_ret, c_out) = run(&p.c);
    let (r_ret, r_out) = run(&p.rs);

    assert_eq!(c_ret.len(), r_ret.len());
    for (i, (&cv, &rv)) in c_ret.iter().zip(r_ret.iter()).enumerate() {
        let (a, b, c, d) = cases[i];
        assert_eq!(cv, rv, "confusion({a}, {b}, {c}, {d}) -> C {cv} vs Rust {rv}");
    }
    assert_stdout_eq("confusion sweep", &c_out, &r_out);
}

// ---------------------------------------------------------------------------
// Custom sequential runner
// ---------------------------------------------------------------------------

fn main() {
    // Ordered lowest-level first, matching the C call hierarchy.
    let cases: &[(&str, fn())] = &[
        ("create_state_matches", create_state_matches),
        ("create_state_zero_capacity", create_state_zero_capacity),
        (
            "create_state_buffer_alloc_failure",
            create_state_buffer_alloc_failure,
        ),
        ("destroy_state_null_is_noop", destroy_state_null_is_noop),
        (
            "destroy_state_cross_and_null_buffer",
            destroy_state_cross_and_null_buffer,
        ),
        ("process_buffer_null_state", process_buffer_null_state),
        ("process_buffer_null_buffer", process_buffer_null_buffer),
        ("process_buffer_matches", process_buffer_matches),
        ("update_flags_null_state", update_flags_null_state),
        ("update_flags_matches", update_flags_matches),
        ("update_flags_counter_wraps", update_flags_counter_wraps),
        ("confuse_types_null_state", confuse_types_null_state),
        ("confuse_types_matches", confuse_types_matches),
        ("confusion_small_grid", confusion_small_grid),
        ("confusion_edge_values", confusion_edge_values),
        ("confusion_random", confusion_random),
        (
            "confusion_float_reinterpretation",
            confusion_float_reinterpretation,
        ),
        // High-volume batched sweeps.
        ("create_state_sweep", create_state_sweep),
        ("process_buffer_sweep", process_buffer_sweep),
        ("update_flags_sweep", update_flags_sweep),
        ("confuse_types_sweep", confuse_types_sweep),
        ("confusion_sweep", confusion_sweep),
    ];

    // Support the usual `cargo test <filter>` invocation.
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();
    let mut skipped = 0usize;

    eprintln!("\nrunning {} differential tests", cases.len());
    for (name, f) in cases {
        if !filters.is_empty() && !filters.iter().any(|x| name.contains(x.as_str())) {
            skipped += 1;
            continue;
        }
        eprint!("test {name} ... ");
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match res {
            Ok(()) => {
                eprintln!("ok");
                passed += 1;
            }
            Err(e) => {
                eprintln!("FAILED");
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic>".to_string()
                };
                failures.push(format!("---- {name} ----\n{msg}"));
            }
        }
    }

    eprintln!();
    if failures.is_empty() {
        eprintln!("test result: ok. {passed} passed; 0 failed; {skipped} filtered out\n");
    } else {
        eprintln!("failures:\n");
        for f in &failures {
            eprintln!("{f}\n");
        }
        eprintln!(
            "test result: FAILED. {passed} passed; {} failed; {skipped} filtered out\n",
            failures.len()
        );
        std::process::exit(1);
    }
}

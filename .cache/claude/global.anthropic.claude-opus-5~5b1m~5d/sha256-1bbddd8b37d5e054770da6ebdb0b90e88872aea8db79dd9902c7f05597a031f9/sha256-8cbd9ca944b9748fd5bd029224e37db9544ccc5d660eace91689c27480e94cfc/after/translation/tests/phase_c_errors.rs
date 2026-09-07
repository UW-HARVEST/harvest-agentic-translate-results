//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! The C function returns `void` and validates nothing, so the "error surface"
//! consists of the generic C-API boundary conditions. Fatal cases (null
//! pointers) are run in a forked child so the raw `wait` status of the C call
//! and of the Rust call can be compared exactly.

mod common;

use common::{hex, Arena, Md5, Pair, Rng};

extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Call `f(m, out)` in a forked child; return the raw `wait` status.
/// A clean return yields status 0; a fault yields the encoded signal.
fn status_of(f: common::DigestFn, m: *const Md5, out: *mut u8) -> i32 {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f(m, out);
            _exit(0);
        }
        let mut st: i32 = -1;
        let r = waitpid(pid, &mut st as *mut i32, 0);
        assert_eq!(r, pid, "waitpid failed");
        st
    }
}

fn termsig(status: i32) -> i32 {
    status & 0x7f
}
fn exitcode(status: i32) -> i32 {
    (status >> 8) & 0xff
}

fn describe(status: i32) -> String {
    if termsig(status) == 0 {
        format!("exited({})", exitcode(status))
    } else {
        format!("signal({})", termsig(status))
    }
}

fn assert_same_fatal(label: &str, p: &Pair, m: *const Md5, out: *mut u8) {
    let sc = status_of(p.c, m, out);
    let sr = status_of(p.rs, m, out);
    assert_eq!(
        termsig(sc),
        termsig(sr),
        "{label}: C {} vs rust {}",
        describe(sc),
        describe(sr)
    );
    assert_eq!(
        exitcode(sc),
        exitcode(sr),
        "{label}: C {} vs rust {}",
        describe(sc),
        describe(sr)
    );
    assert_eq!(
        termsig(sc),
        11,
        "{label}: expected SIGSEGV from the C library, got {}",
        describe(sc)
    );
}

// ------------------------------------------------------------------- row 1
#[test]
fn err_null_m_faults_identically() {
    let p = common::pair();
    let mut out = [0u8; 16];
    assert_same_fatal("m == NULL", &p, std::ptr::null(), out.as_mut_ptr());
    // `out` must be untouched, since the load of m->a precedes any store.
    assert_eq!(out, [0u8; 16]);
}

// ------------------------------------------------------------------- row 2
#[test]
fn err_null_out_faults_identically() {
    let p = common::pair();
    let m = Md5 { a: 1, b: 2, c: 3, d: 4 };
    assert_same_fatal("out == NULL", &p, &m as *const Md5, std::ptr::null_mut());
}

// ------------------------------------------------------------------- row 3
#[test]
fn err_both_null_fault_identically() {
    let p = common::pair();
    assert_same_fatal(
        "m == NULL && out == NULL",
        &p,
        std::ptr::null(),
        std::ptr::null_mut(),
    );
}

// ------------------------------------------------------------------- row 4
#[test]
fn err_undersized_out_writes_16_anyway() {
    // There is no length parameter, so the C cannot reject a short buffer: it
    // writes 16 bytes regardless. Observe the overrun inside a large arena so
    // the "logical" buffer sizes 0,1,8,15 are all covered without crashing.
    let p = common::pair();
    let mut rng = Rng::with_seed(0xC0FF_EE00_1234_5678);
    for logical_len in [0usize, 1, 2, 8, 15, 16] {
        for _ in 0..500 {
            let m = rng.next_md5_biased();
            let mut ac = Arena::new(0x5A);
            let mut ar = Arena::new(0x5A);
            const OFF: usize = 32;
            unsafe {
                (p.c)(&m as *const Md5, ac.ptr(OFF));
                (p.rs)(&m as *const Md5, ar.ptr(OFF));
            }
            assert_eq!(
                &ac.0[..],
                &ar.0[..],
                "logical_len {logical_len}: C={} rust={}",
                hex(&ac.0),
                hex(&ar.0)
            );
            // exactly 16 bytes written, no more
            assert!(ac.0[..OFF].iter().all(|&b| b == 0x5A));
            assert!(ac.0[OFF + 16..].iter().all(|&b| b == 0x5A));
            // the bytes past `logical_len` were clobbered identically
            assert_eq!(
                &ac.0[OFF + logical_len..OFF + 16],
                &ar.0[OFF + logical_len..OFF + 16]
            );
        }
    }
}

// ------------------------------------------------------------------- rows 5 / 9
#[test]
fn err_no_enum_surface_all_bit_patterns_valid() {
    // The header declares no enum, so there is no "invalid variant" to feed.
    // The nearest boundary class is arbitrary 32-bit words, every one of which
    // is a valid tflac_u32; including both ends of the range and the wrap.
    let p = common::pair();
    let mut rng = Rng::with_seed(0x9216_D5D9_8979_FB1B);

    let mut cases: Vec<Md5> = vec![
        Md5 { a: 0, b: 0, c: 0, d: 0 },
        Md5 { a: u32::MAX, b: u32::MAX, c: u32::MAX, d: u32::MAX },
        // "one past the top of the range" wraps to 0 at the caller:
        Md5 {
            a: u32::MAX.wrapping_add(1),
            b: u32::MIN.wrapping_sub(1),
            c: 0x8000_0000,
            d: 0x7FFF_FFFF,
        },
        Md5 { a: 1, b: u32::MAX - 1, c: 0x0000_FFFF, d: 0xFFFF_0000 },
    ];
    for bit in 0..32 {
        let v = 1u32 << bit;
        cases.push(Md5 { a: v, b: !v, c: v.rotate_left(7), d: v.wrapping_neg() });
    }
    for _ in 0..5_000 {
        cases.push(rng.next_md5());
    }

    for m in &cases {
        let mut oc = [0x11u8; 16];
        let mut or = [0x11u8; 16];
        unsafe {
            (p.c)(m as *const Md5, oc.as_mut_ptr());
            (p.rs)(m as *const Md5, or.as_mut_ptr());
        }
        assert_eq!(oc, or, "divergence for {m:?}: C={} rust={}", hex(&oc), hex(&or));
    }
}

// ------------------------------------------------------------------- row 6
#[test]
fn err_overlapping_buffers_cascade() {
    // Well-defined in C (character-type stores may alias any object) and the
    // C re-loads m->a..d from memory after every byte store, so overlapping
    // buffers produce a cascade rather than a snapshot copy. Cover every
    // overlap offset with deterministic patterns AND random content.
    let p = common::pair();
    const M_OFF: usize = 48;
    for delta in -16i32..=16 {
        for pattern in 0..6u32 {
            let mut rng = Rng::with_seed(0x7000 + (delta + 16) as u64 * 8 + pattern as u64);
            for iter in 0..200 {
                let mut ac = Arena::new(0);
                let mut ar = Arena::new(0);
                let mut seed = [0u8; 128];
                match pattern {
                    0 => rng.fill(&mut seed),
                    1 => seed = [0x00; 128],
                    2 => seed = [0xFF; 128],
                    3 => {
                        for (i, s) in seed.iter_mut().enumerate() {
                            *s = i as u8;
                        }
                    }
                    4 => {
                        for (i, s) in seed.iter_mut().enumerate() {
                            *s = if i % 2 == 0 { 0xAA } else { 0x55 };
                        }
                    }
                    _ => {
                        seed = [0x80; 128];
                        seed[M_OFF] = 0x01;
                    }
                }
                ac.0 = seed;
                ar.0 = seed;
                let out_off = (M_OFF as i32 + delta) as usize;
                unsafe {
                    (p.c)(ac.ptr(M_OFF) as *const Md5, ac.ptr(out_off));
                    (p.rs)(ar.ptr(M_OFF) as *const Md5, ar.ptr(out_off));
                }
                assert_eq!(
                    &ac.0[..],
                    &ar.0[..],
                    "overlap delta={delta} pattern={pattern} iter={iter}\nC   ={}\nrust={}",
                    hex(&ac.0),
                    hex(&ar.0)
                );
            }
        }
    }
}

// ------------------------------------------------------------------- rows 7 / 8
#[test]
fn err_misaligned_m_and_out() {
    let p = common::pair();
    for moff in 0..8usize {
        for ooff in 0..8usize {
            let mut rng = Rng::with_seed(0x8000 + (moff * 8 + ooff) as u64);
            for _ in 0..300 {
                let m = rng.next_md5_biased();
                let mut src = Arena::new(0x33);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        &m as *const Md5 as *const u8,
                        src.ptr(moff),
                        16,
                    );
                }
                let mut ac = Arena::new(0x5A);
                let mut ar = Arena::new(0x5A);
                unsafe {
                    (p.c)(src.ptr(moff) as *const Md5, ac.ptr(64 + ooff));
                    (p.rs)(src.ptr(moff) as *const Md5, ar.ptr(64 + ooff));
                }
                assert_eq!(&ac.0[..], &ar.0[..], "m+{moff} out+{ooff} for {m:?}");
            }
        }
    }
}

// ------------------------------------------------- extra: symbol-level check
#[test]
fn err_unknown_symbol_absent_in_both() {
    // Neither library may accidentally export helper symbols the other lacks.
    unsafe {
        let cl = libloading::Library::new(common::c_so_path()).unwrap();
        let rl = libloading::Library::new(common::rust_so_path()).unwrap();
        for name in [
            b"md5_digest_impl\0".as_ref(),
            b"tflac_md5_digest\0".as_ref(),
            b"md5_init\0".as_ref(),
            b"md5_update\0".as_ref(),
        ] {
            let in_c = cl.get::<common::DigestFn>(name).is_ok();
            let in_r = rl.get::<common::DigestFn>(name).is_ok();
            assert_eq!(
                in_c,
                in_r,
                "symbol {:?} present in C={in_c} rust={in_r}",
                std::str::from_utf8(&name[..name.len() - 1]).unwrap()
            );
        }
        // and the one real symbol resolves in both
        assert!(cl.get::<common::DigestFn>(b"md5_digest\0").is_ok());
        assert!(rl.get::<common::DigestFn>(b"md5_digest\0").is_ok());
    }
}

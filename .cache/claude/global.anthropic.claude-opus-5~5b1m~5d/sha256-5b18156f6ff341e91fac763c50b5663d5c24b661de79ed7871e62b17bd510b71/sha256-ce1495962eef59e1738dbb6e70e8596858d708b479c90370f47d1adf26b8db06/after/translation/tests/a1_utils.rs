//! Area 1 — `sodium/utils.c`: memzero/memcmp/compare/is_zero/increment/add/sub,
//! pad/unpad, the guarded allocator, mlock/mprotect, and `sodium/version.c`.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const ENOSYS: c_int = 38;

type VoidPtrLen = unsafe extern "C" fn(*mut c_void, usize);
type Cmp = unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int;
type CmpU8 = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
type IsZero = unsafe extern "C" fn(*const u8, usize) -> c_int;
type Incr = unsafe extern "C" fn(*mut u8, usize);
type AddSub = unsafe extern "C" fn(*mut u8, *const u8, usize);
type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
type AllocArray = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type Free = unsafe extern "C" fn(*mut c_void);
type MProtect = unsafe extern "C" fn(*mut c_void) -> c_int;
type MLock = unsafe extern "C" fn(*mut c_void, usize) -> c_int;

// ------------------------------------------------------------- sodium_memzero

#[test]
fn memzero_all_lengths() {
    let (c, r) = both::<VoidPtrLen>("sodium_memzero");
    let mut rng = Rng::new(0x101);
    for len in 0..=257usize {
        let base = rng.bytes(len + PAD);
        let mut a = base.clone();
        let mut b = base.clone();
        unsafe {
            c(a.as_mut_ptr() as *mut c_void, len);
            r(b.as_mut_ptr() as *mut c_void, len);
        }
        eqb(&format!("memzero({len})"), &a, &b);
        assert!(a[..len].iter().all(|&x| x == 0));
        assert_eq!(&a[len..], &base[len..], "memzero wrote past len");
    }
}

#[test]
fn stackzero_does_not_crash_and_agrees() {
    let (c, r) = both::<unsafe extern "C" fn(usize)>("sodium_stackzero");
    for len in [0usize, 1, 16, 512, 4096] {
        unsafe {
            c(len);
            r(len);
        }
    }
}

// ------------------------------------------------------------- sodium_memcmp

#[test]
fn memcmp_equal_and_differing() {
    let (c, r) = both::<Cmp>("sodium_memcmp");
    let mut rng = Rng::new(0x102);
    for len in 0..=80usize {
        for _ in 0..6 {
            let a = rng.bytes(len);
            // identical
            unsafe {
                let rc = c(a.as_ptr() as *const c_void, a.as_ptr() as *const c_void, len);
                let rr = r(a.as_ptr() as *const c_void, a.as_ptr() as *const c_void, len);
                eqi(&format!("memcmp equal({len})"), rc, rr);
            }
            let b = a.clone();
            unsafe {
                eqi(
                    &format!("memcmp copy({len})"),
                    c(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, len),
                    r(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, len),
                );
            }
            // flip each byte / each bit position
            for k in 0..len {
                for bit in [0u8, 1, 7, 0x80] {
                    let mut d = a.clone();
                    d[k] ^= bit.max(1);
                    unsafe {
                        eqi(
                            &format!("memcmp diff({len},{k})"),
                            c(a.as_ptr() as *const c_void, d.as_ptr() as *const c_void, len),
                            r(a.as_ptr() as *const c_void, d.as_ptr() as *const c_void, len),
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------ sodium_compare

#[test]
fn compare_all_orderings() {
    let (c, r) = both::<CmpU8>("sodium_compare");
    let mut rng = Rng::new(0x103);
    for len in 0..=48usize {
        let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        pairs.push((vec![0u8; len], vec![0u8; len]));
        pairs.push((vec![0u8; len], vec![0xffu8; len]));
        pairs.push((vec![0xffu8; len], vec![0u8; len]));
        for _ in 0..10 {
            pairs.push((rng.bytes(len), rng.bytes(len)));
        }
        for k in 0..len {
            let mut a = vec![0x40u8; len];
            let mut b = a.clone();
            a[k] = 0x41;
            pairs.push((a.clone(), b.clone()));
            b[k] = 0x42;
            pairs.push((a, b));
        }
        for (a, b) in pairs {
            unsafe {
                eqi(
                    &format!("compare({len}) {} vs {}", hex(&a), hex(&b)),
                    c(a.as_ptr(), b.as_ptr(), len),
                    r(a.as_ptr(), b.as_ptr(), len),
                );
            }
        }
    }
}

#[test]
fn is_zero_all_shapes() {
    let (c, r) = both::<IsZero>("sodium_is_zero");
    let mut rng = Rng::new(0x104);
    for len in 0..=64usize {
        let mut cases = vec![vec![0u8; len]];
        for k in 0..len {
            let mut v = vec![0u8; len];
            v[k] = 1;
            cases.push(v.clone());
            v[k] = 0x80;
            cases.push(v);
        }
        for _ in 0..4 {
            cases.push(rng.bytes(len));
        }
        for v in cases {
            unsafe {
                eqi(
                    &format!("is_zero({len}) {}", hex(&v)),
                    c(v.as_ptr(), len),
                    r(v.as_ptr(), len),
                );
            }
        }
    }
}

// --------------------------------------------------- increment / add / sub

#[test]
fn increment_all_widths_and_carries() {
    let (c, r) = both::<Incr>("sodium_increment");
    let mut rng = Rng::new(0x105);
    // widths include the 8/12/24 special cases of the asm path
    for len in 0..=40usize {
        let mut cases: Vec<Vec<u8>> = vec![vec![0u8; len], vec![0xffu8; len]];
        for k in 0..len {
            let mut v = vec![0xffu8; len];
            v[k] = 0xfe;
            cases.push(v);
            let mut w = vec![0u8; len];
            w[k] = 0xff;
            cases.push(w);
        }
        for _ in 0..6 {
            cases.push(rng.bytes(len));
        }
        for v in cases {
            let mut a = v.clone();
            a.extend_from_slice(&[0x5A; PAD]);
            let mut b = a.clone();
            unsafe {
                c(a.as_mut_ptr(), len);
                r(b.as_mut_ptr(), len);
            }
            eqb(&format!("increment({len}) {}", hex(&v)), &a, &b);
            assert!(a[len..].iter().all(|&x| x == 0x5A));
        }
    }
}

#[test]
fn add_all_widths_and_carries() {
    let (c, r) = both::<AddSub>("sodium_add");
    let mut rng = Rng::new(0x106);
    for len in 0..=40usize {
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![0u8; len], vec![0u8; len]),
            (vec![0xffu8; len], vec![1u8; len]),
            (vec![0xffu8; len], vec![0xffu8; len]),
            (vec![0u8; len], vec![0xffu8; len]),
        ];
        for _ in 0..8 {
            cases.push((rng.bytes(len), rng.bytes(len)));
        }
        for (av, bv) in cases {
            let mut a = av.clone();
            a.extend_from_slice(&[0x5A; PAD]);
            let mut a2 = a.clone();
            unsafe {
                c(a.as_mut_ptr(), bv.as_ptr(), len);
                r(a2.as_mut_ptr(), bv.as_ptr(), len);
            }
            eqb(&format!("add({len}) {}+{}", hex(&av), hex(&bv)), &a, &a2);
            assert!(a[len..].iter().all(|&x| x == 0x5A));
        }
    }
}

#[test]
fn sub_all_widths_and_borrows() {
    let (c, r) = both::<AddSub>("sodium_sub");
    let mut rng = Rng::new(0x107);
    for len in 0..=70usize {
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![0u8; len], vec![0u8; len]),
            (vec![0u8; len], vec![1u8; len]),
            (vec![0u8; len], vec![0xffu8; len]),
            (vec![0xffu8; len], vec![0xffu8; len]),
            (vec![1u8; len], vec![2u8; len]),
        ];
        for _ in 0..8 {
            cases.push((rng.bytes(len), rng.bytes(len)));
        }
        for (av, bv) in cases {
            let mut a = av.clone();
            a.extend_from_slice(&[0x5A; PAD]);
            let mut a2 = a.clone();
            unsafe {
                c(a.as_mut_ptr(), bv.as_ptr(), len);
                r(a2.as_mut_ptr(), bv.as_ptr(), len);
            }
            eqb(&format!("sub({len}) {}-{}", hex(&av), hex(&bv)), &a, &a2);
            assert!(a[len..].iter().all(|&x| x == 0x5A));
        }
    }
}

// --------------------------------------------------------------- pad / unpad

#[test]
fn pad_valid_and_rejections() {
    let (c, r) = both::<Pad>("sodium_pad");
    let mut rng = Rng::new(0x108);
    for blocksize in [1usize, 2, 3, 4, 5, 7, 8, 15, 16, 17, 32, 64, 100, 128] {
        for unpadded in 0..=70usize {
            let xpadlen = blocksize - 1 - (unpadded % blocksize);
            let xpadded = unpadded + xpadlen;
            for max_buflen in [xpadded, xpadded + 1, xpadded + 8, 0, 1] {
                for want_len in [true, false] {
                    let cap = xpadded + 1 + PAD;
                    let base = rng.bytes(cap);
                    let mut a = base.clone();
                    let mut b = base.clone();
                    let mut la: usize = usize::MAX;
                    let mut lb: usize = usize::MAX;
                    let rc = unsafe {
                        c(
                            if want_len { &mut la } else { std::ptr::null_mut() },
                            a.as_mut_ptr(),
                            unpadded,
                            blocksize,
                            max_buflen,
                        )
                    };
                    let rr = unsafe {
                        r(
                            if want_len { &mut lb } else { std::ptr::null_mut() },
                            b.as_mut_ptr(),
                            unpadded,
                            blocksize,
                            max_buflen,
                        )
                    };
                    let label = format!("pad(u={unpadded},bs={blocksize},max={max_buflen})");
                    eqi(&label, rc, rr);
                    if want_len {
                        assert_eq!(la, lb, "{label}: *padded_buflen_p");
                    }
                    eqb(&label, &a, &b);
                    if rc == 0 {
                        assert_eq!(&a[xpadded + 1..], &base[xpadded + 1..], "{label}: overwrite");
                    } else {
                        assert_eq!(&a[..], &base[..], "{label}: wrote on failure");
                    }
                }
            }
        }
    }
}

#[test]
fn pad_blocksize_zero_is_rejected() {
    let (c, r) = both::<Pad>("sodium_pad");
    for unpadded in [0usize, 1, 16] {
        let mut buf = [0u8; 64];
        let mut la = usize::MAX;
        let mut lb = usize::MAX;
        let rc = unsafe { c(&mut la, buf.as_mut_ptr(), unpadded, 0, 64) };
        let rr = unsafe { r(&mut lb, buf.as_mut_ptr(), unpadded, 0, 64) };
        eqi("pad blocksize=0", rc, rr);
        assert_eq!(rc, -1);
    }
}

#[test]
fn pad_size_overflow_aborts() {
    // SIZE_MAX - unpadded_buflen <= xpadlen  =>  sodium_misuse()
    let (c, r) = both::<Pad>("sodium_pad");
    for (unpadded, blocksize) in [(usize::MAX, 16usize), (usize::MAX - 1, 16), (usize::MAX, 2)] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("pad overflow u={unpadded} bs={blocksize}"),
            move || unsafe {
                let mut l = 0usize;
                let mut buf = [0u8; 256];
                std::hint::black_box(cc(&mut l, buf.as_mut_ptr(), unpadded, blocksize, usize::MAX));
            },
            move || unsafe {
                let mut l = 0usize;
                let mut buf = [0u8; 256];
                std::hint::black_box(rr(&mut l, buf.as_mut_ptr(), unpadded, blocksize, usize::MAX));
            },
        );
    }
}

#[test]
fn unpad_valid_and_rejections() {
    let (pad, _) = both::<Pad>("sodium_pad");
    let (c, r) = both::<Unpad>("sodium_unpad");
    let mut rng = Rng::new(0x109);
    for blocksize in [1usize, 2, 3, 4, 5, 8, 15, 16, 17, 32, 64] {
        for unpadded in 0..=50usize {
            let xpadlen = blocksize - 1 - (unpadded % blocksize);
            let padded_len = unpadded + xpadlen + 1;
            let mut buf = rng.bytes(padded_len + PAD);
            let mut l = 0usize;
            assert_eq!(
                unsafe { pad(&mut l, buf.as_mut_ptr(), unpadded, blocksize, padded_len + 1) },
                0
            );
            assert_eq!(l, padded_len);
            // valid
            let mut la = usize::MAX;
            let mut lb = usize::MAX;
            let rc = unsafe { c(&mut la, buf.as_ptr(), padded_len, blocksize) };
            let rr = unsafe { r(&mut lb, buf.as_ptr(), padded_len, blocksize) };
            let label = format!("unpad(u={unpadded},bs={blocksize})");
            eqi(&label, rc, rr);
            assert_eq!(la, lb, "{label}: *unpadded_buflen_p");
            assert_eq!(rc, 0);
            assert_eq!(la, unpadded);
            // corrupted padding: clear the 0x80 barrier, or set stray bits
            for k in 0..padded_len.min(blocksize) {
                let mut bad = buf.clone();
                bad[padded_len - 1 - k] ^= 0x80;
                let mut xa = usize::MAX;
                let mut xb = usize::MAX;
                let ra = unsafe { c(&mut xa, bad.as_ptr(), padded_len, blocksize) };
                let rb = unsafe { r(&mut xb, bad.as_ptr(), padded_len, blocksize) };
                eqi(&format!("{label} corrupt{k}"), ra, rb);
                assert_eq!(xa, xb, "{label} corrupt{k}: out len");
            }
            // shorter than blocksize / blocksize 0
            for (plen, bs) in [(blocksize - 1, blocksize), (padded_len, 0usize), (0, blocksize)] {
                let mut xa = usize::MAX;
                let mut xb = usize::MAX;
                let ra = unsafe { c(&mut xa, buf.as_ptr(), plen, bs) };
                let rb = unsafe { r(&mut xb, buf.as_ptr(), plen, bs) };
                eqi(&format!("unpad(plen={plen},bs={bs})"), ra, rb);
                assert_eq!(xa, xb);
            }
        }
    }
}

#[test]
fn unpad_random_fuzz() {
    let (c, r) = both::<Unpad>("sodium_unpad");
    let mut rng = Rng::new(0x10A);
    for _ in 0..6000 {
        let blocksize = rng.range(1, 40);
        let padded_len = rng.below(64);
        let mut buf = rng.bytes(padded_len.max(1) + PAD);
        // bias towards buffers containing 0x80 barriers
        for b in buf.iter_mut() {
            if rng.below(3) == 0 {
                *b = 0x80;
            } else if rng.below(3) == 0 {
                *b = 0;
            }
        }
        let mut xa = usize::MAX;
        let mut xb = usize::MAX;
        let ra = unsafe { c(&mut xa, buf.as_ptr(), padded_len, blocksize) };
        let rb = unsafe { r(&mut xb, buf.as_ptr(), padded_len, blocksize) };
        eqi("unpad fuzz", ra, rb);
        assert_eq!(xa, xb, "unpad fuzz out len");
    }
}

// ------------------------------------------------------------- allocator API

#[test]
fn malloc_contents_and_free() {
    let (c, r) = both::<Malloc>("sodium_malloc");
    let (cf, rf) = both::<Free>("sodium_free");
    for size in [0usize, 1, 2, 15, 16, 17, 63, 64, 4095, 4096, 4097, 100_000] {
        unsafe {
            let pc = c(size);
            let pr = r(size);
            assert!(!pc.is_null(), "C sodium_malloc({size}) returned NULL");
            assert!(!pr.is_null(), "Rust sodium_malloc({size}) returned NULL");
            let sc = std::slice::from_raw_parts(pc as *const u8, size);
            let sr = std::slice::from_raw_parts(pr as *const u8, size);
            eqb(&format!("sodium_malloc({size}) fill"), sc, sr);
            assert!(sc.iter().all(|&x| x == 0xdb), "not filled with 0xdb");
            cf(pc);
            rf(pr);
        }
    }
    // free(NULL) must be a no-op in both
    unsafe {
        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());
    }
}

#[test]
fn allocarray_overflow_rejected() {
    let (c, r) = both::<AllocArray>("sodium_allocarray");
    let (cf, rf) = both::<Free>("sodium_free");
    let cases: &[(usize, usize)] = &[
        (0, 0),
        (0, usize::MAX),
        (1, 1),
        (1, usize::MAX),
        (2, usize::MAX / 2),
        (2, usize::MAX / 2 - 1),
        (usize::MAX, 2),
        (usize::MAX, 1),
        (16, 32),
        (1024, 1024),
        (3, usize::MAX / 3),
    ];
    for &(count, size) in cases {
        unsafe {
            set_errno(0);
            let pc = c(count, size);
            let ec = errno();
            set_errno(0);
            let pr = r(count, size);
            let er = errno();
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "allocarray({count},{size}) NULL-ness mismatch (C {:?} Rust {:?})",
                pc,
                pr
            );
            if pc.is_null() {
                assert_eq!(ec, er, "allocarray({count},{size}) errno");
                assert_eq!(ec, ENOMEM);
            } else {
                let n = count * size;
                let sc = std::slice::from_raw_parts(pc as *const u8, n);
                let sr = std::slice::from_raw_parts(pr as *const u8, n);
                eqb(&format!("allocarray({count},{size})"), sc, sr);
                cf(pc);
                rf(pr);
            }
        }
    }
}

#[test]
fn mprotect_family_matches() {
    let (cm, rm) = both::<Malloc>("sodium_malloc");
    let (cf, rf) = both::<Free>("sodium_free");
    for name in [
        "sodium_mprotect_noaccess",
        "sodium_mprotect_readonly",
        "sodium_mprotect_readwrite",
    ] {
        let (c, r) = both::<MProtect>(name);
        unsafe {
            let pc = cm(64);
            let pr = rm(64);
            set_errno(0);
            let rc = c(pc);
            let ec = errno();
            set_errno(0);
            let rr = r(pr);
            let er = errno();
            eqi(name, rc, rr);
            assert_eq!(ec, er, "{name} errno");
            assert_eq!(rc, -1, "{name} unexpectedly succeeded in this build");
            assert_eq!(ec, ENOSYS);
            cf(pc);
            rf(pr);
        }
    }
}

#[test]
fn mlock_munlock_match() {
    for name in ["sodium_mlock", "sodium_munlock"] {
        let (c, r) = both::<MLock>(name);
        for len in [0usize, 1, 64, 4096] {
            let mut a = vec![0xAAu8; len + PAD];
            let mut b = vec![0xAAu8; len + PAD];
            unsafe {
                set_errno(0);
                let rc = c(a.as_mut_ptr() as *mut c_void, len);
                let ec = errno();
                set_errno(0);
                let rr = r(b.as_mut_ptr() as *mut c_void, len);
                let er = errno();
                eqi(&format!("{name}({len})"), rc, rr);
                assert_eq!(ec, er, "{name}({len}) errno");
                assert_eq!(rc, -1);
                assert_eq!(ec, ENOSYS);
            }
            // sodium_munlock zeroes the region first; verify both did the same
            eqb(&format!("{name}({len}) buffer"), &a, &b);
        }
    }
}

#[test]
fn alloc_init_matches() {
    let (c, r) = both::<unsafe extern "C" fn() -> c_int>("_sodium_alloc_init");
    unsafe {
        rng_reset();
        let rc = c();
        let rr = r();
        eqi("_sodium_alloc_init", rc, rr);
    }
}

// ---------------------------------------------------------------- version.c

#[test]
fn version_accessors() {
    unsafe {
        let (c, r) = both::<unsafe extern "C" fn() -> *const c_char>("sodium_version_string");
        assert_eq!(
            std::ffi::CStr::from_ptr(c()),
            std::ffi::CStr::from_ptr(r()),
            "sodium_version_string"
        );
        for name in ["sodium_library_version_major", "sodium_library_version_minor"] {
            let (c, r) = both::<unsafe extern "C" fn() -> c_int>(name);
            eqi(name, c(), r());
        }
        let (c, r) = both::<unsafe extern "C" fn() -> c_int>("sodium_library_minimal");
        eqi("sodium_library_minimal", c(), r());
    }
}

// ---------------------------------------------------------------- runtime.c

#[test]
fn runtime_feature_predicates() {
    for name in [
        "sodium_runtime_has_neon",
        "sodium_runtime_has_armcrypto",
        "sodium_runtime_has_sse2",
        "sodium_runtime_has_sse3",
        "sodium_runtime_has_ssse3",
        "sodium_runtime_has_sse41",
        "sodium_runtime_has_avx",
        "sodium_runtime_has_avx2",
        "sodium_runtime_has_avx512f",
        "sodium_runtime_has_pclmul",
        "sodium_runtime_has_aesni",
        "sodium_runtime_has_rdrand",
    ] {
        if !has(name) {
            continue;
        }
        let (c, r) = both::<unsafe extern "C" fn() -> c_int>(name);
        unsafe {
            eqi(name, c(), r());
        }
    }
    let (c, r) = both::<unsafe extern "C" fn() -> c_int>("_sodium_runtime_get_cpu_features");
    unsafe {
        eqi("_sodium_runtime_get_cpu_features", c(), r());
    }
}

/// config 1.171: in this build `sodium_malloc` is plain `malloc`, so there are
/// no guard pages — a one-byte overrun must NOT fault, in either implementation.
#[test]
fn sodium_malloc_has_no_guard_page_in_this_build() {
    let (cm, rm) = both::<Malloc>("sodium_malloc");
    let (cf, rf) = both::<Free>("sodium_free");
    eq_abort(
        "one byte past a sodium_malloc region",
        move || unsafe {
            let p = cm(32) as *mut u8;
            std::ptr::write_volatile(p.add(32), 0x11);
            let _ = std::ptr::read_volatile(p.add(32));
            cf(p as *mut c_void);
        },
        move || unsafe {
            let p = rm(32) as *mut u8;
            std::ptr::write_volatile(p.add(32), 0x11);
            let _ = std::ptr::read_volatile(p.add(32));
            rf(p as *mut c_void);
        },
    );
}

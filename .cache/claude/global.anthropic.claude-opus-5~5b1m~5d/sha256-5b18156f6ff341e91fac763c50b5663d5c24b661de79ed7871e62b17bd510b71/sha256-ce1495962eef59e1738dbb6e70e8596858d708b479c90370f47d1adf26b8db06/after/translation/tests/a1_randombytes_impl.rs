//! Area 1 — `randombytes_set_implementation` and the two default
//! implementations (`sysrandom`, `internal`).
//!
//! This lives in its own test binary because installing a different RNG is
//! global, process-wide state that would perturb every other test.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

type SetImpl = unsafe extern "C" fn(*const c_void) -> c_int;
type BufFn = unsafe extern "C" fn(*mut c_void, usize);
type U32Fn = unsafe extern "C" fn() -> u32;
type UniformFn = unsafe extern "C" fn(u32) -> u32;
type NameFn = unsafe extern "C" fn() -> *const c_char;
type IntFn = unsafe extern "C" fn() -> c_int;

/// Every test in this file swaps the process-global installed RNG, so they must
/// not overlap. libtest runs the tests of one binary on several threads.
static IMPL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The two implementation descriptors the C library exports must have exactly
/// the same shape (which callbacks are provided, which are NULL) in Rust.
#[test]
fn descriptor_layout_and_behaviour_of_default_implementations() {
    let _guard = IMPL_LOCK.lock().unwrap();
    let (set_c, set_r) = both::<SetImpl>("randombytes_set_implementation");
    let (name_c, name_r) = both::<NameFn>("randombytes_implementation_name");
    let (buf_c, buf_r) = both::<BufFn>("randombytes_buf");
    let (rnd_c, rnd_r) = both::<U32Fn>("randombytes_random");
    let (uni_c, uni_r) = both::<UniformFn>("randombytes_uniform");
    let (cls_c, cls_r) = both::<IntFn>("randombytes_close");

    for sym in [
        "randombytes_sysrandom_implementation",
        "randombytes_internal_implementation",
    ] {
        let (dc, dr) = both::<*const c_void>(sym);
        unsafe {
            // 6 function-pointer slots; addresses differ, NULL-ness must not.
            let cs = *dc as *const *const c_void;
            let rs = *dr as *const *const c_void;
            for i in 0..6 {
                assert_eq!(
                    (*cs.add(i)).is_null(),
                    (*rs.add(i)).is_null(),
                    "{sym}: callback slot {i} NULL-ness differs"
                );
            }

            // Installing it must succeed identically...
            eqi(
                &format!("set_implementation({sym})"),
                set_c(*dc),
                set_r(*dr),
            );
            // ...and the reported name must match byte for byte.
            let a = std::ffi::CStr::from_ptr(name_c());
            let b = std::ffi::CStr::from_ptr(name_r());
            assert_eq!(a, b, "{sym}: implementation_name()");

            // These implementations draw from the OS, so their *output* cannot
            // be compared; what is comparable is that they work, produce
            // non-constant data, and respect the documented contracts.
            for len in [0usize, 1, 7, 8, 64, 1000, 65536] {
                let mut x = padded(len);
                let mut y = padded(len);
                buf_c(x.as_mut_ptr() as *mut c_void, len);
                buf_r(y.as_mut_ptr() as *mut c_void, len);
                check_pad(&format!("{sym} buf({len}) C"), &x, len);
                check_pad(&format!("{sym} buf({len}) Rust"), &y, len);
                if len >= 64 {
                    assert!(x[..len].iter().any(|&v| v != 0), "{sym}: C buf all zero");
                    assert!(y[..len].iter().any(|&v| v != 0), "{sym}: Rust buf all zero");
                }
            }
            let mut seen_c = std::collections::HashSet::new();
            let mut seen_r = std::collections::HashSet::new();
            for _ in 0..64 {
                seen_c.insert(rnd_c());
                seen_r.insert(rnd_r());
            }
            assert!(seen_c.len() > 32, "{sym}: C random() not varying");
            assert!(seen_r.len() > 32, "{sym}: Rust random() not varying");

            for ub in [0u32, 1, 2, 3, 17, 1000, 0x8000_0001, 0xffff_ffff] {
                for _ in 0..64 {
                    let a = uni_c(ub);
                    let b = uni_r(ub);
                    if ub < 2 {
                        assert_eq!(a, 0, "{sym}: C uniform({ub})");
                        assert_eq!(b, 0, "{sym}: Rust uniform({ub})");
                    } else {
                        assert!(a < ub, "{sym}: C uniform({ub}) = {a} out of range");
                        assert!(b < ub, "{sym}: Rust uniform({ub}) = {b} out of range");
                    }
                }
            }
            eqi(&format!("{sym}: close()"), cls_c(), cls_r());
        }
    }
}

/// `randombytes_set_implementation()` performs no validation and always
/// returns 0 — including for a NULL pointer (the C code stores it verbatim).
#[test]
fn set_implementation_never_rejects() {
    let _guard = IMPL_LOCK.lock().unwrap();
    let (c, r) = both::<SetImpl>("randombytes_set_implementation");
    let (dc, dr) = both::<*const c_void>("randombytes_sysrandom_implementation");
    unsafe {
        eqi("set_implementation(NULL)", c(std::ptr::null()), r(std::ptr::null()));
        // With a NULL implementation installed, randombytes_close() must still
        // return 0 rather than crashing (it checks for NULL explicitly).
        let (cls_c, cls_r) = both::<IntFn>("randombytes_close");
        eqi("close() with NULL impl", cls_c(), cls_r());
        // Restore something usable.
        eqi("set_implementation(restore)", c(*dc), r(*dr));
    }
}

// ------------------------------------------------------- custom descriptors

static CB_COUNT: [std::sync::atomic::AtomicU32; 2] = [
    std::sync::atomic::AtomicU32::new(0),
    std::sync::atomic::AtomicU32::new(0),
];
static LAST_UB: [std::sync::atomic::AtomicU32; 2] = [
    std::sync::atomic::AtomicU32::new(0xdead_beef),
    std::sync::atomic::AtomicU32::new(0xdead_beef),
];

macro_rules! cb_set {
    ($i:expr, $name:ident, $rand:ident, $buf:ident, $uni:ident, $close:ident) => {
        unsafe extern "C" fn $name() -> *const c_char {
            b"custom\0".as_ptr() as *const c_char
        }
        unsafe extern "C" fn $rand() -> u32 {
            CB_COUNT[$i].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            0x1234_5678
        }
        unsafe extern "C" fn $buf(p: *mut c_void, n: usize) {
            CB_COUNT[$i].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            std::ptr::write_bytes(p as *mut u8, 0x5A, n);
        }
        unsafe extern "C" fn $uni(ub: u32) -> u32 {
            LAST_UB[$i].store(ub, std::sync::atomic::Ordering::Relaxed);
            0xFFFF_0000 | ub
        }
        unsafe extern "C" fn $close() -> c_int {
            -7
        }
    };
}

cb_set!(0, c_name2, c_rand2, c_buf2, c_uni2, c_close2);
cb_set!(1, r_name2, r_rand2, r_buf2, r_uni2, r_close2);

#[repr(C)]
struct Impl {
    name: Option<unsafe extern "C" fn() -> *const c_char>,
    random: Option<unsafe extern "C" fn() -> u32>,
    stir: Option<unsafe extern "C" fn()>,
    uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    close: Option<unsafe extern "C" fn() -> c_int>,
}

/// configs 1.184 / 1.186 / 1.188: `stir == NULL` is a silent no-op, a non-NULL
/// `uniform` is consulted BEFORE the `upper_bound < 2` guard, and `close`'s
/// return value is passed through verbatim.
#[test]
fn custom_descriptor_callback_dispatch() {
    let _guard = IMPL_LOCK.lock().unwrap();
    let (set_c, set_r) = both::<SetImpl>("randombytes_set_implementation");
    let (uni_c, uni_r) = both::<UniformFn>("randombytes_uniform");
    let (cls_c, cls_r) = both::<IntFn>("randombytes_close");
    let (stir_c, stir_r) = both::<unsafe extern "C" fn()>("randombytes_stir");
    let (buf_c, buf_r) = both::<BufFn>("randombytes_buf");
    let (rnd_c, rnd_r) = both::<U32Fn>("randombytes_random");
    let (name_c, name_r) = both::<NameFn>("randombytes_implementation_name");

    let ci: &'static Impl = Box::leak(Box::new(Impl {
        name: Some(c_name2),
        random: Some(c_rand2),
        stir: None, // 1.184
        uniform: Some(c_uni2), // 1.186
        buf: Some(c_buf2),
        close: Some(c_close2), // 1.188
    }));
    let ri: &'static Impl = Box::leak(Box::new(Impl {
        name: Some(r_name2),
        random: Some(r_rand2),
        stir: None,
        uniform: Some(r_uni2),
        buf: Some(r_buf2),
        close: Some(r_close2),
    }));

    unsafe {
        eqi(
            "set_implementation(custom)",
            set_c(ci as *const _ as *const c_void),
            set_r(ri as *const _ as *const c_void),
        );
        // stir == NULL must be a no-op, not a crash.
        stir_c();
        stir_r();

        // The custom `uniform` is consulted even for upper_bound 0 and 1, so the
        // library's own `< 2` guard is bypassed.
        for ub in [0u32, 1, 2, 3, 0xffff_ffff] {
            let a = uni_c(ub);
            let b = uni_r(ub);
            assert_eq!(a, b, "custom uniform({ub})");
            assert_eq!(a, 0xFFFF_0000 | ub, "custom uniform({ub}) not delegated");
            assert_eq!(
                LAST_UB[0].load(std::sync::atomic::Ordering::Relaxed),
                LAST_UB[1].load(std::sync::atomic::Ordering::Relaxed),
                "custom uniform({ub}) argument"
            );
        }

        // `close` value is returned verbatim.
        eqi("custom close()", cls_c(), cls_r());
        assert_eq!(cls_c(), -7);

        // buf / random route to the callbacks; size 0 must NOT call buf at all.
        CB_COUNT[0].store(0, std::sync::atomic::Ordering::Relaxed);
        CB_COUNT[1].store(0, std::sync::atomic::Ordering::Relaxed);
        buf_c(std::ptr::null_mut(), 0);
        buf_r(std::ptr::null_mut(), 0);
        assert_eq!(CB_COUNT[0].load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(CB_COUNT[1].load(std::sync::atomic::Ordering::Relaxed), 0);
        let mut x = [0u8; 9];
        let mut y = [0u8; 9];
        buf_c(x.as_mut_ptr() as *mut c_void, 9);
        buf_r(y.as_mut_ptr() as *mut c_void, 9);
        eqb("custom buf", &x, &y);
        assert_eq!(x, [0x5A; 9]);
        assert_eq!(rnd_c(), rnd_r());
        assert_eq!(rnd_c(), 0x1234_5678);
        assert_eq!(
            std::ffi::CStr::from_ptr(name_c()),
            std::ffi::CStr::from_ptr(name_r())
        );

        // Restore a real implementation.
        let (dc, dr) = both::<*const c_void>("randombytes_sysrandom_implementation");
        eqi("restore", set_c(*dc), set_r(*dr));
    }
}

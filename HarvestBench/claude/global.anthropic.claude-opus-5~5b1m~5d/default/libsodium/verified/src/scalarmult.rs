//! Translated from:
//!  - `crypto_scalarmult/crypto_scalarmult.c`
//!  - `crypto_scalarmult/curve25519/scalarmult_curve25519.c`
//!  - `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c`
//!  - `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c`
//!
//! The reference build has no SIMD implementations (`HAVE_AVX_ASM` is not
//! defined), so `_crypto_scalarmult_curve25519_pick_best_implementation`
//! always selects the ref10 implementation.

use core::ffi::c_int;

use crate::types::ge25519_p3;
use crate::x25519_ref10::crypto_scalarmult_curve25519_implementation;

const CRYPTO_SCALARMULT_CURVE25519_BYTES: usize = 32;
const CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES: usize = 32;
const CRYPTO_SCALARMULT_BYTES: usize = CRYPTO_SCALARMULT_CURVE25519_BYTES;
const CRYPTO_SCALARMULT_SCALARBYTES: usize = CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES;
const CRYPTO_SCALARMULT_ED25519_BYTES: usize = 32;
const CRYPTO_SCALARMULT_ED25519_SCALARBYTES: usize = 32;
const CRYPTO_SCALARMULT_RISTRETTO255_BYTES: usize = 32;
const CRYPTO_SCALARMULT_RISTRETTO255_SCALARBYTES: usize = 32;
const CRYPTO_SCALARMULT_PRIMITIVE: &[u8] = b"curve25519\0";

extern "C" {
    #[link_name = "crypto_scalarmult_curve25519_ref10_implementation"]
    static CRYPTO_SCALARMULT_CURVE25519_REF10_IMPLEMENTATION:
        crypto_scalarmult_curve25519_implementation;

    #[link_name = "_sodium_ge25519_scalarmult"]
    fn ge25519_scalarmult(h: *mut ge25519_p3, a: *const u8, p: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_scalarmult_base"]
    fn ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    #[link_name = "_sodium_ge25519_frombytes"]
    fn ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_p3_tobytes"]
    fn ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_has_small_order"]
    fn ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_is_canonical"]
    fn ge25519_is_canonical(s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_is_on_main_subgroup"]
    fn ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ristretto255_frombytes"]
    fn ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ristretto255_p3_tobytes"]
    fn ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3);

    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

// =====================================================================
// crypto_scalarmult/curve25519/scalarmult_curve25519.c
// =====================================================================

static mut IMPLEMENTATION: *const crypto_scalarmult_curve25519_implementation =
    unsafe { &CRYPTO_SCALARMULT_CURVE25519_REF10_IMPLEMENTATION };

/// `int crypto_scalarmult_curve25519(unsigned char *q, const unsigned char *n,
/// const unsigned char *p)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_curve25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let mut d: u8 = 0;
    let mut i: usize;

    if ((*IMPLEMENTATION).mult)(q, n, p) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    i = 0;
    while i < CRYPTO_SCALARMULT_CURVE25519_BYTES {
        d |= *q.add(i);
        i += 1;
    }
    -((1u32 & (((d as u32).wrapping_sub(1)) >> 8)) as c_int)
}

/// `int crypto_scalarmult_curve25519_base(unsigned char *q, const unsigned
/// char *n)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int {
    (CRYPTO_SCALARMULT_CURVE25519_REF10_IMPLEMENTATION.mult_base)(q, n)
}

/// `size_t crypto_scalarmult_curve25519_bytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_bytes() -> usize {
    CRYPTO_SCALARMULT_CURVE25519_BYTES
}

/// `size_t crypto_scalarmult_curve25519_scalarbytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_scalarbytes() -> usize {
    CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES
}

/// `int _crypto_scalarmult_curve25519_pick_best_implementation(void)`
#[no_mangle]
pub unsafe extern "C" fn _crypto_scalarmult_curve25519_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &CRYPTO_SCALARMULT_CURVE25519_REF10_IMPLEMENTATION;

    /* HAVE_AVX_ASM is not defined in the reference build: the sandy2x
     * implementation is never selected. */

    0
}

// =====================================================================
// crypto_scalarmult/crypto_scalarmult.c
// =====================================================================

/// `const char *crypto_scalarmult_primitive(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_primitive() -> *const core::ffi::c_char {
    CRYPTO_SCALARMULT_PRIMITIVE.as_ptr() as *const core::ffi::c_char
}

/// `int crypto_scalarmult_base(unsigned char *q, const unsigned char *n)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int {
    crypto_scalarmult_curve25519_base(q, n)
}

/// `int crypto_scalarmult(unsigned char *q, const unsigned char *n, const
/// unsigned char *p)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    crypto_scalarmult_curve25519(q, n, p)
}

/// `size_t crypto_scalarmult_bytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_bytes() -> usize {
    CRYPTO_SCALARMULT_BYTES
}

/// `size_t crypto_scalarmult_scalarbytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_scalarbytes() -> usize {
    CRYPTO_SCALARMULT_SCALARBYTES
}

// =====================================================================
// crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c
// =====================================================================

/// `static int _crypto_scalarmult_ed25519_is_inf(const unsigned char s[32])`
unsafe fn crypto_scalarmult_ed25519_is_inf(s: *const u8) -> c_int {
    let mut c: u32;
    let mut i: usize;

    c = (*s ^ 0x01) as u32;
    i = 1;
    while i < 31 {
        c |= *s.add(i) as u32;
        i += 1;
    }
    c |= (*s.add(31) & 0x7f) as u32;

    ((c.wrapping_sub(1)) >> 8 & 1) as c_int
}

/// `static inline void _crypto_scalarmult_ed25519_clamp(unsigned char k[32])`
#[inline]
unsafe fn crypto_scalarmult_ed25519_clamp(k: *mut u8) {
    *k &= 248;
    *k.add(31) |= 64;
}

/// `static int _crypto_scalarmult_ed25519(unsigned char *q, const unsigned
/// char *n, const unsigned char *p, const int clamp)`
unsafe fn crypto_scalarmult_ed25519_impl(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
    clamp: c_int,
) -> c_int {
    let t = q;
    let mut qq: ge25519_p3 = core::mem::zeroed();
    let mut pp: ge25519_p3 = core::mem::zeroed();
    let mut i: usize;

    if ge25519_is_canonical(p) == 0
        || ge25519_frombytes(&mut pp, p) != 0
        || ge25519_has_small_order(&pp) != 0
        || ge25519_is_on_main_subgroup(&pp) == 0
    {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i) = *n.add(i);
        i += 1;
    }
    if clamp != 0 {
        crypto_scalarmult_ed25519_clamp(t);
    }
    *t.add(31) &= 127;

    ge25519_scalarmult(&mut qq, t, &pp);
    ge25519_p3_tobytes(q, &qq);
    if crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

/// `int crypto_scalarmult_ed25519(unsigned char *q, const unsigned char *n,
/// const unsigned char *p)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ed25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    crypto_scalarmult_ed25519_impl(q, n, p, 1)
}

/// `int crypto_scalarmult_ed25519_noclamp(unsigned char *q, const unsigned
/// char *n, const unsigned char *p)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_noclamp(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    crypto_scalarmult_ed25519_impl(q, n, p, 0)
}

/// `static int _crypto_scalarmult_ed25519_base(unsigned char *q, const
/// unsigned char *n, const int clamp)`
unsafe fn crypto_scalarmult_ed25519_base_impl(q: *mut u8, n: *const u8, clamp: c_int) -> c_int {
    let t = q;
    let mut qq: ge25519_p3 = core::mem::zeroed();
    let mut i: usize;

    i = 0;
    while i < 32 {
        *t.add(i) = *n.add(i);
        i += 1;
    }
    if clamp != 0 {
        crypto_scalarmult_ed25519_clamp(t);
    }
    *t.add(31) &= 127;

    ge25519_scalarmult_base(&mut qq, t);
    ge25519_p3_tobytes(q, &qq);
    if crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

/// `int crypto_scalarmult_ed25519_base(unsigned char *q, const unsigned char
/// *n)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base(q: *mut u8, n: *const u8) -> c_int {
    crypto_scalarmult_ed25519_base_impl(q, n, 1)
}

/// `int crypto_scalarmult_ed25519_base_noclamp(unsigned char *q, const
/// unsigned char *n)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base_noclamp(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    crypto_scalarmult_ed25519_base_impl(q, n, 0)
}

/// `size_t crypto_scalarmult_ed25519_bytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_bytes() -> usize {
    CRYPTO_SCALARMULT_ED25519_BYTES
}

/// `size_t crypto_scalarmult_ed25519_scalarbytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_scalarbytes() -> usize {
    CRYPTO_SCALARMULT_ED25519_SCALARBYTES
}

// =====================================================================
// crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c
// =====================================================================

/// `int crypto_scalarmult_ristretto255(unsigned char *q, const unsigned char
/// *n, const unsigned char *p)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let t = q;
    let mut qq: ge25519_p3 = core::mem::zeroed();
    let mut pp: ge25519_p3 = core::mem::zeroed();
    let mut i: usize;

    if ristretto255_frombytes(&mut pp, p) != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i) = *n.add(i);
        i += 1;
    }
    *t.add(31) &= 127;
    ge25519_scalarmult(&mut qq, t, &pp);
    ristretto255_p3_tobytes(q, &qq);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

/// `int crypto_scalarmult_ristretto255_base(unsigned char *q, const unsigned
/// char *n)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_base(q: *mut u8, n: *const u8) -> c_int {
    let t = q;
    let mut qq: ge25519_p3 = core::mem::zeroed();
    let mut i: usize;

    i = 0;
    while i < 32 {
        *t.add(i) = *n.add(i);
        i += 1;
    }
    *t.add(31) &= 127;
    ge25519_scalarmult_base(&mut qq, t);
    ristretto255_p3_tobytes(q, &qq);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

/// `size_t crypto_scalarmult_ristretto255_bytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_bytes() -> usize {
    CRYPTO_SCALARMULT_RISTRETTO255_BYTES
}

/// `size_t crypto_scalarmult_ristretto255_scalarbytes(void)`
#[no_mangle]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_scalarbytes() -> usize {
    CRYPTO_SCALARMULT_RISTRETTO255_SCALARBYTES
}

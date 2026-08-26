//! Translation of
//! `crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c`.
//!
//! This is the only scrypt backend compiled in the reference build
//! (`HAVE_EMMINTRIN_H` undefined makes the `sse/` translation unit empty).
//!
//! Target is x86-64 Linux, so `SIZE_MAX > UINT32_MAX` and
//! `SIZE_MAX / 256 > UINT32_MAX`: the `buflen` sanity check *is* compiled and
//! the `(r > SIZE_MAX / 256)` clause is *not*.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_int;

use crate::common::{EINVAL, ENOMEM, SIZE_MAX, load32_le, set_errno, store32_le};
use crate::crypto_pwhash::scrypt::common::escrypt_local_t;
use crate::crypto_pwhash::scrypt::pbkdf2_sha256::_sodium_escrypt_PBKDF2_SHA256;
use crate::crypto_pwhash::scrypt::platform::{
    _sodium_escrypt_alloc_region, _sodium_escrypt_free_region,
};

/// `<errno.h>`: `EFBIG` on Linux.
const EFBIG: c_int = 27;

/// ```c
/// static inline void
/// blkcpy(uint32_t *dest, const uint32_t *src, size_t len)
/// {
///     memcpy(dest, src, len * 64);
/// }
/// ```
#[inline(always)]
unsafe fn blkcpy(dest: *mut u32, src: *const u32, len: usize) {
    let n = len.wrapping_mul(64);
    if n != 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, n);
    }
}

/// ```c
/// static inline void
/// blkxor(uint32_t *dest, const uint32_t *src, size_t len)
/// ```
#[inline(always)]
unsafe fn blkxor(dest: *mut u32, src: *const u32, len: usize) {
    let mut i: usize = 0;
    let n = len.wrapping_mul(16);
    while i < n {
        *dest.add(i) ^= *src.add(i);
        i += 1;
    }
}

/// `#define R(a, b) (((a) << (b)) | ((a) >> (32 - (b))))`
#[inline(always)]
const fn R(a: u32, b: u32) -> u32 {
    (a << b) | (a >> (32 - b))
}

/// `salsa20_8(B)`: apply the salsa20/8 core to the provided block.
///
/// ```c
/// static void
/// salsa20_8(uint32_t B[16])
/// ```
unsafe fn salsa20_8(B: *mut u32) {
    let mut x = [0u32; 16];
    let mut i: usize;

    blkcpy(x.as_mut_ptr(), B, 1);
    i = 0;
    while i < 8 {
        /* Operate on columns. */
        x[4] ^= R(x[0].wrapping_add(x[12]), 7);
        x[8] ^= R(x[4].wrapping_add(x[0]), 9);
        x[12] ^= R(x[8].wrapping_add(x[4]), 13);
        x[0] ^= R(x[12].wrapping_add(x[8]), 18);

        x[9] ^= R(x[5].wrapping_add(x[1]), 7);
        x[13] ^= R(x[9].wrapping_add(x[5]), 9);
        x[1] ^= R(x[13].wrapping_add(x[9]), 13);
        x[5] ^= R(x[1].wrapping_add(x[13]), 18);

        x[14] ^= R(x[10].wrapping_add(x[6]), 7);
        x[2] ^= R(x[14].wrapping_add(x[10]), 9);
        x[6] ^= R(x[2].wrapping_add(x[14]), 13);
        x[10] ^= R(x[6].wrapping_add(x[2]), 18);

        x[3] ^= R(x[15].wrapping_add(x[11]), 7);
        x[7] ^= R(x[3].wrapping_add(x[15]), 9);
        x[11] ^= R(x[7].wrapping_add(x[3]), 13);
        x[15] ^= R(x[11].wrapping_add(x[7]), 18);

        /* Operate on rows. */
        x[1] ^= R(x[0].wrapping_add(x[3]), 7);
        x[2] ^= R(x[1].wrapping_add(x[0]), 9);
        x[3] ^= R(x[2].wrapping_add(x[1]), 13);
        x[0] ^= R(x[3].wrapping_add(x[2]), 18);

        x[6] ^= R(x[5].wrapping_add(x[4]), 7);
        x[7] ^= R(x[6].wrapping_add(x[5]), 9);
        x[4] ^= R(x[7].wrapping_add(x[6]), 13);
        x[5] ^= R(x[4].wrapping_add(x[7]), 18);

        x[11] ^= R(x[10].wrapping_add(x[9]), 7);
        x[8] ^= R(x[11].wrapping_add(x[10]), 9);
        x[9] ^= R(x[8].wrapping_add(x[11]), 13);
        x[10] ^= R(x[9].wrapping_add(x[8]), 18);

        x[12] ^= R(x[15].wrapping_add(x[14]), 7);
        x[13] ^= R(x[12].wrapping_add(x[15]), 9);
        x[14] ^= R(x[13].wrapping_add(x[12]), 13);
        x[15] ^= R(x[14].wrapping_add(x[13]), 18);

        i += 2;
    }
    i = 0;
    while i < 16 {
        *B.add(i) = (*B.add(i)).wrapping_add(x[i]);
        i += 1;
    }
}

/// `blockmix_salsa8(Bin, Bout, X, r)`: compute
/// `Bout = BlockMix_{salsa20/8, r}(Bin)`.
///
/// ```c
/// static void
/// blockmix_salsa8(const uint32_t *Bin, uint32_t *Bout, uint32_t *X, size_t r)
/// ```
unsafe fn blockmix_salsa8(Bin: *const u32, Bout: *mut u32, X: *mut u32, r: usize) {
    let mut i: usize;

    /* 1: X <-- B_{2r - 1} */
    blkcpy(X, Bin.add((2usize.wrapping_mul(r).wrapping_sub(1)).wrapping_mul(16)), 1);

    /* 2: for i = 0 to 2r - 1 do */
    i = 0;
    while i < 2usize.wrapping_mul(r) {
        /* 3: X <-- H(X \xor B_i) */
        blkxor(X, Bin.add(i.wrapping_mul(16)), 1);
        salsa20_8(X);

        /* 4: Y_i <-- X */
        /* 6: B' <-- (Y_0, Y_2 ... Y_{2r-2}, Y_1, Y_3 ... Y_{2r-1}) */
        blkcpy(Bout.add(i.wrapping_mul(8)), X, 1);

        /* 3: X <-- H(X \xor B_i) */
        blkxor(X, Bin.add(i.wrapping_mul(16).wrapping_add(16)), 1);
        salsa20_8(X);

        /* 4: Y_i <-- X */
        /* 6: B' <-- (Y_0, Y_2 ... Y_{2r-2}, Y_1, Y_3 ... Y_{2r-1}) */
        blkcpy(
            Bout.add(i.wrapping_mul(8).wrapping_add(r.wrapping_mul(16))),
            X,
            1,
        );

        i += 2;
    }
}

/// `integerify(B, r)`: return the result of parsing `B_{2r-1}` as a
/// little-endian integer.
///
/// ```c
/// static inline uint64_t
/// integerify(const uint32_t *B, size_t r)
/// ```
#[inline(always)]
unsafe fn integerify(B: *const u32, r: usize) -> u64 {
    let X: *const u32 = B.add((2usize.wrapping_mul(r).wrapping_sub(1)).wrapping_mul(16));

    (((*X.add(1)) as u64) << 32).wrapping_add((*X.add(0)) as u64)
}

/// `smix(B, r, N, V, XY)`: compute `B = SMix_r(B, N)`.
///
/// ```c
/// static void
/// smix(uint8_t *B, size_t r, uint64_t N, uint32_t *V, uint32_t *XY)
/// ```
unsafe fn smix(B: *mut u8, r: usize, N: u64, V: *mut u32, XY: *mut u32) {
    let X: *mut u32 = XY;
    let Y: *mut u32 = XY.add(32usize.wrapping_mul(r));
    let Z: *mut u32 = XY.add(64usize.wrapping_mul(r));
    let mut i: u64;
    let mut j: u64;
    let mut k: usize;

    /* 1: X <-- B */
    k = 0;
    while k < 32usize.wrapping_mul(r) {
        *X.add(k) = load32_le(B.add(4usize.wrapping_mul(k)));
        k += 1;
    }
    /* 2: for i = 0 to N - 1 do */
    i = 0;
    while i < N {
        /* 3: V_i <-- X */
        blkcpy(
            V.add((i.wrapping_mul(32u64.wrapping_mul(r as u64))) as usize),
            X,
            2usize.wrapping_mul(r),
        );

        /* 4: X <-- H(X) */
        blockmix_salsa8(X, Y, Z, r);

        /* 3: V_i <-- X */
        blkcpy(
            V.add((i.wrapping_add(1).wrapping_mul(32u64.wrapping_mul(r as u64))) as usize),
            Y,
            2usize.wrapping_mul(r),
        );

        /* 4: X <-- H(X) */
        blockmix_salsa8(Y, X, Z, r);

        i = i.wrapping_add(2);
    }

    /* 6: for i = 0 to N - 1 do */
    i = 0;
    while i < N {
        /* 7: j <-- Integerify(X) mod N */
        j = integerify(X, r) & N.wrapping_sub(1);

        /* 8: X <-- H(X \xor V_j) */
        blkxor(
            X,
            V.add((j.wrapping_mul(32u64.wrapping_mul(r as u64))) as usize),
            2usize.wrapping_mul(r),
        );
        blockmix_salsa8(X, Y, Z, r);

        /* 7: j <-- Integerify(X) mod N */
        j = integerify(Y, r) & N.wrapping_sub(1);

        /* 8: X <-- H(X \xor V_j) */
        blkxor(
            Y,
            V.add((j.wrapping_mul(32u64.wrapping_mul(r as u64))) as usize),
            2usize.wrapping_mul(r),
        );
        blockmix_salsa8(Y, X, Z, r);

        i = i.wrapping_add(2);
    }
    /* 10: B' <-- X */
    k = 0;
    while k < 32usize.wrapping_mul(r) {
        store32_le(B.add(4usize.wrapping_mul(k)), *X.add(k));
        k += 1;
    }
}

/// `escrypt_kdf(local, passwd, passwdlen, salt, saltlen, N, r, p, buf, buflen)`:
/// compute `scrypt(passwd[0 .. passwdlen - 1], salt[0 .. saltlen - 1], N, r, p,
/// buflen)` and write the result into `buf`.
///
/// Return 0 on success; or -1 on error.
///
/// ```c
/// int
/// escrypt_kdf_nosse(escrypt_local_t *local, const uint8_t *passwd,
///                   size_t passwdlen, const uint8_t *salt, size_t saltlen,
///                   uint64_t N, uint32_t _r, uint32_t _p, uint8_t *buf,
///                   size_t buflen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_kdf_nosse(
    local: *mut escrypt_local_t,
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    N: u64,
    _r: u32,
    _p: u32,
    buf: *mut u8,
    buflen: usize,
) -> c_int {
    let B_size: usize;
    let V_size: usize;
    let XY_size: usize;
    let mut need: usize;
    let B: *mut u8;
    let V: *mut u32;
    let XY: *mut u32;
    let r: usize = _r as usize;
    let p: usize = _p as usize;
    let mut i: u32;

    /* Sanity-check parameters. */
    // #if SIZE_MAX > UINT32_MAX
    if (buflen as u64) > (((1u64) << 32) - 1).wrapping_mul(32) {
        set_errno(EFBIG);
        return -1;
    }
    if (r as u64).wrapping_mul(p as u64) >= ((1u64) << 30) {
        set_errno(EFBIG);
        return -1;
    }
    if N > u32::MAX as u64 {
        set_errno(EFBIG);
        return -1;
    }
    if ((N & N.wrapping_sub(1)) != 0) || (N < 2) {
        set_errno(EINVAL);
        return -1;
    }
    if r == 0 || p == 0 {
        set_errno(EINVAL);
        return -1;
    }
    // #if SIZE_MAX / 256 <= UINT32_MAX  -> false on x86-64, clause omitted
    if (r > SIZE_MAX / 128 / p) || (N > (SIZE_MAX / 128 / r) as u64) {
        set_errno(ENOMEM);
        return -1;
    }

    /* Allocate memory. */
    B_size = 128usize.wrapping_mul(r).wrapping_mul(p);
    V_size = 128usize.wrapping_mul(r).wrapping_mul(N as usize);
    need = B_size.wrapping_add(V_size);
    if need < V_size {
        set_errno(ENOMEM);
        return -1;
    }
    XY_size = 256usize.wrapping_mul(r).wrapping_add(64);
    need = need.wrapping_add(XY_size);
    if need < XY_size {
        set_errno(ENOMEM);
        return -1;
    }
    if (*local).size < need {
        if _sodium_escrypt_free_region(local) != 0 {
            return -1;
        }
        if _sodium_escrypt_alloc_region(local, need).is_null() {
            return -1;
        }
    }
    B = (*local).aligned as *mut u8;
    V = B.add(B_size) as *mut u32;
    XY = (V as *mut u8).add(V_size) as *mut u32;

    /* 1: (B_0 ... B_{p-1}) <-- PBKDF2(P, S, 1, p * MFLen) */
    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, 1, B, B_size);

    /* 2: for i = 0 to p - 1 do */
    i = 0;
    while (i as usize) < p {
        /* 3: B_i <-- MF(B_i, N) */
        smix(
            B.add(128usize.wrapping_mul(i as usize).wrapping_mul(r)),
            r,
            N,
            V,
            XY,
        );
        i = i.wrapping_add(1);
    }

    /* 5: DK <-- PBKDF2(P, B, 1, dkLen) */
    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, B, B_size, 1, buf, buflen);

    /* Success! */
    0
}

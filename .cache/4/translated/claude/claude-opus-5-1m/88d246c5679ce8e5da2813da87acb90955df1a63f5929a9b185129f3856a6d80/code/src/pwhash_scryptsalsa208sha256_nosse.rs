//! Translation of
//! `crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c`.
//!
//! `private/quirks.h` renames `escrypt_kdf_nosse` to
//! `_sodium_escrypt_kdf_nosse`.

use crate::common::{load32_le, memcpy, rotl32, store32_le};
use core::ffi::{c_int, c_void};

/* crypto_scrypt.h */
#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

/* <errno.h> */
const EINVAL: c_int = 22;
const ENOMEM: c_int = 12;
const EFBIG: c_int = 27;

extern "C" {
    fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int;
    fn _sodium_escrypt_alloc_region(region: *mut escrypt_region_t, size: usize) -> *mut c_void;
    fn _sodium_escrypt_PBKDF2_SHA256(
        passwd: *const u8,
        passwdlen: usize,
        salt: *const u8,
        saltlen: usize,
        c: u64,
        buf: *mut u8,
        dkLen: usize,
    );
    fn __errno_location() -> *mut c_int;
}

/* static inline void blkcpy(uint32_t *dest, const uint32_t *src, size_t len) */
#[inline(always)]
unsafe fn blkcpy(dest: *mut u32, src: *const u32, len: usize) {
    memcpy(dest as *mut u8, src as *const u8, len.wrapping_mul(64));
}

/* static inline void blkxor(uint32_t *dest, const uint32_t *src, size_t len) */
#[inline(always)]
unsafe fn blkxor(dest: *mut u32, src: *const u32, len: usize) {
    let n = len.wrapping_mul(16);
    let mut i: usize = 0;
    while i < n {
        *dest.add(i) ^= *src.add(i);
        i = i.wrapping_add(1);
    }
}

/*
 * salsa20_8(B):
 * Apply the salsa20/8 core to the provided block.
 */
unsafe fn salsa20_8(B: *mut u32) {
    let mut x: [u32; 16] = [0; 16];
    let mut i: usize;

    blkcpy(x.as_mut_ptr(), B, 1);
    i = 0;
    while i < 8 {
        /* #define R(a, b) (((a) << (b)) | ((a) >> (32 - (b)))) */
        /* Operate on columns. */
        x[4] ^= rotl32(x[0].wrapping_add(x[12]), 7);
        x[8] ^= rotl32(x[4].wrapping_add(x[0]), 9);
        x[12] ^= rotl32(x[8].wrapping_add(x[4]), 13);
        x[0] ^= rotl32(x[12].wrapping_add(x[8]), 18);

        x[9] ^= rotl32(x[5].wrapping_add(x[1]), 7);
        x[13] ^= rotl32(x[9].wrapping_add(x[5]), 9);
        x[1] ^= rotl32(x[13].wrapping_add(x[9]), 13);
        x[5] ^= rotl32(x[1].wrapping_add(x[13]), 18);

        x[14] ^= rotl32(x[10].wrapping_add(x[6]), 7);
        x[2] ^= rotl32(x[14].wrapping_add(x[10]), 9);
        x[6] ^= rotl32(x[2].wrapping_add(x[14]), 13);
        x[10] ^= rotl32(x[6].wrapping_add(x[2]), 18);

        x[3] ^= rotl32(x[15].wrapping_add(x[11]), 7);
        x[7] ^= rotl32(x[3].wrapping_add(x[15]), 9);
        x[11] ^= rotl32(x[7].wrapping_add(x[3]), 13);
        x[15] ^= rotl32(x[11].wrapping_add(x[7]), 18);

        /* Operate on rows. */
        x[1] ^= rotl32(x[0].wrapping_add(x[3]), 7);
        x[2] ^= rotl32(x[1].wrapping_add(x[0]), 9);
        x[3] ^= rotl32(x[2].wrapping_add(x[1]), 13);
        x[0] ^= rotl32(x[3].wrapping_add(x[2]), 18);

        x[6] ^= rotl32(x[5].wrapping_add(x[4]), 7);
        x[7] ^= rotl32(x[6].wrapping_add(x[5]), 9);
        x[4] ^= rotl32(x[7].wrapping_add(x[6]), 13);
        x[5] ^= rotl32(x[4].wrapping_add(x[7]), 18);

        x[11] ^= rotl32(x[10].wrapping_add(x[9]), 7);
        x[8] ^= rotl32(x[11].wrapping_add(x[10]), 9);
        x[9] ^= rotl32(x[8].wrapping_add(x[11]), 13);
        x[10] ^= rotl32(x[9].wrapping_add(x[8]), 18);

        x[12] ^= rotl32(x[15].wrapping_add(x[14]), 7);
        x[13] ^= rotl32(x[12].wrapping_add(x[15]), 9);
        x[14] ^= rotl32(x[13].wrapping_add(x[12]), 13);
        x[15] ^= rotl32(x[14].wrapping_add(x[13]), 18);

        i = i.wrapping_add(2);
    }
    i = 0;
    while i < 16 {
        *B.add(i) = (*B.add(i)).wrapping_add(x[i]);
        i = i.wrapping_add(1);
    }
}

/*
 * blockmix_salsa8(Bin, Bout, X, r):
 * Compute Bout = BlockMix_{salsa20/8, r}(Bin).
 * The input Bin must be 128r bytes in length;
 * The output Bout must also be the same size.
 * The temporary space X must be 64 bytes.
 */
unsafe fn blockmix_salsa8(Bin: *const u32, Bout: *mut u32, X: *mut u32, r: usize) {
    let mut i: usize;

    /* 1: X <-- B_{2r - 1} */
    blkcpy(
        X,
        Bin.add((2usize.wrapping_mul(r).wrapping_sub(1)).wrapping_mul(16)),
        1,
    );

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

        i = i.wrapping_add(2);
    }
}

/*
 * integerify(B, r):
 * Return the result of parsing B_{2r-1} as a little-endian integer.
 */
#[inline(always)]
unsafe fn integerify(B: *const u32, r: usize) -> u64 {
    let X: *const u32 = B.add((2usize.wrapping_mul(r).wrapping_sub(1)).wrapping_mul(16));

    ((*X.add(1) as u64) << 32).wrapping_add(*X.add(0) as u64)
}

/*
 * smix(B, r, N, V, XY):
 * Compute B = SMix_r(B, N).  The input B must be 128r bytes in length;
 * the temporary storage V must be 128rN bytes in length; the temporary
 * storage XY must be 256r + 64 bytes in length.  The value N must be a
 * power of 2 greater than 1.  The arrays B, V, and XY must be aligned to a
 * multiple of 64 bytes.
 */
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
        k = k.wrapping_add(1);
    }
    /* 2: for i = 0 to N - 1 do */
    i = 0;
    while i < N {
        /* 3: V_i <-- X */
        blkcpy(
            V.add((i as usize).wrapping_mul(32usize.wrapping_mul(r))),
            X,
            2usize.wrapping_mul(r),
        );

        /* 4: X <-- H(X) */
        blockmix_salsa8(X, Y, Z, r);

        /* 3: V_i <-- X */
        blkcpy(
            V.add(
                (i.wrapping_add(1) as usize).wrapping_mul(32usize.wrapping_mul(r)),
            ),
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
            V.add((j as usize).wrapping_mul(32usize.wrapping_mul(r))),
            2usize.wrapping_mul(r),
        );
        blockmix_salsa8(X, Y, Z, r);

        /* 7: j <-- Integerify(X) mod N */
        j = integerify(Y, r) & N.wrapping_sub(1);

        /* 8: X <-- H(X \xor V_j) */
        blkxor(
            Y,
            V.add((j as usize).wrapping_mul(32usize.wrapping_mul(r))),
            2usize.wrapping_mul(r),
        );
        blockmix_salsa8(Y, X, Z, r);

        i = i.wrapping_add(2);
    }
    /* 10: B' <-- X */
    k = 0;
    while k < 32usize.wrapping_mul(r) {
        store32_le(B.add(4usize.wrapping_mul(k)), *X.add(k));
        k = k.wrapping_add(1);
    }
}

/*
 * escrypt_kdf(local, passwd, passwdlen, salt, saltlen,
 *     N, r, p, buf, buflen):
 * Compute scrypt(passwd[0 .. passwdlen - 1], salt[0 .. saltlen - 1], N, r,
 * p, buflen) and write the result into buf.  The parameters r, p, and buflen
 * must satisfy r * p < 2^30 and buflen <= (2^32 - 1) * 32.  The parameter N
 * must be a power of 2 greater than 1.
 *
 * Return 0 on success; or -1 on error.
 */
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
    /* #if SIZE_MAX > UINT32_MAX -- true on x86-64 */
    if buflen as u64 > (((1u64) << 32).wrapping_sub(1)).wrapping_mul(32) {
        *__errno_location() = EFBIG;
        return -1;
    }
    if (r as u64).wrapping_mul(p as u64) >= (1u64 << 30) {
        *__errno_location() = EFBIG;
        return -1;
    }
    if N > u32::MAX as u64 {
        *__errno_location() = EFBIG;
        return -1;
    }
    if ((N & N.wrapping_sub(1)) != 0) || (N < 2) {
        *__errno_location() = EINVAL;
        return -1;
    }
    if r == 0 || p == 0 {
        *__errno_location() = EINVAL;
        return -1;
    }
    /* `(r > SIZE_MAX / 256)` is compiled out: SIZE_MAX / 256 > UINT32_MAX. */
    if (r > usize::MAX / 128 / p) || (N > (usize::MAX / 128 / r) as u64) {
        *__errno_location() = ENOMEM;
        return -1;
    }

    /* Allocate memory. */
    B_size = 128usize.wrapping_mul(r).wrapping_mul(p);
    V_size = 128usize.wrapping_mul(r).wrapping_mul(N as usize);
    need = B_size.wrapping_add(V_size);
    if need < V_size {
        *__errno_location() = ENOMEM;
        return -1;
    }
    XY_size = 256usize.wrapping_mul(r).wrapping_add(64);
    need = need.wrapping_add(XY_size);
    if need < XY_size {
        *__errno_location() = ENOMEM;
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

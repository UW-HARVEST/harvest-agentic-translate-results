//! Translation of c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c

use core::ffi::{c_int, c_void};

use crate::common::{load32_le, rotl32, store32_le};

// EFBIG is not provided by crate::plat; on x86_64 Linux it is 27.
const EFBIG: c_int = 27;

// escrypt_local_t mirror (crypto_scrypt.h).
#[repr(C)]
struct escrypt_region_t {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}
type escrypt_local_t = escrypt_region_t;

extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    // escrypt_alloc_region / escrypt_free_region from scrypt_platform.c
    // (linker names after quirks.h).
    fn _sodium_escrypt_alloc_region(region: *mut escrypt_region_t, size: usize) -> *mut c_void;
    fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int;

    // escrypt_PBKDF2_SHA256 from pbkdf2-sha256.c (linker name after quirks.h).
    fn _sodium_escrypt_PBKDF2_SHA256(
        passwd: *const u8,
        passwdlen: usize,
        salt: *const u8,
        saltlen: usize,
        c: u64,
        buf: *mut u8,
        dkLen: usize,
    );
}

#[inline]
unsafe fn blkcpy(dest: *mut u32, src: *const u32, len: usize) {
    memcpy(dest as *mut c_void, src as *const c_void, len * 64);
}

#[inline]
unsafe fn blkxor(dest: *mut u32, src: *const u32, len: usize) {
    let mut i: usize = 0;

    while i < len * 16 {
        *dest.add(i) ^= *src.add(i);
        i += 1;
    }
}

/// salsa20_8(B): Apply the salsa20/8 core to the provided block.
///
/// The C `R(a, b)` macro is `((a) << (b)) | ((a) >> (32 - (b)))`, i.e. a 32-bit
/// left-rotate; all additions wrap modulo 2^32.
unsafe fn salsa20_8(B: *mut u32) {
    let mut x: [u32; 16] = [0; 16];
    let mut i: usize;

    blkcpy(x.as_mut_ptr(), B as *const u32, 1);
    i = 0;
    while i < 8 {
        // Operate on columns.
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

        // Operate on rows.
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

        i += 2;
    }
    i = 0;
    while i < 16 {
        *B.add(i) = (*B.add(i)).wrapping_add(x[i]);
        i += 1;
    }
}

/// blockmix_salsa8(Bin, Bout, X, r):
/// Compute Bout = BlockMix_{salsa20/8, r}(Bin).
unsafe fn blockmix_salsa8(Bin: *const u32, Bout: *mut u32, X: *mut u32, r: usize) {
    let mut i: usize;

    /* 1: X <-- B_{2r - 1} */
    blkcpy(X, Bin.add((2 * r - 1) * 16), 1);

    /* 2: for i = 0 to 2r - 1 do */
    i = 0;
    while i < 2 * r {
        /* 3: X <-- H(X \xor B_i) */
        blkxor(X, Bin.add(i * 16), 1);
        salsa20_8(X);

        /* 4/6 */
        blkcpy(Bout.add(i * 8), X, 1);

        /* 3: X <-- H(X \xor B_i) */
        blkxor(X, Bin.add(i * 16 + 16), 1);
        salsa20_8(X);

        /* 4/6 */
        blkcpy(Bout.add(i * 8 + r * 16), X, 1);

        i += 2;
    }
}

/// integerify(B, r): parse B_{2r-1} as a little-endian integer.
#[inline]
unsafe fn integerify(B: *const u32, r: usize) -> u64 {
    let X: *const u32 = B.add((2 * r - 1) * 16);

    ((*X.add(1) as u64) << 32) + (*X.add(0) as u64)
}

/// smix(B, r, N, V, XY): Compute B = SMix_r(B, N).
unsafe fn smix(B: *mut u8, r: usize, N: u64, V: *mut u32, XY: *mut u32) {
    let X: *mut u32 = XY;
    let Y: *mut u32 = XY.add(32 * r);
    let Z: *mut u32 = XY.add(64 * r);
    let mut i: u64;
    let mut j: u64;
    let mut k: usize;

    /* 1: X <-- B */
    k = 0;
    while k < 32 * r {
        *X.add(k) = load32_le(B.add(4 * k));
        k += 1;
    }
    /* 2: for i = 0 to N - 1 do */
    i = 0;
    while i < N {
        /* 3: V_i <-- X */
        blkcpy(V.add((i as usize) * (32 * r)), X, 2 * r);

        /* 4: X <-- H(X) */
        blockmix_salsa8(X, Y, Z, r);

        /* 3: V_i <-- X */
        blkcpy(V.add(((i + 1) as usize) * (32 * r)), Y, 2 * r);

        /* 4: X <-- H(X) */
        blockmix_salsa8(Y, X, Z, r);

        i += 2;
    }

    /* 6: for i = 0 to N - 1 do */
    i = 0;
    while i < N {
        /* 7: j <-- Integerify(X) mod N */
        j = integerify(X, r) & (N - 1);

        /* 8: X <-- H(X \xor V_j) */
        blkxor(X, V.add((j as usize) * (32 * r)), 2 * r);
        blockmix_salsa8(X, Y, Z, r);

        /* 7: j <-- Integerify(Y) mod N */
        j = integerify(Y, r) & (N - 1);

        /* 8: X <-- H(X \xor V_j) */
        blkxor(Y, V.add((j as usize) * (32 * r)), 2 * r);
        blockmix_salsa8(Y, X, Z, r);

        i += 2;
    }
    /* 10: B' <-- X */
    k = 0;
    while k < 32 * r {
        store32_le(B.add(4 * k), *X.add(k));
        k += 1;
    }
}

/// escrypt_kdf_nosse(local, passwd, passwdlen, salt, saltlen, N, r, p, buf, buflen)
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

    // Sanity-check parameters.
    // SIZE_MAX > UINT32_MAX on x86_64: this bound check is compiled.
    if buflen as u64 > (((1u64) << 32) - 1) * 32 {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    if (r as u64) * (p as u64) >= ((1u64) << 30) {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    if N > u32::MAX as u64 {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    // C: `(N & (N - 1))` with N == 0 relies on unsigned wraparound.
    if ((N & N.wrapping_sub(1)) != 0) || (N < 2) {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    if r == 0 || p == 0 {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    // SIZE_MAX / 256 <= UINT32_MAX is false on x86_64: that clause is omitted.
    if (r > usize::MAX / 128 / p) || (N > (usize::MAX / 128 / r) as u64) {
        crate::plat::set_errno(crate::plat::ENOMEM);
        return -1;
    }

    /* Allocate memory. */
    B_size = 128usize * r * p;
    V_size = 128usize * r * (N as usize);
    need = B_size + V_size;
    if need < V_size {
        crate::plat::set_errno(crate::plat::ENOMEM);
        return -1;
    }
    XY_size = 256usize * r + 64;
    need += XY_size;
    if need < XY_size {
        crate::plat::set_errno(crate::plat::ENOMEM);
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
    V = (B.add(B_size)) as *mut u32;
    XY = ((V as *mut u8).add(V_size)) as *mut u32;

    /* 1: (B_0 ... B_{p-1}) <-- PBKDF2(P, S, 1, p * MFLen) */
    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, 1, B, B_size);

    /* 2: for i = 0 to p - 1 do */
    i = 0;
    while (i as usize) < p {
        /* 3: B_i <-- MF(B_i, N) */
        smix(B.add(128usize * (i as usize) * r), r, N, V, XY);
        i += 1;
    }

    /* 5: DK <-- PBKDF2(P, B, 1, dkLen) */
    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, B, B_size, 1, buf, buflen);

    /* Success! */
    0
}

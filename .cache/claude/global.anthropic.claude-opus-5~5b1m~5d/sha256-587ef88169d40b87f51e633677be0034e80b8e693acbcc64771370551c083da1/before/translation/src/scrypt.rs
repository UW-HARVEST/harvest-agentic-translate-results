//! Translation of:
//! - `crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c`
//! - `crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c`
//! - `crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c`
//! - `crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c`
//!
//! The reference build defines neither `HAVE_MMAP` nor `HAVE_POSIX_MEMALIGN`,
//! so `escrypt_alloc_region` always uses the plain `malloc` + manual
//! alignment fallback. The `sse/` variant of the kdf is never compiled, so
//! `escrypt_kdf_nosse` is called unconditionally wherever `escrypt_kdf` was
//! selected in the original code.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr::{null, null_mut};

use crate::common::{load32_le, rotl32, store32_be, store32_le};
use crate::csys::{free, malloc, memcpy, set_errno, strchr, ENOMEM};

const EFBIG: c_int = 27;
const EINVAL: c_int = 22;

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
}

// ---------------------------------------------------------------------
// crypto_hash_sha256_state / crypto_auth_hmacsha256_state, mirrored
// locally from crypto_hash_sha256.h / crypto_auth_hmacsha256.h. The real
// implementations live in other translation units; we only need the exact
// layout plus the final linker names of the functions we call.
// ---------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct crypto_auth_hmacsha256_state {
    ictx: crypto_hash_sha256_state,
    octx: crypto_hash_sha256_state,
}

extern "C" {
    fn crypto_auth_hmacsha256_init(
        state: *mut crypto_auth_hmacsha256_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha256_update(
        state: *mut crypto_auth_hmacsha256_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha256_final(
        state: *mut crypto_auth_hmacsha256_state,
        out: *mut u8,
    ) -> c_int;
}

// ---------------------------------------------------------------------
// crypto_scrypt.h
// ---------------------------------------------------------------------

#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

#[allow(dead_code)]
pub type escrypt_kdf_t = unsafe extern "C" fn(
    *mut escrypt_local_t,
    *const u8,
    usize,
    *const u8,
    usize,
    u64,
    u32,
    u32,
    *mut u8,
    usize,
) -> c_int;

// =======================================================================
// scrypt_platform.c
// =======================================================================

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_alloc_region(
    region: *mut escrypt_region_t,
    size: usize,
) -> *mut c_void {
    let mut base: *mut u8 = null_mut();
    let mut aligned: *mut u8 = null_mut();

    if size.wrapping_add(63) < size {
        set_errno(ENOMEM);
    } else {
        base = malloc(size.wrapping_add(63)) as *mut u8;
        if !base.is_null() {
            aligned = base.add(63);
            aligned = aligned.sub((aligned as usize) & 63);
        }
    }

    (*region).base = base as *mut c_void;
    (*region).aligned = aligned as *mut c_void;
    (*region).size = if !base.is_null() { size } else { 0 };

    aligned as *mut c_void
}

#[inline]
unsafe fn init_region(region: *mut escrypt_region_t) {
    (*region).base = null_mut();
    (*region).aligned = null_mut();
    (*region).size = 0;
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int {
    if !(*region).base.is_null() {
        free((*region).base as *mut c_void);
    }
    init_region(region);
    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int {
    init_region(local);
    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int {
    _sodium_escrypt_free_region(local)
}

// =======================================================================
// pbkdf2-sha256.c
// =======================================================================

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_PBKDF2_SHA256(
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    c: u64,
    buf: *mut u8,
    dk_len: usize,
) {
    let mut pshctx: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut hctx: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut ivec = [0u8; 4];
    let mut u = [0u8; 32];
    let mut t = [0u8; 32];

    if dk_len as u64 > 0x1fffffffe0u64 {
        sodium_misuse();
    }

    crypto_auth_hmacsha256_init(&mut pshctx, passwd, passwdlen);
    crypto_auth_hmacsha256_update(&mut pshctx, salt, saltlen as u64);

    let mut i: usize = 0;
    while i.wrapping_mul(32) < dk_len {
        store32_be(ivec.as_mut_ptr(), (i as u32).wrapping_add(1));
        core::ptr::copy_nonoverlapping(
            &pshctx as *const crypto_auth_hmacsha256_state as *const u8,
            &mut hctx as *mut crypto_auth_hmacsha256_state as *mut u8,
            core::mem::size_of::<crypto_auth_hmacsha256_state>(),
        );
        crypto_auth_hmacsha256_update(&mut hctx, ivec.as_ptr(), 4);
        crypto_auth_hmacsha256_final(&mut hctx, u.as_mut_ptr());

        t.copy_from_slice(&u);

        let mut j: u64 = 2;
        while j <= c {
            crypto_auth_hmacsha256_init(&mut hctx, passwd, passwdlen);
            crypto_auth_hmacsha256_update(&mut hctx, u.as_ptr(), 32);
            crypto_auth_hmacsha256_final(&mut hctx, u.as_mut_ptr());

            for k in 0..32usize {
                t[k] ^= u[k];
            }
            j += 1;
        }

        let mut clen = dk_len - i * 32;
        if clen > 32 {
            clen = 32;
        }
        memcpy(
            buf.add(i * 32) as *mut c_void,
            t.as_ptr() as *const c_void,
            clen,
        );

        i += 1;
    }

    sodium_memzero(
        &mut pshctx as *mut crypto_auth_hmacsha256_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );
}

// =======================================================================
// nosse/pwhash_scryptsalsa208sha256_nosse.c
// =======================================================================

#[inline]
unsafe fn blkcpy(dest: *mut u32, src: *const u32, len: usize) {
    memcpy(
        dest as *mut c_void,
        src as *const c_void,
        len.wrapping_mul(64),
    );
}

#[inline]
unsafe fn blkxor(dest: *mut u32, src: *const u32, len: usize) {
    for i in 0..len.wrapping_mul(16) {
        *dest.add(i) ^= *src.add(i);
    }
}

/// `salsa20_8(B)`: apply the salsa20/8 core to the provided block.
unsafe fn salsa20_8(b: *mut u32) {
    let mut x = [0u32; 16];

    blkcpy(x.as_mut_ptr(), b, 1);
    let mut i = 0usize;
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
    for k in 0..16usize {
        *b.add(k) = (*b.add(k)).wrapping_add(x[k]);
    }
}

/// `blockmix_salsa8(Bin, Bout, X, r)`: compute `Bout = BlockMix_{salsa20/8,r}(Bin)`.
unsafe fn blockmix_salsa8(bin: *const u32, bout: *mut u32, x: *mut u32, r: usize) {
    blkcpy(x, bin.add((2 * r - 1) * 16), 1);

    let mut i = 0usize;
    while i < 2 * r {
        blkxor(x, bin.add(i * 16), 1);
        salsa20_8(x);
        blkcpy(bout.add(i * 8), x, 1);

        blkxor(x, bin.add(i * 16 + 16), 1);
        salsa20_8(x);
        blkcpy(bout.add(i * 8 + r * 16), x, 1);

        i += 2;
    }
}

/// `integerify(B, r)`: parse `B_{2r-1}` as a little-endian integer.
#[inline]
unsafe fn integerify(b: *const u32, r: usize) -> u64 {
    let x = b.add((2 * r - 1) * 16);
    ((*x.add(1) as u64) << 32).wrapping_add(*x.add(0) as u64)
}

/// `smix(B, r, N, V, XY)`: compute `B = SMix_r(B, N)`.
unsafe fn smix(b: *mut u8, r: usize, n: u64, v: *mut u32, xy: *mut u32) {
    let x = xy;
    let y = xy.add(32 * r);
    let z = xy.add(64 * r);

    for k in 0..(32 * r) {
        *x.add(k) = load32_le(b.add(4 * k));
    }

    let mut i: u64 = 0;
    while i < n {
        blkcpy(v.add((i as usize) * (32 * r)), x, 2 * r);
        blockmix_salsa8(x, y, z, r);

        blkcpy(v.add(((i + 1) as usize) * (32 * r)), y, 2 * r);
        blockmix_salsa8(y, x, z, r);

        i += 2;
    }

    i = 0;
    while i < n {
        let j = integerify(x, r) & (n - 1);
        blkxor(x, v.add((j as usize) * (32 * r)), 2 * r);
        blockmix_salsa8(x, y, z, r);

        let j2 = integerify(y, r) & (n - 1);
        blkxor(y, v.add((j2 as usize) * (32 * r)), 2 * r);
        blockmix_salsa8(y, x, z, r);

        i += 2;
    }

    for k in 0..(32 * r) {
        store32_le(b.add(4 * k), *x.add(k));
    }
}

/// `escrypt_kdf_nosse(local, passwd, passwdlen, salt, saltlen, N, r, p, buf, buflen)`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_kdf_nosse(
    local: *mut escrypt_local_t,
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    n: u64,
    _r: u32,
    _p: u32,
    buf: *mut u8,
    buflen: usize,
) -> c_int {
    let r: usize = _r as usize;
    let p: usize = _p as usize;

    // SIZE_MAX > UINT32_MAX on this (64-bit) target, so this check is
    // always compiled in.
    if buflen as u64 > (((1u64) << 32) - 1) * 32 {
        set_errno(EFBIG);
        return -1;
    }
    if (r as u64).wrapping_mul(p as u64) >= (1u64 << 30) {
        set_errno(EFBIG);
        return -1;
    }
    if n > u32::MAX as u64 {
        set_errno(EFBIG);
        return -1;
    }
    if (n & (n - 1)) != 0 || n < 2 {
        set_errno(EINVAL);
        return -1;
    }
    if r == 0 || p == 0 {
        set_errno(EINVAL);
        return -1;
    }
    // Note: the `SIZE_MAX / 256 <= UINT32_MAX` branch from the original is
    // not compiled on this (64-bit) target.
    if r > usize::MAX / 128 / p || n > usize::MAX as u64 / 128 / (r as u64) {
        set_errno(ENOMEM);
        return -1;
    }

    let b_size: usize = 128usize.wrapping_mul(r).wrapping_mul(p);
    let v_size: usize = 128usize.wrapping_mul(r).wrapping_mul(n as usize);
    let mut need: usize = b_size.wrapping_add(v_size);
    if need < v_size {
        set_errno(ENOMEM);
        return -1;
    }
    let xy_size: usize = 256usize.wrapping_mul(r).wrapping_add(64);
    need = need.wrapping_add(xy_size);
    if need < xy_size {
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
    let b = (*local).aligned as *mut u8;
    let v = b.add(b_size) as *mut u32;
    let xy = (v as *mut u8).add(v_size) as *mut u32;

    // 1: (B_0 ... B_{p-1}) <-- PBKDF2(P, S, 1, p * MFLen)
    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, 1, b, b_size);

    // 2: for i = 0 to p - 1 do
    for i in 0..p {
        // 3: B_i <-- MF(B_i, N)
        smix(b.add(128usize.wrapping_mul(i).wrapping_mul(r)), r, n, v, xy);
    }

    // 5: DK <-- PBKDF2(P, B, 1, dkLen)
    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, b, b_size, 1, buf, buflen);

    0
}

// =======================================================================
// crypto_scrypt-common.c
// =======================================================================

static ITOA64: &[u8; 65] =
    b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\0";

unsafe fn encode64_uint32(mut dst: *mut u8, mut dstlen: usize, mut src: u32, srcbits: u32) -> *mut u8 {
    let mut bit = 0u32;
    while bit < srcbits {
        if dstlen < 1 {
            return null_mut();
        }
        *dst = ITOA64[(src & 0x3f) as usize];
        dst = dst.add(1);
        dstlen -= 1;
        src >>= 6;
        bit += 6;
    }
    dst
}

unsafe fn encode64(mut dst: *mut u8, mut dstlen: usize, src: *const u8, srclen: usize) -> *mut u8 {
    let mut i: usize = 0;
    while i < srclen {
        let mut value: u32 = 0;
        let mut bits: u32 = 0;

        loop {
            value |= (*src.add(i) as u32) << bits;
            i += 1;
            bits += 8;
            if !(bits < 24 && i < srclen) {
                break;
            }
        }

        let dnext = encode64_uint32(dst, dstlen, value, bits);
        if dnext.is_null() {
            return null_mut();
        }
        dstlen -= dnext as usize - dst as usize;
        dst = dnext;
    }
    dst
}

unsafe fn decode64_one(dst: *mut u32, src: u8) -> c_int {
    let ptr = strchr(ITOA64.as_ptr() as *const c_char, src as c_int);
    if !ptr.is_null() {
        *dst = (ptr as usize - ITOA64.as_ptr() as usize) as u32;
        return 0;
    }
    *dst = 0;
    -1
}

unsafe fn decode64_uint32(dst: *mut u32, dstbits: u32, mut src: *const u8) -> *const u8 {
    let mut bit = 0u32;
    let mut value: u32 = 0;

    while bit < dstbits {
        let mut one: u32 = 0;
        if decode64_one(&mut one, *src) != 0 {
            *dst = 0;
            return null();
        }
        src = src.add(1);
        value |= one << bit;
        bit += 6;
    }
    *dst = value;
    src
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_parse_setting(
    setting: *const u8,
    n_log2_p: *mut u32,
    r_p: *mut u32,
    p_p: *mut u32,
) -> *const u8 {
    if *setting != b'$' || *setting.add(1) != b'7' || *setting.add(2) != b'$' {
        return null();
    }
    let mut src = setting.add(3);

    if decode64_one(n_log2_p, *src) != 0 {
        return null();
    }
    src = src.add(1);

    let src2 = decode64_uint32(r_p, 30, src);
    if src2.is_null() {
        return null();
    }

    let src3 = decode64_uint32(p_p, 30, src2);
    if src3.is_null() {
        return null();
    }
    src3
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_r(
    local: *mut escrypt_local_t,
    passwd: *const u8,
    passwdlen: usize,
    setting: *const u8,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let mut hash = [0u8; 32];
    let mut n_log2: u32 = 0;
    let mut r: u32 = 0;
    let mut p: u32 = 0;

    if !buf.is_null() {
        randombytes_buf(buf as *mut c_void, buflen);
    }

    let src = _sodium_escrypt_parse_setting(setting, &mut n_log2, &mut r, &mut p);
    if src.is_null() {
        return null_mut();
    }
    let n: u64 = 1u64 << n_log2;
    let prefixlen = src as usize - setting as usize;

    let salt = src;
    let src_dollar = strrchr(salt as *const c_char, b'$' as c_int);
    let saltlen: usize = if !src_dollar.is_null() {
        src_dollar as usize - salt as usize
    } else {
        crate::csys::strlen(salt as *const c_char)
    };
    let need = prefixlen + saltlen + 1 + 43 + 1;
    if buf.is_null() || need > buflen || need < saltlen {
        return null_mut();
    }

    if _sodium_escrypt_kdf_nosse(
        local,
        passwd,
        passwdlen,
        salt,
        saltlen,
        n,
        r,
        p,
        hash.as_mut_ptr(),
        hash.len(),
    ) != 0
    {
        return null_mut();
    }

    let mut dst = buf;
    memcpy(
        dst as *mut c_void,
        setting as *const c_void,
        prefixlen + saltlen,
    );
    dst = dst.add(prefixlen + saltlen);
    *dst = b'$';
    dst = dst.add(1);

    let dstlen = buflen - (dst as usize - buf as usize);
    let dnext = encode64(dst, dstlen, hash.as_ptr(), hash.len());
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, hash.len());
    if dnext.is_null() || dnext as usize >= buf as usize + buflen {
        return null_mut();
    }
    *dnext = 0;

    buf
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_escrypt_gensalt_r(
    n_log2: u32,
    r: u32,
    p: u32,
    src: *const u8,
    srclen: usize,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let prefixlen: usize = 3 + 1 + 5 + 5;
    let saltlen: usize = ((srclen * 8) + 5) / 6;
    let need = prefixlen + saltlen + 1;

    if need > buflen || need < saltlen || saltlen < srclen {
        return null_mut();
    }
    if n_log2 > 63 || ((r as u64).wrapping_mul(p as u64) >= (1u64 << 30)) {
        return null_mut();
    }

    let mut dst = buf;
    *dst = b'$';
    dst = dst.add(1);
    *dst = b'7';
    dst = dst.add(1);
    *dst = b'$';
    dst = dst.add(1);

    *dst = ITOA64[n_log2 as usize];
    dst = dst.add(1);

    let dstlen = buflen - (dst as usize - buf as usize);
    let dnext = encode64_uint32(dst, dstlen, r, 30);
    if dnext.is_null() {
        return null_mut();
    }
    dst = dnext;

    let dstlen = buflen - (dst as usize - buf as usize);
    let dnext = encode64_uint32(dst, dstlen, p, 30);
    if dnext.is_null() {
        return null_mut();
    }
    dst = dnext;

    let dstlen = buflen - (dst as usize - buf as usize);
    let dnext = encode64(dst, dstlen, src, srclen);
    if dnext.is_null() || dnext as usize >= buf as usize + buflen {
        return null_mut();
    }
    dst = dnext;
    *dst = 0;

    buf
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_ll(
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    n: u64,
    r: u32,
    p: u32,
    buf: *mut u8,
    buflen: usize,
) -> c_int {
    let mut local: escrypt_local_t = escrypt_local_t {
        base: null_mut(),
        aligned: null_mut(),
        size: 0,
    };

    if _sodium_escrypt_init_local(&mut local) != 0 {
        return -1;
    }

    let retval = _sodium_escrypt_kdf_nosse(
        &mut local, passwd, passwdlen, salt, saltlen, n, r, p, buf, buflen,
    );

    if _sodium_escrypt_free_local(&mut local) != 0 {
        return -1;
    }
    retval
}

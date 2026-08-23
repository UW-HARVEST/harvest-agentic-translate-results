//! Translation of scryptsalsa208sha256:
//! pwhash_scryptsalsa208sha256.c, crypto_scrypt-common.c, pbkdf2-sha256.c,
//! scrypt_platform.c, nosse/pwhash_scryptsalsa208sha256_nosse.c

use core::ffi::{c_char, c_int, c_void};
use crate::common::{load32_le, store32_be, store32_le};

// ---------------------------------------------------------------------------
// Externs
// ---------------------------------------------------------------------------

#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

#[repr(C)]
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
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha256_final(
        state: *mut crypto_auth_hmacsha256_state,
        out: *mut u8,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;
    fn sodium_misuse() -> !;
}

#[inline]
unsafe fn set_errno(e: c_int) {
    *libc::__errno_location() = e;
}

// ---------------------------------------------------------------------------
// Constants (crypto_pwhash_scryptsalsa208sha256.h + crypto_scrypt.h)
// ---------------------------------------------------------------------------

const BYTES_MIN: usize = 16;
const BYTES_MAX: u64 = 0x1fffffffe0; // min(SIZE_MAX, 0x1fffffffe0)
const PASSWD_MIN: u64 = 0;
const PASSWD_MAX: usize = usize::MAX; // SODIUM_SIZE_MAX
const SALTBYTES: usize = 32;
const STRBYTES: usize = 102;
const STRPREFIX: &[u8] = b"$7$\0";
const OPSLIMIT_MIN: u64 = 32768;
const OPSLIMIT_MAX: u64 = 4294967295;
const MEMLIMIT_MIN: usize = 16777216;
const MEMLIMIT_MAX: usize = 68719476736; // min(SIZE_MAX, 68719476736)
const OPSLIMIT_INTERACTIVE: u64 = 524288;
const MEMLIMIT_INTERACTIVE: usize = 16777216;
const OPSLIMIT_SENSITIVE: u64 = 33554432;
const MEMLIMIT_SENSITIVE: usize = 1073741824;

const STRSETTINGBYTES: usize = 57;
const STRSALTBYTES: usize = 32;
const STRHASHBYTES: usize = 32;
const STRHASHBYTES_ENCODED: usize = 43;

#[inline]
fn bytes2chars(bytes: usize) -> usize {
    // C: `#define BYTES2CHARS(bytes) ((((bytes) * 8) + 5) / 6)` — plain size_t
    // arithmetic, which wraps rather than trapping on overflow.
    bytes.wrapping_mul(8).wrapping_add(5) / 6
}

// ---------------------------------------------------------------------------
// escrypt_region_t / escrypt_local_t
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct escrypt_region_t {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}
type escrypt_local_t = escrypt_region_t;

// ---------------------------------------------------------------------------
// scrypt_platform.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_alloc_region(
    region: *mut escrypt_region_t,
    size: usize,
) -> *mut c_void {
    // Portable path: malloc(size + 63) and align to 64.
    let mut base: *mut u8 = core::ptr::null_mut();
    let mut aligned: *mut u8 = core::ptr::null_mut();
    // C: `if (size + 63 < size) { ... } else if ((base = malloc(size + 63)))`
    // — size_t arithmetic wraps, so the overflow test must not trap.
    if size.wrapping_add(63) < size {
        set_errno(libc::ENOMEM);
    } else {
        base = libc::malloc(size.wrapping_add(63)) as *mut u8;
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
    (*region).base = core::ptr::null_mut();
    (*region).aligned = core::ptr::null_mut();
    (*region).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int {
    if !(*region).base.is_null() {
        libc::free((*region).base);
    }
    init_region(region);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int {
    init_region(local);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int {
    _sodium_escrypt_free_region(local)
}

// ---------------------------------------------------------------------------
// pbkdf2-sha256.c  ->  _sodium_escrypt_PBKDF2_SHA256
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_PBKDF2_SHA256(
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    c: u64,
    buf: *mut u8,
    dklen: usize,
) {
    let mut pshctx: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut hctx: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut ivec = [0u8; 4];
    let mut u = [0u8; 32];
    let mut t = [0u8; 32];

    // SIZE_MAX > 0x1fffffffe0ULL on 64-bit.
    if dklen as u64 > 0x1fffffffe0u64 {
        sodium_misuse();
    }

    crypto_auth_hmacsha256_init(&mut pshctx, passwd, passwdlen);
    crypto_auth_hmacsha256_update(&mut pshctx, salt, saltlen as u64);

    let mut i: usize = 0;
    while i * 32 < dklen {
        store32_be(&mut ivec, (i + 1) as u32);
        core::ptr::copy_nonoverlapping(
            &pshctx as *const crypto_auth_hmacsha256_state,
            &mut hctx as *mut crypto_auth_hmacsha256_state,
            1,
        );
        crypto_auth_hmacsha256_update(&mut hctx, ivec.as_ptr(), 4);
        crypto_auth_hmacsha256_final(&mut hctx, u.as_mut_ptr());

        t.copy_from_slice(&u);
        let mut j: u64 = 2;
        while j <= c {
            crypto_auth_hmacsha256_init(&mut hctx, passwd, passwdlen);
            crypto_auth_hmacsha256_update(&mut hctx, u.as_ptr(), 32);
            crypto_auth_hmacsha256_final(&mut hctx, u.as_mut_ptr());
            for k in 0..32 {
                t[k] ^= u[k];
            }
            j += 1;
        }

        let mut clen = dklen - i * 32;
        if clen > 32 {
            clen = 32;
        }
        core::ptr::copy_nonoverlapping(t.as_ptr(), buf.add(i * 32), clen);
        i += 1;
    }
    sodium_memzero(
        &mut pshctx as *mut crypto_auth_hmacsha256_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );
}

// ---------------------------------------------------------------------------
// nosse/pwhash_scryptsalsa208sha256_nosse.c  ->  _sodium_escrypt_kdf_nosse
// ---------------------------------------------------------------------------

#[inline(always)]
unsafe fn blkcpy(dest: *mut u32, src: *const u32, len: usize) {
    core::ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, len * 64);
}

#[inline(always)]
unsafe fn blkxor(dest: *mut u32, src: *const u32, len: usize) {
    for i in 0..len * 16 {
        *dest.add(i) ^= *src.add(i);
    }
}

#[inline(always)]
fn r(a: u32, b: u32) -> u32 {
    (a << b) | (a >> (32 - b))
}

fn salsa20_8(b: &mut [u32; 16]) {
    let mut x = *b;
    let mut i = 0;
    while i < 8 {
        // Operate on columns.
        x[4] ^= r(x[0].wrapping_add(x[12]), 7);
        x[8] ^= r(x[4].wrapping_add(x[0]), 9);
        x[12] ^= r(x[8].wrapping_add(x[4]), 13);
        x[0] ^= r(x[12].wrapping_add(x[8]), 18);

        x[9] ^= r(x[5].wrapping_add(x[1]), 7);
        x[13] ^= r(x[9].wrapping_add(x[5]), 9);
        x[1] ^= r(x[13].wrapping_add(x[9]), 13);
        x[5] ^= r(x[1].wrapping_add(x[13]), 18);

        x[14] ^= r(x[10].wrapping_add(x[6]), 7);
        x[2] ^= r(x[14].wrapping_add(x[10]), 9);
        x[6] ^= r(x[2].wrapping_add(x[14]), 13);
        x[10] ^= r(x[6].wrapping_add(x[2]), 18);

        x[3] ^= r(x[15].wrapping_add(x[11]), 7);
        x[7] ^= r(x[3].wrapping_add(x[15]), 9);
        x[11] ^= r(x[7].wrapping_add(x[3]), 13);
        x[15] ^= r(x[11].wrapping_add(x[7]), 18);

        // Operate on rows.
        x[1] ^= r(x[0].wrapping_add(x[3]), 7);
        x[2] ^= r(x[1].wrapping_add(x[0]), 9);
        x[3] ^= r(x[2].wrapping_add(x[1]), 13);
        x[0] ^= r(x[3].wrapping_add(x[2]), 18);

        x[6] ^= r(x[5].wrapping_add(x[4]), 7);
        x[7] ^= r(x[6].wrapping_add(x[5]), 9);
        x[4] ^= r(x[7].wrapping_add(x[6]), 13);
        x[5] ^= r(x[4].wrapping_add(x[7]), 18);

        x[11] ^= r(x[10].wrapping_add(x[9]), 7);
        x[8] ^= r(x[11].wrapping_add(x[10]), 9);
        x[9] ^= r(x[8].wrapping_add(x[11]), 13);
        x[10] ^= r(x[9].wrapping_add(x[8]), 18);

        x[12] ^= r(x[15].wrapping_add(x[14]), 7);
        x[13] ^= r(x[12].wrapping_add(x[15]), 9);
        x[14] ^= r(x[13].wrapping_add(x[12]), 13);
        x[15] ^= r(x[14].wrapping_add(x[13]), 18);

        i += 2;
    }
    for i in 0..16 {
        b[i] = b[i].wrapping_add(x[i]);
    }
}

unsafe fn blockmix_salsa8(bin: *const u32, bout: *mut u32, x: *mut u32, r_: usize) {
    // 1: X <-- B_{2r - 1}
    blkcpy(x, bin.add((2 * r_ - 1) * 16), 1);

    let mut i = 0;
    while i < 2 * r_ {
        blkxor(x, bin.add(i * 16), 1);
        {
            let xs = &mut *(x as *mut [u32; 16]);
            salsa20_8(xs);
        }
        blkcpy(bout.add(i * 8), x, 1);

        blkxor(x, bin.add(i * 16 + 16), 1);
        {
            let xs = &mut *(x as *mut [u32; 16]);
            salsa20_8(xs);
        }
        blkcpy(bout.add(i * 8 + r_ * 16), x, 1);

        i += 2;
    }
}

#[inline(always)]
unsafe fn integerify(b: *const u32, r_: usize) -> u64 {
    let x = b.add((2 * r_ - 1) * 16);
    ((*x.add(1) as u64) << 32).wrapping_add(*x as u64)
}

unsafe fn smix(b: *mut u8, r_: usize, n: u64, v: *mut u32, xy: *mut u32) {
    let x = xy;
    let y = xy.add(32 * r_);
    let z = xy.add(64 * r_);

    for k in 0..32 * r_ {
        let s = core::slice::from_raw_parts(b.add(4 * k), 4);
        *x.add(k) = load32_le(s);
    }

    let mut i: u64 = 0;
    while i < n {
        blkcpy(v.add((i as usize) * (32 * r_)), x, 2 * r_);
        blockmix_salsa8(x, y, z, r_);

        blkcpy(v.add(((i + 1) as usize) * (32 * r_)), y, 2 * r_);
        blockmix_salsa8(y, x, z, r_);
        i += 2;
    }

    let mut i: u64 = 0;
    while i < n {
        let j = integerify(x, r_) & (n - 1);
        blkxor(x, v.add((j as usize) * (32 * r_)), 2 * r_);
        blockmix_salsa8(x, y, z, r_);

        let j = integerify(y, r_) & (n - 1);
        blkxor(y, v.add((j as usize) * (32 * r_)), 2 * r_);
        blockmix_salsa8(y, x, z, r_);
        i += 2;
    }

    for k in 0..32 * r_ {
        let d = core::slice::from_raw_parts_mut(b.add(4 * k), 4);
        store32_le(d, *x.add(k));
    }
}

#[unsafe(no_mangle)]
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
    let r_: usize = _r as usize;
    let p: usize = _p as usize;

    // SIZE_MAX > UINT32_MAX on 64-bit.
    if buflen as u64 > ((1u64 << 32) - 1) * 32 {
        set_errno(libc::EFBIG);
        return -1;
    }
    if (r_ as u64) * (p as u64) >= (1u64 << 30) {
        set_errno(libc::EFBIG);
        return -1;
    }
    if n > u32::MAX as u64 {
        set_errno(libc::EFBIG);
        return -1;
    }
    // C: `((N & (N - 1)) != 0) || (N < 2)` — for N == 0 the `N - 1` wraps to
    // UINT64_MAX in C, so the subtraction must not trap here either.
    if (n & n.wrapping_sub(1)) != 0 || n < 2 {
        set_errno(libc::EINVAL);
        return -1;
    }
    if r_ == 0 || p == 0 {
        set_errno(libc::EINVAL);
        return -1;
    }
    // SIZE_MAX / 256 <= UINT32_MAX is false on 64-bit, so that check is skipped.
    if (r_ > usize::MAX / 128 / p) || (n > (usize::MAX as u64) / 128 / (r_ as u64)) {
        set_errno(libc::ENOMEM);
        return -1;
    }

    let b_size: usize = 128 * r_ * p;
    let v_size: usize = 128 * r_ * (n as usize);
    // C computes `need = B_size + V_size` in size_t and then tests for wrap
    // (`need < V_size`); the addition itself must therefore wrap, not trap.
    let mut need: usize = b_size.wrapping_add(v_size);
    if need < v_size {
        set_errno(libc::ENOMEM);
        return -1;
    }
    let xy_size: usize = 256 * r_ + 64;
    need = need.wrapping_add(xy_size);
    if need < xy_size {
        set_errno(libc::ENOMEM);
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

    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, 1, b, b_size);

    let mut i: u32 = 0;
    while (i as usize) < p {
        smix(b.add(128 * (i as usize) * r_), r_, n, v, xy);
        i += 1;
    }

    _sodium_escrypt_PBKDF2_SHA256(passwd, passwdlen, b, b_size, 1, buf, buflen);

    0
}

// ---------------------------------------------------------------------------
// crypto_scrypt-common.c
// ---------------------------------------------------------------------------

const ITOA64: &[u8; 64] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

unsafe fn encode64_uint32(
    mut dst: *mut u8,
    mut dstlen: usize,
    mut src: u32,
    srcbits: u32,
) -> *mut u8 {
    let mut bit: u32 = 0;
    while bit < srcbits {
        if dstlen < 1 {
            return core::ptr::null_mut();
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
            return core::ptr::null_mut();
        }
        dstlen -= (dnext as usize) - (dst as usize);
        dst = dnext;
    }
    dst
}

unsafe fn decode64_one(dst: *mut u32, src: u8) -> c_int {
    // strchr(itoa64, src): note src can be 0 (NUL), which strchr matches at end.
    for i in 0..64usize {
        if ITOA64[i] == src {
            *dst = i as u32;
            return 0;
        }
    }
    // Match C strchr semantics: NUL terminator matches (returns pointer to '\0').
    if src == 0 {
        *dst = 64; // ptr - itoa64 == 64 (index of NUL)
        return 0;
    }
    *dst = 0;
    -1
}

unsafe fn decode64_uint32(dst: *mut u32, dstbits: u32, mut src: *const u8) -> *const u8 {
    let mut value: u32 = 0;
    let mut bit: u32 = 0;
    while bit < dstbits {
        let mut one: u32 = 0;
        if decode64_one(&mut one, *src) != 0 {
            *dst = 0;
            return core::ptr::null();
        }
        src = src.add(1);
        value |= one << bit;
        bit += 6;
    }
    *dst = value;
    src
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_parse_setting(
    setting: *const u8,
    n_log2_p: *mut u32,
    r_p: *mut u32,
    p_p: *mut u32,
) -> *const u8 {
    if *setting.add(0) != b'$' || *setting.add(1) != b'7' || *setting.add(2) != b'$' {
        return core::ptr::null();
    }
    let mut src = setting.add(3);

    if decode64_one(n_log2_p, *src) != 0 {
        return core::ptr::null();
    }
    src = src.add(1);

    src = decode64_uint32(r_p, 30, src);
    if src.is_null() {
        return core::ptr::null();
    }

    src = decode64_uint32(p_p, 30, src);
    if src.is_null() {
        return core::ptr::null();
    }
    src
}

unsafe fn strrchr_dollar(s: *const u8) -> *const u8 {
    // Find last '$'.
    let mut last: *const u8 = core::ptr::null();
    let mut p = s;
    loop {
        let c = *p;
        if c == b'$' {
            last = p;
        }
        if c == 0 {
            break;
        }
        p = p.add(1);
    }
    last
}

unsafe fn c_strlen(mut p: *const u8) -> usize {
    let mut n = 0usize;
    while *p != 0 {
        n += 1;
        p = p.add(1);
    }
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_r(
    local: *mut escrypt_local_t,
    passwd: *const u8,
    passwdlen: usize,
    setting: *const u8,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let mut hash = [0u8; STRHASHBYTES];
    let mut n_log2: u32 = 0;
    let mut r: u32 = 0;
    let mut p: u32 = 0;

    if !buf.is_null() {
        randombytes_buf(buf as *mut c_void, buflen);
    }

    let src = _sodium_escrypt_parse_setting(setting, &mut n_log2, &mut r, &mut p);
    if src.is_null() {
        return core::ptr::null_mut();
    }
    let n: u64 = 1u64 << n_log2;
    let prefixlen = (src as usize) - (setting as usize);

    let salt = src;
    let src2 = strrchr_dollar(salt);
    let saltlen: usize;
    if !src2.is_null() {
        saltlen = (src2 as usize) - (salt as usize);
    } else {
        saltlen = c_strlen(salt);
    }
    let need = prefixlen + saltlen + 1 + STRHASHBYTES_ENCODED + 1;
    if buf.is_null() || need > buflen || need < saltlen {
        return core::ptr::null_mut();
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
        STRHASHBYTES,
    ) != 0
    {
        return core::ptr::null_mut();
    }
    let mut dst = buf;
    core::ptr::copy_nonoverlapping(setting, dst, prefixlen + saltlen);
    dst = dst.add(prefixlen + saltlen);
    *dst = b'$';
    dst = dst.add(1);

    dst = encode64(
        dst,
        buflen - ((dst as usize) - (buf as usize)),
        hash.as_ptr(),
        STRHASHBYTES,
    );
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, STRHASHBYTES);
    if dst.is_null() || dst as usize >= (buf as usize) + buflen {
        return core::ptr::null_mut();
    }
    *dst = 0;

    buf
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_gensalt_r(
    n_log2: u32,
    r: u32,
    p: u32,
    src: *const u8,
    srclen: usize,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let prefixlen: usize = (b"$7$".len()) + 1 + 5 + 5; // 3 + 1 + 5 + 5
    let saltlen: usize = bytes2chars(srclen);
    let need = prefixlen + saltlen + 1;
    if need > buflen || need < saltlen || saltlen < srclen {
        return core::ptr::null_mut();
    }
    if n_log2 > 63 || ((r as u64) * (p as u64) >= (1u64 << 30)) {
        return core::ptr::null_mut();
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

    dst = encode64_uint32(dst, buflen - ((dst as usize) - (buf as usize)), r, 30);
    if dst.is_null() {
        return core::ptr::null_mut();
    }
    dst = encode64_uint32(dst, buflen - ((dst as usize) - (buf as usize)), p, 30);
    if dst.is_null() {
        return core::ptr::null_mut();
    }
    dst = encode64(dst, buflen - ((dst as usize) - (buf as usize)), src, srclen);
    if dst.is_null() || dst as usize >= (buf as usize) + buflen {
        return core::ptr::null_mut();
    }
    *dst = 0;

    buf
}

#[unsafe(no_mangle)]
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
    let mut local: escrypt_local_t = core::mem::zeroed();

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

// ---------------------------------------------------------------------------
// pwhash_scryptsalsa208sha256.c
// ---------------------------------------------------------------------------

fn pickparams(
    mut opslimit: u64,
    memlimit: usize,
    n_log2: &mut u32,
    p: &mut u32,
    r: &mut u32,
) -> c_int {
    let maxn: u64;
    let mut maxrp: u64;

    if opslimit < 32768 {
        opslimit = 32768;
    }
    *r = 8;
    if opslimit < (memlimit as u64) / 32 {
        *p = 1;
        maxn = opslimit / ((*r as u64) * 4);
        *n_log2 = 1;
        while *n_log2 < 63 {
            if (1u64 << *n_log2) > maxn / 2 {
                break;
            }
            *n_log2 += 1;
        }
    } else {
        let maxn2 = (memlimit as u64) / ((*r as u64) * 128);
        *n_log2 = 1;
        while *n_log2 < 63 {
            if (1u64 << *n_log2) > maxn2 / 2 {
                break;
            }
            *n_log2 += 1;
        }
        maxrp = (opslimit / 4) / (1u64 << *n_log2);
        if maxrp > 0x3fffffff {
            maxrp = 0x3fffffff;
        }
        *p = (maxrp as u32) / *r;
    }
    0
}

unsafe fn sodium_strnlen(str_: *const c_char, maxlen: usize) -> usize {
    let mut i: usize = 0;
    // ACQUIRE_FENCE
    core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
    while i < maxlen && *str_.add(i) != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_min() -> usize {
    BYTES_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_max() -> usize {
    BYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_min() -> usize {
    PASSWD_MIN as usize
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_max() -> usize {
    PASSWD_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_saltbytes() -> usize {
    SALTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_strbytes() -> usize {
    STRBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_strprefix() -> *const c_char {
    STRPREFIX.as_ptr() as *const c_char
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_min() -> u64 {
    OPSLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_max() -> u64 {
    OPSLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_min() -> usize {
    MEMLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_max() -> usize {
    MEMLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_interactive() -> u64 {
    OPSLIMIT_INTERACTIVE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_interactive() -> usize {
    MEMLIMIT_INTERACTIVE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive() -> u64 {
    OPSLIMIT_SENSITIVE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive() -> usize {
    MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut n_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    core::ptr::write_bytes(out, 0, outlen as usize);
    if passwdlen > PASSWD_MAX as u64 || outlen > BYTES_MAX {
        set_errno(libc::EFBIG);
        return -1;
    }
    if outlen < BYTES_MIN as u64
        || pickparams(opslimit, memlimit, &mut n_log2, &mut p, &mut r) != 0
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        set_errno(libc::EINVAL);
        return -1;
    }
    crypto_pwhash_scryptsalsa208sha256_ll(
        passwd as *const u8,
        passwdlen as usize,
        salt,
        SALTBYTES,
        1u64 << n_log2,
        r,
        p,
        out,
        outlen as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt = [0u8; STRSALTBYTES];
    let mut setting = [0u8; STRSETTINGBYTES + 1];
    let mut escrypt_local: escrypt_local_t = core::mem::zeroed();
    let mut n_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    core::ptr::write_bytes(out, 0, STRBYTES);
    if passwdlen > PASSWD_MAX as u64 {
        set_errno(libc::EFBIG);
        return -1;
    }
    if passwdlen < PASSWD_MIN
        || pickparams(opslimit, memlimit, &mut n_log2, &mut p, &mut r) != 0
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, STRSALTBYTES);
    if _sodium_escrypt_gensalt_r(
        n_log2,
        r,
        p,
        salt.as_ptr(),
        STRSALTBYTES,
        setting.as_mut_ptr(),
        setting.len(),
    )
    .is_null()
    {
        set_errno(libc::EINVAL);
        return -1;
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1;
    }
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        setting.as_ptr(),
        out as *mut u8,
        STRBYTES,
    )
    .is_null()
    {
        _sodium_escrypt_free_local(&mut escrypt_local);
        set_errno(libc::EINVAL);
        return -1;
    }
    _sodium_escrypt_free_local(&mut escrypt_local);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let mut wanted = [0u8; STRBYTES];
    let mut escrypt_local: escrypt_local_t = core::mem::zeroed();
    let ret: c_int;

    if sodium_strnlen(str_, STRBYTES) != STRBYTES - 1 {
        return -1;
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1;
    }
    // memset(wanted, 0) already zeroed.
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        str_ as *const u8,
        wanted.as_mut_ptr(),
        STRBYTES,
    )
    .is_null()
    {
        _sodium_escrypt_free_local(&mut escrypt_local);
        return -1;
    }
    _sodium_escrypt_free_local(&mut escrypt_local);
    ret = sodium_memcmp(
        wanted.as_ptr() as *const c_void,
        str_ as *const c_void,
        STRBYTES,
    );
    sodium_memzero(wanted.as_mut_ptr() as *mut c_void, STRBYTES);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut n_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;
    let mut n_log2_: u32 = 0;
    let mut p_: u32 = 0;
    let mut r_: u32 = 0;

    if pickparams(opslimit, memlimit, &mut n_log2, &mut p, &mut r) != 0 {
        set_errno(libc::EINVAL);
        return -1;
    }
    if sodium_strnlen(str_, STRBYTES) != STRBYTES - 1 {
        set_errno(libc::EINVAL);
        return -1;
    }
    if _sodium_escrypt_parse_setting(str_ as *const u8, &mut n_log2_, &mut r_, &mut p_).is_null() {
        set_errno(libc::EINVAL);
        return -1;
    }
    if n_log2 != n_log2_ || r != r_ || p != p_ {
        return 1;
    }
    0
}

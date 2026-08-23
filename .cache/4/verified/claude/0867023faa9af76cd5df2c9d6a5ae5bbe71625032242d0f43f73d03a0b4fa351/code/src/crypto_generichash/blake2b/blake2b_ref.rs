//! Translation of `crypto_generichash/blake2b/ref/blake2b-ref.c` plus the
//! `blake2b_state` / `blake2b_param` declarations from
//! `crypto_generichash/blake2b/ref/blake2.h`.
//!
//! Build configuration notes:
//!   * `HAVE_TI_MODE` is undefined, so `blake2b_increment_counter` uses the
//!     two-word fallback.
//!   * `NATIVE_LITTLE_ENDIAN` is undefined, so byte-wise load/store is used.
//!   * No SIMD headers are available, so `blake2b_pick_best_implementation`
//!     always selects `blake2b_compress_ref`.
//!   * The reference library is built with `-DNDEBUG`, so `assert()` is a
//!     no-op and `COMPILER_ASSERT()` is a compile-time-only construct.

use core::ffi::{c_int, c_void};

use crate::common::{load64_le, memcpy, memset, store32_le, store64_le};
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::sodium_memzero;

// ---------------------------------------------------------------------------
// enum blake2b_constant
// ---------------------------------------------------------------------------

pub const BLAKE2B_BLOCKBYTES: usize = 128;
pub const BLAKE2B_OUTBYTES: usize = 64;
pub const BLAKE2B_KEYBYTES: usize = 64;
pub const BLAKE2B_SALTBYTES: usize = 16;
pub const BLAKE2B_PERSONALBYTES: usize = 16;

// ---------------------------------------------------------------------------
// struct layouts (blake2.h, inside `#pragma pack(push, 1)`)
// ---------------------------------------------------------------------------

/// `blake2b_param` -- 64 packed bytes.  Every member is a `uint8_t` (or an
/// array thereof), so the natural `#[repr(C)]` layout is already the packed
/// one: size 64, alignment 1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct blake2b_param {
    pub digest_length: u8,                    /*  1 */
    pub key_length: u8,                       /*  2 */
    pub fanout: u8,                           /*  3 */
    pub depth: u8,                            /*  4 */
    pub leaf_length: [u8; 4],                 /*  8 */
    pub node_offset: [u8; 8],                 /* 16 */
    pub node_depth: u8,                       /* 17 */
    pub inner_length: u8,                     /* 18 */
    pub reserved: [u8; 14],                   /* 32 */
    pub salt: [u8; BLAKE2B_SALTBYTES],        /* 48 */
    pub personal: [u8; BLAKE2B_PERSONALBYTES], /* 64 */
}

/// `blake2b_state`.
///
/// The C declaration sits inside `#pragma pack(push, 1)`, which yields
/// `sizeof == 361` / `_Alignof == 1`.  Because every non-`uint8_t` member
/// already lands on a naturally aligned offset, the *member offsets* of the
/// packed struct and of the plain `#[repr(C)]` struct below are identical:
///
///     h = 0, t = 64, f = 80, buf = 96, buflen = 352, last_node = 360
///
/// The only difference is 7 bytes of trailing padding (`sizeof` 368 vs. 361).
/// That padding is unobservable here: `blake2b_state` never escapes these
/// files (callers only ever see the 384-byte `crypto_generichash_blake2b_state`
/// buffer), the struct is never embedded in another struct or array with more
/// than one element, and `blake2b_init0` derives its `memset` length from
/// `offsetof` rather than from `sizeof`.  Using the unpacked layout keeps every
/// `uint64_t` access naturally aligned, exactly as in the C build where the
/// state buffer is 64-byte aligned.
#[repr(C)]
pub struct blake2b_state {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; 2 * 128],
    pub buflen: usize,
    pub last_node: u8,
}

/// `typedef int (*blake2b_compress_fn)(blake2b_state *, const uint8_t[128]);`
pub type blake2b_compress_fn =
    unsafe extern "C" fn(S: *mut blake2b_state, block: *const u8) -> c_int;

// `blake2b_compress_ref` lives in blake2b-compress-ref.c.
unsafe extern "C" {
    fn _sodium_blake2b_compress_ref(S: *mut blake2b_state, block: *const u8) -> c_int;
}

/// `static blake2b_compress_fn blake2b_compress = blake2b_compress_ref;`
static mut blake2b_compress: blake2b_compress_fn = _sodium_blake2b_compress_ref;

static blake2b_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

// ---------------------------------------------------------------------------
// static helpers
// ---------------------------------------------------------------------------

/* LCOV_EXCL_START */
#[inline]
unsafe fn blake2b_set_lastnode(S: *mut blake2b_state) -> c_int {
    /* S->f[1] = -1; -- (uint64_t) -1 */
    unsafe { (*S).f[1] = u64::MAX };
    0
}
/* LCOV_EXCL_STOP */

#[inline]
unsafe fn blake2b_is_lastblock(S: *const blake2b_state) -> c_int {
    (unsafe { (*S).f[0] } != 0) as c_int
}

#[inline]
unsafe fn blake2b_set_lastblock(S: *mut blake2b_state) -> c_int {
    if unsafe { (*S).last_node } != 0 {
        unsafe { blake2b_set_lastnode(S) }; /* LCOV_EXCL_LINE */
    }
    unsafe { (*S).f[0] = u64::MAX };
    0
}

#[inline]
unsafe fn blake2b_increment_counter(S: *mut blake2b_state, inc: u64) -> c_int {
    /* !HAVE_TI_MODE */
    unsafe {
        (*S).t[0] = (*S).t[0].wrapping_add(inc);
        (*S).t[1] = (*S).t[1].wrapping_add(((*S).t[0] < inc) as u64);
    }
    0
}

/* Parameter-related functions */
#[inline]
unsafe fn blake2b_param_set_salt(P: *mut blake2b_param, salt: *const u8) -> c_int {
    unsafe { memcpy((&raw mut (*P).salt) as *mut u8, salt, BLAKE2B_SALTBYTES) };
    0
}

#[inline]
unsafe fn blake2b_param_set_personal(P: *mut blake2b_param, personal: *const u8) -> c_int {
    unsafe {
        memcpy(
            (&raw mut (*P).personal) as *mut u8,
            personal,
            BLAKE2B_PERSONALBYTES,
        )
    };
    0
}

/// Length of the `memset` in `blake2b_init0`:
/// `offsetof(blake2b_state, last_node) + sizeof(S->last_node) - offsetof(blake2b_state, t)`
const BLAKE2B_INIT0_ZERO_LEN: usize = core::mem::offset_of!(blake2b_state, last_node)
    + core::mem::size_of::<u8>()
    - core::mem::offset_of!(blake2b_state, t);

#[inline]
unsafe fn blake2b_init0(S: *mut blake2b_state) -> c_int {
    for i in 0..8usize {
        unsafe { (*S).h[i] = blake2b_IV[i] };
    }
    /* zero everything between .t and .last_node */
    unsafe { memset((&raw mut (*S).t) as *mut u8, 0, BLAKE2B_INIT0_ZERO_LEN) };
    0
}

/// A freshly declared, still-uninitialised `blake2b_param P[1]`.  Each caller
/// assigns every one of the 64 bytes before use, so starting from zero is
/// equivalent to C's indeterminate value.
#[inline(always)]
const fn blake2b_param_new() -> blake2b_param {
    blake2b_param {
        digest_length: 0,
        key_length: 0,
        fanout: 0,
        depth: 0,
        leaf_length: [0; 4],
        node_offset: [0; 8],
        node_depth: 0,
        inner_length: 0,
        reserved: [0; 14],
        salt: [0; BLAKE2B_SALTBYTES],
        personal: [0; BLAKE2B_PERSONALBYTES],
    }
}

// ---------------------------------------------------------------------------
// public API
// ---------------------------------------------------------------------------

/* init xors IV with input parameter block */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_param(
    S: *mut blake2b_state,
    P: *const blake2b_param,
) -> c_int {
    /* COMPILER_ASSERT(sizeof *P == 64); */
    const _: () = assert!(core::mem::size_of::<blake2b_param>() == 64);

    unsafe { blake2b_init0(S) };
    let p: *const u8 = P as *const u8;

    /* IV XOR ParamBlock */
    for i in 0..8usize {
        unsafe { (*S).h[i] ^= load64_le(p.add(core::mem::size_of::<u64>() * i)) };
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init(S: *mut blake2b_state, outlen: u8) -> c_int {
    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }

    let mut P = blake2b_param_new();
    P.digest_length = outlen;
    P.key_length = 0;
    P.fanout = 1;
    P.depth = 1;
    unsafe { store32_le(P.leaf_length.as_mut_ptr(), 0) };
    unsafe { store64_le(P.node_offset.as_mut_ptr(), 0) };
    P.node_depth = 0;
    P.inner_length = 0;
    P.reserved = [0; 14];
    P.salt = [0; BLAKE2B_SALTBYTES];
    P.personal = [0; BLAKE2B_PERSONALBYTES];

    unsafe { _sodium_blake2b_init_param(S, &P) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }

    let mut P = blake2b_param_new();
    P.digest_length = outlen;
    P.key_length = 0;
    P.fanout = 1;
    P.depth = 1;
    unsafe { store32_le(P.leaf_length.as_mut_ptr(), 0) };
    unsafe { store64_le(P.node_offset.as_mut_ptr(), 0) };
    P.node_depth = 0;
    P.inner_length = 0;
    P.reserved = [0; 14];
    if !salt.is_null() {
        unsafe { blake2b_param_set_salt(&mut P, salt as *const u8) };
    } else {
        P.salt = [0; BLAKE2B_SALTBYTES];
    }
    if !personal.is_null() {
        unsafe { blake2b_param_set_personal(&mut P, personal as *const u8) };
    } else {
        P.personal = [0; BLAKE2B_PERSONALBYTES];
    }

    unsafe { _sodium_blake2b_init_param(S, &P) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
) -> c_int {
    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() || keylen == 0 || (keylen as usize) > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }

    let mut P = blake2b_param_new();
    P.digest_length = outlen;
    P.key_length = keylen;
    P.fanout = 1;
    P.depth = 1;
    unsafe { store32_le(P.leaf_length.as_mut_ptr(), 0) };
    unsafe { store64_le(P.node_offset.as_mut_ptr(), 0) };
    P.node_depth = 0;
    P.inner_length = 0;
    P.reserved = [0; 14];
    P.salt = [0; BLAKE2B_SALTBYTES];
    P.personal = [0; BLAKE2B_PERSONALBYTES];

    if unsafe { _sodium_blake2b_init_param(S, &P) } < 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    {
        let mut block = [0u8; BLAKE2B_BLOCKBYTES];
        unsafe { memset(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES) };
        /* key and keylen cannot be 0 */
        unsafe { memcpy(block.as_mut_ptr(), key as *const u8, keylen as usize) };
        unsafe { _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64) };
        /* Burn the key from stack */
        unsafe {
            sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES)
        };
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() || keylen == 0 || (keylen as usize) > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }

    let mut P = blake2b_param_new();
    P.digest_length = outlen;
    P.key_length = keylen;
    P.fanout = 1;
    P.depth = 1;
    unsafe { store32_le(P.leaf_length.as_mut_ptr(), 0) };
    unsafe { store64_le(P.node_offset.as_mut_ptr(), 0) };
    P.node_depth = 0;
    P.inner_length = 0;
    P.reserved = [0; 14];
    if !salt.is_null() {
        unsafe { blake2b_param_set_salt(&mut P, salt as *const u8) };
    } else {
        P.salt = [0; BLAKE2B_SALTBYTES];
    }
    if !personal.is_null() {
        unsafe { blake2b_param_set_personal(&mut P, personal as *const u8) };
    } else {
        P.personal = [0; BLAKE2B_PERSONALBYTES];
    }

    if unsafe { _sodium_blake2b_init_param(S, &P) } < 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    {
        let mut block = [0u8; BLAKE2B_BLOCKBYTES];
        unsafe { memset(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES) };
        /* key and keylen cannot be 0 */
        unsafe { memcpy(block.as_mut_ptr(), key as *const u8, keylen as usize) };
        unsafe { _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64) };
        /* Burn the key from stack */
        unsafe {
            sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES)
        };
    }
    0
}

/* inlen now in bytes */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_update(
    S: *mut blake2b_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    while inlen > 0 {
        let buf: *mut u8 = unsafe { (&raw mut (*S).buf) as *mut u8 };
        let left: usize = unsafe { (*S).buflen };
        let fill: usize = 2 * BLAKE2B_BLOCKBYTES - left;

        if inlen > fill as u64 {
            /* Fill buffer */
            unsafe { memcpy(buf.add(left), in_, fill) };
            unsafe { (*S).buflen += fill };
            unsafe { blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64) };
            /* Compress */
            let compress = unsafe { blake2b_compress };
            unsafe { compress(S, buf) };
            /* Shift buffer left */
            unsafe { memcpy(buf, buf.add(BLAKE2B_BLOCKBYTES), BLAKE2B_BLOCKBYTES) };
            unsafe { (*S).buflen -= BLAKE2B_BLOCKBYTES };
            in_ = unsafe { in_.add(fill) };
            inlen -= fill as u64;
        } else
        /* inlen <= fill */
        {
            unsafe { memcpy(buf.add(left), in_, inlen as usize) };
            /* Be lazy, do not compress */
            unsafe { (*S).buflen += inlen as usize };
            in_ = unsafe { in_.add(inlen as usize) };
            inlen -= inlen;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_final(
    S: *mut blake2b_state,
    out: *mut u8,
    outlen: u8,
) -> c_int {
    let mut buffer = [0u8; BLAKE2B_OUTBYTES];

    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if unsafe { blake2b_is_lastblock(S) } != 0 {
        return -1;
    }
    let buf: *mut u8 = unsafe { (&raw mut (*S).buf) as *mut u8 };
    if unsafe { (*S).buflen } > BLAKE2B_BLOCKBYTES {
        unsafe { blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64) };
        let compress = unsafe { blake2b_compress };
        unsafe { compress(S, buf) };
        unsafe { (*S).buflen -= BLAKE2B_BLOCKBYTES };
        /* assert(S->buflen <= BLAKE2B_BLOCKBYTES); -- NDEBUG */
        unsafe { memcpy(buf, buf.add(BLAKE2B_BLOCKBYTES), (*S).buflen) };
    }

    unsafe { blake2b_increment_counter(S, (*S).buflen as u64) };
    unsafe { blake2b_set_lastblock(S) };
    /* Padding */
    unsafe {
        memset(
            buf.add((*S).buflen),
            0,
            2 * BLAKE2B_BLOCKBYTES - (*S).buflen,
        )
    };
    let compress = unsafe { blake2b_compress };
    unsafe { compress(S, buf) };

    /* COMPILER_ASSERT(sizeof buffer == 64U); */
    const _: () = assert!(BLAKE2B_OUTBYTES == 64);
    unsafe {
        store64_le(buffer.as_mut_ptr().add(8 * 0), (*S).h[0]);
        store64_le(buffer.as_mut_ptr().add(8 * 1), (*S).h[1]);
        store64_le(buffer.as_mut_ptr().add(8 * 2), (*S).h[2]);
        store64_le(buffer.as_mut_ptr().add(8 * 3), (*S).h[3]);
        store64_le(buffer.as_mut_ptr().add(8 * 4), (*S).h[4]);
        store64_le(buffer.as_mut_ptr().add(8 * 5), (*S).h[5]);
        store64_le(buffer.as_mut_ptr().add(8 * 6), (*S).h[6]);
        store64_le(buffer.as_mut_ptr().add(8 * 7), (*S).h[7]);
    }
    /* outlen <= BLAKE2B_OUTBYTES (64) */
    unsafe { memcpy(out, buffer.as_ptr(), outlen as usize) };

    unsafe {
        sodium_memzero(
            (&raw mut (*S).h) as *mut c_void,
            core::mem::size_of::<[u64; 8]>(),
        );
        sodium_memzero(
            (&raw mut (*S).buf) as *mut c_void,
            core::mem::size_of::<[u8; 2 * 128]>(),
        );
    }

    0
}

/// `CRYPTO_ALIGN(64) blake2b_state S[1];`
#[repr(C, align(64))]
struct Aligned64State {
    s: blake2b_state,
}

#[inline(always)]
const fn aligned_state_new() -> Aligned64State {
    Aligned64State {
        s: blake2b_state {
            h: [0; 8],
            t: [0; 2],
            f: [0; 2],
            buf: [0; 2 * 128],
            buflen: 0,
            last_node: 0,
        },
    }
}

/* inlen, at least, should be uint64_t. Others can be size_t. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b(
    out: *mut u8,
    in_: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
) -> c_int {
    let mut state = aligned_state_new();
    let S: *mut blake2b_state = &mut state.s;

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if out.is_null() {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if (keylen as usize) > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen > 0 {
        if unsafe { _sodium_blake2b_init_key(S, outlen, key, keylen) } < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    } else {
        if unsafe { _sodium_blake2b_init(S, outlen) } < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }

    unsafe { _sodium_blake2b_update(S, in_ as *const u8, inlen) };
    unsafe { _sodium_blake2b_final(S, out, outlen) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_salt_personal(
    out: *mut u8,
    in_: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut state = aligned_state_new();
    let S: *mut blake2b_state = &mut state.s;

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if out.is_null() {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if outlen == 0 || (outlen as usize) > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if (keylen as usize) > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen > 0 {
        if unsafe {
            _sodium_blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal)
        } < 0
        {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    } else {
        if unsafe { _sodium_blake2b_init_salt_personal(S, outlen, salt, personal) } < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }

    unsafe { _sodium_blake2b_update(S, in_ as *const u8, inlen) };
    unsafe { _sodium_blake2b_final(S, out, outlen) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_pick_best_implementation() -> c_int {
    /* LCOV_EXCL_START */
    /* No AVX2 / SSE4.1 / SSSE3 intrinsics headers in this build. */
    unsafe { blake2b_compress = _sodium_blake2b_compress_ref };

    0
    /* LCOV_EXCL_STOP */
}

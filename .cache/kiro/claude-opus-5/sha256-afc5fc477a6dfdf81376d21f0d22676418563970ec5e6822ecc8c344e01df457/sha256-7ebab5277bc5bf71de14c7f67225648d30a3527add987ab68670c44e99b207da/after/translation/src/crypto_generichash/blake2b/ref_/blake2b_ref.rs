//! Translation of c_src/libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c

use crate::common::{load64_le, store32_le, store64_le};
use core::ffi::{c_int, c_void};

// blake2b_state, packed (see blake2.h `#pragma pack(push, 1)`).
#[repr(C, packed)]
struct blake2b_state {
    h: [u64; 8],
    t: [u64; 2],
    f: [u64; 2],
    buf: [u8; 2 * 128],
    buflen: usize,
    last_node: u8,
}

// blake2b_param, packed (see blake2.h `#pragma pack(push, 1)`), sizeof == 64.
#[repr(C, packed)]
struct blake2b_param {
    digest_length: u8,      /*  1 */
    key_length: u8,         /*  2 */
    fanout: u8,             /*  3 */
    depth: u8,              /*  4 */
    leaf_length: [u8; 4],   /*  8 */
    node_offset: [u8; 8],   /* 16 */
    node_depth: u8,         /* 17 */
    inner_length: u8,       /* 18 */
    reserved: [u8; 14],     /* 32 */
    salt: [u8; 16],         /* 48 */
    personal: [u8; 16],     /* 64 */
}

// enum blake2b_constant
const BLAKE2B_BLOCKBYTES: usize = 128;
const BLAKE2B_OUTBYTES: usize = 64;
const BLAKE2B_KEYBYTES: usize = 64;
const BLAKE2B_SALTBYTES: usize = 16;
const BLAKE2B_PERSONALBYTES: usize = 16;

// typedef int (*blake2b_compress_fn)(blake2b_state *S, const uint8_t block[BLAKE2B_BLOCKBYTES]);
type blake2b_compress_fn =
    unsafe extern "C" fn(S: *mut blake2b_state, block: *const u8) -> c_int;

extern "C" {
    // quirks.h: blake2b_compress_ref -> _sodium_blake2b_compress_ref
    fn _sodium_blake2b_compress_ref(S: *mut blake2b_state, block: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

// static blake2b_compress_fn blake2b_compress = blake2b_compress_ref;
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

/* LCOV_EXCL_START */
unsafe fn blake2b_set_lastnode(S: *mut blake2b_state) -> c_int {
    core::ptr::addr_of_mut!((*S).f[1]).write_unaligned(-1i64 as u64);
    0
}
/* LCOV_EXCL_STOP */

unsafe fn blake2b_is_lastblock(S: *const blake2b_state) -> c_int {
    (core::ptr::addr_of!((*S).f[0]).read_unaligned() != 0) as c_int
}

unsafe fn blake2b_set_lastblock(S: *mut blake2b_state) -> c_int {
    if (*S).last_node != 0 {
        blake2b_set_lastnode(S); /* LCOV_EXCL_LINE */
    }
    core::ptr::addr_of_mut!((*S).f[0]).write_unaligned(-1i64 as u64);
    0
}

unsafe fn blake2b_increment_counter(S: *mut blake2b_state, inc: u64) -> c_int {
    // HAVE_TI_MODE undefined: 64-bit two-limb variant
    let t0 = core::ptr::addr_of!((*S).t[0]).read_unaligned();
    let new_t0 = t0.wrapping_add(inc);
    core::ptr::addr_of_mut!((*S).t[0]).write_unaligned(new_t0);
    let t1 = core::ptr::addr_of!((*S).t[1]).read_unaligned();
    core::ptr::addr_of_mut!((*S).t[1]).write_unaligned(t1.wrapping_add((new_t0 < inc) as u64));
    0
}

/* Parameter-related functions */
unsafe fn blake2b_param_set_salt(P: *mut blake2b_param, salt: *const u8) -> c_int {
    core::ptr::copy_nonoverlapping(
        salt,
        core::ptr::addr_of_mut!((*P).salt) as *mut u8,
        BLAKE2B_SALTBYTES,
    );
    0
}

unsafe fn blake2b_param_set_personal(P: *mut blake2b_param, personal: *const u8) -> c_int {
    core::ptr::copy_nonoverlapping(
        personal,
        core::ptr::addr_of_mut!((*P).personal) as *mut u8,
        BLAKE2B_PERSONALBYTES,
    );
    0
}

unsafe fn blake2b_init0(S: *mut blake2b_state) -> c_int {
    let mut i: usize = 0;
    while i < 8 {
        core::ptr::addr_of_mut!((*S).h[i]).write_unaligned(blake2b_IV[i]);
        i += 1;
    }
    /* zero everything between .t and .last_node */
    // offsetof(blake2b_state, last_node) + sizeof(last_node) - offsetof(blake2b_state, t)
    let base = core::ptr::addr_of_mut!((*S).t) as *mut u8;
    let t_off = base as usize - (S as usize);
    let last_off = core::ptr::addr_of!((*S).last_node) as usize - (S as usize);
    let n = last_off + core::mem::size_of::<u8>() - t_off;
    core::ptr::write_bytes(base, 0, n);
    0
}

/* init xors IV with input parameter block */
// quirks.h: blake2b_init_param -> _sodium_blake2b_init_param
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_param(
    S: *mut blake2b_state,
    P: *const blake2b_param,
) -> c_int {
    // COMPILER_ASSERT(sizeof *P == 64);
    blake2b_init0(S);
    let p = P as *const u8;

    /* IV XOR ParamBlock */
    let mut i: usize = 0;
    while i < 8 {
        let hi = core::ptr::addr_of!((*S).h[i]).read_unaligned();
        core::ptr::addr_of_mut!((*S).h[i])
            .write_unaligned(hi ^ load64_le(p.add(core::mem::size_of::<u64>() * i)));
        i += 1;
    }
    0
}

// quirks.h: blake2b_init -> _sodium_blake2b_init
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init(S: *mut blake2b_state, outlen: u8) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = 0;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.salt) as *mut u8, 0, 16);
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.personal) as *mut u8, 0, 16);
    _sodium_blake2b_init_param(S, core::ptr::addr_of!(P))
}

// quirks.h: blake2b_init_salt_personal -> _sodium_blake2b_init_salt_personal
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = 0;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    if !salt.is_null() {
        blake2b_param_set_salt(core::ptr::addr_of_mut!(P), salt as *const u8);
    } else {
        core::ptr::write_bytes(core::ptr::addr_of_mut!(P.salt) as *mut u8, 0, 16);
    }
    if !personal.is_null() {
        blake2b_param_set_personal(core::ptr::addr_of_mut!(P), personal as *const u8);
    } else {
        core::ptr::write_bytes(core::ptr::addr_of_mut!(P.personal) as *mut u8, 0, 16);
    }
    _sodium_blake2b_init_param(S, core::ptr::addr_of!(P))
}

// quirks.h: blake2b_init_key -> _sodium_blake2b_init_key
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = keylen;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.salt) as *mut u8, 0, 16);
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.personal) as *mut u8, 0, 16);

    if _sodium_blake2b_init_param(S, core::ptr::addr_of!(P)) < 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    {
        let mut block: [u8; 128] = [0; 128];
        core::ptr::write_bytes(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES);
        core::ptr::copy_nonoverlapping(key as *const u8, block.as_mut_ptr(), keylen as usize);
        _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
    }
    0
}

// quirks.h: blake2b_init_key_salt_personal -> _sodium_blake2b_init_key_salt_personal
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = keylen;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    core::ptr::write_bytes(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    if !salt.is_null() {
        blake2b_param_set_salt(core::ptr::addr_of_mut!(P), salt as *const u8);
    } else {
        core::ptr::write_bytes(core::ptr::addr_of_mut!(P.salt) as *mut u8, 0, 16);
    }
    if !personal.is_null() {
        blake2b_param_set_personal(core::ptr::addr_of_mut!(P), personal as *const u8);
    } else {
        core::ptr::write_bytes(core::ptr::addr_of_mut!(P.personal) as *mut u8, 0, 16);
    }

    if _sodium_blake2b_init_param(S, core::ptr::addr_of!(P)) < 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    {
        let mut block: [u8; 128] = [0; 128];
        core::ptr::write_bytes(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES);
        core::ptr::copy_nonoverlapping(key as *const u8, block.as_mut_ptr(), keylen as usize);
        _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
    }
    0
}

/* inlen now in bytes */
// quirks.h: blake2b_update -> _sodium_blake2b_update
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_update(
    S: *mut blake2b_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    while inlen > 0 {
        let left = core::ptr::addr_of!((*S).buflen).read_unaligned();
        let fill = 2 * BLAKE2B_BLOCKBYTES - left;

        if inlen as usize > fill {
            /* memcpy(S->buf + left, in, fill) */
            core::ptr::copy_nonoverlapping(
                in_,
                (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left),
                fill,
            );
            let bl = core::ptr::addr_of!((*S).buflen).read_unaligned();
            core::ptr::addr_of_mut!((*S).buflen).write_unaligned(bl + fill);
            blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64);
            let buf_ptr = core::ptr::addr_of_mut!((*S).buf) as *mut u8;
            blake2b_compress(S, buf_ptr);
            core::ptr::copy_nonoverlapping(
                buf_ptr.add(BLAKE2B_BLOCKBYTES),
                buf_ptr,
                BLAKE2B_BLOCKBYTES,
            );
            let bl = core::ptr::addr_of!((*S).buflen).read_unaligned();
            core::ptr::addr_of_mut!((*S).buflen).write_unaligned(bl - BLAKE2B_BLOCKBYTES);
            in_ = in_.add(fill);
            inlen -= fill as u64;
        } else {
            core::ptr::copy_nonoverlapping(
                in_,
                (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left),
                inlen as usize,
            );
            let bl = core::ptr::addr_of!((*S).buflen).read_unaligned();
            core::ptr::addr_of_mut!((*S).buflen).write_unaligned(bl + inlen as usize);
            in_ = in_.add(inlen as usize);
            inlen -= inlen;
        }
    }

    0
}

// quirks.h: blake2b_final -> _sodium_blake2b_final
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_final(
    S: *mut blake2b_state,
    out: *mut u8,
    outlen: u8,
) -> c_int {
    let mut buffer: [u8; 64] = [0; 64];

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if blake2b_is_lastblock(S) != 0 {
        return -1;
    }
    let buf_ptr = core::ptr::addr_of_mut!((*S).buf) as *mut u8;
    if core::ptr::addr_of!((*S).buflen).read_unaligned() > BLAKE2B_BLOCKBYTES {
        blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64);
        blake2b_compress(S, buf_ptr);
        let bl = core::ptr::addr_of!((*S).buflen).read_unaligned();
        core::ptr::addr_of_mut!((*S).buflen).write_unaligned(bl - BLAKE2B_BLOCKBYTES);
        // assert(S->buflen <= BLAKE2B_BLOCKBYTES);
        let bl = core::ptr::addr_of!((*S).buflen).read_unaligned();
        core::ptr::copy_nonoverlapping(buf_ptr.add(BLAKE2B_BLOCKBYTES), buf_ptr, bl);
    }

    let bl = core::ptr::addr_of!((*S).buflen).read_unaligned();
    blake2b_increment_counter(S, bl as u64);
    blake2b_set_lastblock(S);
    core::ptr::write_bytes(buf_ptr.add(bl), 0, 2 * BLAKE2B_BLOCKBYTES - bl); /* Padding */
    blake2b_compress(S, buf_ptr);

    // COMPILER_ASSERT(sizeof buffer == 64U);
    let mut idx = 0;
    while idx < 8 {
        let hi = core::ptr::addr_of!((*S).h[idx]).read_unaligned();
        store64_le(buffer.as_mut_ptr().add(8 * idx), hi);
        idx += 1;
    }
    core::ptr::copy_nonoverlapping(buffer.as_ptr(), out, outlen as usize);

    sodium_memzero(
        core::ptr::addr_of_mut!((*S).h) as *mut c_void,
        core::mem::size_of::<[u64; 8]>(),
    );
    sodium_memzero(
        core::ptr::addr_of_mut!((*S).buf) as *mut c_void,
        core::mem::size_of::<[u8; 2 * 128]>(),
    );

    0
}

/* inlen, at least, should be uint64_t. Others can be size_t. */
// quirks.h: blake2b -> _sodium_blake2b
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b(
    out: *mut u8,
    in_: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
) -> c_int {
    // CRYPTO_ALIGN(64) blake2b_state S[1];
    let mut s: blake2b_state = core::mem::zeroed();
    let S = core::ptr::addr_of_mut!(s);

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if out.is_null() {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() && keylen > 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key(S, outlen, key, keylen) < 0 {
            crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    } else if _sodium_blake2b_init(S, outlen) < 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }

    _sodium_blake2b_update(S, in_ as *const u8, inlen);
    _sodium_blake2b_final(S, out, outlen);
    0
}

// quirks.h: blake2b_salt_personal -> _sodium_blake2b_salt_personal
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
    // CRYPTO_ALIGN(64) blake2b_state S[1];
    let mut s: blake2b_state = core::mem::zeroed();
    let S = core::ptr::addr_of_mut!(s);

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if out.is_null() {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() && keylen > 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal) < 0 {
            crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    } else if _sodium_blake2b_init_salt_personal(S, outlen, salt, personal) < 0 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }

    _sodium_blake2b_update(S, in_ as *const u8, inlen);
    _sodium_blake2b_final(S, out, outlen);
    0
}

// quirks.h: blake2b_pick_best_implementation -> _sodium_blake2b_pick_best_implementation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_pick_best_implementation() -> c_int {
    /* LCOV_EXCL_START */
    // HAVE_AVX2INTRIN_H / HAVE_SMMINTRIN_H / HAVE_TMMINTRIN_H / HAVE_EMMINTRIN_H
    // all undefined: only the reference implementation is compiled/selected.
    blake2b_compress = _sodium_blake2b_compress_ref;

    0
    /* LCOV_EXCL_STOP */
}

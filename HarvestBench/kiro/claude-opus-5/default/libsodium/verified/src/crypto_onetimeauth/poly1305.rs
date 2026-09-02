//! Translation of libsodium Poly1305 (donna 32-bit path) and the
//! `crypto_onetimeauth_poly1305` front-end.
//!
//! Combines:
//!   - crypto_onetimeauth/poly1305/onetimeauth_poly1305.c (+ .h)
//!   - crypto_onetimeauth/poly1305/donna/poly1305_donna.c (+ poly1305_donna.h,
//!     poly1305_donna32.h)  [HAVE_TI_MODE undefined -> 32-bit path]
//!   - include/sodium/crypto_onetimeauth_poly1305.h

use core::ffi::{c_char, c_int, c_void};

use crate::common::{load32_le, store32_le};
use crate::crypto_verify::crypto_verify_16;
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::sodium_memzero;

/* ------------------------------------------------------------------ */
/* crypto_onetimeauth_poly1305.h                                       */
/* ------------------------------------------------------------------ */

pub const crypto_onetimeauth_poly1305_BYTES: usize = 16;
pub const crypto_onetimeauth_poly1305_KEYBYTES: usize = 32;

/// `typedef struct CRYPTO_ALIGN(16) crypto_onetimeauth_poly1305_state { unsigned char opaque[256]; }`
#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct crypto_onetimeauth_poly1305_state {
    pub opaque: [u8; 256],
}

/* ------------------------------------------------------------------ */
/* onetimeauth_poly1305.h : the implementation vtable                  */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_onetimeauth_poly1305_implementation {
    pub onetimeauth: Option<
        unsafe extern "C" fn(
            out: *mut u8,
            in_: *const u8,
            inlen: u64,
            k: *const u8,
        ) -> c_int,
    >,
    pub onetimeauth_verify: Option<
        unsafe extern "C" fn(
            h: *const u8,
            in_: *const u8,
            inlen: u64,
            k: *const u8,
        ) -> c_int,
    >,
    pub onetimeauth_init: Option<
        unsafe extern "C" fn(
            state: *mut crypto_onetimeauth_poly1305_state,
            key: *const u8,
        ) -> c_int,
    >,
    pub onetimeauth_update: Option<
        unsafe extern "C" fn(
            state: *mut crypto_onetimeauth_poly1305_state,
            in_: *const u8,
            inlen: u64,
        ) -> c_int,
    >,
    pub onetimeauth_final: Option<
        unsafe extern "C" fn(
            state: *mut crypto_onetimeauth_poly1305_state,
            out: *mut u8,
        ) -> c_int,
    >,
}

unsafe impl Sync for crypto_onetimeauth_poly1305_implementation {}

/* ================================================================== */
/* donna/poly1305_donna32.h : 32-bit poly1305 core                     */
/* ================================================================== */

const poly1305_block_size: usize = 16;

/* 17 + sizeof(unsigned long long) + 14*sizeof(unsigned long)
 * Note LP64: `unsigned long` == u64, `unsigned long long` == u64. */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct poly1305_state_internal_t {
    pub r: [u64; 5],
    pub h: [u64; 5],
    pub pad: [u64; 4],
    pub leftover: u64,
    pub buffer: [u8; poly1305_block_size],
    pub final_: u8,
}

pub unsafe fn poly1305_init(st: *mut poly1305_state_internal_t, key: *const u8) {
    /* r &= 0xffffffc0ffffffc0ffffffc0fffffff - wiped after finalization */
    (*st).r[0] = (load32_le(key.add(0)) as u64) & 0x3ffffff;
    (*st).r[1] = ((load32_le(key.add(3)) >> 2) as u64) & 0x3ffff03;
    (*st).r[2] = ((load32_le(key.add(6)) >> 4) as u64) & 0x3ffc0ff;
    (*st).r[3] = ((load32_le(key.add(9)) >> 6) as u64) & 0x3f03fff;
    (*st).r[4] = ((load32_le(key.add(12)) >> 8) as u64) & 0x00fffff;

    /* h = 0 */
    (*st).h[0] = 0;
    (*st).h[1] = 0;
    (*st).h[2] = 0;
    (*st).h[3] = 0;
    (*st).h[4] = 0;

    /* save pad for later */
    (*st).pad[0] = load32_le(key.add(16)) as u64;
    (*st).pad[1] = load32_le(key.add(20)) as u64;
    (*st).pad[2] = load32_le(key.add(24)) as u64;
    (*st).pad[3] = load32_le(key.add(28)) as u64;

    (*st).leftover = 0;
    (*st).final_ = 0;
}

pub unsafe fn poly1305_blocks(
    st: *mut poly1305_state_internal_t,
    mut m: *const u8,
    mut bytes: u64,
) {
    let hibit: u64 = if (*st).final_ != 0 { 0u64 } else { 1u64 << 24 }; /* 1 << 128 */
    let r0: u64;
    let r1: u64;
    let r2: u64;
    let r3: u64;
    let r4: u64;
    let s1: u64;
    let s2: u64;
    let s3: u64;
    let s4: u64;
    let mut h0: u64;
    let mut h1: u64;
    let mut h2: u64;
    let mut h3: u64;
    let mut h4: u64;
    let mut d0: u64;
    let mut d1: u64;
    let mut d2: u64;
    let mut d3: u64;
    let mut d4: u64;
    let mut c: u64;

    r0 = (*st).r[0];
    r1 = (*st).r[1];
    r2 = (*st).r[2];
    r3 = (*st).r[3];
    r4 = (*st).r[4];

    s1 = r1.wrapping_mul(5);
    s2 = r2.wrapping_mul(5);
    s3 = r3.wrapping_mul(5);
    s4 = r4.wrapping_mul(5);

    h0 = (*st).h[0];
    h1 = (*st).h[1];
    h2 = (*st).h[2];
    h3 = (*st).h[3];
    h4 = (*st).h[4];

    while bytes >= poly1305_block_size as u64 {
        /* h += m[i] */
        h0 = h0.wrapping_add((load32_le(m.add(0)) as u64) & 0x3ffffff);
        h1 = h1.wrapping_add(((load32_le(m.add(3)) >> 2) as u64) & 0x3ffffff);
        h2 = h2.wrapping_add(((load32_le(m.add(6)) >> 4) as u64) & 0x3ffffff);
        h3 = h3.wrapping_add(((load32_le(m.add(9)) >> 6) as u64) & 0x3ffffff);
        h4 = h4.wrapping_add(((load32_le(m.add(12)) >> 8) as u64) | hibit);

        /* h *= r */
        d0 = (h0.wrapping_mul(r0))
            .wrapping_add(h1.wrapping_mul(s4))
            .wrapping_add(h2.wrapping_mul(s3))
            .wrapping_add(h3.wrapping_mul(s2))
            .wrapping_add(h4.wrapping_mul(s1));
        d1 = (h0.wrapping_mul(r1))
            .wrapping_add(h1.wrapping_mul(r0))
            .wrapping_add(h2.wrapping_mul(s4))
            .wrapping_add(h3.wrapping_mul(s3))
            .wrapping_add(h4.wrapping_mul(s2));
        d2 = (h0.wrapping_mul(r2))
            .wrapping_add(h1.wrapping_mul(r1))
            .wrapping_add(h2.wrapping_mul(r0))
            .wrapping_add(h3.wrapping_mul(s4))
            .wrapping_add(h4.wrapping_mul(s3));
        d3 = (h0.wrapping_mul(r3))
            .wrapping_add(h1.wrapping_mul(r2))
            .wrapping_add(h2.wrapping_mul(r1))
            .wrapping_add(h3.wrapping_mul(r0))
            .wrapping_add(h4.wrapping_mul(s4));
        d4 = (h0.wrapping_mul(r4))
            .wrapping_add(h1.wrapping_mul(r3))
            .wrapping_add(h2.wrapping_mul(r2))
            .wrapping_add(h3.wrapping_mul(r1))
            .wrapping_add(h4.wrapping_mul(r0));

        /* (partial) h %= p */
        c = d0 >> 26;
        h0 = d0 & 0x3ffffff;
        d1 = d1.wrapping_add(c);
        c = d1 >> 26;
        h1 = d1 & 0x3ffffff;
        d2 = d2.wrapping_add(c);
        c = d2 >> 26;
        h2 = d2 & 0x3ffffff;
        d3 = d3.wrapping_add(c);
        c = d3 >> 26;
        h3 = d3 & 0x3ffffff;
        d4 = d4.wrapping_add(c);
        c = d4 >> 26;
        h4 = d4 & 0x3ffffff;
        h0 = h0.wrapping_add(c.wrapping_mul(5));
        c = h0 >> 26;
        h0 &= 0x3ffffff;
        h1 = h1.wrapping_add(c);

        m = m.add(poly1305_block_size);
        bytes -= poly1305_block_size as u64;
    }

    (*st).h[0] = h0;
    (*st).h[1] = h1;
    (*st).h[2] = h2;
    (*st).h[3] = h3;
    (*st).h[4] = h4;
}

#[inline(never)]
pub unsafe fn poly1305_finish(st: *mut poly1305_state_internal_t, mac: *mut u8) {
    let mut h0: u64;
    let mut h1: u64;
    let mut h2: u64;
    let mut h3: u64;
    let mut h4: u64;
    let mut c: u64;
    let mut g0: u64;
    let mut g1: u64;
    let mut g2: u64;
    let mut g3: u64;
    let mut g4: u64;
    let mut f: u64;
    let mut mask: u64;

    /* process the remaining block */
    if (*st).leftover != 0 {
        let mut i: u64 = (*st).leftover;

        (*st).buffer[i as usize] = 1;
        i += 1;
        while i < poly1305_block_size as u64 {
            (*st).buffer[i as usize] = 0;
            i += 1;
        }
        (*st).final_ = 1;
        let buf = (*st).buffer.as_ptr();
        poly1305_blocks(st, buf, poly1305_block_size as u64);
    }

    /* fully carry h */
    h0 = (*st).h[0];
    h1 = (*st).h[1];
    h2 = (*st).h[2];
    h3 = (*st).h[3];
    h4 = (*st).h[4];

    c = h1 >> 26;
    h1 = h1 & 0x3ffffff;
    h2 = h2.wrapping_add(c);
    c = h2 >> 26;
    h2 = h2 & 0x3ffffff;
    h3 = h3.wrapping_add(c);
    c = h3 >> 26;
    h3 = h3 & 0x3ffffff;
    h4 = h4.wrapping_add(c);
    c = h4 >> 26;
    h4 = h4 & 0x3ffffff;
    h0 = h0.wrapping_add(c.wrapping_mul(5));
    c = h0 >> 26;
    h0 = h0 & 0x3ffffff;
    h1 = h1.wrapping_add(c);

    /* compute h + -p */
    g0 = h0.wrapping_add(5);
    c = g0 >> 26;
    g0 &= 0x3ffffff;
    g1 = h1.wrapping_add(c);
    c = g1 >> 26;
    g1 &= 0x3ffffff;
    g2 = h2.wrapping_add(c);
    c = g2 >> 26;
    g2 &= 0x3ffffff;
    g3 = h3.wrapping_add(c);
    c = g3 >> 26;
    g3 &= 0x3ffffff;
    g4 = h4.wrapping_add(c).wrapping_sub(1u64 << 26);

    /* select h if h < p, or h + -p if h >= p */
    mask = (g4 >> ((core::mem::size_of::<u64>() * 8) - 1)).wrapping_sub(1);
    g0 &= mask;
    g1 &= mask;
    g2 &= mask;
    g3 &= mask;
    g4 &= mask;
    mask = !mask;

    h0 = (h0 & mask) | g0;
    h1 = (h1 & mask) | g1;
    h2 = (h2 & mask) | g2;
    h3 = (h3 & mask) | g3;
    h4 = (h4 & mask) | g4;

    /* h = h % (2^128) */
    h0 = ((h0) | (h1 << 26)) & 0xffffffff;
    h1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffff;
    h2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffff;
    h3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffff;

    /* mac = (h + pad) % (2^128) */
    f = (h0 as u64).wrapping_add((*st).pad[0]);
    h0 = f;
    f = (h1 as u64).wrapping_add((*st).pad[1]).wrapping_add(f >> 32);
    h1 = f;
    f = (h2 as u64).wrapping_add((*st).pad[2]).wrapping_add(f >> 32);
    h2 = f;
    f = (h3 as u64).wrapping_add((*st).pad[3]).wrapping_add(f >> 32);
    h3 = f;

    store32_le(mac.add(0), h0 as u32);
    store32_le(mac.add(4), h1 as u32);
    store32_le(mac.add(8), h2 as u32);
    store32_le(mac.add(12), h3 as u32);

    /* zero out the state */
    sodium_memzero(
        st as *mut c_void,
        core::mem::size_of::<poly1305_state_internal_t>(),
    );
}

/* ================================================================== */
/* donna/poly1305_donna.c                                              */
/* ================================================================== */

unsafe fn poly1305_update(
    st: *mut poly1305_state_internal_t,
    mut m: *const u8,
    mut bytes: u64,
) {
    let mut i: u64;

    /* handle leftover */
    if (*st).leftover != 0 {
        let mut want: u64 = poly1305_block_size as u64 - (*st).leftover;

        if want > bytes {
            want = bytes;
        }
        i = 0;
        while i < want {
            (*st).buffer[((*st).leftover + i) as usize] = *m.add(i as usize);
            i += 1;
        }
        bytes -= want;
        m = m.add(want as usize);
        (*st).leftover += want;
        if (*st).leftover < poly1305_block_size as u64 {
            return;
        }
        let buf = (*st).buffer.as_ptr();
        poly1305_blocks(st, buf, poly1305_block_size as u64);
        (*st).leftover = 0;
    }

    /* process full blocks */
    if bytes >= poly1305_block_size as u64 {
        let want: u64 = bytes & !(poly1305_block_size as u64 - 1);

        poly1305_blocks(st, m, want);
        m = m.add(want as usize);
        bytes -= want;
    }

    /* store leftover */
    if bytes != 0 {
        i = 0;
        while i < bytes {
            (*st).buffer[((*st).leftover + i) as usize] = *m.add(i as usize);
            i += 1;
        }
        (*st).leftover += bytes;
    }
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna(
    out: *mut u8,
    m: *const u8,
    inlen: u64,
    key: *const u8,
) -> c_int {
    let mut state: poly1305_state_internal_t = core::mem::zeroed();

    poly1305_init(&mut state, key);
    poly1305_update(&mut state, m, inlen);
    poly1305_finish(&mut state, out);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> c_int {
    /* COMPILER_ASSERT(sizeof(crypto_onetimeauth_poly1305_state) >=
       sizeof(poly1305_state_internal_t)); */
    const _: () = assert!(
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>()
            >= core::mem::size_of::<poly1305_state_internal_t>()
    );
    poly1305_init(state as *mut c_void as *mut poly1305_state_internal_t, key);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    poly1305_update(
        state as *mut c_void as *mut poly1305_state_internal_t,
        in_,
        inlen,
    );

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> c_int {
    poly1305_finish(
        state as *mut c_void as *mut poly1305_state_internal_t,
        out,
    );

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 16] = [0; 16];

    crypto_onetimeauth_poly1305_donna(correct.as_mut_ptr(), in_, inlen, k);

    crypto_verify_16(h, correct.as_ptr())
}

#[unsafe(no_mangle)]
pub static crypto_onetimeauth_poly1305_donna_implementation:
    crypto_onetimeauth_poly1305_implementation =
    crypto_onetimeauth_poly1305_implementation {
        onetimeauth: Some(crypto_onetimeauth_poly1305_donna),
        onetimeauth_verify: Some(crypto_onetimeauth_poly1305_donna_verify),
        onetimeauth_init: Some(crypto_onetimeauth_poly1305_donna_init),
        onetimeauth_update: Some(crypto_onetimeauth_poly1305_donna_update),
        onetimeauth_final: Some(crypto_onetimeauth_poly1305_donna_final),
    };

/* ================================================================== */
/* onetimeauth_poly1305.c                                              */
/* ================================================================== */

static mut implementation: *const crypto_onetimeauth_poly1305_implementation =
    &crypto_onetimeauth_poly1305_donna_implementation;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    ((*implementation).onetimeauth.unwrap())(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    ((*implementation).onetimeauth_verify.unwrap())(h, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> c_int {
    ((*implementation).onetimeauth_init.unwrap())(state, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    ((*implementation).onetimeauth_update.unwrap())(state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> c_int {
    ((*implementation).onetimeauth_final.unwrap())(state, out)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_poly1305_bytes() -> usize {
    crypto_onetimeauth_poly1305_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_poly1305_keybytes() -> usize {
    crypto_onetimeauth_poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_onetimeauth_poly1305_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_onetimeauth_poly1305_KEYBYTES);
}

#[unsafe(no_mangle)]
pub extern "C" fn _crypto_onetimeauth_poly1305_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &crypto_onetimeauth_poly1305_donna_implementation;
    }
    0
}

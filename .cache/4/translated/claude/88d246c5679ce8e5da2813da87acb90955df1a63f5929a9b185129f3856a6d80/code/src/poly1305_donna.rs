//! Translation of `crypto_onetimeauth/poly1305/donna/poly1305_donna.c`.
//!
//! `HAVE_TI_MODE` is **not** defined in the reference build, so the file
//! includes `poly1305_donna32.h` (the 32x32->64 multiplication variant), whose
//! `static` helpers (`poly1305_init`, `poly1305_blocks`, `poly1305_finish`) are
//! inlined into this module.
//!
//! Note that `poly1305_donna32.h` uses `unsigned long` for `r`/`h`/`pad`/the
//! intermediate limbs, which is a **64-bit** type on the x86-64 Linux target.
//! All of that arithmetic is therefore reproduced in `u64`, exactly as the C
//! compiler would do it (in particular the `mask` computation shifts right by
//! `sizeof(unsigned long) * 8 - 1 == 63`).

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};
use core::ptr::addr_of;

extern "C" {
    /// Defined in `crypto_verify/verify.c`.
    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    /// Defined in `sodium/utils.c`.
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/// `crypto_onetimeauth_poly1305_state` from
/// `include/sodium/crypto_onetimeauth_poly1305.h`:
/// `typedef struct CRYPTO_ALIGN(16) { unsigned char opaque[256]; }`.
#[repr(C, align(16))]
pub struct crypto_onetimeauth_poly1305_state {
    pub opaque: [u8; 256],
}

/// `typedef struct crypto_onetimeauth_poly1305_implementation` from
/// `crypto_onetimeauth/poly1305/onetimeauth_poly1305.h` (5 function pointers,
/// 0x28 bytes).
#[repr(C)]
pub struct crypto_onetimeauth_poly1305_implementation {
    pub onetimeauth: unsafe extern "C" fn(
        out: *mut u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int,
    pub onetimeauth_verify: unsafe extern "C" fn(
        h: *const u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int,
    pub onetimeauth_init: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int,
    pub onetimeauth_update: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int,
    pub onetimeauth_final: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int,
}

/* #define poly1305_block_size 16 */
const poly1305_block_size: usize = 16;

/// `poly1305_state_internal_t` from `poly1305_donna32.h`.
///
/// ```c
/// typedef struct poly1305_state_internal_t {
///     unsigned long      r[5];
///     unsigned long      h[5];
///     unsigned long      pad[4];
///     unsigned long long leftover;
///     unsigned char      buffer[poly1305_block_size];
///     unsigned char      final;
/// } poly1305_state_internal_t;
/// ```
///
/// 144 bytes, alignment 8 — comfortably below the 256 bytes of
/// `crypto_onetimeauth_poly1305_state` (the C `COMPILER_ASSERT`).
#[repr(C)]
struct poly1305_state_internal_t {
    r: [u64; 5],
    h: [u64; 5],
    pad: [u64; 4],
    leftover: c_ulonglong,
    buffer: [u8; poly1305_block_size],
    final_: u8,
}

unsafe fn poly1305_init(st: *mut poly1305_state_internal_t, key: *const u8) {
    /* r &= 0xffffffc0ffffffc0ffffffc0fffffff - wiped after finalization */
    (*st).r[0] = (load32_le(key.add(0)) & 0x3ffffff) as u64;
    (*st).r[1] = ((load32_le(key.add(3)) >> 2) & 0x3ffff03) as u64;
    (*st).r[2] = ((load32_le(key.add(6)) >> 4) & 0x3ffc0ff) as u64;
    (*st).r[3] = ((load32_le(key.add(9)) >> 6) & 0x3f03fff) as u64;
    (*st).r[4] = ((load32_le(key.add(12)) >> 8) & 0x00fffff) as u64;

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

unsafe fn poly1305_blocks(
    st: *mut poly1305_state_internal_t,
    mut m: *const u8,
    mut bytes: c_ulonglong,
) {
    let hibit: u64 = if (*st).final_ != 0 { 0 } else { 1u64 << 24 }; /* 1 << 128 */

    let r0: u64 = (*st).r[0];
    let r1: u64 = (*st).r[1];
    let r2: u64 = (*st).r[2];
    let r3: u64 = (*st).r[3];
    let r4: u64 = (*st).r[4];

    let s1: u64 = r1.wrapping_mul(5);
    let s2: u64 = r2.wrapping_mul(5);
    let s3: u64 = r3.wrapping_mul(5);
    let s4: u64 = r4.wrapping_mul(5);

    let mut h0: u64 = (*st).h[0];
    let mut h1: u64 = (*st).h[1];
    let mut h2: u64 = (*st).h[2];
    let mut h3: u64 = (*st).h[3];
    let mut h4: u64 = (*st).h[4];

    while bytes >= poly1305_block_size as c_ulonglong {
        /* h += m[i] */
        h0 = h0.wrapping_add((load32_le(m.add(0)) & 0x3ffffff) as u64);
        h1 = h1.wrapping_add(((load32_le(m.add(3)) >> 2) & 0x3ffffff) as u64);
        h2 = h2.wrapping_add(((load32_le(m.add(6)) >> 4) & 0x3ffffff) as u64);
        h3 = h3.wrapping_add(((load32_le(m.add(9)) >> 6) & 0x3ffffff) as u64);
        h4 = h4.wrapping_add(((load32_le(m.add(12)) >> 8) as u64) | hibit);

        /* h *= r */
        let d0: u64 = h0
            .wrapping_mul(r0)
            .wrapping_add(h1.wrapping_mul(s4))
            .wrapping_add(h2.wrapping_mul(s3))
            .wrapping_add(h3.wrapping_mul(s2))
            .wrapping_add(h4.wrapping_mul(s1));
        let mut d1: u64 = h0
            .wrapping_mul(r1)
            .wrapping_add(h1.wrapping_mul(r0))
            .wrapping_add(h2.wrapping_mul(s4))
            .wrapping_add(h3.wrapping_mul(s3))
            .wrapping_add(h4.wrapping_mul(s2));
        let mut d2: u64 = h0
            .wrapping_mul(r2)
            .wrapping_add(h1.wrapping_mul(r1))
            .wrapping_add(h2.wrapping_mul(r0))
            .wrapping_add(h3.wrapping_mul(s4))
            .wrapping_add(h4.wrapping_mul(s3));
        let mut d3: u64 = h0
            .wrapping_mul(r3)
            .wrapping_add(h1.wrapping_mul(r2))
            .wrapping_add(h2.wrapping_mul(r1))
            .wrapping_add(h3.wrapping_mul(r0))
            .wrapping_add(h4.wrapping_mul(s4));
        let mut d4: u64 = h0
            .wrapping_mul(r4)
            .wrapping_add(h1.wrapping_mul(r3))
            .wrapping_add(h2.wrapping_mul(r2))
            .wrapping_add(h3.wrapping_mul(r1))
            .wrapping_add(h4.wrapping_mul(r0));

        /* (partial) h %= p */
        let mut c: u64;
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
        bytes = bytes.wrapping_sub(poly1305_block_size as c_ulonglong);
    }

    (*st).h[0] = h0;
    (*st).h[1] = h1;
    (*st).h[2] = h2;
    (*st).h[3] = h3;
    (*st).h[4] = h4;
}

/* static POLY1305_NOINLINE void poly1305_finish(...) */
#[inline(never)]
unsafe fn poly1305_finish(st: *mut poly1305_state_internal_t, mac: *mut u8) {
    let mut c: u64;
    let mut f: u64;
    let mut mask: u64;

    /* process the remaining block */
    if (*st).leftover != 0 {
        let mut i: c_ulonglong = (*st).leftover;

        (*st).buffer[i as usize] = 1;
        i = i.wrapping_add(1);
        while i < poly1305_block_size as c_ulonglong {
            (*st).buffer[i as usize] = 0;
            i = i.wrapping_add(1);
        }
        (*st).final_ = 1;
        poly1305_blocks(
            st,
            addr_of!((*st).buffer) as *const u8,
            poly1305_block_size as c_ulonglong,
        );
    }

    /* fully carry h */
    let mut h0: u64 = (*st).h[0];
    let mut h1: u64 = (*st).h[1];
    let mut h2: u64 = (*st).h[2];
    let mut h3: u64 = (*st).h[3];
    let mut h4: u64 = (*st).h[4];

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
    let mut g0: u64 = h0.wrapping_add(5);
    c = g0 >> 26;
    g0 &= 0x3ffffff;
    let mut g1: u64 = h1.wrapping_add(c);
    c = g1 >> 26;
    g1 &= 0x3ffffff;
    let mut g2: u64 = h2.wrapping_add(c);
    c = g2 >> 26;
    g2 &= 0x3ffffff;
    let mut g3: u64 = h3.wrapping_add(c);
    c = g3 >> 26;
    g3 &= 0x3ffffff;
    let mut g4: u64 = h4.wrapping_add(c).wrapping_sub(1u64 << 26);

    /* select h if h < p, or h + -p if h >= p */
    /* mask = (g4 >> ((sizeof(unsigned long) * 8) - 1)) - 1;  ==  >> 63 */
    mask = (g4 >> (core::mem::size_of::<u64>() * 8 - 1)).wrapping_sub(1);
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
    h0 = (h0 | (h1 << 26)) & 0xffffffff;
    h1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffff;
    h2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffff;
    h3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffff;

    /* mac = (h + pad) % (2^128) */
    f = h0.wrapping_add((*st).pad[0]);
    h0 = f;
    f = h1.wrapping_add((*st).pad[1]).wrapping_add(f >> 32);
    h1 = f;
    f = h2.wrapping_add((*st).pad[2]).wrapping_add(f >> 32);
    h2 = f;
    f = h3.wrapping_add((*st).pad[3]).wrapping_add(f >> 32);
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

unsafe fn poly1305_update(
    st: *mut poly1305_state_internal_t,
    mut m: *const u8,
    mut bytes: c_ulonglong,
) {
    let mut i: c_ulonglong;

    /* handle leftover */
    if (*st).leftover != 0 {
        let mut want: c_ulonglong =
            (poly1305_block_size as c_ulonglong).wrapping_sub((*st).leftover);

        if want > bytes {
            want = bytes;
        }
        i = 0;
        while i < want {
            (*st).buffer[((*st).leftover.wrapping_add(i)) as usize] = *m.add(i as usize);
            i = i.wrapping_add(1);
        }
        bytes = bytes.wrapping_sub(want);
        m = m.add(want as usize);
        (*st).leftover = (*st).leftover.wrapping_add(want);
        if (*st).leftover < poly1305_block_size as c_ulonglong {
            return;
        }
        poly1305_blocks(
            st,
            addr_of!((*st).buffer) as *const u8,
            poly1305_block_size as c_ulonglong,
        );
        (*st).leftover = 0;
    }

    /* process full blocks */
    if bytes >= poly1305_block_size as c_ulonglong {
        let want: c_ulonglong = bytes & !(poly1305_block_size as c_ulonglong - 1);

        poly1305_blocks(st, m, want);
        m = m.add(want as usize);
        bytes = bytes.wrapping_sub(want);
    }

    /* store leftover */
    if bytes != 0 {
        i = 0;
        while i < bytes {
            (*st).buffer[((*st).leftover.wrapping_add(i)) as usize] = *m.add(i as usize);
            i = i.wrapping_add(1);
        }
        (*st).leftover = (*st).leftover.wrapping_add(bytes);
    }
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna(
    out: *mut u8,
    m: *const u8,
    inlen: c_ulonglong,
    key: *const u8,
) -> c_int {
    /* CRYPTO_ALIGN(64) poly1305_state_internal_t state; -- the alignment is a
     * pure performance hint and is not observable, so it is not reproduced. */
    let mut state = poly1305_state_internal_t {
        r: [0; 5],
        h: [0; 5],
        pad: [0; 4],
        leftover: 0,
        buffer: [0; poly1305_block_size],
        final_: 0,
    };

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
     *                 sizeof(poly1305_state_internal_t)); */
    const _: () = assert!(
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>()
            >= core::mem::size_of::<poly1305_state_internal_t>()
    );
    poly1305_init(state as *mut poly1305_state_internal_t, key);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    poly1305_update(state as *mut poly1305_state_internal_t, in_, inlen);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> c_int {
    poly1305_finish(state as *mut poly1305_state_internal_t, out);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_verify(
    h: *const u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 16] = [0; 16];

    crypto_onetimeauth_poly1305_donna(correct.as_mut_ptr(), in_, inlen, k);

    crypto_verify_16(h, correct.as_ptr())
}

#[unsafe(no_mangle)]
pub static crypto_onetimeauth_poly1305_donna_implementation:
    crypto_onetimeauth_poly1305_implementation = crypto_onetimeauth_poly1305_implementation {
    onetimeauth: crypto_onetimeauth_poly1305_donna,
    onetimeauth_verify: crypto_onetimeauth_poly1305_donna_verify,
    onetimeauth_init: crypto_onetimeauth_poly1305_donna_init,
    onetimeauth_update: crypto_onetimeauth_poly1305_donna_update,
    onetimeauth_final: crypto_onetimeauth_poly1305_donna_final,
};

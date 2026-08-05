//! Translated from crypto_onetimeauth/poly1305/{onetimeauth_poly1305.c, donna/*}
use crate::primitives::cutil::*;
use core::ffi::c_void;

const POLY1305_BLOCK_SIZE: usize = 16;

#[repr(C)]
pub struct crypto_onetimeauth_poly1305_state {
    pub opaque: [u8; 256],
}

#[repr(C)]
pub struct crypto_onetimeauth_poly1305_implementation {
    pub onetimeauth: unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32,
    pub onetimeauth_verify: unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32,
    pub onetimeauth_init:
        unsafe extern "C" fn(*mut crypto_onetimeauth_poly1305_state, *const u8) -> i32,
    pub onetimeauth_update: unsafe extern "C" fn(
        *mut crypto_onetimeauth_poly1305_state,
        *const u8,
        u64,
    ) -> i32,
    pub onetimeauth_final:
        unsafe extern "C" fn(*mut crypto_onetimeauth_poly1305_state, *mut u8) -> i32,
}
unsafe impl Sync for crypto_onetimeauth_poly1305_implementation {}

// Actual state matching C's poly1305_state_internal_t (LP64: unsigned long = 8 bytes)
#[repr(C)]
struct St {
    r: [u64; 5],
    h: [u64; 5],
    pad: [u64; 4],
    leftover: u64,
    buffer: [u8; POLY1305_BLOCK_SIZE],
    final_: u8,
}

unsafe fn poly1305_init(st: *mut St, key: *const u8) {
    let s = &mut *st;
    s.r[0] = (load32_le(key.add(0)) & 0x3ffffff) as u64;
    s.r[1] = ((load32_le(key.add(3)) >> 2) & 0x3ffff03) as u64;
    s.r[2] = ((load32_le(key.add(6)) >> 4) & 0x3ffc0ff) as u64;
    s.r[3] = ((load32_le(key.add(9)) >> 6) & 0x3f03fff) as u64;
    s.r[4] = ((load32_le(key.add(12)) >> 8) & 0x00fffff) as u64;

    s.h = [0; 5];

    s.pad[0] = load32_le(key.add(16)) as u64;
    s.pad[1] = load32_le(key.add(20)) as u64;
    s.pad[2] = load32_le(key.add(24)) as u64;
    s.pad[3] = load32_le(key.add(28)) as u64;

    s.leftover = 0;
    s.final_ = 0;
}

unsafe fn poly1305_blocks(st: *mut St, mut m: *const u8, mut bytes: u64) {
    let s = &mut *st;
    let hibit: u64 = if s.final_ != 0 { 0 } else { 1u64 << 24 };
    let r0 = s.r[0];
    let r1 = s.r[1];
    let r2 = s.r[2];
    let r3 = s.r[3];
    let r4 = s.r[4];

    let s1 = r1 * 5;
    let s2 = r2 * 5;
    let s3 = r3 * 5;
    let s4 = r4 * 5;

    let mut h0 = s.h[0];
    let mut h1 = s.h[1];
    let mut h2 = s.h[2];
    let mut h3 = s.h[3];
    let mut h4 = s.h[4];

    while bytes >= POLY1305_BLOCK_SIZE as u64 {
        h0 = h0.wrapping_add((load32_le(m.add(0)) & 0x3ffffff) as u64);
        h1 = h1.wrapping_add(((load32_le(m.add(3)) >> 2) & 0x3ffffff) as u64);
        h2 = h2.wrapping_add(((load32_le(m.add(6)) >> 4) & 0x3ffffff) as u64);
        h3 = h3.wrapping_add(((load32_le(m.add(9)) >> 6) & 0x3ffffff) as u64);
        h4 = h4.wrapping_add(((load32_le(m.add(12)) >> 8) as u64) | hibit);

        // These products fit in u64 (values < 2^26 * 2^26-ish * small); C uses
        // unsigned long long accumulation with wrapping semantics.
        let d0 = (h0.wrapping_mul(r0))
            .wrapping_add(h1.wrapping_mul(s4))
            .wrapping_add(h2.wrapping_mul(s3))
            .wrapping_add(h3.wrapping_mul(s2))
            .wrapping_add(h4.wrapping_mul(s1));
        let mut d1 = (h0.wrapping_mul(r1))
            .wrapping_add(h1.wrapping_mul(r0))
            .wrapping_add(h2.wrapping_mul(s4))
            .wrapping_add(h3.wrapping_mul(s3))
            .wrapping_add(h4.wrapping_mul(s2));
        let mut d2 = (h0.wrapping_mul(r2))
            .wrapping_add(h1.wrapping_mul(r1))
            .wrapping_add(h2.wrapping_mul(r0))
            .wrapping_add(h3.wrapping_mul(s4))
            .wrapping_add(h4.wrapping_mul(s3));
        let mut d3 = (h0.wrapping_mul(r3))
            .wrapping_add(h1.wrapping_mul(r2))
            .wrapping_add(h2.wrapping_mul(r1))
            .wrapping_add(h3.wrapping_mul(r0))
            .wrapping_add(h4.wrapping_mul(s4));
        let mut d4 = (h0.wrapping_mul(r4))
            .wrapping_add(h1.wrapping_mul(r3))
            .wrapping_add(h2.wrapping_mul(r2))
            .wrapping_add(h3.wrapping_mul(r1))
            .wrapping_add(h4.wrapping_mul(r0));

        let mut c: u64 = d0 >> 26;
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

        m = m.add(POLY1305_BLOCK_SIZE);
        bytes -= POLY1305_BLOCK_SIZE as u64;
    }

    s.h[0] = h0;
    s.h[1] = h1;
    s.h[2] = h2;
    s.h[3] = h3;
    s.h[4] = h4;
}

unsafe fn poly1305_update(st: *mut St, mut m: *const u8, mut bytes: u64) {
    let s = &mut *st;
    if s.leftover != 0 {
        let mut want = POLY1305_BLOCK_SIZE as u64 - s.leftover;
        if want > bytes {
            want = bytes;
        }
        for i in 0..want {
            s.buffer[(s.leftover + i) as usize] = *m.add(i as usize);
        }
        bytes -= want;
        m = m.add(want as usize);
        s.leftover += want;
        if s.leftover < POLY1305_BLOCK_SIZE as u64 {
            return;
        }
        let bufp = s.buffer.as_ptr();
        poly1305_blocks(st, bufp, POLY1305_BLOCK_SIZE as u64);
        (*st).leftover = 0;
    }

    let s = &mut *st;
    if bytes >= POLY1305_BLOCK_SIZE as u64 {
        let want = bytes & !(POLY1305_BLOCK_SIZE as u64 - 1);
        poly1305_blocks(st, m, want);
        m = m.add(want as usize);
        bytes -= want;
    }

    let s = &mut *st;
    if bytes != 0 {
        for i in 0..bytes {
            s.buffer[(s.leftover + i) as usize] = *m.add(i as usize);
        }
        s.leftover += bytes;
    }
    let _ = s;
}

unsafe fn poly1305_finish(st: *mut St, mac: *mut u8) {
    let s = &mut *st;
    if s.leftover != 0 {
        let mut i = s.leftover;
        s.buffer[i as usize] = 1;
        i += 1;
        while i < POLY1305_BLOCK_SIZE as u64 {
            s.buffer[i as usize] = 0;
            i += 1;
        }
        s.final_ = 1;
        let bufp = s.buffer.as_ptr();
        poly1305_blocks(st, bufp, POLY1305_BLOCK_SIZE as u64);
    }

    let s = &mut *st;
    let mut h0 = s.h[0];
    let mut h1 = s.h[1];
    let mut h2 = s.h[2];
    let mut h3 = s.h[3];
    let mut h4 = s.h[4];

    let mut c = h1 >> 26;
    h1 &= 0x3ffffff;
    h2 = h2.wrapping_add(c);
    c = h2 >> 26;
    h2 &= 0x3ffffff;
    h3 = h3.wrapping_add(c);
    c = h3 >> 26;
    h3 &= 0x3ffffff;
    h4 = h4.wrapping_add(c);
    c = h4 >> 26;
    h4 &= 0x3ffffff;
    h0 = h0.wrapping_add(c.wrapping_mul(5));
    c = h0 >> 26;
    h0 &= 0x3ffffff;
    h1 = h1.wrapping_add(c);

    let mut g0 = h0.wrapping_add(5);
    c = g0 >> 26;
    g0 &= 0x3ffffff;
    let mut g1 = h1.wrapping_add(c);
    c = g1 >> 26;
    g1 &= 0x3ffffff;
    let mut g2 = h2.wrapping_add(c);
    c = g2 >> 26;
    g2 &= 0x3ffffff;
    let mut g3 = h3.wrapping_add(c);
    c = g3 >> 26;
    g3 &= 0x3ffffff;
    let mut g4 = h4.wrapping_add(c).wrapping_sub(1u64 << 26);

    // mask = (g4 >> (sizeof(unsigned long)*8 - 1)) - 1  (unsigned long = 64 bits)
    let mut mask = (g4 >> (8 * 8 - 1)).wrapping_sub(1);
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

    h0 = ((h0) | (h1 << 26)) & 0xffffffff;
    h1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffff;
    h2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffff;
    h3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffff;

    let mut f: u64 = h0.wrapping_add(s.pad[0]);
    h0 = f & 0xffffffff;
    f = h1.wrapping_add(s.pad[1]).wrapping_add(f >> 32);
    h1 = f & 0xffffffff;
    f = h2.wrapping_add(s.pad[2]).wrapping_add(f >> 32);
    h2 = f & 0xffffffff;
    f = h3.wrapping_add(s.pad[3]).wrapping_add(f >> 32);
    h3 = f & 0xffffffff;

    store32_le(mac.add(0), h0 as u32);
    store32_le(mac.add(4), h1 as u32);
    store32_le(mac.add(8), h2 as u32);
    store32_le(mac.add(12), h3 as u32);

    sodium_memzero(st as *mut c_void, core::mem::size_of::<St>());
}

// donna implementation functions

unsafe extern "C" fn onetimeauth_donna(
    out: *mut u8,
    m: *const u8,
    inlen: u64,
    key: *const u8,
) -> i32 {
    let mut state = new_st();
    poly1305_init(&mut state, key);
    poly1305_update(&mut state, m, inlen);
    poly1305_finish(&mut state, out);
    0
}

unsafe extern "C" fn onetimeauth_donna_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> i32 {
    poly1305_init(state as *mut St, key);
    0
}

unsafe extern "C" fn onetimeauth_donna_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    poly1305_update(state as *mut St, input, inlen);
    0
}

unsafe extern "C" fn onetimeauth_donna_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> i32 {
    poly1305_finish(state as *mut St, out);
    0
}

unsafe extern "C" fn onetimeauth_donna_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut correct = [0u8; 16];
    onetimeauth_donna(correct.as_mut_ptr(), input, inlen, k);
    crypto_verify_16(h, correct.as_ptr())
}

fn new_st() -> St {
    St {
        r: [0; 5],
        h: [0; 5],
        pad: [0; 4],
        leftover: 0,
        buffer: [0; POLY1305_BLOCK_SIZE],
        final_: 0,
    }
}

#[unsafe(no_mangle)]
pub static crypto_onetimeauth_poly1305_donna_implementation:
    crypto_onetimeauth_poly1305_implementation = crypto_onetimeauth_poly1305_implementation {
    onetimeauth: onetimeauth_donna,
    onetimeauth_verify: onetimeauth_donna_verify,
    onetimeauth_init: onetimeauth_donna_init,
    onetimeauth_update: onetimeauth_donna_update,
    onetimeauth_final: onetimeauth_donna_final,
};

static mut IMPLEMENTATION: *const crypto_onetimeauth_poly1305_implementation =
    &crypto_onetimeauth_poly1305_donna_implementation;

#[inline(always)]
unsafe fn imp() -> &'static crypto_onetimeauth_poly1305_implementation {
    &*core::ptr::read(&raw const IMPLEMENTATION)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    (imp().onetimeauth)(out, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    (imp().onetimeauth_verify)(h, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> i32 {
    (imp().onetimeauth_init)(state, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    (imp().onetimeauth_update)(state, input, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> i32 {
    (imp().onetimeauth_final)(state, out)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_poly1305_bytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_poly1305_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_onetimeauth_poly1305_state>()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_onetimeauth_poly1305_pick_best_implementation() -> i32 {
    core::ptr::write(
        &raw mut IMPLEMENTATION,
        &crypto_onetimeauth_poly1305_donna_implementation,
    );
    0
}

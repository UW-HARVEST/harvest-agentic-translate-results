//! Translation of c_src/libsodium/crypto_onetimeauth/poly1305/donna/poly1305_donna.c

use core::ffi::c_int;

use crate::common::{load32_le, store32_le};

// poly1305_donna.c includes crypto_verify_16.h; crypto_verify_16 is an
// exported symbol defined elsewhere.
extern "C" {
    fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
}

// ---------------------------------------------------------------------------
// poly1305_donna32.h (HAVE_TI_MODE undefined: 32-bit limb variant, #included
// into poly1305_donna.c). `unsigned long` is 64-bit on x86_64 Linux (LP64).
// ---------------------------------------------------------------------------

const POLY1305_BLOCK_SIZE: usize = 16;

// typedef struct poly1305_state_internal_t {
//     unsigned long      r[5];
//     unsigned long      h[5];
//     unsigned long      pad[4];
//     unsigned long long leftover;
//     unsigned char      buffer[poly1305_block_size];
//     unsigned char      final;
// } poly1305_state_internal_t;
#[repr(C)]
struct Poly1305StateInternalT {
    r: [core::ffi::c_ulong; 5],
    h: [core::ffi::c_ulong; 5],
    pad: [core::ffi::c_ulong; 4],
    leftover: core::ffi::c_ulonglong,
    buffer: [u8; POLY1305_BLOCK_SIZE],
    final_: u8,
}

unsafe fn poly1305_init(st: *mut Poly1305StateInternalT, key: *const u8) {
    // r &= 0xffffffc0ffffffc0ffffffc0fffffff - wiped after finalization
    (*st).r[0] = (load32_le(key.add(0)) & 0x3ffffff) as core::ffi::c_ulong;
    (*st).r[1] = ((load32_le(key.add(3)) >> 2) & 0x3ffff03) as core::ffi::c_ulong;
    (*st).r[2] = ((load32_le(key.add(6)) >> 4) & 0x3ffc0ff) as core::ffi::c_ulong;
    (*st).r[3] = ((load32_le(key.add(9)) >> 6) & 0x3f03fff) as core::ffi::c_ulong;
    (*st).r[4] = ((load32_le(key.add(12)) >> 8) & 0x00fffff) as core::ffi::c_ulong;

    // h = 0
    (*st).h[0] = 0;
    (*st).h[1] = 0;
    (*st).h[2] = 0;
    (*st).h[3] = 0;
    (*st).h[4] = 0;

    // save pad for later
    (*st).pad[0] = load32_le(key.add(16)) as core::ffi::c_ulong;
    (*st).pad[1] = load32_le(key.add(20)) as core::ffi::c_ulong;
    (*st).pad[2] = load32_le(key.add(24)) as core::ffi::c_ulong;
    (*st).pad[3] = load32_le(key.add(28)) as core::ffi::c_ulong;

    (*st).leftover = 0;
    (*st).final_ = 0;
}

unsafe fn poly1305_blocks(
    st: *mut Poly1305StateInternalT,
    mut m: *const u8,
    mut bytes: core::ffi::c_ulonglong,
) {
    let hibit: core::ffi::c_ulong = if (*st).final_ != 0 { 0 } else { 1u64 << 24 }; // 1 << 128
    let r0: core::ffi::c_ulong;
    let r1: core::ffi::c_ulong;
    let r2: core::ffi::c_ulong;
    let r3: core::ffi::c_ulong;
    let r4: core::ffi::c_ulong;
    let s1: core::ffi::c_ulong;
    let s2: core::ffi::c_ulong;
    let s3: core::ffi::c_ulong;
    let s4: core::ffi::c_ulong;
    let mut h0: core::ffi::c_ulong;
    let mut h1: core::ffi::c_ulong;
    let mut h2: core::ffi::c_ulong;
    let mut h3: core::ffi::c_ulong;
    let mut h4: core::ffi::c_ulong;
    let mut d0: core::ffi::c_ulonglong;
    let mut d1: core::ffi::c_ulonglong;
    let mut d2: core::ffi::c_ulonglong;
    let mut d3: core::ffi::c_ulonglong;
    let mut d4: core::ffi::c_ulonglong;
    let mut c: core::ffi::c_ulong;

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

    while bytes >= POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong {
        // h += m[i]
        h0 = h0.wrapping_add((load32_le(m.add(0)) & 0x3ffffff) as core::ffi::c_ulong);
        h1 = h1.wrapping_add(((load32_le(m.add(3)) >> 2) & 0x3ffffff) as core::ffi::c_ulong);
        h2 = h2.wrapping_add(((load32_le(m.add(6)) >> 4) & 0x3ffffff) as core::ffi::c_ulong);
        h3 = h3.wrapping_add(((load32_le(m.add(9)) >> 6) & 0x3ffffff) as core::ffi::c_ulong);
        h4 = h4.wrapping_add(((load32_le(m.add(12)) >> 8) as core::ffi::c_ulong) | hibit);

        // h *= r
        d0 = ((h0 as core::ffi::c_ulonglong).wrapping_mul(r0 as core::ffi::c_ulonglong))
            .wrapping_add((h1 as core::ffi::c_ulonglong).wrapping_mul(s4 as core::ffi::c_ulonglong))
            .wrapping_add((h2 as core::ffi::c_ulonglong).wrapping_mul(s3 as core::ffi::c_ulonglong))
            .wrapping_add((h3 as core::ffi::c_ulonglong).wrapping_mul(s2 as core::ffi::c_ulonglong))
            .wrapping_add((h4 as core::ffi::c_ulonglong).wrapping_mul(s1 as core::ffi::c_ulonglong));
        d1 = ((h0 as core::ffi::c_ulonglong).wrapping_mul(r1 as core::ffi::c_ulonglong))
            .wrapping_add((h1 as core::ffi::c_ulonglong).wrapping_mul(r0 as core::ffi::c_ulonglong))
            .wrapping_add((h2 as core::ffi::c_ulonglong).wrapping_mul(s4 as core::ffi::c_ulonglong))
            .wrapping_add((h3 as core::ffi::c_ulonglong).wrapping_mul(s3 as core::ffi::c_ulonglong))
            .wrapping_add((h4 as core::ffi::c_ulonglong).wrapping_mul(s2 as core::ffi::c_ulonglong));
        d2 = ((h0 as core::ffi::c_ulonglong).wrapping_mul(r2 as core::ffi::c_ulonglong))
            .wrapping_add((h1 as core::ffi::c_ulonglong).wrapping_mul(r1 as core::ffi::c_ulonglong))
            .wrapping_add((h2 as core::ffi::c_ulonglong).wrapping_mul(r0 as core::ffi::c_ulonglong))
            .wrapping_add((h3 as core::ffi::c_ulonglong).wrapping_mul(s4 as core::ffi::c_ulonglong))
            .wrapping_add((h4 as core::ffi::c_ulonglong).wrapping_mul(s3 as core::ffi::c_ulonglong));
        d3 = ((h0 as core::ffi::c_ulonglong).wrapping_mul(r3 as core::ffi::c_ulonglong))
            .wrapping_add((h1 as core::ffi::c_ulonglong).wrapping_mul(r2 as core::ffi::c_ulonglong))
            .wrapping_add((h2 as core::ffi::c_ulonglong).wrapping_mul(r1 as core::ffi::c_ulonglong))
            .wrapping_add((h3 as core::ffi::c_ulonglong).wrapping_mul(r0 as core::ffi::c_ulonglong))
            .wrapping_add((h4 as core::ffi::c_ulonglong).wrapping_mul(s4 as core::ffi::c_ulonglong));
        d4 = ((h0 as core::ffi::c_ulonglong).wrapping_mul(r4 as core::ffi::c_ulonglong))
            .wrapping_add((h1 as core::ffi::c_ulonglong).wrapping_mul(r3 as core::ffi::c_ulonglong))
            .wrapping_add((h2 as core::ffi::c_ulonglong).wrapping_mul(r2 as core::ffi::c_ulonglong))
            .wrapping_add((h3 as core::ffi::c_ulonglong).wrapping_mul(r1 as core::ffi::c_ulonglong))
            .wrapping_add((h4 as core::ffi::c_ulonglong).wrapping_mul(r0 as core::ffi::c_ulonglong));

        // (partial) h %= p
        c = (d0 >> 26) as core::ffi::c_ulong;
        h0 = (d0 as core::ffi::c_ulong) & 0x3ffffff;
        d1 = d1.wrapping_add(c as core::ffi::c_ulonglong);
        c = (d1 >> 26) as core::ffi::c_ulong;
        h1 = (d1 as core::ffi::c_ulong) & 0x3ffffff;
        d2 = d2.wrapping_add(c as core::ffi::c_ulonglong);
        c = (d2 >> 26) as core::ffi::c_ulong;
        h2 = (d2 as core::ffi::c_ulong) & 0x3ffffff;
        d3 = d3.wrapping_add(c as core::ffi::c_ulonglong);
        c = (d3 >> 26) as core::ffi::c_ulong;
        h3 = (d3 as core::ffi::c_ulong) & 0x3ffffff;
        d4 = d4.wrapping_add(c as core::ffi::c_ulonglong);
        c = (d4 >> 26) as core::ffi::c_ulong;
        h4 = (d4 as core::ffi::c_ulong) & 0x3ffffff;
        h0 = h0.wrapping_add(c.wrapping_mul(5));
        c = h0 >> 26;
        h0 &= 0x3ffffff;
        h1 = h1.wrapping_add(c);

        m = m.add(POLY1305_BLOCK_SIZE);
        bytes -= POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong;
    }

    (*st).h[0] = h0;
    (*st).h[1] = h1;
    (*st).h[2] = h2;
    (*st).h[3] = h3;
    (*st).h[4] = h4;
}

// POLY1305_NOINLINE
unsafe fn poly1305_finish(st: *mut Poly1305StateInternalT, mac: *mut u8) {
    let mut h0: core::ffi::c_ulong;
    let mut h1: core::ffi::c_ulong;
    let mut h2: core::ffi::c_ulong;
    let mut h3: core::ffi::c_ulong;
    let mut h4: core::ffi::c_ulong;
    let mut c: core::ffi::c_ulong;
    let mut g0: core::ffi::c_ulong;
    let mut g1: core::ffi::c_ulong;
    let mut g2: core::ffi::c_ulong;
    let mut g3: core::ffi::c_ulong;
    let mut g4: core::ffi::c_ulong;
    let mut f: core::ffi::c_ulonglong;
    let mut mask: core::ffi::c_ulong;

    // process the remaining block
    if (*st).leftover != 0 {
        let mut i: core::ffi::c_ulonglong = (*st).leftover;

        (*st).buffer[i as usize] = 1;
        i += 1;
        while i < POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong {
            (*st).buffer[i as usize] = 0;
            i += 1;
        }
        (*st).final_ = 1;
        let buf = core::ptr::addr_of!((*st).buffer) as *const u8;
        poly1305_blocks(st, buf, POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong);
    }

    // fully carry h
    h0 = (*st).h[0];
    h1 = (*st).h[1];
    h2 = (*st).h[2];
    h3 = (*st).h[3];
    h4 = (*st).h[4];

    c = h1 >> 26;
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

    // compute h + -p
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

    // select h if h < p, or h + -p if h >= p
    // mask = (g4 >> ((sizeof(unsigned long) * 8) - 1)) - 1;  (63 on LP64)
    mask = (g4 >> ((core::mem::size_of::<core::ffi::c_ulong>() * 8) - 1)).wrapping_sub(1);
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

    // h = h % (2^128)
    h0 = ((h0) | (h1 << 26)) & 0xffffffff;
    h1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffff;
    h2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffff;
    h3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffff;

    // mac = (h + pad) % (2^128)
    f = (h0 as core::ffi::c_ulonglong).wrapping_add((*st).pad[0] as core::ffi::c_ulonglong);
    h0 = f as core::ffi::c_ulong;
    f = (h1 as core::ffi::c_ulonglong)
        .wrapping_add((*st).pad[1] as core::ffi::c_ulonglong)
        .wrapping_add(f >> 32);
    h1 = f as core::ffi::c_ulong;
    f = (h2 as core::ffi::c_ulonglong)
        .wrapping_add((*st).pad[2] as core::ffi::c_ulonglong)
        .wrapping_add(f >> 32);
    h2 = f as core::ffi::c_ulong;
    f = (h3 as core::ffi::c_ulonglong)
        .wrapping_add((*st).pad[3] as core::ffi::c_ulonglong)
        .wrapping_add(f >> 32);
    h3 = f as core::ffi::c_ulong;

    store32_le(mac.add(0), h0 as u32);
    store32_le(mac.add(4), h1 as u32);
    store32_le(mac.add(8), h2 as u32);
    store32_le(mac.add(12), h3 as u32);

    // zero out the state
    sodium_memzero(
        st as *mut core::ffi::c_void,
        core::mem::size_of::<Poly1305StateInternalT>(),
    );
}

// ---------------------------------------------------------------------------
// poly1305_donna.c
// ---------------------------------------------------------------------------

// crypto_onetimeauth_poly1305_state is the public 256-byte aligned struct.
#[repr(C, align(16))]
struct CryptoOnetimeauthPoly1305State {
    opaque: [u8; 256],
}

// #[repr(C)] mirror of crypto_onetimeauth_poly1305_implementation from
// crypto_onetimeauth/poly1305/onetimeauth_poly1305.h.
#[repr(C)]
pub struct CryptoOnetimeauthPoly1305Implementation {
    pub onetimeauth: Option<
        unsafe extern "C" fn(out: *mut u8, in_: *const u8, inlen: u64, k: *const u8) -> c_int,
    >,
    pub onetimeauth_verify: Option<
        unsafe extern "C" fn(h: *const u8, in_: *const u8, inlen: u64, k: *const u8) -> c_int,
    >,
    pub onetimeauth_init: Option<
        unsafe extern "C" fn(
            state: *mut CryptoOnetimeauthPoly1305State,
            key: *const u8,
        ) -> c_int,
    >,
    pub onetimeauth_update: Option<
        unsafe extern "C" fn(
            state: *mut CryptoOnetimeauthPoly1305State,
            in_: *const u8,
            inlen: u64,
        ) -> c_int,
    >,
    pub onetimeauth_final: Option<
        unsafe extern "C" fn(
            state: *mut CryptoOnetimeauthPoly1305State,
            out: *mut u8,
        ) -> c_int,
    >,
}

unsafe impl Sync for CryptoOnetimeauthPoly1305Implementation {}

unsafe fn poly1305_update(
    st: *mut Poly1305StateInternalT,
    mut m: *const u8,
    mut bytes: core::ffi::c_ulonglong,
) {
    let mut i: core::ffi::c_ulonglong;

    // handle leftover
    if (*st).leftover != 0 {
        let mut want: core::ffi::c_ulonglong =
            POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong - (*st).leftover;

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
        if (*st).leftover < POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong {
            return;
        }
        let buf = core::ptr::addr_of!((*st).buffer) as *const u8;
        poly1305_blocks(st, buf, POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong);
        (*st).leftover = 0;
    }

    // process full blocks
    if bytes >= POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong {
        let want: core::ffi::c_ulonglong =
            bytes & !(POLY1305_BLOCK_SIZE as core::ffi::c_ulonglong - 1);

        poly1305_blocks(st, m, want);
        m = m.add(want as usize);
        bytes -= want;
    }

    // store leftover
    if bytes != 0 {
        i = 0;
        while i < bytes {
            (*st).buffer[((*st).leftover + i) as usize] = *m.add(i as usize);
            i += 1;
        }
        (*st).leftover += bytes;
    }
}

unsafe fn crypto_onetimeauth_poly1305_donna(
    out: *mut u8,
    m: *const u8,
    inlen: u64,
    key: *const u8,
) -> c_int {
    // CRYPTO_ALIGN(64) poly1305_state_internal_t state;
    #[repr(C, align(64))]
    struct Aligned64(Poly1305StateInternalT);
    let mut state = core::mem::MaybeUninit::<Aligned64>::uninit();
    let st = core::ptr::addr_of_mut!((*state.as_mut_ptr()).0);

    poly1305_init(st, key);
    poly1305_update(st, m, inlen);
    poly1305_finish(st, out);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_shim(
    out: *mut u8,
    m: *const u8,
    inlen: u64,
    key: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305_donna(out, m, inlen, key)
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_init(
    state: *mut CryptoOnetimeauthPoly1305State,
    key: *const u8,
) -> c_int {
    // COMPILER_ASSERT(sizeof(state) >= sizeof(poly1305_state_internal_t))
    poly1305_init(state as *mut Poly1305StateInternalT, key);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_update(
    state: *mut CryptoOnetimeauthPoly1305State,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    poly1305_update(state as *mut Poly1305StateInternalT, in_, inlen);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_final(
    state: *mut CryptoOnetimeauthPoly1305State,
    out: *mut u8,
) -> c_int {
    poly1305_finish(state as *mut Poly1305StateInternalT, out);

    0
}

unsafe extern "C" fn crypto_onetimeauth_poly1305_donna_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct = [0u8; 16];

    crypto_onetimeauth_poly1305_donna(correct.as_mut_ptr(), in_, inlen, k);

    crypto_verify_16(h, correct.as_ptr())
}

#[unsafe(no_mangle)]
pub static crypto_onetimeauth_poly1305_donna_implementation:
    CryptoOnetimeauthPoly1305Implementation = CryptoOnetimeauthPoly1305Implementation {
    onetimeauth: Some(crypto_onetimeauth_poly1305_donna_shim),
    onetimeauth_verify: Some(crypto_onetimeauth_poly1305_donna_verify),
    onetimeauth_init: Some(crypto_onetimeauth_poly1305_donna_init),
    onetimeauth_update: Some(crypto_onetimeauth_poly1305_donna_update),
    onetimeauth_final: Some(crypto_onetimeauth_poly1305_donna_final),
};

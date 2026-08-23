//! Translation of `crypto_stream/chacha20/ref/chacha20_ref.c`
//!
//! ```text
//!  chacha-merged.c version 20080118
//!  D. J. Bernstein
//!  Public domain.
//! ```

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

const crypto_stream_chacha20_KEYBYTES: usize = 32;

/* `struct crypto_stream_chacha20_implementation` from
 * `crypto_stream/chacha20/stream_chacha20.h` (private header, duplicated here). */
#[repr(C)]
pub struct crypto_stream_chacha20_implementation {
    pub stream: unsafe extern "C" fn(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int,
    pub stream_ietf_ext: unsafe extern "C" fn(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int,
    pub stream_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int,
    pub stream_ietf_ext_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int,
}

#[repr(C)]
struct chacha_ctx {
    input: [u32; 16],
}

/* #define U32V(v) ((uint32_t)(v) & U32C(0xFFFFFFFF)) */
#[inline(always)]
fn u32v(v: u64) -> u32 {
    (v & 0xFFFFFFFF) as u32
}

/* QUARTERROUND(a, b, c, d) -- all additions wrap modulo 2^32. */
macro_rules! quarterround {
    ($a:ident, $b:ident, $c:ident, $d:ident) => {
        $a = $a.wrapping_add($b);
        $d = rotl32($d ^ $a, 16);
        $c = $c.wrapping_add($d);
        $b = rotl32($b ^ $c, 12);
        $a = $a.wrapping_add($b);
        $d = rotl32($d ^ $a, 8);
        $c = $c.wrapping_add($d);
        $b = rotl32($b ^ $c, 7);
    };
}

unsafe fn chacha_keysetup(ctx: *mut chacha_ctx, k: *const u8) {
    (*ctx).input[0] = 0x61707865;
    (*ctx).input[1] = 0x3320646e;
    (*ctx).input[2] = 0x79622d32;
    (*ctx).input[3] = 0x6b206574;
    (*ctx).input[4] = load32_le(k.add(0));
    (*ctx).input[5] = load32_le(k.add(4));
    (*ctx).input[6] = load32_le(k.add(8));
    (*ctx).input[7] = load32_le(k.add(12));
    (*ctx).input[8] = load32_le(k.add(16));
    (*ctx).input[9] = load32_le(k.add(20));
    (*ctx).input[10] = load32_le(k.add(24));
    (*ctx).input[11] = load32_le(k.add(28));
}

unsafe fn chacha_ivsetup(ctx: *mut chacha_ctx, iv: *const u8, counter: *const u8) {
    (*ctx).input[12] = if counter.is_null() {
        0
    } else {
        load32_le(counter.add(0))
    };
    (*ctx).input[13] = if counter.is_null() {
        0
    } else {
        load32_le(counter.add(4))
    };
    (*ctx).input[14] = load32_le(iv.add(0));
    (*ctx).input[15] = load32_le(iv.add(4));
}

unsafe fn chacha_ietf_ivsetup(ctx: *mut chacha_ctx, iv: *const u8, counter: *const u8) {
    (*ctx).input[12] = if counter.is_null() {
        0
    } else {
        load32_le(counter)
    };
    (*ctx).input[13] = load32_le(iv.add(0));
    (*ctx).input[14] = load32_le(iv.add(4));
    (*ctx).input[15] = load32_le(iv.add(8));
}

unsafe fn chacha20_encrypt_bytes(
    ctx: *mut chacha_ctx,
    mut m: *const u8,
    mut c: *mut u8,
    mut bytes: c_ulonglong,
) {
    let mut ctarget: *mut u8 = core::ptr::null_mut();
    let mut tmp: [u8; 64] = [0; 64];
    let tmp_ptr: *mut u8 = tmp.as_mut_ptr();

    if bytes == 0 {
        return; /* LCOV_EXCL_LINE */
    }
    let j0 = (*ctx).input[0];
    let j1 = (*ctx).input[1];
    let j2 = (*ctx).input[2];
    let j3 = (*ctx).input[3];
    let j4 = (*ctx).input[4];
    let j5 = (*ctx).input[5];
    let j6 = (*ctx).input[6];
    let j7 = (*ctx).input[7];
    let j8 = (*ctx).input[8];
    let j9 = (*ctx).input[9];
    let j10 = (*ctx).input[10];
    let j11 = (*ctx).input[11];
    let mut j12 = (*ctx).input[12];
    let mut j13 = (*ctx).input[13];
    let j14 = (*ctx).input[14];
    let j15 = (*ctx).input[15];

    loop {
        if bytes < 64 {
            memset(tmp_ptr, 0, 64);
            let mut i: u32 = 0;
            while (i as c_ulonglong) < bytes {
                *tmp_ptr.add(i as usize) = *m.add(i as usize);
                i = i.wrapping_add(1);
            }
            m = tmp_ptr as *const u8;
            ctarget = c;
            c = tmp_ptr;
        }
        let mut x0 = j0;
        let mut x1 = j1;
        let mut x2 = j2;
        let mut x3 = j3;
        let mut x4 = j4;
        let mut x5 = j5;
        let mut x6 = j6;
        let mut x7 = j7;
        let mut x8 = j8;
        let mut x9 = j9;
        let mut x10 = j10;
        let mut x11 = j11;
        let mut x12 = j12;
        let mut x13 = j13;
        let mut x14 = j14;
        let mut x15 = j15;

        let mut i: u32 = 20;
        while i > 0 {
            quarterround!(x0, x4, x8, x12);
            quarterround!(x1, x5, x9, x13);
            quarterround!(x2, x6, x10, x14);
            quarterround!(x3, x7, x11, x15);
            quarterround!(x0, x5, x10, x15);
            quarterround!(x1, x6, x11, x12);
            quarterround!(x2, x7, x8, x13);
            quarterround!(x3, x4, x9, x14);
            i -= 2;
        }
        x0 = x0.wrapping_add(j0);
        x1 = x1.wrapping_add(j1);
        x2 = x2.wrapping_add(j2);
        x3 = x3.wrapping_add(j3);
        x4 = x4.wrapping_add(j4);
        x5 = x5.wrapping_add(j5);
        x6 = x6.wrapping_add(j6);
        x7 = x7.wrapping_add(j7);
        x8 = x8.wrapping_add(j8);
        x9 = x9.wrapping_add(j9);
        x10 = x10.wrapping_add(j10);
        x11 = x11.wrapping_add(j11);
        x12 = x12.wrapping_add(j12);
        x13 = x13.wrapping_add(j13);
        x14 = x14.wrapping_add(j14);
        x15 = x15.wrapping_add(j15);

        x0 ^= load32_le(m.add(0));
        x1 ^= load32_le(m.add(4));
        x2 ^= load32_le(m.add(8));
        x3 ^= load32_le(m.add(12));
        x4 ^= load32_le(m.add(16));
        x5 ^= load32_le(m.add(20));
        x6 ^= load32_le(m.add(24));
        x7 ^= load32_le(m.add(28));
        x8 ^= load32_le(m.add(32));
        x9 ^= load32_le(m.add(36));
        x10 ^= load32_le(m.add(40));
        x11 ^= load32_le(m.add(44));
        x12 ^= load32_le(m.add(48));
        x13 ^= load32_le(m.add(52));
        x14 ^= load32_le(m.add(56));
        x15 ^= load32_le(m.add(60));

        j12 = j12.wrapping_add(1);
        /* LCOV_EXCL_START */
        if j12 == 0 {
            j13 = j13.wrapping_add(1);
        }
        /* LCOV_EXCL_STOP */

        store32_le(c.add(0), x0);
        store32_le(c.add(4), x1);
        store32_le(c.add(8), x2);
        store32_le(c.add(12), x3);
        store32_le(c.add(16), x4);
        store32_le(c.add(20), x5);
        store32_le(c.add(24), x6);
        store32_le(c.add(28), x7);
        store32_le(c.add(32), x8);
        store32_le(c.add(36), x9);
        store32_le(c.add(40), x10);
        store32_le(c.add(44), x11);
        store32_le(c.add(48), x12);
        store32_le(c.add(52), x13);
        store32_le(c.add(56), x14);
        store32_le(c.add(60), x15);

        if bytes <= 64 {
            if bytes < 64 {
                let mut i: u32 = 0;
                while (i as c_ulonglong) < bytes {
                    /* ctarget cannot be NULL */
                    *ctarget.add(i as usize) = *c.add(i as usize);
                    i = i.wrapping_add(1);
                }
            }
            (*ctx).input[12] = j12;
            (*ctx).input[13] = j13;

            return;
        }
        bytes -= 64;
        c = c.add(64);
        m = m.add(64);
    }
}

unsafe extern "C" fn stream_ref(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0; 16] };

    if clen == 0 {
        return 0;
    }
    const _: () = assert!(crypto_stream_chacha20_KEYBYTES == 256 / 8);
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, core::ptr::null());
    memset(c, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c as *const u8, c, clen);
    sodium_memzero(
        &mut ctx as *mut chacha_ctx as *mut c_void,
        core::mem::size_of::<chacha_ctx>(),
    );

    0
}

unsafe extern "C" fn stream_ietf_ext_ref(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0; 16] };

    if clen == 0 {
        return 0;
    }
    const _: () = assert!(crypto_stream_chacha20_KEYBYTES == 256 / 8);
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, core::ptr::null());
    memset(c, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c as *const u8, c, clen);
    sodium_memzero(
        &mut ctx as *mut chacha_ctx as *mut c_void,
        core::mem::size_of::<chacha_ctx>(),
    );

    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0; 16] };
    let mut ic_bytes: [u8; 8] = [0; 8];
    let ic_high: u32;
    let ic_low: u32;

    if mlen == 0 {
        return 0;
    }
    ic_high = u32v(ic >> 32);
    ic_low = u32v(ic);
    store32_le(ic_bytes.as_mut_ptr().add(0), ic_low);
    store32_le(ic_bytes.as_mut_ptr().add(4), ic_high);
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
    chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
    sodium_memzero(
        &mut ctx as *mut chacha_ctx as *mut c_void,
        core::mem::size_of::<chacha_ctx>(),
    );

    0
}

unsafe extern "C" fn stream_ietf_ext_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0; 16] };
    let mut ic_bytes: [u8; 4] = [0; 4];

    if mlen == 0 {
        return 0;
    }
    store32_le(ic_bytes.as_mut_ptr(), ic);
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
    chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
    sodium_memzero(
        &mut ctx as *mut chacha_ctx as *mut c_void,
        core::mem::size_of::<chacha_ctx>(),
    );

    0
}

#[unsafe(no_mangle)]
pub static crypto_stream_chacha20_ref_implementation: crypto_stream_chacha20_implementation =
    crypto_stream_chacha20_implementation {
        stream: stream_ref,
        stream_ietf_ext: stream_ietf_ext_ref,
        stream_xor_ic: stream_ref_xor_ic,
        stream_ietf_ext_xor_ic: stream_ietf_ext_ref_xor_ic,
    };

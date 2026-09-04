//! Translation of `crypto_stream/chacha20/stream_chacha20.{c,h}` and
//! `crypto_stream/chacha20/ref/chacha20_ref.{c,h}`.
//!
//! The SIMD (`dolbeau/*`) variants are compiled out and
//! `sodium_runtime_has_*()` always returns 0, so the dispatch table always
//! selects `crypto_stream_chacha20_ref_implementation`. The dispatch code is
//! still translated faithfully.

use core::ffi::{c_int, c_void};

use crate::common::{load32_le, rotl32, store32_le};
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

/* ---- constants from crypto_stream_chacha20.h ---- */

pub const crypto_stream_chacha20_KEYBYTES: usize = 32;
pub const crypto_stream_chacha20_NONCEBYTES: usize = 8;
/* SODIUM_SIZE_MAX */
pub const crypto_stream_chacha20_MESSAGEBYTES_MAX: u64 = crate::common::SODIUM_SIZE_MAX as u64;

pub const crypto_stream_chacha20_ietf_KEYBYTES: usize = 32;
pub const crypto_stream_chacha20_ietf_NONCEBYTES: usize = 12;
/* SODIUM_MIN(SODIUM_SIZE_MAX, 64ULL * (1ULL << 32)) */
pub const crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX: u64 = {
    let a = crate::common::SODIUM_SIZE_MAX as u64;
    let b = 64u64 * (1u64 << 32);
    if a < b {
        a
    } else {
        b
    }
};

/* ---- dispatch table type (stream_chacha20.h) ---- */

#[repr(C)]
pub struct crypto_stream_chacha20_implementation {
    pub stream: unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int,
    pub stream_ietf_ext:
        unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int,
    pub stream_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int,
    pub stream_ietf_ext_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int,
}

unsafe impl Sync for crypto_stream_chacha20_implementation {}

/* =========================================================================
 * ref/chacha20_ref.c
 * ========================================================================= */

#[repr(C)]
struct chacha_ctx {
    input: [u32; 16],
}

#[inline(always)]
fn u32v(v: u32) -> u32 {
    v & 0xFFFFFFFF
}

#[inline(always)]
fn rotate(v: u32, c: i32) -> u32 {
    rotl32(v, c)
}

#[inline(always)]
fn xor(v: u32, w: u32) -> u32 {
    v ^ w
}

#[inline(always)]
fn plus(v: u32, w: u32) -> u32 {
    u32v(v.wrapping_add(w))
}

#[inline(always)]
fn plusone(v: u32) -> u32 {
    plus(v, 1)
}

macro_rules! quarterround {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        $a = plus($a, $b);
        $d = rotate(xor($d, $a), 16);
        $c = plus($c, $d);
        $b = rotate(xor($b, $c), 12);
        $a = plus($a, $b);
        $d = rotate(xor($d, $a), 8);
        $c = plus($c, $d);
        $b = rotate(xor($b, $c), 7);
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
    mut bytes: u64,
) {
    let (mut x0, mut x1, mut x2, mut x3, mut x4, mut x5, mut x6, mut x7);
    let (mut x8, mut x9, mut x10, mut x11, mut x12, mut x13, mut x14, mut x15);
    let (j0, j1, j2, j3, j4, j5, j6, j7, j8, j9, j10, j11);
    let (mut j12, mut j13, j14, j15);
    let mut ctarget: *mut u8 = core::ptr::null_mut();
    let mut tmp: [u8; 64] = [0; 64];
    let mut i: u32;

    if bytes == 0 {
        return; /* LCOV_EXCL_LINE */
    }
    j0 = (*ctx).input[0];
    j1 = (*ctx).input[1];
    j2 = (*ctx).input[2];
    j3 = (*ctx).input[3];
    j4 = (*ctx).input[4];
    j5 = (*ctx).input[5];
    j6 = (*ctx).input[6];
    j7 = (*ctx).input[7];
    j8 = (*ctx).input[8];
    j9 = (*ctx).input[9];
    j10 = (*ctx).input[10];
    j11 = (*ctx).input[11];
    j12 = (*ctx).input[12];
    j13 = (*ctx).input[13];
    j14 = (*ctx).input[14];
    j15 = (*ctx).input[15];

    loop {
        if bytes < 64 {
            crate::common::memset(tmp.as_mut_ptr(), 0, 64);
            i = 0;
            while (i as u64) < bytes {
                tmp[i as usize] = *m.add(i as usize);
                i += 1;
            }
            m = tmp.as_ptr();
            ctarget = c;
            c = tmp.as_mut_ptr();
        }
        x0 = j0;
        x1 = j1;
        x2 = j2;
        x3 = j3;
        x4 = j4;
        x5 = j5;
        x6 = j6;
        x7 = j7;
        x8 = j8;
        x9 = j9;
        x10 = j10;
        x11 = j11;
        x12 = j12;
        x13 = j13;
        x14 = j14;
        x15 = j15;
        i = 20;
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
        x0 = plus(x0, j0);
        x1 = plus(x1, j1);
        x2 = plus(x2, j2);
        x3 = plus(x3, j3);
        x4 = plus(x4, j4);
        x5 = plus(x5, j5);
        x6 = plus(x6, j6);
        x7 = plus(x7, j7);
        x8 = plus(x8, j8);
        x9 = plus(x9, j9);
        x10 = plus(x10, j10);
        x11 = plus(x11, j11);
        x12 = plus(x12, j12);
        x13 = plus(x13, j13);
        x14 = plus(x14, j14);
        x15 = plus(x15, j15);

        x0 = xor(x0, load32_le(m.add(0)));
        x1 = xor(x1, load32_le(m.add(4)));
        x2 = xor(x2, load32_le(m.add(8)));
        x3 = xor(x3, load32_le(m.add(12)));
        x4 = xor(x4, load32_le(m.add(16)));
        x5 = xor(x5, load32_le(m.add(20)));
        x6 = xor(x6, load32_le(m.add(24)));
        x7 = xor(x7, load32_le(m.add(28)));
        x8 = xor(x8, load32_le(m.add(32)));
        x9 = xor(x9, load32_le(m.add(36)));
        x10 = xor(x10, load32_le(m.add(40)));
        x11 = xor(x11, load32_le(m.add(44)));
        x12 = xor(x12, load32_le(m.add(48)));
        x13 = xor(x13, load32_le(m.add(52)));
        x14 = xor(x14, load32_le(m.add(56)));
        x15 = xor(x15, load32_le(m.add(60)));

        j12 = plusone(j12);
        /* LCOV_EXCL_START */
        if j12 == 0 {
            j13 = plusone(j13);
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
                i = 0;
                while i < bytes as u32 {
                    *ctarget.add(i as usize) = *c.add(i as usize); /* ctarget cannot be NULL */
                    i += 1;
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

unsafe extern "C" fn stream_ref(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int {
    let mut ctx = chacha_ctx { input: [0; 16] };

    if clen == 0 {
        return 0;
    }
    /* COMPILER_ASSERT(crypto_stream_chacha20_KEYBYTES == 256 / 8); */
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, core::ptr::null());
    crate::common::memset(c, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c, c, clen);
    sodium_memzero(
        &mut ctx as *mut chacha_ctx as *mut c_void,
        core::mem::size_of::<chacha_ctx>(),
    );

    0
}

unsafe extern "C" fn stream_ietf_ext_ref(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0; 16] };

    if clen == 0 {
        return 0;
    }
    /* COMPILER_ASSERT(crypto_stream_chacha20_KEYBYTES == 256 / 8); */
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, core::ptr::null());
    crate::common::memset(c, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c, c, clen);
    sodium_memzero(
        &mut ctx as *mut chacha_ctx as *mut c_void,
        core::mem::size_of::<chacha_ctx>(),
    );

    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
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
    ic_high = u32v((ic >> 32) as u32);
    ic_low = u32v(ic as u32);
    store32_le(&mut ic_bytes[0] as *mut u8, ic_low);
    store32_le(&mut ic_bytes[4] as *mut u8, ic_high);
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
    mlen: u64,
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

/* exported data symbol */
#[unsafe(no_mangle)]
pub static crypto_stream_chacha20_ref_implementation: crypto_stream_chacha20_implementation =
    crypto_stream_chacha20_implementation {
        stream: stream_ref,
        stream_ietf_ext: stream_ietf_ext_ref,
        stream_xor_ic: stream_ref_xor_ic,
        stream_ietf_ext_xor_ic: stream_ietf_ext_ref_xor_ic,
    };

/* =========================================================================
 * stream_chacha20.c
 * ========================================================================= */

static mut implementation: *const crypto_stream_chacha20_implementation =
    &crypto_stream_chacha20_ref_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_keybytes() -> usize {
    crypto_stream_chacha20_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_noncebytes() -> usize {
    crypto_stream_chacha20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_messagebytes_max() -> usize {
    crypto_stream_chacha20_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_ietf_keybytes() -> usize {
    crypto_stream_chacha20_ietf_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_ietf_noncebytes() -> usize {
    crypto_stream_chacha20_ietf_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_ietf_messagebytes_max() -> usize {
    crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).stream)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).stream_xor_ic)(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).stream_xor_ic)(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).stream_ietf_ext)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).stream_ietf_ext_xor_ic)(c, m, mlen, n, ic, k)
}

unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).stream_ietf_ext_xor_ic)(c, m, mlen, n, 0u32, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if (ic as u64) > (64u64 * (1u64 << 32)) / 64u64 - (mlen + 63u64) / 64u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext_xor_ic(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_chacha20_ietf_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_chacha20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub extern "C" fn _crypto_stream_chacha20_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &crypto_stream_chacha20_ref_implementation;
    }
    0
}

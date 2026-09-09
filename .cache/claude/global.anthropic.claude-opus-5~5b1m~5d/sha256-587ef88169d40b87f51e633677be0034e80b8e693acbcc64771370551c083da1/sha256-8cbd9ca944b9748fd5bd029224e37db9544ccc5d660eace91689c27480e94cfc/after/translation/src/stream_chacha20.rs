//! Translation of:
//!   - `crypto_stream/chacha20/stream_chacha20.c` + `crypto_stream/chacha20/ref/chacha20_ref.c`
//!   - `crypto_stream/xchacha20/stream_xchacha20.c`
//!   - `include/sodium/private/chacha20_ietf_ext.h`
//!
//! The reference build has no SIMD implementations, so `pick_best_implementation`
//! always selects the `ref` implementation.

use crate::common::{load32_le, rotl32, store32_le, SODIUM_SIZE_MAX};
use crate::csys::memset;
use core::ffi::c_int;

extern "C" {
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut u8, len: usize);
    fn randombytes_buf(buf: *mut u8, size: usize);
    fn sodium_misuse() -> !;
}

/// `crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX` = min(SODIUM_SIZE_MAX, 64 * 2^32)
const IETF_MESSAGEBYTES_MAX: u64 = {
    let a = SODIUM_SIZE_MAX;
    let b: u64 = 64u64 * (1u64 << 32);
    if a < b {
        a
    } else {
        b
    }
};

// =====================================================================
// crypto_stream/chacha20/stream_chacha20.h
// =====================================================================

#[repr(C)]
pub struct crypto_stream_chacha20_implementation {
    pub stream: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int,
    pub stream_ietf_ext: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int,
    pub stream_xor_ic:
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int,
    pub stream_ietf_ext_xor_ic:
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int,
}

// =====================================================================
// crypto_stream/chacha20/ref/chacha20_ref.c
// =====================================================================

macro_rules! quarterround {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
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

#[repr(C)]
struct ChachaCtx {
    input: [u32; 16],
}

unsafe fn chacha_keysetup(ctx: &mut ChachaCtx, k: *const u8) {
    ctx.input[0] = 0x6170_7865;
    ctx.input[1] = 0x3320_646e;
    ctx.input[2] = 0x7962_2d32;
    ctx.input[3] = 0x6b20_6574;
    ctx.input[4] = load32_le(k.add(0));
    ctx.input[5] = load32_le(k.add(4));
    ctx.input[6] = load32_le(k.add(8));
    ctx.input[7] = load32_le(k.add(12));
    ctx.input[8] = load32_le(k.add(16));
    ctx.input[9] = load32_le(k.add(20));
    ctx.input[10] = load32_le(k.add(24));
    ctx.input[11] = load32_le(k.add(28));
}

unsafe fn chacha_ivsetup(ctx: &mut ChachaCtx, iv: *const u8, counter: *const u8) {
    ctx.input[12] = if counter.is_null() {
        0
    } else {
        load32_le(counter.add(0))
    };
    ctx.input[13] = if counter.is_null() {
        0
    } else {
        load32_le(counter.add(4))
    };
    ctx.input[14] = load32_le(iv.add(0));
    ctx.input[15] = load32_le(iv.add(4));
}

unsafe fn chacha_ietf_ivsetup(ctx: &mut ChachaCtx, iv: *const u8, counter: *const u8) {
    ctx.input[12] = if counter.is_null() {
        0
    } else {
        load32_le(counter)
    };
    ctx.input[13] = load32_le(iv.add(0));
    ctx.input[14] = load32_le(iv.add(4));
    ctx.input[15] = load32_le(iv.add(8));
}

unsafe fn chacha20_encrypt_bytes(
    ctx: &mut ChachaCtx,
    m: *const u8,
    c: *mut u8,
    bytes: u64,
) {
    let (mut x0, mut x1, mut x2, mut x3, mut x4, mut x5, mut x6, mut x7);
    let (mut x8, mut x9, mut x10, mut x11, mut x12, mut x13, mut x14, mut x15);
    let (mut j12, mut j13);
    let mut ctarget: *mut u8 = core::ptr::null_mut();
    let mut tmp = [0u8; 64];
    let mut bytes = bytes;
    let mut m = m;
    let mut c = c;

    if bytes == 0 {
        return;
    }
    let j0 = ctx.input[0];
    let j1 = ctx.input[1];
    let j2 = ctx.input[2];
    let j3 = ctx.input[3];
    let j4 = ctx.input[4];
    let j5 = ctx.input[5];
    let j6 = ctx.input[6];
    let j7 = ctx.input[7];
    let j8 = ctx.input[8];
    let j9 = ctx.input[9];
    let j10 = ctx.input[10];
    let j11 = ctx.input[11];
    j12 = ctx.input[12];
    j13 = ctx.input[13];
    let j14 = ctx.input[14];
    let j15 = ctx.input[15];
    let mut j12 = j12;
    let mut j13 = j13;

    loop {
        if bytes < 64 {
            memset(tmp.as_mut_ptr() as *mut core::ffi::c_void, 0, 64);
            for i in 0..(bytes as usize) {
                tmp[i] = *m.add(i);
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

        let mut i = 20;
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
        if j12 == 0 {
            j13 = j13.wrapping_add(1);
        }

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
                for i in 0..(bytes as usize) {
                    *ctarget.add(i) = *c.add(i);
                }
            }
            ctx.input[12] = j12;
            ctx.input[13] = j13;

            return;
        }
        bytes -= 64;
        c = c.add(64);
        m = m.add(64);
    }
}

unsafe extern "C" fn stream_ref(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int {
    let mut ctx = ChachaCtx { input: [0u32; 16] };

    if clen == 0 {
        return 0;
    }
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, core::ptr::null());
    memset(c as *mut core::ffi::c_void, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c, c, clen);
    sodium_memzero(
        &mut ctx as *mut ChachaCtx as *mut u8,
        core::mem::size_of::<ChachaCtx>(),
    );

    0
}

unsafe extern "C" fn stream_ietf_ext_ref(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut ctx = ChachaCtx { input: [0u32; 16] };

    if clen == 0 {
        return 0;
    }
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, core::ptr::null());
    memset(c as *mut core::ffi::c_void, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c, c, clen);
    sodium_memzero(
        &mut ctx as *mut ChachaCtx as *mut u8,
        core::mem::size_of::<ChachaCtx>(),
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
    let mut ctx = ChachaCtx { input: [0u32; 16] };
    let mut ic_bytes = [0u8; 8];

    if mlen == 0 {
        return 0;
    }
    let ic_high = (ic >> 32) as u32;
    let ic_low = ic as u32;
    store32_le(ic_bytes.as_mut_ptr(), ic_low);
    store32_le(ic_bytes.as_mut_ptr().add(4), ic_high);
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
    chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
    sodium_memzero(
        &mut ctx as *mut ChachaCtx as *mut u8,
        core::mem::size_of::<ChachaCtx>(),
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
    let mut ctx = ChachaCtx { input: [0u32; 16] };
    let mut ic_bytes = [0u8; 4];

    if mlen == 0 {
        return 0;
    }
    store32_le(ic_bytes.as_mut_ptr(), ic);
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
    chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
    sodium_memzero(
        &mut ctx as *mut ChachaCtx as *mut u8,
        core::mem::size_of::<ChachaCtx>(),
    );

    0
}

#[no_mangle]
pub static crypto_stream_chacha20_ref_implementation: crypto_stream_chacha20_implementation =
    crypto_stream_chacha20_implementation {
        stream: stream_ref,
        stream_ietf_ext: stream_ietf_ext_ref,
        stream_xor_ic: stream_ref_xor_ic,
        stream_ietf_ext_xor_ic: stream_ietf_ext_ref_xor_ic,
    };

// =====================================================================
// crypto_stream/chacha20/stream_chacha20.c
// =====================================================================

static mut IMPLEMENTATION: *const crypto_stream_chacha20_implementation =
    &crypto_stream_chacha20_ref_implementation;

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_noncebytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_noncebytes() -> usize {
    12
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_messagebytes_max() -> usize {
    IETF_MESSAGEBYTES_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > SODIUM_SIZE_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).stream)(c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).stream_xor_ic)(c, m, mlen, n, ic, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).stream_xor_ic)(c, m, mlen, n, 0, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > SODIUM_SIZE_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).stream_ietf_ext)(c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).stream_ietf_ext_xor_ic)(c, m, mlen, n, ic, k)
}

unsafe fn crypto_stream_chacha20_ietf_ext_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX {
        sodium_misuse();
    }
    ((*IMPLEMENTATION).stream_ietf_ext_xor_ic)(c, m, mlen, n, 0, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > IETF_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf_ext(c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if ic as u64 > (64u64 * (1u64 << 32)) / 64u64 - (mlen.wrapping_add(63)) / 64u64 {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf_ext_xor_ic(c, m, mlen, n, ic, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > IETF_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf_ext_xor(c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_chacha20_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

#[no_mangle]
pub unsafe extern "C" fn _crypto_stream_chacha20_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &crypto_stream_chacha20_ref_implementation;
    0
}

// =====================================================================
// include/sodium/private/chacha20_ietf_ext.h
// (aliases already exported above, re-declared here to match the header)
// =====================================================================
// crypto_stream_chacha20_ietf_ext and crypto_stream_chacha20_ietf_ext_xor_ic
// are defined above.

// =====================================================================
// crypto_stream/xchacha20/stream_xchacha20.c
//
// (`crypto_stream_chacha20` and `crypto_stream_chacha20_xor_ic` are defined
// above in this same file, so they are called directly rather than via an
// `extern "C"` block.)
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2 = [0u8; 32];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());

    crypto_stream_chacha20(c, clen, n.add(16), k2.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut k2 = [0u8; 32];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    crypto_stream_chacha20_xor_ic(c, m, mlen, n.add(16), ic, k2.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xchacha20_xor_ic(c, m, mlen, n, 0, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xchacha20_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

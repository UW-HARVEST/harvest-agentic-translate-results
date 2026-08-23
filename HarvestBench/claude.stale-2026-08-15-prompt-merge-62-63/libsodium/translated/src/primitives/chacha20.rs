//! Translated from crypto_stream/chacha20/stream_chacha20.c and ref/chacha20_ref.c
use crate::primitives::cutil::*;
use core::ffi::c_void;

pub const CHACHA20_KEYBYTES: usize = 32;
pub const CHACHA20_NONCEBYTES: usize = 8;
pub const CHACHA20_IETF_KEYBYTES: usize = 32;
pub const CHACHA20_IETF_NONCEBYTES: usize = 12;

// crypto_stream_chacha20_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX = min(u64::MAX, usize::MAX)
#[inline(always)]
fn messagebytes_max() -> u64 {
    core::cmp::min(u64::MAX, usize::MAX as u64)
}
// ietf max = min(SODIUM_SIZE_MAX, 64 * 2^32)
#[inline(always)]
fn ietf_messagebytes_max() -> u64 {
    core::cmp::min(messagebytes_max(), 64u64 * (1u64 << 32))
}

#[repr(C)]
pub struct crypto_stream_chacha20_implementation {
    pub stream: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32,
    pub stream_ietf_ext: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32,
    pub stream_xor_ic:
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> i32,
    pub stream_ietf_ext_xor_ic:
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32,
}
unsafe impl Sync for crypto_stream_chacha20_implementation {}

struct ChachaCtx {
    input: [u32; 16],
}

unsafe fn chacha_keysetup(ctx: &mut ChachaCtx, k: *const u8) {
    ctx.input[0] = 0x61707865;
    ctx.input[1] = 0x3320646e;
    ctx.input[2] = 0x79622d32;
    ctx.input[3] = 0x6b206574;
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

#[inline(always)]
fn quarterround(a: &mut u32, b: &mut u32, c: &mut u32, d: &mut u32) {
    *a = a.wrapping_add(*b);
    *d = rotl32(*d ^ *a, 16);
    *c = c.wrapping_add(*d);
    *b = rotl32(*b ^ *c, 12);
    *a = a.wrapping_add(*b);
    *d = rotl32(*d ^ *a, 8);
    *c = c.wrapping_add(*d);
    *b = rotl32(*b ^ *c, 7);
}

unsafe fn chacha20_encrypt_bytes(
    ctx: &mut ChachaCtx,
    mut m: *const u8,
    mut c: *mut u8,
    mut bytes: u64,
) {
    let mut tmp: [u8; 64] = [0; 64];
    let mut ctarget: *mut u8 = core::ptr::null_mut();

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
    let mut j12 = ctx.input[12];
    let mut j13 = ctx.input[13];
    let j14 = ctx.input[14];
    let j15 = ctx.input[15];

    loop {
        if bytes < 64 {
            for b in tmp.iter_mut() {
                *b = 0;
            }
            let mut i = 0u64;
            while i < bytes {
                tmp[i as usize] = *m.add(i as usize);
                i += 1;
            }
            m = tmp.as_ptr();
            ctarget = c;
            c = tmp.as_mut_ptr();
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
        let mut i = 20;
        while i > 0 {
            quarterround(&mut x0, &mut x4, &mut x8, &mut x12);
            quarterround(&mut x1, &mut x5, &mut x9, &mut x13);
            quarterround(&mut x2, &mut x6, &mut x10, &mut x14);
            quarterround(&mut x3, &mut x7, &mut x11, &mut x15);
            quarterround(&mut x0, &mut x5, &mut x10, &mut x15);
            quarterround(&mut x1, &mut x6, &mut x11, &mut x12);
            quarterround(&mut x2, &mut x7, &mut x8, &mut x13);
            quarterround(&mut x3, &mut x4, &mut x9, &mut x14);
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
                let mut i = 0u64;
                while i < bytes {
                    *ctarget.add(i as usize) = *c.add(i as usize);
                    i += 1;
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

unsafe extern "C" fn stream_ref(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32 {
    let mut ctx = ChachaCtx { input: [0; 16] };
    if clen == 0 {
        return 0;
    }
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, core::ptr::null());
    core::ptr::write_bytes(c, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c, c, clen);
    sodium_memzero(&mut ctx as *mut _ as *mut c_void, core::mem::size_of::<ChachaCtx>());
    0
}

unsafe extern "C" fn stream_ietf_ext_ref(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut ctx = ChachaCtx { input: [0; 16] };
    if clen == 0 {
        return 0;
    }
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, core::ptr::null());
    core::ptr::write_bytes(c, 0, clen as usize);
    chacha20_encrypt_bytes(&mut ctx, c, c, clen);
    sodium_memzero(&mut ctx as *mut _ as *mut c_void, core::mem::size_of::<ChachaCtx>());
    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> i32 {
    let mut ctx = ChachaCtx { input: [0; 16] };
    let mut ic_bytes = [0u8; 8];
    if mlen == 0 {
        return 0;
    }
    let ic_high = (ic >> 32) as u32;
    let ic_low = ic as u32;
    store32_le(ic_bytes.as_mut_ptr().add(0), ic_low);
    store32_le(ic_bytes.as_mut_ptr().add(4), ic_high);
    chacha_keysetup(&mut ctx, k);
    chacha_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
    chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
    sodium_memzero(&mut ctx as *mut _ as *mut c_void, core::mem::size_of::<ChachaCtx>());
    0
}

unsafe extern "C" fn stream_ietf_ext_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> i32 {
    let mut ctx = ChachaCtx { input: [0; 16] };
    let mut ic_bytes = [0u8; 4];
    if mlen == 0 {
        return 0;
    }
    store32_le(ic_bytes.as_mut_ptr(), ic);
    chacha_keysetup(&mut ctx, k);
    chacha_ietf_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
    chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
    sodium_memzero(&mut ctx as *mut _ as *mut c_void, core::mem::size_of::<ChachaCtx>());
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

static mut IMPLEMENTATION: *const crypto_stream_chacha20_implementation =
    &crypto_stream_chacha20_ref_implementation;

#[inline(always)]
unsafe fn imp() -> &'static crypto_stream_chacha20_implementation {
    &*core::ptr::read(&raw const IMPLEMENTATION)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_keybytes() -> usize {
    CHACHA20_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_noncebytes() -> usize {
    CHACHA20_NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_messagebytes_max() -> usize {
    messagebytes_max() as usize
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_ietf_keybytes() -> usize {
    CHACHA20_IETF_KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_ietf_noncebytes() -> usize {
    CHACHA20_IETF_NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_chacha20_ietf_messagebytes_max() -> usize {
    ietf_messagebytes_max() as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen > messagebytes_max() {
        sodium_misuse();
    }
    (imp().stream)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> i32 {
    if mlen > messagebytes_max() {
        sodium_misuse();
    }
    (imp().stream_xor_ic)(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen > messagebytes_max() {
        sodium_misuse();
    }
    (imp().stream_xor_ic)(c, m, mlen, n, 0, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen > messagebytes_max() {
        sodium_misuse();
    }
    (imp().stream_ietf_ext)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> i32 {
    if mlen > messagebytes_max() {
        sodium_misuse();
    }
    (imp().stream_ietf_ext_xor_ic)(c, m, mlen, n, ic, k)
}

unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen > messagebytes_max() {
        sodium_misuse();
    }
    (imp().stream_ietf_ext_xor_ic)(c, m, mlen, n, 0, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen > ietf_messagebytes_max() {
        sodium_misuse();
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
) -> i32 {
    if (ic as u64) > (64u64 * (1u64 << 32)) / 64u64 - (mlen + 63) / 64 {
        sodium_misuse();
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
) -> i32 {
    if mlen > ietf_messagebytes_max() {
        sodium_misuse();
    }
    crypto_stream_chacha20_ietf_ext_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CHACHA20_IETF_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CHACHA20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_chacha20_pick_best_implementation() -> i32 {
    core::ptr::write(
        &raw mut IMPLEMENTATION,
        &crypto_stream_chacha20_ref_implementation,
    );
    0
}

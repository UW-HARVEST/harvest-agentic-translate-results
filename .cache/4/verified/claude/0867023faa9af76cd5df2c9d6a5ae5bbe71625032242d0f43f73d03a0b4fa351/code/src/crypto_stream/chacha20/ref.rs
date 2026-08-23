//! Translation of `crypto_stream/chacha20/ref/chacha20_ref.c`.
//!
//! ```text
//! chacha-merged.c version 20080118
//! D. J. Bernstein
//! Public domain.
//! ```

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::common::{load32_le, memset, rotl32, store32_le};
use crate::sodium::utils::sodium_memzero;

use super::crypto_stream_chacha20_implementation;

/// `struct chacha_ctx`
#[repr(C)]
struct chacha_ctx {
    input: [u32; 16],
}

/// `QUARTERROUND(a, b, c, d)`
macro_rules! quarterround {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {{
        $a = $a.wrapping_add($b);
        $d = rotl32($d ^ $a, 16);
        $c = $c.wrapping_add($d);
        $b = rotl32($b ^ $c, 12);
        $a = $a.wrapping_add($b);
        $d = rotl32($d ^ $a, 8);
        $c = $c.wrapping_add($d);
        $b = rotl32($b ^ $c, 7);
    }};
}

unsafe fn chacha_keysetup(ctx: *mut chacha_ctx, k: *const u8) {
    unsafe {
        (*ctx).input[0] = 0x6170_7865u32;
        (*ctx).input[1] = 0x3320_646eu32;
        (*ctx).input[2] = 0x7962_2d32u32;
        (*ctx).input[3] = 0x6b20_6574u32;
        (*ctx).input[4] = load32_le(k.add(0));
        (*ctx).input[5] = load32_le(k.add(4));
        (*ctx).input[6] = load32_le(k.add(8));
        (*ctx).input[7] = load32_le(k.add(12));
        (*ctx).input[8] = load32_le(k.add(16));
        (*ctx).input[9] = load32_le(k.add(20));
        (*ctx).input[10] = load32_le(k.add(24));
        (*ctx).input[11] = load32_le(k.add(28));
    }
}

unsafe fn chacha_ivsetup(ctx: *mut chacha_ctx, iv: *const u8, counter: *const u8) {
    unsafe {
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
}

unsafe fn chacha_ietf_ivsetup(ctx: *mut chacha_ctx, iv: *const u8, counter: *const u8) {
    unsafe {
        (*ctx).input[12] = if counter.is_null() {
            0
        } else {
            load32_le(counter)
        };
        (*ctx).input[13] = load32_le(iv.add(0));
        (*ctx).input[14] = load32_le(iv.add(4));
        (*ctx).input[15] = load32_le(iv.add(8));
    }
}

unsafe fn chacha20_encrypt_bytes(
    ctx: *mut chacha_ctx,
    mut m: *const u8,
    mut c: *mut u8,
    mut bytes: u64,
) {
    let (mut x0, mut x1, mut x2, mut x3, mut x4, mut x5, mut x6, mut x7);
    let (mut x8, mut x9, mut x10, mut x11, mut x12, mut x13, mut x14, mut x15);
    let mut ctarget: *mut u8 = ptr::null_mut();
    let mut tmp = [0u8; 64];
    let tmp_p: *mut u8 = tmp.as_mut_ptr();
    let mut i: u32;

    if bytes == 0 {
        return; /* LCOV_EXCL_LINE */
    }
    let j0 = unsafe { (*ctx).input[0] };
    let j1 = unsafe { (*ctx).input[1] };
    let j2 = unsafe { (*ctx).input[2] };
    let j3 = unsafe { (*ctx).input[3] };
    let j4 = unsafe { (*ctx).input[4] };
    let j5 = unsafe { (*ctx).input[5] };
    let j6 = unsafe { (*ctx).input[6] };
    let j7 = unsafe { (*ctx).input[7] };
    let j8 = unsafe { (*ctx).input[8] };
    let j9 = unsafe { (*ctx).input[9] };
    let j10 = unsafe { (*ctx).input[10] };
    let j11 = unsafe { (*ctx).input[11] };
    let mut j12 = unsafe { (*ctx).input[12] };
    let mut j13 = unsafe { (*ctx).input[13] };
    let j14 = unsafe { (*ctx).input[14] };
    let j15 = unsafe { (*ctx).input[15] };

    loop {
        if bytes < 64 {
            unsafe { memset(tmp_p, 0, 64) };
            i = 0;
            while (i as u64) < bytes {
                unsafe { *tmp_p.add(i as usize) = *m.add(i as usize) };
                i = i.wrapping_add(1);
            }
            m = tmp_p as *const u8;
            ctarget = c;
            c = tmp_p;
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
            i = i.wrapping_sub(2);
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

        unsafe {
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
        }

        j12 = j12.wrapping_add(1);
        /* LCOV_EXCL_START */
        if j12 == 0 {
            j13 = j13.wrapping_add(1);
        }
        /* LCOV_EXCL_STOP */

        unsafe {
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
        }

        if bytes <= 64 {
            if bytes < 64 {
                i = 0;
                while i < bytes as u32 {
                    /* ctarget cannot be NULL */
                    unsafe { *ctarget.add(i as usize) = *c.add(i as usize) };
                    i = i.wrapping_add(1);
                }
            }
            unsafe {
                (*ctx).input[12] = j12;
                (*ctx).input[13] = j13;
            }

            return;
        }
        bytes -= 64;
        c = unsafe { c.add(64) };
        m = unsafe { m.add(64) };
    }
}

unsafe extern "C" fn stream_ref(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0u32; 16] };

    if clen == 0 {
        return 0;
    }
    /* COMPILER_ASSERT(crypto_stream_chacha20_KEYBYTES == 256 / 8); */
    unsafe {
        chacha_keysetup(&mut ctx, k);
        chacha_ivsetup(&mut ctx, n, ptr::null());
        memset(c, 0, clen as usize);
        chacha20_encrypt_bytes(&mut ctx, c as *const u8, c, clen);
        sodium_memzero(
            (&mut ctx) as *mut chacha_ctx as *mut c_void,
            core::mem::size_of::<chacha_ctx>(),
        );
    }

    0
}

unsafe extern "C" fn stream_ietf_ext_ref(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut ctx = chacha_ctx { input: [0u32; 16] };

    if clen == 0 {
        return 0;
    }
    /* COMPILER_ASSERT(crypto_stream_chacha20_KEYBYTES == 256 / 8); */
    unsafe {
        chacha_keysetup(&mut ctx, k);
        chacha_ietf_ivsetup(&mut ctx, n, ptr::null());
        memset(c, 0, clen as usize);
        chacha20_encrypt_bytes(&mut ctx, c as *const u8, c, clen);
        sodium_memzero(
            (&mut ctx) as *mut chacha_ctx as *mut c_void,
            core::mem::size_of::<chacha_ctx>(),
        );
    }

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
    let mut ctx = chacha_ctx { input: [0u32; 16] };
    let mut ic_bytes = [0u8; 8];
    let ic_high: u32;
    let ic_low: u32;

    if mlen == 0 {
        return 0;
    }
    ic_high = (ic >> 32) as u32;
    ic_low = ic as u32;
    unsafe {
        store32_le(ic_bytes.as_mut_ptr().add(0), ic_low);
        store32_le(ic_bytes.as_mut_ptr().add(4), ic_high);
        chacha_keysetup(&mut ctx, k);
        chacha_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
        chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
        sodium_memzero(
            (&mut ctx) as *mut chacha_ctx as *mut c_void,
            core::mem::size_of::<chacha_ctx>(),
        );
    }

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
    let mut ctx = chacha_ctx { input: [0u32; 16] };
    let mut ic_bytes = [0u8; 4];

    if mlen == 0 {
        return 0;
    }
    unsafe {
        store32_le(ic_bytes.as_mut_ptr(), ic);
        chacha_keysetup(&mut ctx, k);
        chacha_ietf_ivsetup(&mut ctx, n, ic_bytes.as_ptr());
        chacha20_encrypt_bytes(&mut ctx, m, c, mlen);
        sodium_memzero(
            (&mut ctx) as *mut chacha_ctx as *mut c_void,
            core::mem::size_of::<chacha_ctx>(),
        );
    }

    0
}

#[unsafe(no_mangle)]
pub static mut crypto_stream_chacha20_ref_implementation:
    crypto_stream_chacha20_implementation = crypto_stream_chacha20_implementation {
    stream: Some(stream_ref),
    stream_ietf_ext: Some(stream_ietf_ext_ref),
    stream_xor_ic: Some(stream_ref_xor_ic),
    stream_ietf_ext_xor_ic: Some(stream_ietf_ext_ref_xor_ic),
};

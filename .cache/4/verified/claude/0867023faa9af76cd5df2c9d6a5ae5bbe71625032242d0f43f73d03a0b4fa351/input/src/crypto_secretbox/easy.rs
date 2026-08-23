//! Translation of `crypto_secretbox/crypto_secretbox_easy.c`.
//!
//! `NDEBUG` is set in the reference build, so `assert()` is a no-op and
//! `COMPILER_ASSERT()` only checks a compile-time constant.
//! `ACQUIRE_FENCE` expands to `(void) 0`.

use core::ffi::{c_int, c_void};

use crate::common::memmove;
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::sodium_memzero;

use super::xsalsa20poly1305::{
    crypto_secretbox_xsalsa20poly1305_MACBYTES as crypto_secretbox_MACBYTES,
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX as crypto_secretbox_MESSAGEBYTES_MAX,
    crypto_secretbox_xsalsa20poly1305_ZEROBYTES as crypto_secretbox_ZEROBYTES,
};

/// `include/sodium/crypto_onetimeauth_poly1305.h`:
/// `typedef struct CRYPTO_ALIGN(16) crypto_onetimeauth_poly1305_state {
///      unsigned char opaque[256]; } crypto_onetimeauth_poly1305_state;`
#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

unsafe extern "C" {
    fn crypto_core_hsalsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_stream_salsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_salsa20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int;
}

const crypto_stream_salsa20_KEYBYTES: usize = 32;

/// `#define STREAM_POLY1305_CHUNK 131072`
const STREAM_POLY1305_CHUNK: u64 = 131072;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block0: [u8; 64] = [0; 64];
    let mut subkey: [u8; crypto_stream_salsa20_KEYBYTES] = [0; crypto_stream_salsa20_KEYBYTES];
    let mut i: u64;
    let mut mlen0: u64;
    let mut m = m;

    unsafe {
        crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

        /*
         * Allow the m and c buffers to partially overlap, by calling
         * memmove() if necessary.
         */
        if ((c as u64) > (m as u64) && (c as u64).wrapping_sub(m as u64) < mlen)
            || ((m as u64) > (c as u64) && (m as u64).wrapping_sub(c as u64) < mlen)
        {
            memmove(c, m, mlen as usize);
            m = c as *const u8;
        }
        core::ptr::write_bytes(block0.as_mut_ptr(), 0u8, crypto_secretbox_ZEROBYTES);
        /* COMPILER_ASSERT(64U >= crypto_secretbox_ZEROBYTES); */
        mlen0 = mlen;
        if mlen0 > (64 - crypto_secretbox_ZEROBYTES) as u64 {
            mlen0 = (64 - crypto_secretbox_ZEROBYTES) as u64;
        }
        i = 0;
        while i < mlen0 {
            block0[i as usize + crypto_secretbox_ZEROBYTES] = *m.add(i as usize);
            i += 1;
        }
        crypto_stream_salsa20_xor(
            block0.as_mut_ptr(),
            block0.as_ptr(),
            64,
            n.add(16),
            subkey.as_ptr(),
        );
        /* COMPILER_ASSERT(crypto_secretbox_ZEROBYTES >=
                           crypto_onetimeauth_poly1305_KEYBYTES); */
        crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());

        i = 0;
        while i < mlen0 {
            *c.add(i as usize) = block0[crypto_secretbox_ZEROBYTES + i as usize];
            i += 1;
        }
        sodium_memzero(
            block0.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&block0),
        );

        crypto_onetimeauth_poly1305_update(&mut state, c, mlen0);
        {
            let mut off: u64 = mlen0;
            let mut ic: u64 = 1;

            /* COMPILER_ASSERT(STREAM_POLY1305_CHUNK % 64U == 0U); */
            while off < mlen {
                let mut cl: u64 = mlen - off;
                if cl > STREAM_POLY1305_CHUNK {
                    cl = STREAM_POLY1305_CHUNK;
                }
                crypto_stream_salsa20_xor_ic(
                    c.add(off as usize),
                    m.add(off as usize),
                    cl,
                    n.add(16),
                    ic,
                    subkey.as_ptr(),
                );
                crypto_onetimeauth_poly1305_update(&mut state, c.add(off as usize), cl);
                off = off.wrapping_add(cl);
                ic = ic.wrapping_add(cl / 64);
            }
        }
        sodium_memzero(
            subkey.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&subkey),
        );

        crypto_onetimeauth_poly1305_final(&mut state, mac);
        sodium_memzero(
            &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
            core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
        );
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_secretbox_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { crypto_secretbox_detached(c.add(crypto_secretbox_MACBYTES), c, m, mlen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut block0: [u8; 64] = [0; 64];
    let mut subkey: [u8; crypto_stream_salsa20_KEYBYTES] = [0; crypto_stream_salsa20_KEYBYTES];
    let mut i: u64;
    let mut mlen0: u64;
    let mut c = c;

    unsafe {
        crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

        core::ptr::write_bytes(block0.as_mut_ptr(), 0u8, crypto_secretbox_ZEROBYTES);
        mlen0 = clen;
        if mlen0 > (64 - crypto_secretbox_ZEROBYTES) as u64 {
            mlen0 = (64 - crypto_secretbox_ZEROBYTES) as u64;
        }
        i = 0;
        while i < mlen0 {
            block0[crypto_secretbox_ZEROBYTES + i as usize] = *c.add(i as usize);
            i += 1;
        }
        crypto_stream_salsa20_xor(
            block0.as_mut_ptr(),
            block0.as_ptr(),
            64,
            n.add(16),
            subkey.as_ptr(),
        );
        if crypto_onetimeauth_poly1305_verify(mac, c, clen, block0.as_ptr()) != 0 {
            sodium_memzero(
                subkey.as_mut_ptr() as *mut c_void,
                core::mem::size_of_val(&subkey),
            );
            return -1;
        }
        if m.is_null() {
            sodium_memzero(
                subkey.as_mut_ptr() as *mut c_void,
                core::mem::size_of_val(&subkey),
            );
            return 0;
        }
        /* ACQUIRE_FENCE */

        /*
         * Allow the m and c buffers to partially overlap, by calling
         * memmove() if necessary.
         */
        if ((c as u64) > (m as u64) && (c as u64).wrapping_sub(m as u64) < clen)
            || ((m as u64) > (c as u64) && (m as u64).wrapping_sub(c as u64) < clen)
        {
            memmove(m, c, clen as usize);
            c = m as *const u8;
        }
        i = 0;
        while i < mlen0 {
            *m.add(i as usize) = block0[crypto_secretbox_ZEROBYTES + i as usize];
            i += 1;
        }
        sodium_memzero(
            block0.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&block0),
        );
        if clen > mlen0 {
            crypto_stream_salsa20_xor_ic(
                m.add(mlen0 as usize),
                c.add(mlen0 as usize),
                clen - mlen0,
                n.add(16),
                1,
                subkey.as_ptr(),
            );
        }
        sodium_memzero(
            subkey.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&subkey),
        );
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < crypto_secretbox_MACBYTES as u64 {
        return -1;
    }
    unsafe {
        crypto_secretbox_open_detached(
            m,
            c.add(crypto_secretbox_MACBYTES),
            c,
            clen - crypto_secretbox_MACBYTES as u64,
            n,
            k,
        )
    }
}

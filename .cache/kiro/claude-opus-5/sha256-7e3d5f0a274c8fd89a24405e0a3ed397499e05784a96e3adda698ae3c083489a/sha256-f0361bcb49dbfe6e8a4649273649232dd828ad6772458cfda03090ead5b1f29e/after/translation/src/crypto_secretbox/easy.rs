//! Translation of crypto_secretbox/crypto_secretbox_easy.c

use core::ffi::{c_int, c_void};

use crate::common::memmove;
use crate::crypto_core::hsalsa20::crypto_core_hsalsa20;
use crate::crypto_onetimeauth::poly1305::{
    crypto_onetimeauth_poly1305_final, crypto_onetimeauth_poly1305_init,
    crypto_onetimeauth_poly1305_state, crypto_onetimeauth_poly1305_update,
    crypto_onetimeauth_poly1305_verify,
};
use crate::crypto_stream::salsa20::{crypto_stream_salsa20_xor, crypto_stream_salsa20_xor_ic};
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

use crate::crypto_secretbox::xsalsa20poly1305::{
    crypto_secretbox_xsalsa20poly1305_MACBYTES as crypto_secretbox_MACBYTES,
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX as crypto_secretbox_MESSAGEBYTES_MAX,
    crypto_secretbox_xsalsa20poly1305_ZEROBYTES as crypto_secretbox_ZEROBYTES,
};

const STREAM_POLY1305_CHUNK: u64 = 131072;
const crypto_stream_salsa20_KEYBYTES: usize = 32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_detached(
    c: *mut u8,
    mac: *mut u8,
    mut m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut state: crypto_onetimeauth_poly1305_state = core::mem::zeroed();
    let mut block0: [u8; 64] = [0; 64];
    let mut subkey: [u8; crypto_stream_salsa20_KEYBYTES] = [0; crypto_stream_salsa20_KEYBYTES];
    let mut i: u64;
    let mut mlen0: u64;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    if ((c as usize) > (m as usize) && (c as usize) - (m as usize) < mlen as usize)
        || ((m as usize) > (c as usize) && (m as usize) - (c as usize) < mlen as usize)
    {
        memmove(c, m, mlen as usize);
        m = c;
    }
    core::ptr::write_bytes(block0.as_mut_ptr(), 0u8, crypto_secretbox_ZEROBYTES);
    mlen0 = mlen;
    if mlen0 > (64 - crypto_secretbox_ZEROBYTES) as u64 {
        mlen0 = (64 - crypto_secretbox_ZEROBYTES) as u64;
    }
    i = 0;
    while i < mlen0 {
        block0[(i as usize) + crypto_secretbox_ZEROBYTES] = *m.add(i as usize);
        i += 1;
    }
    crypto_stream_salsa20_xor(
        block0.as_mut_ptr(),
        block0.as_ptr(),
        64u64,
        n.add(16),
        subkey.as_ptr(),
    );
    crypto_onetimeauth_poly1305_init(&mut state, block0.as_ptr());

    i = 0;
    while i < mlen0 {
        *c.add(i as usize) = block0[crypto_secretbox_ZEROBYTES + (i as usize)];
        i += 1;
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block0));

    crypto_onetimeauth_poly1305_update(&mut state, c, mlen0);
    {
        let mut off: u64 = mlen0;
        let mut ic: u64 = 1u64;

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
            off += cl;
            ic += cl / 64u64;
        }
    }
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&subkey));

    crypto_onetimeauth_poly1305_final(&mut state, mac);
    sodium_memzero(
        &mut state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
        core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
    );

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
    if mlen > crypto_secretbox_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_secretbox_detached(c.add(crypto_secretbox_MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open_detached(
    m: *mut u8,
    mut c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut block0: [u8; 64] = [0; 64];
    let mut subkey: [u8; crypto_stream_salsa20_KEYBYTES] = [0; crypto_stream_salsa20_KEYBYTES];
    let mut i: u64;
    let mut mlen0: u64;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());

    core::ptr::write_bytes(block0.as_mut_ptr(), 0u8, crypto_secretbox_ZEROBYTES);
    mlen0 = clen;
    if mlen0 > (64 - crypto_secretbox_ZEROBYTES) as u64 {
        mlen0 = (64 - crypto_secretbox_ZEROBYTES) as u64;
    }
    i = 0;
    while i < mlen0 {
        block0[crypto_secretbox_ZEROBYTES + (i as usize)] = *c.add(i as usize);
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
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&subkey));
        return -1;
    }
    if m.is_null() {
        sodium_memzero(subkey.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&subkey));
        return 0;
    }
    // ACQUIRE_FENCE is a no-op in the reference build.

    if ((c as usize) > (m as usize) && (c as usize) - (m as usize) < clen as usize)
        || ((m as usize) > (c as usize) && (m as usize) - (c as usize) < clen as usize)
    {
        memmove(m, c, clen as usize);
        c = m;
    }
    i = 0;
    while i < mlen0 {
        *m.add(i as usize) = block0[crypto_secretbox_ZEROBYTES + (i as usize)];
        i += 1;
    }
    sodium_memzero(block0.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block0));
    if clen > mlen0 {
        crypto_stream_salsa20_xor_ic(
            m.add(mlen0 as usize),
            c.add(mlen0 as usize),
            clen - mlen0,
            n.add(16),
            1u64,
            subkey.as_ptr(),
        );
    }
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&subkey));

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
    crypto_secretbox_open_detached(
        m,
        c.add(crypto_secretbox_MACBYTES),
        c,
        clen - crypto_secretbox_MACBYTES as u64,
        n,
        k,
    )
}

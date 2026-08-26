//! Translation of
//! `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c`.
//!
//! `NDEBUG` is set in the reference build, so `COMPILER_ASSERT()` only checks a
//! compile-time constant. `ACQUIRE_FENCE` expands to `(void) 0` because neither
//! `HAVE_GCC_MEMORY_FENCES` nor `HAVE_C11_MEMORY_FENCES` is defined.
//! `NATIVE_LITTLE_ENDIAN` is undefined, so `STORE64_LE()` is byte-wise.

use core::ffi::{c_int, c_void};

use crate::common::{store64_le, xor_buf, SODIUM_SIZE_MAX};
use crate::randombytes::randombytes_buf;
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::{sodium_increment, sodium_is_zero, sodium_memcmp, sodium_memzero};

/// `include/sodium/crypto_onetimeauth_poly1305.h`:
/// `typedef struct CRYPTO_ALIGN(16) crypto_onetimeauth_poly1305_state {
///      unsigned char opaque[256]; } crypto_onetimeauth_poly1305_state;`
#[repr(C, align(16))]
struct crypto_onetimeauth_poly1305_state {
    opaque: [u8; 256],
}

unsafe extern "C" {
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_ietf_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u32,
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

const crypto_onetimeauth_poly1305_BYTES: usize = 16;
const crypto_core_hchacha20_INPUTBYTES: usize = 16;
const crypto_stream_chacha20_ietf_KEYBYTES: usize = 32;
const crypto_stream_chacha20_ietf_NONCEBYTES: usize = 12;

/// `1U + crypto_aead_xchacha20poly1305_ietf_ABYTES`
pub const crypto_secretstream_xchacha20poly1305_ABYTES: usize = 1 + 16;
/// `crypto_aead_xchacha20poly1305_ietf_NPUBBYTES`
pub const crypto_secretstream_xchacha20poly1305_HEADERBYTES: usize = 24;
/// `crypto_aead_xchacha20poly1305_ietf_KEYBYTES`
pub const crypto_secretstream_xchacha20poly1305_KEYBYTES: usize = 32;
/// `SODIUM_MIN(SODIUM_SIZE_MAX - crypto_secretstream_xchacha20poly1305_ABYTES,
///             (64ULL * ((1ULL << 32) - 2ULL)))`
pub const crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX: u64 = {
    let a = SODIUM_SIZE_MAX - crypto_secretstream_xchacha20poly1305_ABYTES as u64;
    let b = 64u64 * ((1u64 << 32) - 2);
    if a < b { a } else { b }
};

pub const crypto_secretstream_xchacha20poly1305_TAG_MESSAGE: u8 = 0x00;
pub const crypto_secretstream_xchacha20poly1305_TAG_PUSH: u8 = 0x01;
pub const crypto_secretstream_xchacha20poly1305_TAG_REKEY: u8 = 0x02;
pub const crypto_secretstream_xchacha20poly1305_TAG_FINAL: u8 =
    crypto_secretstream_xchacha20poly1305_TAG_PUSH | crypto_secretstream_xchacha20poly1305_TAG_REKEY;

/// ```c
/// typedef struct crypto_secretstream_xchacha20poly1305_state {
///     unsigned char k[crypto_stream_chacha20_ietf_KEYBYTES];
///     unsigned char nonce[crypto_stream_chacha20_ietf_NONCEBYTES];
///     unsigned char _pad[8];
/// } crypto_secretstream_xchacha20poly1305_state;
/// ```
#[repr(C)]
pub struct crypto_secretstream_xchacha20poly1305_state {
    pub k: [u8; crypto_stream_chacha20_ietf_KEYBYTES],
    pub nonce: [u8; crypto_stream_chacha20_ietf_NONCEBYTES],
    pub _pad: [u8; 8],
}

const crypto_secretstream_xchacha20poly1305_COUNTERBYTES: usize = 4;
const crypto_secretstream_xchacha20poly1305_INONCEBYTES: usize = 8;

/// `#define STATE_COUNTER(STATE) ((STATE)->nonce)`
#[inline(always)]
unsafe fn STATE_COUNTER(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut u8 {
    unsafe { (&raw mut (*state).nonce) as *mut u8 }
}

/// `#define STATE_INONCE(STATE) ((STATE)->nonce + COUNTERBYTES)`
#[inline(always)]
unsafe fn STATE_INONCE(state: *mut crypto_secretstream_xchacha20poly1305_state) -> *mut u8 {
    unsafe { STATE_COUNTER(state).add(crypto_secretstream_xchacha20poly1305_COUNTERBYTES) }
}

/// `static const unsigned char _pad0[16] = { 0 };`
static _pad0: [u8; 16] = [0; 16];

/// `static inline void _crypto_secretstream_xchacha20poly1305_counter_reset()`
#[inline]
unsafe fn _crypto_secretstream_xchacha20poly1305_counter_reset(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    unsafe {
        core::ptr::write_bytes(
            STATE_COUNTER(state),
            0u8,
            crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
        );
        *STATE_COUNTER(state).add(0) = 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_secretstream_xchacha20poly1305_KEYBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    k: *const u8,
) -> c_int {
    /* COMPILER_ASSERT(HEADERBYTES == crypto_core_hchacha20_INPUTBYTES + INONCEBYTES); */
    /* COMPILER_ASSERT(HEADERBYTES == crypto_aead_xchacha20poly1305_ietf_NPUBBYTES); */
    /* COMPILER_ASSERT(sizeof state->nonce == INONCEBYTES + COUNTERBYTES); */

    unsafe {
        randombytes_buf(
            out as *mut c_void,
            crypto_secretstream_xchacha20poly1305_HEADERBYTES,
        );
        crypto_core_hchacha20(
            (&raw mut (*state).k) as *mut u8,
            out,
            k,
            core::ptr::null(),
        );
        _crypto_secretstream_xchacha20poly1305_counter_reset(state);
        core::ptr::copy_nonoverlapping(
            out.add(crypto_core_hchacha20_INPUTBYTES),
            STATE_INONCE(state),
            crypto_secretstream_xchacha20poly1305_INONCEBYTES,
        );
        core::ptr::write_bytes((&raw mut (*state)._pad) as *mut u8, 0u8, 8);
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_init_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    in_: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        crypto_core_hchacha20(
            (&raw mut (*state).k) as *mut u8,
            in_,
            k,
            core::ptr::null(),
        );
        _crypto_secretstream_xchacha20poly1305_counter_reset(state);
        core::ptr::copy_nonoverlapping(
            in_.add(crypto_core_hchacha20_INPUTBYTES),
            STATE_INONCE(state),
            crypto_secretstream_xchacha20poly1305_INONCEBYTES,
        );
        core::ptr::write_bytes((&raw mut (*state)._pad) as *mut u8, 0u8, 8);
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_rekey(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
) {
    let mut new_key_and_inonce: [u8; crypto_stream_chacha20_ietf_KEYBYTES
        + crypto_secretstream_xchacha20poly1305_INONCEBYTES] = [0; crypto_stream_chacha20_ietf_KEYBYTES
        + crypto_secretstream_xchacha20poly1305_INONCEBYTES];
    let mut i: usize;

    unsafe {
        i = 0;
        while i < crypto_stream_chacha20_ietf_KEYBYTES {
            new_key_and_inonce[i] = (*state).k[i];
            i += 1;
        }
        i = 0;
        while i < crypto_secretstream_xchacha20poly1305_INONCEBYTES {
            new_key_and_inonce[crypto_stream_chacha20_ietf_KEYBYTES + i] =
                *STATE_INONCE(state).add(i);
            i += 1;
        }
        crypto_stream_chacha20_ietf_xor(
            new_key_and_inonce.as_mut_ptr(),
            new_key_and_inonce.as_ptr(),
            core::mem::size_of_val(&new_key_and_inonce) as u64,
            (&raw const (*state).nonce) as *const u8,
            (&raw const (*state).k) as *const u8,
        );
        i = 0;
        while i < crypto_stream_chacha20_ietf_KEYBYTES {
            (*state).k[i] = new_key_and_inonce[i];
            i += 1;
        }
        i = 0;
        while i < crypto_secretstream_xchacha20poly1305_INONCEBYTES {
            *STATE_INONCE(state).add(i) =
                new_key_and_inonce[crypto_stream_chacha20_ietf_KEYBYTES + i];
            i += 1;
        }
        _crypto_secretstream_xchacha20poly1305_counter_reset(state);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_push(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    out: *mut u8,
    outlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    tag: u8,
) -> c_int {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block: [u8; 64] = [0; 64];
    let mut slen: [u8; 8] = [0; 8];
    let c: *mut u8;
    let mac: *mut u8;

    unsafe {
        if !outlen_p.is_null() {
            *outlen_p = 0;
        }
        /* COMPILER_ASSERT(MESSAGEBYTES_MAX <=
                           crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX); */
        if mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        crypto_stream_chacha20_ietf(
            block.as_mut_ptr(),
            core::mem::size_of_val(&block) as u64,
            (&raw const (*state).nonce) as *const u8,
            (&raw const (*state).k) as *const u8,
        );
        crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
        sodium_memzero(
            block.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&block),
        );

        crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            _pad0.as_ptr(),
            (0x10u64.wrapping_sub(adlen)) & 0xf,
        );
        core::ptr::write_bytes(block.as_mut_ptr(), 0u8, core::mem::size_of_val(&block));
        block[0] = tag;

        crypto_stream_chacha20_ietf_xor_ic(
            block.as_mut_ptr(),
            block.as_ptr(),
            core::mem::size_of_val(&block) as u64,
            (&raw const (*state).nonce) as *const u8,
            1,
            (&raw const (*state).k) as *const u8,
        );
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            block.as_ptr(),
            core::mem::size_of_val(&block) as u64,
        );
        *out.add(0) = block[0];

        c = out.add(core::mem::size_of_val(&tag));
        crypto_stream_chacha20_ietf_xor_ic(
            c,
            m,
            mlen,
            (&raw const (*state).nonce) as *const u8,
            2,
            (&raw const (*state).k) as *const u8,
        );
        crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            _pad0.as_ptr(),
            (0x10u64
                .wrapping_sub(core::mem::size_of_val(&block) as u64)
                .wrapping_add(mlen))
                & 0xf,
        );
        /* should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned */

        store64_le(slen.as_mut_ptr(), adlen);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            slen.as_ptr(),
            core::mem::size_of_val(&slen) as u64,
        );
        store64_le(
            slen.as_mut_ptr(),
            (core::mem::size_of_val(&block) as u64).wrapping_add(mlen),
        );
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            slen.as_ptr(),
            core::mem::size_of_val(&slen) as u64,
        );

        mac = c.add(mlen as usize);
        crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac);
        sodium_memzero(
            &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
            core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
        );

        /* COMPILER_ASSERT(crypto_onetimeauth_poly1305_BYTES >= INONCEBYTES); */
        xor_buf(
            STATE_INONCE(state),
            mac,
            crypto_secretstream_xchacha20poly1305_INONCEBYTES,
        );
        sodium_increment(
            STATE_COUNTER(state),
            crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
        );
        if (tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0
            || sodium_is_zero(
                STATE_COUNTER(state),
                crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
            ) != 0
        {
            crypto_secretstream_xchacha20poly1305_rekey(state);
        }
        if !outlen_p.is_null() {
            *outlen_p = crypto_secretstream_xchacha20poly1305_ABYTES as u64 + mlen;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_pull(
    state: *mut crypto_secretstream_xchacha20poly1305_state,
    m: *mut u8,
    mlen_p: *mut u64,
    tag_p: *mut u8,
    in_: *const u8,
    inlen: u64,
    ad: *const u8,
    adlen: u64,
) -> c_int {
    let mut poly1305_state = crypto_onetimeauth_poly1305_state { opaque: [0u8; 256] };
    let mut block: [u8; 64] = [0; 64];
    let mut slen: [u8; 8] = [0; 8];
    let mut mac: [u8; crypto_onetimeauth_poly1305_BYTES] = [0; crypto_onetimeauth_poly1305_BYTES];
    let c: *const u8;
    let stored_mac: *const u8;
    let mlen: u64;
    let tag: u8;

    unsafe {
        if !mlen_p.is_null() {
            *mlen_p = 0;
        }
        if !tag_p.is_null() {
            *tag_p = 0xff;
        }
        if inlen < crypto_secretstream_xchacha20poly1305_ABYTES as u64 {
            return -1;
        }
        mlen = inlen - crypto_secretstream_xchacha20poly1305_ABYTES as u64;
        if mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        crypto_stream_chacha20_ietf(
            block.as_mut_ptr(),
            core::mem::size_of_val(&block) as u64,
            (&raw const (*state).nonce) as *const u8,
            (&raw const (*state).k) as *const u8,
        );
        crypto_onetimeauth_poly1305_init(&mut poly1305_state, block.as_ptr());
        sodium_memzero(
            block.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&block),
        );

        crypto_onetimeauth_poly1305_update(&mut poly1305_state, ad, adlen);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            _pad0.as_ptr(),
            (0x10u64.wrapping_sub(adlen)) & 0xf,
        );

        core::ptr::write_bytes(block.as_mut_ptr(), 0u8, core::mem::size_of_val(&block));
        block[0] = *in_.add(0);
        crypto_stream_chacha20_ietf_xor_ic(
            block.as_mut_ptr(),
            block.as_ptr(),
            core::mem::size_of_val(&block) as u64,
            (&raw const (*state).nonce) as *const u8,
            1,
            (&raw const (*state).k) as *const u8,
        );
        tag = block[0];
        block[0] = *in_.add(0);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            block.as_ptr(),
            core::mem::size_of_val(&block) as u64,
        );

        c = in_.add(core::mem::size_of_val(&tag));
        crypto_onetimeauth_poly1305_update(&mut poly1305_state, c, mlen);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            _pad0.as_ptr(),
            (0x10u64
                .wrapping_sub(core::mem::size_of_val(&block) as u64)
                .wrapping_add(mlen))
                & 0xf,
        );
        /* should have been (0x10 - (sizeof block + mlen)) & 0xf to keep input blocks aligned */

        store64_le(slen.as_mut_ptr(), adlen);
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            slen.as_ptr(),
            core::mem::size_of_val(&slen) as u64,
        );
        store64_le(
            slen.as_mut_ptr(),
            (core::mem::size_of_val(&block) as u64).wrapping_add(mlen),
        );
        crypto_onetimeauth_poly1305_update(
            &mut poly1305_state,
            slen.as_ptr(),
            core::mem::size_of_val(&slen) as u64,
        );

        crypto_onetimeauth_poly1305_final(&mut poly1305_state, mac.as_mut_ptr());
        sodium_memzero(
            &mut poly1305_state as *mut crypto_onetimeauth_poly1305_state as *mut c_void,
            core::mem::size_of::<crypto_onetimeauth_poly1305_state>(),
        );

        stored_mac = c.add(mlen as usize);
        if sodium_memcmp(
            mac.as_ptr() as *const c_void,
            stored_mac as *const c_void,
            core::mem::size_of_val(&mac),
        ) != 0
        {
            sodium_memzero(
                mac.as_mut_ptr() as *mut c_void,
                core::mem::size_of_val(&mac),
            );
            return -1;
        }

        /* ACQUIRE_FENCE */
        crypto_stream_chacha20_ietf_xor_ic(
            m,
            c,
            mlen,
            (&raw const (*state).nonce) as *const u8,
            2,
            (&raw const (*state).k) as *const u8,
        );
        xor_buf(
            STATE_INONCE(state),
            mac.as_ptr(),
            crypto_secretstream_xchacha20poly1305_INONCEBYTES,
        );
        sodium_increment(
            STATE_COUNTER(state),
            crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
        );
        if (tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0
            || sodium_is_zero(
                STATE_COUNTER(state),
                crypto_secretstream_xchacha20poly1305_COUNTERBYTES,
            ) != 0
        {
            crypto_secretstream_xchacha20poly1305_rekey(state);
        }
        if !mlen_p.is_null() {
            *mlen_p = mlen;
        }
        if !tag_p.is_null() {
            *tag_p = tag;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_secretstream_xchacha20poly1305_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_abytes() -> usize {
    crypto_secretstream_xchacha20poly1305_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_headerbytes() -> usize {
    crypto_secretstream_xchacha20poly1305_HEADERBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_keybytes() -> usize {
    crypto_secretstream_xchacha20poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_messagebytes_max() -> usize {
    crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_message() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_MESSAGE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_push() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_PUSH
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_rekey() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_REKEY
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretstream_xchacha20poly1305_tag_final() -> u8 {
    crypto_secretstream_xchacha20poly1305_TAG_FINAL
}

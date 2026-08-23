//! Translation of `crypto_pwhash/argon2/blake2b-long.c`.
//!
//! Exports (after the `private/quirks.h` renaming):
//!   * `_sodium_blake2b_long`

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* `crypto_generichash_blake2b_BYTES_MAX` from crypto_generichash_blake2b.h */
const crypto_generichash_blake2b_BYTES_MAX: usize = 64;

/* `typedef struct CRYPTO_ALIGN(64) crypto_generichash_blake2b_state` from
 * include/sodium/crypto_generichash_blake2b.h -- declared inside a
 * `#pragma pack(push, 1)` region but the sole member is a byte array, so the
 * observable layout is sizeof == 384, _Alignof == 64. */
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

extern "C" {
    /* crypto_generichash/blake2b/ref/generichash_blake2b.c */
    fn crypto_generichash_blake2b(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: c_ulonglong,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    /* `state` is `crypto_generichash_blake2b_state *`; passed as an opaque
     * pointer here so that the declaration matches the other translation
     * units. */
    fn crypto_generichash_blake2b_init(
        state: *mut c_void,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut c_void,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/* int blake2b_long(void *pout, size_t outlen, const void *in, size_t inlen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_long(
    pout: *mut c_void,
    outlen: usize,
    in_: *const c_void,
    inlen: usize,
) -> c_int {
    let mut out: *mut u8 = pout as *mut u8;
    let mut blake_state = crypto_generichash_blake2b_state { opaque: [0u8; 384] };
    let mut outlen_bytes: [u8; 4 /* sizeof(uint32_t) */] = [0u8; 4];
    let mut ret: c_int = -1;

    /* The C source uses `goto fail`; the labelled block below reproduces it. */
    'fail: {
        if outlen > u32::MAX as usize {
            break 'fail; /* LCOV_EXCL_LINE */
        }

        /* Ensure little-endian byte order! */
        store32_le(outlen_bytes.as_mut_ptr(), outlen as u32);

        if outlen <= crypto_generichash_blake2b_BYTES_MAX {
            ret = crypto_generichash_blake2b_init(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                core::ptr::null(),
                0usize,
                outlen,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                outlen_bytes.as_ptr(),
                outlen_bytes.len() as c_ulonglong,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                in_ as *const u8,
                inlen as c_ulonglong,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_final(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                out,
                outlen,
            );
            if ret < 0 {
                break 'fail;
            }
        } else {
            let mut toproduce: u32;
            let mut out_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
                [0u8; crypto_generichash_blake2b_BYTES_MAX];
            let mut in_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
                [0u8; crypto_generichash_blake2b_BYTES_MAX];

            ret = crypto_generichash_blake2b_init(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                core::ptr::null(),
                0usize,
                crypto_generichash_blake2b_BYTES_MAX,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                outlen_bytes.as_ptr(),
                outlen_bytes.len() as c_ulonglong,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                in_ as *const u8,
                inlen as c_ulonglong,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_final(
                &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
                out_buffer.as_mut_ptr(),
                crypto_generichash_blake2b_BYTES_MAX,
            );
            if ret < 0 {
                break 'fail;
            }
            memcpy(
                out,
                out_buffer.as_ptr(),
                crypto_generichash_blake2b_BYTES_MAX / 2,
            );
            out = out.add(crypto_generichash_blake2b_BYTES_MAX / 2);
            toproduce =
                (outlen as u32).wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);

            while toproduce > crypto_generichash_blake2b_BYTES_MAX as u32 {
                memcpy(
                    in_buffer.as_mut_ptr(),
                    out_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX,
                );
                ret = crypto_generichash_blake2b(
                    out_buffer.as_mut_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX,
                    in_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX as c_ulonglong,
                    core::ptr::null(),
                    0usize,
                );
                if ret < 0 {
                    break 'fail;
                }
                memcpy(
                    out,
                    out_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX / 2,
                );
                out = out.add(crypto_generichash_blake2b_BYTES_MAX / 2);
                toproduce =
                    toproduce.wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);
            }

            memcpy(
                in_buffer.as_mut_ptr(),
                out_buffer.as_ptr(),
                crypto_generichash_blake2b_BYTES_MAX,
            );
            ret = crypto_generichash_blake2b(
                out_buffer.as_mut_ptr(),
                toproduce as usize,
                in_buffer.as_ptr(),
                crypto_generichash_blake2b_BYTES_MAX as c_ulonglong,
                core::ptr::null(),
                0usize,
            );
            if ret < 0 {
                break 'fail;
            }
            memcpy(out, out_buffer.as_ptr(), toproduce as usize);
        }
    }

    sodium_memzero(
        &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_blake2b_state>(),
    );

    ret
}

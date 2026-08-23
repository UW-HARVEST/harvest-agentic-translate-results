//! Translation of `crypto_pwhash/argon2/blake2b-long.c`.
//!
//! `private/quirks.h` renames `blake2b_long` to `_sodium_blake2b_long`.

use core::ffi::{c_int, c_void};

use crate::common::{memcpy, store32_le};
use crate::sodium::utils::sodium_memzero;

/// `crypto_generichash_blake2b_state` from
/// `include/sodium/crypto_generichash_blake2b.h`: an opaque 384-byte buffer
/// with `CRYPTO_ALIGN(64)`.
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

/// `#define crypto_generichash_blake2b_BYTES_MAX 64U`
const crypto_generichash_blake2b_BYTES_MAX: usize = 64;

unsafe extern "C" {
    fn crypto_generichash_blake2b(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_init(
        state: *mut crypto_generichash_blake2b_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut crypto_generichash_blake2b_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(
        state: *mut crypto_generichash_blake2b_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
}

/// `int blake2b_long(void *pout, size_t outlen, const void *in, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_long(
    pout: *mut c_void,
    outlen: usize,
    in_: *const c_void,
    inlen: usize,
) -> c_int {
    let mut out: *mut u8 = pout as *mut u8;
    let mut blake_state: crypto_generichash_blake2b_state =
        crypto_generichash_blake2b_state { opaque: [0u8; 384] };
    let mut outlen_bytes: [u8; 4] = [0u8; 4]; /* 4 == sizeof(uint32_t) */
    let mut ret: c_int = -1;

    'fail: {
        if outlen > u32::MAX as usize {
            break 'fail; /* LCOV_EXCL_LINE */
        }

        /* Ensure little-endian byte order! */
        unsafe { store32_le(outlen_bytes.as_mut_ptr(), outlen as u32) };

        /* `TRY(statement)`: ret = statement; if (ret < 0) goto fail; */
        macro_rules! TRY {
            ($e:expr) => {{
                ret = $e;
                if ret < 0 {
                    break 'fail;
                }
            }};
        }

        if outlen <= crypto_generichash_blake2b_BYTES_MAX {
            TRY!(unsafe {
                crypto_generichash_blake2b_init(
                    &mut blake_state,
                    core::ptr::null(),
                    0,
                    outlen,
                )
            });
            TRY!(unsafe {
                crypto_generichash_blake2b_update(&mut blake_state, outlen_bytes.as_ptr(), 4)
            });
            TRY!(unsafe {
                crypto_generichash_blake2b_update(
                    &mut blake_state,
                    in_ as *const u8,
                    inlen as u64,
                )
            });
            TRY!(unsafe { crypto_generichash_blake2b_final(&mut blake_state, out, outlen) });
        } else {
            let mut toproduce: u32;
            let mut out_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
                [0u8; crypto_generichash_blake2b_BYTES_MAX];
            let mut in_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
                [0u8; crypto_generichash_blake2b_BYTES_MAX];
            TRY!(unsafe {
                crypto_generichash_blake2b_init(
                    &mut blake_state,
                    core::ptr::null(),
                    0,
                    crypto_generichash_blake2b_BYTES_MAX,
                )
            });
            TRY!(unsafe {
                crypto_generichash_blake2b_update(&mut blake_state, outlen_bytes.as_ptr(), 4)
            });
            TRY!(unsafe {
                crypto_generichash_blake2b_update(
                    &mut blake_state,
                    in_ as *const u8,
                    inlen as u64,
                )
            });
            TRY!(unsafe {
                crypto_generichash_blake2b_final(
                    &mut blake_state,
                    out_buffer.as_mut_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX,
                )
            });
            unsafe {
                memcpy(
                    out,
                    out_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX / 2,
                )
            };
            out = unsafe { out.add(crypto_generichash_blake2b_BYTES_MAX / 2) };
            toproduce = (outlen as u32)
                .wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);

            while toproduce > crypto_generichash_blake2b_BYTES_MAX as u32 {
                unsafe {
                    memcpy(
                        in_buffer.as_mut_ptr(),
                        out_buffer.as_ptr(),
                        crypto_generichash_blake2b_BYTES_MAX,
                    )
                };
                TRY!(unsafe {
                    crypto_generichash_blake2b(
                        out_buffer.as_mut_ptr(),
                        crypto_generichash_blake2b_BYTES_MAX,
                        in_buffer.as_ptr(),
                        crypto_generichash_blake2b_BYTES_MAX as u64,
                        core::ptr::null(),
                        0,
                    )
                });
                unsafe {
                    memcpy(
                        out,
                        out_buffer.as_ptr(),
                        crypto_generichash_blake2b_BYTES_MAX / 2,
                    )
                };
                out = unsafe { out.add(crypto_generichash_blake2b_BYTES_MAX / 2) };
                toproduce = toproduce
                    .wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);
            }

            unsafe {
                memcpy(
                    in_buffer.as_mut_ptr(),
                    out_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX,
                )
            };
            TRY!(unsafe {
                crypto_generichash_blake2b(
                    out_buffer.as_mut_ptr(),
                    toproduce as usize,
                    in_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX as u64,
                    core::ptr::null(),
                    0,
                )
            });
            unsafe { memcpy(out, out_buffer.as_ptr(), toproduce as usize) };
        }
    }
    /* fail: */
    unsafe {
        sodium_memzero(
            &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
            core::mem::size_of::<crypto_generichash_blake2b_state>(),
        )
    };
    ret
}

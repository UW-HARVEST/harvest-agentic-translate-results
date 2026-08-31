//! Translation of c_src/libsodium/crypto_pwhash/argon2/blake2b-long.c

use crate::common::store32_le;
use core::ffi::{c_int, c_void};

const crypto_generichash_blake2b_BYTES_MAX: usize = 64;

// crypto_generichash_blake2b_state — #[repr(C, packed)] mirror sized as the
// public opaque[384] storage (see crypto_generichash_blake2b.h: pragma pack(1),
// CRYPTO_ALIGN(64)).
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

extern "C" {
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
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

// blake2b_long -> _sodium_blake2b_long (quirks.h)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_long(
    pout: *mut c_void,
    outlen: usize,
    in_: *const c_void,
    inlen: usize,
) -> c_int {
    let mut out: *mut u8 = pout as *mut u8;
    // MaybeUninit-style: matches C's uninitialized blake_state; only cleared on fail.
    let mut blake_state = crypto_generichash_blake2b_state { opaque: [0u8; 384] };
    let mut outlen_bytes: [u8; 4 /* sizeof(uint32_t) */] = [0; 4];
    let mut ret: c_int = -1;

    // The TRY macro sets `ret = statement; if (ret < 0) goto fail;`
    // Reproduced inline with a labelled block acting as the `fail:` target.
    'fail: {
        if outlen > u32::MAX as usize {
            break 'fail; /* LCOV_EXCL_LINE */
        }

        /* Ensure little-endian byte order! */
        store32_le(outlen_bytes.as_mut_ptr(), outlen as u32);

        if outlen <= crypto_generichash_blake2b_BYTES_MAX {
            ret = crypto_generichash_blake2b_init(&mut blake_state, core::ptr::null(), 0, outlen);
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state,
                outlen_bytes.as_ptr(),
                outlen_bytes.len() as u64,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state,
                in_ as *const u8,
                inlen as u64,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_final(&mut blake_state, out, outlen);
            if ret < 0 {
                break 'fail;
            }
        } else {
            let mut out_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
                [0; crypto_generichash_blake2b_BYTES_MAX];
            let mut in_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
                [0; crypto_generichash_blake2b_BYTES_MAX];
            ret = crypto_generichash_blake2b_init(
                &mut blake_state,
                core::ptr::null(),
                0,
                crypto_generichash_blake2b_BYTES_MAX,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state,
                outlen_bytes.as_ptr(),
                outlen_bytes.len() as u64,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(
                &mut blake_state,
                in_ as *const u8,
                inlen as u64,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_final(
                &mut blake_state,
                out_buffer.as_mut_ptr(),
                crypto_generichash_blake2b_BYTES_MAX,
            );
            if ret < 0 {
                break 'fail;
            }
            core::ptr::copy_nonoverlapping(
                out_buffer.as_ptr(),
                out,
                crypto_generichash_blake2b_BYTES_MAX / 2,
            );
            out = out.add(crypto_generichash_blake2b_BYTES_MAX / 2);
            let mut toproduce =
                (outlen as u32).wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);

            while toproduce > crypto_generichash_blake2b_BYTES_MAX as u32 {
                core::ptr::copy_nonoverlapping(
                    out_buffer.as_ptr(),
                    in_buffer.as_mut_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX,
                );
                ret = crypto_generichash_blake2b(
                    out_buffer.as_mut_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX,
                    in_buffer.as_ptr(),
                    crypto_generichash_blake2b_BYTES_MAX as u64,
                    core::ptr::null(),
                    0,
                );
                if ret < 0 {
                    break 'fail;
                }
                core::ptr::copy_nonoverlapping(
                    out_buffer.as_ptr(),
                    out,
                    crypto_generichash_blake2b_BYTES_MAX / 2,
                );
                out = out.add(crypto_generichash_blake2b_BYTES_MAX / 2);
                toproduce = toproduce
                    .wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);
            }

            core::ptr::copy_nonoverlapping(
                out_buffer.as_ptr(),
                in_buffer.as_mut_ptr(),
                crypto_generichash_blake2b_BYTES_MAX,
            );
            ret = crypto_generichash_blake2b(
                out_buffer.as_mut_ptr(),
                toproduce as usize,
                in_buffer.as_ptr(),
                crypto_generichash_blake2b_BYTES_MAX as u64,
                core::ptr::null(),
                0,
            );
            if ret < 0 {
                break 'fail;
            }
            core::ptr::copy_nonoverlapping(out_buffer.as_ptr(), out, toproduce as usize);
        }
    }
    // fail:
    sodium_memzero(
        &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_blake2b_state>(),
    );
    ret
}

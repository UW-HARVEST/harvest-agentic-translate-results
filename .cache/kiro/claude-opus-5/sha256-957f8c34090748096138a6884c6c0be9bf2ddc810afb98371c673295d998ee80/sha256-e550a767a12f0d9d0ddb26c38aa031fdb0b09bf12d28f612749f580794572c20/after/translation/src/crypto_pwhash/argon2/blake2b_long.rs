//! Translation of `crypto_pwhash/argon2/blake2b-long.c`.

use core::ffi::{c_int, c_void};

use crate::common::{memcpy, store32_le};
use crate::crypto_generichash::blake2b::{
    crypto_generichash_blake2b, crypto_generichash_blake2b_BYTES_MAX,
    crypto_generichash_blake2b_final, crypto_generichash_blake2b_init,
    crypto_generichash_blake2b_state, crypto_generichash_blake2b_update,
};
use crate::sodium_utils::sodium_memzero;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_long(
    pout: *mut c_void,
    outlen: usize,
    in_: *const c_void,
    inlen: usize,
) -> c_int {
    let mut out: *mut u8 = pout as *mut u8;
    let mut blake_state: crypto_generichash_blake2b_state = core::mem::zeroed();
    let mut outlen_bytes: [u8; 4] = [0; 4];
    let mut ret: c_int = -1;

    /* TRY(statement): ret = statement; if ret < 0 goto fail; */
    macro_rules! try_ {
        ($stmt:expr) => {{
            ret = $stmt;
            if ret < 0 {
                sodium_memzero(
                    (&mut blake_state) as *mut _ as *mut c_void,
                    core::mem::size_of::<crypto_generichash_blake2b_state>(),
                );
                return ret;
            }
        }};
    }

    if outlen > u32::MAX as usize {
        /* goto fail */
        sodium_memzero(
            (&mut blake_state) as *mut _ as *mut c_void,
            core::mem::size_of::<crypto_generichash_blake2b_state>(),
        );
        return ret;
    }

    /* Ensure little-endian byte order! */
    store32_le(outlen_bytes.as_mut_ptr(), outlen as u32);

    if outlen <= crypto_generichash_blake2b_BYTES_MAX {
        try_!(crypto_generichash_blake2b_init(
            &mut blake_state,
            core::ptr::null(),
            0,
            outlen
        ));
        try_!(crypto_generichash_blake2b_update(
            &mut blake_state,
            outlen_bytes.as_ptr(),
            outlen_bytes.len() as u64
        ));
        try_!(crypto_generichash_blake2b_update(
            &mut blake_state,
            in_ as *const u8,
            inlen as u64
        ));
        try_!(crypto_generichash_blake2b_final(
            &mut blake_state,
            out,
            outlen
        ));
    } else {
        let mut toproduce: u32;
        let mut out_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
            [0; crypto_generichash_blake2b_BYTES_MAX];
        let mut in_buffer: [u8; crypto_generichash_blake2b_BYTES_MAX] =
            [0; crypto_generichash_blake2b_BYTES_MAX];
        try_!(crypto_generichash_blake2b_init(
            &mut blake_state,
            core::ptr::null(),
            0,
            crypto_generichash_blake2b_BYTES_MAX
        ));
        try_!(crypto_generichash_blake2b_update(
            &mut blake_state,
            outlen_bytes.as_ptr(),
            outlen_bytes.len() as u64
        ));
        try_!(crypto_generichash_blake2b_update(
            &mut blake_state,
            in_ as *const u8,
            inlen as u64
        ));
        try_!(crypto_generichash_blake2b_final(
            &mut blake_state,
            out_buffer.as_mut_ptr(),
            crypto_generichash_blake2b_BYTES_MAX
        ));
        memcpy(out, out_buffer.as_ptr(), crypto_generichash_blake2b_BYTES_MAX / 2);
        out = out.add(crypto_generichash_blake2b_BYTES_MAX / 2);
        toproduce = (outlen as u32).wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);

        while toproduce > crypto_generichash_blake2b_BYTES_MAX as u32 {
            memcpy(
                in_buffer.as_mut_ptr(),
                out_buffer.as_ptr(),
                crypto_generichash_blake2b_BYTES_MAX,
            );
            try_!(crypto_generichash_blake2b(
                out_buffer.as_mut_ptr(),
                crypto_generichash_blake2b_BYTES_MAX,
                in_buffer.as_ptr(),
                crypto_generichash_blake2b_BYTES_MAX as u64,
                core::ptr::null(),
                0
            ));
            memcpy(out, out_buffer.as_ptr(), crypto_generichash_blake2b_BYTES_MAX / 2);
            out = out.add(crypto_generichash_blake2b_BYTES_MAX / 2);
            toproduce = toproduce.wrapping_sub((crypto_generichash_blake2b_BYTES_MAX / 2) as u32);
        }

        memcpy(
            in_buffer.as_mut_ptr(),
            out_buffer.as_ptr(),
            crypto_generichash_blake2b_BYTES_MAX,
        );
        try_!(crypto_generichash_blake2b(
            out_buffer.as_mut_ptr(),
            toproduce as usize,
            in_buffer.as_ptr(),
            crypto_generichash_blake2b_BYTES_MAX as u64,
            core::ptr::null(),
            0
        ));
        memcpy(out, out_buffer.as_ptr(), toproduce as usize);
    }

    /* fail: */
    sodium_memzero(
        (&mut blake_state) as *mut _ as *mut c_void,
        core::mem::size_of::<crypto_generichash_blake2b_state>(),
    );
    ret
}

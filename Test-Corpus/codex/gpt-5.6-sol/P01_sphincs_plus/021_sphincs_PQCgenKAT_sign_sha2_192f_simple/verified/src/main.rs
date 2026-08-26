//! Translation of `c_src/app/src/PQCgenKAT_sign.c`.
//!
//! ```text
//! //  PQCgenKAT_sign.c
//! //
//! //  Created by Bassham, Lawrence E (Fed) on 8/29/17.
//! //  Copyright © 2017 Bassham, Lawrence E (Fed). All rights reserved.
//! ```
//!
//! The C file contains four alternative "KAT transcript" implementations that
//! are selected by the preprocessor symbols `BLAKE_TR`, `HARAKA_TR`, `SHA2_TR`
//! and `SHAKE_TR` (CMake derives them from `HASH_BACKEND`).  Here every variant
//! lives in its own module and the modules are gated on the corresponding cargo
//! feature, using the same priority order as `lib.rs` so that *any* feature
//! combination compiles:
//!
//! | C symbol     | cargo feature | cfg                                                                      |
//! |--------------|---------------|--------------------------------------------------------------------------|
//! | `HARAKA_TR`  | `haraka`      | `any(haraka, not(any(sha2, shake, blake)))`  (the default)                |
//! | `SHA2_TR`    | `sha2`        | `all(sha2, not(haraka))`                                                  |
//! | `SHAKE_TR`   | `shake`       | `all(shake, not(any(haraka, sha2)))`                                      |
//! | `BLAKE_TR`   | `blake`       | `all(blake, not(any(haraka, sha2, shake)))`                               |
//!
//! The nested `#if SPX_N >= 24` sub-selection of the BLAKE and SHA2 variants
//! (blake512/blake256, sha512/sha256) is *not* expressed with `cfg` — the same
//! source has to compile for every `SECPAR` feature — but with the compile-time
//! constant `USE_X_512` derived from `sphincs_plus::params::SPX_N`.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

// A cdylib is not linkable as a Rust dependency. Compile the same translated
// modules into the driver while retaining the exact crate type requested for
// the shared library.
extern crate self as sphincs_plus;

pub mod address;
pub mod context;
pub mod fors;
pub mod merkle;
pub mod params;
pub mod randombytes;
pub mod rng;
pub mod sign;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

#[cfg(all(
    feature = "blake",
    not(any(feature = "haraka", feature = "sha2", feature = "shake"))
))]
pub mod blake;
#[cfg(any(
    feature = "haraka",
    not(any(feature = "sha2", feature = "shake", feature = "blake"))
))]
pub mod haraka;
#[cfg(all(feature = "sha2", not(feature = "haraka")))]
pub mod sha2;
#[cfg(all(feature = "shake", not(any(feature = "haraka", feature = "sha2"))))]
pub mod shake;

pub(crate) mod backend {
    #[cfg(all(
        feature = "blake",
        not(any(feature = "haraka", feature = "sha2", feature = "shake"))
    ))]
    pub(crate) use crate::blake::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };
    #[cfg(any(
        feature = "haraka",
        not(any(feature = "sha2", feature = "shake", feature = "blake"))
    ))]
    pub(crate) use crate::haraka::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };
    #[cfg(all(feature = "sha2", not(feature = "haraka")))]
    pub(crate) use crate::sha2::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };
    #[cfg(all(feature = "shake", not(any(feature = "haraka", feature = "sha2"))))]
    pub(crate) use crate::shake::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };
}

use std::io::Write;

use sphincs_plus::params::{
    CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES,
};
use sphincs_plus::rng::{randombytes, randombytes_init};
use sphincs_plus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

#[allow(dead_code)]
const MAX_MARKER_LEN: usize = 50;
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

/* ------------------------------------------------------------------------- */
/* #ifdef BLAKE_TR                                                           */
/* ------------------------------------------------------------------------- */

#[cfg(all(
    feature = "blake",
    not(any(feature = "haraka", feature = "sha2", feature = "shake"))
))]
mod kat_tr_blake {
    use sphincs_plus::blake::{
        blake256_final, blake256_init, blake256_update, blake512_final, blake512_init,
        blake512_update,
    };

    /* #if SPX_N >= 24 -> blake512, #else -> blake256 */
    const USE_X_512: bool = sphincs_plus::params::SPX_N >= 24;
    /* blakeX_output_bytes */
    const blakeX_output_bytes: usize = if USE_X_512 { 64 } else { 32 };

    /* `blakestate256` / `blakestate512` of `lib/blake/include/blake.h`
     * (identical layout to `sphincs_plus::blake::blake256::blakestate256` and
     * `...::blake512::blakestate512`; the pointers handed to the library are
     * plain `#[repr(C)]` addresses, so the local declarations are enough). */
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct blakestate256 {
        h: [u32; 8],
        s: [u32; 4],
        t: [u32; 2],
        buflen: i32,
        nullt: i32,
        buf: [u8; 64],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct blakestate512 {
        h: [u64; 8],
        s: [u64; 4],
        t: [u64; 2],
        buflen: i32,
        nullt: i32,
        buf: [u8; 128],
    }

    /* typedef blakestateX kat_tr_ctx; (both states are carried so that the
     * `SPX_N >= 24` selection can stay a run-time `if` on a `const`) */
    pub struct KatTrCtx {
        s256: blakestate256,
        s512: blakestate512,
    }

    /* blakeX_update(ctx, in, inlen) */
    fn blakeX_update(ctx: &mut KatTrCtx, input: *const u8, inlen: u64) {
        unsafe {
            if USE_X_512 {
                blake512_update(&mut ctx.s512 as *mut blakestate512 as *mut _, input, inlen);
            } else {
                blake256_update(&mut ctx.s256 as *mut blakestate256 as *mut _, input, inlen);
            }
        }
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut ctx = KatTrCtx {
            s256: blakestate256 {
                h: [0; 8],
                s: [0; 4],
                t: [0; 2],
                buflen: 0,
                nullt: 0,
                buf: [0; 64],
            },
            s512: blakestate512 {
                h: [0; 8],
                s: [0; 4],
                t: [0; 2],
                buflen: 0,
                nullt: 0,
                buf: [0; 128],
            },
        };

        unsafe {
            if USE_X_512 {
                blake512_init(&mut ctx.s512 as *mut blakestate512 as *mut _);
            } else {
                blake256_init(&mut ctx.s256 as *mut blakestate256 as *mut _);
            }
        }

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-BLAKE"; /* sizeof tag - 1 */
        blakeX_update(&mut ctx, tag.as_ptr(), tag.len() as u64);

        let sep: u8 = 0x00;
        blakeX_update(&mut ctx, &sep, 1);

        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        blakeX_update(ctx, p.as_ptr(), n as u64);

        let sep: u8 = 0x00;
        blakeX_update(ctx, &sep, 1);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }

        let mut lenle = [0u8; 8];
        let L: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }

        blakeX_update(ctx, lenle.as_ptr(), 8);
        blakeX_update(ctx, le.as_ptr(), 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; 8];
        let L: u64 = len as u64;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }
        blakeX_update(ctx, lenle.as_ptr(), 8);
        if len != 0 {
            blakeX_update(ctx, buf.as_ptr(), len as u64);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        /* unsigned char outbuf[blakeX_output_bytes] = {0}; */
        let mut outbuf = [0u8; 64];
        debug_assert!(blakeX_output_bytes <= outbuf.len());
        unsafe {
            if USE_X_512 {
                blake512_final(
                    &mut ctx.s512 as *mut blakestate512 as *mut _,
                    outbuf.as_mut_ptr(),
                );
            } else {
                blake256_final(
                    &mut ctx.s256 as *mut blakestate256 as *mut _,
                    outbuf.as_mut_ptr(),
                );
            }
        }
        out32.copy_from_slice(&outbuf[..32]);
    }
}

/* ------------------------------------------------------------------------- */
/* #elif HARAKA_TR                                                           */
/* ------------------------------------------------------------------------- */

#[cfg(any(
    feature = "haraka",
    not(any(feature = "sha2", feature = "shake", feature = "blake"))
))]
mod kat_tr_haraka {
    use sphincs_plus::context::SpxCtx;
    use sphincs_plus::haraka::{
        SPX_haraka_S_inc_absorb, SPX_haraka_S_inc_finalize, SPX_haraka_S_inc_init,
        SPX_haraka_S_inc_squeeze, SPX_tweak_constants,
    };
    use sphincs_plus::params::SPX_N;

    /* typedef struct { spx_ctx inner; uint8_t s[65]; } kat_tr_ctx; */
    pub struct KatTrCtx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    /* haraka_S_inc_absorb(ctx->s, m, mlen, &ctx->inner) */
    fn haraka_S_inc_absorb(ctx: &mut KatTrCtx, m: *const u8, mlen: usize) {
        unsafe {
            let s = ctx.s.as_mut_ptr();
            SPX_haraka_S_inc_absorb(s, m, mlen, &ctx.inner);
        }
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut ctx = KatTrCtx {
            inner: SpxCtx::new(),
            s: [0u8; 65],
        };

        for i in 0..SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
        }

        unsafe {
            SPX_tweak_constants(&mut ctx.inner);
            SPX_haraka_S_inc_init(ctx.s.as_mut_ptr());
        }

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-HARAKA"; /* sizeof tag - 1 */
        haraka_S_inc_absorb(&mut ctx, tag.as_ptr(), tag.len());

        let sep: u8 = 0x00;
        haraka_S_inc_absorb(&mut ctx, &sep, 1);

        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        haraka_S_inc_absorb(ctx, p.as_ptr(), n);

        let sep: u8 = 0x00;
        haraka_S_inc_absorb(ctx, &sep, 1);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }

        let mut lenle = [0u8; 8];
        let L: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }

        haraka_S_inc_absorb(ctx, lenle.as_ptr(), 8);
        haraka_S_inc_absorb(ctx, le.as_ptr(), 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; 8];
        let L: u64 = len as u64;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }
        haraka_S_inc_absorb(ctx, lenle.as_ptr(), 8);
        if len != 0 {
            haraka_S_inc_absorb(ctx, buf.as_ptr(), len);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        unsafe {
            SPX_haraka_S_inc_finalize(ctx.s.as_mut_ptr());
            let s = ctx.s.as_mut_ptr();
            SPX_haraka_S_inc_squeeze(out32.as_mut_ptr(), 32, s, &ctx.inner);
        }
    }
}

/* ------------------------------------------------------------------------- */
/* #elif SHA2_TR                                                             */
/* ------------------------------------------------------------------------- */

#[cfg(all(feature = "sha2", not(feature = "haraka")))]
mod kat_tr_sha2 {
    use sphincs_plus::sha2::{
        sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512_inc_blocks,
        sha512_inc_finalize, sha512_inc_init,
    };

    /* #if SPX_N >= 24 -> sha512, #else -> sha256 */
    const USE_X_512: bool = sphincs_plus::params::SPX_N >= 24;
    /* shaX_block_bytes */
    const shaX_block_bytes: usize = if USE_X_512 { 128 } else { 64 };
    /* Upper bound for both variants, so that the block scratch buffers are
     * large enough whichever branch is taken. */
    const SHAX_MAX_BLOCK: usize = 128;

    /* typedef struct { uint8_t s[shaX_state_len]; } kat_tr_ctx;
     * (40 for sha256, 72 for sha512 -- the larger one is always allocated) */
    pub struct KatTrCtx {
        s: [u8; 72],
    }

    /* shaX_inc_blocks(ctx->s, block, 1) */
    fn shaX_inc_blocks_1(ctx: &mut KatTrCtx, block: &[u8; SHAX_MAX_BLOCK]) {
        unsafe {
            if USE_X_512 {
                sha512_inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
            } else {
                sha256_inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
            }
        }
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHA2"; /* sizeof tag - 1 */
        let mut block = [0u8; SHAX_MAX_BLOCK];

        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        for i in tag.len()..shaX_block_bytes {
            block[i] = 0;
        }

        let mut ctx = KatTrCtx { s: [0u8; 72] };
        unsafe {
            if USE_X_512 {
                sha512_inc_init(ctx.s.as_mut_ptr());
            } else {
                sha256_inc_init(ctx.s.as_mut_ptr());
            }
        }
        shaX_inc_blocks_1(&mut ctx, &block);

        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + (shaX_block_bytes - 1)) / shaX_block_bytes;

        for i in 0..block_count {
            let mut block = [0u8; SHAX_MAX_BLOCK];
            let mut j: usize = 0;

            while i * shaX_block_bytes + j < n && j < shaX_block_bytes {
                block[j] = p[i * shaX_block_bytes + j];
                j += 1;
            }

            if i * shaX_block_bytes + j == n && j < shaX_block_bytes {
                block[j] = 0x00;
                j += 1;
            }

            while j < shaX_block_bytes {
                block[j] = 0;
                j += 1;
            }

            shaX_inc_blocks_1(ctx, &block);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = [0u8; SHAX_MAX_BLOCK];
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }

        let mut lenle = [0u8; 8];
        let L: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }

        for i in 0..8 {
            block[i] = lenle[i];
        }
        for i in 0..8 {
            block[8 + i] = le[i];
        }
        for i in 16..shaX_block_bytes {
            block[i] = 0;
        }

        shaX_inc_blocks_1(ctx, &block);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        /* uint8_t lenle[shaX_block_bytes] = {0}; */
        let mut lenle = [0u8; SHAX_MAX_BLOCK];
        let L: u64 = len as u64;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }
        let block_count = (len + (shaX_block_bytes - 1)) / shaX_block_bytes;
        shaX_inc_blocks_1(ctx, &lenle);

        if len != 0 {
            for i in 0..block_count {
                let mut block = [0u8; SHAX_MAX_BLOCK];
                let mut j: usize = 0;

                while i * shaX_block_bytes + j < len && j < shaX_block_bytes {
                    block[j] = buf[i * shaX_block_bytes + j];
                    j += 1;
                }
                while j < shaX_block_bytes {
                    block[j] = 0;
                    j += 1;
                }

                shaX_inc_blocks_1(ctx, &block);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        /* unsigned char outbuf[shaX_output_bytes] = {0};
         * uint8_t final_block[shaX_block_bytes] = {0}; */
        let mut outbuf = [0u8; 64];
        let final_block = [0u8; SHAX_MAX_BLOCK];
        unsafe {
            if USE_X_512 {
                sha512_inc_finalize(
                    outbuf.as_mut_ptr(),
                    ctx.s.as_mut_ptr(),
                    final_block.as_ptr(),
                    1,
                );
            } else {
                sha256_inc_finalize(
                    outbuf.as_mut_ptr(),
                    ctx.s.as_mut_ptr(),
                    final_block.as_ptr(),
                    1,
                );
            }
        }
        out32.copy_from_slice(&outbuf[..32]);
    }
}

/* ------------------------------------------------------------------------- */
/* #elif SHAKE_TR                                                            */
/* ------------------------------------------------------------------------- */

#[cfg(all(feature = "shake", not(any(feature = "haraka", feature = "sha2"))))]
mod kat_tr_shake {
    use sphincs_plus::shake::fips202::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    /* typedef struct { uint64_t s[26]; } kat_tr_ctx; */
    pub struct KatTrCtx {
        s: [u64; 26],
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut ctx = KatTrCtx { s: [0u64; 26] };
        unsafe {
            shake256_inc_init(ctx.s.as_mut_ptr());
        }

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHAKE"; /* sizeof tag - 1 */
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), tag.as_ptr(), tag.len());
        }

        let sep: u8 = 0x00;
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), &sep, 1);
        }

        ctx
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), p.as_ptr(), n);
        }

        let sep: u8 = 0x00;
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), &sep, 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }

        let mut lenle = [0u8; 8];
        let L: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }

        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
            shake256_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; 8];
        let L: u64 = len as u64;
        for i in 0..8 {
            lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
        }
        unsafe {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
            if len != 0 {
                shake256_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), len);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        unsafe {
            shake256_inc_finalize(ctx.s.as_mut_ptr());
            shake256_inc_squeeze(out32.as_mut_ptr(), 32, ctx.s.as_mut_ptr());
        }
    }
}

/* ---- the transcript implementation picked by the active backend ---------- */

#[cfg(any(
    feature = "haraka",
    not(any(feature = "sha2", feature = "shake", feature = "blake"))
))]
use kat_tr_haraka::{
    kat_tr_absorb_bytes, kat_tr_absorb_label, kat_tr_absorb_u64, kat_tr_final, kat_tr_init,
};

#[cfg(all(feature = "sha2", not(feature = "haraka")))]
use kat_tr_sha2::{
    kat_tr_absorb_bytes, kat_tr_absorb_label, kat_tr_absorb_u64, kat_tr_final, kat_tr_init,
};

#[cfg(all(feature = "shake", not(any(feature = "haraka", feature = "sha2"))))]
use kat_tr_shake::{
    kat_tr_absorb_bytes, kat_tr_absorb_label, kat_tr_absorb_u64, kat_tr_final, kat_tr_init,
};

#[cfg(all(
    feature = "blake",
    not(any(feature = "haraka", feature = "sha2", feature = "shake"))
))]
use kat_tr_blake::{
    kat_tr_absorb_bytes, kat_tr_absorb_label, kat_tr_absorb_u64, kat_tr_final, kat_tr_init,
};

/* ------------------------------------------------------------------------- */
/* int main(void)                                                            */
/* ------------------------------------------------------------------------- */

fn main() {
    /* The C arrays are `static` because of their size. */
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    let mut mlen: u64;
    let mut smlen: u64 = 0;
    let mut mlen1: u64 = 0;
    let mut ret: i32;

    // Deterministic entropy to seed DRBG to make .req
    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    unsafe {
        randombytes_init(entropy_input.as_mut_ptr(), core::ptr::null_mut());
    }

    // Initialize Transcript
    let mut tctx = kat_tr_init();
    kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes());
    kat_tr_absorb_label(&mut tctx, "SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT as i32 {
        unsafe {
            let _ = randombytes(seed.as_mut_ptr(), seed.len() as u64);
        }

        kat_tr_absorb_label(&mut tctx, "count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed");
        kat_tr_absorb_bytes(&mut tctx, &seed);

        let mlen_i = (BASE_MLEN as i32) * (i + 1);
        mlen = mlen_i as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(KAT_OVERFLOW);
        }
        let mlen_us = mlen as usize;

        kat_tr_absorb_label(&mut tctx, "mlen");
        kat_tr_absorb_u64(&mut tctx, mlen);

        unsafe {
            let _ = randombytes(msg.as_mut_ptr(), mlen);
        }
        kat_tr_absorb_label(&mut tctx, "msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen_us]);

        for b in m[..mlen_us].iter_mut() {
            *b = 0;
        }
        for b in m1[..mlen_us + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        for b in sm[..mlen_us + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        m[..mlen_us].copy_from_slice(&msg[..mlen_us]);

        // Keypair
        ret = unsafe { crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        kat_tr_absorb_label(&mut tctx, "pk");
        kat_tr_absorb_bytes(&mut tctx, &pk[..CRYPTO_PUBLICKEYBYTES]);
        kat_tr_absorb_label(&mut tctx, "sk");
        kat_tr_absorb_bytes(&mut tctx, &sk[..CRYPTO_SECRETKEYBYTES]);

        // Sign
        ret = unsafe { crypto_sign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen, sk.as_ptr()) };
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        kat_tr_absorb_label(&mut tctx, "smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, "sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        // Verify
        ret = unsafe {
            crypto_sign_open(m1.as_mut_ptr(), &mut mlen1, sm.as_ptr(), smlen, pk.as_ptr())
        };
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if m[..mlen_us] != m1[..mlen_us] {
            eprintln!("m mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
    }

    // Finalize transcript digest
    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    print!("\n");

    std::io::stdout().flush().unwrap();
    std::process::exit(KAT_SUCCESS);
}

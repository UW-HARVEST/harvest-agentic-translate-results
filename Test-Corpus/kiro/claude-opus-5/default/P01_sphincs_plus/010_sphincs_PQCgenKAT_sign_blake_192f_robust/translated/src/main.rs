//
//  PQCgenKAT_sign.c -> main.rs
//
//  Translated from the SPHINCS+ reference implementation KAT driver.
//  Behaviour must be byte-identical to the C binary. The C file selects one of
//  four "KAT transcript" implementations via the preprocessor macros
//  BLAKE_TR / HARAKA_TR / SHA2_TR / SHAKE_TR (chosen by CMake from
//  HASH_BACKEND). Here we translate ALL FOUR and select via Cargo feature
//  gates identical to the rest of the crate.
//

use std::io::Write;

use sphincs_plus::params::{
    CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES,
};
use sphincs_plus::rng::{randombytes, randombytes_init};
use sphincs_plus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

// -------------------------------------------------------------------------
// #define MAX_MARKER_LEN       50
// #define BASE_MLEN            33
// #define LOOP_COUNT           7
//
// #define KAT_SUCCESS          0
// #define KAT_OVERFLOW        -1
// #define KAT_CRYPTO_FAILURE  -2
// -------------------------------------------------------------------------
const _MAX_MARKER_LEN: usize = 50;
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const _KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// =========================================================================
// SHA2_TR backend
//   #elif SHA2_TR
// =========================================================================
#[cfg(feature = "sha2")]
mod kat_tr {
    use sphincs_plus::sha2::sha2::{
        sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512_inc_blocks,
        sha512_inc_finalize, sha512_inc_init,
    };

    // #if SPX_N >= 24  -> sha512 : state 72, block 128, output 64
    // #else            -> sha256 : state 40, block 64, output 32
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    const SHAX_STATE_LEN: usize = 72;
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    const SHAX_BLOCK_BYTES: usize = 128;
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    const SHAX_OUTPUT_BYTES: usize = 64;

    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    const SHAX_STATE_LEN: usize = 40;
    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    const SHAX_BLOCK_BYTES: usize = 64;
    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    const SHAX_OUTPUT_BYTES: usize = 32;

    #[inline]
    unsafe fn shax_inc_init(state: *mut u8) {
        #[cfg(any(
            feature = "192s",
            feature = "192f",
            feature = "256s",
            feature = "256f"
        ))]
        sha512_inc_init(state);
        #[cfg(not(any(
            feature = "192s",
            feature = "192f",
            feature = "256s",
            feature = "256f"
        )))]
        sha256_inc_init(state);
    }

    #[inline]
    unsafe fn shax_inc_blocks(state: *mut u8, in_: *const u8, inblocks: usize) {
        #[cfg(any(
            feature = "192s",
            feature = "192f",
            feature = "256s",
            feature = "256f"
        ))]
        sha512_inc_blocks(state, in_, inblocks);
        #[cfg(not(any(
            feature = "192s",
            feature = "192f",
            feature = "256s",
            feature = "256f"
        )))]
        sha256_inc_blocks(state, in_, inblocks);
    }

    #[inline]
    unsafe fn shax_inc_finalize(out: *mut u8, state: *mut u8, in_: *const u8, inlen: usize) {
        #[cfg(any(
            feature = "192s",
            feature = "192f",
            feature = "256s",
            feature = "256f"
        ))]
        sha512_inc_finalize(out, state, in_, inlen);
        #[cfg(not(any(
            feature = "192s",
            feature = "192f",
            feature = "256s",
            feature = "256f"
        )))]
        sha256_inc_finalize(out, state, in_, inlen);
    }

    // typedef struct { uint8_t s[shaX_state_len]; } kat_tr_ctx;
    pub struct KatTrCtx {
        pub s: [u8; SHAX_STATE_LEN],
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx {
                s: [0u8; SHAX_STATE_LEN],
            }
        }
    }

    pub unsafe fn kat_tr_init(ctx: &mut KatTrCtx) {
        // static const uint8_t tag[] = "KAT-TRANSCRIPT-v1-SHA2";
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHA2";
        let tag_len = tag.len(); // == sizeof tag - 1

        let mut block = [0u8; SHAX_BLOCK_BYTES];
        // for (i = 0; i < sizeof tag - 1; ++i) block[i] = tag[i];
        let mut i = 0usize;
        while i < tag_len {
            block[i] = tag[i];
            i += 1;
        }
        // for (i = sizeof tag - 1; i < shaX_block_bytes; ++i) block[i] = 0;
        while i < SHAX_BLOCK_BYTES {
            block[i] = 0;
            i += 1;
        }

        shax_inc_init(ctx.s.as_mut_ptr());
        shax_inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
    }

    pub unsafe fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        let p = label;
        let n = p.len(); // strlen (label passed without NUL)
        let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

        let mut i = 0usize;
        while i < block_count {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            let mut j = 0usize;

            while i * SHAX_BLOCK_BYTES + j < n && j < SHAX_BLOCK_BYTES {
                block[j] = p[i * SHAX_BLOCK_BYTES + j];
                j += 1;
            }

            if i * SHAX_BLOCK_BYTES + j == n && j < SHAX_BLOCK_BYTES {
                block[j] = 0x00;
                j += 1;
            }

            while j < SHAX_BLOCK_BYTES {
                block[j] = 0;
                j += 1;
            }

            shax_inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
            i += 1;
        }
    }

    pub unsafe fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        let mut le = [0u8; 8];
        let mut i = 0usize;
        while i < 8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        i = 0;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        i = 0;
        while i < 8 {
            block[i] = lenle[i];
            i += 1;
        }
        i = 0;
        while i < 8 {
            block[8 + i] = le[i];
            i += 1;
        }
        i = 16;
        while i < SHAX_BLOCK_BYTES {
            block[i] = 0;
            i += 1;
        }

        shax_inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
    }

    pub unsafe fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        // uint8_t lenle[shaX_block_bytes] = {0};
        let mut lenle = [0u8; SHAX_BLOCK_BYTES];
        let l: u64 = len as u64;
        let mut i = 0usize;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }
        let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
        shax_inc_blocks(ctx.s.as_mut_ptr(), lenle.as_ptr(), 1);

        if len != 0 {
            i = 0;
            while i < block_count {
                let mut block = [0u8; SHAX_BLOCK_BYTES];
                let mut j = 0usize;

                while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                    block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                    j += 1;
                }
                while j < SHAX_BLOCK_BYTES {
                    block[j] = 0;
                    j += 1;
                }

                shax_inc_blocks(ctx.s.as_mut_ptr(), block.as_ptr(), 1);
                i += 1;
            }
        }
    }

    pub unsafe fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
        let mut final_block = [0u8; SHAX_BLOCK_BYTES];
        shax_inc_finalize(
            outbuf.as_mut_ptr(),
            ctx.s.as_mut_ptr(),
            final_block.as_mut_ptr(),
            1,
        );
        out32.copy_from_slice(&outbuf[..32]);
        let _ = &mut final_block;
    }
}

// =========================================================================
// SHAKE_TR backend
//   #elif SHAKE_TR
// =========================================================================
#[cfg(all(not(feature = "sha2"), feature = "shake"))]
mod kat_tr {
    use sphincs_plus::shake::fips202::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    // typedef struct { uint64_t s[26]; } kat_tr_ctx;
    pub struct KatTrCtx {
        pub s: [u64; 26],
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx { s: [0u64; 26] }
        }
    }

    pub unsafe fn kat_tr_init(ctx: &mut KatTrCtx) {
        shake256_inc_init(ctx.s.as_mut_ptr());

        // static const uint8_t tag[] = "KAT-TRANSCRIPT-v1-SHAKE";
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHAKE";
        shake256_inc_absorb(ctx.s.as_mut_ptr(), tag.as_ptr(), tag.len());

        let sep: [u8; 1] = [0x00];
        shake256_inc_absorb(ctx.s.as_mut_ptr(), sep.as_ptr(), 1);
    }

    pub unsafe fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        let p = label;
        let n = p.len();
        shake256_inc_absorb(ctx.s.as_mut_ptr(), p.as_ptr(), n);

        let sep: [u8; 1] = [0x00];
        shake256_inc_absorb(ctx.s.as_mut_ptr(), sep.as_ptr(), 1);
    }

    pub unsafe fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        let mut i = 0usize;
        while i < 8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        i = 0;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
        shake256_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8);
    }

    pub unsafe fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l: u64 = len as u64;
        let mut i = 0usize;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }
        shake256_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8);
        if len != 0 {
            shake256_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), len);
        }
    }

    pub unsafe fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        shake256_inc_finalize(ctx.s.as_mut_ptr());
        shake256_inc_squeeze(out32.as_mut_ptr(), 32, ctx.s.as_mut_ptr());
    }
}

// =========================================================================
// BLAKE_TR backend
//   #ifdef BLAKE_TR
// =========================================================================
#[cfg(all(not(feature = "sha2"), not(feature = "shake"), feature = "blake"))]
mod kat_tr {
    // #if SPX_N >= 24 -> blake512 / output 64 ; else blake256 / output 32
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    use sphincs_plus::blake::blake512::{
        blake512_final as blakex_final, blake512_init as blakex_init,
        blake512_update as blakex_update, blakestate512 as BlakeStateX,
    };
    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    use sphincs_plus::blake::blake256::{
        blake256_final as blakex_final, blake256_init as blakex_init,
        blake256_update as blakex_update, blakestate256 as BlakeStateX,
    };

    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    const BLAKEX_OUTPUT_BYTES: usize = 64;
    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    const BLAKEX_OUTPUT_BYTES: usize = 32;

    // typedef blakestateX kat_tr_ctx;
    pub struct KatTrCtx {
        pub inner: BlakeStateX,
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx {
                inner: BlakeStateX::new(),
            }
        }
    }

    pub unsafe fn kat_tr_init(ctx: &mut KatTrCtx) {
        blakex_init(&mut ctx.inner);

        // static const uint8_t tag[] = "KAT-TRANSCRIPT-v1-BLAKE";
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-BLAKE";
        // NOTE: byte count, NOT bits.
        blakex_update(&mut ctx.inner, tag.as_ptr(), (tag.len()) as core::ffi::c_ulonglong);

        let sep: [u8; 1] = [0x00];
        blakex_update(&mut ctx.inner, sep.as_ptr(), 1);
    }

    pub unsafe fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        let p = label;
        let n = p.len();
        blakex_update(&mut ctx.inner, p.as_ptr(), n as core::ffi::c_ulonglong);

        let sep: [u8; 1] = [0x00];
        blakex_update(&mut ctx.inner, sep.as_ptr(), 1);
    }

    pub unsafe fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        let mut i = 0usize;
        while i < 8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        i = 0;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        blakex_update(&mut ctx.inner, lenle.as_ptr(), 8);
        blakex_update(&mut ctx.inner, le.as_ptr(), 8);
    }

    pub unsafe fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l: u64 = len as u64;
        let mut i = 0usize;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }
        blakex_update(&mut ctx.inner, lenle.as_ptr(), 8);
        if len != 0 {
            blakex_update(&mut ctx.inner, buf.as_ptr(), len as core::ffi::c_ulonglong);
        }
    }

    pub unsafe fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; BLAKEX_OUTPUT_BYTES];
        blakex_final(&mut ctx.inner, outbuf.as_mut_ptr());
        out32.copy_from_slice(&outbuf[..32]);
    }
}

// =========================================================================
// HARAKA_TR backend
//   #elif HARAKA_TR (the default when no other backend feature is set)
// =========================================================================
#[cfg(all(not(feature = "sha2"), not(feature = "shake"), not(feature = "blake")))]
mod kat_tr {
    use sphincs_plus::context::SpxCtx;
    use sphincs_plus::haraka::haraka::{
        SPX_haraka_S_inc_absorb, SPX_haraka_S_inc_finalize, SPX_haraka_S_inc_init,
        SPX_haraka_S_inc_squeeze, SPX_tweak_constants,
    };
    use sphincs_plus::params::SPX_N;

    // typedef struct { spx_ctx inner; uint8_t s[65]; } kat_tr_ctx;
    pub struct KatTrCtx {
        pub inner: SpxCtx,
        pub s: [u8; 65],
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx {
                inner: SpxCtx::new(),
                s: [0u8; 65],
            }
        }
    }

    pub unsafe fn kat_tr_init(ctx: &mut KatTrCtx) {
        // for (i = 0; i < SPX_N; ++i) { inner.pub_seed[i]=0; inner.sk_seed[i]=0; }
        let mut i = 0usize;
        while i < SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
            i += 1;
        }

        // tweak_constants(&ctx->inner);  -> SPX_tweak_constants
        SPX_tweak_constants(&mut ctx.inner);
        SPX_haraka_S_inc_init(ctx.s.as_mut_ptr());

        // static const uint8_t tag[] = "KAT-TRANSCRIPT-v1-HARAKA";
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-HARAKA";
        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), tag.as_ptr(), tag.len(), &ctx.inner);

        let sep: [u8; 1] = [0x00];
        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), sep.as_ptr(), 1, &ctx.inner);
    }

    pub unsafe fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
        let p = label;
        let n = p.len();
        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), p.as_ptr(), n, &ctx.inner);

        let sep: [u8; 1] = [0x00];
        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), sep.as_ptr(), 1, &ctx.inner);
    }

    pub unsafe fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        let mut i = 0usize;
        while i < 8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        i = 0;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }

        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), le.as_ptr(), 8, &ctx.inner);
    }

    pub unsafe fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l: u64 = len as u64;
        let mut i = 0usize;
        while i < 8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            i += 1;
        }
        SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), lenle.as_ptr(), 8, &ctx.inner);
        if len != 0 {
            SPX_haraka_S_inc_absorb(ctx.s.as_mut_ptr(), buf.as_ptr(), len, &ctx.inner);
        }
    }

    pub unsafe fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        SPX_haraka_S_inc_finalize(ctx.s.as_mut_ptr());
        SPX_haraka_S_inc_squeeze(out32.as_mut_ptr(), 32, ctx.s.as_mut_ptr(), &ctx.inner);
    }
}

// =========================================================================
// Shared main() translated from the C main().
// =========================================================================
fn main() {
    unsafe {
        // static arrays, zero-initialised (kept off the stack via vec!).
        let mut m: Vec<u8> = vec![0u8; BASE_MLEN * LOOP_COUNT];
        let mut sm: Vec<u8> = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
        let mut m1: Vec<u8> = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
        let mut pk: Vec<u8> = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk: Vec<u8> = vec![0u8; CRYPTO_SECRETKEYBYTES];
        let mut seed: Vec<u8> = vec![0u8; 48];
        let mut entropy_input: Vec<u8> = vec![0u8; 48];
        let mut msg: Vec<u8> = vec![0u8; BASE_MLEN * LOOP_COUNT];

        let mut mlen: u64;
        let mut smlen: u64 = 0;
        let mut mlen1: u64 = 0;
        let mut ret: core::ffi::c_int;

        // for (int i = 0; i < 48; i++) entropy_input[i] = (unsigned char)i;
        for i in 0..48usize {
            entropy_input[i] = i as u8;
        }
        randombytes_init(entropy_input.as_mut_ptr(), core::ptr::null_mut());

        // Initialize Transcript
        let mut tctx = kat_tr::KatTrCtx::new();
        kat_tr::kat_tr_init(&mut tctx);
        kat_tr::kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
        // strlen(CRYPTO_ALGNAME) — CRYPTO_ALGNAME is &str (no NUL).
        let algname = CRYPTO_ALGNAME.as_bytes();
        kat_tr::kat_tr_absorb_bytes(&mut tctx, algname, algname.len());
        kat_tr::kat_tr_absorb_label(&mut tctx, b"SKBYTES");
        kat_tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
        kat_tr::kat_tr_absorb_label(&mut tctx, b"PKBYTES");
        kat_tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
        kat_tr::kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
        kat_tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

        for i in 0..LOOP_COUNT {
            randombytes(seed.as_mut_ptr(), seed.len() as core::ffi::c_ulonglong);

            kat_tr::kat_tr_absorb_label(&mut tctx, b"count");
            kat_tr::kat_tr_absorb_u64(&mut tctx, i as u64);
            kat_tr::kat_tr_absorb_label(&mut tctx, b"seed");
            kat_tr::kat_tr_absorb_bytes(&mut tctx, &seed, seed.len());

            // mlen = (unsigned long long)(BASE_MLEN * (i + 1));
            mlen = (BASE_MLEN * (i + 1)) as u64;
            if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
                eprintln!("mlen overflow");
                let _ = std::io::stdout().flush();
                std::process::exit(KAT_OVERFLOW);
            }

            kat_tr::kat_tr_absorb_label(&mut tctx, b"mlen");
            kat_tr::kat_tr_absorb_u64(&mut tctx, mlen);

            randombytes(msg.as_mut_ptr(), mlen);
            kat_tr::kat_tr_absorb_label(&mut tctx, b"msg");
            kat_tr::kat_tr_absorb_bytes(&mut tctx, &msg, mlen as usize);

            // memset(m, 0, mlen);
            core::ptr::write_bytes(m.as_mut_ptr(), 0, mlen as usize);
            // memset(m1, 0, mlen + CRYPTO_BYTES);
            core::ptr::write_bytes(m1.as_mut_ptr(), 0, mlen as usize + CRYPTO_BYTES);
            // memset(sm, 0, mlen + CRYPTO_BYTES);
            core::ptr::write_bytes(sm.as_mut_ptr(), 0, mlen as usize + CRYPTO_BYTES);
            // memcpy(m, msg, mlen);
            core::ptr::copy_nonoverlapping(msg.as_ptr(), m.as_mut_ptr(), mlen as usize);

            // Keypair
            ret = crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr());
            if ret != 0 {
                eprintln!("crypto_sign_keypair={}", ret);
                let _ = std::io::stdout().flush();
                std::process::exit(KAT_CRYPTO_FAILURE);
            }
            kat_tr::kat_tr_absorb_label(&mut tctx, b"pk");
            kat_tr::kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
            kat_tr::kat_tr_absorb_label(&mut tctx, b"sk");
            kat_tr::kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

            // Sign
            ret = crypto_sign(
                sm.as_mut_ptr(),
                &mut smlen as *mut u64,
                m.as_ptr(),
                mlen,
                sk.as_ptr(),
            );
            if ret != 0 {
                eprintln!("crypto_sign={}", ret);
                let _ = std::io::stdout().flush();
                std::process::exit(KAT_CRYPTO_FAILURE);
            }
            kat_tr::kat_tr_absorb_label(&mut tctx, b"smlen");
            kat_tr::kat_tr_absorb_u64(&mut tctx, smlen);
            kat_tr::kat_tr_absorb_label(&mut tctx, b"sm");
            kat_tr::kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

            // Verify
            ret = crypto_sign_open(
                m1.as_mut_ptr(),
                &mut mlen1 as *mut u64,
                sm.as_ptr(),
                smlen,
                pk.as_ptr(),
            );
            if ret != 0 {
                eprintln!("crypto_sign_open={}", ret);
                let _ = std::io::stdout().flush();
                std::process::exit(KAT_CRYPTO_FAILURE);
            }
            if mlen1 != mlen {
                eprintln!("mlen mismatch");
                let _ = std::io::stdout().flush();
                std::process::exit(KAT_CRYPTO_FAILURE);
            }
            // if (memcmp(m, m1, mlen) != 0)
            if m[..mlen as usize] != m1[..mlen as usize] {
                eprintln!("m mismatch");
                let _ = std::io::stdout().flush();
                std::process::exit(KAT_CRYPTO_FAILURE);
            }
        }

        // Finalize transcript digest
        let mut digest = [0u8; 32];
        kat_tr::kat_tr_final(&mut tctx, &mut digest);

        print!("KAT transcript digest = ");
        for i in 0..32usize {
            print!("{:02X}", digest[i]);
        }
        println!();

        // Flush stdout before exiting.
        let _ = std::io::stdout().flush();
        // return KAT_SUCCESS; (0)
    }
}

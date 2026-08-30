//! Translation of `app/src/PQCgenKAT_sign.c`.
//!
//! Originally provided by NIST (Bassham, Lawrence E (Fed), 8/29/17) and altered
//! by the SPHINCS+ authors to no longer perform file IO: it runs an in-memory
//! sign/verify test and prints a digest over the whole transcript.
//!
//! Just like the CMake `driver` target (which links `sphincs_core_det`), this
//! binary always uses the deterministic `rng.c` DRBG.

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]

include!("tree.rs");

use crate::params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// ===========================================================================
// #ifdef BLAKE_TR
// ===========================================================================
#[cfg(feature = "blake")]
mod kat {
    use super::*;
    use crate::blake::blake256::{blake256_final, blake256_init, blake256_update};
    use crate::blake::blake512::{blake512_final, blake512_init, blake512_update};
    use crate::blake::{BlakeState256, BlakeState512};

    /// `blakeX_output_bytes`
    const BLAKE_X_OUTPUT_BYTES: usize = if SPX_N >= 24 { 64 } else { 32 };
    /// Selects `blakestate512`/`blake512_*` over `blakestate256`/`blake256_*`.
    const USE_512: bool = SPX_N >= 24;

    /// `kat_tr_ctx` (either a `blakestate256` or a `blakestate512`).
    pub struct KatTrCtx {
        s256: BlakeState256,
        s512: BlakeState512,
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx {
                s256: BlakeState256::new(),
                s512: BlakeState512::new(),
            }
        }
    }

    /// `blakeX_update`.  Note that the C driver passes byte counts where the
    /// BLAKE reference implementation expects a bit count; that behaviour is
    /// reproduced verbatim.
    fn update(ctx: &mut KatTrCtx, data: &[u8], datalen: u64) {
        if USE_512 {
            blake512_update(&mut ctx.s512, data, datalen);
        } else {
            blake256_update(&mut ctx.s256, data, datalen);
        }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        if USE_512 {
            blake512_init(&mut ctx.s512);
        } else {
            blake256_init(&mut ctx.s256);
        }

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-BLAKE";
        update(ctx, tag, tag.len() as u64);

        let sep = [0x00u8];
        update(ctx, &sep, 1);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        update(ctx, p, n as u64);

        let sep = [0x00u8];
        update(ctx, &sep, 1);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();

        update(ctx, &lenle, 8);
        update(ctx, &le, 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let lenle = (len as u64).to_le_bytes();
        update(ctx, &lenle, 8);
        if len != 0 {
            update(ctx, buf, len as u64);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; BLAKE_X_OUTPUT_BYTES];
        if USE_512 {
            blake512_final(&mut ctx.s512, &mut outbuf);
        } else {
            blake256_final(&mut ctx.s256, &mut outbuf);
        }
        out32.copy_from_slice(&outbuf[..32]);
    }
}

// ===========================================================================
// #elif HARAKA_TR
// ===========================================================================
#[cfg(not(any(feature = "blake", feature = "shake", feature = "sha2")))]
mod kat {
    use super::*;
    use crate::context::SpxCtx;
    use crate::haraka::haraka::{
        haraka_s_inc_absorb, haraka_s_inc_finalize, haraka_s_inc_init, haraka_s_inc_squeeze,
        tweak_constants,
    };

    /// `kat_tr_ctx`
    pub struct KatTrCtx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx {
                inner: SpxCtx::new(),
                s: [0u8; 65],
            }
        }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        for i in 0..SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
        }

        tweak_constants(&mut ctx.inner);
        haraka_s_inc_init(&mut ctx.s);

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-HARAKA";
        haraka_s_inc_absorb(&mut ctx.s, tag, tag.len(), &ctx.inner);

        let sep = [0x00u8];
        haraka_s_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        haraka_s_inc_absorb(&mut ctx.s, p, p.len(), &ctx.inner);

        let sep = [0x00u8];
        haraka_s_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();

        haraka_s_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &le, 8, &ctx.inner);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let lenle = (len as u64).to_le_bytes();
        haraka_s_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
        if len != 0 {
            haraka_s_inc_absorb(&mut ctx.s, buf, len, &ctx.inner);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        haraka_s_inc_finalize(&mut ctx.s);
        let inner = ctx.inner;
        haraka_s_inc_squeeze(out32, 32, &mut ctx.s, &inner);
    }
}

// ===========================================================================
// #elif SHA2_TR
// ===========================================================================
#[cfg(all(feature = "sha2", not(any(feature = "blake", feature = "shake"))))]
mod kat {
    use super::*;
    use crate::sha2::sha2::{
        sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512_inc_blocks,
        sha512_inc_finalize, sha512_inc_init,
    };

    const USE_512: bool = SPX_N >= 24;
    const SHAX_STATE_LEN: usize = if SPX_N >= 24 { 72 } else { 40 };
    const SHAX_BLOCK_BYTES: usize = if SPX_N >= 24 { 128 } else { 64 };
    const SHAX_OUTPUT_BYTES: usize = if SPX_N >= 24 { 64 } else { 32 };

    /// `kat_tr_ctx`
    pub struct KatTrCtx {
        s: [u8; SHAX_STATE_LEN],
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx {
                s: [0u8; SHAX_STATE_LEN],
            }
        }
    }

    fn inc_init(ctx: &mut KatTrCtx) {
        if USE_512 {
            sha512_inc_init(&mut ctx.s);
        } else {
            sha256_inc_init(&mut ctx.s);
        }
    }

    fn inc_blocks(ctx: &mut KatTrCtx, inp: &[u8], inblocks: usize) {
        if USE_512 {
            sha512_inc_blocks(&mut ctx.s, inp, inblocks);
        } else {
            sha256_inc_blocks(&mut ctx.s, inp, inblocks);
        }
    }

    fn inc_finalize(out: &mut [u8], ctx: &mut KatTrCtx, inp: &[u8], inlen: usize) {
        if USE_512 {
            sha512_inc_finalize(out, &mut ctx.s, inp, inlen);
        } else {
            sha256_inc_finalize(out, &mut ctx.s, inp, inlen);
        }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; SHAX_BLOCK_BYTES];

        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        for i in tag.len()..SHAX_BLOCK_BYTES {
            block[i] = 0;
        }

        inc_init(ctx);
        inc_blocks(ctx, &block, 1);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

        for i in 0..block_count {
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

            inc_blocks(ctx, &block, 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();

        block[..8].copy_from_slice(&lenle);
        block[8..16].copy_from_slice(&le);
        for i in 16..SHAX_BLOCK_BYTES {
            block[i] = 0;
        }

        inc_blocks(ctx, &block, 1);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; SHAX_BLOCK_BYTES];
        lenle[..8].copy_from_slice(&(len as u64).to_le_bytes());

        let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
        inc_blocks(ctx, &lenle, 1);

        if len != 0 {
            for i in 0..block_count {
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

                inc_blocks(ctx, &block, 1);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
        let final_block = [0u8; SHAX_BLOCK_BYTES];
        inc_finalize(&mut outbuf, ctx, &final_block, 1);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

// ===========================================================================
// #elif SHAKE_TR
// ===========================================================================
#[cfg(all(feature = "shake", not(feature = "blake")))]
mod kat {
    use crate::shake::fips202::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    /// `kat_tr_ctx`
    pub struct KatTrCtx {
        s: [u64; 26],
    }

    impl KatTrCtx {
        pub fn new() -> Self {
            KatTrCtx { s: [0u64; 26] }
        }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        shake256_inc_init(&mut ctx.s);

        let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHAKE";
        shake256_inc_absorb(&mut ctx.s, tag, tag.len());

        let sep = [0x00u8];
        shake256_inc_absorb(&mut ctx.s, &sep, 1);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        shake256_inc_absorb(&mut ctx.s, p, p.len());

        let sep = [0x00u8];
        shake256_inc_absorb(&mut ctx.s, &sep, 1);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let le = x.to_le_bytes();
        let lenle = 8u64.to_le_bytes();

        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        shake256_inc_absorb(&mut ctx.s, &le, 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let len = buf.len();
        let lenle = (len as u64).to_le_bytes();
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        if len != 0 {
            shake256_inc_absorb(&mut ctx.s, buf, len);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(out32, 32, &mut ctx.s);
    }
}

use kat::*;

fn run() -> i32 {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    // Deterministic entropy to seed DRBG to make .req
    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    rng::randombytes_init_impl(&entropy_input, None);

    // Initialize Transcript
    let mut tctx = KatTrCtx::new();
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes());
    kat_tr_absorb_label(&mut tctx, "SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes_drbg(&mut seed);

        kat_tr_absorb_label(&mut tctx, "count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed");
        kat_tr_absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            return KAT_OVERFLOW;
        }

        kat_tr_absorb_label(&mut tctx, "mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        rng::randombytes_drbg(&mut msg[..mlen]);
        kat_tr_absorb_label(&mut tctx, "msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen]);

        m[..mlen].fill(0);
        m1[..mlen + CRYPTO_BYTES].fill(0);
        sm[..mlen + CRYPTO_BYTES].fill(0);
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = sign::crypto_sign_keypair_impl(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={ret}");
            return KAT_CRYPTO_FAILURE;
        }
        kat_tr_absorb_label(&mut tctx, "pk");
        kat_tr_absorb_bytes(&mut tctx, &pk);
        kat_tr_absorb_label(&mut tctx, "sk");
        kat_tr_absorb_bytes(&mut tctx, &sk);

        // Sign
        let (ret, smlen) = sign::crypto_sign_impl(
            &mut sm[..CRYPTO_BYTES + mlen],
            &m[..mlen],
            &sk,
        );
        if ret != 0 {
            eprintln!("crypto_sign={ret}");
            return KAT_CRYPTO_FAILURE;
        }
        let smlen = smlen as usize;
        kat_tr_absorb_label(&mut tctx, "smlen");
        kat_tr_absorb_u64(&mut tctx, smlen as u64);
        kat_tr_absorb_label(&mut tctx, "sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen]);

        // Verify
        let (ret, mlen1) =
            sign::crypto_sign_open_impl(&mut m1[..smlen], &sm[..smlen], &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={ret}");
            return KAT_CRYPTO_FAILURE;
        }
        if mlen1 as usize != mlen {
            eprintln!("mlen mismatch");
            return KAT_CRYPTO_FAILURE;
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            return KAT_CRYPTO_FAILURE;
        }
    }

    // Finalize transcript digest
    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();

    KAT_SUCCESS
}

fn main() {
    let code = run();
    // The C `main` returns 0/-1/-2; the shell observes them as 0/255/254.
    std::process::exit(code & 0xFF);
}

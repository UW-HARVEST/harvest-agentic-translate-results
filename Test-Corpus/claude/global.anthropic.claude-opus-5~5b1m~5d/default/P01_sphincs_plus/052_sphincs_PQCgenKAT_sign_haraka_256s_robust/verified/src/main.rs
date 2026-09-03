//
//  PQCgenKAT_sign.c
//
//  Created by Bassham, Lawrence E (Fed) on 8/29/17.
//  Copyright (c) 2017 Bassham, Lawrence E (Fed). All rights reserved.
//
// Translation of c_src/app/src/PQCgenKAT_sign.c

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use sphincs_core_det as spx;

use std::io::Write;

const MAX_MARKER_LEN: usize = 50;
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// ===========================================================================
// BLAKE_TR
// ===========================================================================
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
mod tr {
    // #if SPX_N >= 24 -> blake512, else blake256
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    mod sel {
        use sphincs_core_det as spx;
        pub type BlakeStateX = spx::backend::blake512::BlakeState512;
        pub const BLAKEX_OUTPUT_BYTES: usize = 64;
        #[inline]
        pub fn blakeX_init(s: &mut BlakeStateX) {
            spx::backend::blake512::blake512_init(s);
        }
        #[inline]
        pub fn blakeX_update(s: &mut BlakeStateX, input: &[u8]) {
            // NOTE: `blake512_update` takes a *bit* count in the C reference, but
            // PQCgenKAT_sign.c passes BYTE counts (`sizeof tag - 1`, `n`, `len`, ...).
            // Reproduce that original behaviour exactly.
            spx::backend::blake512::blake512_update_bits(s, input, input.len() as u64);
        }
        #[inline]
        pub fn blakeX_final(s: &mut BlakeStateX, out: &mut [u8]) {
            spx::backend::blake512::blake512_final(s, out);
        }
        #[inline]
        pub fn new_state() -> BlakeStateX {
            spx::backend::blake512::BlakeState512::new()
        }
    }

    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    mod sel {
        use sphincs_core_det as spx;
        pub type BlakeStateX = spx::backend::blake256::BlakeState256;
        pub const BLAKEX_OUTPUT_BYTES: usize = 32;
        #[inline]
        pub fn blakeX_init(s: &mut BlakeStateX) {
            spx::backend::blake256::blake256_init(s);
        }
        #[inline]
        pub fn blakeX_update(s: &mut BlakeStateX, input: &[u8]) {
            // NOTE: `blake256_update` takes a *bit* count in the C reference, but
            // PQCgenKAT_sign.c passes BYTE counts (`sizeof tag - 1`, `n`, `len`, ...).
            // Reproduce that original behaviour exactly.
            spx::backend::blake256::blake256_update_bits(s, input, input.len() as u64);
        }
        #[inline]
        pub fn blakeX_final(s: &mut BlakeStateX, out: &mut [u8]) {
            spx::backend::blake256::blake256_final(s, out);
        }
        #[inline]
        pub fn new_state() -> BlakeStateX {
            spx::backend::blake256::BlakeState256::new()
        }
    }

    use sel::*;

    pub struct KatTrCtx {
        st: BlakeStateX,
    }

    impl KatTrCtx {
        pub fn init() -> KatTrCtx {
            let mut ctx = KatTrCtx { st: new_state() };
            blakeX_init(&mut ctx.st);

            const TAG: &[u8] = b"KAT-TRANSCRIPT-v1-BLAKE";
            blakeX_update(&mut ctx.st, TAG);

            let sep: [u8; 1] = [0x00];
            blakeX_update(&mut ctx.st, &sep);

            ctx
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            blakeX_update(&mut self.st, &p[..n]);

            let sep: [u8; 1] = [0x00];
            blakeX_update(&mut self.st, &sep);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }

            let mut lenle = [0u8; 8];
            let L: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }

            blakeX_update(&mut self.st, &lenle);
            blakeX_update(&mut self.st, &le);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; 8];
            let L: u64 = len as u64;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }
            blakeX_update(&mut self.st, &lenle);
            if len != 0 {
                blakeX_update(&mut self.st, &buf[..len]);
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            let mut outbuf = [0u8; BLAKEX_OUTPUT_BYTES];
            blakeX_final(&mut self.st, &mut outbuf);
            out32.copy_from_slice(&outbuf[..32]);
        }
    }
}

// ===========================================================================
// SHA2_TR
// ===========================================================================
#[cfg(feature = "sha2")]
mod tr {
    // #if SPX_N >= 24 -> sha512, else sha256
    #[cfg(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))]
    mod sel {
        use sphincs_core_det as spx;
        pub const SHAX_STATE_LEN: usize = 72;
        pub const SHAX_BLOCK_BYTES: usize = 128;
        pub const SHAX_OUTPUT_BYTES: usize = 64;
        #[inline]
        pub fn shaX_inc_init(state: &mut [u8]) {
            spx::backend::sha2::sha512_inc_init(state);
        }
        #[inline]
        pub fn shaX_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
            spx::backend::sha2::sha512_inc_blocks(state, input, inblocks);
        }
        #[inline]
        pub fn shaX_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8]) {
            spx::backend::sha2::sha512_inc_finalize(out, state, input);
        }
    }

    #[cfg(not(any(
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    )))]
    mod sel {
        use sphincs_core_det as spx;
        pub const SHAX_STATE_LEN: usize = 40;
        pub const SHAX_BLOCK_BYTES: usize = 64;
        pub const SHAX_OUTPUT_BYTES: usize = 32;
        #[inline]
        pub fn shaX_inc_init(state: &mut [u8]) {
            spx::backend::sha2::sha256_inc_init(state);
        }
        #[inline]
        pub fn shaX_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
            spx::backend::sha2::sha256_inc_blocks(state, input, inblocks);
        }
        #[inline]
        pub fn shaX_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8]) {
            spx::backend::sha2::sha256_inc_finalize(out, state, input);
        }
    }

    use sel::*;

    pub struct KatTrCtx {
        s: [u8; SHAX_STATE_LEN],
    }

    impl KatTrCtx {
        pub fn init() -> KatTrCtx {
            const TAG: &[u8] = b"KAT-TRANSCRIPT-v1-SHA2";
            let mut ctx = KatTrCtx {
                s: [0u8; SHAX_STATE_LEN],
            };
            let mut block = [0u8; SHAX_BLOCK_BYTES];

            for i in 0..TAG.len() {
                block[i] = TAG[i];
            }
            for i in TAG.len()..SHAX_BLOCK_BYTES {
                block[i] = 0;
            }

            shaX_inc_init(&mut ctx.s);
            shaX_inc_blocks(&mut ctx.s, &block, 1);

            ctx
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

            for i in 0..block_count {
                let mut block = [0u8; SHAX_BLOCK_BYTES];
                let mut j: usize = 0;

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

                shaX_inc_blocks(&mut self.s, &block, 1);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
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
            for i in 16..SHAX_BLOCK_BYTES {
                block[i] = 0;
            }

            shaX_inc_blocks(&mut self.s, &block, 1);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; SHAX_BLOCK_BYTES];
            let L: u64 = len as u64;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }
            let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
            shaX_inc_blocks(&mut self.s, &lenle, 1);

            if len != 0 {
                for i in 0..block_count {
                    let mut block = [0u8; SHAX_BLOCK_BYTES];
                    let mut j: usize = 0;

                    while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                        block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                        j += 1;
                    }
                    while j < SHAX_BLOCK_BYTES {
                        block[j] = 0;
                        j += 1;
                    }

                    shaX_inc_blocks(&mut self.s, &block, 1);
                }
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
            let final_block = [0u8; SHAX_BLOCK_BYTES];
            shaX_inc_finalize(&mut outbuf, &mut self.s, &final_block[..1]);
            out32.copy_from_slice(&outbuf[..32]);
        }
    }
}

// ===========================================================================
// SHAKE_TR
// ===========================================================================
#[cfg(all(feature = "shake", not(feature = "sha2")))]
mod tr {
    use sphincs_core_det as spx;

    pub struct KatTrCtx {
        s: [u64; 26],
    }

    impl KatTrCtx {
        pub fn init() -> KatTrCtx {
            let mut ctx = KatTrCtx { s: [0u64; 26] };
            spx::backend::fips202::shake256_inc_init(&mut ctx.s);

            const TAG: &[u8] = b"KAT-TRANSCRIPT-v1-SHAKE";
            spx::backend::fips202::shake256_inc_absorb(&mut ctx.s, TAG);

            let sep: [u8; 1] = [0x00];
            spx::backend::fips202::shake256_inc_absorb(&mut ctx.s, &sep);

            ctx
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            spx::backend::fips202::shake256_inc_absorb(&mut self.s, &p[..n]);

            let sep: [u8; 1] = [0x00];
            spx::backend::fips202::shake256_inc_absorb(&mut self.s, &sep);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }

            let mut lenle = [0u8; 8];
            let L: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }

            spx::backend::fips202::shake256_inc_absorb(&mut self.s, &lenle);
            spx::backend::fips202::shake256_inc_absorb(&mut self.s, &le);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; 8];
            let L: u64 = len as u64;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }
            spx::backend::fips202::shake256_inc_absorb(&mut self.s, &lenle);
            if len != 0 {
                spx::backend::fips202::shake256_inc_absorb(&mut self.s, &buf[..len]);
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            spx::backend::fips202::shake256_inc_finalize(&mut self.s);
            spx::backend::fips202::shake256_inc_squeeze(&mut out32[..], &mut self.s);
        }
    }
}

// ===========================================================================
// HARAKA_TR
// ===========================================================================
#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
mod tr {
    use sphincs_core_det as spx;
    use spx::params::SPX_N;

    pub struct KatTrCtx {
        inner: spx::context::SpxCtx,
        s: [u8; 65],
    }

    impl KatTrCtx {
        pub fn init() -> KatTrCtx {
            let mut ctx = KatTrCtx {
                inner: spx::context::SpxCtx::new(),
                s: [0u8; 65],
            };

            for i in 0..SPX_N {
                ctx.inner.pub_seed[i] = 0;
                ctx.inner.sk_seed[i] = 0;
            }

            spx::backend::haraka::tweak_constants(&mut ctx.inner);
            spx::backend::haraka::haraka_S_inc_init(&mut ctx.s);

            const TAG: &[u8] = b"KAT-TRANSCRIPT-v1-HARAKA";
            spx::backend::haraka::haraka_S_inc_absorb(&mut ctx.s, TAG, &ctx.inner);

            let sep: [u8; 1] = [0x00];
            spx::backend::haraka::haraka_S_inc_absorb(&mut ctx.s, &sep, &ctx.inner);

            ctx
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            spx::backend::haraka::haraka_S_inc_absorb(&mut self.s, &p[..n], &self.inner);

            let sep: [u8; 1] = [0x00];
            spx::backend::haraka::haraka_S_inc_absorb(&mut self.s, &sep, &self.inner);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }

            let mut lenle = [0u8; 8];
            let L: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }

            spx::backend::haraka::haraka_S_inc_absorb(&mut self.s, &lenle, &self.inner);
            spx::backend::haraka::haraka_S_inc_absorb(&mut self.s, &le, &self.inner);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; 8];
            let L: u64 = len as u64;
            for i in 0..8 {
                lenle[i] = ((L >> (8 * i)) & 0xFF) as u8;
            }
            spx::backend::haraka::haraka_S_inc_absorb(&mut self.s, &lenle, &self.inner);
            if len != 0 {
                spx::backend::haraka::haraka_S_inc_absorb(&mut self.s, &buf[..len], &self.inner);
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            spx::backend::haraka::haraka_S_inc_finalize(&mut self.s);
            spx::backend::haraka::haraka_S_inc_squeeze(&mut out32[..], &mut self.s, &self.inner);
        }
    }
}

// ===========================================================================
// main
// ===========================================================================

fn finish(code: i32) -> ! {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}

fn main() {
    use spx::params::{CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES};

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
    spx::rng::randombytes_init(&entropy_input, None);

    // Initialize Transcript
    let mut tctx = tr::KatTrCtx::init();
    tctx.absorb_label("CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME.as_bytes());
    tctx.absorb_label("SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label("PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label("SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        spx::rng::randombytes(&mut seed);

        tctx.absorb_label("count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label("seed");
        tctx.absorb_bytes(&seed);

        mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprint!("mlen overflow\n");
            finish(KAT_OVERFLOW);
        }
        let mlen_us = mlen as usize;

        tctx.absorb_label("mlen");
        tctx.absorb_u64(mlen);

        spx::rng::randombytes(&mut msg[..mlen_us]);
        tctx.absorb_label("msg");
        tctx.absorb_bytes(&msg[..mlen_us]);

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
        ret = spx::sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprint!("crypto_sign_keypair={}\n", ret);
            finish(KAT_CRYPTO_FAILURE);
        }
        tctx.absorb_label("pk");
        tctx.absorb_bytes(&pk[..CRYPTO_PUBLICKEYBYTES]);
        tctx.absorb_label("sk");
        tctx.absorb_bytes(&sk[..CRYPTO_SECRETKEYBYTES]);

        // Sign
        ret = spx::sign::crypto_sign(&mut sm, &mut smlen, &m[..mlen_us], &sk);
        if ret != 0 {
            eprint!("crypto_sign={}\n", ret);
            finish(KAT_CRYPTO_FAILURE);
        }
        tctx.absorb_label("smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label("sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);

        // Verify
        ret = spx::sign::crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], &pk);
        if ret != 0 {
            eprint!("crypto_sign_open={}\n", ret);
            finish(KAT_CRYPTO_FAILURE);
        }
        if mlen1 != mlen {
            eprint!("mlen mismatch\n");
            finish(KAT_CRYPTO_FAILURE);
        }
        if m[..mlen_us] != m1[..mlen_us] {
            eprint!("m mismatch\n");
            finish(KAT_CRYPTO_FAILURE);
        }
    }

    // Finalize transcript digest
    let mut digest = [0u8; 32];
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    print!("\n");

    finish(KAT_SUCCESS);
}

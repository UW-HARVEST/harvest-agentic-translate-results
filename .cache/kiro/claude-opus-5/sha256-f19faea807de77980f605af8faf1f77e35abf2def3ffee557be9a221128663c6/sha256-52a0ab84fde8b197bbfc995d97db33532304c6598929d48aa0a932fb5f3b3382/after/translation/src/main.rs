//! Translation of `app/src/PQCgenKAT_sign.c`.
//!
//! Performs an in-memory test of signing and verification before producing a
//! digest of the signature transcript with the selected hash backend.
//!
//! `crate-type` of the library is `cdylib`, which cannot be linked by a binary
//! in the same package, so the driver declares the same module tree itself.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::all)]

mod address;
mod backend;
mod context;
mod fors;
mod hash;
mod merkle;
mod params;
mod randombytes;
mod rng;
mod sign;
mod thash;
mod utils;
mod utilsx1;
mod vla;
mod wots;
mod wotsx1;

use params::*;

const MAX_MARKER_LEN: usize = 50;
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// ===========================================================================
// #ifdef BLAKE_TR
// ===========================================================================
#[cfg(backend_blake)]
mod kat {
    #[cfg(not(spx_n_ge_24))]
    use crate::backend::blake::blake256::SPX_BLAKE256_OUTPUT_BYTES;
    #[cfg(spx_n_ge_24)]
    use crate::backend::blake::blake512::SPX_BLAKE512_OUTPUT_BYTES;
    use crate::backend::blake::hash::{blakex_final, blakex_init, blakex_update, BlakeStateX};

    #[cfg(spx_n_ge_24)]
    const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
    #[cfg(not(spx_n_ge_24))]
    const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

    pub struct KatTr {
        s: BlakeStateX,
    }

    impl KatTr {
        pub fn new() -> Self {
            let mut s = BlakeStateX::new();
            blakex_init(&mut s);

            let tag: &[u8] = b"KAT-TRANSCRIPT-v1-BLAKE";
            blakex_update(&mut s, tag, tag.len() as u64);

            let sep = [0u8; 1];
            blakex_update(&mut s, &sep, 1);

            KatTr { s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            blakex_update(&mut self.s, p, p.len() as u64);

            let sep = [0u8; 1];
            blakex_update(&mut self.s, &sep, 1);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let le = x.to_le_bytes();
            let lenle = 8u64.to_le_bytes();
            blakex_update(&mut self.s, &lenle, 8);
            blakex_update(&mut self.s, &le, 8);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let lenle = (buf.len() as u64).to_le_bytes();
            blakex_update(&mut self.s, &lenle, 8);
            if !buf.is_empty() {
                blakex_update(&mut self.s, buf, buf.len() as u64);
            }
        }

        pub fn final32(&mut self) -> [u8; 32] {
            let mut outbuf = [0u8; BLAKEX_OUTPUT_BYTES];
            blakex_final(&mut self.s, &mut outbuf);
            let mut out32 = [0u8; 32];
            out32.copy_from_slice(&outbuf[..32]);
            out32
        }
    }
}

// ===========================================================================
// #elif HARAKA_TR
// ===========================================================================
#[cfg(backend_haraka)]
mod kat {
    use crate::backend::haraka::haraka::{
        haraka_s_inc_absorb_rs, haraka_s_inc_finalize_rs, haraka_s_inc_init_rs,
        haraka_s_inc_squeeze_rs, tweak_constants_rs,
    };
    use crate::context::SpxCtx;
    use crate::params::SPX_N;

    pub struct KatTr {
        inner: SpxCtx,
        s: [u8; 65],
    }

    impl KatTr {
        pub fn new() -> Self {
            let mut inner = SpxCtx::new();
            for i in 0..SPX_N {
                inner.pub_seed[i] = 0;
                inner.sk_seed[i] = 0;
            }

            tweak_constants_rs(&mut inner);
            let mut s = [0u8; 65];
            haraka_s_inc_init_rs(&mut s);

            let tag: &[u8] = b"KAT-TRANSCRIPT-v1-HARAKA";
            haraka_s_inc_absorb_rs(&mut s, tag, &inner);

            let sep = [0u8; 1];
            haraka_s_inc_absorb_rs(&mut s, &sep, &inner);

            KatTr { inner, s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            haraka_s_inc_absorb_rs(&mut self.s, p, &self.inner);

            let sep = [0u8; 1];
            haraka_s_inc_absorb_rs(&mut self.s, &sep, &self.inner);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let le = x.to_le_bytes();
            let lenle = 8u64.to_le_bytes();
            haraka_s_inc_absorb_rs(&mut self.s, &lenle, &self.inner);
            haraka_s_inc_absorb_rs(&mut self.s, &le, &self.inner);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let lenle = (buf.len() as u64).to_le_bytes();
            haraka_s_inc_absorb_rs(&mut self.s, &lenle, &self.inner);
            if !buf.is_empty() {
                haraka_s_inc_absorb_rs(&mut self.s, buf, &self.inner);
            }
        }

        pub fn final32(&mut self) -> [u8; 32] {
            haraka_s_inc_finalize_rs(&mut self.s);
            let mut out32 = [0u8; 32];
            haraka_s_inc_squeeze_rs(&mut out32, &mut self.s, &self.inner);
            out32
        }
    }
}

// ===========================================================================
// #elif SHA2_TR
// ===========================================================================
#[cfg(backend_sha2)]
mod kat {
    use crate::backend::sha2::hash::{
        shax_inc_blocks, shax_inc_finalize, shax_inc_init, SHAX_STATE_LEN,
        SPX_SHAX_BLOCK_BYTES, SPX_SHAX_OUTPUT_BYTES,
    };

    const BLK: usize = SPX_SHAX_BLOCK_BYTES;
    const OUT: usize = SPX_SHAX_OUTPUT_BYTES;

    pub struct KatTr {
        s: [u8; SHAX_STATE_LEN],
    }

    impl KatTr {
        pub fn new() -> Self {
            let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHA2";
            let mut block = [0u8; BLK];

            for i in 0..tag.len() {
                block[i] = tag[i];
            }
            for i in tag.len()..BLK {
                block[i] = 0;
            }

            let mut s = [0u8; SHAX_STATE_LEN];
            shax_inc_init(&mut s);
            shax_inc_blocks(&mut s, &block, 1);

            KatTr { s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            let block_count = (n + 1 + (BLK - 1)) / BLK;

            for i in 0..block_count {
                let mut block = [0u8; BLK];
                let mut j = 0usize;

                while i * BLK + j < n && j < BLK {
                    block[j] = p[i * BLK + j];
                    j += 1;
                }

                if i * BLK + j == n && j < BLK {
                    block[j] = 0x00;
                    j += 1;
                }

                while j < BLK {
                    block[j] = 0;
                    j += 1;
                }

                shax_inc_blocks(&mut self.s, &block, 1);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut block = [0u8; BLK];
            let le = x.to_le_bytes();
            let lenle = 8u64.to_le_bytes();

            block[..8].copy_from_slice(&lenle);
            block[8..16].copy_from_slice(&le);
            for i in 16..BLK {
                block[i] = 0;
            }

            shax_inc_blocks(&mut self.s, &block, 1);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; BLK];
            lenle[..8].copy_from_slice(&(len as u64).to_le_bytes());

            let block_count = (len + (BLK - 1)) / BLK;
            shax_inc_blocks(&mut self.s, &lenle, 1);

            if len != 0 {
                for i in 0..block_count {
                    let mut block = [0u8; BLK];
                    let mut j = 0usize;

                    while i * BLK + j < len && j < BLK {
                        block[j] = buf[i * BLK + j];
                        j += 1;
                    }
                    while j < BLK {
                        block[j] = 0;
                        j += 1;
                    }

                    shax_inc_blocks(&mut self.s, &block, 1);
                }
            }
        }

        pub fn final32(&mut self) -> [u8; 32] {
            let mut outbuf = [0u8; OUT];
            let final_block = [0u8; BLK];
            shax_inc_finalize(&mut outbuf, &mut self.s, &final_block[..1]);
            let mut out32 = [0u8; 32];
            out32.copy_from_slice(&outbuf[..32]);
            out32
        }
    }
}

// ===========================================================================
// #elif SHAKE_TR
// ===========================================================================
#[cfg(backend_shake)]
mod kat {
    use crate::backend::shake::fips202::{
        shake256_inc_absorb_rs, shake256_inc_finalize_rs, shake256_inc_init_rs,
        shake256_inc_squeeze_rs,
    };

    pub struct KatTr {
        s: [u64; 26],
    }

    impl KatTr {
        pub fn new() -> Self {
            let mut s = [0u64; 26];
            shake256_inc_init_rs(&mut s);

            let tag: &[u8] = b"KAT-TRANSCRIPT-v1-SHAKE";
            shake256_inc_absorb_rs(&mut s, tag);

            let sep = [0u8; 1];
            shake256_inc_absorb_rs(&mut s, &sep);

            KatTr { s }
        }

        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            shake256_inc_absorb_rs(&mut self.s, p);

            let sep = [0u8; 1];
            shake256_inc_absorb_rs(&mut self.s, &sep);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let le = x.to_le_bytes();
            let lenle = 8u64.to_le_bytes();
            shake256_inc_absorb_rs(&mut self.s, &lenle);
            shake256_inc_absorb_rs(&mut self.s, &le);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let lenle = (buf.len() as u64).to_le_bytes();
            shake256_inc_absorb_rs(&mut self.s, &lenle);
            if !buf.is_empty() {
                shake256_inc_absorb_rs(&mut self.s, buf);
            }
        }

        pub fn final32(&mut self) -> [u8; 32] {
            shake256_inc_finalize_rs(&mut self.s);
            let mut out32 = [0u8; 32];
            shake256_inc_squeeze_rs(&mut out32, &mut self.s);
            out32
        }
    }
}

fn main() {
    std::process::exit(run());
}

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
    rng::randombytes_init_rs(&entropy_input, None);

    // Initialize Transcript
    let mut tctx = kat::KatTr::new();
    tctx.absorb_label("CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME.as_bytes());
    tctx.absorb_label("SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label("PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label("SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes_drbg(&mut seed);

        tctx.absorb_label("count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label("seed");
        tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            return KAT_OVERFLOW;
        }

        tctx.absorb_label("mlen");
        tctx.absorb_u64(mlen as u64);

        rng::randombytes_drbg(&mut msg[..mlen]);
        tctx.absorb_label("msg");
        tctx.absorb_bytes(&msg[..mlen]);

        for b in m[..mlen].iter_mut() {
            *b = 0;
        }
        for b in m1[..mlen + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        for b in sm[..mlen + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = sign::crypto_sign_keypair_rs(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        tctx.absorb_label("pk");
        tctx.absorb_bytes(&pk);
        tctx.absorb_label("sk");
        tctx.absorb_bytes(&sk);

        // Sign
        let mut smlen: core::ffi::c_ulonglong = 0;
        let ret = unsafe {
            sign::crypto_sign(
                sm.as_mut_ptr(),
                &mut smlen,
                m.as_ptr(),
                mlen as core::ffi::c_ulonglong,
                sk.as_ptr(),
            )
        };
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        tctx.absorb_label("smlen");
        tctx.absorb_u64(smlen as u64);
        tctx.absorb_label("sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);

        // Verify
        let mut mlen1: core::ffi::c_ulonglong = 0;
        let ret = unsafe {
            sign::crypto_sign_open(
                m1.as_mut_ptr(),
                &mut mlen1,
                sm.as_ptr(),
                smlen,
                pk.as_ptr(),
            )
        };
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
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
    let digest = tctx.final32();

    print!("KAT transcript digest = ");
    for b in digest.iter() {
        print!("{:02X}", b);
    }
    println!();

    KAT_SUCCESS
}

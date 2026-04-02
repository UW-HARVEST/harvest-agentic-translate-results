#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]

mod params;
mod context;
mod address;
mod utils;
mod rng;
mod hash;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod sign;
mod fips202;
mod sha2_impl;
mod blake_impl;
#[cfg(feature = "haraka")]
mod haraka_impl;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// ===== KAT Transcript implementations =====

// BLAKE transcript
#[cfg(feature = "blake")]
mod kat_tr {
    use crate::params::*;
    use crate::blake_impl::*;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub struct KatTrCtx {
        s: BlakeState512,
    }

    #[cfg(any(feature = "128s", feature = "128f"))]
    pub struct KatTrCtx {
        s: BlakeState256,
    }

    impl KatTrCtx {
        pub fn init() -> Self {
            #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
            {
                let mut s = BlakeState512::new();
                blake512_init(&mut s);
                let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
                blake512_update(&mut s, tag, tag.len() as u64);
                let sep = [0x00u8];
                blake512_update(&mut s, &sep, 1);
                KatTrCtx { s }
            }
            #[cfg(any(feature = "128s", feature = "128f"))]
            {
                let mut s = BlakeState256::new();
                blake256_init(&mut s);
                let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
                blake256_update(&mut s, tag, tag.len() as u64);
                let sep = [0x00u8];
                blake256_update(&mut s, &sep, 1);
                KatTrCtx { s }
            }
        }

        pub fn absorb_label(&mut self, label: &[u8]) {
            let n = label.len();
            #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
            {
                blake512_update(&mut self.s, label, n as u64);
                let sep = [0x00u8];
                blake512_update(&mut self.s, &sep, 1);
            }
            #[cfg(any(feature = "128s", feature = "128f"))]
            {
                blake256_update(&mut self.s, label, n as u64);
                let sep = [0x00u8];
                blake256_update(&mut self.s, &sep, 1);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }
            let mut lenle = [0u8; 8];
            let l: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
            {
                blake512_update(&mut self.s, &lenle, 8);
                blake512_update(&mut self.s, &le, 8);
            }
            #[cfg(any(feature = "128s", feature = "128f"))]
            {
                blake256_update(&mut self.s, &lenle, 8);
                blake256_update(&mut self.s, &le, 8);
            }
        }

        pub fn absorb_bytes(&mut self, buf: &[u8], len: usize) {
            let mut lenle = [0u8; 8];
            let l = len as u64;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
            {
                blake512_update(&mut self.s, &lenle, 8);
                if len > 0 {
                    blake512_update(&mut self.s, buf, len as u64);
                }
            }
            #[cfg(any(feature = "128s", feature = "128f"))]
            {
                blake256_update(&mut self.s, &lenle, 8);
                if len > 0 {
                    blake256_update(&mut self.s, buf, len as u64);
                }
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
            {
                let mut outbuf = [0u8; 64];
                blake512_final(&mut self.s, &mut outbuf);
                out32.copy_from_slice(&outbuf[..32]);
            }
            #[cfg(any(feature = "128s", feature = "128f"))]
            {
                let mut outbuf = [0u8; 32];
                blake256_final(&mut self.s, &mut outbuf);
                out32.copy_from_slice(&outbuf[..32]);
            }
        }
    }
}

// HARAKA transcript
#[cfg(feature = "haraka")]
mod kat_tr {
    use crate::context::SpxCtx;
    use crate::params::*;
    use crate::haraka_impl::*;

    pub struct KatTrCtx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    impl KatTrCtx {
        pub fn init() -> Self {
            let mut inner = SpxCtx::new();
            for i in 0..SPX_N {
                inner.pub_seed[i] = 0;
                inner.sk_seed[i] = 0;
            }
            tweak_constants(&mut inner);
            let mut s = [0u8; 65];
            haraka_S_inc_init(&mut s);
            let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
            haraka_S_inc_absorb(&mut s, tag, tag.len(), &inner);
            let sep = [0x00u8];
            haraka_S_inc_absorb(&mut s, &sep, 1, &inner);
            KatTrCtx { inner, s }
        }

        pub fn absorb_label(&mut self, label: &[u8]) {
            let n = label.len();
            haraka_S_inc_absorb(&mut self.s, label, n, &self.inner);
            let sep = [0x00u8];
            haraka_S_inc_absorb(&mut self.s, &sep, 1, &self.inner);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }
            let mut lenle = [0u8; 8];
            let l: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            haraka_S_inc_absorb(&mut self.s, &lenle, 8, &self.inner);
            haraka_S_inc_absorb(&mut self.s, &le, 8, &self.inner);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8], len: usize) {
            let mut lenle = [0u8; 8];
            let l = len as u64;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            haraka_S_inc_absorb(&mut self.s, &lenle, 8, &self.inner);
            if len > 0 {
                haraka_S_inc_absorb(&mut self.s, buf, len, &self.inner);
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            haraka_S_inc_finalize(&mut self.s);
            haraka_S_inc_squeeze(out32, 32, &mut self.s, &self.inner);
        }
    }
}

// SHA2 transcript
#[cfg(feature = "sha2")]
mod kat_tr {
    use crate::sha2_impl::*;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const SHAX_STATE_LEN: usize = 72;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const SHAX_STATE_LEN: usize = 40;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const SHAX_BLOCK_BYTES: usize = 128;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const SHAX_BLOCK_BYTES: usize = 64;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const SHAX_OUTPUT_BYTES: usize = 64;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const SHAX_OUTPUT_BYTES: usize = 32;

    pub struct KatTrCtx {
        s: [u8; SHAX_STATE_LEN],
    }

    fn shax_inc_init(state: &mut [u8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let arr: &mut [u8; 72] = state.try_into().unwrap();
            sha512_inc_init(arr);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let arr: &mut [u8; 40] = state.try_into().unwrap();
            sha256_inc_init(arr);
        }
    }

    fn shax_inc_blocks(state: &mut [u8], data: &[u8], nblocks: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let arr: &mut [u8; 72] = state.try_into().unwrap();
            sha512_inc_blocks(arr, data, nblocks);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let arr: &mut [u8; 40] = state.try_into().unwrap();
            sha256_inc_blocks(arr, data, nblocks);
        }
    }

    fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], datalen: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let arr: &mut [u8; 72] = state.try_into().unwrap();
            sha512_inc_finalize(out, arr, data, datalen);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let arr: &mut [u8; 40] = state.try_into().unwrap();
            sha256_inc_finalize(out, arr, data, datalen);
        }
    }

    impl KatTrCtx {
        pub fn init() -> Self {
            let tag = b"KAT-TRANSCRIPT-v1-SHA2";
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            for i in 0..tag.len() {
                block[i] = tag[i];
            }
            // rest already zero

            let mut s = [0u8; SHAX_STATE_LEN];
            shax_inc_init(&mut s);
            shax_inc_blocks(&mut s, &block, 1);
            KatTrCtx { s }
        }

        pub fn absorb_label(&mut self, label: &[u8]) {
            let n = label.len();
            let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

            for i in 0..block_count {
                let mut block = [0u8; SHAX_BLOCK_BYTES];
                let mut j = 0;
                while i * SHAX_BLOCK_BYTES + j < n && j < SHAX_BLOCK_BYTES {
                    block[j] = label[i * SHAX_BLOCK_BYTES + j];
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
                shax_inc_blocks(&mut self.s, &block, 1);
            }
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }
            let mut lenle = [0u8; 8];
            let l: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
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
            shax_inc_blocks(&mut self.s, &block, 1);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8], len: usize) {
            let mut lenle = [0u8; SHAX_BLOCK_BYTES];
            let l = len as u64;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            let block_count = if len == 0 { 0 } else { (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES };
            shax_inc_blocks(&mut self.s, &lenle, 1);

            if len != 0 {
                for i in 0..block_count {
                    let mut block = [0u8; SHAX_BLOCK_BYTES];
                    let mut j = 0;
                    while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                        block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                        j += 1;
                    }
                    while j < SHAX_BLOCK_BYTES {
                        block[j] = 0;
                        j += 1;
                    }
                    shax_inc_blocks(&mut self.s, &block, 1);
                }
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
            let final_block = [0u8; SHAX_BLOCK_BYTES];
            shax_inc_finalize(&mut outbuf, &mut self.s, &final_block, 1);
            out32.copy_from_slice(&outbuf[..32]);
        }
    }
}

// SHAKE transcript
#[cfg(feature = "shake")]
mod kat_tr {
    use crate::fips202::*;

    pub struct KatTrCtx {
        s: [u64; 26],
    }

    impl KatTrCtx {
        pub fn init() -> Self {
            let mut s = [0u64; 26];
            shake256_inc_init(&mut s);
            let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
            shake256_inc_absorb(&mut s, tag, tag.len());
            let sep = [0x00u8];
            shake256_inc_absorb(&mut s, &sep, 1);
            KatTrCtx { s }
        }

        pub fn absorb_label(&mut self, label: &[u8]) {
            let n = label.len();
            shake256_inc_absorb(&mut self.s, label, n);
            let sep = [0x00u8];
            shake256_inc_absorb(&mut self.s, &sep, 1);
        }

        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = ((x >> (8 * i)) & 0xFF) as u8;
            }
            let mut lenle = [0u8; 8];
            let l: u64 = 8;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            shake256_inc_absorb(&mut self.s, &lenle, 8);
            shake256_inc_absorb(&mut self.s, &le, 8);
        }

        pub fn absorb_bytes(&mut self, buf: &[u8], len: usize) {
            let mut lenle = [0u8; 8];
            let l = len as u64;
            for i in 0..8 {
                lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
            }
            shake256_inc_absorb(&mut self.s, &lenle, 8);
            if len > 0 {
                shake256_inc_absorb(&mut self.s, buf, len);
            }
        }

        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            shake256_inc_finalize(&mut self.s);
            shake256_inc_squeeze(out32, 32, &mut self.s);
        }
    }
}

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    // Deterministic entropy
    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    rng::randombytes_init(&entropy_input, None);

    // Initialize transcript
    let mut tctx = kat_tr::KatTrCtx::init();
    tctx.absorb_label(b"CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME.as_bytes(), CRYPTO_ALGNAME.len());
    tctx.absorb_label(b"SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label(b"PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label(b"SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed, 48);

        tctx.absorb_label(b"count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label(b"seed");
        tctx.absorb_bytes(&seed, 48);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(KAT_OVERFLOW);
        }

        tctx.absorb_label(b"mlen");
        tctx.absorb_u64(mlen as u64);

        rng::randombytes(&mut msg[..mlen], mlen as u64);
        tctx.absorb_label(b"msg");
        tctx.absorb_bytes(&msg, mlen);

        for j in 0..mlen {
            m[j] = 0;
        }
        for j in 0..mlen + CRYPTO_BYTES {
            m1[j] = 0;
            sm[j] = 0;
        }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        tctx.absorb_label(b"pk");
        tctx.absorb_bytes(&pk, CRYPTO_PUBLICKEYBYTES);
        tctx.absorb_label(b"sk");
        tctx.absorb_bytes(&sk, CRYPTO_SECRETKEYBYTES);

        // Sign
        let mut smlen: u64 = 0;
        let ret = sign::crypto_sign(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        tctx.absorb_label(b"smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label(b"sm");
        tctx.absorb_bytes(&sm, smlen as usize);

        // Verify
        let mut mlen1: u64 = 0;
        let ret = sign::crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if mlen1 != mlen as u64 {
            eprintln!("mlen mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
    }

    // Finalize transcript digest
    let mut digest = [0u8; 32];
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

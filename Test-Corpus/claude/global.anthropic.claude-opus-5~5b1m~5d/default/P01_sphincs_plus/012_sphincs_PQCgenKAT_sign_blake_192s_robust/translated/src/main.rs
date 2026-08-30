//! Translation of `app/src/PQCgenKAT_sign.c` — the KAT transcript driver.

#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]

use sphincs_plus::params::*;
use sphincs_plus::rng::{randombytes, randombytes_init};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_CRYPTO_FAILURE: i32 = -2;

// ==================================================================
// Per-backend KAT transcript implementation.
// ==================================================================

#[cfg(spx_backend = "haraka")]
mod tr {
    use sphincs_plus::context::SpxCtx;
    use sphincs_plus::haraka::{
        haraka_s_inc_absorb, haraka_s_inc_finalize, haraka_s_inc_init, haraka_s_inc_squeeze,
        tweak_constants,
    };

    pub struct KatTr {
        inner: SpxCtx,
        s: [u8; 65],
    }

    impl KatTr {
        pub fn init() -> Self {
            let mut inner = SpxCtx::new();
            // pub_seed / sk_seed already zeroed.
            tweak_constants(&mut inner);
            let mut s = [0u8; 65];
            haraka_s_inc_init(&mut s);
            let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
            haraka_s_inc_absorb(&mut s, tag, tag.len(), &inner);
            haraka_s_inc_absorb(&mut s, &[0u8], 1, &inner);
            KatTr { inner, s }
        }
        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            haraka_s_inc_absorb(&mut self.s, p, p.len(), &self.inner);
            haraka_s_inc_absorb(&mut self.s, &[0u8], 1, &self.inner);
        }
        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = (x >> (8 * i)) as u8;
            }
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = (8u64 >> (8 * i)) as u8;
            }
            haraka_s_inc_absorb(&mut self.s, &lenle, 8, &self.inner);
            haraka_s_inc_absorb(&mut self.s, &le, 8, &self.inner);
        }
        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = ((len as u64) >> (8 * i)) as u8;
            }
            haraka_s_inc_absorb(&mut self.s, &lenle, 8, &self.inner);
            if len != 0 {
                haraka_s_inc_absorb(&mut self.s, buf, len, &self.inner);
            }
        }
        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            haraka_s_inc_finalize(&mut self.s);
            haraka_s_inc_squeeze(out32, 32, &mut self.s, &self.inner);
        }
    }
}

#[cfg(spx_backend = "shake")]
mod tr {
    use sphincs_plus::fips202::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    pub struct KatTr {
        s: [u64; 26],
    }

    impl KatTr {
        pub fn init() -> Self {
            let mut s = [0u64; 26];
            shake256_inc_init(&mut s);
            let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
            shake256_inc_absorb(&mut s, tag, tag.len());
            shake256_inc_absorb(&mut s, &[0u8], 1);
            KatTr { s }
        }
        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            shake256_inc_absorb(&mut self.s, p, p.len());
            shake256_inc_absorb(&mut self.s, &[0u8], 1);
        }
        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = (x >> (8 * i)) as u8;
            }
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = (8u64 >> (8 * i)) as u8;
            }
            shake256_inc_absorb(&mut self.s, &lenle, 8);
            shake256_inc_absorb(&mut self.s, &le, 8);
        }
        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = ((len as u64) >> (8 * i)) as u8;
            }
            shake256_inc_absorb(&mut self.s, &lenle, 8);
            if len != 0 {
                shake256_inc_absorb(&mut self.s, buf, len);
            }
        }
        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            shake256_inc_finalize(&mut self.s);
            shake256_inc_squeeze(out32, 32, &mut self.s);
        }
    }
}

#[cfg(spx_backend = "blake")]
mod tr {
    use sphincs_plus::blake256::{blake256_final, blake256_init, blake256_update, BlakeState256};
    use sphincs_plus::blake512::{blake512_final, blake512_init, blake512_update, BlakeState512};
    use sphincs_plus::params::SPX_N;

    const OUTPUT: usize = if SPX_N >= 24 { 64 } else { 32 };

    pub struct KatTr {
        s256: BlakeState256,
        s512: BlakeState512,
    }

    impl KatTr {
        pub fn init() -> Self {
            let mut s256 = BlakeState256::new();
            let mut s512 = BlakeState512::new();
            let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
            if SPX_N >= 24 {
                blake512_init(&mut s512);
                blake512_update(&mut s512, tag, tag.len() as u64);
                blake512_update(&mut s512, &[0u8], 1);
            } else {
                blake256_init(&mut s256);
                blake256_update(&mut s256, tag, tag.len() as u64);
                blake256_update(&mut s256, &[0u8], 1);
            }
            KatTr { s256, s512 }
        }
        fn update(&mut self, data: &[u8], n: u64) {
            if SPX_N >= 24 {
                blake512_update(&mut self.s512, data, n);
            } else {
                blake256_update(&mut self.s256, data, n);
            }
        }
        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            self.update(p, p.len() as u64);
            self.update(&[0u8], 1);
        }
        pub fn absorb_u64(&mut self, x: u64) {
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = (x >> (8 * i)) as u8;
            }
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = (8u64 >> (8 * i)) as u8;
            }
            self.update(&lenle, 8);
            self.update(&le, 8);
        }
        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = ((len as u64) >> (8 * i)) as u8;
            }
            self.update(&lenle, 8);
            if len != 0 {
                self.update(buf, len as u64);
            }
        }
        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            let mut outbuf = [0u8; OUTPUT];
            if SPX_N >= 24 {
                blake512_final(&mut self.s512, &mut outbuf);
            } else {
                blake256_final(&mut self.s256, &mut outbuf);
            }
            out32.copy_from_slice(&outbuf[..32]);
        }
    }
}

#[cfg(spx_backend = "sha2")]
mod tr {
    #[cfg(spx_sha512)]
    use sphincs_plus::sha2::{
        sha512_inc_blocks as shax_inc_blocks, sha512_inc_finalize as shax_inc_finalize,
        sha512_inc_init as shax_inc_init,
    };
    #[cfg(not(spx_sha512))]
    use sphincs_plus::sha2::{
        sha256_inc_blocks as shax_inc_blocks, sha256_inc_finalize as shax_inc_finalize,
        sha256_inc_init as shax_inc_init,
    };

    #[cfg(spx_sha512)]
    const STATE: usize = 72;
    #[cfg(spx_sha512)]
    const BLOCK: usize = 128;
    #[cfg(spx_sha512)]
    const OUTPUT: usize = 64;
    #[cfg(not(spx_sha512))]
    const STATE: usize = 40;
    #[cfg(not(spx_sha512))]
    const BLOCK: usize = 64;
    #[cfg(not(spx_sha512))]
    const OUTPUT: usize = 32;

    pub struct KatTr {
        s: [u8; STATE],
    }

    impl KatTr {
        pub fn init() -> Self {
            let tag = b"KAT-TRANSCRIPT-v1-SHA2";
            let mut block = [0u8; BLOCK];
            for i in 0..tag.len() {
                block[i] = tag[i];
            }
            let mut s = [0u8; STATE];
            shax_inc_init(&mut s);
            shax_inc_blocks(&mut s, &block, 1);
            KatTr { s }
        }
        pub fn absorb_label(&mut self, label: &str) {
            let p = label.as_bytes();
            let n = p.len();
            let block_count = (n + 1 + (BLOCK - 1)) / BLOCK;
            for i in 0..block_count {
                let mut block = [0u8; BLOCK];
                let mut j = 0usize;
                while i * BLOCK + j < n && j < BLOCK {
                    block[j] = p[i * BLOCK + j];
                    j += 1;
                }
                if i * BLOCK + j == n && j < BLOCK {
                    block[j] = 0;
                    j += 1;
                }
                while j < BLOCK {
                    block[j] = 0;
                    j += 1;
                }
                shax_inc_blocks(&mut self.s, &block, 1);
            }
        }
        pub fn absorb_u64(&mut self, x: u64) {
            let mut block = [0u8; BLOCK];
            let mut le = [0u8; 8];
            for i in 0..8 {
                le[i] = (x >> (8 * i)) as u8;
            }
            let mut lenle = [0u8; 8];
            for i in 0..8 {
                lenle[i] = (8u64 >> (8 * i)) as u8;
            }
            block[0..8].copy_from_slice(&lenle);
            block[8..16].copy_from_slice(&le);
            shax_inc_blocks(&mut self.s, &block, 1);
        }
        pub fn absorb_bytes(&mut self, buf: &[u8]) {
            let len = buf.len();
            let mut lenle = [0u8; BLOCK];
            for i in 0..8 {
                lenle[i] = ((len as u64) >> (8 * i)) as u8;
            }
            let block_count = (len + (BLOCK - 1)) / BLOCK;
            shax_inc_blocks(&mut self.s, &lenle, 1);
            if len != 0 {
                for i in 0..block_count {
                    let mut block = [0u8; BLOCK];
                    let mut j = 0usize;
                    while i * BLOCK + j < len && j < BLOCK {
                        block[j] = buf[i * BLOCK + j];
                        j += 1;
                    }
                    while j < BLOCK {
                        block[j] = 0;
                        j += 1;
                    }
                    shax_inc_blocks(&mut self.s, &block, 1);
                }
            }
        }
        pub fn finalize(&mut self, out32: &mut [u8; 32]) {
            let mut outbuf = [0u8; OUTPUT];
            let final_block = [0u8; BLOCK];
            shax_inc_finalize(&mut outbuf, &mut self.s, &final_block, 1);
            out32.copy_from_slice(&outbuf[..32]);
        }
    }
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

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    let mut tctx = tr::KatTr::init();
    tctx.absorb_label("CRYPTO_ALGNAME");
    tctx.absorb_bytes(&CRYPTO_ALGNAME.as_bytes()[..CRYPTO_ALGNAME.len()]);
    tctx.absorb_label("SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label("PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label("SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed);

        tctx.absorb_label("count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label("seed");
        tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);

        tctx.absorb_label("mlen");
        tctx.absorb_u64(mlen as u64);

        randombytes(&mut msg[..mlen]);
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

        let mut smlen: u64 = 0;
        let mut mlen1: u64 = 0;

        unsafe {
            let ret = sphincs_plus::sign::crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr());
            if ret != 0 {
                eprintln!("crypto_sign_keypair={}", ret);
                return KAT_CRYPTO_FAILURE;
            }
        }
        tctx.absorb_label("pk");
        tctx.absorb_bytes(&pk);
        tctx.absorb_label("sk");
        tctx.absorb_bytes(&sk);

        unsafe {
            let ret = sphincs_plus::sign::crypto_sign(
                sm.as_mut_ptr(),
                &mut smlen as *mut u64,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
            if ret != 0 {
                eprintln!("crypto_sign={}", ret);
                return KAT_CRYPTO_FAILURE;
            }
        }
        tctx.absorb_label("smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label("sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);

        unsafe {
            let ret = sphincs_plus::sign::crypto_sign_open(
                m1.as_mut_ptr(),
                &mut mlen1 as *mut u64,
                sm.as_ptr(),
                smlen,
                pk.as_ptr(),
            );
            if ret != 0 {
                eprintln!("crypto_sign_open={}", ret);
                return KAT_CRYPTO_FAILURE;
            }
        }
        if mlen1 != mlen as u64 {
            eprintln!("mlen mismatch");
            return KAT_CRYPTO_FAILURE;
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            return KAT_CRYPTO_FAILURE;
        }
    }

    let mut digest = [0u8; 32];
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for b in digest.iter() {
        print!("{:02X}", b);
    }
    println!();

    KAT_SUCCESS
}

fn main() {
    let code = run();
    std::process::exit(code);
}

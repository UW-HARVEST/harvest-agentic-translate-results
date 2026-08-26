// PQCgenKAT_sign main - test driver
#![allow(non_snake_case, dead_code)]

use sphincsplus::params::*;
use sphincsplus::rng::{randombytes, randombytes_init};
use sphincsplus::sign::{
    crypto_sign_keypair_rs as crypto_sign_keypair, crypto_sign_open_rs as crypto_sign_open,
    crypto_sign_rs as crypto_sign,
};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

#[cfg(feature = "blake")]
use sphincsplus::blake::*;

#[cfg(feature = "shake")]
use sphincsplus::fips202::*;

#[cfg(feature = "sha2")]
use sphincsplus::sha2::*;

#[cfg(feature = "haraka")]
use sphincsplus::context::SpxCtx;
#[cfg(feature = "haraka")]
use sphincsplus::haraka::*;

// =================== BLAKE TR ===================
#[cfg(feature = "blake")]
mod tr {
    use super::*;

    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    pub type KatTrCtx = BlakeState512;
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    pub const BLAKEX_OUTPUT_BYTES: usize = 64;

    #[cfg(any(feature = "128f", feature = "128s"))]
    pub type KatTrCtx = BlakeState256;
    #[cfg(any(feature = "128f", feature = "128s"))]
    pub const BLAKEX_OUTPUT_BYTES: usize = 32;

    pub fn new_ctx() -> KatTrCtx {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        return BlakeState512::new();
        #[cfg(any(feature = "128f", feature = "128s"))]
        return BlakeState256::new();
    }

    fn blake_init(s: &mut KatTrCtx) {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        blake512_init(s);
        #[cfg(any(feature = "128f", feature = "128s"))]
        blake256_init(s);
    }
    fn blake_update(s: &mut KatTrCtx, data: &[u8], datalen: u64) {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        blake512_update(s, data, datalen);
        #[cfg(any(feature = "128f", feature = "128s"))]
        blake256_update(s, data, datalen);
    }
    fn blake_final(s: &mut KatTrCtx, out: &mut [u8]) {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        blake512_final(s, out);
        #[cfg(any(feature = "128f", feature = "128s"))]
        blake256_final(s, out);
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        blake_init(ctx);
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        // Note: C calls blakeX_update with byte length (interpreted as bits) — preserved.
        blake_update(ctx, tag, tag.len() as u64);
        let sep = [0x00u8];
        blake_update(ctx, &sep, 1);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        blake_update(ctx, p, p.len() as u64);
        let sep = [0x00u8];
        blake_update(ctx, &sep, 1);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        blake_update(ctx, &lenle, 8);
        blake_update(ctx, &le, 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        blake_update(ctx, &lenle, 8);
        if len > 0 {
            blake_update(ctx, buf, len as u64);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8]) {
        let mut outbuf = vec![0u8; BLAKEX_OUTPUT_BYTES];
        blake_final(ctx, &mut outbuf);
        out32[..32].copy_from_slice(&outbuf[..32]);
    }
}

// =================== HARAKA TR ===================
#[cfg(feature = "haraka")]
mod tr {
    use super::*;

    pub struct KatTrCtx {
        pub inner: SpxCtx,
        pub s: [u8; 65],
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx {
            inner: SpxCtx::new(),
            s: [0u8; 65],
        }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        for i in 0..SPX_N {
            ctx.inner.pub_seed[i] = 0;
            ctx.inner.sk_seed[i] = 0;
        }
        tweak_constants(&mut ctx.inner);
        haraka_S_inc_init(&mut ctx.s);

        let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
        haraka_S_inc_absorb(&mut ctx.s, tag, tag.len(), &ctx.inner.clone());

        let sep = [0x00u8];
        haraka_S_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner.clone());
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        haraka_S_inc_absorb(&mut ctx.s, p, p.len(), &ctx.inner.clone());
        let sep = [0x00u8];
        haraka_S_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner.clone());
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        haraka_S_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner.clone());
        haraka_S_inc_absorb(&mut ctx.s, &le, 8, &ctx.inner.clone());
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        haraka_S_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner.clone());
        if len > 0 {
            haraka_S_inc_absorb(&mut ctx.s, buf, len, &ctx.inner.clone());
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8]) {
        haraka_S_inc_finalize(&mut ctx.s);
        let inner_clone = ctx.inner.clone();
        haraka_S_inc_squeeze(out32, 32, &mut ctx.s, &inner_clone);
    }
}

// =================== SHA2 TR ===================
#[cfg(feature = "sha2")]
mod tr {
    use super::*;

    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    pub const SHAX_STATE_LEN: usize = 72;
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    pub const SHAX_BLOCK_BYTES: usize = 128;
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    pub const SHAX_OUTPUT_BYTES: usize = 64;

    #[cfg(any(feature = "128f", feature = "128s"))]
    pub const SHAX_STATE_LEN: usize = 40;
    #[cfg(any(feature = "128f", feature = "128s"))]
    pub const SHAX_BLOCK_BYTES: usize = 64;
    #[cfg(any(feature = "128f", feature = "128s"))]
    pub const SHAX_OUTPUT_BYTES: usize = 32;

    pub struct KatTrCtx {
        pub s: Vec<u8>,
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx {
            s: vec![0u8; SHAX_STATE_LEN],
        }
    }

    fn shax_inc_init(s: &mut [u8]) {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        sha512_inc_init(s);
        #[cfg(any(feature = "128f", feature = "128s"))]
        sha256_inc_init(s);
    }
    fn shax_inc_blocks(s: &mut [u8], data: &[u8], blocks: usize) {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        sha512_inc_blocks(s, data, blocks);
        #[cfg(any(feature = "128f", feature = "128s"))]
        sha256_inc_blocks(s, data, blocks);
    }
    fn shax_inc_finalize(out: &mut [u8], s: &mut [u8], data: &[u8], inlen: usize) {
        #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
        sha512_inc_finalize(out, s, data, inlen);
        #[cfg(any(feature = "128f", feature = "128s"))]
        sha256_inc_finalize(out, s, data, inlen);
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = vec![0u8; SHAX_BLOCK_BYTES];
        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        shax_inc_init(&mut ctx.s);
        shax_inc_blocks(&mut ctx.s, &block, 1);
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

        for i in 0..block_count {
            let mut block = vec![0u8; SHAX_BLOCK_BYTES];
            let mut j = 0;
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
            shax_inc_blocks(&mut ctx.s, &block, 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = vec![0u8; SHAX_BLOCK_BYTES];
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
        shax_inc_blocks(&mut ctx.s, &block, 1);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = vec![0u8; SHAX_BLOCK_BYTES];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
        shax_inc_blocks(&mut ctx.s, &lenle, 1);
        if len != 0 {
            for i in 0..block_count {
                let mut block = vec![0u8; SHAX_BLOCK_BYTES];
                let mut j = 0;
                while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                    block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                    j += 1;
                }
                while j < SHAX_BLOCK_BYTES {
                    block[j] = 0;
                    j += 1;
                }
                shax_inc_blocks(&mut ctx.s, &block, 1);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8]) {
        let mut outbuf = vec![0u8; SHAX_OUTPUT_BYTES];
        let final_block = vec![0u8; SHAX_BLOCK_BYTES];
        shax_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
        out32[..32].copy_from_slice(&outbuf[..32]);
    }
}

// =================== SHAKE TR ===================
#[cfg(feature = "shake")]
mod tr {
    use super::*;

    pub struct KatTrCtx {
        pub s: [u64; 26],
    }

    pub fn new_ctx() -> KatTrCtx {
        KatTrCtx { s: [0u64; 26] }
    }

    pub fn kat_tr_init(ctx: &mut KatTrCtx) {
        shake256_inc_init(&mut ctx.s);
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
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
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        shake256_inc_absorb(&mut ctx.s, &le, 8);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        shake256_inc_absorb(&mut ctx.s, &lenle, 8);
        if len > 0 {
            shake256_inc_absorb(&mut ctx.s, buf, len);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8]) {
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(out32, 32, &mut ctx.s);
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

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    let mut tctx = tr::new_ctx();
    tr::kat_tr_init(&mut tctx);
    tr::kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    tr::kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes(), CRYPTO_ALGNAME.len());
    tr::kat_tr_absorb_label(&mut tctx, "SKBYTES");
    tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    tr::kat_tr_absorb_label(&mut tctx, "PKBYTES");
    tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    tr::kat_tr_absorb_label(&mut tctx, "SIGBYTES");
    tr::kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);

        tr::kat_tr_absorb_label(&mut tctx, "count");
        tr::kat_tr_absorb_u64(&mut tctx, i as u64);
        tr::kat_tr_absorb_label(&mut tctx, "seed");
        tr::kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen as usize > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit((-1i32 as u32) as i32);
        }

        tr::kat_tr_absorb_label(&mut tctx, "mlen");
        tr::kat_tr_absorb_u64(&mut tctx, mlen);

        randombytes(&mut msg, mlen as usize);
        tr::kat_tr_absorb_label(&mut tctx, "msg");
        tr::kat_tr_absorb_bytes(&mut tctx, &msg, mlen as usize);

        for j in 0..mlen as usize {
            m[j] = 0;
            m1[j] = 0;
            sm[j] = 0;
        }
        for j in 0..(mlen as usize + CRYPTO_BYTES) {
            m1[j] = 0;
            sm[j] = 0;
        }
        m[..mlen as usize].copy_from_slice(&msg[..mlen as usize]);

        // Keypair
        let r = crypto_sign_keypair(&mut pk, &mut sk);
        if r != 0 {
            eprintln!("crypto_sign_keypair={}", r);
            std::process::exit(((-2i32) as u32) as i32);
        }
        tr::kat_tr_absorb_label(&mut tctx, "pk");
        tr::kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        tr::kat_tr_absorb_label(&mut tctx, "sk");
        tr::kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        // Sign
        let mut smlen: u64 = 0;
        let r = crypto_sign(&mut sm, &mut smlen, &m[..mlen as usize], mlen, &sk);
        if r != 0 {
            eprintln!("crypto_sign={}", r);
            std::process::exit(((-2i32) as u32) as i32);
        }
        tr::kat_tr_absorb_label(&mut tctx, "smlen");
        tr::kat_tr_absorb_u64(&mut tctx, smlen);
        tr::kat_tr_absorb_label(&mut tctx, "sm");
        tr::kat_tr_absorb_bytes(&mut tctx, &sm[..smlen as usize], smlen as usize);

        // Verify
        let mut mlen1: u64 = 0;
        let r = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], smlen, &pk);
        if r != 0 {
            eprintln!("crypto_sign_open={}", r);
            std::process::exit(((-2i32) as u32) as i32);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(((-2i32) as u32) as i32);
        }
        if m[..mlen as usize] != m1[..mlen as usize] {
            eprintln!("m mismatch");
            std::process::exit(((-2i32) as u32) as i32);
        }
    }

    let mut digest = [0u8; 32];
    tr::kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

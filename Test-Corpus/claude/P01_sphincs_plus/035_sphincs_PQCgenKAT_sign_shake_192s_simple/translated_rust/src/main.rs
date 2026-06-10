// PQCgenKAT_sign-style driver. Translated from c_src/app/src/PQCgenKAT_sign.c.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use sphincs_plus::context::SpxCtx;
use sphincs_plus::params::*;
use sphincs_plus::randombytes::{randombytes_init_rs, randombytes_rs};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// =========================
// Backend-specific KAT transcript context
// =========================

#[cfg(feature = "haraka")]
mod kat_tr {
    use super::*;
    use sphincs_plus::backend::haraka_backend::{
        haraka_S_inc_absorb, haraka_S_inc_finalize, haraka_S_inc_init, haraka_S_inc_squeeze,
        tweak_constants,
    };

    pub struct KatTrCtx {
        pub inner: SpxCtx,
        pub s: [u8; 65],
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut inner = SpxCtx::new();
        // pub_seed and sk_seed are zero (default)
        tweak_constants(&mut inner);
        let mut s = [0u8; 65];
        haraka_S_inc_init(&mut s);
        let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
        haraka_S_inc_absorb(&mut s, tag, &inner);
        haraka_S_inc_absorb(&mut s, &[0u8], &inner);
        KatTrCtx { inner, s }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        haraka_S_inc_absorb(&mut ctx.s, p, &ctx.inner);
        haraka_S_inc_absorb(&mut ctx.s, &[0u8], &ctx.inner);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xff) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        haraka_S_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
        haraka_S_inc_absorb(&mut ctx.s, &le, &ctx.inner);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        haraka_S_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
        if !buf.is_empty() {
            haraka_S_inc_absorb(&mut ctx.s, buf, &ctx.inner);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        haraka_S_inc_finalize(&mut ctx.s);
        haraka_S_inc_squeeze(out32, &mut ctx.s, &ctx.inner);
    }
}

#[cfg(feature = "shake")]
mod kat_tr {
    use super::*;
    use sphincs_plus::backend::shake_backend::{
        shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
    };

    pub struct KatTrCtx {
        pub s: [u64; 26],
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut s = [0u64; 26];
        shake256_inc_init(&mut s);
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        shake256_inc_absorb(&mut s, tag);
        shake256_inc_absorb(&mut s, &[0u8]);
        KatTrCtx { s }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        shake256_inc_absorb(&mut ctx.s, p);
        shake256_inc_absorb(&mut ctx.s, &[0u8]);
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xff) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        shake256_inc_absorb(&mut ctx.s, &lenle);
        shake256_inc_absorb(&mut ctx.s, &le);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        shake256_inc_absorb(&mut ctx.s, &lenle);
        if !buf.is_empty() {
            shake256_inc_absorb(&mut ctx.s, buf);
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(out32, &mut ctx.s);
    }
}

#[cfg(feature = "sha2")]
mod kat_tr {
    use super::*;
    use sphincs_plus::backend::sha2_backend::{
        sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512_inc_blocks,
        sha512_inc_finalize, sha512_inc_init,
    };

    // Two state sizes depending on N.
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const STATE_LEN: usize = 72;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const BLOCK_BYTES: usize = 128;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const OUTPUT_BYTES: usize = 64;

    #[cfg(any(feature = "128s", feature = "128f"))]
    const STATE_LEN: usize = 40;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const BLOCK_BYTES: usize = 64;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const OUTPUT_BYTES: usize = 32;

    pub struct KatTrCtx {
        pub s: Vec<u8>,
    }

    fn shaX_inc_init(s: &mut [u8]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let st: &mut [u8; 72] = (&mut s[..72]).try_into().unwrap();
            sha512_inc_init(st);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let st: &mut [u8; 40] = (&mut s[..40]).try_into().unwrap();
            sha256_inc_init(st);
        }
    }

    fn shaX_inc_blocks(s: &mut [u8], inp: &[u8], inblocks: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let st: &mut [u8; 72] = (&mut s[..72]).try_into().unwrap();
            sha512_inc_blocks(st, inp, inblocks);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let st: &mut [u8; 40] = (&mut s[..40]).try_into().unwrap();
            sha256_inc_blocks(st, inp, inblocks);
        }
    }

    fn shaX_inc_finalize(out: &mut [u8], s: &mut [u8], inp: &[u8], inlen: usize) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let st: &mut [u8; 72] = (&mut s[..72]).try_into().unwrap();
            sha512_inc_finalize(out, st, inp, inlen);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let st: &mut [u8; 40] = (&mut s[..40]).try_into().unwrap();
            sha256_inc_finalize(out, st, inp, inlen);
        }
    }

    pub fn kat_tr_init() -> KatTrCtx {
        let mut s = vec![0u8; STATE_LEN];
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = vec![0u8; BLOCK_BYTES];
        for i in 0..tag.len() {
            block[i] = tag[i];
        }
        shaX_inc_init(&mut s);
        shaX_inc_blocks(&mut s, &block, 1);
        KatTrCtx { s }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        let n = p.len();
        let block_count = (n + 1 + BLOCK_BYTES - 1) / BLOCK_BYTES;
        for i in 0..block_count {
            let mut block = vec![0u8; BLOCK_BYTES];
            let mut j: usize = 0;
            while i * BLOCK_BYTES + j < n && j < BLOCK_BYTES {
                block[j] = p[i * BLOCK_BYTES + j];
                j += 1;
            }
            if i * BLOCK_BYTES + j == n && j < BLOCK_BYTES {
                block[j] = 0x00;
                j += 1;
            }
            while j < BLOCK_BYTES {
                block[j] = 0;
                j += 1;
            }
            shaX_inc_blocks(&mut ctx.s, &block, 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut block = vec![0u8; BLOCK_BYTES];
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xff) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        for i in 0..8 {
            block[i] = lenle[i];
        }
        for i in 0..8 {
            block[8 + i] = le[i];
        }
        for i in 16..BLOCK_BYTES {
            block[i] = 0;
        }
        shaX_inc_blocks(&mut ctx.s, &block, 1);
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let mut lenle = vec![0u8; BLOCK_BYTES];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        let block_count = (buf.len() + BLOCK_BYTES - 1) / BLOCK_BYTES;
        shaX_inc_blocks(&mut ctx.s, &lenle, 1);
        if !buf.is_empty() {
            for i in 0..block_count {
                let mut block = vec![0u8; BLOCK_BYTES];
                let mut j: usize = 0;
                while i * BLOCK_BYTES + j < buf.len() && j < BLOCK_BYTES {
                    block[j] = buf[i * BLOCK_BYTES + j];
                    j += 1;
                }
                while j < BLOCK_BYTES {
                    block[j] = 0;
                    j += 1;
                }
                shaX_inc_blocks(&mut ctx.s, &block, 1);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        let mut outbuf = vec![0u8; OUTPUT_BYTES];
        let final_block = vec![0u8; BLOCK_BYTES];
        // Match C: shaX_inc_finalize(outbuf, ctx->s, final_block, 1)
        shaX_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

#[cfg(feature = "blake")]
mod kat_tr {
    use super::*;
    use sphincs_plus::backend::blake_backend::{
        blake256_final, blake256_init, blake256_update, blake512_final, blake512_init,
        blake512_update, BlakeState256, BlakeState512,
    };

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub struct KatTrCtx {
        pub s: BlakeState512,
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub struct KatTrCtx {
        pub s: BlakeState256,
    }

    pub fn kat_tr_init() -> KatTrCtx {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let mut s = BlakeState512 {
                h: [0; 8],
                s: [0; 4],
                t: [0; 2],
                buflen: 0,
                nullt: 0,
                buf: [0u8; 128],
            };
            blake512_init(&mut s);
            let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
            // Mirror the reference C: blake_update lengths are passed in bytes
            // (instead of bits) — we reproduce the quirk verbatim.
            blake512_update(&mut s, tag, tag.len() as u64);
            blake512_update(&mut s, &[0u8], 1);
            KatTrCtx { s }
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let mut s = BlakeState256 {
                h: [0; 8],
                s: [0; 4],
                t: [0; 2],
                buflen: 0,
                nullt: 0,
                buf: [0u8; 64],
            };
            blake256_init(&mut s);
            let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
            // Match C: pass byte count where bit count is expected.
            blake256_update(&mut s, tag, tag.len() as u64);
            blake256_update(&mut s, &[0u8], 1);
            KatTrCtx { s }
        }
    }

    pub fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
        let p = label.as_bytes();
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            blake512_update(&mut ctx.s, p, p.len() as u64);
            blake512_update(&mut ctx.s, &[0u8], 1);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            blake256_update(&mut ctx.s, p, p.len() as u64);
            blake256_update(&mut ctx.s, &[0u8], 1);
        }
    }

    pub fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xff) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            blake512_update(&mut ctx.s, &lenle, 8);
            blake512_update(&mut ctx.s, &le, 8);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            blake256_update(&mut ctx.s, &lenle, 8);
            blake256_update(&mut ctx.s, &le, 8);
        }
    }

    pub fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
        }
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            blake512_update(&mut ctx.s, &lenle, 8);
            if !buf.is_empty() {
                blake512_update(&mut ctx.s, buf, buf.len() as u64);
            }
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            blake256_update(&mut ctx.s, &lenle, 8);
            if !buf.is_empty() {
                blake256_update(&mut ctx.s, buf, buf.len() as u64);
            }
        }
    }

    pub fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let mut outbuf = [0u8; 64];
            blake512_final(&mut ctx.s, &mut outbuf);
            out32.copy_from_slice(&outbuf[..32]);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let mut outbuf = [0u8; 32];
            blake256_final(&mut ctx.s, &mut outbuf);
            out32.copy_from_slice(&outbuf);
        }
    }
}

use kat_tr::*;

fn main() {
    let main_buffer_size = BASE_MLEN * LOOP_COUNT;
    let mut m = vec![0u8; main_buffer_size];
    let mut sm = vec![0u8; main_buffer_size + CRYPTO_BYTES];
    let mut m1 = vec![0u8; main_buffer_size + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; main_buffer_size];

    for i in 0..48u8 {
        entropy_input[i as usize] = i;
    }
    randombytes_init_rs(&entropy_input, None);

    let mut tctx = kat_tr_init();
    kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes());
    kat_tr_absorb_label(&mut tctx, "SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes_rs(&mut seed);

        kat_tr_absorb_label(&mut tctx, "count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed");
        kat_tr_absorb_bytes(&mut tctx, &seed);

        let mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(KAT_OVERFLOW as i32 & 0xff);
        }

        kat_tr_absorb_label(&mut tctx, "mlen");
        kat_tr_absorb_u64(&mut tctx, mlen);

        randombytes_rs(&mut msg[..mlen as usize]);
        kat_tr_absorb_label(&mut tctx, "msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen as usize]);

        for b in m[..mlen as usize].iter_mut() {
            *b = 0;
        }
        for b in m1[..mlen as usize + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        for b in sm[..mlen as usize + CRYPTO_BYTES].iter_mut() {
            *b = 0;
        }
        m[..mlen as usize].copy_from_slice(&msg[..mlen as usize]);

        // crypto_sign_keypair
        let ret = unsafe {
            sphincs_plus::sign::SPX_crypto_sign_keypair(pk.as_mut_ptr(), sk.as_mut_ptr())
        };
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE as i32 & 0xff);
        }
        kat_tr_absorb_label(&mut tctx, "pk");
        kat_tr_absorb_bytes(&mut tctx, &pk);
        kat_tr_absorb_label(&mut tctx, "sk");
        kat_tr_absorb_bytes(&mut tctx, &sk);

        let mut smlen: u64 = 0;
        let ret = unsafe {
            sphincs_plus::sign::SPX_crypto_sign(
                sm.as_mut_ptr(),
                &mut smlen as *mut u64,
                m.as_ptr(),
                mlen,
                sk.as_ptr(),
            )
        };
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE as i32 & 0xff);
        }
        kat_tr_absorb_label(&mut tctx, "smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, "sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = unsafe {
            sphincs_plus::sign::SPX_crypto_sign_open(
                m1.as_mut_ptr(),
                &mut mlen1 as *mut u64,
                sm.as_ptr(),
                smlen,
                pk.as_ptr(),
            )
        };
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE as i32 & 0xff);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE as i32 & 0xff);
        }
        if &m[..mlen as usize] != &m1[..mlen as usize] {
            eprintln!("m mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE as i32 & 0xff);
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for b in digest.iter() {
        print!("{:02X}", b);
    }
    println!();

    std::process::exit(KAT_SUCCESS);
}

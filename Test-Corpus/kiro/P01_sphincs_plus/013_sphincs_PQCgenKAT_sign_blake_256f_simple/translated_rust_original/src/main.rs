#![allow(non_snake_case, unused_assignments)]

use sphincsplus::params::*;
use sphincsplus::sign::{crypto_sign_keypair, crypto_sign_fn, crypto_sign_open};
use sphincsplus::rng::{randombytes_init, randombytes};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

// ── BLAKE transcript ──────────────────────────────────────────────────────────
#[cfg(feature = "blake")]
mod tr {
    use sphincsplus::params::SPX_N;
    use sphincsplus::blake::blake256::{BlakeState256, blake256_init, blake256_update, blake256_final};
    use sphincsplus::blake::blake512::{BlakeState512, blake512_init, blake512_update, blake512_final};

    pub enum Ctx {
        B256(BlakeState256),
        B512(BlakeState512),
    }

    pub fn init() -> Ctx {
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        let sep = [0u8];
        if SPX_N >= 24 {
            let mut s = BlakeState512 {
                h: [0u64; 8], s: [0u64; 4], t: [0u64; 2],
                buflen: 0, nullt: 0, buf: [0u8; 128],
            };
            blake512_init(&mut s);
            blake512_update(&mut s, tag, (tag.len() as u64) * 8);
            blake512_update(&mut s, &sep, 8);
            Ctx::B512(s)
        } else {
            let mut s = BlakeState256 {
                h: [0u32; 8], s: [0u32; 4], t: [0u32; 2],
                buflen: 0, nullt: 0, buf: [0u8; 64],
            };
            blake256_init(&mut s);
            blake256_update(&mut s, tag, (tag.len() as u64) * 8);
            blake256_update(&mut s, &sep, 8);
            Ctx::B256(s)
        }
    }

    fn update(ctx: &mut Ctx, data: &[u8]) {
        let bits = (data.len() as u64) * 8;
        match ctx {
            Ctx::B256(s) => blake256_update(s, data, bits),
            Ctx::B512(s) => blake512_update(s, data, bits),
        }
    }

    pub fn absorb_label(ctx: &mut Ctx, label: &[u8]) {
        update(ctx, label);
        update(ctx, &[0u8]);
    }

    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        let lenle = 8u64.to_le_bytes();
        let le = x.to_le_bytes();
        update(ctx, &lenle);
        update(ctx, &le);
    }

    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        let lenle = (buf.len() as u64).to_le_bytes();
        update(ctx, &lenle);
        if !buf.is_empty() {
            update(ctx, buf);
        }
    }

    pub fn finalize(ctx: &mut Ctx, out32: &mut [u8; 32]) {
        match ctx {
            Ctx::B256(s) => {
                let mut outbuf = [0u8; 32];
                blake256_final(s, &mut outbuf);
                out32.copy_from_slice(&outbuf[..32]);
            }
            Ctx::B512(s) => {
                let mut outbuf = [0u8; 64];
                blake512_final(s, &mut outbuf);
                out32.copy_from_slice(&outbuf[..32]);
            }
        }
    }
}

// ── HARAKA transcript ─────────────────────────────────────────────────────────
#[cfg(feature = "haraka")]
mod tr {
    use sphincsplus::params::SPX_N;
    use sphincsplus::context::SpxCtx;
    use sphincsplus::haraka::haraka_impl::{
        haraka_s_inc_init, haraka_s_inc_absorb, haraka_s_inc_finalize, haraka_s_inc_squeeze,
        tweak_constants,
    };

    pub struct Ctx {
        inner: SpxCtx,
        s: [u8; 65],
    }

    pub fn init() -> Ctx {
        let mut inner = SpxCtx::new();
        for i in 0..SPX_N { inner.pub_seed[i] = 0; inner.sk_seed[i] = 0; }
        tweak_constants(&mut inner);
        let mut s = [0u8; 65];
        haraka_s_inc_init(&mut s);
        let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
        haraka_s_inc_absorb(&mut s, tag, &inner);
        haraka_s_inc_absorb(&mut s, &[0u8], &inner);
        Ctx { inner, s }
    }

    pub fn absorb_label(ctx: &mut Ctx, label: &[u8]) {
        haraka_s_inc_absorb(&mut ctx.s, label, &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &[0u8], &ctx.inner);
    }

    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        let lenle = 8u64.to_le_bytes();
        let le = x.to_le_bytes();
        haraka_s_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
        haraka_s_inc_absorb(&mut ctx.s, &le, &ctx.inner);
    }

    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        let lenle = (buf.len() as u64).to_le_bytes();
        haraka_s_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
        if !buf.is_empty() {
            haraka_s_inc_absorb(&mut ctx.s, buf, &ctx.inner);
        }
    }

    pub fn finalize(ctx: &mut Ctx, out32: &mut [u8; 32]) {
        haraka_s_inc_finalize(&mut ctx.s);
        haraka_s_inc_squeeze(out32, 32, &mut ctx.s, &ctx.inner);
    }
}

// ── SHA2 transcript ───────────────────────────────────────────────────────────
#[cfg(feature = "sha2")]
mod tr {
    use sphincsplus::params::SPX_N;
    use sphincsplus::sha2::sha2_impl::{
        sha256_inc_init, sha256_inc_blocks, sha256_inc_finalize,
        sha512_inc_init, sha512_inc_blocks, sha512_inc_finalize,
    };

    const BLOCK: usize = if SPX_N >= 24 { 128 } else { 64 };
    const STATE_LEN: usize = if SPX_N >= 24 { 72 } else { 40 };
    const OUTPUT: usize = if SPX_N >= 24 { 64 } else { 32 };

    pub struct Ctx { s: [u8; STATE_LEN] }

    fn inc_init(s: &mut [u8]) {
        if SPX_N >= 24 { sha512_inc_init(s); } else { sha256_inc_init(s); }
    }
    fn inc_blocks(s: &mut [u8], data: &[u8], n: usize) {
        if SPX_N >= 24 { sha512_inc_blocks(s, data, n); } else { sha256_inc_blocks(s, data, n); }
    }
    fn inc_finalize(out: &mut [u8], s: &mut [u8], data: &[u8], inlen: usize) {
        if SPX_N >= 24 { sha512_inc_finalize(out, s, data, inlen); } else { sha256_inc_finalize(out, s, data, inlen); }
    }

    pub fn init() -> Ctx {
        let tag = b"KAT-TRANSCRIPT-v1-SHA2";
        let mut block = [0u8; BLOCK];
        for i in 0..tag.len() { block[i] = tag[i]; }
        let mut ctx = Ctx { s: [0u8; STATE_LEN] };
        inc_init(&mut ctx.s);
        inc_blocks(&mut ctx.s, &block, 1);
        ctx
    }

    pub fn absorb_label(ctx: &mut Ctx, label: &[u8]) {
        let n = label.len();
        let block_count = (n + 1 + BLOCK - 1) / BLOCK;
        for i in 0..block_count {
            let mut block = [0u8; BLOCK];
            let mut j = 0usize;
            while i * BLOCK + j < n && j < BLOCK {
                block[j] = label[i * BLOCK + j];
                j += 1;
            }
            if i * BLOCK + j == n && j < BLOCK {
                block[j] = 0x00;
                j += 1;
            }
            // rest already zero
            inc_blocks(&mut ctx.s, &block, 1);
        }
    }

    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        let mut block = [0u8; BLOCK];
        let lenle = 8u64.to_le_bytes();
        let le = x.to_le_bytes();
        block[..8].copy_from_slice(&lenle);
        block[8..16].copy_from_slice(&le);
        inc_blocks(&mut ctx.s, &block, 1);
    }

    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        let len = buf.len();
        let mut lenblock = [0u8; BLOCK];
        let lenle = (len as u64).to_le_bytes();
        lenblock[..8].copy_from_slice(&lenle);
        inc_blocks(&mut ctx.s, &lenblock, 1);

        if len != 0 {
            let block_count = (len + BLOCK - 1) / BLOCK;
            for i in 0..block_count {
                let mut block = [0u8; BLOCK];
                let mut j = 0usize;
                while i * BLOCK + j < len && j < BLOCK {
                    block[j] = buf[i * BLOCK + j];
                    j += 1;
                }
                inc_blocks(&mut ctx.s, &block, 1);
            }
        }
    }

    pub fn finalize(ctx: &mut Ctx, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; OUTPUT];
        let final_block = [0u8; BLOCK];
        inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

// ── SHAKE transcript ──────────────────────────────────────────────────────────
#[cfg(feature = "shake")]
mod tr {
    use sphincsplus::shake::fips202::{
        shake256_inc_init, shake256_inc_absorb, shake256_inc_finalize, shake256_inc_squeeze,
    };

    pub struct Ctx { s: [u64; 26] }

    pub fn init() -> Ctx {
        let mut ctx = Ctx { s: [0u64; 26] };
        shake256_inc_init(&mut ctx.s);
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        shake256_inc_absorb(&mut ctx.s, tag);
        shake256_inc_absorb(&mut ctx.s, &[0u8]);
        ctx
    }

    pub fn absorb_label(ctx: &mut Ctx, label: &[u8]) {
        shake256_inc_absorb(&mut ctx.s, label);
        shake256_inc_absorb(&mut ctx.s, &[0u8]);
    }

    pub fn absorb_u64(ctx: &mut Ctx, x: u64) {
        let lenle = 8u64.to_le_bytes();
        let le = x.to_le_bytes();
        shake256_inc_absorb(&mut ctx.s, &lenle);
        shake256_inc_absorb(&mut ctx.s, &le);
    }

    pub fn absorb_bytes(ctx: &mut Ctx, buf: &[u8]) {
        let lenle = (buf.len() as u64).to_le_bytes();
        shake256_inc_absorb(&mut ctx.s, &lenle);
        if !buf.is_empty() {
            shake256_inc_absorb(&mut ctx.s, buf);
        }
    }

    pub fn finalize(ctx: &mut Ctx, out32: &mut [u8; 32]) {
        shake256_inc_finalize(&mut ctx.s);
        shake256_inc_squeeze(out32, 32, &mut ctx.s);
    }
}

fn main() {
    let mut entropy_input = [0u8; 48];
    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = tr::init();

    tr::absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    tr::absorb_bytes(&mut tctx, CRYPTO_ALGNAME);
    tr::absorb_label(&mut tctx, b"SKBYTES");
    tr::absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    tr::absorb_label(&mut tctx, b"PKBYTES");
    tr::absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    tr::absorb_label(&mut tctx, b"SIGBYTES");
    tr::absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..LOOP_COUNT {
        let mut seed = [0u8; 48];
        randombytes(&mut seed, 48);

        tr::absorb_label(&mut tctx, b"count");
        tr::absorb_u64(&mut tctx, i as u64);
        tr::absorb_label(&mut tctx, b"seed");
        tr::absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        tr::absorb_label(&mut tctx, b"mlen");
        tr::absorb_u64(&mut tctx, mlen as u64);

        randombytes(&mut msg[..mlen], mlen as u64);
        tr::absorb_label(&mut tctx, b"msg");
        tr::absorb_bytes(&mut tctx, &msg[..mlen]);

        let mut m = vec![0u8; mlen];
        m.copy_from_slice(&msg[..mlen]);
        let mut m1 = vec![0u8; mlen + CRYPTO_BYTES];
        let mut sm = vec![0u8; mlen + CRYPTO_BYTES];
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 { eprintln!("crypto_sign_keypair={}", ret); std::process::exit(-2); }
        tr::absorb_label(&mut tctx, b"pk");
        tr::absorb_bytes(&mut tctx, &pk);
        tr::absorb_label(&mut tctx, b"sk");
        tr::absorb_bytes(&mut tctx, &sk);

        let mut smlen: u64 = 0;
        let ret = crypto_sign_fn(&mut sm, &mut smlen, &m, mlen as u64, &sk);
        if ret != 0 { eprintln!("crypto_sign={}", ret); std::process::exit(-2); }
        tr::absorb_label(&mut tctx, b"smlen");
        tr::absorb_u64(&mut tctx, smlen);
        tr::absorb_label(&mut tctx, b"sm");
        tr::absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 { eprintln!("crypto_sign_open={}", ret); std::process::exit(-2); }
        if mlen1 != mlen as u64 { eprintln!("mlen mismatch"); std::process::exit(-2); }
        if m[..mlen] != m1[..mlen] { eprintln!("m mismatch"); std::process::exit(-2); }
    }

    let mut digest = [0u8; 32];
    tr::finalize(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();
}

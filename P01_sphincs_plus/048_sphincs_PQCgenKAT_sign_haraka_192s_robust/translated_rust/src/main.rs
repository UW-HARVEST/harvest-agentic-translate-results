#![allow(unused_imports, dead_code, static_mut_refs)]
mod params;
mod context;
mod address;
mod utils;
mod haraka;
mod wots;
mod wotsx1;
mod fors;
mod merkle;
mod rng;
mod sign;

use params::*;
use rng::{randombytes, randombytes_init};
use sign::*;
use haraka::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut entropy_input = [0u8; 48];
    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx::new();
    tctx.absorb_label("CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME);
    tctx.absorb_label("SKBYTES"); tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label("PKBYTES"); tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label("SIGBYTES"); tctx.absorb_u64(CRYPTO_BYTES as u64);

    let mut seed = [0u8; 48];
    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);
        tctx.absorb_label("count"); tctx.absorb_u64(i as u64);
        tctx.absorb_label("seed"); tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);
        tctx.absorb_label("mlen"); tctx.absorb_u64(mlen as u64);

        let mut msg = vec![0u8; mlen];
        randombytes(&mut msg, mlen);
        tctx.absorb_label("msg"); tctx.absorb_bytes(&msg);

        let mut m = vec![0u8; mlen];
        let mut m1 = vec![0u8; mlen + CRYPTO_BYTES];
        let mut sm = vec![0u8; mlen + CRYPTO_BYTES];
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        tctx.absorb_label("pk"); tctx.absorb_bytes(&pk);
        tctx.absorb_label("sk"); tctx.absorb_bytes(&sk);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m, mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        tctx.absorb_label("smlen"); tctx.absorb_u64(smlen);
        tctx.absorb_bytes(&sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(-2);
        }
        if mlen1 != mlen as u64 {
            eprintln!("mlen mismatch");
            std::process::exit(-2);
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            std::process::exit(-2);
        }
    }

    let mut digest = [0u8; 32];
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();
}

// KAT transcript context using Haraka sponge
struct KatTrCtx {
    inner: context::SpxCtx,
    s: [u8; 65],
}

impl KatTrCtx {
    fn new() -> Self {
        let mut ctx = context::SpxCtx::default();
        // zero pub_seed and sk_seed already default
        haraka::tweak_constants(&mut ctx);
        let mut s = [0u8; 65];
        haraka_s_inc_init(&mut s);
        let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
        haraka_s_inc_absorb(&mut s, tag, tag.len(), &ctx);
        let sep = [0u8; 1];
        haraka_s_inc_absorb(&mut s, &sep, 1, &ctx);
        KatTrCtx { inner: ctx, s }
    }

    fn absorb_label(&mut self, label: &str) {
        let p = label.as_bytes();
        haraka_s_inc_absorb(&mut self.s, p, p.len(), &self.inner);
        let sep = [0u8; 1];
        haraka_s_inc_absorb(&mut self.s, &sep, 1, &self.inner);
    }

    fn absorb_u64(&mut self, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
        haraka_s_inc_absorb(&mut self.s, &lenle, 8, &self.inner);
        haraka_s_inc_absorb(&mut self.s, &le, 8, &self.inner);
    }

    fn absorb_bytes(&mut self, buf: &[u8]) {
        let len = buf.len();
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
        haraka_s_inc_absorb(&mut self.s, &lenle, 8, &self.inner);
        if len > 0 {
            haraka_s_inc_absorb(&mut self.s, buf, len, &self.inner);
        }
    }

    fn finalize(&mut self, out32: &mut [u8; 32]) {
        haraka_s_inc_finalize(&mut self.s);
        haraka_s_inc_squeeze(out32, 32, &mut self.s, &self.inner);
    }
}

mod params;
mod context;
mod address;
mod rng;
mod haraka;
mod hash;
mod thash;
mod wots;
mod fors;
mod utils;
mod utilsx1;
mod merkle;
mod sign;

use params::*;
use context::SpxCtx;
use rng::{randombytes_init, randombytes};
use haraka::*;
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    inner: SpxCtx,
    s: [u8; 65],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    ctx.inner.pub_seed = [0u8; SPX_N];
    ctx.inner.sk_seed = [0u8; SPX_N];
    haraka::tweak_constants(&mut ctx.inner);
    haraka_s_inc_init(&mut ctx.s);
    let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
    haraka_s_inc_absorb(&mut ctx.s, tag, &ctx.inner);
    let sep = [0u8; 1];
    haraka_s_inc_absorb(&mut ctx.s, &sep, &ctx.inner);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    haraka_s_inc_absorb(&mut ctx.s, label, &ctx.inner);
    let sep = [0u8; 1];
    haraka_s_inc_absorb(&mut ctx.s, &sep, &ctx.inner);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    haraka_s_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
    haraka_s_inc_absorb(&mut ctx.s, &le, &ctx.inner);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
    let len = buf.len() as u64;
    let mut lenle = [0u8; 8];
    for i in 0..8 { lenle[i] = ((len >> (8 * i)) & 0xFF) as u8; }
    haraka_s_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
    if !buf.is_empty() {
        haraka_s_inc_absorb(&mut ctx.s, buf, &ctx.inner);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    haraka_s_inc_finalize(&mut ctx.s);
    haraka_s_inc_squeeze(out32, 32, &mut ctx.s, &ctx.inner);
}

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { inner: SpxCtx::default(), s: [0u8; 65] };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME);
    kat_tr_absorb_label(&mut tctx, b"SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);
        kat_tr_absorb_label(&mut tctx, b"count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, b"seed");
        kat_tr_absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }
        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        randombytes(&mut msg[..mlen], mlen);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen]);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk);

        let smlen = crypto_sign(&mut sm, &m[..mlen], &sk);
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen as u64);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen]);

        let (ret, mlen1) = crypto_sign_open_with_pk(&mut m1, &sm[..smlen], &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(-2);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(-2);
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            std::process::exit(-2);
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);
    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();
}

mod address;
mod context;
mod fors;
mod haraka;
mod hash;
mod merkle;
mod params;
mod rng;
mod sign;
mod thash;
mod utils;
mod utilsx1;
mod wots;

use context::SpxCtx;
use haraka::*;
use params::*;
use rng::{randombytes, randombytes_init};
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    inner: SpxCtx,
    s: [u8; 65],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    for i in 0..SPX_N {
        ctx.inner.pub_seed[i] = 0;
        ctx.inner.sk_seed[i] = 0;
    }
    tweak_constants(&mut ctx.inner);
    haraka_s_inc_init(&mut ctx.s);

    let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
    haraka_s_inc_absorb(&mut ctx.s, tag, &ctx.inner);

    let sep = [0u8; 1];
    haraka_s_inc_absorb(&mut ctx.s, &sep, &ctx.inner);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
    let p = label.as_bytes();
    haraka_s_inc_absorb(&mut ctx.s, p, &ctx.inner);
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
    let len = buf.len();
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    haraka_s_inc_absorb(&mut ctx.s, &lenle, &ctx.inner);
    if len > 0 {
        haraka_s_inc_absorb(&mut ctx.s, buf, &ctx.inner);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    haraka_s_inc_finalize(&mut ctx.s);
    haraka_s_inc_squeeze(out32, &mut ctx.s, &ctx.inner);
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

    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx {
        inner: SpxCtx::default(),
        s: [0u8; 65],
    };
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
        randombytes(&mut seed);

        kat_tr_absorb_label(&mut tctx, "count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed");
        kat_tr_absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, "mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        randombytes(&mut msg[..mlen]);
        kat_tr_absorb_label(&mut tctx, "msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen]);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, "pk");
        kat_tr_absorb_bytes(&mut tctx, &pk[..CRYPTO_PUBLICKEYBYTES]);
        kat_tr_absorb_label(&mut tctx, "sk");
        kat_tr_absorb_bytes(&mut tctx, &sk[..CRYPTO_SECRETKEYBYTES]);

        // Sign
        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..mlen], &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, "smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, "sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        // Verify
        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], &pk);
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
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 { print!("{:02X}", digest[i]); }
    println!();
}

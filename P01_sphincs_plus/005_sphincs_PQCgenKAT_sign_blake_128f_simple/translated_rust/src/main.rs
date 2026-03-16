#![allow(clippy::needless_range_loop)]
mod params;
mod blake256;
mod utils;
mod hash;
mod rng;
mod wots;
mod fors;
mod sign;

use params::*;
use blake256::*;
use rng::*;
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    state: Blake256State,
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    blake256_init(&mut ctx.state);
    let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
    blake256_update(&mut ctx.state, tag, (tag.len() as u64) * 8);
    blake256_update(&mut ctx.state, &[0x00u8], 8);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    blake256_update(&mut ctx.state, label, (label.len() as u64) * 8);
    blake256_update(&mut ctx.state, &[0x00u8], 8);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    blake256_update(&mut ctx.state, &lenle, 64);
    blake256_update(&mut ctx.state, &le, 64);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    blake256_update(&mut ctx.state, &lenle, 64);
    if len > 0 {
        blake256_update(&mut ctx.state, &buf[..len], (len as u64) * 8);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; 32];
    blake256_final(&mut ctx.state, &mut outbuf);
    out32.copy_from_slice(&outbuf);
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

    let mut tctx = KatTrCtx {
        state: Blake256State { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;64] },
    };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
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
        kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        randombytes(&mut msg[..mlen], mlen as u64);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen);

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
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let (ret, smlen) = crypto_sign(&mut sm, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let (ret, mlen1) = crypto_sign_open(&mut m1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(-2);
        }
        if mlen1 as usize != mlen {
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

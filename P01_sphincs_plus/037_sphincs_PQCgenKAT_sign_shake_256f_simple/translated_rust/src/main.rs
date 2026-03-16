#![allow(static_mut_refs)]

mod params;
mod fips202;
mod rng;
mod context;
mod hash;
mod wots;
mod treehash;
mod fors;
mod merkle;
mod sign;

use params::*;
use rng::{randombytes_init, randombytes};
use fips202::{shake256_inc_init, shake256_inc_absorb, shake256_inc_finalize, shake256_inc_squeeze};
use sign::{crypto_sign_keypair, crypto_sign, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    s: [u64; 26],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    shake256_inc_init(&mut ctx.s);
    let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
    shake256_inc_absorb(&mut ctx.s, tag);
    shake256_inc_absorb(&mut ctx.s, &[0x00]);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    shake256_inc_absorb(&mut ctx.s, label);
    shake256_inc_absorb(&mut ctx.s, &[0x00]);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 {
        le[i] = ((x >> (8 * i)) & 0xFF) as u8;
    }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    shake256_inc_absorb(&mut ctx.s, &lenle);
    shake256_inc_absorb(&mut ctx.s, &le);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8]) {
    let len = buf.len();
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    shake256_inc_absorb(&mut ctx.s, &lenle);
    if len > 0 {
        shake256_inc_absorb(&mut ctx.s, buf);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    shake256_inc_finalize(&mut ctx.s);
    shake256_inc_squeeze(out32, 32, &mut ctx.s);
}

fn main() {
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { s: [0u64; 26] };
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

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        let mut msg = vec![0u8; mlen];
        randombytes(&mut msg, mlen);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg);

        let mut m = vec![0u8; mlen];
        m.copy_from_slice(&msg);
        let mut sm = vec![0u8; mlen + CRYPTO_BYTES];
        let mut m1 = vec![0u8; mlen + CRYPTO_BYTES];

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk);

        let (ret, smlen) = crypto_sign(&mut sm, &m, mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen as u64);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen]);

        let (ret, mlen1) = crypto_sign_open(&mut m1, &sm, smlen, &pk);
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
    for b in &digest {
        print!("{:02X}", b);
    }
    println!();
}

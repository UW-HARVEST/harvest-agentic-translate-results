#![allow(static_mut_refs)]

mod params;
mod sha2;
mod rng;
mod address;
mod utils;
mod context;
mod hash_sha2;
mod thash;
mod wots;
mod fors;
mod merkle;
mod utilsx1;
mod wotsx1;
mod sign;
mod kat_transcript;

use params::*;
use rng::{randombytes_init, randombytes};
use sign::{crypto_sign_keypair, crypto_sign, crypto_sign_open};
use kat_transcript::KatTrCtx;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_CRYPTO_FAILURE: i32 = -2;

const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut pk = [0u8; SPX_PK_BYTES];
    let mut sk = [0u8; SPX_SK_BYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx::new();
    tctx.init();
    tctx.absorb_label(b"CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME);
    tctx.absorb_label(b"SKBYTES");
    tctx.absorb_u64(SPX_SK_BYTES as u64);
    tctx.absorb_label(b"PKBYTES");
    tctx.absorb_u64(SPX_PK_BYTES as u64);
    tctx.absorb_label(b"SIGBYTES");
    tctx.absorb_u64(SPX_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed);

        tctx.absorb_label(b"count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label(b"seed");
        tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);

        tctx.absorb_label(b"mlen");
        tctx.absorb_u64(mlen as u64);

        randombytes(&mut msg[..mlen]);
        tctx.absorb_label(b"msg");
        tctx.absorb_bytes(&msg[..mlen]);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..(mlen + SPX_BYTES) { m1[j] = 0; }
        for j in 0..(mlen + SPX_BYTES) { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        tctx.absorb_label(b"pk");
        tctx.absorb_bytes(&pk);
        tctx.absorb_label(b"sk");
        tctx.absorb_bytes(&sk);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..mlen], &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        tctx.absorb_label(b"smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label(b"sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            return KAT_CRYPTO_FAILURE;
        }
        if mlen1 as usize != mlen {
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
    for b in &digest {
        print!("{:02X}", b);
    }
    println!();

    KAT_SUCCESS
}

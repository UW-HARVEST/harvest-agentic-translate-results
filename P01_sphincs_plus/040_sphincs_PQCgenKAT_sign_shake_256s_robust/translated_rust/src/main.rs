#![allow(unused_imports, dead_code, static_mut_refs)]
mod params;
mod fips202;
mod rng;
mod address;
mod hash;
mod wots;
mod fors;
mod merkle;
mod sign;

use params::*;
use fips202::*;
use rng::*;
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    // Initialize transcript
    let mut tctx = [0u64; 26];
    shake256_inc_init(&mut tctx);

    let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
    shake256_inc_absorb(&mut tctx, tag);
    shake256_inc_absorb(&mut tctx, &[0x00]);

    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, b"SPHINCS+");
    kat_tr_absorb_label(&mut tctx, b"SKBYTES");
    kat_tr_absorb_u64(&mut tctx, SPX_SK_BYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES");
    kat_tr_absorb_u64(&mut tctx, SPX_PK_BYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, SPX_BYTES as u64);

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

        randombytes(&mut msg, mlen as usize);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen]);

        m[..mlen].fill(0);
        m1[..mlen + SPX_BYTES].fill(0);
        sm[..mlen + SPX_BYTES].fill(0);
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk);

        // Sign
        let smlen = crypto_sign(&mut sm, &m[..mlen], &sk);
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen as u64);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen]);

        // Verify
        match crypto_sign_open_with_pk(&mut m1, &sm[..smlen], &pk) {
            Ok(mlen1) => {
                if mlen1 != mlen {
                    eprintln!("mlen mismatch");
                    std::process::exit(-2);
                }
                if m[..mlen] != m1[..mlen] {
                    eprintln!("m mismatch");
                    std::process::exit(-2);
                }
            }
            Err(_) => {
                eprintln!("crypto_sign_open=-1");
                std::process::exit(-2);
            }
        }
    }

    // Finalize transcript digest
    shake256_inc_finalize(&mut tctx);
    let mut digest = [0u8; 32];
    shake256_inc_squeeze(&mut digest, 32, &mut tctx);

    print!("KAT transcript digest = ");
    for b in &digest {
        print!("{:02X}", b);
    }
    println!();
}

fn kat_tr_absorb_label(ctx: &mut [u64; 26], label: &[u8]) {
    shake256_inc_absorb(ctx, label);
    shake256_inc_absorb(ctx, &[0x00]);
}

fn kat_tr_absorb_u64(ctx: &mut [u64; 26], x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 {
        le[i] = ((x >> (8 * i)) & 0xFF) as u8;
    }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    shake256_inc_absorb(ctx, &lenle);
    shake256_inc_absorb(ctx, &le);
}

fn kat_tr_absorb_bytes(ctx: &mut [u64; 26], buf: &[u8]) {
    let mut lenle = [0u8; 8];
    let l = buf.len() as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    shake256_inc_absorb(ctx, &lenle);
    if !buf.is_empty() {
        shake256_inc_absorb(ctx, buf);
    }
}

#![allow(static_mut_refs, unused_assignments, unused_variables)]
mod address;
mod blake;
mod context;
mod fors;
mod hash;
mod params;
mod rng;
mod sign;
mod wots;

use blake::BlakeState512;
use params::*;
use rng::{randombytes, randombytes_init};
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

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

    // Initialize transcript (BLAKE_TR with blake512 since SPX_N >= 24)
    let mut tctx = BlakeState512::new();

    // kat_tr_init
    {
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        tctx.update(tag, (tag.len() as u64) * 8);
        tctx.update(&[0x00], 8);
    }

    // Helper closures via functions
    fn tr_absorb_label(tctx: &mut BlakeState512, label: &[u8]) {
        tctx.update(label, (label.len() as u64) * 8);
        tctx.update(&[0x00], 8);
    }

    fn tr_absorb_u64(tctx: &mut BlakeState512, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        tctx.update(&lenle, 64);
        tctx.update(&le, 64);
    }

    fn tr_absorb_bytes(tctx: &mut BlakeState512, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        tctx.update(&lenle, 64);
        if len > 0 {
            tctx.update(&buf[..len], (len as u64) * 8);
        }
    }

    tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
    tr_absorb_label(&mut tctx, b"SKBYTES");
    tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    tr_absorb_label(&mut tctx, b"PKBYTES");
    tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    tr_absorb_label(&mut tctx, b"SIGBYTES");
    tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);

        tr_absorb_label(&mut tctx, b"count");
        tr_absorb_u64(&mut tctx, i as u64);
        tr_absorb_label(&mut tctx, b"seed");
        tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        tr_absorb_label(&mut tctx, b"mlen");
        tr_absorb_u64(&mut tctx, mlen);

        randombytes(&mut msg, mlen);
        tr_absorb_label(&mut tctx, b"msg");
        tr_absorb_bytes(&mut tctx, &msg, mlen as usize);

        let ml = mlen as usize;
        for j in 0..ml { m[j] = 0; }
        for j in 0..ml + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..ml + CRYPTO_BYTES { sm[j] = 0; }
        m[..ml].copy_from_slice(&msg[..ml]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        tr_absorb_label(&mut tctx, b"pk");
        tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        tr_absorb_label(&mut tctx, b"sk");
        tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m, mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        tr_absorb_label(&mut tctx, b"smlen");
        tr_absorb_u64(&mut tctx, smlen);
        tr_absorb_label(&mut tctx, b"sm");
        tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(-2);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(-2);
        }
        if m[..ml] != m1[..ml] {
            eprintln!("m mismatch");
            std::process::exit(-2);
        }
    }

    // Finalize transcript digest
    let mut outbuf = [0u8; 64];
    tctx.finalize(&mut outbuf);
    let digest = &outbuf[..32];

    print!("KAT transcript digest = ");
    for b in digest {
        print!("{:02X}", b);
    }
    println!();
}

mod params;
mod blake256;
mod blake512;
mod utils;
mod address;
mod hash;
mod thash;
mod rng;
mod fors;
mod wots;
mod treehash;
mod merkle;
mod sign;
mod compute_root;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut entropy_input = [0u8; 48];
    for i in 0..48 { entropy_input[i] = i as u8; }
    rng::randombytes_init(&entropy_input, None);

    // KAT transcript using blake512
    let mut tctx = blake512::BlakeState512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512::blake512_init(&mut tctx);

    // kat_tr_init
    {
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        blake512::blake512_update(&mut tctx, tag, (tag.len() as u64) * 8);
        blake512::blake512_update(&mut tctx, &[0x00], 8);
    }

    // Helper closures
    fn tr_absorb_label(tctx: &mut blake512::BlakeState512, label: &[u8]) {
        blake512::blake512_update(tctx, label, (label.len() as u64) * 8);
        blake512::blake512_update(tctx, &[0x00], 8);
    }

    fn tr_absorb_u64(tctx: &mut blake512::BlakeState512, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
        blake512::blake512_update(tctx, &lenle, 64);
        blake512::blake512_update(tctx, &le, 64);
    }

    fn tr_absorb_bytes(tctx: &mut blake512::BlakeState512, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
        blake512::blake512_update(tctx, &lenle, 64);
        if len > 0 {
            blake512::blake512_update(tctx, &buf[..len], (len as u64) * 8);
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

    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed, 48);

        tr_absorb_label(&mut tctx, b"count");
        tr_absorb_u64(&mut tctx, i as u64);
        tr_absorb_label(&mut tctx, b"seed");
        tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen = BASE_MLEN * (i + 1);

        tr_absorb_label(&mut tctx, b"mlen");
        tr_absorb_u64(&mut tctx, mlen as u64);

        rng::randombytes(&mut msg[..mlen], mlen);
        tr_absorb_label(&mut tctx, b"msg");
        tr_absorb_bytes(&mut tctx, &msg, mlen);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { m1[j] = 0; sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        sign::crypto_sign_keypair(&mut pk, &mut sk);

        tr_absorb_label(&mut tctx, b"pk");
        tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        tr_absorb_label(&mut tctx, b"sk");
        tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let smlen = sign::crypto_sign(&mut sm, &m[..mlen], mlen, &sk);

        tr_absorb_label(&mut tctx, b"smlen");
        tr_absorb_u64(&mut tctx, smlen as u64);
        tr_absorb_label(&mut tctx, b"sm");
        tr_absorb_bytes(&mut tctx, &sm, smlen);

        let (ret, mlen1) = sign::crypto_sign_open(&mut m1, &sm[..smlen], smlen, &pk);
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

    // Finalize
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    blake512::blake512_final(&mut tctx, &mut outbuf);
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&outbuf[..32]);

    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();
}

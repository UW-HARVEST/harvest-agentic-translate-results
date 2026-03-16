#![allow(unused)]
mod params;
mod fips202;
mod rng;
mod address;
mod hash;
mod utils;
mod wots;
mod wotsx1;
mod utilsx1;
mod fors;
mod merkle;
mod sign;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    s: [u64; 26],
}

impl KatTrCtx {
    fn new() -> Self {
        let mut ctx = KatTrCtx { s: [0u64; 26] };
        fips202::Shake256Inc::init_raw(&mut ctx.s);
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        fips202::Shake256Inc::absorb_raw(&mut ctx.s, tag);
        let sep = [0u8; 1];
        fips202::Shake256Inc::absorb_raw(&mut ctx.s, &sep);
        ctx
    }

    fn absorb_label(&mut self, label: &[u8]) {
        fips202::Shake256Inc::absorb_raw(&mut self.s, label);
        let sep = [0u8; 1];
        fips202::Shake256Inc::absorb_raw(&mut self.s, &sep);
    }

    fn absorb_u64(&mut self, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        fips202::Shake256Inc::absorb_raw(&mut self.s, &lenle);
        fips202::Shake256Inc::absorb_raw(&mut self.s, &le);
    }

    fn absorb_bytes(&mut self, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        fips202::Shake256Inc::absorb_raw(&mut self.s, &lenle);
        if !buf.is_empty() {
            fips202::Shake256Inc::absorb_raw(&mut self.s, buf);
        }
    }

    fn finalize(&mut self, out: &mut [u8; 32]) {
        fips202::Shake256Inc::finalize_raw(&mut self.s);
        fips202::Shake256Inc::squeeze_raw(out, 32, &mut self.s);
    }
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

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    rng::randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx::new();
    tctx.absorb_label(b"CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME);
    tctx.absorb_label(b"SKBYTES"); tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label(b"PKBYTES"); tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label(b"SIGBYTES"); tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed, 48);

        tctx.absorb_label(b"count"); tctx.absorb_u64(i as u64);
        tctx.absorb_label(b"seed"); tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);

        tctx.absorb_label(b"mlen"); tctx.absorb_u64(mlen as u64);

        rng::randombytes(&mut msg[..mlen], mlen);
        tctx.absorb_label(b"msg"); tctx.absorb_bytes(&msg[..mlen]);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        tctx.absorb_label(b"pk"); tctx.absorb_bytes(&pk[..CRYPTO_PUBLICKEYBYTES]);
        tctx.absorb_label(b"sk"); tctx.absorb_bytes(&sk[..CRYPTO_SECRETKEYBYTES]);

        // Sign
        let smlen = sign::crypto_sign(&mut sm, &m[..mlen], &sk);
        tctx.absorb_label(b"smlen"); tctx.absorb_u64(smlen as u64);
        tctx.absorb_label(b"sm"); tctx.absorb_bytes(&sm[..smlen]);

        // Verify
        let (ret, mlen1) = sign::crypto_sign_open(&mut m1, &sm[..smlen], &pk);
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
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

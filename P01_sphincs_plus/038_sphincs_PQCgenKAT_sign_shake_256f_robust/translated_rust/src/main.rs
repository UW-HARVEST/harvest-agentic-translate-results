mod params;
mod fips202;
mod rng;
mod address;
mod hash;
mod wots;
mod fors;
mod sign;

use params::*;
use fips202::Shake256Inc;
use rng::{randombytes_init, randombytes};
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    randombytes_init(&entropy_input, None);

    // Initialize transcript
    let mut tctx = Shake256Inc::new();
    // init
    {
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        tctx.absorb(tag);
        tctx.absorb(&[0x00]);
    }

    // absorb_label helper
    fn absorb_label(ctx: &mut Shake256Inc, label: &[u8]) {
        ctx.absorb(label);
        ctx.absorb(&[0x00]);
    }

    fn absorb_u64(ctx: &mut Shake256Inc, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        ctx.absorb(&lenle);
        ctx.absorb(&le);
    }

    fn absorb_bytes(ctx: &mut Shake256Inc, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        ctx.absorb(&lenle);
        if !buf.is_empty() {
            ctx.absorb(buf);
        }
    }

    absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    absorb_bytes(&mut tctx, CRYPTO_ALGNAME);
    absorb_label(&mut tctx, b"SKBYTES");
    absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    absorb_label(&mut tctx, b"PKBYTES");
    absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    absorb_label(&mut tctx, b"SIGBYTES");
    absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed);

        absorb_label(&mut tctx, b"count");
        absorb_u64(&mut tctx, i as u64);
        absorb_label(&mut tctx, b"seed");
        absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        absorb_label(&mut tctx, b"mlen");
        absorb_u64(&mut tctx, mlen as u64);

        randombytes(&mut msg[..mlen]);
        absorb_label(&mut tctx, b"msg");
        absorb_bytes(&mut tctx, &msg[..mlen]);

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
        absorb_label(&mut tctx, b"pk");
        absorb_bytes(&mut tctx, &pk);
        absorb_label(&mut tctx, b"sk");
        absorb_bytes(&mut tctx, &sk);

        // Sign
        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..mlen], &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        absorb_label(&mut tctx, b"smlen");
        absorb_u64(&mut tctx, smlen);
        absorb_label(&mut tctx, b"sm");
        absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        // Verify
        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], smlen, &pk);
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

    // Finalize transcript digest
    tctx.finalize();
    let mut digest = [0u8; 32];
    tctx.squeeze(&mut digest);

    print!("KAT transcript digest = ");
    for b in &digest {
        print!("{:02X}", b);
    }
    println!();
}

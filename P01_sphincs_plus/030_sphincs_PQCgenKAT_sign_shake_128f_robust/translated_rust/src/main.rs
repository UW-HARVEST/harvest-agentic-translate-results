mod params;
mod fips202;
mod rng;
mod address;
mod hash;
mod utils;
mod wots;
mod fors;
mod merkle;
mod sign;

use params::*;
use fips202::Shake256Inc;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
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
    rng::randombytes_init(&entropy_input, None);

    // Initialize Transcript
    let mut tctx = Shake256Inc::new();
    // init
    {
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        tctx.absorb(tag);
        tctx.absorb(&[0x00]);
    }

    // Helper closures matching the C kat_tr_* functions
    fn tr_absorb_label(tctx: &mut Shake256Inc, label: &[u8]) {
        tctx.absorb(label);
        tctx.absorb(&[0x00]);
    }

    fn tr_absorb_u64(tctx: &mut Shake256Inc, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        tctx.absorb(&lenle);
        tctx.absorb(&le);
    }

    fn tr_absorb_bytes(tctx: &mut Shake256Inc, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        tctx.absorb(&lenle);
        if !buf.is_empty() {
            tctx.absorb(buf);
        }
    }

    tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME);
    tr_absorb_label(&mut tctx, b"SKBYTES");
    tr_absorb_u64(&mut tctx, SPX_SK_BYTES as u64);
    tr_absorb_label(&mut tctx, b"PKBYTES");
    tr_absorb_u64(&mut tctx, SPX_PK_BYTES as u64);
    tr_absorb_label(&mut tctx, b"SIGBYTES");
    tr_absorb_u64(&mut tctx, SPX_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed);

        tr_absorb_label(&mut tctx, b"count");
        tr_absorb_u64(&mut tctx, i as u64);
        tr_absorb_label(&mut tctx, b"seed");
        tr_absorb_bytes(&mut tctx, &seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        tr_absorb_label(&mut tctx, b"mlen");
        tr_absorb_u64(&mut tctx, mlen as u64);

        rng::randombytes(&mut msg[..mlen]);
        tr_absorb_label(&mut tctx, b"msg");
        tr_absorb_bytes(&mut tctx, &msg[..mlen]);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..(mlen + SPX_BYTES) { m1[j] = 0; }
        for j in 0..(mlen + SPX_BYTES) { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        // Keypair
        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        tr_absorb_label(&mut tctx, b"pk");
        tr_absorb_bytes(&mut tctx, &pk);
        tr_absorb_label(&mut tctx, b"sk");
        tr_absorb_bytes(&mut tctx, &sk);

        // Sign
        let siglen = sign::crypto_sign_signature(&mut sm, &m[..mlen], &sk);
        // memmove message after signature
        for j in (0..mlen).rev() {
            sm[SPX_BYTES + j] = m[j];
        }
        let smlen = (siglen + mlen) as u64;

        tr_absorb_label(&mut tctx, b"smlen");
        tr_absorb_u64(&mut tctx, smlen);
        tr_absorb_label(&mut tctx, b"sm");
        tr_absorb_bytes(&mut tctx, &sm[..smlen as usize]);

        // Verify
        let mut mlen1 = 0u64;
        let ret = sign::crypto_sign_open_full(&mut m1, &mut mlen1, &sm, smlen, &pk);
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

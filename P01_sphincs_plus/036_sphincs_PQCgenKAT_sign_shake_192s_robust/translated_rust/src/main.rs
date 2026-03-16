mod address;
mod context;
mod fips202;
mod fors;
mod hash;
mod merkle;
mod params;
mod rng;
mod sign;
mod thash;
mod utils;
mod utilsx1;
mod wots;
mod wotsx1;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    s: [u64; 26],
}

impl KatTrCtx {
    fn init() -> Self {
        let mut ctx = KatTrCtx { s: [0u64; 26] };
        fips202::shake256_inc_init(&mut ctx.s);
        let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
        fips202::shake256_inc_absorb(&mut ctx.s, tag);
        fips202::shake256_inc_absorb(&mut ctx.s, &[0x00]);
        ctx
    }

    fn absorb_label(&mut self, label: &[u8]) {
        fips202::shake256_inc_absorb(&mut self.s, label);
        fips202::shake256_inc_absorb(&mut self.s, &[0x00]);
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
        fips202::shake256_inc_absorb(&mut self.s, &lenle);
        fips202::shake256_inc_absorb(&mut self.s, &le);
    }

    fn absorb_bytes(&mut self, buf: &[u8]) {
        let mut lenle = [0u8; 8];
        let l = buf.len() as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        fips202::shake256_inc_absorb(&mut self.s, &lenle);
        if !buf.is_empty() {
            fips202::shake256_inc_absorb(&mut self.s, buf);
        }
    }

    fn finalize(&mut self, out32: &mut [u8; 32]) {
        fips202::shake256_inc_finalize(&mut self.s);
        fips202::shake256_inc_squeeze(out32, &mut self.s);
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

    let mut tctx = KatTrCtx::init();
    tctx.absorb_label(b"CRYPTO_ALGNAME");
    tctx.absorb_bytes(CRYPTO_ALGNAME);
    tctx.absorb_label(b"SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label(b"PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label(b"SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed);

        tctx.absorb_label(b"count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label(b"seed");
        tctx.absorb_bytes(&seed);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        tctx.absorb_label(b"mlen");
        tctx.absorb_u64(mlen as u64);

        rng::randombytes(&mut msg[..mlen]);
        tctx.absorb_label(b"msg");
        tctx.absorb_bytes(&msg[..mlen]);

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
        tctx.absorb_label(b"pk");
        tctx.absorb_bytes(&pk);
        tctx.absorb_label(b"sk");
        tctx.absorb_bytes(&sk);

        // Sign
        let (ret, smlen) = sign::crypto_sign(&mut sm, &m[..mlen], &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        tctx.absorb_label(b"smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label(b"sm");
        tctx.absorb_bytes(&sm[..smlen as usize]);

        // Verify
        let (ret, mlen1) = sign::crypto_sign_open(&mut m1, &sm[..smlen as usize], &pk);
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
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

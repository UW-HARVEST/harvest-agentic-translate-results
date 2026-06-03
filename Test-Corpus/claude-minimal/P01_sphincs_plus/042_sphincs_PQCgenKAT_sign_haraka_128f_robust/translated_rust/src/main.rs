//! KAT driver for the Rust SPHINCS+ port.
//! Translated from PQCgenKAT_sign.c (SHAKE branch).

use sphincsplus::fips202::{
    shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
};
use sphincsplus::params::{
    CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES,
};
use sphincsplus::rng::{randombytes, randombytes_init, Aes256CtrDrbgState};
use sphincsplus::sign::{crypto_sign, crypto_sign_keypair, crypto_sign_open};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

struct KatTrCtx {
    s: [u64; 26],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    shake256_inc_init(&mut ctx.s);
    let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
    shake256_inc_absorb(&mut ctx.s, tag, tag.len());
    let sep = [0u8; 1];
    shake256_inc_absorb(&mut ctx.s, &sep, 1);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
    let bytes = label.as_bytes();
    shake256_inc_absorb(&mut ctx.s, bytes, bytes.len());
    let sep = [0u8; 1];
    shake256_inc_absorb(&mut ctx.s, &sep, 1);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 {
        le[i] = ((x >> (8 * i)) & 0xff) as u8;
    }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
    }
    shake256_inc_absorb(&mut ctx.s, &lenle, 8);
    shake256_inc_absorb(&mut ctx.s, &le, 8);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l: u64 = len as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xff) as u8;
    }
    shake256_inc_absorb(&mut ctx.s, &lenle, 8);
    if len > 0 {
        shake256_inc_absorb(&mut ctx.s, buf, len);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    shake256_inc_finalize(&mut ctx.s);
    shake256_inc_squeeze(out32, 32, &mut ctx.s);
}

fn main() -> std::process::ExitCode {
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
    let mut drbg = Aes256CtrDrbgState::new();
    randombytes_init(&mut drbg, &entropy_input, None);

    let mut tctx = KatTrCtx { s: [0u64; 26] };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes(), CRYPTO_ALGNAME.len());
    kat_tr_absorb_label(&mut tctx, "SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, "SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut drbg, &mut seed);

        kat_tr_absorb_label(&mut tctx, "count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed");
        kat_tr_absorb_bytes(&mut tctx, &seed, seed.len());

        let mlen = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            return std::process::ExitCode::from(KAT_OVERFLOW as u8);
        }

        kat_tr_absorb_label(&mut tctx, "mlen");
        kat_tr_absorb_u64(&mut tctx, mlen);

        randombytes(&mut drbg, &mut msg[..mlen as usize]);
        kat_tr_absorb_label(&mut tctx, "msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen as usize);

        for b in m.iter_mut().take(mlen as usize) {
            *b = 0;
        }
        for b in m1.iter_mut().take(mlen as usize + CRYPTO_BYTES) {
            *b = 0;
        }
        for b in sm.iter_mut().take(mlen as usize + CRYPTO_BYTES) {
            *b = 0;
        }
        m[..mlen as usize].copy_from_slice(&msg[..mlen as usize]);

        // Generate keypair using DRBG.
        let drbg_ref = &mut drbg;
        let ret = crypto_sign_keypair(&mut pk, &mut sk, |buf: &mut [u8]| {
            randombytes(drbg_ref, buf);
        });
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            return std::process::ExitCode::from(KAT_CRYPTO_FAILURE as u8);
        }
        kat_tr_absorb_label(&mut tctx, "pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, "sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let drbg_ref = &mut drbg;
        let ret = crypto_sign(
            &mut sm,
            &mut smlen,
            &m[..mlen as usize],
            mlen,
            &sk,
            |buf: &mut [u8]| {
                randombytes(drbg_ref, buf);
            },
        );
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            return std::process::ExitCode::from(KAT_CRYPTO_FAILURE as u8);
        }
        kat_tr_absorb_label(&mut tctx, "smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, "sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            return std::process::ExitCode::from(KAT_CRYPTO_FAILURE as u8);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            return std::process::ExitCode::from(KAT_CRYPTO_FAILURE as u8);
        }
        if m[..mlen as usize] != m1[..mlen as usize] {
            eprintln!("m mismatch");
            return std::process::ExitCode::from(KAT_CRYPTO_FAILURE as u8);
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for byte in digest.iter() {
        print!("{:02X}", byte);
    }
    println!();

    std::process::ExitCode::from(KAT_SUCCESS as u8)
}

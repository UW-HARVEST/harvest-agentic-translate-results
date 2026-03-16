mod params;
mod context;
mod address;
mod blake;
mod utils;
mod hash;
mod thash;
mod rng;
mod wots;
mod wotsx1;
mod utilsx1;
mod fors;
mod merkle;
mod sign;

use params::*;
use blake::blake512::{BlakeState512, blake512_init, blake512_update, blake512_final};

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// KAT transcript using BLAKE-512 (since SPX_N >= 24)
struct KatTrCtx {
    inner: BlakeState512,
}

impl KatTrCtx {
    fn new() -> Self {
        KatTrCtx { inner: BlakeState512::new() }
    }
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    blake512_init(&mut ctx.inner);
    let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
    blake512_update(&mut ctx.inner, tag, (tag.len() as u64) * 8);
    let sep: u8 = 0x00;
    blake512_update(&mut ctx.inner, &[sep], 8);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    let n = label.len();
    blake512_update(&mut ctx.inner, label, (n as u64) * 8);
    let sep: u8 = 0x00;
    blake512_update(&mut ctx.inner, &[sep], 8);
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
    blake512_update(&mut ctx.inner, &lenle, 64);
    blake512_update(&mut ctx.inner, &le, 64);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    blake512_update(&mut ctx.inner, &lenle, 64);
    if len > 0 {
        blake512_update(&mut ctx.inner, &buf[..len], (len as u64) * 8);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; 64];
    blake512_final(&mut ctx.inner, &mut outbuf);
    out32.copy_from_slice(&outbuf[..32]);
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
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes(), CRYPTO_ALGNAME.len());
    kat_tr_absorb_label(&mut tctx, b"SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed, 48);

        kat_tr_absorb_label(&mut tctx, b"count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, b"seed");
        kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(KAT_OVERFLOW);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen);

        rng::randombytes(&mut msg, mlen);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen as usize);

        let ml = mlen as usize;
        for j in 0..ml { m[j] = 0; }
        for j in 0..ml + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..ml + CRYPTO_BYTES { sm[j] = 0; }
        m[..ml].copy_from_slice(&msg[..ml]);

        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let ret = sign::crypto_sign(&mut sm, &mut smlen, &m[..ml], mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = sign::crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if m[..ml] != m1[..ml] {
            eprintln!("m mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();

    std::process::exit(KAT_SUCCESS);
}

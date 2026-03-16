mod params;
mod blake256;
mod blake512;
mod rng;
mod address;
mod utils;
mod hash_blake;
mod thash;
mod wots;
mod fors;
mod merkle;
mod wotsx1;
mod utilsx1;
mod sign;

use params::*;
use blake512::{Blake512State, blake512_init, blake512_update, blake512_final};

struct KatTrCtx {
    state: Blake512State,
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    blake512_init(&mut ctx.state);
    let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
    blake512_update(&mut ctx.state, tag, (tag.len() as u64) * 8);
    let sep = [0u8; 1];
    blake512_update(&mut ctx.state, &sep, 8);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    blake512_update(&mut ctx.state, label, (label.len() as u64) * 8);
    let sep = [0u8; 1];
    blake512_update(&mut ctx.state, &sep, 8);
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
    blake512_update(&mut ctx.state, &lenle, 64);
    blake512_update(&mut ctx.state, &le, 64);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    blake512_update(&mut ctx.state, &lenle, 64);
    if len > 0 {
        blake512_update(&mut ctx.state, &buf[..len], (len as u64) * 8);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; 64];
    blake512_final(&mut ctx.state, &mut outbuf);
    out32.copy_from_slice(&outbuf[..32]);
}

fn main() {
    const BASE_MLEN: usize = 33;
    const LOOP_COUNT: usize = 7;

    let mut m = [0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = [0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut m1 = [0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut pk = [0u8; SPX_PK_BYTES];
    let mut sk = [0u8; SPX_SK_BYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = [0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    rng::randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { state: Blake512State::new() };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    let algname = b"SPHINCS+";
    kat_tr_absorb_bytes(&mut tctx, algname, algname.len());
    kat_tr_absorb_label(&mut tctx, b"SKBYTES");
    kat_tr_absorb_u64(&mut tctx, SPX_SK_BYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES");
    kat_tr_absorb_u64(&mut tctx, SPX_PK_BYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, SPX_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed, 48);

        kat_tr_absorb_label(&mut tctx, b"count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, b"seed");
        kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen);

        rng::randombytes(&mut msg, mlen as u64);

        let ml = mlen as usize;
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, ml);

        for j in 0..ml { m[j] = 0; }
        for j in 0..(ml + SPX_BYTES) { m1[j] = 0; }
        for j in 0..(ml + SPX_BYTES) { sm[j] = 0; }
        m[..ml].copy_from_slice(&msg[..ml]);

        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, SPX_PK_BYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, SPX_SK_BYTES);

        let mut smlen: u64 = 0;
        let ret = sign::crypto_sign(&mut sm, &mut smlen, &m, mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = sign::crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
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

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

mod params;
mod context;
mod fips202;
mod rng;
mod address;
mod utils;
mod thash;
mod hash;
mod wots;
mod wotsx1;
mod fors;
mod merkle;
mod sign;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    s: [u64; 26],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    fips202::shake256_inc_init(&mut ctx.s);
    let tag = b"KAT-TRANSCRIPT-v1-SHAKE";
    fips202::shake256_inc_absorb(&mut ctx.s, tag);
    fips202::shake256_inc_absorb(&mut ctx.s, &[0x00]);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &str) {
    fips202::shake256_inc_absorb(&mut ctx.s, label.as_bytes());
    fips202::shake256_inc_absorb(&mut ctx.s, &[0x00]);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    fips202::shake256_inc_absorb(&mut ctx.s, &lenle);
    fips202::shake256_inc_absorb(&mut ctx.s, &le);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    fips202::shake256_inc_absorb(&mut ctx.s, &lenle);
    if len > 0 {
        fips202::shake256_inc_absorb(&mut ctx.s, &buf[..len]);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    fips202::shake256_inc_finalize(&mut ctx.s);
    fips202::shake256_inc_squeeze(out32, 32, &mut ctx.s);
}

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + SPX_BYTES];
    let mut pk = [0u8; SPX_PK_BYTES];
    let mut sk = [0u8; SPX_SK_BYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 { entropy_input[i] = i as u8; }
    rng::randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { s: [0u64; 26] };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, "CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME.as_bytes(), CRYPTO_ALGNAME.len());
    kat_tr_absorb_label(&mut tctx, "SKBYTES"); kat_tr_absorb_u64(&mut tctx, SPX_SK_BYTES as u64);
    kat_tr_absorb_label(&mut tctx, "PKBYTES"); kat_tr_absorb_u64(&mut tctx, SPX_PK_BYTES as u64);
    kat_tr_absorb_label(&mut tctx, "SIGBYTES"); kat_tr_absorb_u64(&mut tctx, SPX_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng::randombytes(&mut seed, 48);

        kat_tr_absorb_label(&mut tctx, "count"); kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, "seed"); kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, "mlen"); kat_tr_absorb_u64(&mut tctx, mlen as u64);

        rng::randombytes(&mut msg, mlen);
        kat_tr_absorb_label(&mut tctx, "msg"); kat_tr_absorb_bytes(&mut tctx, &msg, mlen);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..(mlen + SPX_BYTES) { m1[j] = 0; }
        for j in 0..(mlen + SPX_BYTES) { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, "pk"); kat_tr_absorb_bytes(&mut tctx, &pk, SPX_PK_BYTES);
        kat_tr_absorb_label(&mut tctx, "sk"); kat_tr_absorb_bytes(&mut tctx, &sk, SPX_SK_BYTES);

        let mut smlen: u64 = 0;
        let ret = sign::crypto_sign(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, "smlen"); kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, "sm"); kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = sign::crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
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

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 { print!("{:02X}", digest[i]); }
    println!();
}

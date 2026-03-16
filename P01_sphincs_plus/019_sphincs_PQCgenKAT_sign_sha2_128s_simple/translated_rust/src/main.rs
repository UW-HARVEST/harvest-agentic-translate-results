#![allow(unused_imports, unused_assignments, static_mut_refs)]

mod params;
mod context;
mod address;
mod sha2;
mod utils;
mod thash;
mod hash_sha2;
mod wots;
mod wotsx1;
mod fors;
mod merkle;
mod rng;
mod sign;

#[allow(unused_imports)]
use params::*;

const MAX_MARKER_LEN: usize = 50;
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const KAT_SUCCESS: i32 = 0;
const KAT_OVERFLOW: i32 = -1;
const KAT_CRYPTO_FAILURE: i32 = -2;

// SHA2 transcript context (sha256 incremental, since SPX_N < 24)
const SHAX_STATE_LEN: usize = 40;
const SHAX_BLOCK_BYTES: usize = 64;
const SHAX_OUTPUT_BYTES: usize = 32;

struct KatTrCtx {
    s: [u8; SHAX_STATE_LEN],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    let tag = b"KAT-TRANSCRIPT-v1-SHA2";
    let mut block = [0u8; SHAX_BLOCK_BYTES];
    for i in 0..tag.len() {
        block[i] = tag[i];
    }
    // rest already zero

    sha2::sha256_inc_init(&mut ctx.s);
    sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    let n = label.len();
    let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

    for i in 0..block_count {
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        let mut j = 0usize;
        while i * SHAX_BLOCK_BYTES + j < n && j < SHAX_BLOCK_BYTES {
            block[j] = label[i * SHAX_BLOCK_BYTES + j];
            j += 1;
        }
        if i * SHAX_BLOCK_BYTES + j == n && j < SHAX_BLOCK_BYTES {
            block[j] = 0x00;
            j += 1;
        }
        // rest already zero
        sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
    }
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

    let mut block = [0u8; SHAX_BLOCK_BYTES];
    block[..8].copy_from_slice(&lenle);
    block[8..16].copy_from_slice(&le);
    // rest zero
    sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; SHAX_BLOCK_BYTES];
    let l = len as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    let block_count = (len + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;
    sha2::sha256_inc_blocks(&mut ctx.s, &lenle, 1);

    if len != 0 {
        for i in 0..block_count {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            let mut j = 0usize;
            while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                j += 1;
            }
            // rest zero
            sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
        }
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
    // C: shaX_inc_finalize(outbuf, ctx->s, final_block, 1) where final_block is zeroed
    // inlen=1 means 1 byte of zero data
    let final_block = [0u8; SHAX_BLOCK_BYTES];
    sha2::sha256_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
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

    let mut tctx = KatTrCtx { s: [0u8; SHAX_STATE_LEN] };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
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

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(KAT_OVERFLOW);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        rng::randombytes(&mut msg[..mlen], mlen as u64);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg[..mlen], mlen);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..(mlen + CRYPTO_BYTES) { m1[j] = 0; }
        for j in 0..(mlen + CRYPTO_BYTES) { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen = 0u64;
        let ret = sign::crypto_sign(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm[..smlen as usize], smlen as usize);

        let mut mlen1 = 0u64;
        let ret = sign::crypto_sign_open(&mut m1, &mut mlen1, &sm[..smlen as usize], smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if mlen1 as usize != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(KAT_CRYPTO_FAILURE);
        }
        if m[..mlen] != m1[..mlen] {
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

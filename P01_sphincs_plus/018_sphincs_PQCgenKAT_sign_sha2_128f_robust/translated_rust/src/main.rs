mod address;
mod context;
mod fors;
mod hash;
mod merkle;
mod params;
mod rng;
mod sha2;
mod sign;
mod utils;
mod wots;
mod wotsx1;

use params::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    s: [u8; 40], // sha256 incremental state
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    let tag = b"KAT-TRANSCRIPT-v1-SHA2";
    let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
    block[..tag.len()].copy_from_slice(tag);
    // rest is already zero

    sha2::sha256_inc_init(&mut ctx.s);
    sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    let n = label.len();
    let block_count = (n + 1 + (SPX_SHA256_BLOCK_BYTES - 1)) / SPX_SHA256_BLOCK_BYTES;

    for i in 0..block_count {
        let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
        let mut j = 0usize;
        while i * SPX_SHA256_BLOCK_BYTES + j < n && j < SPX_SHA256_BLOCK_BYTES {
            block[j] = label[i * SPX_SHA256_BLOCK_BYTES + j];
            j += 1;
        }
        if i * SPX_SHA256_BLOCK_BYTES + j == n && j < SPX_SHA256_BLOCK_BYTES {
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

    let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
    block[..8].copy_from_slice(&lenle);
    block[8..16].copy_from_slice(&le);
    // rest zero
    sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; SPX_SHA256_BLOCK_BYTES];
    let l = len as u64;
    for i in 0..8 {
        lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
    }
    let block_count = (len + SPX_SHA256_BLOCK_BYTES - 1) / SPX_SHA256_BLOCK_BYTES;
    sha2::sha256_inc_blocks(&mut ctx.s, &lenle, 1);

    if len != 0 {
        for i in 0..block_count {
            let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
            let mut j = 0usize;
            while i * SPX_SHA256_BLOCK_BYTES + j < len && j < SPX_SHA256_BLOCK_BYTES {
                block[j] = buf[i * SPX_SHA256_BLOCK_BYTES + j];
                j += 1;
            }
            // rest zero
            sha2::sha256_inc_blocks(&mut ctx.s, &block, 1);
        }
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let final_block = [0u8; SPX_SHA256_BLOCK_BYTES];
    sha2::sha256_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
    // Note: C code passes final_block with inlen=1 (1 byte, not 1 block)
    // Actually looking at the C: shaX_inc_finalize(outbuf, ctx->s, final_block, 1)
    // The last param is inlen in bytes (not blocks). So it's 1 byte.
    // But wait - we already called it with 1. Let me re-check.
    // sha256_inc_finalize(out, state, in, inlen) where inlen is byte count
    // So final_block is 64 bytes of zeros, and inlen=1 means only 1 byte is used.
    // Our sha256_inc_finalize already handles this correctly.
    out32.copy_from_slice(&outbuf[..32]);
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

    for i in 0..48 {
        entropy_input[i] = i as u8;
    }
    rng::randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { s: [0u8; 40] };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
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

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        rng::randombytes(&mut msg, mlen as usize);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + SPX_BYTES { m1[j] = 0; }
        for j in 0..mlen + SPX_BYTES { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = sign::crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, SPX_PK_BYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, SPX_SK_BYTES);

        let mut smlen = 0u64;
        let ret = sign::crypto_sign(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1 = 0u64;
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
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

mod params;
mod context;
mod sha2;
mod spx;
mod sign;
mod rng;

use params::*;
use context::SpxCtx;
use rng::{randombytes_init, randombytes};
use spx::*;
use sign::*;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = SpxCtx::default();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    initialize_hash_function(&mut ctx);
    let mut root = [0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);
    0
}

fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, &seed)
}

fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx::default();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u8; 32];
    let mut tree_addr = [0u8; 32];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    randombytes(&mut optrand, SPX_N);

    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, mlen, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    if siglen != SPX_BYTES { return -1; }

    let mut ctx = SpxCtx::default();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u8; 32];
    let mut tree_addr = [0u8; 32];
    let mut wots_pk_addr = [0u8; 32];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen, &ctx);

    let mut sig_off = SPX_N;
    let mut root = [0u8; SPX_N];

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        let mut leaf = [0u8; SPX_N];
        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    0
}

fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen = 0usize;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    sm[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(&m[..mlen as usize]);
    *smlen = siglen as u64 + mlen;
    0
}

fn crypto_sign_open(m: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    if (smlen as usize) < SPX_BYTES {
        for i in 0..smlen as usize { m[i] = 0; }
        *mlen = 0;
        return -1;
    }
    *mlen = smlen - SPX_BYTES as u64;
    if crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for i in 0..smlen as usize { m[i] = 0; }
        *mlen = 0;
        return -1;
    }
    let ml = *mlen as usize;
    // memmove equivalent
    let src = sm[SPX_BYTES..SPX_BYTES + ml].to_vec();
    m[..ml].copy_from_slice(&src);
    0
}

// KAT transcript (SHA2_TR mode, SPX_N >= 24 => SHA-512)
struct KatTrCtx {
    s: [u8; 72], // sha512 state
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    let tag = b"KAT-TRANSCRIPT-v1-SHA2";
    let mut block = [0u8; 128]; // SHA512 block size
    block[..tag.len()].copy_from_slice(tag);
    sha2::sha512_inc_init(&mut ctx.s);
    sha2::sha512_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    let n = label.len();
    let block_count = (n + 1 + 127) / 128;
    for i in 0..block_count {
        let mut block = [0u8; 128];
        let mut j = 0;
        while i * 128 + j < n && j < 128 {
            block[j] = label[i * 128 + j];
            j += 1;
        }
        if i * 128 + j == n && j < 128 {
            block[j] = 0x00;
            j += 1;
        }
        while j < 128 { block[j] = 0; j += 1; }
        sha2::sha512_inc_blocks(&mut ctx.s, &block, 1);
    }
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }

    let mut block = [0u8; 128];
    block[..8].copy_from_slice(&lenle);
    block[8..16].copy_from_slice(&le);
    sha2::sha512_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle_block = [0u8; 128];
    let l = len as u64;
    for i in 0..8 { lenle_block[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    let block_count = (len + 127) / 128;
    sha2::sha512_inc_blocks(&mut ctx.s, &lenle_block, 1);

    if len != 0 {
        for i in 0..block_count {
            let mut block = [0u8; 128];
            let mut j = 0;
            while i * 128 + j < len && j < 128 {
                block[j] = buf[i * 128 + j];
                j += 1;
            }
            while j < 128 { block[j] = 0; j += 1; }
            sha2::sha512_inc_blocks(&mut ctx.s, &block, 1);
        }
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; 64];
    let final_block = [0u8; 128];
    sha2::sha512_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
    out32.copy_from_slice(&outbuf[..32]);
}

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { s: [0u8; 72] };
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
        randombytes(&mut seed, 48);

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

        randombytes(&mut msg, mlen);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen = 0u64;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1 = 0u64;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
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
    for b in &digest { print!("{:02X}", b); }
    println!();
}

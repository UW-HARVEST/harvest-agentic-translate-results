use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::{SPX_fors_pk_from_sig, SPX_fors_sign};
use crate::merkle::{SPX_merkle_gen_root, SPX_merkle_sign};
use crate::params::*;
use crate::randombytes::randombytes;
use crate::utils::compute_root;
use crate::wots::SPX_wots_pk_from_sig;

extern "C" {
    fn SPX_initialize_hash_function(ctx: *mut SpxCtx);
    fn SPX_gen_message_random(
        r: *mut u8, sk_prf: *const u8, optrand: *const u8,
        m: *const u8, mlen: u64, ctx: *const SpxCtx,
    );
    fn SPX_hash_message(
        digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
        r: *const u8, pk: *const u8, m: *const u8, mlen: u64, ctx: *const SpxCtx,
    );
    fn SPX_thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let mut ctx = SpxCtx::new();

    core::ptr::copy_nonoverlapping(seed, sk, CRYPTO_SEEDBYTES);
    core::ptr::copy_nonoverlapping(sk.add(2 * SPX_N), pk, SPX_N);

    core::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);

    SPX_initialize_hash_function(&mut ctx);

    SPX_merkle_gen_root(sk.add(3 * SPX_N), &ctx);

    core::ptr::copy_nonoverlapping(sk.add(3 * SPX_N), pk.add(SPX_N), SPX_N);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    mut sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let mut ctx = SpxCtx::new();

    let sk_prf = sk.add(SPX_N);
    let pk = sk.add(2 * SPX_N);

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    core::ptr::copy_nonoverlapping(sk, ctx.sk_seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

    SPX_initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(optrand.as_mut_ptr(), SPX_N as u64);
    SPX_gen_message_random(sig, sk_prf, optrand.as_ptr(), m, mlen as u64, &ctx);

    SPX_hash_message(
        mhash.as_mut_ptr(),
        &mut tree,
        &mut idx_leaf,
        sig,
        pk,
        m,
        mlen as u64,
        &ctx,
    );
    sig = sig.add(SPX_N);

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    SPX_fors_sign(sig, root.as_mut_ptr(), mhash.as_ptr(), &ctx, wots_addr.as_ptr());
    sig = sig.add(SPX_FORS_BYTES);

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        SPX_merkle_sign(
            sig,
            root.as_mut_ptr(),
            &ctx,
            wots_addr.as_mut_ptr(),
            tree_addr.as_mut_ptr(),
            idx_leaf,
        );
        sig = sig.add(SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N);

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1) as u64) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    mut sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let mut ctx = SpxCtx::new();
    let pub_root = pk.add(SPX_N);
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    if siglen != SPX_BYTES {
        return -1;
    }

    core::ptr::copy_nonoverlapping(pk, ctx.pub_seed.as_mut_ptr(), SPX_N);

    SPX_initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    SPX_hash_message(
        mhash.as_mut_ptr(),
        &mut tree,
        &mut idx_leaf,
        sig,
        pk,
        m,
        mlen as u64,
        &ctx,
    );
    sig = sig.add(SPX_N);

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    SPX_fors_pk_from_sig(root.as_mut_ptr(), sig, mhash.as_ptr(), &ctx, wots_addr.as_ptr());
    sig = sig.add(SPX_FORS_BYTES);

    for _i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, _i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        SPX_wots_pk_from_sig(
            wots_pk.as_mut_ptr(),
            sig,
            root.as_ptr(),
            &ctx,
            wots_addr.as_mut_ptr(),
        );
        sig = sig.add(SPX_WOTS_BYTES);

        SPX_thash(leaf.as_mut_ptr(), wots_pk.as_ptr(), SPX_WOTS_LEN as u32, &ctx, wots_pk_addr.as_mut_ptr());

        compute_root(
            root.as_mut_ptr(),
            leaf.as_ptr(),
            idx_leaf,
            0,
            sig,
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig = sig.add(SPX_TREE_HEIGHT * SPX_N);

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1) as u64) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if core::slice::from_raw_parts(root.as_ptr(), SPX_N)
        != core::slice::from_raw_parts(pub_root, SPX_N)
    {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let mut siglen: usize = 0;

    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
    *smlen = siglen as u64 + mlen;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    if smlen < SPX_BYTES as u64 {
        core::ptr::write_bytes(m, 0, smlen as usize);
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;

    if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
        core::ptr::write_bytes(m, 0, smlen as usize);
        *mlen = 0;
        return -1;
    }

    core::ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);

    0
}

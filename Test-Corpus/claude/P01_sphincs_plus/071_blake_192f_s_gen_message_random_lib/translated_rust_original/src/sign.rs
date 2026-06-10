use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::{SPX_fors_pk_from_sig, SPX_fors_sign};
use crate::hash::{gen_message_random, hash_message, initialize_hash_function};
use crate::merkle::{SPX_merkle_gen_root, SPX_merkle_sign};
use crate::params::*;
use crate::randombytes::randombytes_rs;
use crate::thash::thash;
use crate::utils::compute_root_rs;
use crate::wots::SPX_wots_pk_from_sig;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let mut ctx = SpxCtx::new();
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk_slice = unsafe { core::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed_slice = unsafe { core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    sk_slice[..CRYPTO_SEEDBYTES].copy_from_slice(seed_slice);
    pk_slice[..SPX_N].copy_from_slice(&sk_slice[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk_slice[..SPX_N]);

    initialize_hash_function(&mut ctx);

    // merkle_gen_root(sk + 3*SPX_N, &ctx)
    SPX_merkle_gen_root(unsafe { sk.add(3 * SPX_N) }, &ctx as *const _);

    pk_slice[SPX_N..2 * SPX_N].copy_from_slice(&sk_slice[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = vec![0u8; CRYPTO_SEEDBYTES];
    randombytes_rs(&mut seed);
    SPX_crypto_sign_seed_keypair(pk, sk, seed.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let mut ctx = SpxCtx::new();
    let sk_slice = unsafe { core::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen) };

    let sk_seed = &sk_slice[0..SPX_N];
    let sk_prf = &sk_slice[SPX_N..2 * SPX_N];
    let pk = &sk_slice[2 * SPX_N..2 * SPX_N + SPX_PK_BYTES];

    ctx.sk_seed.copy_from_slice(sk_seed);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = vec![0u8; SPX_N];
    randombytes_rs(&mut optrand);

    // Layout sig: [R || FORS sig || hypertree sigs]
    // Compute R into sig[0..SPX_N]
    let r_slice = unsafe { core::slice::from_raw_parts_mut(sig, SPX_N) };
    gen_message_random(r_slice, sk_prf, &optrand, m_slice, &ctx);

    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let r_const = unsafe { core::slice::from_raw_parts(sig, SPX_N) };
    let (mut tree, mut idx_leaf) = hash_message(&mut mhash, r_const, pk, m_slice, &ctx);

    let mut sig_offset = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    // FORS sign
    let mut root = vec![0u8; SPX_N];
    SPX_fors_sign(
        unsafe { sig.add(sig_offset) },
        root.as_mut_ptr(),
        mhash.as_ptr(),
        &ctx,
        &wots_addr as *const _,
    );
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D as u32 {
        set_layer_addr(&mut tree_addr, i);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        SPX_merkle_sign(
            unsafe { sig.add(sig_offset) },
            root.as_mut_ptr(),
            &ctx,
            &mut wots_addr as *mut _,
            &mut tree_addr as *mut _,
            idx_leaf,
        );
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree as u32) & ((1u32 << SPX_TREE_HEIGHT) - 1);
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe {
        *siglen = SPX_BYTES;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let mut ctx = SpxCtx::new();
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let pub_root = &pk_slice[SPX_N..2 * SPX_N];
    ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);

    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let r_const = unsafe { core::slice::from_raw_parts(sig, SPX_N) };
    let (mut tree, mut idx_leaf) = hash_message(&mut mhash, r_const, pk_slice, m_slice, &ctx);

    let mut sig_offset = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = vec![0u8; SPX_N];
    SPX_fors_pk_from_sig(
        root.as_mut_ptr(),
        unsafe { sig.add(sig_offset) },
        mhash.as_ptr(),
        &ctx,
        &wots_addr as *const _,
    );
    sig_offset += SPX_FORS_BYTES;

    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    for i in 0..SPX_D as u32 {
        set_layer_addr(&mut tree_addr, i);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        SPX_wots_pk_from_sig(
            wots_pk.as_mut_ptr(),
            unsafe { sig.add(sig_offset) },
            root.as_ptr(),
            &ctx,
            &mut wots_addr as *mut _,
        );
        sig_offset += SPX_WOTS_BYTES;

        thash(&mut root, &wots_pk, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);

        let auth = unsafe { core::slice::from_raw_parts(sig.add(sig_offset), SPX_TREE_HEIGHT * SPX_N) };
        let mut new_root = vec![0u8; SPX_N];
        compute_root_rs(
            &mut new_root,
            &root,
            idx_leaf,
            0,
            auth,
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        root = new_root;
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree as u32) & ((1u32 << SPX_TREE_HEIGHT) - 1);
        tree >>= SPX_TREE_HEIGHT;
    }

    if root != pub_root {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let mut siglen: usize = 0;
    SPX_crypto_sign_signature(sm, &mut siglen as *mut _, m, mlen as usize, sk);

    // memmove(sm + SPX_BYTES, m, mlen)
    unsafe {
        core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen + mlen as usize) as u64;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    if smlen < SPX_BYTES as u64 {
        unsafe {
            let s = core::slice::from_raw_parts_mut(m, smlen as usize);
            for b in s {
                *b = 0;
            }
            *mlen = 0;
        }
        return -1;
    }

    let computed_mlen = smlen - SPX_BYTES as u64;
    unsafe {
        *mlen = computed_mlen;
    }

    let verify = SPX_crypto_sign_verify(
        sm,
        SPX_BYTES,
        unsafe { sm.add(SPX_BYTES) },
        computed_mlen as usize,
        pk,
    );
    if verify != 0 {
        unsafe {
            let s = core::slice::from_raw_parts_mut(m, smlen as usize);
            for b in s {
                *b = 0;
            }
            *mlen = 0;
        }
        return -1;
    }

    // memmove(m, sm + SPX_BYTES, *mlen)
    unsafe {
        core::ptr::copy(sm.add(SPX_BYTES), m, computed_mlen as usize);
    }

    0
}

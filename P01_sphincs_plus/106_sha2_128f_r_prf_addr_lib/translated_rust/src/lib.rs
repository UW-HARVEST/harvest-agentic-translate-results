#![allow(clippy::all)]
#![allow(unused_unsafe)]

mod params;
mod address;
mod hash;
mod thash;
mod wots;
mod fors;
mod utils;
mod utilsx1;
mod merkle;

use address::SpxCtx;
use params::*;

fn randombytes(x: &mut [u8], xlen: usize) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut remaining = xlen;
    let mut offset = 0;
    while remaining > 0 {
        match f.read(&mut x[offset..offset + remaining]) {
            Ok(0) => continue,
            Ok(n) => { offset += n; remaining -= n; }
            Err(_) => { std::thread::sleep(std::time::Duration::from_secs(1)); }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    hash::initialize_hash_function(&mut ctx);
    merkle::merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    hash::initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    randombytes(&mut optrand, SPX_N);

    hash::gen_message_random(sig, sk_prf, &optrand, m, mlen, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen, &ctx);

    let mut sig_off = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors::fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        address::set_layer_addr(&mut tree_addr, i as u32);
        address::set_tree_addr(&mut tree_addr, tree);

        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle::merkle_sign(
            &mut sig[sig_off..],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe { *siglen = SPX_BYTES; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let sig_slice = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    let pub_root = &pk[SPX_N..];

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    hash::initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    address::set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    hash::hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_slice, pk, m, mlen, &ctx);

    let mut sig_off = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors::fors_pk_from_sig(&mut root, &sig_slice[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for _i in 0..SPX_D {
        address::set_layer_addr(&mut tree_addr, _i as u32);
        address::set_tree_addr(&mut tree_addr, tree);

        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        address::copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        wots::wots_pk_from_sig(&mut wots_pk, &sig_slice[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        let mut leaf = [0u8; SPX_N];
        thash::thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        utils::compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig_slice[sig_off..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root != pub_root[..SPX_N] {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let _sm_slice = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let _m_slice = unsafe { std::slice::from_raw_parts(m, mlen as usize) };

    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen as *mut usize, m, mlen as usize, sk);

    // memmove sm + SPX_BYTES <- m
    unsafe {
        core::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
    }
    unsafe { *smlen = (siglen as u64) + mlen; }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    if (smlen as usize) < SPX_BYTES {
        unsafe {
            core::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }

    let msg_len = smlen as usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64; }

    let result = crypto_sign_verify(sm, SPX_BYTES, unsafe { sm.add(SPX_BYTES) }, msg_len, pk);
    if result != 0 {
        unsafe {
            core::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe {
        core::ptr::copy(sm.add(SPX_BYTES), m, msg_len);
    }
    0
}

mod params;
mod blake256;
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

use params::*;
use rng::{randombytes_init, randombytes};
use blake256::Blakestate256;

const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    state: Blakestate256,
}

impl KatTrCtx {
    fn init() -> Self {
        let mut state = Blakestate256::new();
        blake256::blake256_init(&mut state);
        let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
        blake256::blake256_update(&mut state, tag, tag.len() as u64);
        let sep = [0u8; 1];
        blake256::blake256_update(&mut state, &sep, 1);
        KatTrCtx { state }
    }

    fn absorb_label(&mut self, label: &[u8]) {
        let n = label.len();
        blake256::blake256_update(&mut self.state, label, n as u64);
        let sep = [0u8; 1];
        blake256::blake256_update(&mut self.state, &sep, 1);
    }

    fn absorb_u64(&mut self, x: u64) {
        let mut le = [0u8; 8];
        for i in 0..8 {
            le[i] = ((x >> (8 * i)) & 0xFF) as u8;
        }
        let mut lenle = [0u8; 8];
        let l: u64 = 8;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        blake256::blake256_update(&mut self.state, &lenle, 8);
        blake256::blake256_update(&mut self.state, &le, 8);
    }

    fn absorb_bytes(&mut self, buf: &[u8], len: usize) {
        let mut lenle = [0u8; 8];
        let l = len as u64;
        for i in 0..8 {
            lenle[i] = ((l >> (8 * i)) & 0xFF) as u8;
        }
        blake256::blake256_update(&mut self.state, &lenle, 8);
        if len > 0 {
            blake256::blake256_update(&mut self.state, &buf[..len], len as u64);
        }
    }

    fn finalize(&mut self, out32: &mut [u8; 32]) {
        let mut outbuf = [0u8; 32];
        blake256::blake256_final(&mut self.state, &mut outbuf);
        out32.copy_from_slice(&outbuf[..32]);
    }
}

fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> i32 {
    let mut ctx = hash_blake::SpxCtx::new();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed[..SPX_N].copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed[..SPX_N].copy_from_slice(&sk[..SPX_N]);
    hash_blake::initialize_hash_function(&mut ctx);
    merkle::merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = hash_blake::SpxCtx::new();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed[..SPX_N].copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed[..SPX_N].copy_from_slice(&pk[..SPX_N]);
    hash_blake::initialize_hash_function(&mut ctx);

    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N as u64);
    hash_blake::gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, &m[..mlen], &ctx);
    hash_blake::hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, &m[..mlen], &ctx);

    let mut sig_offset = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_sign(&mut sig[sig_offset..], &mut root, &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        address::set_layer_addr(&mut tree_addr, i as u32);
        address::set_tree_addr(&mut tree_addr, tree);
        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle::merkle_sign(&mut sig[sig_offset..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_offset += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let mut ctx = hash_blake::SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk_buf = [0u8; SPX_WOTS_BYTES];
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

    ctx.pub_seed[..SPX_N].copy_from_slice(&pk[..SPX_N]);
    hash_blake::initialize_hash_function(&mut ctx);

    address::set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    address::set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    address::set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_blake::hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, &m[..mlen], &ctx);
    let mut sig_offset = SPX_N;

    address::set_tree_addr(&mut wots_addr, tree);
    address::set_keypair_addr(&mut wots_addr, idx_leaf);

    fors::fors_pk_from_sig(&mut root, &sig[sig_offset..], &mhash, &ctx, &wots_addr);
    sig_offset += SPX_FORS_BYTES;

    for _i in 0..SPX_D {
        address::set_layer_addr(&mut tree_addr, _i as u32);
        address::set_tree_addr(&mut tree_addr, tree);
        address::copy_subtree_addr(&mut wots_addr, &tree_addr);
        address::set_keypair_addr(&mut wots_addr, idx_leaf);
        address::copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots::wots_pk_from_sig(&mut wots_pk_buf, &sig[sig_offset..], &root, &ctx, &mut wots_addr);
        sig_offset += SPX_WOTS_BYTES;

        thash::thash(&mut leaf, &wots_pk_buf, SPX_WOTS_LEN as u32, &ctx, &mut wots_pk_addr);
        utils::compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_offset..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_offset += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }
    0
}

fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen: usize = 0;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    // memmove sm + SPX_BYTES <- m
    let mlen_usize = mlen as usize;
    for i in (0..mlen_usize).rev() {
        sm[SPX_BYTES + i] = m[i];
    }
    *smlen = (siglen as u64) + mlen;
    0
}

fn crypto_sign_open(m_out: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    let smlen_usize = smlen as usize;
    if smlen_usize < SPX_BYTES {
        for i in 0..smlen_usize {
            m_out[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;

    if crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for i in 0..smlen_usize {
            m_out[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    let ml = *mlen as usize;
    m_out[..ml].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + ml]);
    0
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
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx::init();
    tctx.absorb_label(b"CRYPTO_ALGNAME");
    let algname = CRYPTO_ALGNAME;
    tctx.absorb_bytes(algname, algname.len());
    tctx.absorb_label(b"SKBYTES");
    tctx.absorb_u64(CRYPTO_SECRETKEYBYTES as u64);
    tctx.absorb_label(b"PKBYTES");
    tctx.absorb_u64(CRYPTO_PUBLICKEYBYTES as u64);
    tctx.absorb_label(b"SIGBYTES");
    tctx.absorb_u64(CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);

        tctx.absorb_label(b"count");
        tctx.absorb_u64(i as u64);
        tctx.absorb_label(b"seed");
        tctx.absorb_bytes(&seed, 48);

        let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
        if mlen as usize > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        tctx.absorb_label(b"mlen");
        tctx.absorb_u64(mlen);

        randombytes(&mut msg[..mlen as usize], mlen);
        tctx.absorb_label(b"msg");
        tctx.absorb_bytes(&msg, mlen as usize);

        let ml = mlen as usize;
        for j in 0..ml { m[j] = 0; }
        for j in 0..(ml + CRYPTO_BYTES) { m1[j] = 0; }
        for j in 0..(ml + CRYPTO_BYTES) { sm[j] = 0; }
        m[..ml].copy_from_slice(&msg[..ml]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        tctx.absorb_label(b"pk");
        tctx.absorb_bytes(&pk, CRYPTO_PUBLICKEYBYTES);
        tctx.absorb_label(b"sk");
        tctx.absorb_bytes(&sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..ml], mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        tctx.absorb_label(b"smlen");
        tctx.absorb_u64(smlen);
        tctx.absorb_label(b"sm");
        tctx.absorb_bytes(&sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
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
    tctx.finalize(&mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}

use crate::params::*;
use crate::context::SpxCtx;
use sha2::{Sha256, Sha512, Digest};

#[cfg(any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s"))]
type ShaX = Sha512;
#[cfg(not(any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s")))]
type ShaX = Sha256;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    let mut block = [0u8; 64];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let mut hasher = Sha256::new();
    hasher.update(&block);
    ctx.state_seeded = hasher;

    #[cfg(any(feature = "secpar-192f", feature = "secpar-192s", feature = "secpar-256f", feature = "secpar-256s"))]
    {
        let mut block512 = [0u8; 128];
        block512[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let mut hasher512 = Sha512::new();
        hasher512.update(&block512);
        ctx.state_seeded_512 = hasher512;
    }
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut hasher = ctx.state_seeded.clone();
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    hasher.update(&addr_bytes[..22]);
    hasher.update(&ctx.sk_seed);
    let res = hasher.finalize();
    out[..SPX_N].copy_from_slice(&res[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut hasher = ShaX::new();
    let mut buf = vec![0u8; if SPX_N > 16 { 128 } else { 64 }];
    let block_size = buf.len();
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..block_size { buf[i] = 0x36; }
    hasher.update(&buf);
    hasher.update(&optrand[..SPX_N]);
    hasher.update(m);
    let inner_res = hasher.finalize();

    let mut hasher = ShaX::new();
    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..block_size { buf[i] = 0x5c; }
    hasher.update(&buf);
    hasher.update(&inner_res);
    let res = hasher.finalize();
    r[..SPX_N].copy_from_slice(&res[..SPX_N]);
}

pub fn mgf1_x(out: &mut [u8], in_val: &[u8]) {
    let mut i = 0u32;
    let mut out_pos = 0;
    while out_pos < out.len() {
        let mut hasher = ShaX::new();
        hasher.update(in_val);
        hasher.update(&i.to_be_bytes());
        let res = hasher.finalize();
        let take = core::cmp::min(res.len(), out.len() - out_pos);
        out[out_pos..out_pos + take].copy_from_slice(&res[..take]);
        out_pos += take;
        i += 1;
    }
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut hasher = ShaX::new();
    hasher.update(&r[..SPX_N]);
    hasher.update(&pk[..SPX_PK_BYTES]);
    hasher.update(m);
    let seed_hash = hasher.finalize();

    let mut seed = vec![0u8; 2 * SPX_N + seed_hash.len()];
    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);
    seed[2 * SPX_N..].copy_from_slice(&seed_hash);

    let mut buf = vec![0u8; SPX_DGST_BYTES];
    mgf1_x(&mut buf, &seed);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut v: u64) {
    for i in (0..outlen).rev() {
        out[i] = (v & 0xff) as u8;
        v >>= 8;
    }
}

pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn set_layer_addr(addr: &mut [u8; 32], layer: u32) {
    addr[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut [u8; 32], tree: u64) {
    ull_to_bytes(&mut addr[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type(addr: &mut [u8; 32], typ: u32) {
    addr[SPX_OFFSET_TYPE] = typ as u8;
}

pub fn copy_subtree_addr(out: &mut [u8; 32], inp: &[u8; 32]) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&inp[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut [u8; 32], keypair: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut [u8; 32], inp: &[u8; 32]) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&inp[..SPX_OFFSET_TREE + 8]);
    out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&inp[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut [u8; 32], chain: u32) {
    addr[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut [u8; 32], hash: u32) {
    addr[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut [u8; 32], tree_height: u32) {
    addr[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut [u8; 32], tree_index: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_TREE_INDEX..], tree_index);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u8; 32]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: usize, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC inner
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x36; }

    sha512_inc_init(&mut state);
    sha512_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf[..SPX_N + mlen].to_vec();
        sha512_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &tmp, SPX_N + mlen);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha512_inc_blocks(&mut state, &buf, 1);
        let m_off = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let mlen_rem = mlen - m_off;
        sha512_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &m[m_off..], mlen_rem);
    }

    // HMAC outer
    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x5c; }

    let mut full = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    full.copy_from_slice(&buf);
    sha512(&mut buf[..SPX_SHA512_OUTPUT_BYTES], &full, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], mlen: usize, _ctx: &SpxCtx) {
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha512_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha512_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
        let m_off = fill;
        let mlen_rem = mlen - m_off;
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[m_off..], mlen_rem);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[SPX_FORS_MSG_BYTES..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    *leaf_idx = bytes_to_ull(&buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// thash: robust variant - SHA256 for 1 block, SHA512 for >1 blocks
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u8; 32]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u8; 32]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u8; 32]) {
    let mut buffer = [0u8; 2 * SPX_N];

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut ap_off = SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        if leaf_idx & 1 != 0 {
            let tmp = buffer.clone();
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let tmp = buffer.clone();
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    thash(root, &buffer, 2, ctx, addr);
}

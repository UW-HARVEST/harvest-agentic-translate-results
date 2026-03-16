use crate::params::*;
use crate::sha2::*;

pub fn u32_to_bytes(out: &mut [u8], v: u32) {
    out[0] = (v >> 24) as u8;
    out[1] = (v >> 16) as u8;
    out[2] = (v >> 8) as u8;
    out[3] = v as u8;
}

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut v: u64) {
    for i in (0..outlen).rev() {
        out[i] = (v & 0xff) as u8;
        v >>= 8;
    }
}

pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[i * SPX_SHA256_OUTPUT_BYTES..], &inbuf, inlen + 4);
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        out[i * SPX_SHA256_OUTPUT_BYTES..outlen]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA256_OUTPUT_BYTES]);
    }
}

pub fn mgf1_512(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    while (i + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha512(&mut out[i * SPX_SHA512_OUTPUT_BYTES..], &inbuf, inlen + 4);
        i += 1;
    }
    if outlen > i * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha512(&mut outbuf, &inbuf, inlen + 4);
        out[i * SPX_SHA512_OUTPUT_BYTES..outlen]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA512_OUTPUT_BYTES]);
    }
}

pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
    pub state_seeded_512: [u8; 72],
}

impl SpxCtx {
    pub fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            state_seeded: [0u8; 40],
            state_seeded_512: [0u8; 72],
        }
    }
}

pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
    sha512_inc_init(&mut ctx.state_seeded_512);
    sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

// Address manipulation (operates on [u8; 32])
pub type Addr = [u8; SPX_ADDR_BYTES];

pub fn set_layer_addr(addr: &mut Addr, layer: u32) {
    addr[SPX_OFFSET_LAYER] = layer as u8;
}

pub fn set_tree_addr(addr: &mut Addr, tree: u64) {
    ull_to_bytes(&mut addr[SPX_OFFSET_TREE..], 8, tree);
}

pub fn set_type(addr: &mut Addr, type_val: u32) {
    addr[SPX_OFFSET_TYPE] = type_val as u8;
}

pub fn copy_subtree_addr(out: &mut Addr, inp: &Addr) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&inp[..SPX_OFFSET_TREE + 8]);
}

pub fn set_keypair_addr(addr: &mut Addr, keypair: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_KP_ADDR..], keypair);
}

pub fn copy_keypair_addr(out: &mut Addr, inp: &Addr) {
    out[..SPX_OFFSET_TREE + 8].copy_from_slice(&inp[..SPX_OFFSET_TREE + 8]);
    out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&inp[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

pub fn set_chain_addr(addr: &mut Addr, chain: u32) {
    addr[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

pub fn set_hash_addr(addr: &mut Addr, hash: u32) {
    addr[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

pub fn set_tree_height(addr: &mut Addr, tree_height: u32) {
    addr[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

pub fn set_tree_index(addr: &mut Addr, tree_index: u32) {
    u32_to_bytes(&mut addr[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// thash - simple variant
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &Addr) {
    // For inblocks > 1, use SHA-512 (SPX_SHA512=1)
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let buflen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &Addr) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    let buflen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);

    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// prf_addr
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// gen_message_random - HMAC-SHA-512
pub fn gen_message_random(
    r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // ipad
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha512_inc_init(&mut state);
    sha512_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(optrand);

    let mlen = mlen as usize;
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf[..SPX_N + mlen].to_vec();
        sha512_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &tmp,
            SPX_N + mlen,
        );
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha512_inc_blocks(&mut state, &buf, 1);
        let m_off = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let mlen_rem = mlen - m_off;
        sha512_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &m[m_off..],
            mlen_rem,
        );
    }

    // opad
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let mut full = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    full.copy_from_slice(&buf[..SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES]);
    sha512(&mut buf, &full, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

// hash_message
pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mlen = mlen as usize;
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha512_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha512_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
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

    let bufp = &buf[SPX_FORS_MSG_BYTES..];
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(bufp, SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp2 = &bufp[SPX_TREE_BYTES..];
    *leaf_idx = bytes_to_ull(bufp2, SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

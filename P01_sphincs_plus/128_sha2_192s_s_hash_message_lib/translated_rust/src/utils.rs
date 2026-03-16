use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        crate::address::set_tree_height(addr, i + 1);
        crate::address::set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            crate::thash::thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let tmp = buffer;
            crate::thash::thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    crate::address::set_tree_height(addr, tree_height);
    crate::address::set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    crate::thash::thash(root, &buffer, 2, ctx, addr);
}

// MGF1-SHA256
pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha2::sha256(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha2::sha256(&mut outbuf, &inbuf, inlen + 4);
        out[off..off + (outlen - i * SPX_SHA256_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA256_OUTPUT_BYTES]);
    }
}

// MGF1-SHA512
pub fn mgf1_512(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha2::sha512(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha2::sha512(&mut outbuf, &inbuf, inlen + 4);
        out[off..off + (outlen - i * SPX_SHA512_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA512_OUTPUT_BYTES]);
    }
}

pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);

    sha2::sha256_inc_init(&mut ctx.state_seeded);
    sha2::sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);

    sha2::sha512_inc_init(&mut ctx.state_seeded_512);
    sha2::sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
}

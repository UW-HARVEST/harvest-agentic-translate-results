use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut input: u64) {
    for i in (0..outlen).rev() {
        out[i] = (input & 0xff) as u8;
        input >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], input: u32) {
    out[0] = (input >> 24) as u8;
    out[1] = (input >> 16) as u8;
    out[2] = (input >> 8) as u8;
    out[3] = input as u8;
}

pub fn bytes_to_ull(input: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
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
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(leaf);
        buffer[0..SPX_N].copy_from_slice(&auth_path[0..SPX_N]);
    } else {
        buffer[0..SPX_N].copy_from_slice(leaf);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[0..SPX_N]);
    }
    let mut auth_off = SPX_N;
    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);
        if leaf_idx & 1 != 0 {
            let out = thash(&buffer, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&out);
            buffer[0..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let out = thash(&buffer, 2, ctx, addr);
            buffer[0..SPX_N].copy_from_slice(&out);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    let out = thash(&buffer, 2, ctx, addr);
    root[..SPX_N].copy_from_slice(&out);
}

pub fn treehash(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: fn(&mut [u8], &SpxCtx, u32, &[u32; 8]),
    tree_addr: &mut [u32; 8],
) {
    let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset = 0usize;
    for idx in 0..(1u32 << tree_height) {
        gen_leaf(&mut stack[offset * SPX_N..(offset + 1) * SPX_N], ctx, idx + idx_offset, tree_addr);
        offset += 1;
        heights[offset - 1] = 0;
        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }
        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            set_tree_height(tree_addr, heights[offset - 1] + 1);
            set_tree_index(tree_addr, tree_idx + (idx_offset >> (heights[offset - 1] + 1)));
            let start = (offset - 2) * SPX_N;
            let out = thash(&stack[start..start + 2 * SPX_N], 2, ctx, tree_addr);
            stack[start..start + SPX_N].copy_from_slice(&out);
            offset -= 1;
            heights[offset - 1] += 1;
            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_path[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

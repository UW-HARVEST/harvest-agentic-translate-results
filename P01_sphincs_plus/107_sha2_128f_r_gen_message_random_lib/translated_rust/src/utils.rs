use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::thash::thash;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut in_: u64) {
    for i in (0..outlen).rev() {
        out[i] = (in_ & 0xff) as u8;
        in_ >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], in_: u32) {
    out[0] = (in_ >> 24) as u8;
    out[1] = (in_ >> 16) as u8;
    out[2] = (in_ >> 8) as u8;
    out[3] = in_ as u8;
}

pub fn bytes_to_ull(in_: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (in_[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2*SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2*SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        {
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&buffer);
            if leaf_idx & 1 != 0 {
                thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
                buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
            } else {
                thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
                buffer[SPX_N..2*SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
            }
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    thash(root, &buffer, 2, ctx, addr);
}

pub fn treehash(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                leaf_idx: u32, idx_offset: u32, tree_height: u32,
                gen_leaf: fn(&mut [u8], &SpxCtx, u32, &[u32; 8]),
                tree_addr: &mut [u32; 8]) {
    let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset = 0usize;

    for idx in 0..(1u32 << tree_height) {
        gen_leaf(&mut stack[offset * SPX_N..], ctx, idx.wrapping_add(idx_offset), tree_addr);
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..(offset) * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            set_tree_height(tree_addr, heights[offset - 1] + 1);
            set_tree_index(tree_addr, tree_idx.wrapping_add(idx_offset >> (heights[offset - 1] + 1)));

            let base = (offset - 2) * SPX_N;
            let mut tmp = [0u8; 2 * SPX_N];
            tmp[..2 * SPX_N].copy_from_slice(&stack[base..base + 2 * SPX_N]);
            thash(&mut stack[base..base + SPX_N], &tmp, 2, ctx, tree_addr);
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

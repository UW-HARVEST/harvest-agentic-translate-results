use crate::params::*;
use crate::context::SpxCtx;
use crate::thash::thash;
use crate::address::{set_tree_height, set_tree_index};

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut in_val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (in_val & 0xff) as u8;
        in_val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], in_val: u32) {
    out[0] = (in_val >> 24) as u8;
    out[1] = (in_val >> 16) as u8;
    out[2] = (in_val >> 8) as u8;
    out[3] = in_val as u8;
}

pub fn bytes_to_ull(in_val: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (in_val[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32, auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = vec![0u8; 2 * SPX_N];
    let mut path_pos = 0;

    if (leaf_idx & 1) != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[path_pos..path_pos + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[path_pos..path_pos + SPX_N]);
    }
    path_pos += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if (leaf_idx & 1) != 0 {
            let mut temp = vec![0u8; SPX_N];
            thash(&mut temp, &buffer, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&temp);
            buffer[..SPX_N].copy_from_slice(&auth_path[path_pos..path_pos + SPX_N]);
        } else {
            let mut temp = vec![0u8; SPX_N];
            thash(&mut temp, &buffer, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&temp);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[path_pos..path_pos + SPX_N]);
        }
        path_pos += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

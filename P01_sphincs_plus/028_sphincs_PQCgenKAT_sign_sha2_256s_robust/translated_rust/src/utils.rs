use crate::params::*;
use crate::context::SpxCtx;

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
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (in_[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2*SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off+SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2*SPX_N].copy_from_slice(&auth_path[ap_off..ap_off+SPX_N]);
    }
    ap_off += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        crate::address::set_tree_height(addr, i + 1);
        crate::address::set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            crate::thash::thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off+SPX_N]);
        } else {
            let tmp = buffer;
            crate::thash::thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2*SPX_N].copy_from_slice(&auth_path[ap_off..ap_off+SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    crate::address::set_tree_height(addr, tree_height);
    crate::address::set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    crate::thash::thash(root, &buffer, 2, ctx, addr);
}

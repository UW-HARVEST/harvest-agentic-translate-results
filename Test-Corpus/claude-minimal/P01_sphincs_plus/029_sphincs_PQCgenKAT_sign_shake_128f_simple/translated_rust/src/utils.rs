use crate::address;
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;

/// Converts the value of `in_val` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut in_val: u64) {
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (in_val & 0xff) as u8;
        in_val >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], in_val: u32) {
    out[0] = (in_val >> 24) as u8;
    out[1] = (in_val >> 16) as u8;
    out[2] = (in_val >> 8) as u8;
    out[3] = in_val as u8;
}

/// Converts `inlen` bytes in `in_buf` from big-endian byte order to an integer.
pub fn bytes_to_ull(in_buf: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (in_buf[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
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
    let mut buffer = vec![0u8; 2 * SPX_N];
    let mut ap_off = 0usize;

    if (leaf_idx & 1) == 1 {
        buffer[SPX_N..SPX_N + SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..SPX_N + SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    ap_off += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        address::set_tree_height(addr, i + 1);
        address::set_tree_index(addr, leaf_idx + idx_offset);

        if (leaf_idx & 1) == 1 {
            let mut out_tmp = vec![0u8; SPX_N];
            thash(&mut out_tmp, &buffer.clone(), 2, ctx, addr);
            buffer[SPX_N..SPX_N + SPX_N].copy_from_slice(&out_tmp);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let mut out_tmp = vec![0u8; SPX_N];
            thash(&mut out_tmp, &buffer.clone(), 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&out_tmp);
            buffer[SPX_N..SPX_N + SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    address::set_tree_height(addr, tree_height);
    address::set_tree_index(addr, leaf_idx + idx_offset);
    thash(&mut root[..SPX_N], &buffer, 2, ctx, addr);
}

/// Computes the Merkle treehash root and authentication path.
pub fn treehash<F>(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    mut gen_leaf: F,
    tree_addr: &mut [u32; 8],
) where
    F: FnMut(&mut [u8], &SpxCtx, u32, &[u32; 8]),
{
    let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset: usize = 0;

    let n_leaves: u32 = 1 << tree_height;
    for idx in 0..n_leaves {
        gen_leaf(
            &mut stack[offset * SPX_N..(offset + 1) * SPX_N],
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);

            address::set_tree_height(tree_addr, heights[offset - 1] + 1);
            address::set_tree_index(
                tree_addr,
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );
            // hash 2 blocks at stack[offset-2..offset]
            let in_start = (offset - 2) * SPX_N;
            let in_end = offset * SPX_N;
            let mut tmp_in = vec![0u8; 2 * SPX_N];
            tmp_in.copy_from_slice(&stack[in_start..in_end]);
            let mut tmp_out = vec![0u8; SPX_N];
            thash(&mut tmp_out, &tmp_in, 2, ctx, tree_addr);
            stack[in_start..in_start + SPX_N].copy_from_slice(&tmp_out);
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_path[h * SPX_N..(h + 1) * SPX_N]
                    .copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

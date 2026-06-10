// Utility functions: byte conversion, treehash etc.
use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;

#[inline]
pub fn ull_to_bytes_rs(out: &mut [u8], outlen: usize, mut inp: u64) {
    for i in (0..outlen).rev() {
        out[i] = (inp & 0xff) as u8;
        inp >>= 8;
    }
}

#[inline]
pub fn u32_to_bytes_rs(out: &mut [u8], inp: u32) {
    out[0] = (inp >> 24) as u8;
    out[1] = (inp >> 16) as u8;
    out[2] = (inp >> 8) as u8;
    out[3] = inp as u8;
}

#[inline]
pub fn bytes_to_ull_rs(inp: &[u8]) -> u64 {
    let mut retval: u64 = 0;
    let inlen = inp.len();
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, inp: u64) {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes_rs(slice, outlen as usize, inp);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, inp: u32) {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes_rs(slice, inp);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let slice = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    bytes_to_ull_rs(slice)
}

/// Computes a root node given a leaf and an auth path.
pub fn compute_root_rs(
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

    // place leaf
    if (leaf_idx & 1) != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut auth_idx = SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if (leaf_idx & 1) != 0 {
            let mut tmp = vec![0u8; SPX_N];
            thash(&mut tmp, &buffer, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&tmp);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_idx..auth_idx + SPX_N]);
        } else {
            let mut tmp = vec![0u8; SPX_N];
            thash(&mut tmp, &buffer, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&tmp);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_idx..auth_idx + SPX_N]);
        }
        auth_idx += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut [u32; 8],
) {
    let root_slice = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let leaf_slice = unsafe { core::slice::from_raw_parts(leaf, SPX_N) };
    let auth_slice =
        unsafe { core::slice::from_raw_parts(auth_path, (tree_height as usize) * SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let addr_ref = unsafe { &mut *addr };
    compute_root_rs(
        root_slice,
        leaf_slice,
        leaf_idx,
        idx_offset,
        auth_slice,
        tree_height,
        ctx_ref,
        addr_ref,
    );
}

/// `treehash`: builds a Merkle tree, computing root and authentication path.
pub fn treehash_rs<F>(
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
    let stack_len = (tree_height as usize + 1) * SPX_N;
    let mut stack = vec![0u8; stack_len];
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

            set_tree_height(tree_addr, heights[offset - 1] + 1);
            set_tree_index(
                tree_addr,
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );

            // hash stack[(offset-2)*SPX_N..(offset)*SPX_N] -> stack[(offset-2)*SPX_N..]
            let src_start = (offset - 2) * SPX_N;
            let mut input_buf = vec![0u8; 2 * SPX_N];
            input_buf.copy_from_slice(&stack[src_start..src_start + 2 * SPX_N]);
            let mut out_buf = vec![0u8; SPX_N];
            thash(&mut out_buf, &input_buf, 2, ctx, tree_addr);
            stack[src_start..src_start + SPX_N].copy_from_slice(&out_buf);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: extern "C" fn(*mut u8, *const SpxCtx, u32, *const [u32; 8]),
    tree_addr: *mut [u32; 8],
) {
    let root_slice = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_slice =
        unsafe { core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let tree_addr_ref = unsafe { &mut *tree_addr };
    treehash_rs(
        root_slice,
        auth_slice,
        ctx_ref,
        leaf_idx,
        idx_offset,
        tree_height,
        |leaf, ctx, idx, addr| gen_leaf(leaf.as_mut_ptr(), ctx as *const _, idx, addr as *const _),
        tree_addr_ref,
    );
}

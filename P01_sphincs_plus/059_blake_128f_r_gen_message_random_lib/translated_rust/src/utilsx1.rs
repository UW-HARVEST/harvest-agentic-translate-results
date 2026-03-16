use crate::params::*;
use crate::address::*;
use crate::context::SpxCtx;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};

/// Shared treehash logic parameterized by a leaf generation function
fn treehashx1_inner(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    gen_leaf: &mut dyn FnMut(&mut [u8], &SpxCtx, u32),
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..2 * SPX_N], ctx, idx.wrapping_add(idx_offset));

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            // Right node: combine with left from stack
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp: Vec<u8> = current.to_vec();
            thash(&mut current[SPX_N..2 * SPX_N], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child to stack
        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

pub fn wots_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
) {
    treehashx1_inner(
        root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, leaf_idx_arg| {
            wots_gen_leafx1(dest, ctx, leaf_idx_arg, info);
        },
    );
}

pub fn fors_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
) {
    treehashx1_inner(
        root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, leaf_idx_arg| {
            fors_gen_leafx1(dest, ctx, leaf_idx_arg, info);
        },
    );
}

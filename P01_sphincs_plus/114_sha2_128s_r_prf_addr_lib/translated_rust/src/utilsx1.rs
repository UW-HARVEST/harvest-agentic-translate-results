use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::{ForsGenLeafInfo, fors_gen_leafx1};
use crate::params::*;
use crate::thash::thash;
use crate::wots::LeafInfoX1;

fn treehash_generic(
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
        gen_leaf(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset));

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = (h as usize) * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let off = (h as usize) * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);

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
    let info_ptr = info as *mut LeafInfoX1;
    treehash_generic(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr,
        &mut |dest, ctx, addr_idx| {
            let info = unsafe { &mut *info_ptr };
            crate::wots::wots_gen_leafx1(dest, ctx, addr_idx, info);
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
    let info_ptr = info as *mut ForsGenLeafInfo;
    treehash_generic(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr,
        &mut |dest, ctx, addr_idx| {
            let info = unsafe { &mut *info_ptr };
            fors_gen_leafx1(dest, ctx, addr_idx, info);
        },
    );
}

use crate::params::*;
use crate::context::SpxCtx;
use crate::utils::*;
use crate::thash::thash;
use crate::wotsx1::{LeafInfoX1, wots_gen_leafx1};
use crate::fors::{ForsGenLeafInfo, fors_gen_leafx1};

pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx: u32 = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_start = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_start..left_start + SPX_N]);
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&current);
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save current to stack
        let stack_h = compute_stack_h(idx, tree_height, max_idx);
        stack[stack_h as usize * SPX_N..(stack_h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

fn compute_stack_h(idx: u32, tree_height: u32, max_idx: u32) -> u32 {
    let mut internal_idx = idx;
    let mut h: u32 = 0;
    loop {
        if h == tree_height { return h; }
        if (internal_idx & 1) == 0 && idx < max_idx { return h; }
        internal_idx >>= 1;
        h += 1;
    }
}

pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx: u32 = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_start = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_start..left_start + SPX_N]);
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&current);
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let stack_h = compute_stack_h(idx, tree_height, max_idx);
        stack[stack_h as usize * SPX_N..(stack_h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

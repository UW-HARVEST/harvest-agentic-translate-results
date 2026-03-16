use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};
use crate::params::*;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};

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
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

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
            let tmp = current.clone();
            crate::thash::thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save current to stack at height h
        // h is the height where we broke out of the inner loop
        // We need to figure out h: it's the number of trailing 1-bits of idx, but
        // we can compute it from the break condition
        let mut h = 0u32;
        {
            let mut ti = idx;
            while ti & 1 == 1 {
                h += 1;
                ti >>= 1;
            }
        }
        // Actually, the C code saves at the h where the inner for-loop breaks.
        // The inner loop increments h each iteration. It breaks when (internal_idx & 1) == 0 && idx < max_idx.
        // At that point h is the current height. Let's recompute properly.
        // The inner loop runs: h=0,1,2,... incrementing internal_idx >>= 1 each time.
        // It breaks when the shifted internal_idx is even and idx < max_idx.
        // internal_idx starts as idx. After h iterations, internal_idx = idx >> h.
        // Break condition: (idx >> h) & 1 == 0 && idx < max_idx
        // So h = number of trailing 1-bits of idx (when idx < max_idx).
        // When idx == max_idx, the loop continues until h == tree_height (return).
        let save_h = if idx < max_idx {
            let mut count = 0u32;
            let mut v = idx;
            while v & 1 == 1 {
                count += 1;
                v >>= 1;
            }
            count
        } else {
            continue; // we already returned above
        };
        stack[save_h as usize * SPX_N..(save_h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
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
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

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
            let tmp = current.clone();
            crate::thash::thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let save_h = if idx < max_idx {
            let mut count = 0u32;
            let mut v = idx;
            while v & 1 == 1 {
                count += 1;
                v >>= 1;
            }
            count
        } else {
            continue;
        };
        stack[save_h as usize * SPX_N..(save_h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

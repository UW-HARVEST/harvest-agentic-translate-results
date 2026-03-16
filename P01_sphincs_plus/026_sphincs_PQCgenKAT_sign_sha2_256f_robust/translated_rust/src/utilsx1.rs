use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::fors::ForsGenLeafInfo;
use crate::wotsx1::LeafInfoX1;

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        crate::wotsx1::wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            crate::thash::thash_inplace_right(&mut current, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child
        let h = {
            let mut h = 0u32;
            let mut ti = idx;
            while (ti & 1) != 0 || idx >= max_idx {
                ti >>= 1;
                h += 1;
                if h == tree_height { break; }
            }
            // We already broke out of the for loop at h
            // Recalculate h: it's the height where we stopped
            let mut hh = 0u32;
            let mut ii = idx;
            let mut il = leaf_idx;
            loop {
                if hh == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1;
                il >>= 1;
                hh += 1;
            }
            hh
        };
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        crate::fors::fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            crate::thash::thash_inplace_right(&mut current, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h = {
            let mut hh = 0u32;
            let mut ii = idx;
            loop {
                if hh == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1;
                hh += 1;
            }
            hh
        };
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

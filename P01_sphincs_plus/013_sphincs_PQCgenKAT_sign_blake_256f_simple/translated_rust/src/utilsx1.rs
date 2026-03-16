use crate::params::*;
use crate::address;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};

pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32,
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
                let off = (h as usize) * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            address::set_tree_height(tree_addr, h + 1);
            address::set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save current to stack
        let h_val = {
            // Determine h: count trailing ones of (idx+1) basically
            let mut ii = idx;
            let mut il = leaf_idx;
            let mut _iio = idx_offset;
            let mut hh = 0u32;
            loop {
                if hh == tree_height { break; }
                if (ii ^ il) == 0x01 { }
                if (ii & 1) == 0 && idx < max_idx { break; }
                _iio >>= 1;
                ii >>= 1;
                il >>= 1;
                hh += 1;
            }
            hh
        };
        let off = h_val as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32,
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
                let off = (h as usize) * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            address::set_tree_height(tree_addr, h + 1);
            address::set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h_val = {
            let mut ii = idx;
            let mut _il = leaf_idx;
            let mut _iio = idx_offset;
            let mut hh = 0u32;
            loop {
                if hh == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                _iio >>= 1;
                ii >>= 1;
                _il >>= 1;
                hh += 1;
            }
            hh
        };
        let off = h_val as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

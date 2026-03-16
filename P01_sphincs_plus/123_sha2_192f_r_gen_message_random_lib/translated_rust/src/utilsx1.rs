use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};
use crate::fors::{fors_gen_leafx1, ForsGenLeafInfo};

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
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
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
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let src = current.to_vec();
            thash(&mut current[SPX_N..], &src, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child to stack
        let h_val = {
            // Compute h: number of trailing 1-bits after the break
            // The break happens when (internal_idx & 1) == 0 && idx < max_idx
            // At that point h is the height we stopped at
            let mut tmp_idx = idx;
            let mut h_count = 0u32;
            // After the inner loop breaks, h is the current height
            // We need to figure out what h was when we broke out
            let mut ti = idx;
            let mut tl = leaf_idx;
            let mut tio = idx_offset;
            let mut hh = 0u32;
            loop {
                if hh == tree_height { break; }
                if (ti ^ tl) == 0x01 { }
                if (ti & 1) == 0 && idx < max_idx { break; }
                tio >>= 1;
                ti >>= 1;
                tl >>= 1;
                hh += 1;
            }
            hh
        };
        let ho = h_val as usize;
        if ho < th {
            stack[ho * SPX_N..(ho + 1) * SPX_N]
                .copy_from_slice(&current[SPX_N..2 * SPX_N]);
        }
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
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
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
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let src = current.to_vec();
            thash(&mut current[SPX_N..], &src, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child - compute h the same way
        let h_val = {
            let mut ti = idx;
            let mut tl = leaf_idx;
            let mut tio = idx_offset;
            let mut hh = 0u32;
            loop {
                if hh == tree_height { break; }
                if (ti & 1) == 0 && idx < max_idx { break; }
                tio >>= 1;
                ti >>= 1;
                tl >>= 1;
                hh += 1;
            }
            hh
        };
        let ho = h_val as usize;
        if ho < th {
            stack[ho * SPX_N..(ho + 1) * SPX_N]
                .copy_from_slice(&current[SPX_N..2 * SPX_N]);
        }
    }
}

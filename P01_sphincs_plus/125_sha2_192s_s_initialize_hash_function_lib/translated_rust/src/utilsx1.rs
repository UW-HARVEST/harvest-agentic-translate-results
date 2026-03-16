use crate::context::{SpxCtx, LeafInfoX1, ForsGenLeafInfo};
use crate::params::*;
use crate::utils::*;

pub fn wots_treehashx1_internal(
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
    let max_idx: u32 = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        crate::wotsx1::wots_gen_leafx1_internal(
            &mut current[SPX_N..],
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let hp = h as usize;
                auth_path[hp * SPX_N..(hp + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height_internal(tree_addr, h + 1);
            set_tree_index_internal(tree_addr, internal_idx / 2 + internal_idx_offset);

            let hp = h as usize;
            current[..SPX_N].copy_from_slice(&stack[hp * SPX_N..(hp + 1) * SPX_N]);
            let tmp = current.clone();
            crate::thash::thash_internal(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child
        let h_val = {
            let mut internal_idx = idx;
            let mut internal_leaf = leaf_idx;
            let mut h = 0u32;
            loop {
                if h == tree_height { break h; }
                if (internal_idx ^ internal_leaf) == 0x01 { }
                if (internal_idx & 1) == 0 && idx < max_idx { break h; }
                internal_idx >>= 1;
                internal_leaf >>= 1;
                h += 1;
            }
        };
        let hp = h_val as usize;
        stack[hp * SPX_N..(hp + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn fors_gen_leafx1_internal(
    leaf: &mut [u8],
    ctx: &SpxCtx,
    addr_idx: u32,
    info: &mut ForsGenLeafInfo,
) {
    set_tree_index_internal(&mut info.leaf_addrx, addr_idx);
    set_type_internal(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    crate::hash::prf_addr_internal(leaf, ctx, &info.leaf_addrx);

    set_type_internal(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp = leaf[..SPX_N].to_vec();
    crate::thash::thash_internal(leaf, &tmp, 1, ctx, &mut info.leaf_addrx);
}

pub fn fors_treehashx1_internal(
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
    let max_idx: u32 = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1_internal(
            &mut current[SPX_N..],
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let hp = h as usize;
                auth_path[hp * SPX_N..(hp + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height_internal(tree_addr, h + 1);
            set_tree_index_internal(tree_addr, internal_idx / 2 + internal_idx_offset);

            let hp = h as usize;
            current[..SPX_N].copy_from_slice(&stack[hp * SPX_N..(hp + 1) * SPX_N]);
            let tmp = current.clone();
            crate::thash::thash_internal(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child - compute h again
        let h_val = {
            let mut ii = idx;
            let mut il = leaf_idx;
            let mut h = 0u32;
            loop {
                if h == tree_height { break h; }
                if (ii & 1) == 0 && idx < max_idx { break h; }
                ii >>= 1;
                il >>= 1;
                h += 1;
            }
        };
        let hp = h_val as usize;
        stack[hp * SPX_N..(hp + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

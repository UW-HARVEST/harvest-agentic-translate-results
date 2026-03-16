use crate::params::*;
use crate::address::*;
use crate::hash::SpxCtx;
use crate::thash::thash_rs;
use crate::wotsx1::{LeafInfoX1, wots_gen_leafx1_rs};
use crate::fors::{ForsGenLeafInfo, fors_gen_leafx1_rs};


// Actually, the above approach has issues with tracking h properly. Let me rewrite more faithfully.

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_treehashx1(
    root: *mut u8, auth_path: *mut u8, ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: *mut u32, info: *mut LeafInfoX1,
) {
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let info = unsafe { &mut *info };
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N) };
    wots_treehashx1_rs(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info);
}

pub fn wots_treehashx1_rs(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    do_treehashx1(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info, false);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_treehashx1(
    root: *mut u8, auth_path: *mut u8, ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: *mut u32, info: *mut LeafInfoX1,
) {
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let info = unsafe { &mut *info };
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N) };
    fors_treehashx1_rs(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info);
}

pub fn fors_treehashx1_rs(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    do_treehashx1(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info, true);
}

fn do_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
    is_fors: bool,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];

        if is_fors {
            let mut fors_info = ForsGenLeafInfo { leaf_addrx: info.leaf_addr };
            fors_gen_leafx1_rs(&mut current[SPX_N..], ctx, idx + idx_offset, &mut fors_info);
            info.leaf_addr = fors_info.leaf_addrx;
        } else {
            wots_gen_leafx1_rs(&mut current[SPX_N..], ctx, idx + idx_offset, info);
        }

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0u32;

        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            // Right node: combine with left from stack
            internal_idx_offset >>= 1;
            set_tree_height_rs(tree_addr, h + 1);
            set_tree_index_rs(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp = current;
            thash_rs(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Left child: save to stack
        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

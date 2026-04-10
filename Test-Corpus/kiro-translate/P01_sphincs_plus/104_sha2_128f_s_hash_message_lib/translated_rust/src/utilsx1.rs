use crate::params::*;
use crate::context::SpxCtx;
use crate::address::{set_tree_height, set_tree_index};
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

    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];

        wots_gen_leafx1(&mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset, info);

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

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let src = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..2 * SPX_N], &src, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
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
    info_ptr: *mut u8, // Actually *mut ForsGenLeafInfo, passed as opaque
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let info = unsafe { &mut *(info_ptr as *mut ForsGenLeafInfo) };

    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];

        fors_gen_leafx1(&mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset, info);

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

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let src = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..2 * SPX_N], &src, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

// --- extern "C" wrappers ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut LeafInfoX1,
) {
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N) };
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let info = unsafe { &mut *info };
    wots_treehashx1(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut u8,
) {
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N) };
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    fors_treehashx1(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info);
}

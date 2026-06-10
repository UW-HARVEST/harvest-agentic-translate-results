// utilsx1: wots_treehashx1 and fors_treehashx1 implementations

use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::thash::thash;
use crate::wotsx1::{LeafInfoX1, SPX_wots_gen_leafx1};
use crate::fors::SPX_fors_gen_leafx1;

pub fn wots_treehashx1_rs(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; (tree_height as usize) * SPX_N];

    let max_idx = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        unsafe {
            SPX_wots_gen_leafx1(
                current.as_mut_ptr().add(SPX_N),
                ctx as *const _,
                idx + idx_offset,
                info as *mut _,
            );
        }

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
                auth_path[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let h_idx = h as usize;
            current[..SPX_N].copy_from_slice(&stack[h_idx * SPX_N..(h_idx + 1) * SPX_N]);
            // thash(&current[1*SPX_N], &current[0], 2, ctx, tree_addr)
            let mut input = vec![0u8; 2 * SPX_N];
            input.copy_from_slice(&current[..2 * SPX_N]);
            let mut out = vec![0u8; SPX_N];
            thash(&mut out, &input, 2, ctx, tree_addr);
            current[SPX_N..2 * SPX_N].copy_from_slice(&out);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h_idx = h as usize;
        stack[h_idx * SPX_N..(h_idx + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut [u32; 8],
    info: *mut LeafInfoX1,
) {
    let root_slice = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_slice =
        unsafe { core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let tree_addr_ref = unsafe { &mut *tree_addr };
    let info_ref = unsafe { &mut *info };
    wots_treehashx1_rs(
        root_slice,
        auth_slice,
        ctx_ref,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr_ref,
        info_ref,
    );
}

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

pub fn fors_treehashx1_rs(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
) {
    let mut stack = vec![0u8; (tree_height as usize) * SPX_N];

    let max_idx = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
        unsafe {
            SPX_fors_gen_leafx1(
                current.as_mut_ptr().add(SPX_N),
                ctx as *const _,
                idx + idx_offset,
                info as *mut _,
            );
        }

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
                auth_path[(h as usize) * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let h_idx = h as usize;
            current[..SPX_N].copy_from_slice(&stack[h_idx * SPX_N..(h_idx + 1) * SPX_N]);
            let mut input = vec![0u8; 2 * SPX_N];
            input.copy_from_slice(&current[..2 * SPX_N]);
            let mut out = vec![0u8; SPX_N];
            thash(&mut out, &input, 2, ctx, tree_addr);
            current[SPX_N..2 * SPX_N].copy_from_slice(&out);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h_idx = h as usize;
        stack[h_idx * SPX_N..(h_idx + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut [u32; 8],
    info: *mut ForsGenLeafInfo,
) {
    let root_slice = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_slice =
        unsafe { core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let tree_addr_ref = unsafe { &mut *tree_addr };
    let info_ref = unsafe { &mut *info };
    fors_treehashx1_rs(
        root_slice,
        auth_slice,
        ctx_ref,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr_ref,
        info_ref,
    );
}

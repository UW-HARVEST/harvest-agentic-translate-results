use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::fors::{ForsGenLeafInfo, SPX_fors_gen_leafx1};
use crate::params::SPX_N;
use crate::thash::thash;
use crate::wotsx1::{wots_gen_leafx1, LeafInfoX1};

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
    unsafe {
        let mut stack = vec![0u8; tree_height as usize * SPX_N];
        let max_idx = (1u32 << tree_height) - 1;

        let mut idx: u32 = 0;
        loop {
            let mut current = [0u8; 2 * SPX_N];

            wots_gen_leafx1(
                current.as_mut_ptr().add(SPX_N),
                ctx,
                idx + idx_offset,
                info,
            );

            let mut internal_idx_offset = idx_offset;
            let mut internal_idx = idx;
            let mut internal_leaf = leaf_idx;

            let mut h: u32 = 0;
            loop {
                if h == tree_height {
                    std::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                    return;
                }

                if (internal_idx ^ internal_leaf) == 0x01 {
                    std::ptr::copy_nonoverlapping(
                        current.as_ptr().add(SPX_N),
                        auth_path.add(h as usize * SPX_N),
                        SPX_N,
                    );
                }

                if (internal_idx & 1) == 0 && idx < max_idx {
                    break;
                }

                internal_idx_offset >>= 1;
                set_tree_height(tree_addr, h + 1);
                set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

                let left = stack.as_ptr().add(h as usize * SPX_N);
                std::ptr::copy_nonoverlapping(left, current.as_mut_ptr(), SPX_N);
                thash(
                    current.as_mut_ptr().add(SPX_N),
                    current.as_ptr(),
                    2,
                    ctx,
                    tree_addr,
                );

                h += 1;
                internal_idx >>= 1;
                internal_leaf >>= 1;
            }

            std::ptr::copy_nonoverlapping(
                current.as_ptr().add(SPX_N),
                stack.as_mut_ptr().add(h as usize * SPX_N),
                SPX_N,
            );

            idx += 1;
        }
    }
}

pub fn wots_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut LeafInfoX1,
) {
    SPX_wots_treehashx1(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr, info);
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
    info: *mut ForsGenLeafInfo,
) {
    unsafe {
        let mut stack = vec![0u8; tree_height as usize * SPX_N];
        let max_idx = (1u32 << tree_height) - 1;

        let mut idx: u32 = 0;
        loop {
            let mut current = [0u8; 2 * SPX_N];

            SPX_fors_gen_leafx1(
                current.as_mut_ptr().add(SPX_N),
                ctx,
                idx + idx_offset,
                info,
            );

            let mut internal_idx_offset = idx_offset;
            let mut internal_idx = idx;
            let mut internal_leaf = leaf_idx;

            let mut h: u32 = 0;
            loop {
                if h == tree_height {
                    std::ptr::copy_nonoverlapping(current.as_ptr().add(SPX_N), root, SPX_N);
                    return;
                }

                if (internal_idx ^ internal_leaf) == 0x01 {
                    std::ptr::copy_nonoverlapping(
                        current.as_ptr().add(SPX_N),
                        auth_path.add(h as usize * SPX_N),
                        SPX_N,
                    );
                }

                if (internal_idx & 1) == 0 && idx < max_idx {
                    break;
                }

                internal_idx_offset >>= 1;
                set_tree_height(tree_addr, h + 1);
                set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

                let left = stack.as_ptr().add(h as usize * SPX_N);
                std::ptr::copy_nonoverlapping(left, current.as_mut_ptr(), SPX_N);
                thash(
                    current.as_mut_ptr().add(SPX_N),
                    current.as_ptr(),
                    2,
                    ctx,
                    tree_addr,
                );

                h += 1;
                internal_idx >>= 1;
                internal_leaf >>= 1;
            }

            std::ptr::copy_nonoverlapping(
                current.as_ptr().add(SPX_N),
                stack.as_mut_ptr().add(h as usize * SPX_N),
                SPX_N,
            );

            idx += 1;
        }
    }
}

pub fn fors_treehashx1(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: *mut u32,
    info: *mut u8,
) {
    SPX_fors_treehashx1(
        root,
        auth_path,
        ctx,
        leaf_idx,
        idx_offset,
        tree_height,
        tree_addr,
        info as *mut ForsGenLeafInfo,
    );
}

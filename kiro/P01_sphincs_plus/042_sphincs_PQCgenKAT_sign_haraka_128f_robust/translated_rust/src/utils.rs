use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;

extern "C" {
    fn SPX_thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32);
}

pub unsafe fn thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_thash(out, in_, inblocks, ctx as *const SpxCtx, addr.as_mut_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, mut in_: u64) {
    let mut i = outlen as i32 - 1;
    while i >= 0 {
        *out.add(i as usize) = (in_ & 0xff) as u8;
        in_ >>= 8;
        i -= 1;
    }
}

pub unsafe fn ull_to_bytes(out: *mut u8, outlen: u32, in_: u64) {
    SPX_ull_to_bytes(out, outlen, in_);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_u32_to_bytes(out: *mut u8, in_: u32) {
    *out.add(0) = (in_ >> 24) as u8;
    *out.add(1) = (in_ >> 16) as u8;
    *out.add(2) = (in_ >> 8) as u8;
    *out.add(3) = in_ as u8;
}

pub unsafe fn u32_to_bytes(out: *mut u8, in_: u32) {
    SPX_u32_to_bytes(out, in_);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_bytes_to_ull(in_: *const u8, inlen: u32) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (*in_.add(i as usize) as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub unsafe fn bytes_to_ull(in_: *const u8, inlen: u32) -> u64 {
    SPX_bytes_to_ull(in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    mut leaf_idx: u32,
    mut idx_offset: u32,
    mut auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let addr_ref = &mut *(addr as *mut [u32; 8]);

    if leaf_idx & 1 != 0 {
        core::ptr::copy_nonoverlapping(leaf, buffer.as_mut_ptr().add(SPX_N), SPX_N);
        core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr(), SPX_N);
    } else {
        core::ptr::copy_nonoverlapping(leaf, buffer.as_mut_ptr(), SPX_N);
        core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr().add(SPX_N), SPX_N);
    }
    auth_path = auth_path.add(SPX_N);

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr_ref, i + 1);
        set_tree_index(addr_ref, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            thash(buffer.as_mut_ptr().add(SPX_N), buffer.as_ptr(), 2, &*ctx, addr_ref);
            core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr(), SPX_N);
        } else {
            thash(buffer.as_mut_ptr(), buffer.as_ptr(), 2, &*ctx, addr_ref);
            core::ptr::copy_nonoverlapping(auth_path, buffer.as_mut_ptr().add(SPX_N), SPX_N);
        }
        auth_path = auth_path.add(SPX_N);
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr_ref, tree_height);
    set_tree_index(addr_ref, leaf_idx + idx_offset);
    thash(root, buffer.as_ptr(), 2, &*ctx, addr_ref);
}

pub unsafe fn compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    SPX_compute_root(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, ctx as *const SpxCtx, addr.as_mut_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32),
    tree_addr: *mut u32,
) {
    let stack_size = (tree_height as usize + 1) * SPX_N;
    let mut stack = vec![0u8; stack_size];
    let mut heights = vec![0u32; tree_height as usize + 1];
    let mut offset: u32 = 0;
    let addr_ref = &mut *(tree_addr as *mut [u32; 8]);

    for idx in 0..(1u32 << tree_height) {
        gen_leaf(
            stack.as_mut_ptr().add(offset as usize * SPX_N),
            ctx,
            idx + idx_offset,
            tree_addr,
        );
        offset += 1;
        heights[offset as usize - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            core::ptr::copy_nonoverlapping(
                stack.as_ptr().add((offset as usize - 1) * SPX_N),
                auth_path,
                SPX_N,
            );
        }

        while offset >= 2 && heights[offset as usize - 1] == heights[offset as usize - 2] {
            let tree_idx = idx >> (heights[offset as usize - 1] + 1);

            set_tree_height(addr_ref, heights[offset as usize - 1] + 1);
            set_tree_index(
                addr_ref,
                tree_idx + (idx_offset >> (heights[offset as usize - 1] + 1)),
            );

            thash(
                stack.as_mut_ptr().add((offset as usize - 2) * SPX_N),
                stack.as_ptr().add((offset as usize - 2) * SPX_N),
                2,
                &*ctx,
                addr_ref,
            );
            offset -= 1;
            heights[offset as usize - 1] += 1;

            if ((leaf_idx >> heights[offset as usize - 1]) ^ 0x1) == tree_idx {
                core::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset as usize - 1) * SPX_N),
                    auth_path.add(heights[offset as usize - 1] as usize * SPX_N),
                    SPX_N,
                );
            }
        }
    }
    core::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
}

use crate::address::{set_tree_height, set_tree_index};
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, mut val: u64) {
    unsafe {
        let mut i = outlen as i32 - 1;
        while i >= 0 {
            *out.add(i as usize) = (val & 0xff) as u8;
            val >>= 8;
            i -= 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    unsafe {
        *out.add(0) = (val >> 24) as u8;
        *out.add(1) = (val >> 16) as u8;
        *out.add(2) = (val >> 8) as u8;
        *out.add(3) = val as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    unsafe {
        let mut retval: u64 = 0;
        for i in 0..inlen as usize {
            retval |= (*inp.add(i) as u64) << (8 * (inlen as usize - 1 - i));
        }
        retval
    }
}

// Rust-callable wrappers
pub fn ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    SPX_ull_to_bytes(out, outlen, val);
}

pub fn u32_to_bytes(out: *mut u8, val: u32) {
    SPX_u32_to_bytes(out, val);
}

pub fn bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    SPX_bytes_to_ull(inp, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let mut buffer = [0u8; 2 * SPX_N];
        let mut auth_ptr = auth_path;

        if leaf_idx & 1 != 0 {
            std::ptr::copy_nonoverlapping(leaf, buffer.as_mut_ptr().add(SPX_N), SPX_N);
            std::ptr::copy_nonoverlapping(auth_ptr, buffer.as_mut_ptr(), SPX_N);
        } else {
            std::ptr::copy_nonoverlapping(leaf, buffer.as_mut_ptr(), SPX_N);
            std::ptr::copy_nonoverlapping(auth_ptr, buffer.as_mut_ptr().add(SPX_N), SPX_N);
        }
        auth_ptr = auth_ptr.add(SPX_N);

        for i in 0..tree_height - 1 {
            leaf_idx >>= 1;
            idx_offset >>= 1;
            set_tree_height(addr, i + 1);
            set_tree_index(addr, leaf_idx + idx_offset);

            if leaf_idx & 1 != 0 {
                thash(
                    buffer.as_mut_ptr().add(SPX_N),
                    buffer.as_ptr(),
                    2,
                    ctx,
                    addr,
                );
                std::ptr::copy_nonoverlapping(auth_ptr, buffer.as_mut_ptr(), SPX_N);
            } else {
                thash(buffer.as_mut_ptr(), buffer.as_ptr(), 2, ctx, addr);
                std::ptr::copy_nonoverlapping(auth_ptr, buffer.as_mut_ptr().add(SPX_N), SPX_N);
            }
            auth_ptr = auth_ptr.add(SPX_N);
        }

        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, tree_height);
        set_tree_index(addr, leaf_idx + idx_offset);
        thash(root, buffer.as_ptr(), 2, ctx, addr);
    }
}

pub fn compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    SPX_compute_root(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32),
    tree_addr: *mut u32,
) {
    unsafe {
        let stack_size = (tree_height as usize + 1) * SPX_N;
        let mut stack = vec![0u8; stack_size];
        let mut heights = vec![0u32; tree_height as usize + 1];
        let mut offset: u32 = 0;

        for idx in 0..(1u32 << tree_height) {
            gen_leaf(
                stack.as_mut_ptr().add(offset as usize * SPX_N),
                ctx,
                idx + idx_offset,
                tree_addr as *const u32,
            );
            offset += 1;
            heights[offset as usize - 1] = 0;

            if (leaf_idx ^ 0x1) == idx {
                std::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset as usize - 1) * SPX_N),
                    auth_path,
                    SPX_N,
                );
            }

            while offset >= 2
                && heights[offset as usize - 1] == heights[offset as usize - 2]
            {
                let tree_idx = idx >> (heights[offset as usize - 1] + 1);

                set_tree_height(tree_addr, heights[offset as usize - 1] + 1);
                set_tree_index(
                    tree_addr,
                    tree_idx + (idx_offset >> (heights[offset as usize - 1] + 1)),
                );

                thash(
                    stack.as_mut_ptr().add((offset as usize - 2) * SPX_N),
                    stack.as_ptr().add((offset as usize - 2) * SPX_N),
                    2,
                    ctx,
                    tree_addr,
                );
                offset -= 1;
                heights[offset as usize - 1] += 1;

                if ((leaf_idx >> heights[offset as usize - 1]) ^ 0x1) == tree_idx {
                    std::ptr::copy_nonoverlapping(
                        stack.as_ptr().add((offset as usize - 1) * SPX_N),
                        auth_path.add(heights[offset as usize - 1] as usize * SPX_N),
                        SPX_N,
                    );
                }
            }
        }
        std::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
    }
}

pub fn treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32),
    tree_addr: *mut u32,
) {
    SPX_treehash(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, gen_leaf, tree_addr);
}

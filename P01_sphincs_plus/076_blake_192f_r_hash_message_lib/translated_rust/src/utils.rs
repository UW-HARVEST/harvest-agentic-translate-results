use crate::params::*;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// --- Exported C functions ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(
    out: *mut u8, outlen: std::ffi::c_uint, val: std::ffi::c_ulonglong,
) {
    let s = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes(s, outlen as usize, val as u64);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let s = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes(s, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(
    inp: *const u8, inlen: std::ffi::c_uint,
) -> std::ffi::c_ulonglong {
    let s = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    bytes_to_ull(s, inlen as usize) as std::ffi::c_ulonglong
}

// compute_root and treehash use thash and address functions
// They are complex tree operations. We export them as C functions.

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8,
    leaf: *const u8,
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const crate::context::spx_ctx,
    addr: *mut u32,
) {
    unsafe {
        let ctx_ref = &*ctx;
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

        for i in 0..(tree_height - 1) {
            leaf_idx >>= 1;
            idx_offset >>= 1;
            crate::address::set_tree_height(addr, i + 1);
            crate::address::set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));

            if leaf_idx & 1 != 0 {
                crate::thash::thash(
                    buffer.as_mut_ptr().add(SPX_N),
                    buffer.as_ptr(),
                    2,
                    ctx_ref,
                    addr,
                );
                std::ptr::copy_nonoverlapping(auth_ptr, buffer.as_mut_ptr(), SPX_N);
            } else {
                crate::thash::thash(buffer.as_mut_ptr(), buffer.as_ptr(), 2, ctx_ref, addr);
                std::ptr::copy_nonoverlapping(auth_ptr, buffer.as_mut_ptr().add(SPX_N), SPX_N);
            }
            auth_ptr = auth_ptr.add(SPX_N);
        }

        leaf_idx >>= 1;
        idx_offset >>= 1;
        crate::address::set_tree_height(addr, tree_height);
        crate::address::set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
        crate::thash::thash(root, buffer.as_ptr(), 2, ctx_ref, addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const crate::context::spx_ctx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: unsafe extern "C" fn(*mut u8, *const crate::context::spx_ctx, u32, *const u32),
    tree_addr: *mut u32,
) {
    unsafe {
        let ctx_ref = &*ctx;
        let th = tree_height as usize;
        let mut stack = vec![0u8; (th + 1) * SPX_N];
        let mut heights = vec![0u32; th + 1];
        let mut offset: usize = 0;

        for idx in 0..(1u32 << tree_height) {
            gen_leaf(
                stack.as_mut_ptr().add(offset * SPX_N),
                ctx,
                idx.wrapping_add(idx_offset),
                tree_addr as *const u32,
            );
            offset += 1;
            heights[offset - 1] = 0;

            if (leaf_idx ^ 0x1) == idx {
                std::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset - 1) * SPX_N),
                    auth_path,
                    SPX_N,
                );
            }

            while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
                let tree_idx = idx >> (heights[offset - 1] + 1);
                crate::address::set_tree_height(tree_addr, heights[offset - 1] + 1);
                crate::address::set_tree_index(
                    tree_addr,
                    tree_idx.wrapping_add(idx_offset >> (heights[offset - 1] + 1)),
                );
                crate::thash::thash(
                    stack.as_mut_ptr().add((offset - 2) * SPX_N),
                    stack.as_ptr().add((offset - 2) * SPX_N),
                    2,
                    ctx_ref,
                    tree_addr,
                );
                offset -= 1;
                heights[offset - 1] += 1;

                if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                    std::ptr::copy_nonoverlapping(
                        stack.as_ptr().add((offset - 1) * SPX_N),
                        auth_path.add(heights[offset - 1] as usize * SPX_N),
                        SPX_N,
                    );
                }
            }
        }
        std::ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
    }
}

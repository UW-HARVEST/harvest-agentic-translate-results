use crate::address;
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;

/// Converts the value of `in` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut input: u64) {
    if outlen == 0 {
        return;
    }
    let mut i = (outlen as i64) - 1;
    while i >= 0 {
        out[i as usize] = (input & 0xff) as u8;
        input >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], input: u32) {
    out[0] = (input >> 24) as u8;
    out[1] = (input >> 16) as u8;
    out[2] = (input >> 8) as u8;
    out[3] = input as u8;
}

/// Converts inlen bytes from big-endian to an integer.
pub fn bytes_to_ull(input: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let n = SPX_N;
    let mut buffer = vec![0u8; 2 * n];
    let mut auth_pos: usize = 0;

    if leaf_idx & 1 != 0 {
        buffer[n..2 * n].copy_from_slice(&leaf[..n]);
        buffer[..n].copy_from_slice(&auth_path[..n]);
    } else {
        buffer[..n].copy_from_slice(&leaf[..n]);
        buffer[n..2 * n].copy_from_slice(&auth_path[..n]);
    }
    auth_pos += n;

    let mut i: u32 = 0;
    while i + 1 < tree_height {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        address::set_tree_height(addr, i + 1);
        address::set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp_in = buffer.clone();
            thash(&mut buffer[n..2 * n], &tmp_in, 2, ctx, addr);
            buffer[..n].copy_from_slice(&auth_path[auth_pos..auth_pos + n]);
        } else {
            let tmp_in = buffer.clone();
            thash(&mut buffer[..n], &tmp_in, 2, ctx, addr);
            buffer[n..2 * n].copy_from_slice(&auth_path[auth_pos..auth_pos + n]);
        }
        auth_pos += n;
        i += 1;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    address::set_tree_height(addr, tree_height);
    address::set_tree_index(addr, leaf_idx + idx_offset);
    let tmp_in = buffer.clone();
    thash(&mut root[..n], &tmp_in, 2, ctx, addr);
}

// ---------------------------------------------------------------------
// C-ABI exports (renamed to SPX_* to match the C linker symbols)
// ---------------------------------------------------------------------

#[unsafe(export_name = "SPX_ull_to_bytes")]
pub unsafe extern "C" fn spx_ull_to_bytes(
    out: *mut u8,
    outlen: core::ffi::c_uint,
    input: core::ffi::c_ulonglong,
) {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes(slice, outlen as usize, input);
}

#[unsafe(export_name = "SPX_u32_to_bytes")]
pub unsafe extern "C" fn spx_u32_to_bytes(out: *mut u8, input: u32) {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes(slice, input);
}

#[unsafe(export_name = "SPX_bytes_to_ull")]
pub unsafe extern "C" fn spx_bytes_to_ull(
    input: *const u8,
    inlen: core::ffi::c_uint,
) -> core::ffi::c_ulonglong {
    let slice = unsafe { core::slice::from_raw_parts(input, inlen as usize) };
    bytes_to_ull(slice, inlen as usize)
}

#[unsafe(export_name = "SPX_compute_root")]
pub unsafe extern "C" fn spx_compute_root(
    root: *mut u8,
    leaf: *const u8,
    leaf_idx: u32,
    idx_offset: u32,
    auth_path: *const u8,
    tree_height: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let root_slice = unsafe { core::slice::from_raw_parts_mut(root, SPX_N) };
    let leaf_slice = unsafe { core::slice::from_raw_parts(leaf, SPX_N) };
    let auth_slice = unsafe { core::slice::from_raw_parts(auth_path, SPX_N * tree_height as usize) };
    let addr_ref = unsafe { &mut *(addr as *mut [u32; 8]) };
    compute_root(
        root_slice,
        leaf_slice,
        leaf_idx,
        idx_offset,
        auth_slice,
        tree_height,
        unsafe { &*ctx },
        addr_ref,
    );
}

// Treehash: a generic tree-hash that takes a function pointer for leaf
// generation. Since the C callback signature is
//    void (*gen_leaf)(uint8_t*, const spx_ctx*, uint32_t, const uint32_t[8])
// we accept a function pointer and call it directly inside our copy of the
// algorithm.
type GenLeafFn = unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32);

unsafe fn treehash_inner(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: *mut u32,
) {
    let n = SPX_N;
    let h = tree_height as usize;
    let mut stack = vec![0u8; (h + 1) * n];
    let mut heights = vec![0u32; h + 1];
    let mut offset: usize = 0;

    for idx in 0..(1u32 << tree_height) {
        unsafe {
            gen_leaf(stack.as_mut_ptr().add(offset * n), ctx, idx + idx_offset, tree_addr);
        }
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    stack.as_ptr().add((offset - 1) * n),
                    auth_path,
                    n,
                );
            }
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            crate::address::set_tree_height(
                unsafe { &mut *(tree_addr as *mut [u32; 8]) },
                heights[offset - 1] + 1,
            );
            crate::address::set_tree_index(
                unsafe { &mut *(tree_addr as *mut [u32; 8]) },
                tree_idx + (idx_offset >> (heights[offset - 1] + 1)),
            );
            // thash with input==output pointer is what the C does.
            let in_ptr = stack.as_mut_ptr().add((offset - 2) * n);
            let in_slice = unsafe { core::slice::from_raw_parts(in_ptr, 2 * n) }.to_vec();
            let out_slice = unsafe { core::slice::from_raw_parts_mut(in_ptr, n) };
            crate::thash::thash(
                out_slice,
                &in_slice,
                2,
                unsafe { &*ctx },
                unsafe { &mut *(tree_addr as *mut [u32; 8]) },
            );
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        stack.as_ptr().add((offset - 1) * n),
                        auth_path.add(heights[offset - 1] as usize * n),
                        n,
                    );
                }
            }
        }
    }
    unsafe {
        core::ptr::copy_nonoverlapping(stack.as_ptr(), root, n);
    }
}

#[unsafe(export_name = "SPX_treehash")]
pub unsafe extern "C" fn spx_treehash(
    root: *mut u8,
    auth_path: *mut u8,
    ctx: *const SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    gen_leaf: GenLeafFn,
    tree_addr: *mut u32,
) {
    unsafe {
        treehash_inner(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, gen_leaf, tree_addr);
    }
}

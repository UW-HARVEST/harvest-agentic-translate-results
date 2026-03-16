use crate::params::*;
use crate::address::*;
use crate::hash::SpxCtx;
use crate::thash::thash_rs;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes_rs(out, val);
}

pub fn ull_to_bytes_rs(out: &mut [u8], mut val: u64) {
    let outlen = out.len();
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

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes(out, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let inp = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    bytes_to_ull_rs(inp, inlen as usize)
}

pub fn bytes_to_ull_rs(inp: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8, leaf: *const u8, leaf_idx: u32, idx_offset: u32,
    auth_path: *const u8, tree_height: u32, ctx: *const SpxCtx, addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &mut *(addr as *mut [u32; 8]) };
    let leaf = unsafe { std::slice::from_raw_parts(leaf, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N) };
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    compute_root_rs(root, leaf, leaf_idx, idx_offset, auth_path, tree_height, ctx, addr);
}

pub fn compute_root_rs(
    root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap_offset = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    ap_offset += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height_rs(addr, i + 1);
        set_tree_index_rs(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            thash_rs(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_offset..ap_offset + SPX_N]);
        } else {
            let tmp = buffer;
            thash_rs(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_offset..ap_offset + SPX_N]);
        }
        ap_offset += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height_rs(addr, tree_height);
    set_tree_index_rs(addr, leaf_idx + idx_offset);
    thash_rs(root, &buffer, 2, ctx, addr);
}

pub type GenLeafFn = fn(&mut [u8], &SpxCtx, u32, &[u32; 8]);

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8, auth_path: *mut u8, ctx: *const SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    gen_leaf: unsafe extern "C" fn(*mut u8, *const SpxCtx, u32, *const u32),
    tree_addr: *mut u32,
) {
    let ctx = unsafe { &*ctx };
    let tree_addr = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let root = unsafe { std::slice::from_raw_parts_mut(root, SPX_N) };
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path, tree_height as usize * SPX_N) };

    let th = tree_height as usize;
    let mut stack = vec![0u8; (th + 1) * SPX_N];
    let mut heights = vec![0u32; th + 1];
    let mut offset = 0usize;

    for idx in 0..(1u32 << tree_height) {
        unsafe { gen_leaf(stack[offset * SPX_N..].as_mut_ptr(), ctx, idx + idx_offset, tree_addr.as_ptr()); }
        offset += 1;
        heights[offset - 1] = 0;

        if (leaf_idx ^ 0x1) == idx {
            auth_path[..SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..(offset) * SPX_N]);
        }

        while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
            let tree_idx = idx >> (heights[offset - 1] + 1);
            set_tree_height_rs(tree_addr, heights[offset - 1] + 1);
            set_tree_index_rs(tree_addr, tree_idx + (idx_offset >> (heights[offset - 1] + 1)));
            let base = (offset - 2) * SPX_N;
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&stack[base..base + 2 * SPX_N]);
            thash_rs(&mut stack[base..base + SPX_N], &tmp, 2, ctx, tree_addr);
            offset -= 1;
            heights[offset - 1] += 1;

            if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                let h = heights[offset - 1] as usize;
                auth_path[h * SPX_N..(h + 1) * SPX_N].copy_from_slice(&stack[(offset - 1) * SPX_N..offset * SPX_N]);
            }
        }
    }
    root[..SPX_N].copy_from_slice(&stack[..SPX_N]);
}

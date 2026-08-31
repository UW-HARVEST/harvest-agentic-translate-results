//! Translation of c_src/libsodium/crypto_pwhash/argon2/argon2-fill-block-ref.c

use crate::common::rotr64;
use core::ffi::{c_int, c_void};

const ARGON2_BLOCK_SIZE: usize = 1024;
const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8; // 128
const ARGON2_SYNC_POINTS: u32 = 4;
const ARGON2_ADDRESSES_IN_BLOCK: u32 = 128;

const Argon2_id: c_int = 2;

// ---- shared #[repr(C)] types ----

#[repr(C)]
#[derive(Clone, Copy)]
struct block {
    v: [u64; ARGON2_QWORDS_IN_BLOCK],
}

#[repr(C)]
struct block_region {
    base: *mut c_void,
    memory: *mut block,
    size: usize,
}

#[repr(C)]
struct argon2_instance_t {
    region: *mut block_region,
    pseudo_rands: *mut u64,
    passes: u32,
    current_pass: u32,
    memory_blocks: u32,
    segment_length: u32,
    lane_length: u32,
    lanes: u32,
    threads: u32,
    type_: c_int, /* argon2_type */
    print_internals: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct argon2_position_t {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

extern "C" {
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

// ---- block helpers (argon2-core.h inline) ----

unsafe fn init_block_value(b: *mut block, in_: u8) {
    memset(
        (*b).v.as_mut_ptr() as *mut c_void,
        in_ as c_int,
        core::mem::size_of::<[u64; ARGON2_QWORDS_IN_BLOCK]>(),
    );
}

unsafe fn copy_block(dst: *mut block, src: *const block) {
    core::ptr::copy_nonoverlapping(
        (*src).v.as_ptr(),
        (*dst).v.as_mut_ptr(),
        ARGON2_QWORDS_IN_BLOCK,
    );
}

unsafe fn xor_block(dst: *mut block, src: *const block) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] ^= (*src).v[i];
        i += 1;
    }
}

// index_alpha() from argon2-core.h (static, duplicated per translation unit)
unsafe fn index_alpha(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rand: u32,
    same_lane: c_int,
) -> u32 {
    let reference_area_size: u32;
    let mut relative_position: u64;
    let mut absolute_position: u64;
    let start_position: u32;

    if (*position).pass == 0 {
        if (*position).slice == 0 {
            reference_area_size = (*position).index.wrapping_sub(1);
        } else {
            if same_lane != 0 {
                reference_area_size = ((*position).slice as u32)
                    .wrapping_mul((*instance).segment_length)
                    .wrapping_add((*position).index)
                    .wrapping_sub(1);
            } else {
                reference_area_size = ((*position).slice as u32)
                    .wrapping_mul((*instance).segment_length)
                    .wrapping_add(if (*position).index == 0 {
                        (-1i32) as u32
                    } else {
                        0
                    });
            }
        }
    } else {
        if same_lane != 0 {
            reference_area_size = (*instance)
                .lane_length
                .wrapping_sub((*instance).segment_length)
                .wrapping_add((*position).index)
                .wrapping_sub(1);
        } else {
            reference_area_size = (*instance)
                .lane_length
                .wrapping_sub((*instance).segment_length)
                .wrapping_add(if (*position).index == 0 {
                    (-1i32) as u32
                } else {
                    0
                });
        }
    }

    relative_position = pseudo_rand as u64;
    relative_position = (relative_position.wrapping_mul(relative_position)) >> 32;
    relative_position = (reference_area_size as u64)
        .wrapping_sub(1)
        .wrapping_sub((reference_area_size as u64).wrapping_mul(relative_position) >> 32);

    let start_pos_tmp: u32;
    if (*position).pass != 0 {
        start_pos_tmp = if (*position).slice as u32 == ARGON2_SYNC_POINTS - 1 {
            0
        } else {
            ((*position).slice as u32 + 1).wrapping_mul((*instance).segment_length)
        };
    } else {
        start_pos_tmp = 0;
    }
    start_position = start_pos_tmp;

    absolute_position = (start_position as u64)
        .wrapping_add(relative_position)
        .wrapping_sub((*instance).lane_length as u64);
    absolute_position =
        absolute_position.wrapping_add((*instance).lane_length as u64 & (absolute_position >> 32));
    absolute_position as u32
}

// ---- blamka-round-ref.h ----

#[inline(always)]
fn f_bla_mka(x: u64, y: u64) -> u64 {
    let m: u64 = 0xFFFFFFFF;
    let xy: u64 = (x & m).wrapping_mul(y & m);
    x.wrapping_add(y).wrapping_add(2u64.wrapping_mul(xy))
}

// G(a, b, c, d) macro
#[inline(always)]
fn g(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
    *a = f_bla_mka(*a, *b);
    *d = rotr64(*d ^ *a, 32);
    *c = f_bla_mka(*c, *d);
    *b = rotr64(*b ^ *c, 24);
    *a = f_bla_mka(*a, *b);
    *d = rotr64(*d ^ *a, 16);
    *c = f_bla_mka(*c, *d);
    *b = rotr64(*b ^ *c, 63);
}

// BLAKE2_ROUND_NOMSG(v0..v15) macro operating on 16 slots identified by `idx`.
#[inline(always)]
unsafe fn blake2_round_nomsg(v: *mut u64, idx: [usize; 16]) {
    // All 16 indices are distinct, so raw-pointer access mirrors the C's
    // in-place reads/writes of block words.
    macro_rules! gm {
        ($ia:expr, $ib:expr, $ic:expr, $id:expr) => {{
            let pa = v.add(idx[$ia]);
            let pb = v.add(idx[$ib]);
            let pc = v.add(idx[$ic]);
            let pd = v.add(idx[$id]);
            let mut a = *pa;
            let mut b = *pb;
            let mut c = *pc;
            let mut d = *pd;
            g(&mut a, &mut b, &mut c, &mut d);
            *pa = a;
            *pb = b;
            *pc = c;
            *pd = d;
        }};
    }
    gm!(0, 4, 8, 12);
    gm!(1, 5, 9, 13);
    gm!(2, 6, 10, 14);
    gm!(3, 7, 11, 15);
    gm!(0, 5, 10, 15);
    gm!(1, 6, 11, 12);
    gm!(2, 7, 8, 13);
    gm!(3, 4, 9, 14);
}

unsafe fn fill_block(prev_block: *const block, ref_block: *const block, next_block: *mut block) {
    let mut block_r: block = core::mem::zeroed();
    let mut block_tmp: block = core::mem::zeroed();
    let mut i: usize;

    copy_block(&mut block_r, ref_block);
    xor_block(&mut block_r, prev_block);
    copy_block(&mut block_tmp, &block_r);

    /* columns */
    i = 0;
    while i < 8 {
        let base = 16 * i;
        blake2_round_nomsg(
            block_r.v.as_mut_ptr(),
            [
                base,
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6,
                base + 7,
                base + 8,
                base + 9,
                base + 10,
                base + 11,
                base + 12,
                base + 13,
                base + 14,
                base + 15,
            ],
        );
        i += 1;
    }

    /* rows */
    i = 0;
    while i < 8 {
        let b = 2 * i;
        blake2_round_nomsg(
            block_r.v.as_mut_ptr(),
            [
                b,
                b + 1,
                b + 16,
                b + 17,
                b + 32,
                b + 33,
                b + 48,
                b + 49,
                b + 64,
                b + 65,
                b + 80,
                b + 81,
                b + 96,
                b + 97,
                b + 112,
                b + 113,
            ],
        );
        i += 1;
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &block_r);
}

unsafe fn fill_block_with_xor(
    prev_block: *const block,
    ref_block: *const block,
    next_block: *mut block,
) {
    let mut block_r: block = core::mem::zeroed();
    let mut block_tmp: block = core::mem::zeroed();
    let mut i: usize;

    copy_block(&mut block_r, ref_block);
    xor_block(&mut block_r, prev_block);
    copy_block(&mut block_tmp, &block_r);
    xor_block(&mut block_tmp, next_block); /* Saving the next block contents for XOR over */

    /* columns */
    i = 0;
    while i < 8 {
        let base = 16 * i;
        blake2_round_nomsg(
            block_r.v.as_mut_ptr(),
            [
                base,
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6,
                base + 7,
                base + 8,
                base + 9,
                base + 10,
                base + 11,
                base + 12,
                base + 13,
                base + 14,
                base + 15,
            ],
        );
        i += 1;
    }

    /* rows */
    i = 0;
    while i < 8 {
        let b = 2 * i;
        blake2_round_nomsg(
            block_r.v.as_mut_ptr(),
            [
                b,
                b + 1,
                b + 16,
                b + 17,
                b + 32,
                b + 33,
                b + 48,
                b + 49,
                b + 64,
                b + 65,
                b + 80,
                b + 81,
                b + 96,
                b + 97,
                b + 112,
                b + 113,
            ],
        );
        i += 1;
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &block_r);
}

unsafe fn generate_addresses(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rands: *mut u64,
) {
    let mut zero_block: block = core::mem::zeroed();
    let mut input_block: block = core::mem::zeroed();
    let mut address_block: block = core::mem::zeroed();
    let mut tmp_block: block = core::mem::zeroed();
    let mut i: u32;

    init_block_value(&mut zero_block, 0);
    init_block_value(&mut input_block, 0);

    if !instance.is_null() && !position.is_null() {
        input_block.v[0] = (*position).pass as u64;
        input_block.v[1] = (*position).lane as u64;
        input_block.v[2] = (*position).slice as u64;
        input_block.v[3] = (*instance).memory_blocks as u64;
        input_block.v[4] = (*instance).passes as u64;
        input_block.v[5] = (*instance).type_ as u64;

        i = 0;
        while i < (*instance).segment_length {
            if i % ARGON2_ADDRESSES_IN_BLOCK == 0 {
                input_block.v[6] = input_block.v[6].wrapping_add(1);
                init_block_value(&mut tmp_block, 0);
                init_block_value(&mut address_block, 0);
                fill_block_with_xor(&zero_block, &input_block, &mut tmp_block);
                fill_block_with_xor(&zero_block, &tmp_block, &mut address_block);
            }

            *pseudo_rands.add(i as usize) =
                address_block.v[(i % ARGON2_ADDRESSES_IN_BLOCK) as usize];
            i = i.wrapping_add(1);
        }
    }
}

// argon2_fill_segment_ref -> _sodium_argon2_fill_segment_ref
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_segment_ref(
    instance: *const argon2_instance_t,
    mut position: argon2_position_t,
) {
    let mut ref_block: *mut block;
    let mut curr_block: *mut block;
    let pseudo_rands: *mut u64;
    let mut pseudo_rand: u64;
    let mut ref_index: u64;
    let mut ref_lane: u64;
    let mut prev_offset: u32;
    let mut curr_offset: u32;
    let starting_index: u32;
    let mut i: u32;
    let mut data_independent_addressing: c_int = 1;

    if instance.is_null() {
        return;
    }

    if (*instance).type_ == Argon2_id
        && (position.pass != 0 || (position.slice as u32) >= ARGON2_SYNC_POINTS / 2)
    {
        data_independent_addressing = 0;
    }

    pseudo_rands = (*instance).pseudo_rands;

    if data_independent_addressing != 0 {
        generate_addresses(instance, &position, pseudo_rands);
    }

    let mut si: u32 = 0;
    if 0 == position.pass && 0 == position.slice {
        si = 2; /* we have already generated the first two blocks */
    }
    starting_index = si;

    /* Offset of the current block */
    curr_offset = position
        .lane
        .wrapping_mul((*instance).lane_length)
        .wrapping_add((position.slice as u32).wrapping_mul((*instance).segment_length))
        .wrapping_add(starting_index);

    if 0 == curr_offset % (*instance).lane_length {
        /* Last block in this lane */
        prev_offset = curr_offset
            .wrapping_add((*instance).lane_length)
            .wrapping_sub(1);
    } else {
        /* Previous block */
        prev_offset = curr_offset.wrapping_sub(1);
    }

    i = starting_index;
    while i < (*instance).segment_length {
        /*1.1 Rotating prev_offset if needed */
        if curr_offset % (*instance).lane_length == 1 {
            prev_offset = curr_offset.wrapping_sub(1);
        }

        /* 1.2.1 Taking pseudo-random value from the previous block */
        if data_independent_addressing != 0 {
            pseudo_rand = *pseudo_rands.add(i as usize);
        } else {
            pseudo_rand = (*(*(*instance).region).memory.add(prev_offset as usize)).v[0];
        }

        /* 1.2.2 Computing the lane of the reference block */
        ref_lane = (pseudo_rand >> 32) % (*instance).lanes as u64;

        if position.pass == 0 && position.slice == 0 {
            /* Can not reference other lanes yet */
            ref_lane = position.lane as u64;
        }

        /* 1.2.3 Computing the number of possible reference block within the lane */
        position.index = i;
        ref_index = index_alpha(
            instance,
            &position,
            (pseudo_rand & 0xFFFFFFFF) as u32,
            (ref_lane == position.lane as u64) as c_int,
        ) as u64;

        /* 2 Creating a new block */
        ref_block = (*(*instance).region)
            .memory
            .add(((*instance).lane_length as u64 * ref_lane + ref_index) as usize);
        curr_block = (*(*instance).region).memory.add(curr_offset as usize);
        if position.pass != 0 {
            fill_block_with_xor(
                (*(*instance).region).memory.add(prev_offset as usize),
                ref_block,
                curr_block,
            );
        } else {
            fill_block(
                (*(*instance).region).memory.add(prev_offset as usize),
                ref_block,
                curr_block,
            );
        }

        i = i.wrapping_add(1);
        curr_offset = curr_offset.wrapping_add(1);
        prev_offset = prev_offset.wrapping_add(1);
    }
    let _ = curr_block;
    let _ = ref_block;
}

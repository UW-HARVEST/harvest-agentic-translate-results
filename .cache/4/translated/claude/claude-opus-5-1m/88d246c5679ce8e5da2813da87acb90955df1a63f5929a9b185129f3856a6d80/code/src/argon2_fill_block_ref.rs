//! Translation of `crypto_pwhash/argon2/argon2-fill-block-ref.c`.
//!
//! Exports (after the `private/quirks.h` renaming):
//!   * `_sodium_argon2_fill_segment_ref`
//!
//! The `fBlaMka` / `G` / `BLAKE2_ROUND_NOMSG` macros come from
//! `blamka-round-ref.h`; `index_alpha()`, `init_block_value()`, `copy_block()`
//! and `xor_block()` are `static`/`static inline` helpers of `argon2-core.h`
//! and are duplicated here as private items.

use crate::common::*;
use core::ffi::c_int;

/* ---------------------------------------------------------------- argon2.h */

/* Number of synchronization points between lanes per pass */
const ARGON2_SYNC_POINTS: u32 = 4;

/* typedef enum Argon2_type { Argon2_i = 1, Argon2_id = 2 } argon2_type; */
pub type argon2_type = c_int;
const Argon2_id: argon2_type = 2;

/* ----------------------------------------------------------- argon2-core.h */

/* enum argon2_ctx_constants */
const ARGON2_BLOCK_SIZE: usize = 1024;
const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8;
const ARGON2_ADDRESSES_IN_BLOCK: u32 = 128;

/* typedef struct block_ { uint64_t v[ARGON2_QWORDS_IN_BLOCK]; } block; */
#[repr(C)]
pub struct block {
    pub v: [u64; ARGON2_QWORDS_IN_BLOCK],
}

/* typedef struct block_region_ (argon2-core.h); size 24, align 8 */
#[repr(C)]
pub struct block_region {
    pub base: *mut core::ffi::c_void,
    pub memory: *mut block,
    pub size: usize,
}

/* typedef struct Argon2_instance_t (argon2-core.h); size 56, align 8 */
#[repr(C)]
pub struct argon2_instance_t {
    pub region: *mut block_region, /* Memory region pointer */
    pub pseudo_rands: *mut u64,
    pub passes: u32, /* Number of passes */
    pub current_pass: u32,
    pub memory_blocks: u32, /* Number of blocks in memory */
    pub segment_length: u32,
    pub lane_length: u32,
    pub lanes: u32,
    pub threads: u32,
    pub type_: argon2_type,
    pub print_internals: c_int, /* whether to print the memory blocks */
}

/* typedef struct Argon2_position_t (argon2-core.h); size 16, align 4 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct argon2_position_t {
    pub pass: u32,
    pub lane: u32,
    pub slice: u8,
    pub index: u32,
}

/* --------------------------------------------- static inline block helpers */

/* static inline void init_block_value(block *b, uint8_t in) */
#[inline(always)]
unsafe fn init_block_value(b: *mut block, in_: u8) {
    memset(
        core::ptr::addr_of_mut!((*b).v) as *mut u8,
        in_,
        8usize * ARGON2_QWORDS_IN_BLOCK,
    );
}

/* static inline void copy_block(block *dst, const block *src) */
#[inline(always)]
unsafe fn copy_block(dst: *mut block, src: *const block) {
    memcpy(
        core::ptr::addr_of_mut!((*dst).v) as *mut u8,
        core::ptr::addr_of!((*src).v) as *const u8,
        8usize * ARGON2_QWORDS_IN_BLOCK,
    );
}

/* static inline void xor_block(block *dst, const block *src) */
#[inline(always)]
unsafe fn xor_block(dst: *mut block, src: *const block) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] ^= (*src).v[i];
        i += 1;
    }
}

/* -------------------------------------------------- index_alpha (core.h)   */

/* static uint32_t index_alpha(const argon2_instance_t *instance,
 *                             const argon2_position_t *position,
 *                             uint32_t pseudo_rand, int same_lane) */
unsafe fn index_alpha(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rand: u32,
    same_lane: c_int,
) -> u32 {
    /*
     * Pass 0:
     *      This lane : all already finished segments plus already constructed
     * blocks in this segment
     *      Other lanes : all already finished segments
     * Pass 1+:
     *      This lane : (SYNC_POINTS - 1) last segments plus already constructed
     * blocks in this segment
     *      Other lanes : (SYNC_POINTS - 1) last segments
     */
    let reference_area_size: u32;
    let mut relative_position: u64;
    let mut absolute_position: u64;
    let mut start_position: u32;

    if (*position).pass == 0 {
        /* First pass */
        if (*position).slice == 0 {
            /* First slice */
            reference_area_size = (*position).index.wrapping_sub(1); /* all but the previous */
        } else {
            if same_lane != 0 {
                /* The same lane => add current segment */
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
        /* Second pass */
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

    /* 1.2.4. Mapping pseudo_rand to 0..<reference_area_size-1> and produce
     * relative position */
    relative_position = pseudo_rand as u64;
    relative_position = relative_position.wrapping_mul(relative_position) >> 32;
    relative_position = (reference_area_size.wrapping_sub(1) as u64)
        .wrapping_sub((reference_area_size as u64).wrapping_mul(relative_position) >> 32);

    /* 1.2.5 Computing starting position */
    start_position = 0;

    if (*position).pass != 0 {
        start_position = if ((*position).slice as u32) == ARGON2_SYNC_POINTS - 1 {
            0
        } else {
            ((*position).slice as u32)
                .wrapping_add(1)
                .wrapping_mul((*instance).segment_length)
        };
    }

    /* 1.2.6. Computing absolute position */
    absolute_position = (start_position as u64)
        .wrapping_add(relative_position)
        .wrapping_sub((*instance).lane_length as u64);
    absolute_position = absolute_position
        .wrapping_add(((*instance).lane_length as u64) & (absolute_position >> 32));

    absolute_position as u32
}

/* ------------------------------------------------- blamka-round-ref.h      */

/* designed by the Lyra PHC team */
#[inline(always)]
fn fBlaMka(x: u64, y: u64) -> u64 {
    let m: u64 = 0xFFFF_FFFF;
    let xy: u64 = (x & m).wrapping_mul(y & m);
    x.wrapping_add(y).wrapping_add(2u64.wrapping_mul(xy))
}

/* #define G(a, b, c, d) ... */
#[inline(always)]
fn G(v: &mut [u64; ARGON2_QWORDS_IN_BLOCK], ia: usize, ib: usize, ic: usize, id: usize) {
    let mut a = v[ia];
    let mut b = v[ib];
    let mut c = v[ic];
    let mut d = v[id];

    a = fBlaMka(a, b);
    d = rotr64(d ^ a, 32);
    c = fBlaMka(c, d);
    b = rotr64(b ^ c, 24);
    a = fBlaMka(a, b);
    d = rotr64(d ^ a, 16);
    c = fBlaMka(c, d);
    b = rotr64(b ^ c, 63);

    v[ia] = a;
    v[ib] = b;
    v[ic] = c;
    v[id] = d;
}

/* #define BLAKE2_ROUND_NOMSG(v0, ..., v15) ... */
#[inline(always)]
fn BLAKE2_ROUND_NOMSG(v: &mut [u64; ARGON2_QWORDS_IN_BLOCK], i: [usize; 16]) {
    G(v, i[0], i[4], i[8], i[12]);
    G(v, i[1], i[5], i[9], i[13]);
    G(v, i[2], i[6], i[10], i[14]);
    G(v, i[3], i[7], i[11], i[15]);
    G(v, i[0], i[5], i[10], i[15]);
    G(v, i[1], i[6], i[11], i[12]);
    G(v, i[2], i[7], i[8], i[13]);
    G(v, i[3], i[4], i[9], i[14]);
}

/* Column indices used by the first round loop: (16*i .. 16*i + 15). */
#[inline(always)]
fn column_indices(i: usize) -> [usize; 16] {
    [
        16 * i,
        16 * i + 1,
        16 * i + 2,
        16 * i + 3,
        16 * i + 4,
        16 * i + 5,
        16 * i + 6,
        16 * i + 7,
        16 * i + 8,
        16 * i + 9,
        16 * i + 10,
        16 * i + 11,
        16 * i + 12,
        16 * i + 13,
        16 * i + 14,
        16 * i + 15,
    ]
}

/* Row indices used by the second round loop. */
#[inline(always)]
fn row_indices(i: usize) -> [usize; 16] {
    [
        2 * i,
        2 * i + 1,
        2 * i + 16,
        2 * i + 17,
        2 * i + 32,
        2 * i + 33,
        2 * i + 48,
        2 * i + 49,
        2 * i + 64,
        2 * i + 65,
        2 * i + 80,
        2 * i + 81,
        2 * i + 96,
        2 * i + 97,
        2 * i + 112,
        2 * i + 113,
    ]
}

/* ------------------------------------------------------------------ bodies */

/* static void fill_block(const block *prev_block, const block *ref_block,
 *                        block *next_block) */
unsafe fn fill_block(prev_block: *const block, ref_block: *const block, next_block: *mut block) {
    let mut blockR = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut block_tmp = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut i: usize;

    copy_block(&mut blockR, ref_block);
    xor_block(&mut blockR, prev_block);
    copy_block(&mut block_tmp, &blockR);
    /* Now blockR = ref_block + prev_block and block_tmp = ref_block + prev_block
       Apply Blake2 on columns of 64-bit words: (0,1,...,15), then
       (16,17,..31)... finally (112,113,...127) */
    i = 0;
    while i < 8 {
        BLAKE2_ROUND_NOMSG(&mut blockR.v, column_indices(i));
        i += 1;
    }

    /* Apply Blake2 on rows of 64-bit words: (0,1,16,17,...112,113), then
       (2,3,18,19,...,114,115).. finally (14,15,30,31,...,126,127) */
    i = 0;
    while i < 8 {
        BLAKE2_ROUND_NOMSG(&mut blockR.v, row_indices(i));
        i += 1;
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &blockR);
}

/* static void fill_block_with_xor(const block *prev_block,
 *                                 const block *ref_block, block *next_block) */
unsafe fn fill_block_with_xor(
    prev_block: *const block,
    ref_block: *const block,
    next_block: *mut block,
) {
    let mut blockR = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut block_tmp = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut i: usize;

    copy_block(&mut blockR, ref_block);
    xor_block(&mut blockR, prev_block);
    copy_block(&mut block_tmp, &blockR);
    xor_block(&mut block_tmp, next_block); /* Saving the next block contents for XOR over */
    /* Now blockR = ref_block + prev_block and block_tmp = ref_block + prev_block
     * + next_block */
    /* Apply Blake2 on columns of 64-bit words: (0,1,...,15) , then
       (16,17,..31)... finally (112,113,...127) */
    i = 0;
    while i < 8 {
        BLAKE2_ROUND_NOMSG(&mut blockR.v, column_indices(i));
        i += 1;
    }

    /* Apply Blake2 on rows of 64-bit words: (0,1,16,17,...112,113), then
       (2,3,18,19,...,114,115).. finally (14,15,30,31,...,126,127) */
    i = 0;
    while i < 8 {
        BLAKE2_ROUND_NOMSG(&mut blockR.v, row_indices(i));
        i += 1;
    }

    copy_block(next_block, &block_tmp);
    xor_block(next_block, &blockR);
}

/*
 * Generate pseudo-random values to reference blocks in the segment and puts
 * them into the array
 */
/* static void generate_addresses(const argon2_instance_t *instance,
 *                                const argon2_position_t *position,
 *                                uint64_t *pseudo_rands) */
unsafe fn generate_addresses(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rands: *mut u64,
) {
    let mut zero_block = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut input_block = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut address_block = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
    let mut tmp_block = block {
        v: [0u64; ARGON2_QWORDS_IN_BLOCK],
    };
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

/* void argon2_fill_segment_ref(const argon2_instance_t *instance,
 *                              argon2_position_t position) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_segment_ref(
    instance: *const argon2_instance_t,
    mut position: argon2_position_t,
) {
    let mut ref_block: *mut block;
    let mut curr_block: *mut block;
    /* Pseudo-random values that determine the reference block position */
    let pseudo_rands: *mut u64;
    let mut pseudo_rand: u64;
    let mut ref_index: u64;
    let mut ref_lane: u64;
    let mut prev_offset: u32;
    let mut curr_offset: u32;
    let mut starting_index: u32;
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

    starting_index = 0;

    if 0 == position.pass && 0 == position.slice {
        starting_index = 2; /* we have already generated the first two blocks */
    }

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

        /* 1.2 Computing the index of the reference block */
        /* 1.2.1 Taking pseudo-random value from the previous block */
        if data_independent_addressing != 0 {
            pseudo_rand = *pseudo_rands.add(i as usize);
        } else {
            pseudo_rand = (*(*(*instance).region).memory.add(prev_offset as usize)).v[0];
        }

        /* 1.2.2 Computing the lane of the reference block */
        ref_lane = (pseudo_rand >> 32) % ((*instance).lanes as u64);

        if position.pass == 0 && position.slice == 0 {
            /* Can not reference other lanes yet */
            ref_lane = position.lane as u64;
        }

        /* 1.2.3 Computing the number of possible reference block within the
         * lane.
         */
        position.index = i;
        ref_index = index_alpha(
            instance,
            &position,
            (pseudo_rand & 0xFFFF_FFFF) as u32,
            (ref_lane == position.lane as u64) as c_int,
        ) as u64;

        /* 2 Creating a new block */
        ref_block = (*(*instance).region).memory.add(
            (((*instance).lane_length as u64)
                .wrapping_mul(ref_lane)
                .wrapping_add(ref_index)) as usize,
        );
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
}

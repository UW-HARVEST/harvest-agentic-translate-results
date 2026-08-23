//! Shared constants / tables / helpers for dictBuilder/divsufsort.c
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

pub const ALPHABET_SIZE: usize = 256;
pub const BUCKET_A_SIZE: usize = ALPHABET_SIZE;
pub const BUCKET_B_SIZE: usize = ALPHABET_SIZE * ALPHABET_SIZE;
pub const SS_INSERTIONSORT_THRESHOLD: i32 = 8;
pub const SS_BLOCKSIZE: i32 = 1024;
/* SS_BLOCKSIZE (1024) <= 4096 -> 16 */
pub const SS_MISORT_STACKSIZE: usize = 16;
pub const SS_SMERGE_STACKSIZE: usize = 32;
pub const TR_INSERTIONSORT_THRESHOLD: i32 = 8;
pub const TR_STACKSIZE: usize = 64;

pub static lg_table: [i32; 256] = [
    -1, 0, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

pub static sqq_table: [i32; 256] = [
    0, 16, 22, 27, 32, 35, 39, 42, 45, 48, 50, 53, 55, 57, 59, 61,
    64, 65, 67, 69, 71, 73, 75, 76, 78, 80, 81, 83, 84, 86, 87, 89,
    90, 91, 93, 94, 96, 97, 98, 99, 101, 102, 103, 104, 106, 107, 108, 109,
    110, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126,
    128, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142,
    143, 144, 144, 145, 146, 147, 148, 149, 150, 150, 151, 152, 153, 154, 155, 155,
    156, 157, 158, 159, 160, 160, 161, 162, 163, 163, 164, 165, 166, 167, 167, 168,
    169, 170, 170, 171, 172, 173, 173, 174, 175, 176, 176, 177, 178, 178, 179, 180,
    181, 181, 182, 183, 183, 184, 185, 185, 186, 187, 187, 188, 189, 189, 190, 191,
    192, 192, 193, 193, 194, 195, 195, 196, 197, 197, 198, 199, 199, 200, 201, 201,
    202, 203, 203, 204, 204, 205, 206, 206, 207, 208, 208, 209, 209, 210, 211, 211,
    212, 212, 213, 214, 214, 215, 215, 216, 217, 217, 218, 218, 219, 219, 220, 221,
    221, 222, 222, 223, 224, 224, 225, 225, 226, 226, 227, 227, 228, 229, 229, 230,
    230, 231, 231, 232, 232, 233, 234, 234, 235, 235, 236, 236, 237, 237, 238, 238,
    239, 240, 240, 241, 241, 242, 242, 243, 243, 244, 244, 245, 245, 246, 246, 247,
    247, 248, 248, 249, 249, 250, 250, 251, 251, 252, 252, 253, 253, 254, 254, 255,
];

/* SS_BLOCKSIZE == 1024, so 256 <= SS_BLOCKSIZE -> the `#else` branch */
#[inline(always)]
pub fn ss_ilg(n: i32) -> i32 {
    if (n & 0xff00) != 0 {
        8 + lg_table[((n >> 8) & 0xff) as usize]
    } else {
        0 + lg_table[((n >> 0) & 0xff) as usize]
    }
}

#[inline(always)]
pub fn ss_isqrt(x: i32) -> i32 {
    let mut y: i32;
    let e: i32;

    if x >= (SS_BLOCKSIZE * SS_BLOCKSIZE) {
        return SS_BLOCKSIZE;
    }
    e = if (x & 0xffff0000u32 as i32) != 0 {
        if (x & 0xff000000u32 as i32) != 0 {
            24 + lg_table[(((x as u32) >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[(((x as u32) >> 16) & 0xff) as usize]
        }
    } else {
        if (x & 0x0000ff00) != 0 {
            8 + lg_table[(((x as u32) >> 8) & 0xff) as usize]
        } else {
            0 + lg_table[(((x as u32) >> 0) & 0xff) as usize]
        }
    };

    if e >= 16 {
        y = sqq_table[((x as u32) >> ((e - 6) - (e & 1))) as usize] << ((e >> 1) - 7);
        if e >= 24 {
            y = (y + 1 + x / y) >> 1;
        }
        y = (y + 1 + x / y) >> 1;
    } else if e >= 8 {
        y = (sqq_table[((x as u32) >> ((e - 6) - (e & 1))) as usize] >> (7 - (e >> 1))) + 1;
    } else {
        return sqq_table[x as usize] >> 4;
    }

    if x < y.wrapping_mul(y) {
        y - 1
    } else {
        y
    }
}

#[inline(always)]
pub fn tr_ilg(n: i32) -> i32 {
    if (n & 0xffff0000u32 as i32) != 0 {
        if (n & 0xff000000u32 as i32) != 0 {
            24 + lg_table[(((n as u32) >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[(((n as u32) >> 16) & 0xff) as usize]
        }
    } else {
        if (n & 0x0000ff00) != 0 {
            8 + lg_table[(((n as u32) >> 8) & 0xff) as usize]
        } else {
            0 + lg_table[(((n as u32) >> 0) & 0xff) as usize]
        }
    }
}

/* Compares two suffixes. */
#[inline(always)]
pub unsafe fn ss_compare(T: *const u8, p1: *const i32, p2: *const i32, depth: i32) -> i32 {
    let mut U1 = T.wrapping_offset(depth as isize).wrapping_offset(*p1 as isize);
    let mut U2 = T.wrapping_offset(depth as isize).wrapping_offset(*p2 as isize);
    let U1n = T.wrapping_offset(*p1.wrapping_offset(1) as isize).wrapping_offset(2);
    let U2n = T.wrapping_offset(*p2.wrapping_offset(1) as isize).wrapping_offset(2);
    while (U1 < U1n) && (U2 < U2n) && (*U1 == *U2) {
        U1 = U1.wrapping_offset(1);
        U2 = U2.wrapping_offset(1);
    }
    if U1 < U1n {
        if U2 < U2n {
            (*U1 as i32) - (*U2 as i32)
        } else {
            1
        }
    } else {
        if U2 < U2n {
            -1
        } else {
            0
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct trbudget_t {
    pub chance: i32,
    pub remain: i32,
    pub incval: i32,
    pub count: i32,
}

#[inline(always)]
pub unsafe fn trbudget_init(budget: *mut trbudget_t, chance: i32, incval: i32) {
    (*budget).chance = chance;
    (*budget).incval = incval;
    (*budget).remain = incval;
}

#[inline(always)]
pub unsafe fn trbudget_check(budget: *mut trbudget_t, size: i32) -> i32 {
    if size <= (*budget).remain {
        (*budget).remain -= size;
        return 1;
    }
    if (*budget).chance == 0 {
        (*budget).count += size;
        return 0;
    }
    (*budget).remain += (*budget).incval - size;
    (*budget).chance -= 1;
    1
}

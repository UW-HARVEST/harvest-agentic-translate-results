//! Translation of dictBuilder/divsufsort.c (libdivsufsort-lite).
//! Self-contained suffix-array construction; depends only on libc malloc/free.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use crate::common::allocations::{free, malloc};
use core::ffi::c_void;

/*- Constants -*/
const ALPHABET_SIZE: i32 = 256;
const BUCKET_A_SIZE: usize = 256;
const BUCKET_B_SIZE: usize = 256 * 256;
const SS_INSERTIONSORT_THRESHOLD: i32 = 8;
const SS_BLOCKSIZE: i32 = 1024;
const SS_MISORT_STACKSIZE: usize = 16;
const SS_SMERGE_STACKSIZE: usize = 32;
const TR_INSERTIONSORT_THRESHOLD: i32 = 8;
const TR_STACKSIZE: usize = 64;

/*- Pointer helper macros -*/
// pointer offset (element count), never UB via wrapping_offset
macro_rules! po {
    ($p:expr, $n:expr) => {
        ($p).wrapping_offset(($n) as isize)
    };
}
// pointer difference in i32 elements (int-sized)
macro_rules! pd {
    ($a:expr, $b:expr) => {
        ((($a) as isize - ($b) as isize) / (core::mem::size_of::<i32>() as isize)) as i32
    };
}
// Td[PA[*p]] as i32  (Td: *const u8, PA: *const i32, p: pointer whose value indexes PA)
macro_rules! tdpa {
    ($Td:expr, $PA:expr, $p:expr) => {
        *po!($Td, *po!($PA, *($p))) as i32
    };
}
// ISAd[*p]  (ISAd: *const i32)
macro_rules! isad {
    ($ISAd:expr, $p:expr) => {
        *po!($ISAd, *($p))
    };
}

// read T[i] as int (unsigned char promoted to int)
#[inline(always)]
unsafe fn tget(T: *const u8, i: i32) -> i32 {
    *T.wrapping_offset(i as isize) as i32
}

/*- lg / sqq tables -*/
static LG_TABLE: [i32; 256] = [
    -1, 0, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7,
];

static SQQ_TABLE: [i32; 256] = [
    0, 16, 22, 27, 32, 35, 39, 42, 45, 48, 50, 53, 55, 57, 59, 61, 64, 65, 67, 69, 71, 73, 75, 76,
    78, 80, 81, 83, 84, 86, 87, 89, 90, 91, 93, 94, 96, 97, 98, 99, 101, 102, 103, 104, 106, 107,
    108, 109, 110, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 128,
    128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 144, 145,
    146, 147, 148, 149, 150, 150, 151, 152, 153, 154, 155, 155, 156, 157, 158, 159, 160, 160, 161,
    162, 163, 163, 164, 165, 166, 167, 167, 168, 169, 170, 170, 171, 172, 173, 173, 174, 175, 176,
    176, 177, 178, 178, 179, 180, 181, 181, 182, 183, 183, 184, 185, 185, 186, 187, 187, 188, 189,
    189, 190, 191, 192, 192, 193, 193, 194, 195, 195, 196, 197, 197, 198, 199, 199, 200, 201, 201,
    202, 203, 203, 204, 204, 205, 206, 206, 207, 208, 208, 209, 209, 210, 211, 211, 212, 212, 213,
    214, 214, 215, 215, 216, 217, 217, 218, 218, 219, 219, 220, 221, 221, 222, 222, 223, 224, 224,
    225, 225, 226, 226, 227, 227, 228, 229, 229, 230, 230, 231, 231, 232, 232, 233, 234, 234, 235,
    235, 236, 236, 237, 237, 238, 238, 239, 240, 240, 241, 241, 242, 242, 243, 243, 244, 244, 245,
    245, 246, 246, 247, 247, 248, 248, 249, 249, 250, 250, 251, 251, 252, 252, 253, 253, 254, 254,
    255,
];

#[inline]
fn ss_ilg(n: i32) -> i32 {
    // SS_BLOCKSIZE == 1024 (>= 256, != 0)
    if (n & 0xff00) != 0 {
        8 + LG_TABLE[((n >> 8) & 0xff) as usize]
    } else {
        0 + LG_TABLE[((n >> 0) & 0xff) as usize]
    }
}

#[inline]
fn ss_isqrt(x: i32) -> i32 {
    let mut y: i32;
    let e: i32;

    if x >= (SS_BLOCKSIZE * SS_BLOCKSIZE) {
        return SS_BLOCKSIZE;
    }
    e = if (x as u32 & 0xffff0000) != 0 {
        if (x as u32 & 0xff000000) != 0 {
            24 + LG_TABLE[((x >> 24) & 0xff) as usize]
        } else {
            16 + LG_TABLE[((x >> 16) & 0xff) as usize]
        }
    } else {
        if (x as u32 & 0x0000ff00) != 0 {
            8 + LG_TABLE[((x >> 8) & 0xff) as usize]
        } else {
            0 + LG_TABLE[((x >> 0) & 0xff) as usize]
        }
    };

    if e >= 16 {
        y = SQQ_TABLE[(x >> ((e - 6) - (e & 1))) as usize] << ((e >> 1) - 7);
        if e >= 24 {
            y = (y + 1 + x / y) >> 1;
        }
        y = (y + 1 + x / y) >> 1;
    } else if e >= 8 {
        y = (SQQ_TABLE[(x >> ((e - 6) - (e & 1))) as usize] >> (7 - (e >> 1))) + 1;
    } else {
        return SQQ_TABLE[x as usize] >> 4;
    }

    if x < (y * y) {
        y - 1
    } else {
        y
    }
}

/*---------------------------------------------------------------------------*/
/* Compares two suffixes. */
#[inline]
unsafe fn ss_compare(T: *const u8, p1: *const i32, p2: *const i32, depth: i32) -> i32 {
    let mut U1 = po!(T, depth + *p1);
    let mut U2 = po!(T, depth + *p2);
    let U1n = po!(T, *po!(p1, 1) + 2);
    let U2n = po!(T, *po!(p2, 1) + 2);

    while (U1 < U1n) && (U2 < U2n) && (*U1 == *U2) {
        U1 = po!(U1, 1);
        U2 = po!(U2, 1);
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

/*---------------------------------------------------------------------------*/
/* Insertionsort for small size groups */
unsafe fn ss_insertionsort(T: *const u8, PA: *const i32, first: *mut i32, last: *mut i32, depth: i32) {
    let mut i = po!(last, -2);
    let mut j: *mut i32;
    let mut t: i32;
    let mut r: i32 = 0;

    while first <= i {
        t = *i;
        j = po!(i, 1);
        loop {
            r = ss_compare(T, po!(PA, t), po!(PA, *j), depth);
            if !(0 < r) {
                break;
            }
            loop {
                *po!(j, -1) = *j;
                j = po!(j, 1);
                if !((j < last) && (*j < 0)) {
                    break;
                }
            }
            if last <= j {
                break;
            }
        }
        if r == 0 {
            *j = !*j;
        }
        *po!(j, -1) = t;
        i = po!(i, -1);
    }
}

/*---------------------------------------------------------------------------*/
#[inline]
unsafe fn ss_fixdown(Td: *const u8, PA: *const i32, SA: *mut i32, mut i: i32, size: i32) {
    let mut j: i32;
    let mut k: i32 = 0;
    let v: i32;
    let mut c: i32;
    let mut d: i32;
    let mut e: i32;

    v = *po!(SA, i);
    c = *po!(Td, *po!(PA, v)) as i32;
    loop {
        j = 2 * i + 1;
        if !(j < size) {
            break;
        }
        k = j;
        j += 1;
        d = *po!(Td, *po!(PA, *po!(SA, k))) as i32;
        e = *po!(Td, *po!(PA, *po!(SA, j))) as i32;
        if d < e {
            k = j;
            d = e;
        }
        if d <= c {
            break;
        }
        *po!(SA, i) = *po!(SA, k);
        i = k;
    }
    *po!(SA, i) = v;
}

/* Simple top-down heapsort. */
unsafe fn ss_heapsort(Td: *const u8, PA: *const i32, SA: *mut i32, size: i32) {
    let mut i: i32;
    let mut m: i32;
    let mut t: i32;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if (*po!(Td, *po!(PA, *po!(SA, m / 2))) as i32) < (*po!(Td, *po!(PA, *po!(SA, m))) as i32) {
            t = *po!(SA, m);
            *po!(SA, m) = *po!(SA, m / 2);
            *po!(SA, m / 2) = t;
        }
    }

    i = m / 2 - 1;
    while 0 <= i {
        ss_fixdown(Td, PA, SA, i, m);
        i -= 1;
    }
    if (size % 2) == 0 {
        t = *po!(SA, 0);
        *po!(SA, 0) = *po!(SA, m);
        *po!(SA, m) = t;
        ss_fixdown(Td, PA, SA, 0, m);
    }
    i = m - 1;
    while 0 < i {
        t = *po!(SA, 0);
        *po!(SA, 0) = *po!(SA, i);
        ss_fixdown(Td, PA, SA, 0, i);
        *po!(SA, i) = t;
        i -= 1;
    }
}

/*---------------------------------------------------------------------------*/
/* Returns the median of three elements. */
#[inline]
unsafe fn ss_median3(
    Td: *const u8,
    PA: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    v3: *mut i32,
) -> *mut i32 {
    if tdpa!(Td, PA, v1) > tdpa!(Td, PA, v2) {
        let t = v1;
        v1 = v2;
        v2 = t;
    }
    if tdpa!(Td, PA, v2) > tdpa!(Td, PA, v3) {
        if tdpa!(Td, PA, v1) > tdpa!(Td, PA, v3) {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

/* Returns the median of five elements. */
#[inline]
unsafe fn ss_median5(
    Td: *const u8,
    PA: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    mut v3: *mut i32,
    mut v4: *mut i32,
    mut v5: *mut i32,
) -> *mut i32 {
    let mut t: *mut i32;
    if tdpa!(Td, PA, v2) > tdpa!(Td, PA, v3) {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if tdpa!(Td, PA, v4) > tdpa!(Td, PA, v5) {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if tdpa!(Td, PA, v2) > tdpa!(Td, PA, v4) {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if tdpa!(Td, PA, v1) > tdpa!(Td, PA, v3) {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if tdpa!(Td, PA, v1) > tdpa!(Td, PA, v4) {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if tdpa!(Td, PA, v3) > tdpa!(Td, PA, v4) {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
#[inline]
unsafe fn ss_pivot(Td: *const u8, PA: *const i32, mut first: *mut i32, mut last: *mut i32) -> *mut i32 {
    let mut middle: *mut i32;
    let mut t: i32;

    t = pd!(last, first);
    middle = po!(first, t / 2);

    if t <= 512 {
        if t <= 32 {
            return ss_median3(Td, PA, first, middle, po!(last, -1));
        } else {
            t >>= 2;
            return ss_median5(Td, PA, first, po!(first, t), middle, po!(last, -1 - t), po!(last, -1));
        }
    }
    t >>= 3;
    first = ss_median3(Td, PA, first, po!(first, t), po!(first, t << 1));
    middle = ss_median3(Td, PA, po!(middle, -t), middle, po!(middle, t));
    last = ss_median3(Td, PA, po!(last, -1 - (t << 1)), po!(last, -1 - t), po!(last, -1));
    ss_median3(Td, PA, first, middle, last)
}

/*---------------------------------------------------------------------------*/
/* Binary partition for substrings. */
#[inline]
unsafe fn ss_partition(PA: *const i32, first: *mut i32, last: *mut i32, depth: i32) -> *mut i32 {
    let mut a = po!(first, -1);
    let mut b = last;
    let mut t: i32;
    loop {
        loop {
            a = po!(a, 1);
            if !((a < b) && ((*po!(PA, *a) + depth) >= (*po!(PA, *a + 1) + 1))) {
                break;
            }
            *a = !*a;
        }
        loop {
            b = po!(b, -1);
            if !((a < b) && ((*po!(PA, *b) + depth) < (*po!(PA, *b + 1) + 1))) {
                break;
            }
        }
        if b <= a {
            break;
        }
        t = !*b;
        *b = *a;
        *a = t;
    }
    if first < a {
        *first = !*first;
    }
    a
}

/* Multikey introsort for medium size groups. */
unsafe fn ss_mintrosort(
    T: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    mut last: *mut i32,
    mut depth: i32,
) {
    const STACK_SIZE: usize = SS_MISORT_STACKSIZE;
    #[derive(Clone, Copy)]
    struct StackEntry {
        a: *mut i32,
        b: *mut i32,
        c: i32,
        d: i32,
    }
    let mut stack = [StackEntry {
        a: core::ptr::null_mut(),
        b: core::ptr::null_mut(),
        c: 0,
        d: 0,
    }; STACK_SIZE];
    let mut ssize: i32 = 0;

    macro_rules! STACK_PUSH {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            debug_assert!((ssize as usize) < STACK_SIZE);
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            ssize += 1;
        }};
    }
    macro_rules! STACK_POP {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            debug_assert!(0 <= ssize);
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            $a = stack[ssize as usize].a;
            $b = stack[ssize as usize].b;
            $c = stack[ssize as usize].c;
            $d = stack[ssize as usize].d;
        }};
    }

    let mut Td: *const u8;
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut c: *mut i32;
    let mut d: *mut i32;
    let mut e: *mut i32;
    let mut f: *mut i32;
    let mut s: i32;
    let mut t: i32;
    let mut limit: i32;
    let mut v: i32;
    let mut x: i32 = 0;

    limit = ss_ilg(pd!(last, first));
    loop {
        if pd!(last, first) <= SS_INSERTIONSORT_THRESHOLD {
            if 1 < pd!(last, first) {
                ss_insertionsort(T, PA, first, last, depth);
            }
            STACK_POP!(first, last, depth, limit);
            continue;
        }

        Td = po!(T, depth);
        let old_limit = limit;
        limit -= 1;
        if old_limit == 0 {
            ss_heapsort(Td, PA, first, pd!(last, first));
        }
        if limit < 0 {
            a = po!(first, 1);
            v = *po!(Td, *po!(PA, *first)) as i32;
            while a < last {
                x = *po!(Td, *po!(PA, *a)) as i32;
                if x != v {
                    if 1 < pd!(a, first) {
                        break;
                    }
                    v = x;
                    first = a;
                }
                a = po!(a, 1);
            }
            if (*po!(Td, *po!(PA, *first) - 1) as i32) < v {
                first = ss_partition(PA, first, a, depth);
            }
            if pd!(a, first) <= pd!(last, a) {
                if 1 < pd!(a, first) {
                    STACK_PUSH!(a, last, depth, -1);
                    last = a;
                    depth += 1;
                    limit = ss_ilg(pd!(a, first));
                } else {
                    first = a;
                    limit = -1;
                }
            } else {
                if 1 < pd!(last, a) {
                    STACK_PUSH!(first, a, depth + 1, ss_ilg(pd!(a, first)));
                    first = a;
                    limit = -1;
                } else {
                    last = a;
                    depth += 1;
                    limit = ss_ilg(pd!(a, first));
                }
            }
            continue;
        }

        /* choose pivot */
        a = ss_pivot(Td, PA, first, last);
        v = *po!(Td, *po!(PA, *a)) as i32;
        {
            let tt = *first;
            *first = *a;
            *a = tt;
        }

        /* partition */
        b = first;
        loop {
            b = po!(b, 1);
            if !(b < last) {
                break;
            }
            x = *po!(Td, *po!(PA, *b)) as i32;
            if x != v {
                break;
            }
        }
        a = b;
        if (a < last) && (x < v) {
            loop {
                b = po!(b, 1);
                if !(b < last) {
                    break;
                }
                x = *po!(Td, *po!(PA, *b)) as i32;
                if !(x <= v) {
                    break;
                }
                if x == v {
                    let tt = *b;
                    *b = *a;
                    *a = tt;
                    a = po!(a, 1);
                }
            }
        }
        c = last;
        loop {
            c = po!(c, -1);
            if !(b < c) {
                break;
            }
            x = *po!(Td, *po!(PA, *c)) as i32;
            if x != v {
                break;
            }
        }
        d = c;
        if (b < d) && (x > v) {
            loop {
                c = po!(c, -1);
                if !(b < c) {
                    break;
                }
                x = *po!(Td, *po!(PA, *c)) as i32;
                if !(x >= v) {
                    break;
                }
                if x == v {
                    let tt = *c;
                    *c = *d;
                    *d = tt;
                    d = po!(d, -1);
                }
            }
        }
        while b < c {
            {
                let tt = *b;
                *b = *c;
                *c = tt;
            }
            loop {
                b = po!(b, 1);
                if !(b < c) {
                    break;
                }
                x = *po!(Td, *po!(PA, *b)) as i32;
                if !(x <= v) {
                    break;
                }
                if x == v {
                    let tt = *b;
                    *b = *a;
                    *a = tt;
                    a = po!(a, 1);
                }
            }
            loop {
                c = po!(c, -1);
                if !(b < c) {
                    break;
                }
                x = *po!(Td, *po!(PA, *c)) as i32;
                if !(x >= v) {
                    break;
                }
                if x == v {
                    let tt = *c;
                    *c = *d;
                    *d = tt;
                    d = po!(d, -1);
                }
            }
        }

        if a <= d {
            c = po!(b, -1);

            s = pd!(a, first);
            t = pd!(b, a);
            if s > t {
                s = t;
            }
            e = first;
            f = po!(b, -s);
            while 0 < s {
                let tt = *e;
                *e = *f;
                *f = tt;
                s -= 1;
                e = po!(e, 1);
                f = po!(f, 1);
            }
            s = pd!(d, c);
            t = pd!(last, d) - 1;
            if s > t {
                s = t;
            }
            e = b;
            f = po!(last, -s);
            while 0 < s {
                let tt = *e;
                *e = *f;
                *f = tt;
                s -= 1;
                e = po!(e, 1);
                f = po!(f, 1);
            }

            a = po!(first, pd!(b, a));
            c = po!(last, -pd!(d, c));
            b = if v <= (*po!(Td, *po!(PA, *a) - 1) as i32) {
                a
            } else {
                ss_partition(PA, a, c, depth)
            };

            if pd!(a, first) <= pd!(last, c) {
                if pd!(last, c) <= pd!(c, b) {
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pd!(c, b)));
                    STACK_PUSH!(c, last, depth, limit);
                    last = a;
                } else if pd!(a, first) <= pd!(c, b) {
                    STACK_PUSH!(c, last, depth, limit);
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pd!(c, b)));
                    last = a;
                } else {
                    STACK_PUSH!(c, last, depth, limit);
                    STACK_PUSH!(first, a, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(pd!(c, b));
                }
            } else {
                if pd!(a, first) <= pd!(c, b) {
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pd!(c, b)));
                    STACK_PUSH!(first, a, depth, limit);
                    first = c;
                } else if pd!(last, c) <= pd!(c, b) {
                    STACK_PUSH!(first, a, depth, limit);
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pd!(c, b)));
                    first = c;
                } else {
                    STACK_PUSH!(first, a, depth, limit);
                    STACK_PUSH!(c, last, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(pd!(c, b));
                }
            }
        } else {
            limit += 1;
            if (*po!(Td, *po!(PA, *first) - 1) as i32) < v {
                first = ss_partition(PA, first, last, depth);
                limit = ss_ilg(pd!(last, first));
            }
            depth += 1;
        }
    }
}

/*---------------------------------------------------------------------------*/
#[inline]
unsafe fn ss_blockswap(mut a: *mut i32, mut b: *mut i32, mut n: i32) {
    let mut t: i32;
    while 0 < n {
        t = *a;
        *a = *b;
        *b = t;
        n -= 1;
        a = po!(a, 1);
        b = po!(b, 1);
    }
}

#[inline]
unsafe fn ss_rotate(mut first: *mut i32, middle: *mut i32, mut last: *mut i32) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;
    let mut l: i32;
    let mut r: i32;
    l = pd!(middle, first);
    r = pd!(last, middle);
    while (0 < l) && (0 < r) {
        if l == r {
            ss_blockswap(first, middle, l);
            break;
        }
        if l < r {
            a = po!(last, -1);
            b = po!(middle, -1);
            t = *a;
            loop {
                *a = *b;
                a = po!(a, -1);
                *b = *a;
                b = po!(b, -1);
                if b < first {
                    *a = t;
                    last = a;
                    r -= l + 1;
                    if r <= l {
                        break;
                    }
                    a = po!(a, -1);
                    b = po!(middle, -1);
                    t = *a;
                }
            }
        } else {
            a = first;
            b = middle;
            t = *a;
            loop {
                *a = *b;
                a = po!(a, 1);
                *b = *a;
                b = po!(b, 1);
                if last <= b {
                    *a = t;
                    first = po!(a, 1);
                    l -= r + 1;
                    if l <= r {
                        break;
                    }
                    a = po!(a, 1);
                    b = middle;
                    t = *a;
                }
            }
        }
    }
}

/*---------------------------------------------------------------------------*/
unsafe fn ss_inplacemerge(
    T: *const u8,
    PA: *const i32,
    first: *mut i32,
    mut middle: *mut i32,
    mut last: *mut i32,
    depth: i32,
) {
    let mut p: *const i32;
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut len: i32;
    let mut half: i32;
    let mut q: i32;
    let mut r: i32;
    let mut x: i32;

    loop {
        if *po!(last, -1) < 0 {
            x = 1;
            p = po!(PA, !*po!(last, -1));
        } else {
            x = 0;
            p = po!(PA, *po!(last, -1));
        }
        a = first;
        len = pd!(middle, first);
        half = len >> 1;
        r = -1;
        while 0 < len {
            b = po!(a, half);
            q = ss_compare(T, po!(PA, if 0 <= *b { *b } else { !*b }), p, depth);
            if q < 0 {
                a = po!(b, 1);
                half -= (len & 1) ^ 1;
            } else {
                r = q;
            }
            len = half;
            half >>= 1;
        }
        if a < middle {
            if r == 0 {
                *a = !*a;
            }
            ss_rotate(a, middle, last);
            last = po!(last, -pd!(middle, a));
            middle = a;
            if first == middle {
                break;
            }
        }
        last = po!(last, -1);
        if x != 0 {
            loop {
                last = po!(last, -1);
                if !(*last < 0) {
                    break;
                }
            }
        }
        if middle == last {
            break;
        }
    }
}

/*---------------------------------------------------------------------------*/
/* Merge-forward with internal buffer. */
unsafe fn ss_mergeforward(
    T: *const u8,
    PA: *const i32,
    first: *mut i32,
    middle: *mut i32,
    last: *mut i32,
    buf: *mut i32,
    depth: i32,
) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut c: *mut i32;
    let bufend: *mut i32;
    let t: i32;
    let mut r: i32;

    bufend = po!(buf, pd!(middle, first) - 1);
    ss_blockswap(buf, first, pd!(middle, first));

    a = first;
    t = *a;
    b = buf;
    c = middle;
    loop {
        r = ss_compare(T, po!(PA, *b), po!(PA, *c), depth);
        if r < 0 {
            loop {
                *a = *b;
                a = po!(a, 1);
                if bufend <= b {
                    *bufend = t;
                    return;
                }
                *b = *a;
                b = po!(b, 1);
                if !(*b < 0) {
                    break;
                }
            }
        } else if r > 0 {
            loop {
                *a = *c;
                a = po!(a, 1);
                *c = *a;
                c = po!(c, 1);
                if last <= c {
                    while b < bufend {
                        *a = *b;
                        a = po!(a, 1);
                        *b = *a;
                        b = po!(b, 1);
                    }
                    *a = *b;
                    *b = t;
                    return;
                }
                if !(*c < 0) {
                    break;
                }
            }
        } else {
            *c = !*c;
            loop {
                *a = *b;
                a = po!(a, 1);
                if bufend <= b {
                    *bufend = t;
                    return;
                }
                *b = *a;
                b = po!(b, 1);
                if !(*b < 0) {
                    break;
                }
            }

            loop {
                *a = *c;
                a = po!(a, 1);
                *c = *a;
                c = po!(c, 1);
                if last <= c {
                    while b < bufend {
                        *a = *b;
                        a = po!(a, 1);
                        *b = *a;
                        b = po!(b, 1);
                    }
                    *a = *b;
                    *b = t;
                    return;
                }
                if !(*c < 0) {
                    break;
                }
            }
        }
    }
}

/* Merge-backward with internal buffer. */
unsafe fn ss_mergebackward(
    T: *const u8,
    PA: *const i32,
    first: *mut i32,
    middle: *mut i32,
    last: *mut i32,
    buf: *mut i32,
    depth: i32,
) {
    let mut p1: *const i32;
    let mut p2: *const i32;
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut c: *mut i32;
    let bufend: *mut i32;
    let t: i32;
    let mut r: i32;
    let mut x: i32;

    bufend = po!(buf, pd!(last, middle) - 1);
    ss_blockswap(buf, middle, pd!(last, middle));

    x = 0;
    if *bufend < 0 {
        p1 = po!(PA, !*bufend);
        x |= 1;
    } else {
        p1 = po!(PA, *bufend);
    }
    if *po!(middle, -1) < 0 {
        p2 = po!(PA, !*po!(middle, -1));
        x |= 2;
    } else {
        p2 = po!(PA, *po!(middle, -1));
    }
    a = po!(last, -1);
    t = *a;
    b = bufend;
    c = po!(middle, -1);
    loop {
        r = ss_compare(T, p1, p2, depth);
        if 0 < r {
            if x & 1 != 0 {
                loop {
                    *a = *b;
                    a = po!(a, -1);
                    *b = *a;
                    b = po!(b, -1);
                    if !(*b < 0) {
                        break;
                    }
                }
                x ^= 1;
            }
            *a = *b;
            a = po!(a, -1);
            if b <= buf {
                *buf = t;
                break;
            }
            *b = *a;
            b = po!(b, -1);
            if *b < 0 {
                p1 = po!(PA, !*b);
                x |= 1;
            } else {
                p1 = po!(PA, *b);
            }
        } else if r < 0 {
            if x & 2 != 0 {
                loop {
                    *a = *c;
                    a = po!(a, -1);
                    *c = *a;
                    c = po!(c, -1);
                    if !(*c < 0) {
                        break;
                    }
                }
                x ^= 2;
            }
            *a = *c;
            a = po!(a, -1);
            *c = *a;
            c = po!(c, -1);
            if c < first {
                while buf < b {
                    *a = *b;
                    a = po!(a, -1);
                    *b = *a;
                    b = po!(b, -1);
                }
                *a = *b;
                *b = t;
                break;
            }
            if *c < 0 {
                p2 = po!(PA, !*c);
                x |= 2;
            } else {
                p2 = po!(PA, *c);
            }
        } else {
            if x & 1 != 0 {
                loop {
                    *a = *b;
                    a = po!(a, -1);
                    *b = *a;
                    b = po!(b, -1);
                    if !(*b < 0) {
                        break;
                    }
                }
                x ^= 1;
            }
            *a = !*b;
            a = po!(a, -1);
            if b <= buf {
                *buf = t;
                break;
            }
            *b = *a;
            b = po!(b, -1);
            if x & 2 != 0 {
                loop {
                    *a = *c;
                    a = po!(a, -1);
                    *c = *a;
                    c = po!(c, -1);
                    if !(*c < 0) {
                        break;
                    }
                }
                x ^= 2;
            }
            *a = *c;
            a = po!(a, -1);
            *c = *a;
            c = po!(c, -1);
            if c < first {
                while buf < b {
                    *a = *b;
                    a = po!(a, -1);
                    *b = *a;
                    b = po!(b, -1);
                }
                *a = *b;
                *b = t;
                break;
            }
            if *b < 0 {
                p1 = po!(PA, !*b);
                x |= 1;
            } else {
                p1 = po!(PA, *b);
            }
            if *c < 0 {
                p2 = po!(PA, !*c);
                x |= 2;
            } else {
                p2 = po!(PA, *c);
            }
        }
    }
}

/* D&C based merge. */
unsafe fn ss_swapmerge(
    T: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    mut middle: *mut i32,
    mut last: *mut i32,
    buf: *mut i32,
    bufsize: i32,
    depth: i32,
) {
    const STACK_SIZE: usize = SS_SMERGE_STACKSIZE;
    #[derive(Clone, Copy)]
    struct StackEntry {
        a: *mut i32,
        b: *mut i32,
        c: *mut i32,
        d: i32,
    }
    let mut stack = [StackEntry {
        a: core::ptr::null_mut(),
        b: core::ptr::null_mut(),
        c: core::ptr::null_mut(),
        d: 0,
    }; STACK_SIZE];
    let mut ssize: i32 = 0;

    macro_rules! GETIDX {
        ($a:expr) => {
            if 0 <= ($a) {
                $a
            } else {
                !($a)
            }
        };
    }
    macro_rules! STACK_PUSH {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            debug_assert!((ssize as usize) < STACK_SIZE);
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            ssize += 1;
        }};
    }
    // MERGE_CHECK operates on (a, b, c) where a,b are pointers and c is check int
    macro_rules! MERGE_CHECK {
        ($a:expr, $b:expr, $c:expr) => {{
            if (($c) & 1 != 0)
                || ((($c) & 2 != 0)
                    && (ss_compare(T, po!(PA, GETIDX!(*po!($a, -1))), po!(PA, *($a)), depth) == 0))
            {
                *($a) = !*($a);
            }
            if (($c) & 4 != 0)
                && (ss_compare(T, po!(PA, GETIDX!(*po!($b, -1))), po!(PA, *($b)), depth) == 0)
            {
                *($b) = !*($b);
            }
        }};
    }

    let mut l: *mut i32;
    let mut r: *mut i32;
    let mut lm: *mut i32;
    let mut rm: *mut i32;
    let mut m: i32;
    let mut len: i32;
    let mut half: i32;
    let mut check: i32 = 0;
    let mut next: i32;

    loop {
        if pd!(last, middle) <= bufsize {
            if (first < middle) && (middle < last) {
                ss_mergebackward(T, PA, first, middle, last, buf, depth);
            }
            MERGE_CHECK!(first, last, check);
            {
                debug_assert!(0 <= ssize);
                if ssize == 0 {
                    return;
                }
                ssize -= 1;
                first = stack[ssize as usize].a;
                middle = stack[ssize as usize].b;
                last = stack[ssize as usize].c;
                check = stack[ssize as usize].d;
            }
            continue;
        }

        if pd!(middle, first) <= bufsize {
            if first < middle {
                ss_mergeforward(T, PA, first, middle, last, buf, depth);
            }
            MERGE_CHECK!(first, last, check);
            {
                debug_assert!(0 <= ssize);
                if ssize == 0 {
                    return;
                }
                ssize -= 1;
                first = stack[ssize as usize].a;
                middle = stack[ssize as usize].b;
                last = stack[ssize as usize].c;
                check = stack[ssize as usize].d;
            }
            continue;
        }

        m = 0;
        len = core::cmp::min(pd!(middle, first), pd!(last, middle));
        half = len >> 1;
        while 0 < len {
            if ss_compare(
                T,
                po!(PA, GETIDX!(*po!(middle, m + half))),
                po!(PA, GETIDX!(*po!(middle, -m - half - 1))),
                depth,
            ) < 0
            {
                m += half + 1;
                half -= (len & 1) ^ 1;
            }
            len = half;
            half >>= 1;
        }

        if 0 < m {
            lm = po!(middle, -m);
            rm = po!(middle, m);
            ss_blockswap(lm, middle, m);
            l = middle;
            r = middle;
            next = 0;
            if rm < last {
                if *rm < 0 {
                    *rm = !*rm;
                    if first < lm {
                        loop {
                            l = po!(l, -1);
                            if !(*l < 0) {
                                break;
                            }
                        }
                        next |= 4;
                    }
                    next |= 1;
                } else if first < lm {
                    while *r < 0 {
                        r = po!(r, 1);
                    }
                    next |= 2;
                }
            }

            if pd!(l, first) <= pd!(last, r) {
                STACK_PUSH!(r, rm, last, (next & 3) | (check & 4));
                middle = lm;
                last = l;
                check = (check & 3) | (next & 4);
            } else {
                if (next & 2 != 0) && (r == middle) {
                    next ^= 6;
                }
                STACK_PUSH!(first, lm, l, (check & 3) | (next & 4));
                first = r;
                middle = rm;
                check = (next & 3) | (check & 4);
            }
        } else {
            if ss_compare(T, po!(PA, GETIDX!(*po!(middle, -1))), po!(PA, *middle), depth) == 0 {
                *middle = !*middle;
            }
            MERGE_CHECK!(first, last, check);
            {
                debug_assert!(0 <= ssize);
                if ssize == 0 {
                    return;
                }
                ssize -= 1;
                first = stack[ssize as usize].a;
                middle = stack[ssize as usize].b;
                last = stack[ssize as usize].c;
                check = stack[ssize as usize].d;
            }
        }
    }
}

/*---------------------------------------------------------------------------*/
/* Substring sort */
unsafe fn sssort(
    T: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    last: *mut i32,
    mut buf: *mut i32,
    mut bufsize: i32,
    depth: i32,
    n: i32,
    lastsuffix: i32,
) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut middle: *mut i32;
    let mut curbuf: *mut i32;
    let mut j: i32;
    let mut k: i32;
    let mut curbufsize: i32;
    let mut limit: i32;
    let mut i: i32;

    if lastsuffix != 0 {
        first = po!(first, 1);
    }

    // SS_BLOCKSIZE != 0
    limit = ss_isqrt(pd!(last, first));
    if (bufsize < SS_BLOCKSIZE) && (bufsize < pd!(last, first)) && (bufsize < limit) {
        if SS_BLOCKSIZE < limit {
            limit = SS_BLOCKSIZE;
        }
        middle = po!(last, -limit);
        buf = middle;
        bufsize = limit;
    } else {
        middle = last;
        limit = 0;
    }
    a = first;
    i = 0;
    while SS_BLOCKSIZE < pd!(middle, a) {
        // SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE
        ss_mintrosort(T, PA, a, po!(a, SS_BLOCKSIZE), depth);
        curbufsize = pd!(last, po!(a, SS_BLOCKSIZE));
        curbuf = po!(a, SS_BLOCKSIZE);
        if curbufsize <= bufsize {
            curbufsize = bufsize;
            curbuf = buf;
        }
        b = a;
        k = SS_BLOCKSIZE;
        j = i;
        while j & 1 != 0 {
            ss_swapmerge(T, PA, po!(b, -k), b, po!(b, k), curbuf, curbufsize, depth);
            b = po!(b, -k);
            k <<= 1;
            j >>= 1;
        }
        a = po!(a, SS_BLOCKSIZE);
        i += 1;
    }
    // SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE
    ss_mintrosort(T, PA, a, middle, depth);
    k = SS_BLOCKSIZE;
    while i != 0 {
        if i & 1 != 0 {
            ss_swapmerge(T, PA, po!(a, -k), a, middle, buf, bufsize, depth);
            a = po!(a, -k);
        }
        k <<= 1;
        i >>= 1;
    }
    if limit != 0 {
        // SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE
        ss_mintrosort(T, PA, middle, last, depth);
        ss_inplacemerge(T, PA, first, middle, last, depth);
    }

    if lastsuffix != 0 {
        /* Insert last type B* suffix. */
        let mut PAi: [i32; 2] = [*po!(PA, *po!(first, -1)), n - 2];
        a = first;
        i = *po!(first, -1);
        while (a < last)
            && ((*a < 0) || (0 < ss_compare(T, PAi.as_ptr(), po!(PA, *a), depth)))
        {
            *po!(a, -1) = *a;
            a = po!(a, 1);
        }
        *po!(a, -1) = i;
    }
}

/*---------------------------------------------------------------------------*/
#[inline]
fn tr_ilg(n: i32) -> i32 {
    if (n as u32 & 0xffff0000) != 0 {
        if (n as u32 & 0xff000000) != 0 {
            24 + LG_TABLE[((n >> 24) & 0xff) as usize]
        } else {
            16 + LG_TABLE[((n >> 16) & 0xff) as usize]
        }
    } else {
        if (n as u32 & 0x0000ff00) != 0 {
            8 + LG_TABLE[((n >> 8) & 0xff) as usize]
        } else {
            0 + LG_TABLE[((n >> 0) & 0xff) as usize]
        }
    }
}

/*---------------------------------------------------------------------------*/
/* Simple insertionsort for small size groups. */
unsafe fn tr_insertionsort(ISAd: *const i32, first: *mut i32, last: *mut i32) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;
    let mut r: i32 = 0;

    a = po!(first, 1);
    while a < last {
        t = *a;
        b = po!(a, -1);
        loop {
            r = isad!(ISAd, &t) - isad!(ISAd, b);
            if !(0 > r) {
                break;
            }
            loop {
                *po!(b, 1) = *b;
                b = po!(b, -1);
                if !((first <= b) && (*b < 0)) {
                    break;
                }
            }
            if b < first {
                break;
            }
        }
        if r == 0 {
            *b = !*b;
        }
        *po!(b, 1) = t;
        a = po!(a, 1);
    }
}

/*---------------------------------------------------------------------------*/
#[inline]
unsafe fn tr_fixdown(ISAd: *const i32, SA: *mut i32, mut i: i32, size: i32) {
    let mut j: i32;
    let mut k: i32 = 0;
    let v: i32;
    let mut c: i32;
    let mut d: i32;
    let mut e: i32;

    v = *po!(SA, i);
    c = *po!(ISAd, v);
    loop {
        j = 2 * i + 1;
        if !(j < size) {
            break;
        }
        k = j;
        j += 1;
        d = *po!(ISAd, *po!(SA, k));
        e = *po!(ISAd, *po!(SA, j));
        if d < e {
            k = j;
            d = e;
        }
        if d <= c {
            break;
        }
        *po!(SA, i) = *po!(SA, k);
        i = k;
    }
    *po!(SA, i) = v;
}

/* Simple top-down heapsort. */
unsafe fn tr_heapsort(ISAd: *const i32, SA: *mut i32, size: i32) {
    let mut i: i32;
    let mut m: i32;
    let mut t: i32;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if *po!(ISAd, *po!(SA, m / 2)) < *po!(ISAd, *po!(SA, m)) {
            t = *po!(SA, m);
            *po!(SA, m) = *po!(SA, m / 2);
            *po!(SA, m / 2) = t;
        }
    }

    i = m / 2 - 1;
    while 0 <= i {
        tr_fixdown(ISAd, SA, i, m);
        i -= 1;
    }
    if (size % 2) == 0 {
        t = *po!(SA, 0);
        *po!(SA, 0) = *po!(SA, m);
        *po!(SA, m) = t;
        tr_fixdown(ISAd, SA, 0, m);
    }
    i = m - 1;
    while 0 < i {
        t = *po!(SA, 0);
        *po!(SA, 0) = *po!(SA, i);
        tr_fixdown(ISAd, SA, 0, i);
        *po!(SA, i) = t;
        i -= 1;
    }
}

/*---------------------------------------------------------------------------*/
/* Returns the median of three elements. */
#[inline]
unsafe fn tr_median3(ISAd: *const i32, mut v1: *mut i32, mut v2: *mut i32, v3: *mut i32) -> *mut i32 {
    if isad!(ISAd, v1) > isad!(ISAd, v2) {
        let t = v1;
        v1 = v2;
        v2 = t;
    }
    if isad!(ISAd, v2) > isad!(ISAd, v3) {
        if isad!(ISAd, v1) > isad!(ISAd, v3) {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

/* Returns the median of five elements. */
#[inline]
unsafe fn tr_median5(
    ISAd: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    mut v3: *mut i32,
    mut v4: *mut i32,
    mut v5: *mut i32,
) -> *mut i32 {
    let mut t: *mut i32;
    if isad!(ISAd, v2) > isad!(ISAd, v3) {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if isad!(ISAd, v4) > isad!(ISAd, v5) {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if isad!(ISAd, v2) > isad!(ISAd, v4) {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if isad!(ISAd, v1) > isad!(ISAd, v3) {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if isad!(ISAd, v1) > isad!(ISAd, v4) {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if isad!(ISAd, v3) > isad!(ISAd, v4) {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
#[inline]
unsafe fn tr_pivot(ISAd: *const i32, mut first: *mut i32, mut last: *mut i32) -> *mut i32 {
    let mut middle: *mut i32;
    let mut t: i32;

    t = pd!(last, first);
    middle = po!(first, t / 2);

    if t <= 512 {
        if t <= 32 {
            return tr_median3(ISAd, first, middle, po!(last, -1));
        } else {
            t >>= 2;
            return tr_median5(ISAd, first, po!(first, t), middle, po!(last, -1 - t), po!(last, -1));
        }
    }
    t >>= 3;
    first = tr_median3(ISAd, first, po!(first, t), po!(first, t << 1));
    middle = tr_median3(ISAd, po!(middle, -t), middle, po!(middle, t));
    last = tr_median3(ISAd, po!(last, -1 - (t << 1)), po!(last, -1 - t), po!(last, -1));
    tr_median3(ISAd, first, middle, last)
}

/*---------------------------------------------------------------------------*/
#[derive(Clone, Copy)]
struct trbudget_t {
    chance: i32,
    remain: i32,
    incval: i32,
    count: i32,
}

#[inline]
fn trbudget_init(budget: &mut trbudget_t, chance: i32, incval: i32) {
    budget.chance = chance;
    budget.incval = incval;
    budget.remain = incval;
}

#[inline]
fn trbudget_check(budget: &mut trbudget_t, size: i32) -> i32 {
    if size <= budget.remain {
        budget.remain -= size;
        return 1;
    }
    if budget.chance == 0 {
        budget.count += size;
        return 0;
    }
    budget.remain += budget.incval - size;
    budget.chance -= 1;
    1
}

/*---------------------------------------------------------------------------*/
#[inline]
unsafe fn tr_partition(
    ISAd: *const i32,
    mut first: *mut i32,
    middle: *mut i32,
    mut last: *mut i32,
    pa: *mut *mut i32,
    pb: *mut *mut i32,
    v: i32,
) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut c: *mut i32;
    let mut d: *mut i32;
    let mut e: *mut i32;
    let mut f: *mut i32;
    let mut t: i32;
    let mut s: i32;
    let mut x: i32 = 0;

    b = po!(middle, -1);
    loop {
        b = po!(b, 1);
        if !(b < last) {
            break;
        }
        x = *po!(ISAd, *b);
        if x != v {
            break;
        }
    }
    a = b;
    if (a < last) && (x < v) {
        loop {
            b = po!(b, 1);
            if !(b < last) {
                break;
            }
            x = *po!(ISAd, *b);
            if !(x <= v) {
                break;
            }
            if x == v {
                let tt = *b;
                *b = *a;
                *a = tt;
                a = po!(a, 1);
            }
        }
    }
    c = last;
    loop {
        c = po!(c, -1);
        if !(b < c) {
            break;
        }
        x = *po!(ISAd, *c);
        if x != v {
            break;
        }
    }
    d = c;
    if (b < d) && (x > v) {
        loop {
            c = po!(c, -1);
            if !(b < c) {
                break;
            }
            x = *po!(ISAd, *c);
            if !(x >= v) {
                break;
            }
            if x == v {
                let tt = *c;
                *c = *d;
                *d = tt;
                d = po!(d, -1);
            }
        }
    }
    while b < c {
        {
            let tt = *b;
            *b = *c;
            *c = tt;
        }
        loop {
            b = po!(b, 1);
            if !(b < c) {
                break;
            }
            x = *po!(ISAd, *b);
            if !(x <= v) {
                break;
            }
            if x == v {
                let tt = *b;
                *b = *a;
                *a = tt;
                a = po!(a, 1);
            }
        }
        loop {
            c = po!(c, -1);
            if !(b < c) {
                break;
            }
            x = *po!(ISAd, *c);
            if !(x >= v) {
                break;
            }
            if x == v {
                let tt = *c;
                *c = *d;
                *d = tt;
                d = po!(d, -1);
            }
        }
    }

    if a <= d {
        c = po!(b, -1);
        s = pd!(a, first);
        t = pd!(b, a);
        if s > t {
            s = t;
        }
        e = first;
        f = po!(b, -s);
        while 0 < s {
            let tt = *e;
            *e = *f;
            *f = tt;
            s -= 1;
            e = po!(e, 1);
            f = po!(f, 1);
        }
        s = pd!(d, c);
        t = pd!(last, d) - 1;
        if s > t {
            s = t;
        }
        e = b;
        f = po!(last, -s);
        while 0 < s {
            let tt = *e;
            *e = *f;
            *f = tt;
            s -= 1;
            e = po!(e, 1);
            f = po!(f, 1);
        }
        first = po!(first, pd!(b, a));
        last = po!(last, -pd!(d, c));
    }
    *pa = first;
    *pb = last;
}

unsafe fn tr_copy(
    ISA: *mut i32,
    SA: *const i32,
    first: *mut i32,
    a: *mut i32,
    b: *mut i32,
    last: *mut i32,
    depth: i32,
) {
    let mut c: *mut i32;
    let mut d: *mut i32;
    let mut e: *mut i32;
    let mut s: i32;
    let v: i32;

    v = pd!(b, SA) - 1;
    c = first;
    d = po!(a, -1);
    while c <= d {
        s = *c - depth;
        if (0 <= s) && (*po!(ISA, s) == v) {
            d = po!(d, 1);
            *d = s;
            *po!(ISA, s) = pd!(d, SA);
        }
        c = po!(c, 1);
    }
    c = po!(last, -1);
    e = po!(d, 1);
    d = b;
    while e < d {
        s = *c - depth;
        if (0 <= s) && (*po!(ISA, s) == v) {
            d = po!(d, -1);
            *d = s;
            *po!(ISA, s) = pd!(d, SA);
        }
        c = po!(c, -1);
    }
}

unsafe fn tr_partialcopy(
    ISA: *mut i32,
    SA: *const i32,
    first: *mut i32,
    a: *mut i32,
    b: *mut i32,
    last: *mut i32,
    depth: i32,
) {
    let mut c: *mut i32;
    let mut d: *mut i32;
    let mut e: *mut i32;
    let mut s: i32;
    let v: i32;
    let mut rank: i32;
    let mut lastrank: i32;
    let mut newrank: i32 = -1;

    v = pd!(b, SA) - 1;
    lastrank = -1;
    c = first;
    d = po!(a, -1);
    while c <= d {
        s = *c - depth;
        if (0 <= s) && (*po!(ISA, s) == v) {
            d = po!(d, 1);
            *d = s;
            rank = *po!(ISA, s + depth);
            if lastrank != rank {
                lastrank = rank;
                newrank = pd!(d, SA);
            }
            *po!(ISA, s) = newrank;
        }
        c = po!(c, 1);
    }

    lastrank = -1;
    e = d;
    while first <= e {
        rank = *po!(ISA, *e);
        if lastrank != rank {
            lastrank = rank;
            newrank = pd!(e, SA);
        }
        if newrank != rank {
            *po!(ISA, *e) = newrank;
        }
        e = po!(e, -1);
    }

    lastrank = -1;
    c = po!(last, -1);
    e = po!(d, 1);
    d = b;
    while e < d {
        s = *c - depth;
        if (0 <= s) && (*po!(ISA, s) == v) {
            d = po!(d, -1);
            *d = s;
            rank = *po!(ISA, s + depth);
            if lastrank != rank {
                lastrank = rank;
                newrank = pd!(d, SA);
            }
            *po!(ISA, s) = newrank;
        }
        c = po!(c, -1);
    }
}

unsafe fn tr_introsort(
    ISA: *mut i32,
    mut ISAd: *const i32,
    SA: *mut i32,
    mut first: *mut i32,
    mut last: *mut i32,
    budget: &mut trbudget_t,
) {
    const STACK_SIZE: usize = TR_STACKSIZE;
    #[derive(Clone, Copy)]
    struct StackEntry {
        a: *const i32,
        b: *mut i32,
        c: *mut i32,
        d: i32,
        e: i32,
    }
    let mut stack = [StackEntry {
        a: core::ptr::null(),
        b: core::ptr::null_mut(),
        c: core::ptr::null_mut(),
        d: 0,
        e: 0,
    }; STACK_SIZE];
    let mut ssize: i32 = 0;
    let mut trlink: i32 = -1;

    macro_rules! STACK_PUSH5 {
        ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {{
            debug_assert!((ssize as usize) < STACK_SIZE);
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            stack[ssize as usize].e = $e;
            ssize += 1;
        }};
    }
    macro_rules! STACK_POP5 {
        ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {{
            debug_assert!(0 <= ssize);
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            $a = stack[ssize as usize].a;
            $b = stack[ssize as usize].b;
            $c = stack[ssize as usize].c;
            $d = stack[ssize as usize].d;
            $e = stack[ssize as usize].e;
        }};
    }

    let mut a: *mut i32 = core::ptr::null_mut();
    let mut b: *mut i32 = core::ptr::null_mut();
    let mut c: *mut i32;
    let mut t: i32;
    let mut v: i32;
    let mut x: i32 = 0;
    let incr: i32 = pd!(ISAd, ISA);
    let mut limit: i32;
    let mut next: i32;

    limit = tr_ilg(pd!(last, first));
    loop {
        if limit < 0 {
            if limit == -1 {
                /* tandem repeat partition */
                tr_partition(
                    po!(ISAd, -incr),
                    first,
                    first,
                    last,
                    &mut a,
                    &mut b,
                    pd!(last, SA) - 1,
                );

                /* update ranks */
                if a < last {
                    c = first;
                    v = pd!(a, SA) - 1;
                    while c < a {
                        *po!(ISA, *c) = v;
                        c = po!(c, 1);
                    }
                }
                if b < last {
                    c = a;
                    v = pd!(b, SA) - 1;
                    while c < b {
                        *po!(ISA, *c) = v;
                        c = po!(c, 1);
                    }
                }

                /* push */
                if 1 < pd!(b, a) {
                    STACK_PUSH5!(core::ptr::null(), a, b, 0, 0);
                    STACK_PUSH5!(po!(ISAd, -incr), first, last, -2, trlink);
                    trlink = ssize - 2;
                }
                if pd!(a, first) <= pd!(last, b) {
                    if 1 < pd!(a, first) {
                        STACK_PUSH5!(ISAd, b, last, tr_ilg(pd!(last, b)), trlink);
                        last = a;
                        limit = tr_ilg(pd!(a, first));
                    } else if 1 < pd!(last, b) {
                        first = b;
                        limit = tr_ilg(pd!(last, b));
                    } else {
                        STACK_POP5!(ISAd, first, last, limit, trlink);
                    }
                } else {
                    if 1 < pd!(last, b) {
                        STACK_PUSH5!(ISAd, first, a, tr_ilg(pd!(a, first)), trlink);
                        first = b;
                        limit = tr_ilg(pd!(last, b));
                    } else if 1 < pd!(a, first) {
                        last = a;
                        limit = tr_ilg(pd!(a, first));
                    } else {
                        STACK_POP5!(ISAd, first, last, limit, trlink);
                    }
                }
            } else if limit == -2 {
                /* tandem repeat copy */
                ssize -= 1;
                a = stack[ssize as usize].b;
                b = stack[ssize as usize].c;
                if stack[ssize as usize].d == 0 {
                    tr_copy(ISA, SA, first, a, b, last, pd!(ISAd, ISA));
                } else {
                    if 0 <= trlink {
                        stack[trlink as usize].d = -1;
                    }
                    tr_partialcopy(ISA, SA, first, a, b, last, pd!(ISAd, ISA));
                }
                STACK_POP5!(ISAd, first, last, limit, trlink);
            } else {
                /* sorted partition */
                if 0 <= *first {
                    a = first;
                    loop {
                        *po!(ISA, *a) = pd!(a, SA);
                        a = po!(a, 1);
                        if !((a < last) && (0 <= *a)) {
                            break;
                        }
                    }
                    first = a;
                }
                if first < last {
                    a = first;
                    loop {
                        *a = !*a;
                        a = po!(a, 1);
                        if !(*a < 0) {
                            break;
                        }
                    }
                    next = if *po!(ISA, *a) != *po!(ISAd, *a) {
                        tr_ilg(pd!(a, first) + 1)
                    } else {
                        -1
                    };
                    a = po!(a, 1);
                    if a < last {
                        b = first;
                        v = pd!(a, SA) - 1;
                        while b < a {
                            *po!(ISA, *b) = v;
                            b = po!(b, 1);
                        }
                    }

                    /* push */
                    if trbudget_check(budget, pd!(a, first)) != 0 {
                        if pd!(a, first) <= pd!(last, a) {
                            STACK_PUSH5!(ISAd, a, last, -3, trlink);
                            ISAd = po!(ISAd, incr);
                            last = a;
                            limit = next;
                        } else {
                            if 1 < pd!(last, a) {
                                STACK_PUSH5!(po!(ISAd, incr), first, a, next, trlink);
                                first = a;
                                limit = -3;
                            } else {
                                ISAd = po!(ISAd, incr);
                                last = a;
                                limit = next;
                            }
                        }
                    } else {
                        if 0 <= trlink {
                            stack[trlink as usize].d = -1;
                        }
                        if 1 < pd!(last, a) {
                            first = a;
                            limit = -3;
                        } else {
                            STACK_POP5!(ISAd, first, last, limit, trlink);
                        }
                    }
                } else {
                    STACK_POP5!(ISAd, first, last, limit, trlink);
                }
            }
            continue;
        }

        if pd!(last, first) <= TR_INSERTIONSORT_THRESHOLD {
            tr_insertionsort(ISAd, first, last);
            limit = -3;
            continue;
        }

        let old_limit = limit;
        limit -= 1;
        if old_limit == 0 {
            tr_heapsort(ISAd, first, pd!(last, first));
            a = po!(last, -1);
            while first < a {
                x = *po!(ISAd, *a);
                b = po!(a, -1);
                while (first <= b) && (*po!(ISAd, *b) == x) {
                    *b = !*b;
                    b = po!(b, -1);
                }
                a = b;
            }
            limit = -3;
            continue;
        }

        /* choose pivot */
        a = tr_pivot(ISAd, first, last);
        {
            let tt = *first;
            *first = *a;
            *a = tt;
        }
        v = *po!(ISAd, *first);

        /* partition */
        tr_partition(ISAd, first, po!(first, 1), last, &mut a, &mut b, v);
        if pd!(last, first) != pd!(b, a) {
            next = if *po!(ISA, *a) != v {
                tr_ilg(pd!(b, a))
            } else {
                -1
            };

            /* update ranks */
            c = first;
            v = pd!(a, SA) - 1;
            while c < a {
                *po!(ISA, *c) = v;
                c = po!(c, 1);
            }
            if b < last {
                c = a;
                v = pd!(b, SA) - 1;
                while c < b {
                    *po!(ISA, *c) = v;
                    c = po!(c, 1);
                }
            }

            /* push */
            if (1 < pd!(b, a)) && (trbudget_check(budget, pd!(b, a)) != 0) {
                if pd!(a, first) <= pd!(last, b) {
                    if pd!(last, b) <= pd!(b, a) {
                        if 1 < pd!(a, first) {
                            STACK_PUSH5!(po!(ISAd, incr), a, b, next, trlink);
                            STACK_PUSH5!(ISAd, b, last, limit, trlink);
                            last = a;
                        } else if 1 < pd!(last, b) {
                            STACK_PUSH5!(po!(ISAd, incr), a, b, next, trlink);
                            first = b;
                        } else {
                            ISAd = po!(ISAd, incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if pd!(a, first) <= pd!(b, a) {
                        if 1 < pd!(a, first) {
                            STACK_PUSH5!(ISAd, b, last, limit, trlink);
                            STACK_PUSH5!(po!(ISAd, incr), a, b, next, trlink);
                            last = a;
                        } else {
                            STACK_PUSH5!(ISAd, b, last, limit, trlink);
                            ISAd = po!(ISAd, incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        STACK_PUSH5!(ISAd, b, last, limit, trlink);
                        STACK_PUSH5!(ISAd, first, a, limit, trlink);
                        ISAd = po!(ISAd, incr);
                        first = a;
                        last = b;
                        limit = next;
                    }
                } else {
                    if pd!(a, first) <= pd!(b, a) {
                        if 1 < pd!(last, b) {
                            STACK_PUSH5!(po!(ISAd, incr), a, b, next, trlink);
                            STACK_PUSH5!(ISAd, first, a, limit, trlink);
                            first = b;
                        } else if 1 < pd!(a, first) {
                            STACK_PUSH5!(po!(ISAd, incr), a, b, next, trlink);
                            last = a;
                        } else {
                            ISAd = po!(ISAd, incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if pd!(last, b) <= pd!(b, a) {
                        if 1 < pd!(last, b) {
                            STACK_PUSH5!(ISAd, first, a, limit, trlink);
                            STACK_PUSH5!(po!(ISAd, incr), a, b, next, trlink);
                            first = b;
                        } else {
                            STACK_PUSH5!(ISAd, first, a, limit, trlink);
                            ISAd = po!(ISAd, incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        STACK_PUSH5!(ISAd, first, a, limit, trlink);
                        STACK_PUSH5!(ISAd, b, last, limit, trlink);
                        ISAd = po!(ISAd, incr);
                        first = a;
                        last = b;
                        limit = next;
                    }
                }
            } else {
                if (1 < pd!(b, a)) && (0 <= trlink) {
                    stack[trlink as usize].d = -1;
                }
                if pd!(a, first) <= pd!(last, b) {
                    if 1 < pd!(a, first) {
                        STACK_PUSH5!(ISAd, b, last, limit, trlink);
                        last = a;
                    } else if 1 < pd!(last, b) {
                        first = b;
                    } else {
                        STACK_POP5!(ISAd, first, last, limit, trlink);
                    }
                } else {
                    if 1 < pd!(last, b) {
                        STACK_PUSH5!(ISAd, first, a, limit, trlink);
                        first = b;
                    } else if 1 < pd!(a, first) {
                        last = a;
                    } else {
                        STACK_POP5!(ISAd, first, last, limit, trlink);
                    }
                }
            }
        } else {
            if trbudget_check(budget, pd!(last, first)) != 0 {
                limit = tr_ilg(pd!(last, first));
                ISAd = po!(ISAd, incr);
            } else {
                if 0 <= trlink {
                    stack[trlink as usize].d = -1;
                }
                STACK_POP5!(ISAd, first, last, limit, trlink);
            }
        }
    }
}

/*---------------------------------------------------------------------------*/
/* Tandem repeat sort */
unsafe fn trsort(ISA: *mut i32, SA: *mut i32, n: i32, depth: i32) {
    let mut ISAd: *mut i32;
    let mut first: *mut i32;
    let mut last: *mut i32;
    let mut budget = trbudget_t {
        chance: 0,
        remain: 0,
        incval: 0,
        count: 0,
    };
    let mut t: i32;
    let mut skip: i32;
    let mut unsorted: i32;

    trbudget_init(&mut budget, tr_ilg(n) * 2 / 3, n);
    ISAd = po!(ISA, depth);
    while -n < *SA {
        first = SA;
        skip = 0;
        unsorted = 0;
        loop {
            t = *first;
            if t < 0 {
                first = po!(first, -t);
                skip += t;
            } else {
                if skip != 0 {
                    *po!(first, skip) = skip;
                    skip = 0;
                }
                last = po!(SA, *po!(ISA, t) + 1);
                if 1 < pd!(last, first) {
                    budget.count = 0;
                    tr_introsort(ISA, ISAd, SA, first, last, &mut budget);
                    if budget.count != 0 {
                        unsorted += budget.count;
                    } else {
                        skip = pd!(first, last);
                    }
                } else if pd!(last, first) == 1 {
                    skip = -1;
                }
                first = last;
            }
            if !(first < po!(SA, n)) {
                break;
            }
        }
        if skip != 0 {
            *po!(first, skip) = skip;
        }
        if unsorted == 0 {
            break;
        }
        // ISAd += ISAd - ISA
        ISAd = po!(ISAd, pd!(ISAd, ISA));
    }
}

/*---------------------------------------------------------------------------*/
/* Sorts suffixes of type B*. */
unsafe fn sort_typeBstar(
    T: *const u8,
    SA: *mut i32,
    bucket_A: *mut i32,
    bucket_B: *mut i32,
    n: i32,
    _openMP: i32,
) -> i32 {
    let PAb: *mut i32;
    let ISAb: *mut i32;
    let mut buf: *mut i32;
    let mut i: i32;
    let mut j: i32;
    let mut k: i32;
    let mut t: i32;
    let mut m: i32;
    let mut bufsize: i32;
    let mut c0: i32;
    let mut c1: i32;

    // BUCKET macros
    macro_rules! BUCKET_A {
        ($c0:expr) => {
            *bucket_A.wrapping_offset(($c0) as isize)
        };
    }
    macro_rules! BUCKET_B {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c1) << 8) | ($c0)) as isize)
        };
    }
    macro_rules! BUCKET_BSTAR {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c0) << 8) | ($c1)) as isize)
        };
    }

    /* Initialize bucket arrays. */
    i = 0;
    while i < BUCKET_A_SIZE as i32 {
        *bucket_A.wrapping_offset(i as isize) = 0;
        i += 1;
    }
    i = 0;
    while i < BUCKET_B_SIZE as i32 {
        *bucket_B.wrapping_offset(i as isize) = 0;
        i += 1;
    }

    /* Count the number of occurrences ... store B* suffixes into SA. */
    i = n - 1;
    m = n;
    c0 = tget(T, n - 1);
    while 0 <= i {
        /* type A suffix. */
        loop {
            c1 = c0;
            BUCKET_A!(c1) += 1;
            i -= 1;
            if !(0 <= i) {
                break;
            }
            c0 = tget(T, i);
            if !(c0 >= c1) {
                break;
            }
        }
        if 0 <= i {
            /* type B* suffix. */
            BUCKET_BSTAR!(c0, c1) += 1;
            m -= 1;
            *po!(SA, m) = i;
            /* type B suffix. */
            i -= 1;
            c1 = c0;
            while (0 <= i) && ({
                c0 = tget(T, i);
                c0 <= c1
            }) {
                BUCKET_B!(c0, c1) += 1;
                i -= 1;
                c1 = c0;
            }
        }
    }
    m = n - m;

    /* Calculate the index of start/end point of each bucket. */
    c0 = 0;
    i = 0;
    j = 0;
    while c0 < ALPHABET_SIZE {
        t = i + BUCKET_A!(c0);
        BUCKET_A!(c0) = i + j; /* start point */
        i = t + BUCKET_B!(c0, c0);
        c1 = c0 + 1;
        while c1 < ALPHABET_SIZE {
            j += BUCKET_BSTAR!(c0, c1);
            BUCKET_BSTAR!(c0, c1) = j; /* end point */
            i += BUCKET_B!(c0, c1);
            c1 += 1;
        }
        c0 += 1;
    }

    if 0 < m {
        /* Sort the type B* suffixes by their first two characters. */
        PAb = po!(SA, n - m);
        ISAb = po!(SA, m);
        i = m - 2;
        while 0 <= i {
            t = *po!(PAb, i);
            c0 = tget(T, t);
            c1 = tget(T, t + 1);
            BUCKET_BSTAR!(c0, c1) -= 1;
            *po!(SA, BUCKET_BSTAR!(c0, c1)) = i;
            i -= 1;
        }
        t = *po!(PAb, m - 1);
        c0 = tget(T, t);
        c1 = tget(T, t + 1);
        BUCKET_BSTAR!(c0, c1) -= 1;
        *po!(SA, BUCKET_BSTAR!(c0, c1)) = m - 1;

        /* Sort the type B* substrings using sssort. */
        buf = po!(SA, m);
        bufsize = n - (2 * m);
        c0 = ALPHABET_SIZE - 2;
        j = m;
        while 0 < j {
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                i = BUCKET_BSTAR!(c0, c1);
                if 1 < (j - i) {
                    sssort(
                        T,
                        PAb,
                        po!(SA, i),
                        po!(SA, j),
                        buf,
                        bufsize,
                        2,
                        n,
                        (*po!(SA, i) == (m - 1)) as i32,
                    );
                }
                j = i;
                c1 -= 1;
            }
            c0 -= 1;
        }

        /* Compute ranks of type B* substrings. */
        i = m - 1;
        while 0 <= i {
            if 0 <= *po!(SA, i) {
                j = i;
                loop {
                    *po!(ISAb, *po!(SA, i)) = i;
                    i -= 1;
                    if !((0 <= i) && (0 <= *po!(SA, i))) {
                        break;
                    }
                }
                *po!(SA, i + 1) = i - j;
                if i <= 0 {
                    break;
                }
            }
            j = i;
            loop {
                *po!(SA, i) = !*po!(SA, i);
                *po!(ISAb, *po!(SA, i)) = j;
                i -= 1;
                if !(*po!(SA, i) < 0) {
                    break;
                }
            }
            *po!(ISAb, *po!(SA, i)) = j;
            i -= 1;
        }

        /* Construct the inverse suffix array of type B* suffixes using trsort. */
        trsort(ISAb, SA, m, 1);

        /* Set the sorted order of type B* suffixes. */
        i = n - 1;
        j = m;
        c0 = tget(T, n - 1);
        while 0 <= i {
            i -= 1;
            c1 = c0;
            while (0 <= i) && ({
                c0 = tget(T, i);
                c0 >= c1
            }) {
                i -= 1;
                c1 = c0;
            }
            if 0 <= i {
                t = i;
                i -= 1;
                c1 = c0;
                while (0 <= i) && ({
                    c0 = tget(T, i);
                    c0 <= c1
                }) {
                    i -= 1;
                    c1 = c0;
                }
                j -= 1;
                *po!(SA, *po!(ISAb, j)) = if (t == 0) || (1 < (t - i)) { t } else { !t };
            }
        }

        /* Calculate the index of start/end point of each bucket. */
        BUCKET_B!(ALPHABET_SIZE - 1, ALPHABET_SIZE - 1) = n; /* end point */
        c0 = ALPHABET_SIZE - 2;
        k = m - 1;
        while 0 <= c0 {
            i = BUCKET_A!(c0 + 1) - 1;
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                t = i - BUCKET_B!(c0, c1);
                BUCKET_B!(c0, c1) = i; /* end point */

                /* Move all type B* suffixes to the correct position. */
                i = t;
                j = BUCKET_BSTAR!(c0, c1);
                while j <= k {
                    *po!(SA, i) = *po!(SA, k);
                    i -= 1;
                    k -= 1;
                }
                c1 -= 1;
            }
            BUCKET_BSTAR!(c0, c0 + 1) = i - BUCKET_B!(c0, c0) + 1; /* start point */
            BUCKET_B!(c0, c0) = i; /* end point */
            c0 -= 1;
        }
    }

    m
}

/* Constructs the suffix array by using the sorted order of type B* suffixes. */
unsafe fn construct_SA(
    T: *const u8,
    SA: *mut i32,
    bucket_A: *mut i32,
    bucket_B: *mut i32,
    n: i32,
    m: i32,
) {
    macro_rules! BUCKET_A {
        ($c0:expr) => {
            *bucket_A.wrapping_offset(($c0) as isize)
        };
    }
    macro_rules! BUCKET_B {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c1) << 8) | ($c0)) as isize)
        };
    }
    macro_rules! BUCKET_BSTAR {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c0) << 8) | ($c1)) as isize)
        };
    }

    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut k: *mut i32;
    let mut s: i32;
    let mut c0: i32;
    let mut c1: i32;
    let mut c2: i32;

    if 0 < m {
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            i = po!(SA, BUCKET_BSTAR!(c1, c1 + 1));
            j = po!(SA, BUCKET_A!(c1 + 1) - 1);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    debug_assert!(tget(T, s) == c1);
                    debug_assert!(((s + 1) < n) && (tget(T, s) <= tget(T, s + 1)));
                    debug_assert!(tget(T, s - 1) <= tget(T, s));
                    *j = !s;
                    s -= 1;
                    c0 = tget(T, s);
                    if (0 < s) && (tget(T, s - 1) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            BUCKET_B!(c2, c1) = pd!(k, SA);
                        }
                        c2 = c0;
                        k = po!(SA, BUCKET_B!(c2, c1));
                    }
                    debug_assert!(k < j);
                    *k = s;
                    k = po!(k, -1);
                } else {
                    debug_assert!(((s == 0) && (tget(T, s) == c1)) || (s < 0));
                    *j = !s;
                }
                j = po!(j, -1);
            }
            c1 -= 1;
        }
    }

    /* Construct the suffix array by using the sorted order of type B suffixes. */
    c2 = tget(T, n - 1);
    k = po!(SA, BUCKET_A!(c2));
    *k = if tget(T, n - 2) < c2 { !(n - 1) } else { n - 1 };
    k = po!(k, 1);
    /* Scan the suffix array from left to right. */
    i = SA;
    j = po!(SA, n);
    while i < j {
        s = *i;
        if 0 < s {
            debug_assert!(tget(T, s - 1) >= tget(T, s));
            s -= 1;
            c0 = tget(T, s);
            if (s == 0) || (tget(T, s - 1) < c0) {
                s = !s;
            }
            if c0 != c2 {
                BUCKET_A!(c2) = pd!(k, SA);
                c2 = c0;
                k = po!(SA, BUCKET_A!(c2));
            }
            debug_assert!(i < k);
            *k = s;
            k = po!(k, 1);
        } else {
            debug_assert!(s < 0);
            *i = !s;
        }
        i = po!(i, 1);
    }
}

/* Constructs the BWT string directly by using the sorted order of type B* suffixes. */
unsafe fn construct_BWT(
    T: *const u8,
    SA: *mut i32,
    bucket_A: *mut i32,
    bucket_B: *mut i32,
    n: i32,
    m: i32,
) -> i32 {
    macro_rules! BUCKET_A {
        ($c0:expr) => {
            *bucket_A.wrapping_offset(($c0) as isize)
        };
    }
    macro_rules! BUCKET_B {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c1) << 8) | ($c0)) as isize)
        };
    }
    macro_rules! BUCKET_BSTAR {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c0) << 8) | ($c1)) as isize)
        };
    }

    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut k: *mut i32;
    let mut orig: *mut i32;
    let mut s: i32;
    let mut c0: i32;
    let mut c1: i32;
    let mut c2: i32;

    if 0 < m {
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            i = po!(SA, BUCKET_BSTAR!(c1, c1 + 1));
            j = po!(SA, BUCKET_A!(c1 + 1) - 1);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    debug_assert!(tget(T, s) == c1);
                    debug_assert!(((s + 1) < n) && (tget(T, s) <= tget(T, s + 1)));
                    debug_assert!(tget(T, s - 1) <= tget(T, s));
                    s -= 1;
                    c0 = tget(T, s);
                    *j = !c0;
                    if (0 < s) && (tget(T, s - 1) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            BUCKET_B!(c2, c1) = pd!(k, SA);
                        }
                        c2 = c0;
                        k = po!(SA, BUCKET_B!(c2, c1));
                    }
                    debug_assert!(k < j);
                    *k = s;
                    k = po!(k, -1);
                } else if s != 0 {
                    *j = !s;
                } else {
                    debug_assert!(tget(T, s) == c1);
                }
                j = po!(j, -1);
            }
            c1 -= 1;
        }
    }

    /* Construct the BWTed string by using the sorted order of type B suffixes. */
    c2 = tget(T, n - 1);
    k = po!(SA, BUCKET_A!(c2));
    *k = if tget(T, n - 2) < c2 {
        !tget(T, n - 2)
    } else {
        n - 1
    };
    k = po!(k, 1);
    /* Scan the suffix array from left to right. */
    i = SA;
    j = po!(SA, n);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            debug_assert!(tget(T, s - 1) >= tget(T, s));
            s -= 1;
            c0 = tget(T, s);
            *i = c0;
            if (0 < s) && (tget(T, s - 1) < c0) {
                s = !tget(T, s - 1);
            }
            if c0 != c2 {
                BUCKET_A!(c2) = pd!(k, SA);
                c2 = c0;
                k = po!(SA, BUCKET_A!(c2));
            }
            debug_assert!(i < k);
            *k = s;
            k = po!(k, 1);
        } else if s != 0 {
            *i = !s;
        } else {
            orig = i;
        }
        i = po!(i, 1);
    }

    pd!(orig, SA)
}

/* Constructs the BWT string directly, computing secondary indexes. */
unsafe fn construct_BWT_indexes(
    T: *const u8,
    SA: *mut i32,
    bucket_A: *mut i32,
    bucket_B: *mut i32,
    n: i32,
    m: i32,
    num_indexes: *mut u8,
    indexes: *mut i32,
) -> i32 {
    macro_rules! BUCKET_A {
        ($c0:expr) => {
            *bucket_A.wrapping_offset(($c0) as isize)
        };
    }
    macro_rules! BUCKET_B {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c1) << 8) | ($c0)) as isize)
        };
    }
    macro_rules! BUCKET_BSTAR {
        ($c0:expr, $c1:expr) => {
            *bucket_B.wrapping_offset(((($c0) << 8) | ($c1)) as isize)
        };
    }

    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut k: *mut i32;
    let mut orig: *mut i32;
    let mut s: i32;
    let mut c0: i32;
    let mut c1: i32;
    let mut c2: i32;

    let mut mod_: i32 = n / 8;
    {
        mod_ |= mod_ >> 1;
        mod_ |= mod_ >> 2;
        mod_ |= mod_ >> 4;
        mod_ |= mod_ >> 8;
        mod_ |= mod_ >> 16;
        mod_ >>= 1;

        *num_indexes = ((n - 1) / (mod_ + 1)) as u8;
    }

    if 0 < m {
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            i = po!(SA, BUCKET_BSTAR!(c1, c1 + 1));
            j = po!(SA, BUCKET_A!(c1 + 1) - 1);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    debug_assert!(tget(T, s) == c1);
                    debug_assert!(((s + 1) < n) && (tget(T, s) <= tget(T, s + 1)));
                    debug_assert!(tget(T, s - 1) <= tget(T, s));

                    if (s & mod_) == 0 {
                        *indexes.wrapping_offset((s / (mod_ + 1) - 1) as isize) = pd!(j, SA);
                    }

                    s -= 1;
                    c0 = tget(T, s);
                    *j = !c0;
                    if (0 < s) && (tget(T, s - 1) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            BUCKET_B!(c2, c1) = pd!(k, SA);
                        }
                        c2 = c0;
                        k = po!(SA, BUCKET_B!(c2, c1));
                    }
                    debug_assert!(k < j);
                    *k = s;
                    k = po!(k, -1);
                } else if s != 0 {
                    *j = !s;
                } else {
                    debug_assert!(tget(T, s) == c1);
                }
                j = po!(j, -1);
            }
            c1 -= 1;
        }
    }

    /* Construct the BWTed string by using the sorted order of type B suffixes. */
    c2 = tget(T, n - 1);
    k = po!(SA, BUCKET_A!(c2));
    if tget(T, n - 2) < c2 {
        if ((n - 1) & mod_) == 0 {
            *indexes.wrapping_offset(((n - 1) / (mod_ + 1) - 1) as isize) = pd!(k, SA);
        }
        *k = !tget(T, n - 2);
    } else {
        *k = n - 1;
    }
    k = po!(k, 1);

    /* Scan the suffix array from left to right. */
    i = SA;
    j = po!(SA, n);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            debug_assert!(tget(T, s - 1) >= tget(T, s));

            if (s & mod_) == 0 {
                *indexes.wrapping_offset((s / (mod_ + 1) - 1) as isize) = pd!(i, SA);
            }

            s -= 1;
            c0 = tget(T, s);
            *i = c0;
            if c0 != c2 {
                BUCKET_A!(c2) = pd!(k, SA);
                c2 = c0;
                k = po!(SA, BUCKET_A!(c2));
            }
            debug_assert!(i < k);
            if (0 < s) && (tget(T, s - 1) < c0) {
                if (s & mod_) == 0 {
                    *indexes.wrapping_offset((s / (mod_ + 1) - 1) as isize) = pd!(k, SA);
                }
                *k = !tget(T, s - 1);
            } else {
                *k = s;
            }
            k = po!(k, 1);
        } else if s != 0 {
            *i = !s;
        } else {
            orig = i;
        }
        i = po!(i, 1);
    }

    pd!(orig, SA)
}

/*---------------------------------------------------------------------------*/
/*- Function -*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divsufsort(T: *const u8, SA: *mut i32, n: i32, openMP: i32) -> i32 {
    let bucket_A: *mut i32;
    let bucket_B: *mut i32;
    let m: i32;
    let mut err: i32 = 0;

    /* Check arguments. */
    if T.is_null() || SA.is_null() || (n < 0) {
        return -1;
    } else if n == 0 {
        return 0;
    } else if n == 1 {
        *SA = 0;
        return 0;
    } else if n == 2 {
        let mm = (tget(T, 0) < tget(T, 1)) as i32;
        *po!(SA, mm ^ 1) = 0;
        *po!(SA, mm) = 1;
        return 0;
    }

    bucket_A = malloc(BUCKET_A_SIZE * core::mem::size_of::<i32>()) as *mut i32;
    bucket_B = malloc(BUCKET_B_SIZE * core::mem::size_of::<i32>()) as *mut i32;

    /* Suffixsort. */
    if (!bucket_A.is_null()) && (!bucket_B.is_null()) {
        m = sort_typeBstar(T, SA, bucket_A, bucket_B, n, openMP);
        construct_SA(T, SA, bucket_A, bucket_B, n, m);
    } else {
        err = -2;
    }

    free(bucket_B as *mut c_void);
    free(bucket_A as *mut c_void);

    err
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divbwt(
    T: *const u8,
    U: *mut u8,
    A: *mut i32,
    n: i32,
    num_indexes: *mut u8,
    indexes: *mut i32,
    openMP: i32,
) -> i32 {
    let mut B: *mut i32;
    let bucket_A: *mut i32;
    let bucket_B: *mut i32;
    let m: i32;
    let mut pidx: i32;
    let mut i: i32;

    /* Check arguments. */
    if T.is_null() || U.is_null() || (n < 0) {
        return -1;
    } else if n <= 1 {
        if n == 1 {
            *U = *T;
        }
        return n;
    }

    B = A;
    if B.is_null() {
        B = malloc((n as usize + 1) * core::mem::size_of::<i32>()) as *mut i32;
    }
    bucket_A = malloc(BUCKET_A_SIZE * core::mem::size_of::<i32>()) as *mut i32;
    bucket_B = malloc(BUCKET_B_SIZE * core::mem::size_of::<i32>()) as *mut i32;

    /* Burrows-Wheeler Transform. */
    if (!B.is_null()) && (!bucket_A.is_null()) && (!bucket_B.is_null()) {
        m = sort_typeBstar(T, B, bucket_A, bucket_B, n, openMP);

        if num_indexes.is_null() || indexes.is_null() {
            pidx = construct_BWT(T, B, bucket_A, bucket_B, n, m);
        } else {
            pidx = construct_BWT_indexes(T, B, bucket_A, bucket_B, n, m, num_indexes, indexes);
        }

        /* Copy to output string. */
        *U = tget(T, n - 1) as u8;
        i = 0;
        while i < pidx {
            *U.wrapping_offset((i + 1) as isize) = *po!(B, i) as u8;
            i += 1;
        }
        i += 1;
        while i < n {
            *U.wrapping_offset(i as isize) = *po!(B, i) as u8;
            i += 1;
        }
        pidx += 1;
    } else {
        pidx = -2;
    }

    free(bucket_B as *mut c_void);
    free(bucket_A as *mut c_void);
    if A.is_null() {
        free(B as *mut c_void);
    }

    pidx
}








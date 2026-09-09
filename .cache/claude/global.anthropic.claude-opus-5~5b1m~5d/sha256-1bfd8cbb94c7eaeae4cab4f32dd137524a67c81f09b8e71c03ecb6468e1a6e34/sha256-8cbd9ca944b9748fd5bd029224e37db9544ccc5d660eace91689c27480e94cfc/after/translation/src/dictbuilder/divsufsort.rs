//! Translation of `src/dictBuilder/divsufsort.c` (+ `divsufsort.h`)
//!
//! libdivsufsort-lite, Copyright (c) 2003-2008 Yuta Mori.
//!
//! Notes on the transliteration:
//! * `_OPENMP` / `LIBBSC_OPENMP` are NOT defined, so all OpenMP blocks are
//!   compiled out and the `openMP` parameters are unused.
//! * `assert()` is a no-op (DEBUGLEVEL 0 / NDEBUG) -> dropped.
//! * `ALPHABET_SIZE` is 256, `SS_INSERTIONSORT_THRESHOLD` is 8,
//!   `SS_BLOCKSIZE` is 1024 -> `SS_MISORT_STACKSIZE` is 16.
#![allow(dead_code)]

use crate::cmem::{free, malloc};
use core::ffi::c_void;
use core::ptr::{null, null_mut};

/*- Constants -*/
const ALPHABET_SIZE: i32 = 256;
const BUCKET_A_SIZE: i32 = ALPHABET_SIZE;
const BUCKET_B_SIZE: i32 = ALPHABET_SIZE * ALPHABET_SIZE;
const SS_INSERTIONSORT_THRESHOLD: i32 = 8;
const SS_BLOCKSIZE: i32 = 1024;
/* minstacksize = log(SS_BLOCKSIZE) / log(3) * 2 ; SS_BLOCKSIZE <= 4096 -> 16 */
const SS_MISORT_STACKSIZE: usize = 16;
const SS_SMERGE_STACKSIZE: usize = 32;
const TR_INSERTIONSORT_THRESHOLD: i32 = 8;
const TR_STACKSIZE: usize = 64;

/* `p - q` in units of i32, as C's ptrdiff truncated to int. */
#[inline(always)]
fn pdiff(a: *const i32, b: *const i32) -> i32 {
    (((a as isize).wrapping_sub(b as isize)) / (core::mem::size_of::<i32>() as isize)) as i32
}

/*- Macros -*/
/* GETIDX(a) ((0 <= (a)) ? (a) : (~(a))) */
#[inline(always)]
fn getidx(a: i32) -> i32 {
    if 0 <= a {
        a
    } else {
        !a
    }
}

/* Td[PA[*v]] */
#[inline(always)]
unsafe fn ss_key(Td: *const u8, PA: *const i32, v: *const i32) -> i32 {
    *Td.offset(*PA.offset(*v as isize) as isize) as i32
}

/*- Stack element types (anonymous structs in the C) -*/
#[derive(Clone, Copy)]
struct SsMisortStack {
    a: *mut i32,
    b: *mut i32,
    c: i32,
    d: i32,
}

#[derive(Clone, Copy)]
struct SsSmergeStack {
    a: *mut i32,
    b: *mut i32,
    c: *mut i32,
    d: i32,
}

#[derive(Clone, Copy)]
struct TrStack {
    a: *const i32,
    b: *mut i32,
    c: *mut i32,
    d: i32,
    e: i32,
}

/*- Private Functions -*/

static lg_table: [i32; 256] = [
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

/* SS_BLOCKSIZE == 1024 -> 256 <= SS_BLOCKSIZE branch */
#[inline]
unsafe fn ss_ilg(n: i32) -> i32 {
    if (n & 0xff00) != 0 {
        8 + lg_table[((n >> 8) & 0xff) as usize]
    } else {
        0 + lg_table[((n >> 0) & 0xff) as usize]
    }
}

static sqq_table: [i32; 256] = [
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
unsafe fn ss_isqrt(x: i32) -> i32 {
    let mut y: i32;
    let e: i32;

    if x >= (SS_BLOCKSIZE * SS_BLOCKSIZE) {
        return SS_BLOCKSIZE;
    }
    e = if (x & 0xffff0000u32 as i32) != 0 {
        if (x & 0xff000000u32 as i32) != 0 {
            24 + lg_table[((x >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[((x >> 16) & 0xff) as usize]
        }
    } else {
        if (x & 0x0000ff00) != 0 {
            8 + lg_table[((x >> 8) & 0xff) as usize]
        } else {
            0 + lg_table[((x >> 0) & 0xff) as usize]
        }
    };

    if e >= 16 {
        y = sqq_table[(x >> ((e - 6) - (e & 1))) as usize] << ((e >> 1) - 7);
        if e >= 24 {
            y = (y + 1 + x / y) >> 1;
        }
        y = (y + 1 + x / y) >> 1;
    } else if e >= 8 {
        y = (sqq_table[(x >> ((e - 6) - (e & 1))) as usize] >> (7 - (e >> 1))) + 1;
    } else {
        return sqq_table[x as usize] >> 4;
    }

    if x < y.wrapping_mul(y) {
        y - 1
    } else {
        y
    }
}

/*---------------------------------------------------------------------------*/

/* Compares two suffixes. */
#[inline]
unsafe fn ss_compare(T: *const u8, p1: *const i32, p2: *const i32, depth: i32) -> i32 {
    let mut U1: *const u8;
    let mut U2: *const u8;
    let U1n: *const u8;
    let U2n: *const u8;

    U1 = T.offset(depth.wrapping_add(*p1) as isize);
    U2 = T.offset(depth.wrapping_add(*p2) as isize);
    U1n = T.offset((*p1.offset(1)).wrapping_add(2) as isize);
    U2n = T.offset((*p2.offset(1)).wrapping_add(2) as isize);
    while (U1 < U1n) && (U2 < U2n) && (*U1 == *U2) {
        U1 = U1.offset(1);
        U2 = U2.offset(1);
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
unsafe fn ss_insertionsort(
    T: *const u8,
    PA: *const i32,
    first: *mut i32,
    last: *mut i32,
    depth: i32,
) {
    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut t: i32;
    let mut r: i32;

    i = last.offset(-2);
    while first <= i {
        t = *i;
        j = i.offset(1);
        r = ss_compare(T, PA.offset(t as isize), PA.offset(*j as isize), depth);
        while 0 < r {
            loop {
                *j.offset(-1) = *j;
                j = j.offset(1);
                if !((j < last) && (*j < 0)) {
                    break;
                }
            }
            if last <= j {
                break;
            }
            r = ss_compare(T, PA.offset(t as isize), PA.offset(*j as isize), depth);
        }
        if r == 0 {
            *j = !*j;
        }
        *j.offset(-1) = t;
        i = i.offset(-1);
    }
}

/*---------------------------------------------------------------------------*/

#[inline]
unsafe fn ss_fixdown(Td: *const u8, PA: *const i32, SA: *mut i32, mut i: i32, size: i32) {
    let mut j: i32;
    let mut k: i32;
    let v: i32;
    let c: i32;
    let mut d: i32;
    let mut e: i32;

    v = *SA.offset(i as isize);
    c = *Td.offset(*PA.offset(v as isize) as isize) as i32;
    loop {
        j = (2i32).wrapping_mul(i).wrapping_add(1);
        if !(j < size) {
            break;
        }
        k = j;
        j += 1;
        d = *Td.offset(*PA.offset(*SA.offset(k as isize) as isize) as isize) as i32;
        e = *Td.offset(*PA.offset(*SA.offset(j as isize) as isize) as isize) as i32;
        if d < e {
            k = j;
            d = e;
        }
        if d <= c {
            break;
        }
        *SA.offset(i as isize) = *SA.offset(k as isize);
        i = k;
    }
    *SA.offset(i as isize) = v;
}

/* Simple top-down heapsort. */
unsafe fn ss_heapsort(Td: *const u8, PA: *const i32, SA: *mut i32, size: i32) {
    let mut i: i32;
    let mut m: i32;
    let mut t: i32;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if (*Td.offset(*PA.offset(*SA.offset((m / 2) as isize) as isize) as isize))
            < (*Td.offset(*PA.offset(*SA.offset(m as isize) as isize) as isize))
        {
            t = *SA.offset(m as isize);
            *SA.offset(m as isize) = *SA.offset((m / 2) as isize);
            *SA.offset((m / 2) as isize) = t;
        }
    }

    i = m / 2 - 1;
    while 0 <= i {
        ss_fixdown(Td, PA, SA, i, m);
        i -= 1;
    }
    if (size % 2) == 0 {
        t = *SA.offset(0);
        *SA.offset(0) = *SA.offset(m as isize);
        *SA.offset(m as isize) = t;
        ss_fixdown(Td, PA, SA, 0, m);
    }
    i = m - 1;
    while 0 < i {
        t = *SA.offset(0);
        *SA.offset(0) = *SA.offset(i as isize);
        ss_fixdown(Td, PA, SA, 0, i);
        *SA.offset(i as isize) = t;
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
    let t: *mut i32;
    if ss_key(Td, PA, v1) > ss_key(Td, PA, v2) {
        t = v1;
        v1 = v2;
        v2 = t;
    }
    if ss_key(Td, PA, v2) > ss_key(Td, PA, v3) {
        if ss_key(Td, PA, v1) > ss_key(Td, PA, v3) {
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
    if ss_key(Td, PA, v2) > ss_key(Td, PA, v3) {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if ss_key(Td, PA, v4) > ss_key(Td, PA, v5) {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if ss_key(Td, PA, v2) > ss_key(Td, PA, v4) {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if ss_key(Td, PA, v1) > ss_key(Td, PA, v3) {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if ss_key(Td, PA, v1) > ss_key(Td, PA, v4) {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if ss_key(Td, PA, v3) > ss_key(Td, PA, v4) {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
#[inline]
unsafe fn ss_pivot(
    Td: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    mut last: *mut i32,
) -> *mut i32 {
    let mut middle: *mut i32;
    let mut t: i32;

    t = pdiff(last, first);
    middle = first.offset((t / 2) as isize);

    if t <= 512 {
        if t <= 32 {
            return ss_median3(Td, PA, first, middle, last.offset(-1));
        } else {
            t >>= 2;
            return ss_median5(
                Td,
                PA,
                first,
                first.offset(t as isize),
                middle,
                last.offset(-1 - (t as isize)),
                last.offset(-1),
            );
        }
    }
    t >>= 3;
    first = ss_median3(
        Td,
        PA,
        first,
        first.offset(t as isize),
        first.offset((t << 1) as isize),
    );
    middle = ss_median3(
        Td,
        PA,
        middle.offset(-(t as isize)),
        middle,
        middle.offset(t as isize),
    );
    last = ss_median3(
        Td,
        PA,
        last.offset(-1 - ((t << 1) as isize)),
        last.offset(-1 - (t as isize)),
        last.offset(-1),
    );
    ss_median3(Td, PA, first, middle, last)
}

/*---------------------------------------------------------------------------*/

/* Binary partition for substrings. */
#[inline]
unsafe fn ss_partition(PA: *const i32, first: *mut i32, last: *mut i32, depth: i32) -> *mut i32 {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;

    a = first.offset(-1);
    b = last;
    loop {
        loop {
            a = a.offset(1);
            if !(a < b) {
                break;
            }
            if !((*PA.offset(*a as isize)).wrapping_add(depth)
                >= (*PA.offset((*a).wrapping_add(1) as isize)).wrapping_add(1))
            {
                break;
            }
            *a = !*a;
        }
        loop {
            b = b.offset(-1);
            if !(a < b) {
                break;
            }
            if !((*PA.offset(*b as isize)).wrapping_add(depth)
                < (*PA.offset((*b).wrapping_add(1) as isize)).wrapping_add(1))
            {
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
    let mut stack: [SsMisortStack; STACK_SIZE] = [SsMisortStack {
        a: null_mut(),
        b: null_mut(),
        c: 0,
        d: 0,
    }; STACK_SIZE];
    let mut Td: *const u8;
    let mut a: *mut i32 = null_mut();
    let mut b: *mut i32;
    let mut c: *mut i32;
    let mut d: *mut i32;
    let mut e: *mut i32;
    let mut f: *mut i32;
    let mut s: i32;
    let mut t: i32;
    let mut ssize: i32;
    let mut limit: i32;
    let mut v: i32;
    let mut x: i32 = 0;

    macro_rules! STACK_PUSH {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            ssize += 1;
        }};
    }
    macro_rules! STACK_POP {
        () => {{
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize as usize].a;
            last = stack[ssize as usize].b;
            depth = stack[ssize as usize].c;
            limit = stack[ssize as usize].d;
        }};
    }

    ssize = 0;
    limit = ss_ilg(pdiff(last, first));
    loop {
        if pdiff(last, first) <= SS_INSERTIONSORT_THRESHOLD {
            /* 1 < SS_INSERTIONSORT_THRESHOLD */
            if 1 < pdiff(last, first) {
                ss_insertionsort(T, PA, first, last, depth);
            }
            STACK_POP!();
            continue;
        }

        Td = T.offset(depth as isize);
        {
            let old = limit;
            limit -= 1;
            if old == 0 {
                ss_heapsort(Td, PA, first, pdiff(last, first));
            }
        }
        if limit < 0 {
            a = first.offset(1);
            v = ss_key(Td, PA, first);
            while a < last {
                x = ss_key(Td, PA, a);
                if x != v {
                    if 1 < pdiff(a, first) {
                        break;
                    }
                    v = x;
                    first = a;
                }
                a = a.offset(1);
            }
            if (*Td.offset((*PA.offset(*first as isize)).wrapping_sub(1) as isize) as i32) < v {
                first = ss_partition(PA, first, a, depth);
            }
            if pdiff(a, first) <= pdiff(last, a) {
                if 1 < pdiff(a, first) {
                    STACK_PUSH!(a, last, depth, -1);
                    last = a;
                    depth += 1;
                    limit = ss_ilg(pdiff(a, first));
                } else {
                    first = a;
                    limit = -1;
                }
            } else {
                if 1 < pdiff(last, a) {
                    STACK_PUSH!(first, a, depth + 1, ss_ilg(pdiff(a, first)));
                    first = a;
                    limit = -1;
                } else {
                    last = a;
                    depth += 1;
                    limit = ss_ilg(pdiff(a, first));
                }
            }
            continue;
        }

        /* choose pivot */
        a = ss_pivot(Td, PA, first, last);
        v = ss_key(Td, PA, a);
        t = *first;
        *first = *a;
        *a = t;

        /* partition */
        b = first;
        loop {
            b = b.offset(1);
            if !(b < last) {
                break;
            }
            x = ss_key(Td, PA, b);
            if !(x == v) {
                break;
            }
        }
        a = b;
        if (a < last) && (x < v) {
            loop {
                b = b.offset(1);
                if !(b < last) {
                    break;
                }
                x = ss_key(Td, PA, b);
                if !(x <= v) {
                    break;
                }
                if x == v {
                    t = *b;
                    *b = *a;
                    *a = t;
                    a = a.offset(1);
                }
            }
        }
        c = last;
        loop {
            c = c.offset(-1);
            if !(b < c) {
                break;
            }
            x = ss_key(Td, PA, c);
            if !(x == v) {
                break;
            }
        }
        d = c;
        if (b < d) && (x > v) {
            loop {
                c = c.offset(-1);
                if !(b < c) {
                    break;
                }
                x = ss_key(Td, PA, c);
                if !(x >= v) {
                    break;
                }
                if x == v {
                    t = *c;
                    *c = *d;
                    *d = t;
                    d = d.offset(-1);
                }
            }
        }
        while b < c {
            t = *b;
            *b = *c;
            *c = t;
            loop {
                b = b.offset(1);
                if !(b < c) {
                    break;
                }
                x = ss_key(Td, PA, b);
                if !(x <= v) {
                    break;
                }
                if x == v {
                    t = *b;
                    *b = *a;
                    *a = t;
                    a = a.offset(1);
                }
            }
            loop {
                c = c.offset(-1);
                if !(b < c) {
                    break;
                }
                x = ss_key(Td, PA, c);
                if !(x >= v) {
                    break;
                }
                if x == v {
                    t = *c;
                    *c = *d;
                    *d = t;
                    d = d.offset(-1);
                }
            }
        }

        if a <= d {
            c = b.offset(-1);

            s = pdiff(a, first);
            t = pdiff(b, a);
            if s > t {
                s = t;
            }
            e = first;
            f = b.offset(-(s as isize));
            while 0 < s {
                t = *e;
                *e = *f;
                *f = t;
                s -= 1;
                e = e.offset(1);
                f = f.offset(1);
            }
            s = pdiff(d, c);
            t = pdiff(last, d) - 1;
            if s > t {
                s = t;
            }
            e = b;
            f = last.offset(-(s as isize));
            while 0 < s {
                t = *e;
                *e = *f;
                *f = t;
                s -= 1;
                e = e.offset(1);
                f = f.offset(1);
            }

            a = first.offset(pdiff(b, a) as isize);
            c = last.offset(-(pdiff(d, c) as isize));
            b = if v <= (*Td.offset((*PA.offset(*a as isize)).wrapping_sub(1) as isize) as i32) {
                a
            } else {
                ss_partition(PA, a, c, depth)
            };

            if pdiff(a, first) <= pdiff(last, c) {
                if pdiff(last, c) <= pdiff(c, b) {
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pdiff(c, b)));
                    STACK_PUSH!(c, last, depth, limit);
                    last = a;
                } else if pdiff(a, first) <= pdiff(c, b) {
                    STACK_PUSH!(c, last, depth, limit);
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pdiff(c, b)));
                    last = a;
                } else {
                    STACK_PUSH!(c, last, depth, limit);
                    STACK_PUSH!(first, a, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(pdiff(c, b));
                }
            } else {
                if pdiff(a, first) <= pdiff(c, b) {
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pdiff(c, b)));
                    STACK_PUSH!(first, a, depth, limit);
                    first = c;
                } else if pdiff(last, c) <= pdiff(c, b) {
                    STACK_PUSH!(first, a, depth, limit);
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(pdiff(c, b)));
                    first = c;
                } else {
                    STACK_PUSH!(first, a, depth, limit);
                    STACK_PUSH!(c, last, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(pdiff(c, b));
                }
            }
        } else {
            limit += 1;
            if (*Td.offset((*PA.offset(*first as isize)).wrapping_sub(1) as isize) as i32) < v {
                first = ss_partition(PA, first, last, depth);
                limit = ss_ilg(pdiff(last, first));
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
        a = a.offset(1);
        b = b.offset(1);
    }
}

#[inline]
unsafe fn ss_rotate(mut first: *mut i32, middle: *mut i32, mut last: *mut i32) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;
    let mut l: i32;
    let mut r: i32;

    l = pdiff(middle, first);
    r = pdiff(last, middle);
    while (0 < l) && (0 < r) {
        if l == r {
            ss_blockswap(first, middle, l);
            break;
        }
        if l < r {
            a = last.offset(-1);
            b = middle.offset(-1);
            t = *a;
            loop {
                *a = *b;
                a = a.offset(-1);
                *b = *a;
                b = b.offset(-1);
                if b < first {
                    *a = t;
                    last = a;
                    r -= l + 1;
                    if r <= l {
                        break;
                    }
                    a = a.offset(-1);
                    b = middle.offset(-1);
                    t = *a;
                }
            }
        } else {
            a = first;
            b = middle;
            t = *a;
            loop {
                *a = *b;
                a = a.offset(1);
                *b = *a;
                b = b.offset(1);
                if last <= b {
                    *a = t;
                    first = a.offset(1);
                    l -= r + 1;
                    if l <= r {
                        break;
                    }
                    a = a.offset(1);
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
        if *last.offset(-1) < 0 {
            x = 1;
            p = PA.offset(!*last.offset(-1) as isize);
        } else {
            x = 0;
            p = PA.offset(*last.offset(-1) as isize);
        }
        a = first;
        len = pdiff(middle, first);
        half = len >> 1;
        r = -1;
        while 0 < len {
            b = a.offset(half as isize);
            q = ss_compare(
                T,
                PA.offset(if 0 <= *b { *b } else { !*b } as isize),
                p,
                depth,
            );
            if q < 0 {
                a = b.offset(1);
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
            last = last.offset(-(pdiff(middle, a) as isize));
            middle = a;
            if first == middle {
                break;
            }
        }
        last = last.offset(-1);
        if x != 0 {
            loop {
                last = last.offset(-1);
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

    bufend = buf.offset((pdiff(middle, first) - 1) as isize);
    ss_blockswap(buf, first, pdiff(middle, first));

    a = first;
    t = *a;
    b = buf;
    c = middle;
    loop {
        r = ss_compare(T, PA.offset(*b as isize), PA.offset(*c as isize), depth);
        if r < 0 {
            loop {
                *a = *b;
                a = a.offset(1);
                if bufend <= b {
                    *bufend = t;
                    return;
                }
                *b = *a;
                b = b.offset(1);
                if !(*b < 0) {
                    break;
                }
            }
        } else if r > 0 {
            loop {
                *a = *c;
                a = a.offset(1);
                *c = *a;
                c = c.offset(1);
                if last <= c {
                    while b < bufend {
                        *a = *b;
                        a = a.offset(1);
                        *b = *a;
                        b = b.offset(1);
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
                a = a.offset(1);
                if bufend <= b {
                    *bufend = t;
                    return;
                }
                *b = *a;
                b = b.offset(1);
                if !(*b < 0) {
                    break;
                }
            }

            loop {
                *a = *c;
                a = a.offset(1);
                *c = *a;
                c = c.offset(1);
                if last <= c {
                    while b < bufend {
                        *a = *b;
                        a = a.offset(1);
                        *b = *a;
                        b = b.offset(1);
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

    bufend = buf.offset((pdiff(last, middle) - 1) as isize);
    ss_blockswap(buf, middle, pdiff(last, middle));

    x = 0;
    if *bufend < 0 {
        p1 = PA.offset(!*bufend as isize);
        x |= 1;
    } else {
        p1 = PA.offset(*bufend as isize);
    }
    if *middle.offset(-1) < 0 {
        p2 = PA.offset(!*middle.offset(-1) as isize);
        x |= 2;
    } else {
        p2 = PA.offset(*middle.offset(-1) as isize);
    }
    a = last.offset(-1);
    t = *a;
    b = bufend;
    c = middle.offset(-1);
    loop {
        r = ss_compare(T, p1, p2, depth);
        if 0 < r {
            if (x & 1) != 0 {
                loop {
                    *a = *b;
                    a = a.offset(-1);
                    *b = *a;
                    b = b.offset(-1);
                    if !(*b < 0) {
                        break;
                    }
                }
                x ^= 1;
            }
            *a = *b;
            a = a.offset(-1);
            if b <= buf {
                *buf = t;
                break;
            }
            *b = *a;
            b = b.offset(-1);
            if *b < 0 {
                p1 = PA.offset(!*b as isize);
                x |= 1;
            } else {
                p1 = PA.offset(*b as isize);
            }
        } else if r < 0 {
            if (x & 2) != 0 {
                loop {
                    *a = *c;
                    a = a.offset(-1);
                    *c = *a;
                    c = c.offset(-1);
                    if !(*c < 0) {
                        break;
                    }
                }
                x ^= 2;
            }
            *a = *c;
            a = a.offset(-1);
            *c = *a;
            c = c.offset(-1);
            if c < first {
                while buf < b {
                    *a = *b;
                    a = a.offset(-1);
                    *b = *a;
                    b = b.offset(-1);
                }
                *a = *b;
                *b = t;
                break;
            }
            if *c < 0 {
                p2 = PA.offset(!*c as isize);
                x |= 2;
            } else {
                p2 = PA.offset(*c as isize);
            }
        } else {
            if (x & 1) != 0 {
                loop {
                    *a = *b;
                    a = a.offset(-1);
                    *b = *a;
                    b = b.offset(-1);
                    if !(*b < 0) {
                        break;
                    }
                }
                x ^= 1;
            }
            *a = !*b;
            a = a.offset(-1);
            if b <= buf {
                *buf = t;
                break;
            }
            *b = *a;
            b = b.offset(-1);
            if (x & 2) != 0 {
                loop {
                    *a = *c;
                    a = a.offset(-1);
                    *c = *a;
                    c = c.offset(-1);
                    if !(*c < 0) {
                        break;
                    }
                }
                x ^= 2;
            }
            *a = *c;
            a = a.offset(-1);
            *c = *a;
            c = c.offset(-1);
            if c < first {
                while buf < b {
                    *a = *b;
                    a = a.offset(-1);
                    *b = *a;
                    b = b.offset(-1);
                }
                *a = *b;
                *b = t;
                break;
            }
            if *b < 0 {
                p1 = PA.offset(!*b as isize);
                x |= 1;
            } else {
                p1 = PA.offset(*b as isize);
            }
            if *c < 0 {
                p2 = PA.offset(!*c as isize);
                x |= 2;
            } else {
                p2 = PA.offset(*c as isize);
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
    let mut stack: [SsSmergeStack; STACK_SIZE] = [SsSmergeStack {
        a: null_mut(),
        b: null_mut(),
        c: null_mut(),
        d: 0,
    }; STACK_SIZE];
    let mut l: *mut i32;
    let mut r: *mut i32;
    let mut lm: *mut i32;
    let mut rm: *mut i32;
    let mut m: i32;
    let mut len: i32;
    let mut half: i32;
    let mut ssize: i32;
    let mut check: i32;
    let mut next: i32;

    macro_rules! STACK_PUSH {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            ssize += 1;
        }};
    }
    macro_rules! STACK_POP {
        () => {{
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize as usize].a;
            middle = stack[ssize as usize].b;
            last = stack[ssize as usize].c;
            check = stack[ssize as usize].d;
        }};
    }
    /* MERGE_CHECK(first, last, check) */
    macro_rules! MERGE_CHECK {
        ($a:expr, $b:expr, $c:expr) => {{
            if (($c) & 1) != 0
                || ((($c) & 2) != 0
                    && (ss_compare(
                        T,
                        PA.offset(getidx(*($a).offset(-1)) as isize),
                        PA.offset(*($a) as isize),
                        depth,
                    ) == 0))
            {
                *($a) = !*($a);
            }
            if ((($c) & 4) != 0)
                && (ss_compare(
                    T,
                    PA.offset(getidx(*($b).offset(-1)) as isize),
                    PA.offset(*($b) as isize),
                    depth,
                ) == 0)
            {
                *($b) = !*($b);
            }
        }};
    }

    check = 0;
    ssize = 0;
    loop {
        if pdiff(last, middle) <= bufsize {
            if (first < middle) && (middle < last) {
                ss_mergebackward(T, PA, first, middle, last, buf, depth);
            }
            MERGE_CHECK!(first, last, check);
            STACK_POP!();
            continue;
        }

        if pdiff(middle, first) <= bufsize {
            if first < middle {
                ss_mergeforward(T, PA, first, middle, last, buf, depth);
            }
            MERGE_CHECK!(first, last, check);
            STACK_POP!();
            continue;
        }

        m = 0;
        len = {
            let _a = pdiff(middle, first);
            let _b = pdiff(last, middle);
            if _a < _b {
                _a
            } else {
                _b
            }
        };
        half = len >> 1;
        while 0 < len {
            if ss_compare(
                T,
                PA.offset(getidx(*middle.offset((m + half) as isize)) as isize),
                PA.offset(getidx(*middle.offset((-m - half - 1) as isize)) as isize),
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
            lm = middle.offset(-(m as isize));
            rm = middle.offset(m as isize);
            ss_blockswap(lm, middle, m);
            l = middle;
            r = middle;
            next = 0;
            if rm < last {
                if *rm < 0 {
                    *rm = !*rm;
                    if first < lm {
                        loop {
                            l = l.offset(-1);
                            if !(*l < 0) {
                                break;
                            }
                        }
                        next |= 4;
                    }
                    next |= 1;
                } else if first < lm {
                    while *r < 0 {
                        r = r.offset(1);
                    }
                    next |= 2;
                }
            }

            if pdiff(l, first) <= pdiff(last, r) {
                STACK_PUSH!(r, rm, last, (next & 3) | (check & 4));
                middle = lm;
                last = l;
                check = (check & 3) | (next & 4);
            } else {
                if ((next & 2) != 0) && (r == middle) {
                    next ^= 6;
                }
                STACK_PUSH!(first, lm, l, (check & 3) | (next & 4));
                first = r;
                middle = rm;
                check = (next & 3) | (check & 4);
            }
        } else {
            if ss_compare(
                T,
                PA.offset(getidx(*middle.offset(-1)) as isize),
                PA.offset(*middle as isize),
                depth,
            ) == 0
            {
                *middle = !*middle;
            }
            MERGE_CHECK!(first, last, check);
            STACK_POP!();
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
    let middle: *mut i32;
    let mut curbuf: *mut i32;
    let mut j: i32;
    let mut k: i32;
    let mut curbufsize: i32;
    let mut limit: i32 = 0;
    let mut i: i32;

    if lastsuffix != 0 {
        first = first.offset(1);
    }

    if (bufsize < SS_BLOCKSIZE)
        && (bufsize < pdiff(last, first))
        && (bufsize < {
            limit = ss_isqrt(pdiff(last, first));
            limit
        })
    {
        if SS_BLOCKSIZE < limit {
            limit = SS_BLOCKSIZE;
        }
        middle = last.offset(-(limit as isize));
        buf = middle;
        bufsize = limit;
    } else {
        middle = last;
        limit = 0;
    }
    a = first;
    i = 0;
    while SS_BLOCKSIZE < pdiff(middle, a) {
        /* SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE */
        ss_mintrosort(T, PA, a, a.offset(SS_BLOCKSIZE as isize), depth);
        curbufsize = pdiff(last, a.offset(SS_BLOCKSIZE as isize));
        curbuf = a.offset(SS_BLOCKSIZE as isize);
        if curbufsize <= bufsize {
            curbufsize = bufsize;
            curbuf = buf;
        }
        b = a;
        k = SS_BLOCKSIZE;
        j = i;
        while (j & 1) != 0 {
            ss_swapmerge(
                T,
                PA,
                b.offset(-(k as isize)),
                b,
                b.offset(k as isize),
                curbuf,
                curbufsize,
                depth,
            );
            b = b.offset(-(k as isize));
            k <<= 1;
            j >>= 1;
        }
        a = a.offset(SS_BLOCKSIZE as isize);
        i += 1;
    }
    /* SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE */
    ss_mintrosort(T, PA, a, middle, depth);
    k = SS_BLOCKSIZE;
    while i != 0 {
        if (i & 1) != 0 {
            ss_swapmerge(
                T,
                PA,
                a.offset(-(k as isize)),
                a,
                middle,
                buf,
                bufsize,
                depth,
            );
            a = a.offset(-(k as isize));
        }
        k <<= 1;
        i >>= 1;
    }
    if limit != 0 {
        /* SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE */
        ss_mintrosort(T, PA, middle, last, depth);
        ss_inplacemerge(T, PA, first, middle, last, depth);
    }

    if lastsuffix != 0 {
        /* Insert last type B* suffix. */
        let mut PAi: [i32; 2] = [0; 2];
        PAi[0] = *PA.offset(*first.offset(-1) as isize);
        PAi[1] = n - 2;
        a = first;
        i = *first.offset(-1);
        while (a < last)
            && ((*a < 0)
                || (0 < ss_compare(
                    T,
                    &PAi[0] as *const i32,
                    PA.offset(*a as isize),
                    depth,
                )))
        {
            *a.offset(-1) = *a;
            a = a.offset(1);
        }
        *a.offset(-1) = i;
    }
}

/*---------------------------------------------------------------------------*/

#[inline]
unsafe fn tr_ilg(n: i32) -> i32 {
    if (n & 0xffff0000u32 as i32) != 0 {
        if (n & 0xff000000u32 as i32) != 0 {
            24 + lg_table[((n >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[((n >> 16) & 0xff) as usize]
        }
    } else {
        if (n & 0x0000ff00) != 0 {
            8 + lg_table[((n >> 8) & 0xff) as usize]
        } else {
            0 + lg_table[((n >> 0) & 0xff) as usize]
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Simple insertionsort for small size groups. */
unsafe fn tr_insertionsort(ISAd: *const i32, first: *mut i32, last: *mut i32) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;
    let mut r: i32;

    a = first.offset(1);
    while a < last {
        t = *a;
        b = a.offset(-1);
        r = (*ISAd.offset(t as isize)).wrapping_sub(*ISAd.offset(*b as isize));
        while 0 > r {
            loop {
                *b.offset(1) = *b;
                b = b.offset(-1);
                if !((first <= b) && (*b < 0)) {
                    break;
                }
            }
            if b < first {
                break;
            }
            r = (*ISAd.offset(t as isize)).wrapping_sub(*ISAd.offset(*b as isize));
        }
        if r == 0 {
            *b = !*b;
        }
        *b.offset(1) = t;
        a = a.offset(1);
    }
}

/*---------------------------------------------------------------------------*/

#[inline]
unsafe fn tr_fixdown(ISAd: *const i32, SA: *mut i32, mut i: i32, size: i32) {
    let mut j: i32;
    let mut k: i32;
    let v: i32;
    let c: i32;
    let mut d: i32;
    let mut e: i32;

    v = *SA.offset(i as isize);
    c = *ISAd.offset(v as isize);
    loop {
        j = (2i32).wrapping_mul(i).wrapping_add(1);
        if !(j < size) {
            break;
        }
        k = j;
        j += 1;
        d = *ISAd.offset(*SA.offset(k as isize) as isize);
        e = *ISAd.offset(*SA.offset(j as isize) as isize);
        if d < e {
            k = j;
            d = e;
        }
        if d <= c {
            break;
        }
        *SA.offset(i as isize) = *SA.offset(k as isize);
        i = k;
    }
    *SA.offset(i as isize) = v;
}

/* Simple top-down heapsort. */
unsafe fn tr_heapsort(ISAd: *const i32, SA: *mut i32, size: i32) {
    let mut i: i32;
    let mut m: i32;
    let mut t: i32;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if *ISAd.offset(*SA.offset((m / 2) as isize) as isize)
            < *ISAd.offset(*SA.offset(m as isize) as isize)
        {
            t = *SA.offset(m as isize);
            *SA.offset(m as isize) = *SA.offset((m / 2) as isize);
            *SA.offset((m / 2) as isize) = t;
        }
    }

    i = m / 2 - 1;
    while 0 <= i {
        tr_fixdown(ISAd, SA, i, m);
        i -= 1;
    }
    if (size % 2) == 0 {
        t = *SA.offset(0);
        *SA.offset(0) = *SA.offset(m as isize);
        *SA.offset(m as isize) = t;
        tr_fixdown(ISAd, SA, 0, m);
    }
    i = m - 1;
    while 0 < i {
        t = *SA.offset(0);
        *SA.offset(0) = *SA.offset(i as isize);
        tr_fixdown(ISAd, SA, 0, i);
        *SA.offset(i as isize) = t;
        i -= 1;
    }
}

/*---------------------------------------------------------------------------*/

/* Returns the median of three elements. */
#[inline]
unsafe fn tr_median3(
    ISAd: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    v3: *mut i32,
) -> *mut i32 {
    let t: *mut i32;
    if *ISAd.offset(*v1 as isize) > *ISAd.offset(*v2 as isize) {
        t = v1;
        v1 = v2;
        v2 = t;
    }
    if *ISAd.offset(*v2 as isize) > *ISAd.offset(*v3 as isize) {
        if *ISAd.offset(*v1 as isize) > *ISAd.offset(*v3 as isize) {
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
    if *ISAd.offset(*v2 as isize) > *ISAd.offset(*v3 as isize) {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if *ISAd.offset(*v4 as isize) > *ISAd.offset(*v5 as isize) {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if *ISAd.offset(*v2 as isize) > *ISAd.offset(*v4 as isize) {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if *ISAd.offset(*v1 as isize) > *ISAd.offset(*v3 as isize) {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if *ISAd.offset(*v1 as isize) > *ISAd.offset(*v4 as isize) {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if *ISAd.offset(*v3 as isize) > *ISAd.offset(*v4 as isize) {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
#[inline]
unsafe fn tr_pivot(ISAd: *const i32, mut first: *mut i32, mut last: *mut i32) -> *mut i32 {
    let mut middle: *mut i32;
    let mut t: i32;

    t = pdiff(last, first);
    middle = first.offset((t / 2) as isize);

    if t <= 512 {
        if t <= 32 {
            return tr_median3(ISAd, first, middle, last.offset(-1));
        } else {
            t >>= 2;
            return tr_median5(
                ISAd,
                first,
                first.offset(t as isize),
                middle,
                last.offset(-1 - (t as isize)),
                last.offset(-1),
            );
        }
    }
    t >>= 3;
    first = tr_median3(
        ISAd,
        first,
        first.offset(t as isize),
        first.offset((t << 1) as isize),
    );
    middle = tr_median3(
        ISAd,
        middle.offset(-(t as isize)),
        middle,
        middle.offset(t as isize),
    );
    last = tr_median3(
        ISAd,
        last.offset(-1 - ((t << 1) as isize)),
        last.offset(-1 - (t as isize)),
        last.offset(-1),
    );
    tr_median3(ISAd, first, middle, last)
}

/*---------------------------------------------------------------------------*/

#[repr(C)]
struct trbudget_t {
    chance: i32,
    remain: i32,
    incval: i32,
    count: i32,
}

#[inline]
unsafe fn trbudget_init(budget: *mut trbudget_t, chance: i32, incval: i32) {
    (*budget).chance = chance;
    (*budget).incval = incval;
    (*budget).remain = (*budget).incval;
}

#[inline]
unsafe fn trbudget_check(budget: *mut trbudget_t, size: i32) -> i32 {
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

    b = middle.offset(-1);
    loop {
        b = b.offset(1);
        if !(b < last) {
            break;
        }
        x = *ISAd.offset(*b as isize);
        if !(x == v) {
            break;
        }
    }
    a = b;
    if (a < last) && (x < v) {
        loop {
            b = b.offset(1);
            if !(b < last) {
                break;
            }
            x = *ISAd.offset(*b as isize);
            if !(x <= v) {
                break;
            }
            if x == v {
                t = *b;
                *b = *a;
                *a = t;
                a = a.offset(1);
            }
        }
    }
    c = last;
    loop {
        c = c.offset(-1);
        if !(b < c) {
            break;
        }
        x = *ISAd.offset(*c as isize);
        if !(x == v) {
            break;
        }
    }
    d = c;
    if (b < d) && (x > v) {
        loop {
            c = c.offset(-1);
            if !(b < c) {
                break;
            }
            x = *ISAd.offset(*c as isize);
            if !(x >= v) {
                break;
            }
            if x == v {
                t = *c;
                *c = *d;
                *d = t;
                d = d.offset(-1);
            }
        }
    }
    while b < c {
        t = *b;
        *b = *c;
        *c = t;
        loop {
            b = b.offset(1);
            if !(b < c) {
                break;
            }
            x = *ISAd.offset(*b as isize);
            if !(x <= v) {
                break;
            }
            if x == v {
                t = *b;
                *b = *a;
                *a = t;
                a = a.offset(1);
            }
        }
        loop {
            c = c.offset(-1);
            if !(b < c) {
                break;
            }
            x = *ISAd.offset(*c as isize);
            if !(x >= v) {
                break;
            }
            if x == v {
                t = *c;
                *c = *d;
                *d = t;
                d = d.offset(-1);
            }
        }
    }

    if a <= d {
        c = b.offset(-1);
        s = pdiff(a, first);
        t = pdiff(b, a);
        if s > t {
            s = t;
        }
        e = first;
        f = b.offset(-(s as isize));
        while 0 < s {
            t = *e;
            *e = *f;
            *f = t;
            s -= 1;
            e = e.offset(1);
            f = f.offset(1);
        }
        s = pdiff(d, c);
        t = pdiff(last, d) - 1;
        if s > t {
            s = t;
        }
        e = b;
        f = last.offset(-(s as isize));
        while 0 < s {
            t = *e;
            *e = *f;
            *f = t;
            s -= 1;
            e = e.offset(1);
            f = f.offset(1);
        }
        first = first.offset(pdiff(b, a) as isize);
        last = last.offset(-(pdiff(d, c) as isize));
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
    /* sort suffixes of middle partition
    by using sorted order of suffixes of left and right partition. */
    let mut c: *mut i32;
    let mut d: *mut i32;
    let mut e: *mut i32;
    let mut s: i32;
    let v: i32;

    v = pdiff(b, SA) - 1;
    c = first;
    d = a.offset(-1);
    while c <= d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(1);
            *d = s;
            *ISA.offset(s as isize) = pdiff(d, SA);
        }
        c = c.offset(1);
    }
    c = last.offset(-1);
    e = d.offset(1);
    d = b;
    while e < d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(-1);
            *d = s;
            *ISA.offset(s as isize) = pdiff(d, SA);
        }
        c = c.offset(-1);
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

    v = pdiff(b, SA) - 1;
    lastrank = -1;
    c = first;
    d = a.offset(-1);
    while c <= d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(1);
            *d = s;
            rank = *ISA.offset((s.wrapping_add(depth)) as isize);
            if lastrank != rank {
                lastrank = rank;
                newrank = pdiff(d, SA);
            }
            *ISA.offset(s as isize) = newrank;
        }
        c = c.offset(1);
    }

    lastrank = -1;
    e = d;
    while first <= e {
        rank = *ISA.offset(*e as isize);
        if lastrank != rank {
            lastrank = rank;
            newrank = pdiff(e, SA);
        }
        if newrank != rank {
            *ISA.offset(*e as isize) = newrank;
        }
        e = e.offset(-1);
    }

    lastrank = -1;
    c = last.offset(-1);
    e = d.offset(1);
    d = b;
    while e < d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(-1);
            *d = s;
            rank = *ISA.offset((s.wrapping_add(depth)) as isize);
            if lastrank != rank {
                lastrank = rank;
                newrank = pdiff(d, SA);
            }
            *ISA.offset(s as isize) = newrank;
        }
        c = c.offset(-1);
    }
}

unsafe fn tr_introsort(
    ISA: *mut i32,
    ISAd_arg: *const i32,
    SA: *mut i32,
    first_arg: *mut i32,
    last_arg: *mut i32,
    budget: *mut trbudget_t,
) {
    const STACK_SIZE: usize = TR_STACKSIZE;
    let mut stack: [TrStack; STACK_SIZE] = [TrStack {
        a: null(),
        b: null_mut(),
        c: null_mut(),
        d: 0,
        e: 0,
    }; STACK_SIZE];
    let mut ISAd: *const i32 = ISAd_arg;
    let mut first: *mut i32 = first_arg;
    let mut last: *mut i32 = last_arg;
    let mut a: *mut i32 = null_mut();
    let mut b: *mut i32 = null_mut();
    let mut c: *mut i32;
    let mut t: i32;
    let mut v: i32;
    let mut x: i32 = 0;
    let incr: i32 = pdiff(ISAd, ISA);
    let mut limit: i32;
    let mut next: i32;
    let mut ssize: i32;
    let mut trlink: i32 = -1;

    macro_rules! STACK_PUSH5 {
        ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {{
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            stack[ssize as usize].e = $e;
            ssize += 1;
        }};
    }
    macro_rules! STACK_POP5 {
        () => {{
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            ISAd = stack[ssize as usize].a;
            first = stack[ssize as usize].b;
            last = stack[ssize as usize].c;
            limit = stack[ssize as usize].d;
            trlink = stack[ssize as usize].e;
        }};
    }

    ssize = 0;
    limit = tr_ilg(pdiff(last, first));
    loop {
        if limit < 0 {
            if limit == -1 {
                /* tandem repeat partition */
                tr_partition(
                    ISAd.offset(-(incr as isize)),
                    first,
                    first,
                    last,
                    &mut a,
                    &mut b,
                    pdiff(last, SA) - 1,
                );

                /* update ranks */
                if a < last {
                    c = first;
                    v = pdiff(a, SA) - 1;
                    while c < a {
                        *ISA.offset(*c as isize) = v;
                        c = c.offset(1);
                    }
                }
                if b < last {
                    c = a;
                    v = pdiff(b, SA) - 1;
                    while c < b {
                        *ISA.offset(*c as isize) = v;
                        c = c.offset(1);
                    }
                }

                /* push */
                if 1 < pdiff(b, a) {
                    STACK_PUSH5!(null(), a, b, 0, 0);
                    STACK_PUSH5!(ISAd.offset(-(incr as isize)), first, last, -2, trlink);
                    trlink = ssize - 2;
                }
                if pdiff(a, first) <= pdiff(last, b) {
                    if 1 < pdiff(a, first) {
                        STACK_PUSH5!(ISAd, b, last, tr_ilg(pdiff(last, b)), trlink);
                        last = a;
                        limit = tr_ilg(pdiff(a, first));
                    } else if 1 < pdiff(last, b) {
                        first = b;
                        limit = tr_ilg(pdiff(last, b));
                    } else {
                        STACK_POP5!();
                    }
                } else {
                    if 1 < pdiff(last, b) {
                        STACK_PUSH5!(ISAd, first, a, tr_ilg(pdiff(a, first)), trlink);
                        first = b;
                        limit = tr_ilg(pdiff(last, b));
                    } else if 1 < pdiff(a, first) {
                        last = a;
                        limit = tr_ilg(pdiff(a, first));
                    } else {
                        STACK_POP5!();
                    }
                }
            } else if limit == -2 {
                /* tandem repeat copy */
                ssize -= 1;
                a = stack[ssize as usize].b;
                b = stack[ssize as usize].c;
                if stack[ssize as usize].d == 0 {
                    tr_copy(ISA, SA, first, a, b, last, pdiff(ISAd, ISA));
                } else {
                    if 0 <= trlink {
                        stack[trlink as usize].d = -1;
                    }
                    tr_partialcopy(ISA, SA, first, a, b, last, pdiff(ISAd, ISA));
                }
                STACK_POP5!();
            } else {
                /* sorted partition */
                if 0 <= *first {
                    a = first;
                    loop {
                        *ISA.offset(*a as isize) = pdiff(a, SA);
                        a = a.offset(1);
                        if !(a < last) {
                            break;
                        }
                        if !(0 <= *a) {
                            break;
                        }
                    }
                    first = a;
                }
                if first < last {
                    a = first;
                    loop {
                        *a = !*a;
                        a = a.offset(1);
                        if !(*a < 0) {
                            break;
                        }
                    }
                    next = if *ISA.offset(*a as isize) != *ISAd.offset(*a as isize) {
                        tr_ilg(pdiff(a, first) + 1)
                    } else {
                        -1
                    };
                    a = a.offset(1);
                    if a < last {
                        b = first;
                        v = pdiff(a, SA) - 1;
                        while b < a {
                            *ISA.offset(*b as isize) = v;
                            b = b.offset(1);
                        }
                    }

                    /* push */
                    if trbudget_check(budget, pdiff(a, first)) != 0 {
                        if pdiff(a, first) <= pdiff(last, a) {
                            STACK_PUSH5!(ISAd, a, last, -3, trlink);
                            ISAd = ISAd.offset(incr as isize);
                            last = a;
                            limit = next;
                        } else {
                            if 1 < pdiff(last, a) {
                                STACK_PUSH5!(ISAd.offset(incr as isize), first, a, next, trlink);
                                first = a;
                                limit = -3;
                            } else {
                                ISAd = ISAd.offset(incr as isize);
                                last = a;
                                limit = next;
                            }
                        }
                    } else {
                        if 0 <= trlink {
                            stack[trlink as usize].d = -1;
                        }
                        if 1 < pdiff(last, a) {
                            first = a;
                            limit = -3;
                        } else {
                            STACK_POP5!();
                        }
                    }
                } else {
                    STACK_POP5!();
                }
            }
            continue;
        }

        if pdiff(last, first) <= TR_INSERTIONSORT_THRESHOLD {
            tr_insertionsort(ISAd, first, last);
            limit = -3;
            continue;
        }

        {
            let old = limit;
            limit -= 1;
            if old == 0 {
                tr_heapsort(ISAd, first, pdiff(last, first));
                a = last.offset(-1);
                while first < a {
                    x = *ISAd.offset(*a as isize);
                    b = a.offset(-1);
                    while (first <= b) && (*ISAd.offset(*b as isize) == x) {
                        *b = !*b;
                        b = b.offset(-1);
                    }
                    a = b;
                }
                limit = -3;
                continue;
            }
        }

        /* choose pivot */
        a = tr_pivot(ISAd, first, last);
        t = *first;
        *first = *a;
        *a = t;
        v = *ISAd.offset(*first as isize);

        /* partition */
        tr_partition(ISAd, first, first.offset(1), last, &mut a, &mut b, v);
        if pdiff(last, first) != pdiff(b, a) {
            next = if *ISA.offset(*a as isize) != v {
                tr_ilg(pdiff(b, a))
            } else {
                -1
            };

            /* update ranks */
            c = first;
            v = pdiff(a, SA) - 1;
            while c < a {
                *ISA.offset(*c as isize) = v;
                c = c.offset(1);
            }
            if b < last {
                c = a;
                v = pdiff(b, SA) - 1;
                while c < b {
                    *ISA.offset(*c as isize) = v;
                    c = c.offset(1);
                }
            }

            /* push */
            if (1 < pdiff(b, a)) && (trbudget_check(budget, pdiff(b, a)) != 0) {
                if pdiff(a, first) <= pdiff(last, b) {
                    if pdiff(last, b) <= pdiff(b, a) {
                        if 1 < pdiff(a, first) {
                            STACK_PUSH5!(ISAd.offset(incr as isize), a, b, next, trlink);
                            STACK_PUSH5!(ISAd, b, last, limit, trlink);
                            last = a;
                        } else if 1 < pdiff(last, b) {
                            STACK_PUSH5!(ISAd.offset(incr as isize), a, b, next, trlink);
                            first = b;
                        } else {
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if pdiff(a, first) <= pdiff(b, a) {
                        if 1 < pdiff(a, first) {
                            STACK_PUSH5!(ISAd, b, last, limit, trlink);
                            STACK_PUSH5!(ISAd.offset(incr as isize), a, b, next, trlink);
                            last = a;
                        } else {
                            STACK_PUSH5!(ISAd, b, last, limit, trlink);
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        STACK_PUSH5!(ISAd, b, last, limit, trlink);
                        STACK_PUSH5!(ISAd, first, a, limit, trlink);
                        ISAd = ISAd.offset(incr as isize);
                        first = a;
                        last = b;
                        limit = next;
                    }
                } else {
                    if pdiff(a, first) <= pdiff(b, a) {
                        if 1 < pdiff(last, b) {
                            STACK_PUSH5!(ISAd.offset(incr as isize), a, b, next, trlink);
                            STACK_PUSH5!(ISAd, first, a, limit, trlink);
                            first = b;
                        } else if 1 < pdiff(a, first) {
                            STACK_PUSH5!(ISAd.offset(incr as isize), a, b, next, trlink);
                            last = a;
                        } else {
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if pdiff(last, b) <= pdiff(b, a) {
                        if 1 < pdiff(last, b) {
                            STACK_PUSH5!(ISAd, first, a, limit, trlink);
                            STACK_PUSH5!(ISAd.offset(incr as isize), a, b, next, trlink);
                            first = b;
                        } else {
                            STACK_PUSH5!(ISAd, first, a, limit, trlink);
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        STACK_PUSH5!(ISAd, first, a, limit, trlink);
                        STACK_PUSH5!(ISAd, b, last, limit, trlink);
                        ISAd = ISAd.offset(incr as isize);
                        first = a;
                        last = b;
                        limit = next;
                    }
                }
            } else {
                if (1 < pdiff(b, a)) && (0 <= trlink) {
                    stack[trlink as usize].d = -1;
                }
                if pdiff(a, first) <= pdiff(last, b) {
                    if 1 < pdiff(a, first) {
                        STACK_PUSH5!(ISAd, b, last, limit, trlink);
                        last = a;
                    } else if 1 < pdiff(last, b) {
                        first = b;
                    } else {
                        STACK_POP5!();
                    }
                } else {
                    if 1 < pdiff(last, b) {
                        STACK_PUSH5!(ISAd, first, a, limit, trlink);
                        first = b;
                    } else if 1 < pdiff(a, first) {
                        last = a;
                    } else {
                        STACK_POP5!();
                    }
                }
            }
        } else {
            if trbudget_check(budget, pdiff(last, first)) != 0 {
                limit = tr_ilg(pdiff(last, first));
                ISAd = ISAd.offset(incr as isize);
            } else {
                if 0 <= trlink {
                    stack[trlink as usize].d = -1;
                }
                STACK_POP5!();
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
    /*  trbudget_init(&budget, tr_ilg(n) * 3 / 4, n); */
    ISAd = ISA.offset(depth as isize);
    while -n < *SA {
        first = SA;
        skip = 0;
        unsorted = 0;
        loop {
            t = *first;
            if t < 0 {
                first = first.offset(-(t as isize));
                skip += t;
            } else {
                if skip != 0 {
                    *first.offset(skip as isize) = skip;
                    skip = 0;
                }
                last = SA.offset((*ISA.offset(t as isize) + 1) as isize);
                if 1 < pdiff(last, first) {
                    budget.count = 0;
                    tr_introsort(ISA, ISAd, SA, first, last, &mut budget);
                    if budget.count != 0 {
                        unsorted += budget.count;
                    } else {
                        skip = pdiff(first, last);
                    }
                } else if pdiff(last, first) == 1 {
                    skip = -1;
                }
                first = last;
            }
            if !(first < SA.offset(n as isize)) {
                break;
            }
        }
        if skip != 0 {
            *first.offset(skip as isize) = skip;
        }
        if unsorted == 0 {
            break;
        }
        ISAd = ISAd.offset(pdiff(ISAd, ISA) as isize);
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
    openMP: i32,
) -> i32 {
    let PAb: *mut i32;
    let ISAb: *mut i32;
    let buf: *mut i32;
    let mut i: i32;
    let mut j: i32;
    let mut k: i32;
    let mut t: i32;
    let mut m: i32;
    let bufsize: i32;
    let mut c0: i32;
    let mut c1: i32;
    let _ = openMP;

    /* Initialize bucket arrays. */
    i = 0;
    while i < BUCKET_A_SIZE {
        *bucket_A.offset(i as isize) = 0;
        i += 1;
    }
    i = 0;
    while i < BUCKET_B_SIZE {
        *bucket_B.offset(i as isize) = 0;
        i += 1;
    }

    /* Count the number of occurrences of the first one or two characters of each
    type A, B and B* suffix. Moreover, store the beginning position of all
    type B* suffixes into the array SA. */
    i = n - 1;
    m = n;
    c0 = *T.offset((n - 1) as isize) as i32;
    c1 = 0;
    while 0 <= i {
        /* type A suffix. */
        loop {
            c1 = c0;
            *bucket_A.offset(c1 as isize) += 1;
            i -= 1;
            if !(0 <= i) {
                break;
            }
            c0 = *T.offset(i as isize) as i32;
            if !(c0 >= c1) {
                break;
            }
        }
        if 0 <= i {
            /* type B* suffix. */
            *bucket_B.offset(((c0 << 8) | c1) as isize) += 1;
            m -= 1;
            *SA.offset(m as isize) = i;
            /* type B suffix. */
            i -= 1;
            c1 = c0;
            while (0 <= i) && {
                c0 = *T.offset(i as isize) as i32;
                c0 <= c1
            } {
                *bucket_B.offset(((c1 << 8) | c0) as isize) += 1;
                i -= 1;
                c1 = c0;
            }
        }
    }
    m = n - m;
    /*
    note:
      A type B* suffix is lexicographically smaller than a type B suffix that
      begins with the same first two characters.
    */

    /* Calculate the index of start/end point of each bucket. */
    c0 = 0;
    i = 0;
    j = 0;
    while c0 < ALPHABET_SIZE {
        t = i + *bucket_A.offset(c0 as isize);
        *bucket_A.offset(c0 as isize) = i + j; /* start point */
        i = t + *bucket_B.offset(((c0 << 8) | c0) as isize);
        c1 = c0 + 1;
        while c1 < ALPHABET_SIZE {
            j += *bucket_B.offset(((c0 << 8) | c1) as isize);
            *bucket_B.offset(((c0 << 8) | c1) as isize) = j; /* end point */
            i += *bucket_B.offset(((c1 << 8) | c0) as isize);
            c1 += 1;
        }
        c0 += 1;
    }

    if 0 < m {
        /* Sort the type B* suffixes by their first two characters. */
        PAb = SA.offset((n - m) as isize);
        ISAb = SA.offset(m as isize);
        i = m - 2;
        while 0 <= i {
            t = *PAb.offset(i as isize);
            c0 = *T.offset(t as isize) as i32;
            c1 = *T.offset((t + 1) as isize) as i32;
            {
                let bp = bucket_B.offset(((c0 << 8) | c1) as isize);
                *bp -= 1;
                *SA.offset(*bp as isize) = i;
            }
            i -= 1;
        }
        t = *PAb.offset((m - 1) as isize);
        c0 = *T.offset(t as isize) as i32;
        c1 = *T.offset((t + 1) as isize) as i32;
        {
            let bp = bucket_B.offset(((c0 << 8) | c1) as isize);
            *bp -= 1;
            *SA.offset(*bp as isize) = m - 1;
        }

        /* Sort the type B* substrings using sssort. */
        buf = SA.offset(m as isize);
        bufsize = n - (2 * m);
        c0 = ALPHABET_SIZE - 2;
        j = m;
        while 0 < j {
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                i = *bucket_B.offset(((c0 << 8) | c1) as isize);
                if 1 < (j - i) {
                    sssort(
                        T,
                        PAb,
                        SA.offset(i as isize),
                        SA.offset(j as isize),
                        buf,
                        bufsize,
                        2,
                        n,
                        (*SA.offset(i as isize) == (m - 1)) as i32,
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
            if 0 <= *SA.offset(i as isize) {
                j = i;
                loop {
                    *ISAb.offset(*SA.offset(i as isize) as isize) = i;
                    i -= 1;
                    if !(0 <= i) {
                        break;
                    }
                    if !(0 <= *SA.offset(i as isize)) {
                        break;
                    }
                }
                *SA.offset((i + 1) as isize) = i - j;
                if i <= 0 {
                    break;
                }
            }
            j = i;
            loop {
                let v = !*SA.offset(i as isize);
                *SA.offset(i as isize) = v;
                *ISAb.offset(v as isize) = j;
                i -= 1;
                if !(*SA.offset(i as isize) < 0) {
                    break;
                }
            }
            *ISAb.offset(*SA.offset(i as isize) as isize) = j;
            i -= 1;
        }

        /* Construct the inverse suffix array of type B* suffixes using trsort. */
        trsort(ISAb, SA, m, 1);

        /* Set the sorted order of type B* suffixes. */
        i = n - 1;
        j = m;
        c0 = *T.offset((n - 1) as isize) as i32;
        while 0 <= i {
            i -= 1;
            c1 = c0;
            while (0 <= i) && {
                c0 = *T.offset(i as isize) as i32;
                c0 >= c1
            } {
                i -= 1;
                c1 = c0;
            }
            if 0 <= i {
                t = i;
                i -= 1;
                c1 = c0;
                while (0 <= i) && {
                    c0 = *T.offset(i as isize) as i32;
                    c0 <= c1
                } {
                    i -= 1;
                    c1 = c0;
                }
                j -= 1;
                *SA.offset(*ISAb.offset(j as isize) as isize) =
                    if (t == 0) || (1 < (t - i)) { t } else { !t };
            }
        }

        /* Calculate the index of start/end point of each bucket. */
        *bucket_B.offset((((ALPHABET_SIZE - 1) << 8) | (ALPHABET_SIZE - 1)) as isize) = n; /* end point */
        c0 = ALPHABET_SIZE - 2;
        k = m - 1;
        while 0 <= c0 {
            i = *bucket_A.offset((c0 + 1) as isize) - 1;
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                t = i - *bucket_B.offset(((c1 << 8) | c0) as isize);
                *bucket_B.offset(((c1 << 8) | c0) as isize) = i; /* end point */

                /* Move all type B* suffixes to the correct position. */
                i = t;
                j = *bucket_B.offset(((c0 << 8) | c1) as isize);
                while j <= k {
                    *SA.offset(i as isize) = *SA.offset(k as isize);
                    i -= 1;
                    k -= 1;
                }
                c1 -= 1;
            }
            *bucket_B.offset(((c0 << 8) | (c0 + 1)) as isize) =
                i - *bucket_B.offset(((c0 << 8) | c0) as isize) + 1; /* start point */
            *bucket_B.offset(((c0 << 8) | c0) as isize) = i; /* end point */
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
    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut k: *mut i32;
    let mut s: i32;
    let mut c0: i32;
    let mut c1: i32;
    let mut c2: i32 = 0;

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.offset(*bucket_B.offset(((c1 << 8) | (c1 + 1)) as isize) as isize);
            j = SA.offset((*bucket_A.offset((c1 + 1) as isize) - 1) as isize);
            k = null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    *j = !s;
                    s -= 1;
                    c0 = *T.offset(s as isize) as i32;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as i32) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *bucket_B.offset(((c1 << 8) | c2) as isize) = pdiff(k, SA);
                        }
                        c2 = c0;
                        k = SA.offset(*bucket_B.offset(((c1 << 8) | c2) as isize) as isize);
                    }
                    *k = s;
                    k = k.offset(-1);
                } else {
                    *j = !s;
                }
                j = j.offset(-1);
            }
            c1 -= 1;
        }
    }

    /* Construct the suffix array by using
    the sorted order of type B suffixes. */
    c2 = *T.offset((n - 1) as isize) as i32;
    k = SA.offset(*bucket_A.offset(c2 as isize) as isize);
    *k = if (*T.offset((n - 2) as isize) as i32) < c2 {
        !(n - 1)
    } else {
        n - 1
    };
    k = k.offset(1);
    /* Scan the suffix array from left to right. */
    i = SA;
    j = SA.offset(n as isize);
    while i < j {
        s = *i;
        if 0 < s {
            s -= 1;
            c0 = *T.offset(s as isize) as i32;
            if (s == 0) || ((*T.offset((s - 1) as isize) as i32) < c0) {
                s = !s;
            }
            if c0 != c2 {
                *bucket_A.offset(c2 as isize) = pdiff(k, SA);
                c2 = c0;
                k = SA.offset(*bucket_A.offset(c2 as isize) as isize);
            }
            *k = s;
            k = k.offset(1);
        } else {
            *i = !s;
        }
        i = i.offset(1);
    }
}

/* Constructs the burrows-wheeler transformed string directly
by using the sorted order of type B* suffixes. */
unsafe fn construct_BWT(
    T: *const u8,
    SA: *mut i32,
    bucket_A: *mut i32,
    bucket_B: *mut i32,
    n: i32,
    m: i32,
) -> i32 {
    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut k: *mut i32;
    let mut orig: *mut i32;
    let mut s: i32;
    let mut c0: i32;
    let mut c1: i32;
    let mut c2: i32 = 0;

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.offset(*bucket_B.offset(((c1 << 8) | (c1 + 1)) as isize) as isize);
            j = SA.offset((*bucket_A.offset((c1 + 1) as isize) - 1) as isize);
            k = null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    s -= 1;
                    c0 = *T.offset(s as isize) as i32;
                    *j = !c0;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as i32) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *bucket_B.offset(((c1 << 8) | c2) as isize) = pdiff(k, SA);
                        }
                        c2 = c0;
                        k = SA.offset(*bucket_B.offset(((c1 << 8) | c2) as isize) as isize);
                    }
                    *k = s;
                    k = k.offset(-1);
                } else if s != 0 {
                    *j = !s;
                }
                j = j.offset(-1);
            }
            c1 -= 1;
        }
    }

    /* Construct the BWTed string by using
    the sorted order of type B suffixes. */
    c2 = *T.offset((n - 1) as isize) as i32;
    k = SA.offset(*bucket_A.offset(c2 as isize) as isize);
    *k = if (*T.offset((n - 2) as isize) as i32) < c2 {
        !(*T.offset((n - 2) as isize) as i32)
    } else {
        n - 1
    };
    k = k.offset(1);
    /* Scan the suffix array from left to right. */
    i = SA;
    j = SA.offset(n as isize);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            s -= 1;
            c0 = *T.offset(s as isize) as i32;
            *i = c0;
            if (0 < s) && ((*T.offset((s - 1) as isize) as i32) < c0) {
                s = !(*T.offset((s - 1) as isize) as i32);
            }
            if c0 != c2 {
                *bucket_A.offset(c2 as isize) = pdiff(k, SA);
                c2 = c0;
                k = SA.offset(*bucket_A.offset(c2 as isize) as isize);
            }
            *k = s;
            k = k.offset(1);
        } else if s != 0 {
            *i = !s;
        } else {
            orig = i;
        }
        i = i.offset(1);
    }

    pdiff(orig, SA)
}

/* Constructs the burrows-wheeler transformed string directly
by using the sorted order of type B* suffixes. */
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
    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut k: *mut i32;
    let mut orig: *mut i32;
    let mut s: i32;
    let mut c0: i32;
    let mut c1: i32;
    let mut c2: i32 = 0;

    let mut r#mod: i32 = n / 8;
    {
        r#mod |= r#mod >> 1;
        r#mod |= r#mod >> 2;
        r#mod |= r#mod >> 4;
        r#mod |= r#mod >> 8;
        r#mod |= r#mod >> 16;
        r#mod >>= 1;

        *num_indexes = ((n - 1) / (r#mod + 1)) as u8;
    }

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.offset(*bucket_B.offset(((c1 << 8) | (c1 + 1)) as isize) as isize);
            j = SA.offset((*bucket_A.offset((c1 + 1) as isize) - 1) as isize);
            k = null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    if (s & r#mod) == 0 {
                        *indexes.offset((s / (r#mod + 1) - 1) as isize) = pdiff(j, SA);
                    }

                    s -= 1;
                    c0 = *T.offset(s as isize) as i32;
                    *j = !c0;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as i32) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *bucket_B.offset(((c1 << 8) | c2) as isize) = pdiff(k, SA);
                        }
                        c2 = c0;
                        k = SA.offset(*bucket_B.offset(((c1 << 8) | c2) as isize) as isize);
                    }
                    *k = s;
                    k = k.offset(-1);
                } else if s != 0 {
                    *j = !s;
                }
                j = j.offset(-1);
            }
            c1 -= 1;
        }
    }

    /* Construct the BWTed string by using
    the sorted order of type B suffixes. */
    c2 = *T.offset((n - 1) as isize) as i32;
    k = SA.offset(*bucket_A.offset(c2 as isize) as isize);
    if (*T.offset((n - 2) as isize) as i32) < c2 {
        if ((n - 1) & r#mod) == 0 {
            *indexes.offset(((n - 1) / (r#mod + 1) - 1) as isize) = pdiff(k, SA);
        }
        *k = !(*T.offset((n - 2) as isize) as i32);
        k = k.offset(1);
    } else {
        *k = n - 1;
        k = k.offset(1);
    }

    /* Scan the suffix array from left to right. */
    i = SA;
    j = SA.offset(n as isize);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            if (s & r#mod) == 0 {
                *indexes.offset((s / (r#mod + 1) - 1) as isize) = pdiff(i, SA);
            }

            s -= 1;
            c0 = *T.offset(s as isize) as i32;
            *i = c0;
            if c0 != c2 {
                *bucket_A.offset(c2 as isize) = pdiff(k, SA);
                c2 = c0;
                k = SA.offset(*bucket_A.offset(c2 as isize) as isize);
            }
            if (0 < s) && ((*T.offset((s - 1) as isize) as i32) < c0) {
                if (s & r#mod) == 0 {
                    *indexes.offset((s / (r#mod + 1) - 1) as isize) = pdiff(k, SA);
                }
                *k = !(*T.offset((s - 1) as isize) as i32);
                k = k.offset(1);
            } else {
                *k = s;
                k = k.offset(1);
            }
        } else if s != 0 {
            *i = !s;
        } else {
            orig = i;
        }
        i = i.offset(1);
    }

    pdiff(orig, SA)
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
        *SA.offset(0) = 0;
        return 0;
    } else if n == 2 {
        let m = (*T.offset(0) < *T.offset(1)) as i32;
        *SA.offset((m ^ 1) as isize) = 0;
        *SA.offset(m as isize) = 1;
        return 0;
    }

    bucket_A = malloc(BUCKET_A_SIZE as usize * core::mem::size_of::<i32>()) as *mut i32;
    bucket_B = malloc(BUCKET_B_SIZE as usize * core::mem::size_of::<i32>()) as *mut i32;

    /* Suffixsort. */
    if !bucket_A.is_null() && !bucket_B.is_null() {
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
            *U.offset(0) = *T.offset(0);
        }
        return n;
    }

    B = A;
    if B.is_null() {
        B = malloc((n + 1) as usize * core::mem::size_of::<i32>()) as *mut i32;
    }
    bucket_A = malloc(BUCKET_A_SIZE as usize * core::mem::size_of::<i32>()) as *mut i32;
    bucket_B = malloc(BUCKET_B_SIZE as usize * core::mem::size_of::<i32>()) as *mut i32;

    /* Burrows-Wheeler Transform. */
    if !B.is_null() && !bucket_A.is_null() && !bucket_B.is_null() {
        m = sort_typeBstar(T, B, bucket_A, bucket_B, n, openMP);

        if num_indexes.is_null() || indexes.is_null() {
            pidx = construct_BWT(T, B, bucket_A, bucket_B, n, m);
        } else {
            pidx = construct_BWT_indexes(T, B, bucket_A, bucket_B, n, m, num_indexes, indexes);
        }

        /* Copy to output string. */
        *U.offset(0) = *T.offset((n - 1) as isize);
        i = 0;
        while i < pidx {
            *U.offset((i + 1) as isize) = *B.offset(i as isize) as u8;
            i += 1;
        }
        i += 1;
        while i < n {
            *U.offset(i as isize) = *B.offset(i as isize) as u8;
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

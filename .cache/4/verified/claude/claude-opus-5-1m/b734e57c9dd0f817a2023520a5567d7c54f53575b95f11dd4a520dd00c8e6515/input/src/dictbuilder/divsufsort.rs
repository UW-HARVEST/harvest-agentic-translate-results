//! Translation of `dictBuilder/divsufsort.c` (libdivsufsort-lite).
//!
//! Self-contained suffix-array construction library.  `_OPENMP` /
//! `LIBBSC_OPENMP` are not defined for this port, so the `openMP` parameters
//! are accepted and ignored exactly like the C does.
#![allow(dead_code)]

use crate::libc::{free, malloc};
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;

pub type saint_t = i32;
pub type saidx_t = i32;
pub type sauchar_t = u8;

/*- Constants -*/
const ALPHABET_SIZE: saidx_t = 256;
const BUCKET_A_SIZE: saidx_t = ALPHABET_SIZE;
const BUCKET_B_SIZE: saidx_t = ALPHABET_SIZE * ALPHABET_SIZE;
const SS_INSERTIONSORT_THRESHOLD: saidx_t = 8;
const SS_BLOCKSIZE: saidx_t = 1024;
/* minstacksize = log(SS_BLOCKSIZE) / log(3) * 2 ; SS_BLOCKSIZE <= 4096 */
const SS_MISORT_STACKSIZE: usize = 16;
const SS_SMERGE_STACKSIZE: usize = 32;
const TR_INSERTIONSORT_THRESHOLD: saidx_t = 8;
const TR_STACKSIZE: usize = 64;

/*- Helpers -*/

/// C `(int)(a - b)` for `saidx_t *` pointers.
#[inline(always)]
unsafe fn ptr_diff(a: *const saidx_t, b: *const saidx_t) -> saidx_t {
    a.offset_from(b) as saidx_t
}

#[inline(always)]
fn MIN(a: saidx_t, b: saidx_t) -> saidx_t {
    if a < b {
        a
    } else {
        b
    }
}

/* ALIGNED_STACK / STACK_PUSH / STACK_POP emulation.
   The `stack` array and `ssize` index are passed in explicitly because
   macro_rules! hygiene prevents referring to the caller's locals. */

macro_rules! STACK_PUSH {
    ($stack:expr, $ssize:expr, $a:expr, $b:expr, $c:expr, $d:expr) => {{
        let __i = $ssize as usize;
        $stack[__i].a = $a;
        $stack[__i].b = $b;
        $stack[__i].c = $c;
        $stack[__i].d = $d;
        $ssize += 1;
    }};
}

macro_rules! STACK_POP {
    ($stack:expr, $ssize:expr, $a:expr, $b:expr, $c:expr, $d:expr) => {{
        if $ssize == 0 {
            return;
        }
        $ssize -= 1;
        let __i = $ssize as usize;
        $a = $stack[__i].a;
        $b = $stack[__i].b;
        $c = $stack[__i].c;
        $d = $stack[__i].d;
    }};
}

macro_rules! STACK_PUSH5 {
    ($stack:expr, $ssize:expr, $a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {{
        let __i = $ssize as usize;
        $stack[__i].a = $a;
        $stack[__i].b = $b;
        $stack[__i].c = $c;
        $stack[__i].d = $d;
        $stack[__i].e = $e;
        $ssize += 1;
    }};
}

macro_rules! STACK_POP5 {
    ($stack:expr, $ssize:expr, $a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {{
        if $ssize == 0 {
            return;
        }
        $ssize -= 1;
        let __i = $ssize as usize;
        $a = $stack[__i].a;
        $b = $stack[__i].b;
        $c = $stack[__i].c;
        $d = $stack[__i].d;
        $e = $stack[__i].e;
    }};
}

/* BUCKET_A(_c0) / BUCKET_B(_c0,_c1) / BUCKET_BSTAR(_c0,_c1) (ALPHABET_SIZE == 256) */

#[inline(always)]
unsafe fn BUCKET_A(bucket_A: *mut saidx_t, _c0: saidx_t) -> *mut saidx_t {
    bucket_A.offset(_c0 as isize)
}

#[inline(always)]
unsafe fn BUCKET_B(bucket_B: *mut saidx_t, _c0: saidx_t, _c1: saidx_t) -> *mut saidx_t {
    bucket_B.offset(((_c1 << 8) | _c0) as isize)
}

#[inline(always)]
unsafe fn BUCKET_BSTAR(bucket_B: *mut saidx_t, _c0: saidx_t, _c1: saidx_t) -> *mut saidx_t {
    bucket_B.offset(((_c0 << 8) | _c1) as isize)
}

/*- Private Functions -*/

static lg_table: [saint_t; 256] = [
    -1, 0, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

/* (SS_BLOCKSIZE == 0) || (SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE) */
#[inline]
fn ss_ilg(n: saidx_t) -> saint_t {
    /* SS_BLOCKSIZE == 1024 -> 256 <= SS_BLOCKSIZE */
    if (n & 0xff00) != 0 {
        8 + lg_table[((n >> 8) & 0xff) as usize]
    } else {
        0 + lg_table[((n >> 0) & 0xff) as usize]
    }
}

/* SS_BLOCKSIZE != 0 */
static sqq_table: [saint_t; 256] = [
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
fn ss_isqrt(x: saidx_t) -> saidx_t {
    let mut y: saidx_t;
    let e: saint_t;

    if x >= (SS_BLOCKSIZE * SS_BLOCKSIZE) {
        return SS_BLOCKSIZE;
    }
    e = if (x as u32 & 0xffff0000) != 0 {
        if (x as u32 & 0xff000000) != 0 {
            24 + lg_table[((x >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[((x >> 16) & 0xff) as usize]
        }
    } else {
        if (x as u32 & 0x0000ff00) != 0 {
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

    if x < (y * y) {
        y - 1
    } else {
        y
    }
}

/*---------------------------------------------------------------------------*/

/* Compares two suffixes. */
#[inline]
unsafe fn ss_compare(
    T: *const sauchar_t,
    p1: *const saidx_t,
    p2: *const saidx_t,
    depth: saidx_t,
) -> saint_t {
    let mut U1: *const sauchar_t;
    let mut U2: *const sauchar_t;
    let U1n: *const sauchar_t;
    let U2n: *const sauchar_t;

    U1 = T.offset(depth as isize).offset(*p1 as isize);
    U2 = T.offset(depth as isize).offset(*p2 as isize);
    U1n = T.offset(*p1.offset(1) as isize).offset(2);
    U2n = T.offset(*p2.offset(1) as isize).offset(2);
    while (U1 < U1n) && (U2 < U2n) && (*U1 == *U2) {
        U1 = U1.offset(1);
        U2 = U2.offset(1);
    }

    if U1 < U1n {
        if U2 < U2n {
            *U1 as saint_t - *U2 as saint_t
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
    T: *const sauchar_t,
    PA: *const saidx_t,
    first: *mut saidx_t,
    last: *mut saidx_t,
    depth: saidx_t,
) {
    let mut i: *mut saidx_t;
    let mut j: *mut saidx_t;
    let mut t: saidx_t;
    let mut r: saint_t = 0;

    i = last.offset(-2);
    while first <= i {
        t = *i;
        j = i.offset(1);
        loop {
            r = ss_compare(T, PA.offset(t as isize), PA.offset(*j as isize), depth);
            if !(0 < r) {
                break;
            }
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
unsafe fn ss_fixdown(
    Td: *const sauchar_t,
    PA: *const saidx_t,
    SA: *mut saidx_t,
    mut i: saidx_t,
    size: saidx_t,
) {
    let mut j: saidx_t;
    let mut k: saidx_t = 0;
    let v: saidx_t;
    let c: saint_t;
    let mut d: saint_t;
    let mut e: saint_t;

    v = *SA.offset(i as isize);
    c = *Td.offset(*PA.offset(v as isize) as isize) as saint_t;
    loop {
        j = 2 * i + 1;
        if !(j < size) {
            break;
        }
        k = j;
        j += 1;
        d = *Td.offset(*PA.offset(*SA.offset(k as isize) as isize) as isize) as saint_t;
        e = *Td.offset(*PA.offset(*SA.offset(j as isize) as isize) as isize) as saint_t;
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
unsafe fn ss_heapsort(Td: *const sauchar_t, PA: *const saidx_t, SA: *mut saidx_t, size: saidx_t) {
    let mut i: saidx_t;
    let mut m: saidx_t;
    let mut t: saidx_t;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if (*Td.offset(*PA.offset(*SA.offset((m / 2) as isize) as isize) as isize) as saint_t)
            < (*Td.offset(*PA.offset(*SA.offset(m as isize) as isize) as isize) as saint_t)
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
    Td: *const sauchar_t,
    PA: *const saidx_t,
    mut v1: *mut saidx_t,
    mut v2: *mut saidx_t,
    v3: *mut saidx_t,
) -> *mut saidx_t {
    let t: *mut saidx_t;
    if *Td.offset(*PA.offset(*v1 as isize) as isize)
        > *Td.offset(*PA.offset(*v2 as isize) as isize)
    {
        t = v1;
        v1 = v2;
        v2 = t;
    }
    if *Td.offset(*PA.offset(*v2 as isize) as isize)
        > *Td.offset(*PA.offset(*v3 as isize) as isize)
    {
        if *Td.offset(*PA.offset(*v1 as isize) as isize)
            > *Td.offset(*PA.offset(*v3 as isize) as isize)
        {
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
    Td: *const sauchar_t,
    PA: *const saidx_t,
    mut v1: *mut saidx_t,
    mut v2: *mut saidx_t,
    mut v3: *mut saidx_t,
    mut v4: *mut saidx_t,
    mut v5: *mut saidx_t,
) -> *mut saidx_t {
    let mut t: *mut saidx_t;
    if *Td.offset(*PA.offset(*v2 as isize) as isize)
        > *Td.offset(*PA.offset(*v3 as isize) as isize)
    {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if *Td.offset(*PA.offset(*v4 as isize) as isize)
        > *Td.offset(*PA.offset(*v5 as isize) as isize)
    {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if *Td.offset(*PA.offset(*v2 as isize) as isize)
        > *Td.offset(*PA.offset(*v4 as isize) as isize)
    {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if *Td.offset(*PA.offset(*v1 as isize) as isize)
        > *Td.offset(*PA.offset(*v3 as isize) as isize)
    {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if *Td.offset(*PA.offset(*v1 as isize) as isize)
        > *Td.offset(*PA.offset(*v4 as isize) as isize)
    {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if *Td.offset(*PA.offset(*v3 as isize) as isize)
        > *Td.offset(*PA.offset(*v4 as isize) as isize)
    {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
#[inline]
unsafe fn ss_pivot(
    Td: *const sauchar_t,
    PA: *const saidx_t,
    mut first: *mut saidx_t,
    mut last: *mut saidx_t,
) -> *mut saidx_t {
    let mut middle: *mut saidx_t;
    let mut t: saidx_t;

    t = ptr_diff(last, first);
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
                last.offset(-1).offset(-(t as isize)),
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
        last.offset(-1).offset(-((t << 1) as isize)),
        last.offset(-1).offset(-(t as isize)),
        last.offset(-1),
    );
    ss_median3(Td, PA, first, middle, last)
}

/*---------------------------------------------------------------------------*/

/* Binary partition for substrings. */
#[inline]
unsafe fn ss_partition(
    PA: *const saidx_t,
    first: *mut saidx_t,
    last: *mut saidx_t,
    depth: saidx_t,
) -> *mut saidx_t {
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut t: saidx_t;

    a = first.offset(-1);
    b = last;
    loop {
        loop {
            a = a.offset(1);
            if !(a < b) {
                break;
            }
            if !((*PA.offset(*a as isize) + depth) >= (*PA.offset((*a + 1) as isize) + 1)) {
                break;
            }
            *a = !*a;
        }
        loop {
            b = b.offset(-1);
            if !(a < b) {
                break;
            }
            if !((*PA.offset(*b as isize) + depth) < (*PA.offset((*b + 1) as isize) + 1)) {
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

/*---------------------------------------------------------------------------*/

#[derive(Copy, Clone)]
struct ss_mintrosort_stackitem {
    a: *mut saidx_t,
    b: *mut saidx_t,
    c: saidx_t,
    d: saint_t,
}

/* Multikey introsort for medium size groups. */
unsafe fn ss_mintrosort(
    T: *const sauchar_t,
    PA: *const saidx_t,
    mut first: *mut saidx_t,
    mut last: *mut saidx_t,
    mut depth: saidx_t,
) {
    /* STACK_SIZE == SS_MISORT_STACKSIZE */
    let mut stack: [ss_mintrosort_stackitem; SS_MISORT_STACKSIZE] = [ss_mintrosort_stackitem {
        a: ptr::null_mut(),
        b: ptr::null_mut(),
        c: 0,
        d: 0,
    }; SS_MISORT_STACKSIZE];
    let mut Td: *const sauchar_t;
    let mut a: *mut saidx_t = ptr::null_mut();
    let mut b: *mut saidx_t;
    let mut c: *mut saidx_t;
    let mut d: *mut saidx_t = ptr::null_mut();
    let mut e: *mut saidx_t;
    let mut f: *mut saidx_t;
    let mut s: saidx_t;
    let mut t: saidx_t;
    let mut ssize: saint_t;
    let mut limit: saint_t;
    let mut v: saint_t;
    let mut x: saint_t = 0;

    ssize = 0;
    limit = ss_ilg(ptr_diff(last, first));
    loop {
        if ptr_diff(last, first) <= SS_INSERTIONSORT_THRESHOLD {
            /* 1 < SS_INSERTIONSORT_THRESHOLD */
            if 1 < ptr_diff(last, first) {
                ss_insertionsort(T, PA, first, last, depth);
            }
            STACK_POP!(stack, ssize, first, last, depth, limit);
            continue;
        }

        Td = T.offset(depth as isize);
        {
            let old_limit = limit;
            limit -= 1;
            if old_limit == 0 {
                ss_heapsort(Td, PA, first, ptr_diff(last, first));
            }
        }
        if limit < 0 {
            a = first.offset(1);
            v = *Td.offset(*PA.offset(*first as isize) as isize) as saint_t;
            while a < last {
                x = *Td.offset(*PA.offset(*a as isize) as isize) as saint_t;
                if x != v {
                    if 1 < ptr_diff(a, first) {
                        break;
                    }
                    v = x;
                    first = a;
                }
                a = a.offset(1);
            }
            if (*Td.offset((*PA.offset(*first as isize) - 1) as isize) as saint_t) < v {
                first = ss_partition(PA, first, a, depth);
            }
            if ptr_diff(a, first) <= ptr_diff(last, a) {
                if 1 < ptr_diff(a, first) {
                    STACK_PUSH!(stack, ssize, a, last, depth, -1);
                    last = a;
                    depth += 1;
                    limit = ss_ilg(ptr_diff(a, first));
                } else {
                    first = a;
                    limit = -1;
                }
            } else {
                if 1 < ptr_diff(last, a) {
                    STACK_PUSH!(
                        stack,
                        ssize,
                        first,
                        a,
                        depth + 1,
                        ss_ilg(ptr_diff(a, first))
                    );
                    first = a;
                    limit = -1;
                } else {
                    last = a;
                    depth += 1;
                    limit = ss_ilg(ptr_diff(a, first));
                }
            }
            continue;
        }

        /* choose pivot */
        a = ss_pivot(Td, PA, first, last);
        v = *Td.offset(*PA.offset(*a as isize) as isize) as saint_t;
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
            x = *Td.offset(*PA.offset(*b as isize) as isize) as saint_t;
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
                x = *Td.offset(*PA.offset(*b as isize) as isize) as saint_t;
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
            x = *Td.offset(*PA.offset(*c as isize) as isize) as saint_t;
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
                x = *Td.offset(*PA.offset(*c as isize) as isize) as saint_t;
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
                x = *Td.offset(*PA.offset(*b as isize) as isize) as saint_t;
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
                x = *Td.offset(*PA.offset(*c as isize) as isize) as saint_t;
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

            s = ptr_diff(a, first);
            t = ptr_diff(b, a);
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
            s = ptr_diff(d, c);
            t = ptr_diff(last, d) - 1;
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

            a = first.offset(ptr_diff(b, a) as isize);
            c = last.offset(-(ptr_diff(d, c) as isize));
            b = if v <= (*Td.offset((*PA.offset(*a as isize) - 1) as isize) as saint_t) {
                a
            } else {
                ss_partition(PA, a, c, depth)
            };

            if ptr_diff(a, first) <= ptr_diff(last, c) {
                if ptr_diff(last, c) <= ptr_diff(c, b) {
                    STACK_PUSH!(stack, ssize, b, c, depth + 1, ss_ilg(ptr_diff(c, b)));
                    STACK_PUSH!(stack, ssize, c, last, depth, limit);
                    last = a;
                } else if ptr_diff(a, first) <= ptr_diff(c, b) {
                    STACK_PUSH!(stack, ssize, c, last, depth, limit);
                    STACK_PUSH!(stack, ssize, b, c, depth + 1, ss_ilg(ptr_diff(c, b)));
                    last = a;
                } else {
                    STACK_PUSH!(stack, ssize, c, last, depth, limit);
                    STACK_PUSH!(stack, ssize, first, a, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(ptr_diff(c, b));
                }
            } else {
                if ptr_diff(a, first) <= ptr_diff(c, b) {
                    STACK_PUSH!(stack, ssize, b, c, depth + 1, ss_ilg(ptr_diff(c, b)));
                    STACK_PUSH!(stack, ssize, first, a, depth, limit);
                    first = c;
                } else if ptr_diff(last, c) <= ptr_diff(c, b) {
                    STACK_PUSH!(stack, ssize, first, a, depth, limit);
                    STACK_PUSH!(stack, ssize, b, c, depth + 1, ss_ilg(ptr_diff(c, b)));
                    first = c;
                } else {
                    STACK_PUSH!(stack, ssize, first, a, depth, limit);
                    STACK_PUSH!(stack, ssize, c, last, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(ptr_diff(c, b));
                }
            }
        } else {
            limit += 1;
            if (*Td.offset((*PA.offset(*first as isize) - 1) as isize) as saint_t) < v {
                first = ss_partition(PA, first, last, depth);
                limit = ss_ilg(ptr_diff(last, first));
            }
            depth += 1;
        }
    }
}

/*---------------------------------------------------------------------------*/

/* SS_BLOCKSIZE != 0 */

#[inline]
unsafe fn ss_blockswap(mut a: *mut saidx_t, mut b: *mut saidx_t, mut n: saidx_t) {
    let mut t: saidx_t;
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
unsafe fn ss_rotate(mut first: *mut saidx_t, middle: *mut saidx_t, mut last: *mut saidx_t) {
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut t: saidx_t;
    let mut l: saidx_t;
    let mut r: saidx_t;

    l = ptr_diff(middle, first);
    r = ptr_diff(last, middle);
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
    T: *const sauchar_t,
    PA: *const saidx_t,
    first: *mut saidx_t,
    mut middle: *mut saidx_t,
    mut last: *mut saidx_t,
    depth: saidx_t,
) {
    let mut p: *const saidx_t;
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut len: saidx_t;
    let mut half: saidx_t;
    let mut q: saint_t;
    let mut r: saint_t;
    let mut x: saint_t;

    loop {
        if *last.offset(-1) < 0 {
            x = 1;
            p = PA.offset(!*last.offset(-1) as isize);
        } else {
            x = 0;
            p = PA.offset(*last.offset(-1) as isize);
        }
        a = first;
        len = ptr_diff(middle, first);
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
            last = last.offset(-(ptr_diff(middle, a) as isize));
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
    T: *const sauchar_t,
    PA: *const saidx_t,
    first: *mut saidx_t,
    middle: *mut saidx_t,
    last: *mut saidx_t,
    buf: *mut saidx_t,
    depth: saidx_t,
) {
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut c: *mut saidx_t;
    let bufend: *mut saidx_t;
    let t: saidx_t;
    let mut r: saint_t;

    bufend = buf.offset(ptr_diff(middle, first) as isize).offset(-1);
    ss_blockswap(buf, first, ptr_diff(middle, first));

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
    T: *const sauchar_t,
    PA: *const saidx_t,
    first: *mut saidx_t,
    middle: *mut saidx_t,
    last: *mut saidx_t,
    buf: *mut saidx_t,
    depth: saidx_t,
) {
    let mut p1: *const saidx_t;
    let mut p2: *const saidx_t;
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut c: *mut saidx_t;
    let bufend: *mut saidx_t;
    let t: saidx_t;
    let mut r: saint_t;
    let mut x: saint_t;

    bufend = buf.offset(ptr_diff(last, middle) as isize).offset(-1);
    ss_blockswap(buf, middle, ptr_diff(last, middle));

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

/* GETIDX(a) */
#[inline(always)]
fn ss_GETIDX(a: saidx_t) -> saidx_t {
    if 0 <= a {
        a
    } else {
        !a
    }
}

/* MERGE_CHECK(a, b, c) */
#[inline]
unsafe fn ss_MERGE_CHECK(
    T: *const sauchar_t,
    PA: *const saidx_t,
    depth: saidx_t,
    a: *mut saidx_t,
    b: *mut saidx_t,
    c: saint_t,
) {
    if ((c & 1) != 0)
        || (((c & 2) != 0)
            && (ss_compare(
                T,
                PA.offset(ss_GETIDX(*a.offset(-1)) as isize),
                PA.offset(*a as isize),
                depth,
            ) == 0))
    {
        *a = !*a;
    }
    if ((c & 4) != 0)
        && (ss_compare(
            T,
            PA.offset(ss_GETIDX(*b.offset(-1)) as isize),
            PA.offset(*b as isize),
            depth,
        ) == 0)
    {
        *b = !*b;
    }
}

#[derive(Copy, Clone)]
struct ss_swapmerge_stackitem {
    a: *mut saidx_t,
    b: *mut saidx_t,
    c: *mut saidx_t,
    d: saint_t,
}

/* D&C based merge. */
unsafe fn ss_swapmerge(
    T: *const sauchar_t,
    PA: *const saidx_t,
    mut first: *mut saidx_t,
    mut middle: *mut saidx_t,
    mut last: *mut saidx_t,
    buf: *mut saidx_t,
    bufsize: saidx_t,
    depth: saidx_t,
) {
    /* STACK_SIZE == SS_SMERGE_STACKSIZE */
    let mut stack: [ss_swapmerge_stackitem; SS_SMERGE_STACKSIZE] = [ss_swapmerge_stackitem {
        a: ptr::null_mut(),
        b: ptr::null_mut(),
        c: ptr::null_mut(),
        d: 0,
    }; SS_SMERGE_STACKSIZE];
    let mut l: *mut saidx_t;
    let mut r: *mut saidx_t;
    let mut lm: *mut saidx_t;
    let mut rm: *mut saidx_t;
    let mut m: saidx_t;
    let mut len: saidx_t;
    let mut half: saidx_t;
    let mut ssize: saint_t;
    let mut check: saint_t;
    let mut next: saint_t;

    check = 0;
    ssize = 0;
    loop {
        if ptr_diff(last, middle) <= bufsize {
            if (first < middle) && (middle < last) {
                ss_mergebackward(T, PA, first, middle, last, buf, depth);
            }
            ss_MERGE_CHECK(T, PA, depth, first, last, check);
            STACK_POP!(stack, ssize, first, middle, last, check);
            continue;
        }

        if ptr_diff(middle, first) <= bufsize {
            if first < middle {
                ss_mergeforward(T, PA, first, middle, last, buf, depth);
            }
            ss_MERGE_CHECK(T, PA, depth, first, last, check);
            STACK_POP!(stack, ssize, first, middle, last, check);
            continue;
        }

        m = 0;
        len = MIN(ptr_diff(middle, first), ptr_diff(last, middle));
        half = len >> 1;
        while 0 < len {
            if ss_compare(
                T,
                PA.offset(ss_GETIDX(*middle.offset((m + half) as isize)) as isize),
                PA.offset(ss_GETIDX(*middle.offset((-m - half - 1) as isize)) as isize),
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
            r = middle;
            l = r;
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

            if ptr_diff(l, first) <= ptr_diff(last, r) {
                STACK_PUSH!(stack, ssize, r, rm, last, (next & 3) | (check & 4));
                middle = lm;
                last = l;
                check = (check & 3) | (next & 4);
            } else {
                if ((next & 2) != 0) && (r == middle) {
                    next ^= 6;
                }
                STACK_PUSH!(stack, ssize, first, lm, l, (check & 3) | (next & 4));
                first = r;
                middle = rm;
                check = (next & 3) | (check & 4);
            }
        } else {
            if ss_compare(
                T,
                PA.offset(ss_GETIDX(*middle.offset(-1)) as isize),
                PA.offset(*middle as isize),
                depth,
            ) == 0
            {
                *middle = !*middle;
            }
            ss_MERGE_CHECK(T, PA, depth, first, last, check);
            STACK_POP!(stack, ssize, first, middle, last, check);
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Substring sort */
unsafe fn sssort(
    T: *const sauchar_t,
    PA: *const saidx_t,
    mut first: *mut saidx_t,
    last: *mut saidx_t,
    mut buf: *mut saidx_t,
    mut bufsize: saidx_t,
    depth: saidx_t,
    n: saidx_t,
    lastsuffix: saint_t,
) {
    let mut a: *mut saidx_t;
    /* SS_BLOCKSIZE != 0 */
    let mut b: *mut saidx_t;
    let middle: *mut saidx_t;
    let mut curbuf: *mut saidx_t;
    let mut j: saidx_t;
    let mut k: saidx_t;
    let mut curbufsize: saidx_t;
    let mut limit: saidx_t = 0;
    let mut i: saidx_t;

    if lastsuffix != 0 {
        first = first.offset(1);
    }

    let use_internal_buf: bool = (bufsize < SS_BLOCKSIZE)
        && (bufsize < ptr_diff(last, first))
        && {
            limit = ss_isqrt(ptr_diff(last, first));
            bufsize < limit
        };
    if use_internal_buf {
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
    while SS_BLOCKSIZE < ptr_diff(middle, a) {
        /* SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE */
        ss_mintrosort(T, PA, a, a.offset(SS_BLOCKSIZE as isize), depth);
        curbufsize = ptr_diff(last, a.offset(SS_BLOCKSIZE as isize));
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
        let mut PAi: [saidx_t; 2] = [0; 2];
        PAi[0] = *PA.offset(*first.offset(-1) as isize);
        PAi[1] = n - 2;
        a = first;
        i = *first.offset(-1);
        while (a < last)
            && ((*a < 0)
                || (0 < ss_compare(T, PAi.as_ptr(), PA.offset(*a as isize), depth)))
        {
            *a.offset(-1) = *a;
            a = a.offset(1);
        }
        *a.offset(-1) = i;
    }
}

/*---------------------------------------------------------------------------*/

#[inline]
fn tr_ilg(n: saidx_t) -> saint_t {
    if (n as u32 & 0xffff0000) != 0 {
        if (n as u32 & 0xff000000) != 0 {
            24 + lg_table[((n >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[((n >> 16) & 0xff) as usize]
        }
    } else {
        if (n as u32 & 0x0000ff00) != 0 {
            8 + lg_table[((n >> 8) & 0xff) as usize]
        } else {
            0 + lg_table[((n >> 0) & 0xff) as usize]
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Simple insertionsort for small size groups. */
unsafe fn tr_insertionsort(ISAd: *const saidx_t, first: *mut saidx_t, last: *mut saidx_t) {
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut t: saidx_t;
    let mut r: saint_t = 0;

    a = first.offset(1);
    while a < last {
        t = *a;
        b = a.offset(-1);
        loop {
            r = *ISAd.offset(t as isize) - *ISAd.offset(*b as isize);
            if !(0 > r) {
                break;
            }
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
unsafe fn tr_fixdown(ISAd: *const saidx_t, SA: *mut saidx_t, mut i: saidx_t, size: saidx_t) {
    let mut j: saidx_t;
    let mut k: saidx_t = 0;
    let v: saidx_t;
    let c: saidx_t;
    let mut d: saidx_t;
    let mut e: saidx_t;

    v = *SA.offset(i as isize);
    c = *ISAd.offset(v as isize);
    loop {
        j = 2 * i + 1;
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
unsafe fn tr_heapsort(ISAd: *const saidx_t, SA: *mut saidx_t, size: saidx_t) {
    let mut i: saidx_t;
    let mut m: saidx_t;
    let mut t: saidx_t;

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
    ISAd: *const saidx_t,
    mut v1: *mut saidx_t,
    mut v2: *mut saidx_t,
    v3: *mut saidx_t,
) -> *mut saidx_t {
    let t: *mut saidx_t;
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
    ISAd: *const saidx_t,
    mut v1: *mut saidx_t,
    mut v2: *mut saidx_t,
    mut v3: *mut saidx_t,
    mut v4: *mut saidx_t,
    mut v5: *mut saidx_t,
) -> *mut saidx_t {
    let mut t: *mut saidx_t;
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
unsafe fn tr_pivot(
    ISAd: *const saidx_t,
    mut first: *mut saidx_t,
    mut last: *mut saidx_t,
) -> *mut saidx_t {
    let mut middle: *mut saidx_t;
    let mut t: saidx_t;

    t = ptr_diff(last, first);
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
                last.offset(-1).offset(-(t as isize)),
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
        last.offset(-1).offset(-((t << 1) as isize)),
        last.offset(-1).offset(-(t as isize)),
        last.offset(-1),
    );
    tr_median3(ISAd, first, middle, last)
}

/*---------------------------------------------------------------------------*/

struct trbudget_t {
    chance: saidx_t,
    remain: saidx_t,
    incval: saidx_t,
    count: saidx_t,
}

#[inline]
unsafe fn trbudget_init(budget: *mut trbudget_t, chance: saidx_t, incval: saidx_t) {
    (*budget).chance = chance;
    (*budget).incval = incval;
    (*budget).remain = (*budget).incval;
}

#[inline]
unsafe fn trbudget_check(budget: *mut trbudget_t, size: saidx_t) -> saint_t {
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
    ISAd: *const saidx_t,
    mut first: *mut saidx_t,
    middle: *mut saidx_t,
    mut last: *mut saidx_t,
    pa: *mut *mut saidx_t,
    pb: *mut *mut saidx_t,
    v: saidx_t,
) {
    let mut a: *mut saidx_t;
    let mut b: *mut saidx_t;
    let mut c: *mut saidx_t;
    let mut d: *mut saidx_t;
    let mut e: *mut saidx_t;
    let mut f: *mut saidx_t;
    let mut t: saidx_t;
    let mut s: saidx_t;
    let mut x: saidx_t = 0;

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
        s = ptr_diff(a, first);
        t = ptr_diff(b, a);
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
        s = ptr_diff(d, c);
        t = ptr_diff(last, d) - 1;
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
        first = first.offset(ptr_diff(b, a) as isize);
        last = last.offset(-(ptr_diff(d, c) as isize));
    }
    *pa = first;
    *pb = last;
}

unsafe fn tr_copy(
    ISA: *mut saidx_t,
    SA: *const saidx_t,
    first: *mut saidx_t,
    a: *mut saidx_t,
    b: *mut saidx_t,
    last: *mut saidx_t,
    depth: saidx_t,
) {
    /* sort suffixes of middle partition
    by using sorted order of suffixes of left and right partition. */
    let mut c: *mut saidx_t;
    let mut d: *mut saidx_t;
    let mut e: *mut saidx_t;
    let mut s: saidx_t;
    let v: saidx_t;

    v = ptr_diff(b, SA) - 1;
    c = first;
    d = a.offset(-1);
    while c <= d {
        s = *c - depth;
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(1);
            *d = s;
            *ISA.offset(s as isize) = ptr_diff(d, SA);
        }
        c = c.offset(1);
    }
    c = last.offset(-1);
    e = d.offset(1);
    d = b;
    while e < d {
        s = *c - depth;
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(-1);
            *d = s;
            *ISA.offset(s as isize) = ptr_diff(d, SA);
        }
        c = c.offset(-1);
    }
}

unsafe fn tr_partialcopy(
    ISA: *mut saidx_t,
    SA: *const saidx_t,
    first: *mut saidx_t,
    a: *mut saidx_t,
    b: *mut saidx_t,
    last: *mut saidx_t,
    depth: saidx_t,
) {
    let mut c: *mut saidx_t;
    let mut d: *mut saidx_t;
    let mut e: *mut saidx_t;
    let mut s: saidx_t;
    let v: saidx_t;
    let mut rank: saidx_t;
    let mut lastrank: saidx_t;
    let mut newrank: saidx_t = -1;

    v = ptr_diff(b, SA) - 1;
    lastrank = -1;
    c = first;
    d = a.offset(-1);
    while c <= d {
        s = *c - depth;
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(1);
            *d = s;
            rank = *ISA.offset((s + depth) as isize);
            if lastrank != rank {
                lastrank = rank;
                newrank = ptr_diff(d, SA);
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
            newrank = ptr_diff(e, SA);
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
        s = *c - depth;
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(-1);
            *d = s;
            rank = *ISA.offset((s + depth) as isize);
            if lastrank != rank {
                lastrank = rank;
                newrank = ptr_diff(d, SA);
            }
            *ISA.offset(s as isize) = newrank;
        }
        c = c.offset(-1);
    }
}

#[derive(Copy, Clone)]
struct tr_introsort_stackitem {
    a: *const saidx_t,
    b: *mut saidx_t,
    c: *mut saidx_t,
    d: saint_t,
    e: saint_t,
}

unsafe fn tr_introsort(
    ISA: *mut saidx_t,
    mut ISAd: *const saidx_t,
    SA: *mut saidx_t,
    mut first: *mut saidx_t,
    mut last: *mut saidx_t,
    budget: *mut trbudget_t,
) {
    /* STACK_SIZE == TR_STACKSIZE */
    let mut stack: [tr_introsort_stackitem; TR_STACKSIZE] = [tr_introsort_stackitem {
        a: ptr::null(),
        b: ptr::null_mut(),
        c: ptr::null_mut(),
        d: 0,
        e: 0,
    }; TR_STACKSIZE];
    let mut a: *mut saidx_t = ptr::null_mut();
    let mut b: *mut saidx_t = ptr::null_mut();
    let mut c: *mut saidx_t;
    let mut t: saidx_t;
    let mut v: saidx_t;
    let mut x: saidx_t = 0;
    let incr: saidx_t = ptr_diff(ISAd, ISA);
    let mut limit: saint_t;
    let mut next: saint_t = 0;
    let mut ssize: saint_t;
    let mut trlink: saint_t = -1;

    ssize = 0;
    limit = tr_ilg(ptr_diff(last, first));
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
                    ptr_diff(last, SA) - 1,
                );

                /* update ranks */
                if a < last {
                    c = first;
                    v = ptr_diff(a, SA) - 1;
                    while c < a {
                        *ISA.offset(*c as isize) = v;
                        c = c.offset(1);
                    }
                }
                if b < last {
                    c = a;
                    v = ptr_diff(b, SA) - 1;
                    while c < b {
                        *ISA.offset(*c as isize) = v;
                        c = c.offset(1);
                    }
                }

                /* push */
                if 1 < ptr_diff(b, a) {
                    STACK_PUSH5!(stack, ssize, ptr::null(), a, b, 0, 0);
                    STACK_PUSH5!(
                        stack,
                        ssize,
                        ISAd.offset(-(incr as isize)),
                        first,
                        last,
                        -2,
                        trlink
                    );
                    trlink = ssize - 2;
                }
                if ptr_diff(a, first) <= ptr_diff(last, b) {
                    if 1 < ptr_diff(a, first) {
                        STACK_PUSH5!(
                            stack,
                            ssize,
                            ISAd,
                            b,
                            last,
                            tr_ilg(ptr_diff(last, b)),
                            trlink
                        );
                        last = a;
                        limit = tr_ilg(ptr_diff(a, first));
                    } else if 1 < ptr_diff(last, b) {
                        first = b;
                        limit = tr_ilg(ptr_diff(last, b));
                    } else {
                        STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
                    }
                } else {
                    if 1 < ptr_diff(last, b) {
                        STACK_PUSH5!(
                            stack,
                            ssize,
                            ISAd,
                            first,
                            a,
                            tr_ilg(ptr_diff(a, first)),
                            trlink
                        );
                        first = b;
                        limit = tr_ilg(ptr_diff(last, b));
                    } else if 1 < ptr_diff(a, first) {
                        last = a;
                        limit = tr_ilg(ptr_diff(a, first));
                    } else {
                        STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
                    }
                }
            } else if limit == -2 {
                /* tandem repeat copy */
                ssize -= 1;
                a = stack[ssize as usize].b;
                b = stack[ssize as usize].c;
                if stack[ssize as usize].d == 0 {
                    tr_copy(ISA, SA, first, a, b, last, ptr_diff(ISAd, ISA));
                } else {
                    if 0 <= trlink {
                        stack[trlink as usize].d = -1;
                    }
                    tr_partialcopy(ISA, SA, first, a, b, last, ptr_diff(ISAd, ISA));
                }
                STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
            } else {
                /* sorted partition */
                if 0 <= *first {
                    a = first;
                    loop {
                        *ISA.offset(*a as isize) = ptr_diff(a, SA);
                        a = a.offset(1);
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
                        a = a.offset(1);
                        if !(*a < 0) {
                            break;
                        }
                    }
                    next = if *ISA.offset(*a as isize) != *ISAd.offset(*a as isize) {
                        tr_ilg(ptr_diff(a, first) + 1)
                    } else {
                        -1
                    };
                    a = a.offset(1);
                    if a < last {
                        b = first;
                        v = ptr_diff(a, SA) - 1;
                        while b < a {
                            *ISA.offset(*b as isize) = v;
                            b = b.offset(1);
                        }
                    }

                    /* push */
                    if trbudget_check(budget, ptr_diff(a, first)) != 0 {
                        if ptr_diff(a, first) <= ptr_diff(last, a) {
                            STACK_PUSH5!(stack, ssize, ISAd, a, last, -3, trlink);
                            ISAd = ISAd.offset(incr as isize);
                            last = a;
                            limit = next;
                        } else {
                            if 1 < ptr_diff(last, a) {
                                STACK_PUSH5!(
                                    stack,
                                    ssize,
                                    ISAd.offset(incr as isize),
                                    first,
                                    a,
                                    next,
                                    trlink
                                );
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
                        if 1 < ptr_diff(last, a) {
                            first = a;
                            limit = -3;
                        } else {
                            STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
                        }
                    }
                } else {
                    STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
                }
            }
            continue;
        }

        if ptr_diff(last, first) <= TR_INSERTIONSORT_THRESHOLD {
            tr_insertionsort(ISAd, first, last);
            limit = -3;
            continue;
        }

        {
            let old_limit = limit;
            limit -= 1;
            if old_limit == 0 {
                tr_heapsort(ISAd, first, ptr_diff(last, first));
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
        if ptr_diff(last, first) != ptr_diff(b, a) {
            next = if *ISA.offset(*a as isize) != v {
                tr_ilg(ptr_diff(b, a))
            } else {
                -1
            };

            /* update ranks */
            c = first;
            v = ptr_diff(a, SA) - 1;
            while c < a {
                *ISA.offset(*c as isize) = v;
                c = c.offset(1);
            }
            if b < last {
                c = a;
                v = ptr_diff(b, SA) - 1;
                while c < b {
                    *ISA.offset(*c as isize) = v;
                    c = c.offset(1);
                }
            }

            /* push */
            if (1 < ptr_diff(b, a)) && (trbudget_check(budget, ptr_diff(b, a)) != 0) {
                if ptr_diff(a, first) <= ptr_diff(last, b) {
                    if ptr_diff(last, b) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(a, first) {
                            STACK_PUSH5!(
                                stack,
                                ssize,
                                ISAd.offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            STACK_PUSH5!(stack, ssize, ISAd, b, last, limit, trlink);
                            last = a;
                        } else if 1 < ptr_diff(last, b) {
                            STACK_PUSH5!(
                                stack,
                                ssize,
                                ISAd.offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            first = b;
                        } else {
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if ptr_diff(a, first) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(a, first) {
                            STACK_PUSH5!(stack, ssize, ISAd, b, last, limit, trlink);
                            STACK_PUSH5!(
                                stack,
                                ssize,
                                ISAd.offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            last = a;
                        } else {
                            STACK_PUSH5!(stack, ssize, ISAd, b, last, limit, trlink);
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        STACK_PUSH5!(stack, ssize, ISAd, b, last, limit, trlink);
                        STACK_PUSH5!(stack, ssize, ISAd, first, a, limit, trlink);
                        ISAd = ISAd.offset(incr as isize);
                        first = a;
                        last = b;
                        limit = next;
                    }
                } else {
                    if ptr_diff(a, first) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(last, b) {
                            STACK_PUSH5!(
                                stack,
                                ssize,
                                ISAd.offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            STACK_PUSH5!(stack, ssize, ISAd, first, a, limit, trlink);
                            first = b;
                        } else if 1 < ptr_diff(a, first) {
                            STACK_PUSH5!(
                                stack,
                                ssize,
                                ISAd.offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            last = a;
                        } else {
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if ptr_diff(last, b) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(last, b) {
                            STACK_PUSH5!(stack, ssize, ISAd, first, a, limit, trlink);
                            STACK_PUSH5!(
                                stack,
                                ssize,
                                ISAd.offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            first = b;
                        } else {
                            STACK_PUSH5!(stack, ssize, ISAd, first, a, limit, trlink);
                            ISAd = ISAd.offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        STACK_PUSH5!(stack, ssize, ISAd, first, a, limit, trlink);
                        STACK_PUSH5!(stack, ssize, ISAd, b, last, limit, trlink);
                        ISAd = ISAd.offset(incr as isize);
                        first = a;
                        last = b;
                        limit = next;
                    }
                }
            } else {
                if (1 < ptr_diff(b, a)) && (0 <= trlink) {
                    stack[trlink as usize].d = -1;
                }
                if ptr_diff(a, first) <= ptr_diff(last, b) {
                    if 1 < ptr_diff(a, first) {
                        STACK_PUSH5!(stack, ssize, ISAd, b, last, limit, trlink);
                        last = a;
                    } else if 1 < ptr_diff(last, b) {
                        first = b;
                    } else {
                        STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
                    }
                } else {
                    if 1 < ptr_diff(last, b) {
                        STACK_PUSH5!(stack, ssize, ISAd, first, a, limit, trlink);
                        first = b;
                    } else if 1 < ptr_diff(a, first) {
                        last = a;
                    } else {
                        STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
                    }
                }
            }
        } else {
            if trbudget_check(budget, ptr_diff(last, first)) != 0 {
                limit = tr_ilg(ptr_diff(last, first));
                ISAd = ISAd.offset(incr as isize);
            } else {
                if 0 <= trlink {
                    stack[trlink as usize].d = -1;
                }
                STACK_POP5!(stack, ssize, ISAd, first, last, limit, trlink);
            }
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Tandem repeat sort */
unsafe fn trsort(ISA: *mut saidx_t, SA: *mut saidx_t, n: saidx_t, depth: saidx_t) {
    let mut ISAd: *mut saidx_t;
    let mut first: *mut saidx_t;
    let mut last: *mut saidx_t;
    let mut budget: trbudget_t = trbudget_t {
        chance: 0,
        remain: 0,
        incval: 0,
        count: 0,
    };
    let mut t: saidx_t;
    let mut skip: saidx_t;
    let mut unsorted: saidx_t;

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
                last = SA.offset(*ISA.offset(t as isize) as isize).offset(1);
                if 1 < ptr_diff(last, first) {
                    budget.count = 0;
                    tr_introsort(ISA, ISAd, SA, first, last, &mut budget);
                    if budget.count != 0 {
                        unsorted += budget.count;
                    } else {
                        skip = ptr_diff(first, last);
                    }
                } else if ptr_diff(last, first) == 1 {
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
        ISAd = ISAd.offset(ptr_diff(ISAd, ISA) as isize);
    }
}

/*---------------------------------------------------------------------------*/

/* Sorts suffixes of type B*. */
unsafe fn sort_typeBstar(
    T: *const sauchar_t,
    SA: *mut saidx_t,
    bucket_A: *mut saidx_t,
    bucket_B: *mut saidx_t,
    n: saidx_t,
    openMP: saint_t,
) -> saidx_t {
    let PAb: *mut saidx_t;
    let ISAb: *mut saidx_t;
    let buf: *mut saidx_t;
    let mut i: saidx_t;
    let mut j: saidx_t;
    let mut k: saidx_t;
    let mut t: saidx_t;
    let mut m: saidx_t;
    let bufsize: saidx_t;
    let mut c0: saidx_t;
    let mut c1: saidx_t;
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
    c0 = *T.offset((n - 1) as isize) as saidx_t;
    c1 = 0;
    while 0 <= i {
        /* type A suffix. */
        loop {
            c1 = c0;
            *BUCKET_A(bucket_A, c1) += 1;
            i -= 1;
            if !(0 <= i) {
                break;
            }
            c0 = *T.offset(i as isize) as saidx_t;
            if !(c0 >= c1) {
                break;
            }
        }
        if 0 <= i {
            /* type B* suffix. */
            *BUCKET_BSTAR(bucket_B, c0, c1) += 1;
            m -= 1;
            *SA.offset(m as isize) = i;
            /* type B suffix. */
            i -= 1;
            c1 = c0;
            while 0 <= i {
                c0 = *T.offset(i as isize) as saidx_t;
                if !(c0 <= c1) {
                    break;
                }
                *BUCKET_B(bucket_B, c0, c1) += 1;
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
        t = i + *BUCKET_A(bucket_A, c0);
        *BUCKET_A(bucket_A, c0) = i + j; /* start point */
        i = t + *BUCKET_B(bucket_B, c0, c0);
        c1 = c0 + 1;
        while c1 < ALPHABET_SIZE {
            j += *BUCKET_BSTAR(bucket_B, c0, c1);
            *BUCKET_BSTAR(bucket_B, c0, c1) = j; /* end point */
            i += *BUCKET_B(bucket_B, c0, c1);
            c1 += 1;
        }
        c0 += 1;
    }

    if 0 < m {
        /* Sort the type B* suffixes by their first two characters. */
        PAb = SA.offset(n as isize).offset(-(m as isize));
        ISAb = SA.offset(m as isize);
        i = m - 2;
        while 0 <= i {
            t = *PAb.offset(i as isize);
            c0 = *T.offset(t as isize) as saidx_t;
            c1 = *T.offset((t + 1) as isize) as saidx_t;
            let bp = BUCKET_BSTAR(bucket_B, c0, c1);
            *bp -= 1;
            *SA.offset(*bp as isize) = i;
            i -= 1;
        }
        t = *PAb.offset((m - 1) as isize);
        c0 = *T.offset(t as isize) as saidx_t;
        c1 = *T.offset((t + 1) as isize) as saidx_t;
        {
            let bp = BUCKET_BSTAR(bucket_B, c0, c1);
            *bp -= 1;
            *SA.offset(*bp as isize) = m - 1;
        }

        /* Sort the type B* substrings using sssort. */
        buf = SA.offset(m as isize);
        bufsize = n - (2 * m);
        c0 = ALPHABET_SIZE - 2;
        j = m;
        i = 0;
        while 0 < j {
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                i = *BUCKET_BSTAR(bucket_B, c0, c1);
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
                        (*SA.offset(i as isize) == (m - 1)) as saint_t,
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
                    if !((0 <= i) && (0 <= *SA.offset(i as isize))) {
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
        c0 = *T.offset((n - 1) as isize) as saidx_t;
        while 0 <= i {
            i -= 1;
            c1 = c0;
            while 0 <= i {
                c0 = *T.offset(i as isize) as saidx_t;
                if !(c0 >= c1) {
                    break;
                }
                i -= 1;
                c1 = c0;
            }
            if 0 <= i {
                t = i;
                i -= 1;
                c1 = c0;
                while 0 <= i {
                    c0 = *T.offset(i as isize) as saidx_t;
                    if !(c0 <= c1) {
                        break;
                    }
                    i -= 1;
                    c1 = c0;
                }
                j -= 1;
                *SA.offset(*ISAb.offset(j as isize) as isize) =
                    if (t == 0) || (1 < (t - i)) { t } else { !t };
            }
        }

        /* Calculate the index of start/end point of each bucket. */
        *BUCKET_B(bucket_B, ALPHABET_SIZE - 1, ALPHABET_SIZE - 1) = n; /* end point */
        c0 = ALPHABET_SIZE - 2;
        k = m - 1;
        while 0 <= c0 {
            i = *BUCKET_A(bucket_A, c0 + 1) - 1;
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                t = i - *BUCKET_B(bucket_B, c0, c1);
                *BUCKET_B(bucket_B, c0, c1) = i; /* end point */

                /* Move all type B* suffixes to the correct position. */
                i = t;
                j = *BUCKET_BSTAR(bucket_B, c0, c1);
                while j <= k {
                    *SA.offset(i as isize) = *SA.offset(k as isize);
                    i -= 1;
                    k -= 1;
                }
                c1 -= 1;
            }
            *BUCKET_BSTAR(bucket_B, c0, c0 + 1) = i - *BUCKET_B(bucket_B, c0, c0) + 1; /* start point */
            *BUCKET_B(bucket_B, c0, c0) = i; /* end point */
            c0 -= 1;
        }
    }

    m
}

/* Constructs the suffix array by using the sorted order of type B* suffixes. */
unsafe fn construct_SA(
    T: *const sauchar_t,
    SA: *mut saidx_t,
    bucket_A: *mut saidx_t,
    bucket_B: *mut saidx_t,
    n: saidx_t,
    m: saidx_t,
) {
    let mut i: *mut saidx_t;
    let mut j: *mut saidx_t;
    let mut k: *mut saidx_t;
    let mut s: saidx_t;
    let mut c0: saidx_t;
    let mut c1: saidx_t;
    let mut c2: saidx_t = 0;

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.offset(*BUCKET_BSTAR(bucket_B, c1, c1 + 1) as isize);
            j = SA
                .offset(*BUCKET_A(bucket_A, c1 + 1) as isize)
                .offset(-1);
            k = ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    *j = !s;
                    s -= 1;
                    c0 = *T.offset(s as isize) as saidx_t;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as saidx_t) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *BUCKET_B(bucket_B, c2, c1) = ptr_diff(k, SA);
                        }
                        c2 = c0;
                        k = SA.offset(*BUCKET_B(bucket_B, c2, c1) as isize);
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
    c2 = *T.offset((n - 1) as isize) as saidx_t;
    k = SA.offset(*BUCKET_A(bucket_A, c2) as isize);
    *k = if (*T.offset((n - 2) as isize) as saidx_t) < c2 {
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
            c0 = *T.offset(s as isize) as saidx_t;
            if (s == 0) || ((*T.offset((s - 1) as isize) as saidx_t) < c0) {
                s = !s;
            }
            if c0 != c2 {
                *BUCKET_A(bucket_A, c2) = ptr_diff(k, SA);
                c2 = c0;
                k = SA.offset(*BUCKET_A(bucket_A, c2) as isize);
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
    T: *const sauchar_t,
    SA: *mut saidx_t,
    bucket_A: *mut saidx_t,
    bucket_B: *mut saidx_t,
    n: saidx_t,
    m: saidx_t,
) -> saidx_t {
    let mut i: *mut saidx_t;
    let mut j: *mut saidx_t;
    let mut k: *mut saidx_t;
    let mut orig: *mut saidx_t;
    let mut s: saidx_t;
    let mut c0: saidx_t;
    let mut c1: saidx_t;
    let mut c2: saidx_t = 0;

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.offset(*BUCKET_BSTAR(bucket_B, c1, c1 + 1) as isize);
            j = SA
                .offset(*BUCKET_A(bucket_A, c1 + 1) as isize)
                .offset(-1);
            k = ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    s -= 1;
                    c0 = *T.offset(s as isize) as saidx_t;
                    *j = !c0;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as saidx_t) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *BUCKET_B(bucket_B, c2, c1) = ptr_diff(k, SA);
                        }
                        c2 = c0;
                        k = SA.offset(*BUCKET_B(bucket_B, c2, c1) as isize);
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
    c2 = *T.offset((n - 1) as isize) as saidx_t;
    k = SA.offset(*BUCKET_A(bucket_A, c2) as isize);
    *k = if (*T.offset((n - 2) as isize) as saidx_t) < c2 {
        !(*T.offset((n - 2) as isize) as saidx_t)
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
            c0 = *T.offset(s as isize) as saidx_t;
            *i = c0;
            if (0 < s) && ((*T.offset((s - 1) as isize) as saidx_t) < c0) {
                s = !(*T.offset((s - 1) as isize) as saidx_t);
            }
            if c0 != c2 {
                *BUCKET_A(bucket_A, c2) = ptr_diff(k, SA);
                c2 = c0;
                k = SA.offset(*BUCKET_A(bucket_A, c2) as isize);
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

    ptr_diff(orig, SA)
}

/* Constructs the burrows-wheeler transformed string directly
by using the sorted order of type B* suffixes. */
unsafe fn construct_BWT_indexes(
    T: *const sauchar_t,
    SA: *mut saidx_t,
    bucket_A: *mut saidx_t,
    bucket_B: *mut saidx_t,
    n: saidx_t,
    m: saidx_t,
    num_indexes: *mut sauchar_t,
    indexes: *mut saidx_t,
) -> saidx_t {
    let mut i: *mut saidx_t;
    let mut j: *mut saidx_t;
    let mut k: *mut saidx_t;
    let mut orig: *mut saidx_t;
    let mut s: saidx_t;
    let mut c0: saidx_t;
    let mut c1: saidx_t;
    let mut c2: saidx_t = 0;

    let mut r#mod: saidx_t = n / 8;
    {
        r#mod |= r#mod >> 1;
        r#mod |= r#mod >> 2;
        r#mod |= r#mod >> 4;
        r#mod |= r#mod >> 8;
        r#mod |= r#mod >> 16;
        r#mod >>= 1;

        *num_indexes = ((n - 1) / (r#mod + 1)) as sauchar_t;
    }

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.offset(*BUCKET_BSTAR(bucket_B, c1, c1 + 1) as isize);
            j = SA
                .offset(*BUCKET_A(bucket_A, c1 + 1) as isize)
                .offset(-1);
            k = ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    if (s & r#mod) == 0 {
                        *indexes.offset((s / (r#mod + 1) - 1) as isize) = ptr_diff(j, SA);
                    }

                    s -= 1;
                    c0 = *T.offset(s as isize) as saidx_t;
                    *j = !c0;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as saidx_t) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *BUCKET_B(bucket_B, c2, c1) = ptr_diff(k, SA);
                        }
                        c2 = c0;
                        k = SA.offset(*BUCKET_B(bucket_B, c2, c1) as isize);
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
    c2 = *T.offset((n - 1) as isize) as saidx_t;
    k = SA.offset(*BUCKET_A(bucket_A, c2) as isize);
    if (*T.offset((n - 2) as isize) as saidx_t) < c2 {
        if ((n - 1) & r#mod) == 0 {
            *indexes.offset(((n - 1) / (r#mod + 1) - 1) as isize) = ptr_diff(k, SA);
        }
        *k = !(*T.offset((n - 2) as isize) as saidx_t);
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
                *indexes.offset((s / (r#mod + 1) - 1) as isize) = ptr_diff(i, SA);
            }

            s -= 1;
            c0 = *T.offset(s as isize) as saidx_t;
            *i = c0;
            if c0 != c2 {
                *BUCKET_A(bucket_A, c2) = ptr_diff(k, SA);
                c2 = c0;
                k = SA.offset(*BUCKET_A(bucket_A, c2) as isize);
            }
            if (0 < s) && ((*T.offset((s - 1) as isize) as saidx_t) < c0) {
                if (s & r#mod) == 0 {
                    *indexes.offset((s / (r#mod + 1) - 1) as isize) = ptr_diff(k, SA);
                }
                *k = !(*T.offset((s - 1) as isize) as saidx_t);
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

    ptr_diff(orig, SA)
}

/*---------------------------------------------------------------------------*/

/*- Function -*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divsufsort(
    T: *const sauchar_t,
    SA: *mut saidx_t,
    n: saidx_t,
    openMP: saint_t,
) -> saidx_t {
    let bucket_A: *mut saidx_t;
    let bucket_B: *mut saidx_t;
    let m: saidx_t;
    let mut err: saidx_t = 0;

    /* Check arguments. */
    if T.is_null() || SA.is_null() || (n < 0) {
        return -1;
    } else if n == 0 {
        return 0;
    } else if n == 1 {
        *SA.offset(0) = 0;
        return 0;
    } else if n == 2 {
        let m2: saidx_t = (*T.offset(0) < *T.offset(1)) as saidx_t;
        *SA.offset((m2 ^ 1) as isize) = 0;
        *SA.offset(m2 as isize) = 1;
        return 0;
    }

    bucket_A = malloc(BUCKET_A_SIZE as usize * size_of::<saidx_t>()) as *mut saidx_t;
    bucket_B = malloc(BUCKET_B_SIZE as usize * size_of::<saidx_t>()) as *mut saidx_t;

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
    T: *const sauchar_t,
    U: *mut sauchar_t,
    A: *mut saidx_t,
    n: saidx_t,
    num_indexes: *mut sauchar_t,
    indexes: *mut saidx_t,
    openMP: saint_t,
) -> saidx_t {
    let mut B: *mut saidx_t;
    let bucket_A: *mut saidx_t;
    let bucket_B: *mut saidx_t;
    let m: saidx_t;
    let mut pidx: saidx_t;
    let mut i: saidx_t;

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
        B = malloc((n + 1) as usize * size_of::<saidx_t>()) as *mut saidx_t;
    }
    bucket_A = malloc(BUCKET_A_SIZE as usize * size_of::<saidx_t>()) as *mut saidx_t;
    bucket_B = malloc(BUCKET_B_SIZE as usize * size_of::<saidx_t>()) as *mut saidx_t;

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
            *U.offset((i + 1) as isize) = *B.offset(i as isize) as sauchar_t;
            i += 1;
        }
        i += 1;
        while i < n {
            *U.offset(i as isize) = *B.offset(i as isize) as sauchar_t;
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

//! Literal translation of `c_src/src/dictBuilder/divsufsort.c`
//! (libdivsufsort-lite, Copyright (c) 2003-2008 Yuta Mori).

use core::ffi::{c_int, c_uchar, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/*- Constants -*/
const ALPHABET_SIZE: c_int = 256;
const BUCKET_A_SIZE: c_int = ALPHABET_SIZE;
const BUCKET_B_SIZE: c_int = ALPHABET_SIZE * ALPHABET_SIZE;
const SS_INSERTIONSORT_THRESHOLD: c_int = 8;
const SS_BLOCKSIZE: c_int = 1024;
/* minstacksize = log(SS_BLOCKSIZE) / log(3) * 2 ; SS_BLOCKSIZE <= 4096 -> 16 */
const SS_MISORT_STACKSIZE: usize = 16;
const SS_SMERGE_STACKSIZE: usize = 32;
const TR_INSERTIONSORT_THRESHOLD: c_int = 8;
const TR_STACKSIZE: usize = 64;

/*- Macros -*/
// BUCKET_A(_c0) bucket_A[(_c0)]
macro_rules! BUCKET_A {
    ($bucket_A:expr, $c0:expr) => {
        *$bucket_A.offset(($c0) as isize)
    };
}
// ALPHABET_SIZE == 256:
// BUCKET_B(_c0, _c1)     (bucket_B[((_c1) << 8) | (_c0)])
// BUCKET_BSTAR(_c0, _c1) (bucket_B[((_c0) << 8) | (_c1)])
macro_rules! BUCKET_B {
    ($bucket_B:expr, $c0:expr, $c1:expr) => {
        *$bucket_B.offset(((($c1) << 8) | ($c0)) as isize)
    };
}
macro_rules! BUCKET_BSTAR {
    ($bucket_B:expr, $c0:expr, $c1:expr) => {
        *$bucket_B.offset(((($c0) << 8) | ($c1)) as isize)
    };
}

/*- Private Functions -*/

static lg_table: [c_int; 256] = [
    -1, 0, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

// SS_BLOCKSIZE (1024) so SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE branch is active,
// and SS_BLOCKSIZE >= 256 branch of ss_ilg.
pub unsafe fn ss_ilg(n: c_int) -> c_int {
    if (n & 0xff00) != 0 {
        8 + lg_table[((n >> 8) & 0xff) as usize]
    } else {
        0 + lg_table[((n >> 0) & 0xff) as usize]
    }
}

static sqq_table: [c_int; 256] = [
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

pub unsafe fn ss_isqrt(x: c_int) -> c_int {
    let y: c_int;
    let e: c_int;

    if x >= (SS_BLOCKSIZE * SS_BLOCKSIZE) {
        return SS_BLOCKSIZE;
    }
    e = if (x & 0xffff0000u32 as c_int) != 0 {
        if (x & 0xff000000u32 as c_int) != 0 {
            24 + lg_table[((x >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[((x >> 16) & 0xff) as usize]
        }
    } else if (x & 0x0000ff00) != 0 {
        8 + lg_table[((x >> 8) & 0xff) as usize]
    } else {
        0 + lg_table[((x >> 0) & 0xff) as usize]
    };

    if e >= 16 {
        let mut y2 = sqq_table[(x >> ((e - 6) - (e & 1))) as usize] << ((e >> 1) - 7);
        if e >= 24 {
            y2 = (y2 + 1 + x / y2) >> 1;
        }
        y2 = (y2 + 1 + x / y2) >> 1;
        y = y2;
    } else if e >= 8 {
        y = (sqq_table[(x >> ((e - 6) - (e & 1))) as usize] >> (7 - (e >> 1))) + 1;
    } else {
        return sqq_table[x as usize] >> 4;
    }

    if x < (y * y) { y - 1 } else { y }
}

/*---------------------------------------------------------------------------*/

/* Compares two suffixes. */
pub unsafe fn ss_compare(
    T: *const c_uchar,
    p1: *const c_int,
    p2: *const c_int,
    depth: c_int,
) -> c_int {
    let mut U1: *const c_uchar;
    let mut U2: *const c_uchar;
    let U1n: *const c_uchar;
    let U2n: *const c_uchar;

    U1 = T.offset((depth + *p1) as isize);
    U2 = T.offset((depth + *p2) as isize);
    U1n = T.offset((*(p1.offset(1)) + 2) as isize);
    U2n = T.offset((*(p2.offset(1)) + 2) as isize);
    while (U1 < U1n) && (U2 < U2n) && (*U1 == *U2) {
        U1 = U1.offset(1);
        U2 = U2.offset(1);
    }

    if U1 < U1n {
        if U2 < U2n {
            (*U1 as c_int) - (*U2 as c_int)
        } else {
            1
        }
    } else if U2 < U2n {
        -1
    } else {
        0
    }
}

/*---------------------------------------------------------------------------*/

/* Insertionsort for small size groups */
pub unsafe fn ss_insertionsort(
    T: *const c_uchar,
    PA: *const c_int,
    first: *mut c_int,
    last: *mut c_int,
    depth: c_int,
) {
    let mut i: *mut c_int;
    let mut j: *mut c_int;
    let mut t: c_int;
    let mut r: c_int;

    i = last.offset(-2);
    while first <= i {
        t = *i;
        j = i.offset(1);
        r = ss_compare(T, PA.offset(t as isize), PA.offset(*j as isize), depth);
        while 0 < r {
            loop {
                *(j.offset(-1)) = *j;
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
        *(j.offset(-1)) = t;
        i = i.offset(-1);
    }
}

/*---------------------------------------------------------------------------*/

pub unsafe fn ss_fixdown(
    Td: *const c_uchar,
    PA: *const c_int,
    SA: *mut c_int,
    mut i: c_int,
    size: c_int,
) {
    let mut j: c_int;
    let mut k: c_int;
    let v: c_int;
    let c: c_int;
    let mut d: c_int;
    let mut e: c_int;

    v = *SA.offset(i as isize);
    c = *Td.offset(*PA.offset(v as isize) as isize) as c_int;
    loop {
        j = 2 * i + 1;
        if !(j < size) {
            break;
        }
        k = j;
        d = *Td.offset(*PA.offset(*SA.offset(k as isize) as isize) as isize) as c_int;
        j += 1;
        e = *Td.offset(*PA.offset(*SA.offset(j as isize) as isize) as isize) as c_int;
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
pub unsafe fn ss_heapsort(Td: *const c_uchar, PA: *const c_int, SA: *mut c_int, size: c_int) {
    let mut i: c_int;
    let mut m: c_int;
    let mut t: c_int;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if (*Td.offset(*PA.offset(*SA.offset((m / 2) as isize) as isize) as isize) as c_int)
            < (*Td.offset(*PA.offset(*SA.offset(m as isize) as isize) as isize) as c_int)
        {
            // SWAP(SA[m], SA[m / 2])
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
        // SWAP(SA[0], SA[m])
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
pub unsafe fn ss_median3(
    Td: *const c_uchar,
    PA: *const c_int,
    mut v1: *mut c_int,
    mut v2: *mut c_int,
    v3: *mut c_int,
) -> *mut c_int {
    let t: *mut c_int;
    if (*Td.offset(*PA.offset(*v1 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v2 as isize) as isize) as c_int)
    {
        t = v1;
        v1 = v2;
        v2 = t;
    }
    if (*Td.offset(*PA.offset(*v2 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v3 as isize) as isize) as c_int)
    {
        if (*Td.offset(*PA.offset(*v1 as isize) as isize) as c_int)
            > (*Td.offset(*PA.offset(*v3 as isize) as isize) as c_int)
        {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

/* Returns the median of five elements. */
pub unsafe fn ss_median5(
    Td: *const c_uchar,
    PA: *const c_int,
    mut v1: *mut c_int,
    mut v2: *mut c_int,
    mut v3: *mut c_int,
    mut v4: *mut c_int,
    mut v5: *mut c_int,
) -> *mut c_int {
    let mut t: *mut c_int;
    if (*Td.offset(*PA.offset(*v2 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v3 as isize) as isize) as c_int)
    {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if (*Td.offset(*PA.offset(*v4 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v5 as isize) as isize) as c_int)
    {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if (*Td.offset(*PA.offset(*v2 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v4 as isize) as isize) as c_int)
    {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if (*Td.offset(*PA.offset(*v1 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v3 as isize) as isize) as c_int)
    {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if (*Td.offset(*PA.offset(*v1 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v4 as isize) as isize) as c_int)
    {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if (*Td.offset(*PA.offset(*v3 as isize) as isize) as c_int)
        > (*Td.offset(*PA.offset(*v4 as isize) as isize) as c_int)
    {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
pub unsafe fn ss_pivot(
    Td: *const c_uchar,
    PA: *const c_int,
    mut first: *mut c_int,
    mut last: *mut c_int,
) -> *mut c_int {
    let mut middle: *mut c_int;
    let mut t: c_int;

    t = last.offset_from(first) as c_int;
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
    first = ss_median3(Td, PA, first, first.offset(t as isize), first.offset((t << 1) as isize));
    middle = ss_median3(Td, PA, middle.offset(-(t as isize)), middle, middle.offset(t as isize));
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
pub unsafe fn ss_partition(
    PA: *const c_int,
    first: *mut c_int,
    last: *mut c_int,
    depth: c_int,
) -> *mut c_int {
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let t: c_int;
    a = first.offset(-1);
    b = last;
    loop {
        loop {
            a = a.offset(1);
            if !((a < b) && ((*PA.offset(*a as isize) + depth) >= (*PA.offset((*a + 1) as isize) + 1)))
            {
                break;
            }
            *a = !*a;
        }
        loop {
            b = b.offset(-1);
            if !((a < b) && ((*PA.offset(*b as isize) + depth) < (*PA.offset((*b + 1) as isize) + 1)))
            {
                break;
            }
        }
        if b <= a {
            break;
        }
        // t = ~*b; *b = *a; *a = t;
        let tt = !*b;
        *b = *a;
        *a = tt;
    }
    let _ = t;
    if first < a {
        *first = !*first;
    }
    a
}

/* Multikey introsort for medium size groups. */
pub unsafe fn ss_mintrosort(
    T: *const c_uchar,
    PA: *const c_int,
    mut first: *mut c_int,
    mut last: *mut c_int,
    mut depth: c_int,
) {
    #[derive(Clone, Copy)]
    struct StackEntry {
        a: *mut c_int,
        b: *mut c_int,
        c: c_int,
        d: c_int,
    }
    let mut stack: [StackEntry; SS_MISORT_STACKSIZE] = [StackEntry {
        a: core::ptr::null_mut(),
        b: core::ptr::null_mut(),
        c: 0,
        d: 0,
    }; SS_MISORT_STACKSIZE];
    let mut ssize: usize = 0;

    macro_rules! STACK_PUSH {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {{
            stack[ssize].a = $a;
            stack[ssize].b = $b;
            stack[ssize].c = $c;
            stack[ssize].d = $d;
            ssize += 1;
        }};
    }

    let Td: *const c_uchar;
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut c: *mut c_int;
    let mut d: *mut c_int;
    let mut e: *mut c_int;
    let mut f: *mut c_int;
    let mut s: c_int;
    let mut t: c_int;
    let mut limit: c_int;
    let mut v: c_int;
    let mut x: c_int = 0;

    limit = ss_ilg(last.offset_from(first) as c_int);
    'outer: loop {
        if (last.offset_from(first) as c_int) <= SS_INSERTIONSORT_THRESHOLD {
            if 1 < (last.offset_from(first) as c_int) {
                ss_insertionsort(T, PA, first, last, depth);
            }
            // STACK_POP(first, last, depth, limit)
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            last = stack[ssize].b;
            depth = stack[ssize].c;
            limit = stack[ssize].d;
            continue 'outer;
        }

        let Tdl = T.offset(depth as isize);
        let Td = Tdl;
        let old_limit = limit;
        limit -= 1;
        if old_limit == 0 {
            ss_heapsort(Td, PA, first, last.offset_from(first) as c_int);
        }
        if limit < 0 {
            a = first.offset(1);
            v = *Td.offset(*PA.offset(*first as isize) as isize) as c_int;
            while a < last {
                x = *Td.offset(*PA.offset(*a as isize) as isize) as c_int;
                if x != v {
                    if 1 < (a.offset_from(first) as c_int) {
                        break;
                    }
                    v = x;
                    first = a;
                }
                a = a.offset(1);
            }
            if (*Td.offset((*PA.offset(*first as isize) - 1) as isize) as c_int) < v {
                first = ss_partition(PA, first, a, depth);
            }
            if (a.offset_from(first) as c_int) <= (last.offset_from(a) as c_int) {
                if 1 < (a.offset_from(first) as c_int) {
                    STACK_PUSH!(a, last, depth, -1);
                    last = a;
                    depth += 1;
                    limit = ss_ilg(a.offset_from(first) as c_int);
                } else {
                    first = a;
                    limit = -1;
                }
            } else {
                if 1 < (last.offset_from(a) as c_int) {
                    STACK_PUSH!(first, a, depth + 1, ss_ilg(a.offset_from(first) as c_int));
                    first = a;
                    limit = -1;
                } else {
                    last = a;
                    depth += 1;
                    limit = ss_ilg(a.offset_from(first) as c_int);
                }
            }
            continue 'outer;
        }

        /* choose pivot */
        a = ss_pivot(Td, PA, first, last);
        v = *Td.offset(*PA.offset(*a as isize) as isize) as c_int;
        // SWAP(*first, *a)
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
            x = *Td.offset(*PA.offset(*b as isize) as isize) as c_int;
            if x != v {
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
                x = *Td.offset(*PA.offset(*b as isize) as isize) as c_int;
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
            x = *Td.offset(*PA.offset(*c as isize) as isize) as c_int;
            if x != v {
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
                x = *Td.offset(*PA.offset(*c as isize) as isize) as c_int;
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
                x = *Td.offset(*PA.offset(*b as isize) as isize) as c_int;
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
                x = *Td.offset(*PA.offset(*c as isize) as isize) as c_int;
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

            s = a.offset_from(first) as c_int;
            t = b.offset_from(a) as c_int;
            if s > t {
                s = t;
            }
            e = first;
            f = b.offset(-(s as isize));
            while 0 < s {
                let tt = *e;
                *e = *f;
                *f = tt;
                s -= 1;
                e = e.offset(1);
                f = f.offset(1);
            }
            s = d.offset_from(c) as c_int;
            t = (last.offset_from(d) as c_int) - 1;
            if s > t {
                s = t;
            }
            e = b;
            f = last.offset(-(s as isize));
            while 0 < s {
                let tt = *e;
                *e = *f;
                *f = tt;
                s -= 1;
                e = e.offset(1);
                f = f.offset(1);
            }

            a = first.offset(b.offset_from(a));
            c = last.offset(-(d.offset_from(c)));
            b = if v <= (*Td.offset((*PA.offset(*a as isize) - 1) as isize) as c_int) {
                a
            } else {
                ss_partition(PA, a, c, depth)
            };

            if (a.offset_from(first) as c_int) <= (last.offset_from(c) as c_int) {
                if (last.offset_from(c) as c_int) <= (c.offset_from(b) as c_int) {
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(c.offset_from(b) as c_int));
                    STACK_PUSH!(c, last, depth, limit);
                    last = a;
                } else if (a.offset_from(first) as c_int) <= (c.offset_from(b) as c_int) {
                    STACK_PUSH!(c, last, depth, limit);
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(c.offset_from(b) as c_int));
                    last = a;
                } else {
                    STACK_PUSH!(c, last, depth, limit);
                    STACK_PUSH!(first, a, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(c.offset_from(b) as c_int);
                }
            } else {
                if (a.offset_from(first) as c_int) <= (c.offset_from(b) as c_int) {
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(c.offset_from(b) as c_int));
                    STACK_PUSH!(first, a, depth, limit);
                    first = c;
                } else if (last.offset_from(c) as c_int) <= (c.offset_from(b) as c_int) {
                    STACK_PUSH!(first, a, depth, limit);
                    STACK_PUSH!(b, c, depth + 1, ss_ilg(c.offset_from(b) as c_int));
                    first = c;
                } else {
                    STACK_PUSH!(first, a, depth, limit);
                    STACK_PUSH!(c, last, depth, limit);
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(c.offset_from(b) as c_int);
                }
            }
        } else {
            limit += 1;
            if (*Td.offset((*PA.offset(*first as isize) - 1) as isize) as c_int) < v {
                first = ss_partition(PA, first, last, depth);
                limit = ss_ilg(last.offset_from(first) as c_int);
            }
            depth += 1;
        }
    }
}

/*---------------------------------------------------------------------------*/

pub unsafe fn ss_blockswap(mut a: *mut c_int, mut b: *mut c_int, mut n: c_int) {
    let mut t: c_int;
    while 0 < n {
        t = *a;
        *a = *b;
        *b = t;
        n -= 1;
        a = a.offset(1);
        b = b.offset(1);
    }
}

pub unsafe fn ss_rotate(mut first: *mut c_int, middle: *mut c_int, mut last: *mut c_int) {
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut t: c_int;
    let mut l: c_int;
    let mut r: c_int;
    l = middle.offset_from(first) as c_int;
    r = last.offset_from(middle) as c_int;
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
                // *a-- = *b, *b-- = *a;
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
                // *a++ = *b, *b++ = *a;
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

pub unsafe fn ss_inplacemerge(
    T: *const c_uchar,
    PA: *const c_int,
    mut first: *mut c_int,
    mut middle: *mut c_int,
    mut last: *mut c_int,
    depth: c_int,
) {
    let mut p: *const c_int;
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut len: c_int;
    let mut half: c_int;
    let mut q: c_int;
    let mut r: c_int;
    let mut x: c_int;

    loop {
        if *(last.offset(-1)) < 0 {
            x = 1;
            p = PA.offset(!*(last.offset(-1)) as isize);
        } else {
            x = 0;
            p = PA.offset(*(last.offset(-1)) as isize);
        }
        a = first;
        len = middle.offset_from(first) as c_int;
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
            last = last.offset(-(middle.offset_from(a)));
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
pub unsafe fn ss_mergeforward(
    T: *const c_uchar,
    PA: *const c_int,
    first: *mut c_int,
    middle: *mut c_int,
    last: *mut c_int,
    buf: *mut c_int,
    depth: c_int,
) {
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut c: *mut c_int;
    let bufend: *mut c_int;
    let t: c_int;
    let mut r: c_int;

    bufend = buf.offset((middle.offset_from(first)) - 1);
    ss_blockswap(buf, first, middle.offset_from(first) as c_int);

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
pub unsafe fn ss_mergebackward(
    T: *const c_uchar,
    PA: *const c_int,
    first: *mut c_int,
    middle: *mut c_int,
    last: *mut c_int,
    buf: *mut c_int,
    depth: c_int,
) {
    let mut p1: *const c_int;
    let mut p2: *const c_int;
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut c: *mut c_int;
    let bufend: *mut c_int;
    let t: c_int;
    let mut r: c_int;
    let mut x: c_int;

    bufend = buf.offset((last.offset_from(middle)) - 1);
    ss_blockswap(buf, middle, last.offset_from(middle) as c_int);

    x = 0;
    if *bufend < 0 {
        p1 = PA.offset(!*bufend as isize);
        x |= 1;
    } else {
        p1 = PA.offset(*bufend as isize);
    }
    if *(middle.offset(-1)) < 0 {
        p2 = PA.offset(!*(middle.offset(-1)) as isize);
        x |= 2;
    } else {
        p2 = PA.offset(*(middle.offset(-1)) as isize);
    }
    a = last.offset(-1);
    t = *a;
    b = bufend;
    c = middle.offset(-1);
    loop {
        r = ss_compare(T, p1, p2, depth);
        if 0 < r {
            if x & 1 != 0 {
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
            if x & 2 != 0 {
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
            if x & 1 != 0 {
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
            if x & 2 != 0 {
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
pub unsafe fn ss_swapmerge(
    T: *const c_uchar,
    PA: *const c_int,
    mut first: *mut c_int,
    mut middle: *mut c_int,
    mut last: *mut c_int,
    buf: *mut c_int,
    bufsize: c_int,
    depth: c_int,
) {
    #[derive(Clone, Copy)]
    struct StackEntry {
        a: *mut c_int,
        b: *mut c_int,
        c: *mut c_int,
        d: c_int,
    }
    let mut stack: [StackEntry; SS_SMERGE_STACKSIZE] = [StackEntry {
        a: core::ptr::null_mut(),
        b: core::ptr::null_mut(),
        c: core::ptr::null_mut(),
        d: 0,
    }; SS_SMERGE_STACKSIZE];
    let mut ssize: usize = 0;

    macro_rules! GETIDX {
        ($a:expr) => {
            if 0 <= ($a) { ($a) } else { !($a) }
        };
    }
    // MERGE_CHECK operates on (first, last, check) with the given a,b args.
    macro_rules! MERGE_CHECK {
        ($a:expr, $b:expr, $c:expr) => {{
            if (($c) & 1 != 0)
                || ((($c) & 2 != 0)
                    && (ss_compare(
                        T,
                        PA.offset(GETIDX!(*(($a).offset(-1))) as isize),
                        PA.offset(*($a) as isize),
                        depth,
                    ) == 0))
            {
                *($a) = !*($a);
            }
            if (($c) & 4 != 0)
                && (ss_compare(
                    T,
                    PA.offset(GETIDX!(*(($b).offset(-1))) as isize),
                    PA.offset(*($b) as isize),
                    depth,
                ) == 0)
            {
                *($b) = !*($b);
            }
        }};
    }

    let mut l: *mut c_int;
    let mut r: *mut c_int;
    let mut lm: *mut c_int;
    let mut rm: *mut c_int;
    let mut m: c_int;
    let mut len: c_int;
    let mut half: c_int;
    let mut check: c_int;
    let mut next: c_int;

    check = 0;
    'outer: loop {
        if (last.offset_from(middle) as c_int) <= bufsize {
            if (first < middle) && (middle < last) {
                ss_mergebackward(T, PA, first, middle, last, buf, depth);
            }
            MERGE_CHECK!(first, last, check);
            // STACK_POP(first, middle, last, check)
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            middle = stack[ssize].b;
            last = stack[ssize].c;
            check = stack[ssize].d;
            continue 'outer;
        }

        if (middle.offset_from(first) as c_int) <= bufsize {
            if first < middle {
                ss_mergeforward(T, PA, first, middle, last, buf, depth);
            }
            MERGE_CHECK!(first, last, check);
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            middle = stack[ssize].b;
            last = stack[ssize].c;
            check = stack[ssize].d;
            continue 'outer;
        }

        m = 0;
        len = {
            let a = middle.offset_from(first) as c_int;
            let b = last.offset_from(middle) as c_int;
            if a < b { a } else { b }
        };
        half = len >> 1;
        while 0 < len {
            if ss_compare(
                T,
                PA.offset(GETIDX!(*(middle.offset((m + half) as isize))) as isize),
                PA.offset(GETIDX!(*(middle.offset(-((m + half + 1) as isize)))) as isize),
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

            if (l.offset_from(first) as c_int) <= (last.offset_from(r) as c_int) {
                stack[ssize].a = r;
                stack[ssize].b = rm;
                stack[ssize].c = last;
                stack[ssize].d = (next & 3) | (check & 4);
                ssize += 1;
                middle = lm;
                last = l;
                check = (check & 3) | (next & 4);
            } else {
                if (next & 2 != 0) && (r == middle) {
                    next ^= 6;
                }
                stack[ssize].a = first;
                stack[ssize].b = lm;
                stack[ssize].c = l;
                stack[ssize].d = (check & 3) | (next & 4);
                ssize += 1;
                first = r;
                middle = rm;
                check = (next & 3) | (check & 4);
            }
        } else {
            if ss_compare(
                T,
                PA.offset(GETIDX!(*(middle.offset(-1))) as isize),
                PA.offset(*middle as isize),
                depth,
            ) == 0
            {
                *middle = !*middle;
            }
            MERGE_CHECK!(first, last, check);
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            middle = stack[ssize].b;
            last = stack[ssize].c;
            check = stack[ssize].d;
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Substring sort */
pub unsafe fn sssort(
    T: *const c_uchar,
    PA: *const c_int,
    mut first: *mut c_int,
    last: *mut c_int,
    mut buf: *mut c_int,
    mut bufsize: c_int,
    depth: c_int,
    n: c_int,
    lastsuffix: c_int,
) {
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut middle: *mut c_int;
    let mut curbuf: *mut c_int;
    let mut j: c_int;
    let mut k: c_int;
    let mut curbufsize: c_int;
    let mut limit: c_int;
    let mut i: c_int;

    if lastsuffix != 0 {
        first = first.offset(1);
    }

    if (bufsize < SS_BLOCKSIZE)
        && (bufsize < (last.offset_from(first) as c_int))
        && {
            limit = ss_isqrt(last.offset_from(first) as c_int);
            bufsize < limit
        }
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
    while SS_BLOCKSIZE < (middle.offset_from(a) as c_int) {
        ss_mintrosort(T, PA, a, a.offset(SS_BLOCKSIZE as isize), depth);
        curbufsize = last.offset_from(a.offset(SS_BLOCKSIZE as isize)) as c_int;
        curbuf = a.offset(SS_BLOCKSIZE as isize);
        if curbufsize <= bufsize {
            curbufsize = bufsize;
            curbuf = buf;
        }
        b = a;
        k = SS_BLOCKSIZE;
        j = i;
        while j & 1 != 0 {
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
    ss_mintrosort(T, PA, a, middle, depth);
    k = SS_BLOCKSIZE;
    while i != 0 {
        if i & 1 != 0 {
            ss_swapmerge(T, PA, a.offset(-(k as isize)), a, middle, buf, bufsize, depth);
            a = a.offset(-(k as isize));
        }
        k <<= 1;
        i >>= 1;
    }
    if limit != 0 {
        ss_mintrosort(T, PA, middle, last, depth);
        ss_inplacemerge(T, PA, first, middle, last, depth);
    }

    if lastsuffix != 0 {
        /* Insert last type B* suffix. */
        let mut PAi: [c_int; 2] = [0; 2];
        PAi[0] = *PA.offset(*(first.offset(-1)) as isize);
        PAi[1] = n - 2;
        a = first;
        i = *(first.offset(-1));
        while (a < last)
            && ((*a < 0)
                || (0 < ss_compare(T, &PAi[0] as *const c_int, PA.offset(*a as isize), depth)))
        {
            *(a.offset(-1)) = *a;
            a = a.offset(1);
        }
        *(a.offset(-1)) = i;
    }
}

/*---------------------------------------------------------------------------*/

pub unsafe fn tr_ilg(n: c_int) -> c_int {
    if (n & 0xffff0000u32 as c_int) != 0 {
        if (n & 0xff000000u32 as c_int) != 0 {
            24 + lg_table[((n >> 24) & 0xff) as usize]
        } else {
            16 + lg_table[((n >> 16) & 0xff) as usize]
        }
    } else if (n & 0x0000ff00) != 0 {
        8 + lg_table[((n >> 8) & 0xff) as usize]
    } else {
        0 + lg_table[((n >> 0) & 0xff) as usize]
    }
}

/*---------------------------------------------------------------------------*/

/* Simple insertionsort for small size groups. */
pub unsafe fn tr_insertionsort(ISAd: *const c_int, first: *mut c_int, last: *mut c_int) {
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut t: c_int;
    let mut r: c_int;

    a = first.offset(1);
    while a < last {
        t = *a;
        b = a.offset(-1);
        r = *ISAd.offset(t as isize) - *ISAd.offset(*b as isize);
        while 0 > r {
            loop {
                *(b.offset(1)) = *b;
                b = b.offset(-1);
                if !((first <= b) && (*b < 0)) {
                    break;
                }
            }
            if b < first {
                break;
            }
            r = *ISAd.offset(t as isize) - *ISAd.offset(*b as isize);
        }
        if r == 0 {
            *b = !*b;
        }
        *(b.offset(1)) = t;
        a = a.offset(1);
    }
}

/*---------------------------------------------------------------------------*/

pub unsafe fn tr_fixdown(ISAd: *const c_int, SA: *mut c_int, mut i: c_int, size: c_int) {
    let mut j: c_int;
    let mut k: c_int;
    let v: c_int;
    let c: c_int;
    let mut d: c_int;
    let mut e: c_int;

    v = *SA.offset(i as isize);
    c = *ISAd.offset(v as isize);
    loop {
        j = 2 * i + 1;
        if !(j < size) {
            break;
        }
        k = j;
        d = *ISAd.offset(*SA.offset(k as isize) as isize);
        j += 1;
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
pub unsafe fn tr_heapsort(ISAd: *const c_int, SA: *mut c_int, size: c_int) {
    let mut i: c_int;
    let mut m: c_int;
    let mut t: c_int;

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
pub unsafe fn tr_median3(
    ISAd: *const c_int,
    mut v1: *mut c_int,
    mut v2: *mut c_int,
    v3: *mut c_int,
) -> *mut c_int {
    let t: *mut c_int;
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
pub unsafe fn tr_median5(
    ISAd: *const c_int,
    mut v1: *mut c_int,
    mut v2: *mut c_int,
    mut v3: *mut c_int,
    mut v4: *mut c_int,
    mut v5: *mut c_int,
) -> *mut c_int {
    let mut t: *mut c_int;
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
pub unsafe fn tr_pivot(
    ISAd: *const c_int,
    mut first: *mut c_int,
    mut last: *mut c_int,
) -> *mut c_int {
    let mut middle: *mut c_int;
    let mut t: c_int;

    t = last.offset_from(first) as c_int;
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
    first = tr_median3(ISAd, first, first.offset(t as isize), first.offset((t << 1) as isize));
    middle = tr_median3(ISAd, middle.offset(-(t as isize)), middle, middle.offset(t as isize));
    last = tr_median3(
        ISAd,
        last.offset(-1).offset(-((t << 1) as isize)),
        last.offset(-1).offset(-(t as isize)),
        last.offset(-1),
    );
    tr_median3(ISAd, first, middle, last)
}

/*---------------------------------------------------------------------------*/

#[derive(Clone, Copy)]
pub struct trbudget_t {
    pub chance: c_int,
    pub remain: c_int,
    pub incval: c_int,
    pub count: c_int,
}

pub unsafe fn trbudget_init(budget: *mut trbudget_t, chance: c_int, incval: c_int) {
    (*budget).chance = chance;
    (*budget).incval = incval;
    (*budget).remain = incval;
}

pub unsafe fn trbudget_check(budget: *mut trbudget_t, size: c_int) -> c_int {
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

pub unsafe fn tr_partition(
    ISAd: *const c_int,
    mut first: *mut c_int,
    middle: *mut c_int,
    mut last: *mut c_int,
    pa: *mut *mut c_int,
    pb: *mut *mut c_int,
    v: c_int,
) {
    let mut a: *mut c_int;
    let mut b: *mut c_int;
    let mut c: *mut c_int;
    let mut d: *mut c_int;
    let mut e: *mut c_int;
    let mut f: *mut c_int;
    let mut t: c_int;
    let mut s: c_int;
    let mut x: c_int = 0;

    b = middle.offset(-1);
    loop {
        b = b.offset(1);
        if !(b < last) {
            break;
        }
        x = *ISAd.offset(*b as isize);
        if x != v {
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
        if x != v {
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
        s = a.offset_from(first) as c_int;
        t = b.offset_from(a) as c_int;
        if s > t {
            s = t;
        }
        e = first;
        f = b.offset(-(s as isize));
        while 0 < s {
            let tt = *e;
            *e = *f;
            *f = tt;
            s -= 1;
            e = e.offset(1);
            f = f.offset(1);
        }
        s = d.offset_from(c) as c_int;
        t = (last.offset_from(d) as c_int) - 1;
        if s > t {
            s = t;
        }
        e = b;
        f = last.offset(-(s as isize));
        while 0 < s {
            let tt = *e;
            *e = *f;
            *f = tt;
            s -= 1;
            e = e.offset(1);
            f = f.offset(1);
        }
        first = first.offset(b.offset_from(a));
        last = last.offset(-(d.offset_from(c)));
    }
    *pa = first;
    *pb = last;
}

pub unsafe fn tr_copy(
    ISA: *mut c_int,
    SA: *const c_int,
    first: *mut c_int,
    a: *mut c_int,
    b: *mut c_int,
    last: *mut c_int,
    depth: c_int,
) {
    /* sort suffixes of middle partition
       by using sorted order of suffixes of left and right partition. */
    let mut c: *mut c_int;
    let mut d: *mut c_int;
    let mut e: *mut c_int;
    let mut s: c_int;
    let v: c_int;

    v = (b.offset_from(SA) as c_int) - 1;
    c = first;
    d = a.offset(-1);
    while c <= d {
        s = *c - depth;
        if (0 <= s) && (*ISA.offset(s as isize) == v) {
            d = d.offset(1);
            *d = s;
            *ISA.offset(s as isize) = d.offset_from(SA) as c_int;
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
            *ISA.offset(s as isize) = d.offset_from(SA) as c_int;
        }
        c = c.offset(-1);
    }
}

pub unsafe fn tr_partialcopy(
    ISA: *mut c_int,
    SA: *const c_int,
    first: *mut c_int,
    a: *mut c_int,
    b: *mut c_int,
    last: *mut c_int,
    depth: c_int,
) {
    let mut c: *mut c_int;
    let mut d: *mut c_int;
    let mut e: *mut c_int;
    let mut s: c_int;
    let v: c_int;
    let mut rank: c_int;
    let mut lastrank: c_int;
    let mut newrank: c_int = -1;

    v = (b.offset_from(SA) as c_int) - 1;
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
                newrank = d.offset_from(SA) as c_int;
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
            newrank = e.offset_from(SA) as c_int;
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
                newrank = d.offset_from(SA) as c_int;
            }
            *ISA.offset(s as isize) = newrank;
        }
        c = c.offset(-1);
    }
}

/*---------------------------------------------------------------------------*/

#[derive(Clone, Copy)]
pub struct TrStackEntry {
    pub a: *const c_int,
    pub b: *mut c_int,
    pub c: *mut c_int,
    pub d: c_int,
    pub e: c_int,
}

#[inline]
unsafe fn push5(
    stack: &mut [TrStackEntry; TR_STACKSIZE],
    ssize: &mut usize,
    a: *const c_int,
    b: *mut c_int,
    c: *mut c_int,
    d: c_int,
    e: c_int,
) {
    stack[*ssize].a = a;
    stack[*ssize].b = b;
    stack[*ssize].c = c;
    stack[*ssize].d = d;
    stack[*ssize].e = e;
    *ssize += 1;
}

pub unsafe fn tr_introsort(
    ISA: *mut c_int,
    mut ISAd: *const c_int,
    SA: *mut c_int,
    mut first: *mut c_int,
    mut last: *mut c_int,
    budget: *mut trbudget_t,
) {
    let mut stack: [TrStackEntry; TR_STACKSIZE] = [TrStackEntry {
        a: core::ptr::null(),
        b: core::ptr::null_mut(),
        c: core::ptr::null_mut(),
        d: 0,
        e: 0,
    }; TR_STACKSIZE];
    let mut ssize: usize = 0;

    let mut a: *mut c_int = core::ptr::null_mut();
    let mut b: *mut c_int = core::ptr::null_mut();
    let mut c: *mut c_int;
    let mut v: c_int;
    let mut x: c_int = 0;
    let incr: isize = ISAd.offset_from(ISA);
    let mut limit: c_int;
    let mut next: c_int;
    let mut trlink: c_int = -1;

    limit = tr_ilg(last.offset_from(first) as c_int);
    'outer: loop {
        if limit < 0 {
            if limit == -1 {
                /* tandem repeat partition */
                tr_partition(
                    ISAd.offset(-incr),
                    first,
                    first,
                    last,
                    &mut a,
                    &mut b,
                    (last.offset_from(SA) as c_int) - 1,
                );

                /* update ranks */
                if a < last {
                    c = first;
                    v = (a.offset_from(SA) as c_int) - 1;
                    while c < a {
                        *ISA.offset(*c as isize) = v;
                        c = c.offset(1);
                    }
                }
                if b < last {
                    c = a;
                    v = (b.offset_from(SA) as c_int) - 1;
                    while c < b {
                        *ISA.offset(*c as isize) = v;
                        c = c.offset(1);
                    }
                }

                /* push */
                if 1 < (b.offset_from(a) as c_int) {
                    // STACK_PUSH5(NULL, a, b, 0, 0)
                    stack[ssize].a = core::ptr::null();
                    stack[ssize].b = a;
                    stack[ssize].c = b;
                    stack[ssize].d = 0;
                    stack[ssize].e = 0;
                    ssize += 1;
                    // STACK_PUSH5(ISAd - incr, first, last, -2, trlink)
                    stack[ssize].a = ISAd.offset(-incr);
                    stack[ssize].b = first;
                    stack[ssize].c = last;
                    stack[ssize].d = -2;
                    stack[ssize].e = trlink;
                    ssize += 1;
                    trlink = (ssize as c_int) - 2;
                }
                if (a.offset_from(first) as c_int) <= (last.offset_from(b) as c_int) {
                    if 1 < (a.offset_from(first) as c_int) {
                        stack[ssize].a = ISAd;
                        stack[ssize].b = b;
                        stack[ssize].c = last;
                        stack[ssize].d = tr_ilg(last.offset_from(b) as c_int);
                        stack[ssize].e = trlink;
                        ssize += 1;
                        last = a;
                        limit = tr_ilg(a.offset_from(first) as c_int);
                    } else if 1 < (last.offset_from(b) as c_int) {
                        first = b;
                        limit = tr_ilg(last.offset_from(b) as c_int);
                    } else {
                        // STACK_POP5
                        if ssize == 0 {
                            return;
                        }
                        ssize -= 1;
                        ISAd = stack[ssize].a;
                        first = stack[ssize].b;
                        last = stack[ssize].c;
                        limit = stack[ssize].d;
                        trlink = stack[ssize].e;
                    }
                } else {
                    if 1 < (last.offset_from(b) as c_int) {
                        stack[ssize].a = ISAd;
                        stack[ssize].b = first;
                        stack[ssize].c = a;
                        stack[ssize].d = tr_ilg(a.offset_from(first) as c_int);
                        stack[ssize].e = trlink;
                        ssize += 1;
                        first = b;
                        limit = tr_ilg(last.offset_from(b) as c_int);
                    } else if 1 < (a.offset_from(first) as c_int) {
                        last = a;
                        limit = tr_ilg(a.offset_from(first) as c_int);
                    } else {
                        if ssize == 0 {
                            return;
                        }
                        ssize -= 1;
                        ISAd = stack[ssize].a;
                        first = stack[ssize].b;
                        last = stack[ssize].c;
                        limit = stack[ssize].d;
                        trlink = stack[ssize].e;
                    }
                }
            } else if limit == -2 {
                /* tandem repeat copy */
                ssize -= 1;
                a = stack[ssize].b;
                b = stack[ssize].c;
                if stack[ssize].d == 0 {
                    tr_copy(ISA, SA, first, a, b, last, ISAd.offset_from(ISA) as c_int);
                } else {
                    if 0 <= trlink {
                        stack[trlink as usize].d = -1;
                    }
                    tr_partialcopy(ISA, SA, first, a, b, last, ISAd.offset_from(ISA) as c_int);
                }
                // STACK_POP5
                if ssize == 0 {
                    return;
                }
                ssize -= 1;
                ISAd = stack[ssize].a;
                first = stack[ssize].b;
                last = stack[ssize].c;
                limit = stack[ssize].d;
                trlink = stack[ssize].e;
            } else {
                /* sorted partition */
                if 0 <= *first {
                    a = first;
                    loop {
                        *ISA.offset(*a as isize) = a.offset_from(SA) as c_int;
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
                        tr_ilg((a.offset_from(first) as c_int) + 1)
                    } else {
                        -1
                    };
                    a = a.offset(1);
                    if a < last {
                        b = first;
                        v = (a.offset_from(SA) as c_int) - 1;
                        while b < a {
                            *ISA.offset(*b as isize) = v;
                            b = b.offset(1);
                        }
                    }

                    /* push */
                    if trbudget_check(budget, a.offset_from(first) as c_int) != 0 {
                        if (a.offset_from(first) as c_int) <= (last.offset_from(a) as c_int) {
                            stack[ssize].a = ISAd;
                            stack[ssize].b = a;
                            stack[ssize].c = last;
                            stack[ssize].d = -3;
                            stack[ssize].e = trlink;
                            ssize += 1;
                            ISAd = ISAd.offset(incr);
                            last = a;
                            limit = next;
                        } else {
                            if 1 < (last.offset_from(a) as c_int) {
                                stack[ssize].a = ISAd.offset(incr);
                                stack[ssize].b = first;
                                stack[ssize].c = a;
                                stack[ssize].d = next;
                                stack[ssize].e = trlink;
                                ssize += 1;
                                first = a;
                                limit = -3;
                            } else {
                                ISAd = ISAd.offset(incr);
                                last = a;
                                limit = next;
                            }
                        }
                    } else {
                        if 0 <= trlink {
                            stack[trlink as usize].d = -1;
                        }
                        if 1 < (last.offset_from(a) as c_int) {
                            first = a;
                            limit = -3;
                        } else {
                            if ssize == 0 {
                                return;
                            }
                            ssize -= 1;
                            ISAd = stack[ssize].a;
                            first = stack[ssize].b;
                            last = stack[ssize].c;
                            limit = stack[ssize].d;
                            trlink = stack[ssize].e;
                        }
                    }
                } else {
                    if ssize == 0 {
                        return;
                    }
                    ssize -= 1;
                    ISAd = stack[ssize].a;
                    first = stack[ssize].b;
                    last = stack[ssize].c;
                    limit = stack[ssize].d;
                    trlink = stack[ssize].e;
                }
            }
            continue 'outer;
        }

        if (last.offset_from(first) as c_int) <= TR_INSERTIONSORT_THRESHOLD {
            tr_insertionsort(ISAd, first, last);
            limit = -3;
            continue 'outer;
        }

        let old_limit = limit;
        limit -= 1;
        if old_limit == 0 {
            tr_heapsort(ISAd, first, last.offset_from(first) as c_int);
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
            continue 'outer;
        }

        /* choose pivot */
        a = tr_pivot(ISAd, first, last);
        // SWAP(*first, *a)
        let tt = *first;
        *first = *a;
        *a = tt;
        v = *ISAd.offset(*first as isize);

        /* partition */
        tr_partition(ISAd, first, first.offset(1), last, &mut a, &mut b, v);
        if (last.offset_from(first) as c_int) != (b.offset_from(a) as c_int) {
            next = if *ISA.offset(*a as isize) != v {
                tr_ilg(b.offset_from(a) as c_int)
            } else {
                -1
            };

            /* update ranks */
            c = first;
            v = (a.offset_from(SA) as c_int) - 1;
            while c < a {
                *ISA.offset(*c as isize) = v;
                c = c.offset(1);
            }
            if b < last {
                c = a;
                v = (b.offset_from(SA) as c_int) - 1;
                while c < b {
                    *ISA.offset(*c as isize) = v;
                    c = c.offset(1);
                }
            }

            /* push */
            if (1 < (b.offset_from(a) as c_int))
                && (trbudget_check(budget, b.offset_from(a) as c_int) != 0)
            {
                if (a.offset_from(first) as c_int) <= (last.offset_from(b) as c_int) {
                    if (last.offset_from(b) as c_int) <= (b.offset_from(a) as c_int) {
                        if 1 < (a.offset_from(first) as c_int) {
                            push5(&mut stack, &mut ssize, ISAd.offset(incr), a, b, next, trlink);
                            push5(&mut stack, &mut ssize, ISAd, b, last, limit, trlink);
                            last = a;
                        } else if 1 < (last.offset_from(b) as c_int) {
                            push5(&mut stack, &mut ssize, ISAd.offset(incr), a, b, next, trlink);
                            first = b;
                        } else {
                            ISAd = ISAd.offset(incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if (a.offset_from(first) as c_int) <= (b.offset_from(a) as c_int) {
                        if 1 < (a.offset_from(first) as c_int) {
                            push5(&mut stack, &mut ssize, ISAd, b, last, limit, trlink);
                            push5(&mut stack, &mut ssize, ISAd.offset(incr), a, b, next, trlink);
                            last = a;
                        } else {
                            push5(&mut stack, &mut ssize, ISAd, b, last, limit, trlink);
                            ISAd = ISAd.offset(incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        push5(&mut stack, &mut ssize, ISAd, b, last, limit, trlink);
                        push5(&mut stack, &mut ssize, ISAd, first, a, limit, trlink);
                        ISAd = ISAd.offset(incr);
                        first = a;
                        last = b;
                        limit = next;
                    }
                } else {
                    if (a.offset_from(first) as c_int) <= (b.offset_from(a) as c_int) {
                        if 1 < (last.offset_from(b) as c_int) {
                            push5(&mut stack, &mut ssize, ISAd.offset(incr), a, b, next, trlink);
                            push5(&mut stack, &mut ssize, ISAd, first, a, limit, trlink);
                            first = b;
                        } else if 1 < (a.offset_from(first) as c_int) {
                            push5(&mut stack, &mut ssize, ISAd.offset(incr), a, b, next, trlink);
                            last = a;
                        } else {
                            ISAd = ISAd.offset(incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if (last.offset_from(b) as c_int) <= (b.offset_from(a) as c_int) {
                        if 1 < (last.offset_from(b) as c_int) {
                            push5(&mut stack, &mut ssize, ISAd, first, a, limit, trlink);
                            push5(&mut stack, &mut ssize, ISAd.offset(incr), a, b, next, trlink);
                            first = b;
                        } else {
                            push5(&mut stack, &mut ssize, ISAd, first, a, limit, trlink);
                            ISAd = ISAd.offset(incr);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        push5(&mut stack, &mut ssize, ISAd, first, a, limit, trlink);
                        push5(&mut stack, &mut ssize, ISAd, b, last, limit, trlink);
                        ISAd = ISAd.offset(incr);
                        first = a;
                        last = b;
                        limit = next;
                    }
                }
            } else {
                if (1 < (b.offset_from(a) as c_int)) && (0 <= trlink) {
                    stack[trlink as usize].d = -1;
                }
                if (a.offset_from(first) as c_int) <= (last.offset_from(b) as c_int) {
                    if 1 < (a.offset_from(first) as c_int) {
                        push5(&mut stack, &mut ssize, ISAd, b, last, limit, trlink);
                        last = a;
                    } else if 1 < (last.offset_from(b) as c_int) {
                        first = b;
                    } else {
                        if ssize == 0 {
                            return;
                        }
                        ssize -= 1;
                        ISAd = stack[ssize].a;
                        first = stack[ssize].b;
                        last = stack[ssize].c;
                        limit = stack[ssize].d;
                        trlink = stack[ssize].e;
                    }
                } else {
                    if 1 < (last.offset_from(b) as c_int) {
                        push5(&mut stack, &mut ssize, ISAd, first, a, limit, trlink);
                        first = b;
                    } else if 1 < (a.offset_from(first) as c_int) {
                        last = a;
                    } else {
                        if ssize == 0 {
                            return;
                        }
                        ssize -= 1;
                        ISAd = stack[ssize].a;
                        first = stack[ssize].b;
                        last = stack[ssize].c;
                        limit = stack[ssize].d;
                        trlink = stack[ssize].e;
                    }
                }
            }
        } else {
            if trbudget_check(budget, last.offset_from(first) as c_int) != 0 {
                limit = tr_ilg(last.offset_from(first) as c_int);
                ISAd = ISAd.offset(incr);
            } else {
                if 0 <= trlink {
                    stack[trlink as usize].d = -1;
                }
                if ssize == 0 {
                    return;
                }
                ssize -= 1;
                ISAd = stack[ssize].a;
                first = stack[ssize].b;
                last = stack[ssize].c;
                limit = stack[ssize].d;
                trlink = stack[ssize].e;
            }
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Tandem repeat sort */
pub unsafe fn trsort(ISA: *mut c_int, SA: *mut c_int, n: c_int, depth: c_int) {
    let mut ISAd: *mut c_int;
    let mut first: *mut c_int;
    let mut last: *mut c_int;
    let mut budget: trbudget_t = trbudget_t {
        chance: 0,
        remain: 0,
        incval: 0,
        count: 0,
    };
    let mut t: c_int;
    let mut skip: c_int;
    let mut unsorted: c_int;

    trbudget_init(&mut budget, tr_ilg(n) * 2 / 3, n);
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
                    *(first.offset(skip as isize)) = skip;
                    skip = 0;
                }
                last = SA.offset((*ISA.offset(t as isize) + 1) as isize);
                if 1 < (last.offset_from(first) as c_int) {
                    budget.count = 0;
                    tr_introsort(ISA, ISAd, SA, first, last, &mut budget);
                    if budget.count != 0 {
                        unsorted += budget.count;
                    } else {
                        skip = first.offset_from(last) as c_int;
                    }
                } else if (last.offset_from(first) as c_int) == 1 {
                    skip = -1;
                }
                first = last;
            }
            if !(first < SA.offset(n as isize)) {
                break;
            }
        }
        if skip != 0 {
            *(first.offset(skip as isize)) = skip;
        }
        if unsorted == 0 {
            break;
        }
        // ISAd += ISAd - ISA
        ISAd = ISAd.offset(ISAd.offset_from(ISA));
    }
}

/*---------------------------------------------------------------------------*/

/* Sorts suffixes of type B*. */
pub unsafe fn sort_typeBstar(
    T: *const c_uchar,
    SA: *mut c_int,
    bucket_A: *mut c_int,
    bucket_B: *mut c_int,
    n: c_int,
    openMP: c_int,
) -> c_int {
    let PAb: *mut c_int;
    let ISAb: *mut c_int;
    let buf: *mut c_int;
    let mut i: c_int;
    let mut j: c_int;
    let mut k: c_int;
    let mut t: c_int;
    let mut m: c_int;
    let mut bufsize: c_int;
    let mut c0: c_int;
    let mut c1: c_int;
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

    /* Count occurrences. */
    i = n - 1;
    m = n;
    c0 = *T.offset((n - 1) as isize) as c_int;
    while 0 <= i {
        /* type A suffix. */
        loop {
            c1 = c0;
            BUCKET_A!(bucket_A, c1) += 1;
            i -= 1;
            if !(0 <= i) {
                break;
            }
            c0 = *T.offset(i as isize) as c_int;
            if !(c0 >= c1) {
                break;
            }
        }
        if 0 <= i {
            /* type B* suffix. */
            BUCKET_BSTAR!(bucket_B, c0, c1) += 1;
            m -= 1;
            *SA.offset(m as isize) = i;
            /* type B suffix. */
            i -= 1;
            c1 = c0;
            while (0 <= i) && {
                c0 = *T.offset(i as isize) as c_int;
                c0 <= c1
            } {
                BUCKET_B!(bucket_B, c0, c1) += 1;
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
        t = i + BUCKET_A!(bucket_A, c0);
        BUCKET_A!(bucket_A, c0) = i + j; /* start point */
        i = t + BUCKET_B!(bucket_B, c0, c0);
        c1 = c0 + 1;
        while c1 < ALPHABET_SIZE {
            j += BUCKET_BSTAR!(bucket_B, c0, c1);
            BUCKET_BSTAR!(bucket_B, c0, c1) = j; /* end point */
            i += BUCKET_B!(bucket_B, c0, c1);
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
            c0 = *T.offset(t as isize) as c_int;
            c1 = *T.offset((t + 1) as isize) as c_int;
            BUCKET_BSTAR!(bucket_B, c0, c1) -= 1;
            *SA.offset(BUCKET_BSTAR!(bucket_B, c0, c1) as isize) = i;
            i -= 1;
        }
        t = *PAb.offset((m - 1) as isize);
        c0 = *T.offset(t as isize) as c_int;
        c1 = *T.offset((t + 1) as isize) as c_int;
        BUCKET_BSTAR!(bucket_B, c0, c1) -= 1;
        *SA.offset(BUCKET_BSTAR!(bucket_B, c0, c1) as isize) = m - 1;

        /* Sort the type B* substrings using sssort. (non-OpenMP) */
        buf = SA.offset(m as isize);
        bufsize = n - (2 * m);
        c0 = ALPHABET_SIZE - 2;
        j = m;
        while 0 < j {
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                i = BUCKET_BSTAR!(bucket_B, c0, c1);
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
                        (*SA.offset(i as isize) == (m - 1)) as c_int,
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
                *SA.offset(i as isize) = !*SA.offset(i as isize);
                *ISAb.offset(*SA.offset(i as isize) as isize) = j;
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
        c0 = *T.offset((n - 1) as isize) as c_int;
        while 0 <= i {
            i -= 1;
            c1 = c0;
            while (0 <= i) && {
                c0 = *T.offset(i as isize) as c_int;
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
                    c0 = *T.offset(i as isize) as c_int;
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
        BUCKET_B!(bucket_B, ALPHABET_SIZE - 1, ALPHABET_SIZE - 1) = n; /* end point */
        c0 = ALPHABET_SIZE - 2;
        k = m - 1;
        while 0 <= c0 {
            i = BUCKET_A!(bucket_A, c0 + 1) - 1;
            c1 = ALPHABET_SIZE - 1;
            while c0 < c1 {
                t = i - BUCKET_B!(bucket_B, c0, c1);
                BUCKET_B!(bucket_B, c0, c1) = i; /* end point */

                /* Move all type B* suffixes to the correct position. */
                i = t;
                j = BUCKET_BSTAR!(bucket_B, c0, c1);
                while j <= k {
                    *SA.offset(i as isize) = *SA.offset(k as isize);
                    i -= 1;
                    k -= 1;
                }
                c1 -= 1;
            }
            BUCKET_BSTAR!(bucket_B, c0, c0 + 1) = i - BUCKET_B!(bucket_B, c0, c0) + 1; /* start point */
            BUCKET_B!(bucket_B, c0, c0) = i; /* end point */
            c0 -= 1;
        }
    }

    m
}

/* Constructs the suffix array by using the sorted order of type B* suffixes. */
pub unsafe fn construct_SA(
    T: *const c_uchar,
    SA: *mut c_int,
    bucket_A: *mut c_int,
    bucket_B: *mut c_int,
    n: c_int,
    m: c_int,
) {
    let mut i: *mut c_int;
    let mut j: *mut c_int;
    let mut k: *mut c_int;
    let mut s: c_int;
    let mut c0: c_int;
    let mut c1: c_int;
    let mut c2: c_int;

    if 0 < m {
        /* Construct the sorted order of type B suffixes. */
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            i = SA.offset(BUCKET_BSTAR!(bucket_B, c1, c1 + 1) as isize);
            j = SA.offset((BUCKET_A!(bucket_A, c1 + 1) - 1) as isize);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    *j = !s;
                    s -= 1;
                    c0 = *T.offset(s as isize) as c_int;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as c_int) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            BUCKET_B!(bucket_B, c2, c1) = k.offset_from(SA) as c_int;
                        }
                        c2 = c0;
                        k = SA.offset(BUCKET_B!(bucket_B, c2, c1) as isize);
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

    /* Construct the suffix array by using the sorted order of type B suffixes. */
    c2 = *T.offset((n - 1) as isize) as c_int;
    k = SA.offset(BUCKET_A!(bucket_A, c2) as isize);
    *k = if (*T.offset((n - 2) as isize) as c_int) < c2 {
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
            c0 = *T.offset(s as isize) as c_int;
            if (s == 0) || ((*T.offset((s - 1) as isize) as c_int) < c0) {
                s = !s;
            }
            if c0 != c2 {
                BUCKET_A!(bucket_A, c2) = k.offset_from(SA) as c_int;
                c2 = c0;
                k = SA.offset(BUCKET_A!(bucket_A, c2) as isize);
            }
            *k = s;
            k = k.offset(1);
        } else {
            *i = !s;
        }
        i = i.offset(1);
    }
}

/* Constructs the burrows-wheeler transformed string directly. */
pub unsafe fn construct_BWT(
    T: *const c_uchar,
    SA: *mut c_int,
    bucket_A: *mut c_int,
    bucket_B: *mut c_int,
    n: c_int,
    m: c_int,
) -> c_int {
    let mut i: *mut c_int;
    let mut j: *mut c_int;
    let mut k: *mut c_int;
    let mut orig: *mut c_int;
    let mut s: c_int;
    let mut c0: c_int;
    let mut c1: c_int;
    let mut c2: c_int;

    if 0 < m {
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            i = SA.offset(BUCKET_BSTAR!(bucket_B, c1, c1 + 1) as isize);
            j = SA.offset((BUCKET_A!(bucket_A, c1 + 1) - 1) as isize);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    s -= 1;
                    c0 = *T.offset(s as isize) as c_int;
                    *j = !c0;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as c_int) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            BUCKET_B!(bucket_B, c2, c1) = k.offset_from(SA) as c_int;
                        }
                        c2 = c0;
                        k = SA.offset(BUCKET_B!(bucket_B, c2, c1) as isize);
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

    c2 = *T.offset((n - 1) as isize) as c_int;
    k = SA.offset(BUCKET_A!(bucket_A, c2) as isize);
    *k = if (*T.offset((n - 2) as isize) as c_int) < c2 {
        !(*T.offset((n - 2) as isize) as c_int)
    } else {
        n - 1
    };
    k = k.offset(1);
    i = SA;
    j = SA.offset(n as isize);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            s -= 1;
            c0 = *T.offset(s as isize) as c_int;
            *i = c0;
            if (0 < s) && ((*T.offset((s - 1) as isize) as c_int) < c0) {
                s = !(*T.offset((s - 1) as isize) as c_int);
            }
            if c0 != c2 {
                BUCKET_A!(bucket_A, c2) = k.offset_from(SA) as c_int;
                c2 = c0;
                k = SA.offset(BUCKET_A!(bucket_A, c2) as isize);
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

    orig.offset_from(SA) as c_int
}

/* Constructs the BWT with indexes. */
pub unsafe fn construct_BWT_indexes(
    T: *const c_uchar,
    SA: *mut c_int,
    bucket_A: *mut c_int,
    bucket_B: *mut c_int,
    n: c_int,
    m: c_int,
    num_indexes: *mut c_uchar,
    indexes: *mut c_int,
) -> c_int {
    let mut i: *mut c_int;
    let mut j: *mut c_int;
    let mut k: *mut c_int;
    let mut orig: *mut c_int;
    let mut s: c_int;
    let mut c0: c_int;
    let mut c1: c_int;
    let mut c2: c_int;

    let mut mod_: c_int = n / 8;
    {
        mod_ |= mod_ >> 1;
        mod_ |= mod_ >> 2;
        mod_ |= mod_ >> 4;
        mod_ |= mod_ >> 8;
        mod_ |= mod_ >> 16;
        mod_ >>= 1;

        *num_indexes = ((n - 1) / (mod_ + 1)) as c_uchar;
    }

    if 0 < m {
        c1 = ALPHABET_SIZE - 2;
        while 0 <= c1 {
            i = SA.offset(BUCKET_BSTAR!(bucket_B, c1, c1 + 1) as isize);
            j = SA.offset((BUCKET_A!(bucket_A, c1 + 1) - 1) as isize);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    if (s & mod_) == 0 {
                        *indexes.offset((s / (mod_ + 1) - 1) as isize) = j.offset_from(SA) as c_int;
                    }

                    s -= 1;
                    c0 = *T.offset(s as isize) as c_int;
                    *j = !c0;
                    if (0 < s) && ((*T.offset((s - 1) as isize) as c_int) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            BUCKET_B!(bucket_B, c2, c1) = k.offset_from(SA) as c_int;
                        }
                        c2 = c0;
                        k = SA.offset(BUCKET_B!(bucket_B, c2, c1) as isize);
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

    c2 = *T.offset((n - 1) as isize) as c_int;
    k = SA.offset(BUCKET_A!(bucket_A, c2) as isize);
    if (*T.offset((n - 2) as isize) as c_int) < c2 {
        if ((n - 1) & mod_) == 0 {
            *indexes.offset(((n - 1) / (mod_ + 1) - 1) as isize) = k.offset_from(SA) as c_int;
        }
        *k = !(*T.offset((n - 2) as isize) as c_int);
        k = k.offset(1);
    } else {
        *k = n - 1;
        k = k.offset(1);
    }

    i = SA;
    j = SA.offset(n as isize);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            if (s & mod_) == 0 {
                *indexes.offset((s / (mod_ + 1) - 1) as isize) = i.offset_from(SA) as c_int;
            }

            s -= 1;
            c0 = *T.offset(s as isize) as c_int;
            *i = c0;
            if c0 != c2 {
                BUCKET_A!(bucket_A, c2) = k.offset_from(SA) as c_int;
                c2 = c0;
                k = SA.offset(BUCKET_A!(bucket_A, c2) as isize);
            }
            if (0 < s) && ((*T.offset((s - 1) as isize) as c_int) < c0) {
                if (s & mod_) == 0 {
                    *indexes.offset((s / (mod_ + 1) - 1) as isize) = k.offset_from(SA) as c_int;
                }
                *k = !(*T.offset((s - 1) as isize) as c_int);
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

    orig.offset_from(SA) as c_int
}

/*---------------------------------------------------------------------------*/

/*- Function -*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divsufsort(
    T: *const c_uchar,
    SA: *mut c_int,
    n: c_int,
    openMP: c_int,
) -> c_int {
    let bucket_A: *mut c_int;
    let bucket_B: *mut c_int;
    let m: c_int;
    let mut err: c_int = 0;

    /* Check arguments. */
    if T.is_null() || SA.is_null() || (n < 0) {
        return -1;
    } else if n == 0 {
        return 0;
    } else if n == 1 {
        *SA.offset(0) = 0;
        return 0;
    } else if n == 2 {
        let mm = (*T.offset(0) < *T.offset(1)) as c_int;
        *SA.offset((mm ^ 1) as isize) = 0;
        *SA.offset(mm as isize) = 1;
        return 0;
    }

    bucket_A = malloc((BUCKET_A_SIZE as usize) * core::mem::size_of::<c_int>()) as *mut c_int;
    bucket_B = malloc((BUCKET_B_SIZE as usize) * core::mem::size_of::<c_int>()) as *mut c_int;

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
    T: *const c_uchar,
    U: *mut c_uchar,
    A: *mut c_int,
    n: c_int,
    num_indexes: *mut c_uchar,
    indexes: *mut c_int,
    openMP: c_int,
) -> c_int {
    let mut B: *mut c_int;
    let bucket_A: *mut c_int;
    let bucket_B: *mut c_int;
    let m: c_int;
    let mut pidx: c_int;
    let mut i: c_int;

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
        B = malloc(((n + 1) as usize) * core::mem::size_of::<c_int>()) as *mut c_int;
    }
    bucket_A = malloc((BUCKET_A_SIZE as usize) * core::mem::size_of::<c_int>()) as *mut c_int;
    bucket_B = malloc((BUCKET_B_SIZE as usize) * core::mem::size_of::<c_int>()) as *mut c_int;

    /* Burrows-Wheeler Transform. */
    if (!B.is_null()) && (!bucket_A.is_null()) && (!bucket_B.is_null()) {
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
            *U.offset((i + 1) as isize) = *B.offset(i as isize) as c_uchar;
            i += 1;
        }
        i += 1;
        while i < n {
            *U.offset(i as isize) = *B.offset(i as isize) as c_uchar;
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

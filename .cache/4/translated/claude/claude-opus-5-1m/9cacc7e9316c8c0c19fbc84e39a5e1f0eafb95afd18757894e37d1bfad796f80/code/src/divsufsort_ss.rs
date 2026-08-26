//! Substring sort section of dictBuilder/divsufsort.c (lines 250-906).
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use crate::divsufsort_common::*;

/*---------------------------------------------------------------------------*/

/* #if (SS_BLOCKSIZE != 1) && (SS_INSERTIONSORT_THRESHOLD != 1) -> included */

/* Insertionsort for small size groups */
pub unsafe fn ss_insertionsort(
    T: *const u8,
    PA: *const i32,
    first: *mut i32,
    last: *mut i32,
    depth: i32,
) {
    let mut i: *mut i32;
    let mut j: *mut i32;
    let mut t: i32;
    let mut r: i32 = 0;

    i = last.wrapping_offset(-2);
    while first <= i {
        t = *i;
        j = i.wrapping_offset(1);
        loop {
            r = ss_compare(
                T,
                PA.wrapping_offset(t as isize),
                PA.wrapping_offset(*j as isize),
                depth,
            );
            if !(0 < r) {
                break;
            }
            loop {
                *j.wrapping_offset(-1) = *j;
                j = j.wrapping_offset(1);
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
        *j.wrapping_offset(-1) = t;

        i = i.wrapping_offset(-1);
    }
}

/*---------------------------------------------------------------------------*/

/* #if (SS_BLOCKSIZE == 0) || (SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE) -> included */

pub unsafe fn ss_fixdown(Td: *const u8, PA: *const i32, SA: *mut i32, mut i: i32, size: i32) {
    let mut j: i32;
    let mut k: i32 = 0;
    let v: i32;
    let c: i32;
    let mut d: i32;
    let mut e: i32;

    v = *SA.wrapping_offset(i as isize);
    c = *Td.wrapping_offset(*PA.wrapping_offset(v as isize) as isize) as i32;
    loop {
        j = i.wrapping_mul(2).wrapping_add(1);
        if !(j < size) {
            break;
        }

        k = j;
        j = j.wrapping_add(1);
        d = *Td.wrapping_offset(
            *PA.wrapping_offset(*SA.wrapping_offset(k as isize) as isize) as isize,
        ) as i32;
        e = *Td.wrapping_offset(
            *PA.wrapping_offset(*SA.wrapping_offset(j as isize) as isize) as isize,
        ) as i32;
        if d < e {
            k = j;
            d = e;
        }
        if d <= c {
            break;
        }

        *SA.wrapping_offset(i as isize) = *SA.wrapping_offset(k as isize);
        i = k;
    }
    *SA.wrapping_offset(i as isize) = v;
}

/* Simple top-down heapsort. */
pub unsafe fn ss_heapsort(Td: *const u8, PA: *const i32, SA: *mut i32, size: i32) {
    let mut i: i32;
    let mut m: i32;
    let mut t: i32;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if (*Td.wrapping_offset(
            *PA.wrapping_offset(*SA.wrapping_offset((m / 2) as isize) as isize) as isize,
        ) as i32)
            < (*Td.wrapping_offset(
                *PA.wrapping_offset(*SA.wrapping_offset(m as isize) as isize) as isize,
            ) as i32)
        {
            t = *SA.wrapping_offset(m as isize);
            *SA.wrapping_offset(m as isize) = *SA.wrapping_offset((m / 2) as isize);
            *SA.wrapping_offset((m / 2) as isize) = t;
        }
    }

    i = m / 2 - 1;
    while 0 <= i {
        ss_fixdown(Td, PA, SA, i, m);
        i -= 1;
    }
    if (size % 2) == 0 {
        t = *SA.wrapping_offset(0);
        *SA.wrapping_offset(0) = *SA.wrapping_offset(m as isize);
        *SA.wrapping_offset(m as isize) = t;
        ss_fixdown(Td, PA, SA, 0, m);
    }
    i = m - 1;
    while 0 < i {
        t = *SA.wrapping_offset(0);
        *SA.wrapping_offset(0) = *SA.wrapping_offset(i as isize);
        ss_fixdown(Td, PA, SA, 0, i);
        *SA.wrapping_offset(i as isize) = t;
        i -= 1;
    }
}

/*---------------------------------------------------------------------------*/

/* Returns the median of three elements. */
pub unsafe fn ss_median3(
    Td: *const u8,
    PA: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    mut v3: *mut i32,
) -> *mut i32 {
    let mut t: *mut i32;
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v1 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v2 as isize) as isize) as i32)
    {
        t = v1;
        v1 = v2;
        v2 = t;
    }
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v2 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v3 as isize) as isize) as i32)
    {
        if (*Td.wrapping_offset(*PA.wrapping_offset(*v1 as isize) as isize) as i32)
            > (*Td.wrapping_offset(*PA.wrapping_offset(*v3 as isize) as isize) as i32)
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
    Td: *const u8,
    PA: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    mut v3: *mut i32,
    mut v4: *mut i32,
    mut v5: *mut i32,
) -> *mut i32 {
    let mut t: *mut i32;
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v2 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v3 as isize) as isize) as i32)
    {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v4 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v5 as isize) as isize) as i32)
    {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v2 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v4 as isize) as isize) as i32)
    {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v1 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v3 as isize) as isize) as i32)
    {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v1 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v4 as isize) as isize) as i32)
    {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if (*Td.wrapping_offset(*PA.wrapping_offset(*v3 as isize) as isize) as i32)
        > (*Td.wrapping_offset(*PA.wrapping_offset(*v4 as isize) as isize) as i32)
    {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
pub unsafe fn ss_pivot(
    Td: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    mut last: *mut i32,
) -> *mut i32 {
    let mut middle: *mut i32;
    let mut t: i32;

    t = last.offset_from(first) as i32;
    middle = first.wrapping_offset((t / 2) as isize);

    if t <= 512 {
        if t <= 32 {
            return ss_median3(Td, PA, first, middle, last.wrapping_offset(-1));
        } else {
            t >>= 2;
            return ss_median5(
                Td,
                PA,
                first,
                first.wrapping_offset(t as isize),
                middle,
                last.wrapping_offset(-1).wrapping_offset(-(t as isize)),
                last.wrapping_offset(-1),
            );
        }
    }
    t >>= 3;
    first = ss_median3(
        Td,
        PA,
        first,
        first.wrapping_offset(t as isize),
        first.wrapping_offset((t << 1) as isize),
    );
    middle = ss_median3(
        Td,
        PA,
        middle.wrapping_offset(-(t as isize)),
        middle,
        middle.wrapping_offset(t as isize),
    );
    last = ss_median3(
        Td,
        PA,
        last.wrapping_offset(-1).wrapping_offset(-((t << 1) as isize)),
        last.wrapping_offset(-1).wrapping_offset(-(t as isize)),
        last.wrapping_offset(-1),
    );
    ss_median3(Td, PA, first, middle, last)
}

/*---------------------------------------------------------------------------*/

/* Binary partition for substrings. */
pub unsafe fn ss_partition(
    PA: *const i32,
    first: *mut i32,
    last: *mut i32,
    depth: i32,
) -> *mut i32 {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;

    a = first.wrapping_offset(-1);
    b = last;
    loop {
        loop {
            a = a.wrapping_offset(1);
            if !(a < b) {
                break;
            }
            if !((*PA.wrapping_offset(*a as isize)).wrapping_add(depth)
                >= (*PA.wrapping_offset((*a).wrapping_add(1) as isize)).wrapping_add(1))
            {
                break;
            }
            *a = !*a;
        }
        loop {
            b = b.wrapping_offset(-1);
            if !(a < b) {
                break;
            }
            if !((*PA.wrapping_offset(*b as isize)).wrapping_add(depth)
                < (*PA.wrapping_offset((*b).wrapping_add(1) as isize)).wrapping_add(1))
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

#[derive(Clone, Copy)]
struct SsMintrosortStackEntry {
    a: *mut i32,
    b: *mut i32,
    c: i32,
    d: i32,
}

impl Default for SsMintrosortStackEntry {
    fn default() -> Self {
        SsMintrosortStackEntry {
            a: core::ptr::null_mut(),
            b: core::ptr::null_mut(),
            c: 0,
            d: 0,
        }
    }
}

/* Multikey introsort for medium size groups. */
pub unsafe fn ss_mintrosort(
    T: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    mut last: *mut i32,
    mut depth: i32,
) {
    /* STACK_SIZE == SS_MISORT_STACKSIZE */
    let mut stack: [SsMintrosortStackEntry; SS_MISORT_STACKSIZE] =
        [Default::default(); SS_MISORT_STACKSIZE];
    let mut Td: *const u8;
    let mut a: *mut i32 = core::ptr::null_mut();
    let mut b: *mut i32 = core::ptr::null_mut();
    let mut c: *mut i32 = core::ptr::null_mut();
    let mut d: *mut i32 = core::ptr::null_mut();
    let mut e: *mut i32 = core::ptr::null_mut();
    let mut f: *mut i32 = core::ptr::null_mut();
    let mut s: i32;
    let mut t: i32;
    let mut ssize: usize;
    let mut limit: i32;
    let mut v: i32;
    let mut x: i32 = 0;

    ssize = 0;
    limit = ss_ilg(last.offset_from(first) as i32);
    loop {
        if last.offset_from(first) <= SS_INSERTIONSORT_THRESHOLD as isize {
            /* #if 1 < SS_INSERTIONSORT_THRESHOLD -> included */
            if 1 < last.offset_from(first) {
                ss_insertionsort(T, PA, first, last, depth);
            }
            /* STACK_POP(first, last, depth, limit); */
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            last = stack[ssize].b;
            depth = stack[ssize].c;
            limit = stack[ssize].d;
            continue;
        }

        Td = T.wrapping_offset(depth as isize);
        {
            let old_limit = limit;
            limit = limit.wrapping_sub(1);
            if old_limit == 0 {
                ss_heapsort(Td, PA, first, last.offset_from(first) as i32);
            }
        }
        if limit < 0 {
            a = first.wrapping_offset(1);
            v = *Td.wrapping_offset(*PA.wrapping_offset(*first as isize) as isize) as i32;
            while a < last {
                x = *Td.wrapping_offset(*PA.wrapping_offset(*a as isize) as isize) as i32;
                if x != v {
                    if 1 < a.offset_from(first) {
                        break;
                    }
                    v = x;
                    first = a;
                }
                a = a.wrapping_offset(1);
            }
            if (*Td.wrapping_offset(
                (*PA.wrapping_offset(*first as isize)).wrapping_sub(1) as isize,
            ) as i32)
                < v
            {
                first = ss_partition(PA, first, a, depth);
            }
            if a.offset_from(first) <= last.offset_from(a) {
                if 1 < a.offset_from(first) {
                    /* STACK_PUSH(a, last, depth, -1); */
                    stack[ssize].a = a;
                    stack[ssize].b = last;
                    stack[ssize].c = depth;
                    stack[ssize].d = -1;
                    ssize += 1;
                    last = a;
                    depth += 1;
                    limit = ss_ilg(a.offset_from(first) as i32);
                } else {
                    first = a;
                    limit = -1;
                }
            } else {
                if 1 < last.offset_from(a) {
                    /* STACK_PUSH(first, a, depth + 1, ss_ilg(a - first)); */
                    stack[ssize].a = first;
                    stack[ssize].b = a;
                    stack[ssize].c = depth + 1;
                    stack[ssize].d = ss_ilg(a.offset_from(first) as i32);
                    ssize += 1;
                    first = a;
                    limit = -1;
                } else {
                    last = a;
                    depth += 1;
                    limit = ss_ilg(a.offset_from(first) as i32);
                }
            }
            continue;
        }

        /* choose pivot */
        a = ss_pivot(Td, PA, first, last);
        v = *Td.wrapping_offset(*PA.wrapping_offset(*a as isize) as isize) as i32;
        t = *first;
        *first = *a;
        *a = t;

        /* partition */
        b = first;
        loop {
            b = b.wrapping_offset(1);
            if !(b < last) {
                break;
            }
            x = *Td.wrapping_offset(*PA.wrapping_offset(*b as isize) as isize) as i32;
            if !(x == v) {
                break;
            }
        }
        a = b;
        if (a < last) && (x < v) {
            loop {
                b = b.wrapping_offset(1);
                if !(b < last) {
                    break;
                }
                x = *Td.wrapping_offset(*PA.wrapping_offset(*b as isize) as isize) as i32;
                if !(x <= v) {
                    break;
                }
                if x == v {
                    t = *b;
                    *b = *a;
                    *a = t;
                    a = a.wrapping_offset(1);
                }
            }
        }
        c = last;
        loop {
            c = c.wrapping_offset(-1);
            if !(b < c) {
                break;
            }
            x = *Td.wrapping_offset(*PA.wrapping_offset(*c as isize) as isize) as i32;
            if !(x == v) {
                break;
            }
        }
        d = c;
        if (b < d) && (x > v) {
            loop {
                c = c.wrapping_offset(-1);
                if !(b < c) {
                    break;
                }
                x = *Td.wrapping_offset(*PA.wrapping_offset(*c as isize) as isize) as i32;
                if !(x >= v) {
                    break;
                }
                if x == v {
                    t = *c;
                    *c = *d;
                    *d = t;
                    d = d.wrapping_offset(-1);
                }
            }
        }
        while b < c {
            t = *b;
            *b = *c;
            *c = t;
            loop {
                b = b.wrapping_offset(1);
                if !(b < c) {
                    break;
                }
                x = *Td.wrapping_offset(*PA.wrapping_offset(*b as isize) as isize) as i32;
                if !(x <= v) {
                    break;
                }
                if x == v {
                    t = *b;
                    *b = *a;
                    *a = t;
                    a = a.wrapping_offset(1);
                }
            }
            loop {
                c = c.wrapping_offset(-1);
                if !(b < c) {
                    break;
                }
                x = *Td.wrapping_offset(*PA.wrapping_offset(*c as isize) as isize) as i32;
                if !(x >= v) {
                    break;
                }
                if x == v {
                    t = *c;
                    *c = *d;
                    *d = t;
                    d = d.wrapping_offset(-1);
                }
            }
        }

        if a <= d {
            c = b.wrapping_offset(-1);

            s = a.offset_from(first) as i32;
            t = b.offset_from(a) as i32;
            if s > t {
                s = t;
            }
            e = first;
            f = b.wrapping_offset(-(s as isize));
            while 0 < s {
                t = *e;
                *e = *f;
                *f = t;
                s -= 1;
                e = e.wrapping_offset(1);
                f = f.wrapping_offset(1);
            }
            s = d.offset_from(c) as i32;
            t = last.offset_from(d).wrapping_sub(1) as i32;
            if s > t {
                s = t;
            }
            e = b;
            f = last.wrapping_offset(-(s as isize));
            while 0 < s {
                t = *e;
                *e = *f;
                *f = t;
                s -= 1;
                e = e.wrapping_offset(1);
                f = f.wrapping_offset(1);
            }

            a = first.wrapping_offset(b.offset_from(a));
            c = last.wrapping_offset(-(d.offset_from(c)));
            b = if v
                <= (*Td.wrapping_offset(
                    (*PA.wrapping_offset(*a as isize)).wrapping_sub(1) as isize,
                ) as i32)
            {
                a
            } else {
                ss_partition(PA, a, c, depth)
            };

            if a.offset_from(first) <= last.offset_from(c) {
                if last.offset_from(c) <= c.offset_from(b) {
                    /* STACK_PUSH(b, c, depth + 1, ss_ilg(c - b)); */
                    stack[ssize].a = b;
                    stack[ssize].b = c;
                    stack[ssize].c = depth + 1;
                    stack[ssize].d = ss_ilg(c.offset_from(b) as i32);
                    ssize += 1;
                    /* STACK_PUSH(c, last, depth, limit); */
                    stack[ssize].a = c;
                    stack[ssize].b = last;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    last = a;
                } else if a.offset_from(first) <= c.offset_from(b) {
                    /* STACK_PUSH(c, last, depth, limit); */
                    stack[ssize].a = c;
                    stack[ssize].b = last;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    /* STACK_PUSH(b, c, depth + 1, ss_ilg(c - b)); */
                    stack[ssize].a = b;
                    stack[ssize].b = c;
                    stack[ssize].c = depth + 1;
                    stack[ssize].d = ss_ilg(c.offset_from(b) as i32);
                    ssize += 1;
                    last = a;
                } else {
                    /* STACK_PUSH(c, last, depth, limit); */
                    stack[ssize].a = c;
                    stack[ssize].b = last;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    /* STACK_PUSH(first, a, depth, limit); */
                    stack[ssize].a = first;
                    stack[ssize].b = a;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(c.offset_from(b) as i32);
                }
            } else {
                if a.offset_from(first) <= c.offset_from(b) {
                    /* STACK_PUSH(b, c, depth + 1, ss_ilg(c - b)); */
                    stack[ssize].a = b;
                    stack[ssize].b = c;
                    stack[ssize].c = depth + 1;
                    stack[ssize].d = ss_ilg(c.offset_from(b) as i32);
                    ssize += 1;
                    /* STACK_PUSH(first, a, depth, limit); */
                    stack[ssize].a = first;
                    stack[ssize].b = a;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    first = c;
                } else if last.offset_from(c) <= c.offset_from(b) {
                    /* STACK_PUSH(first, a, depth, limit); */
                    stack[ssize].a = first;
                    stack[ssize].b = a;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    /* STACK_PUSH(b, c, depth + 1, ss_ilg(c - b)); */
                    stack[ssize].a = b;
                    stack[ssize].b = c;
                    stack[ssize].c = depth + 1;
                    stack[ssize].d = ss_ilg(c.offset_from(b) as i32);
                    ssize += 1;
                    first = c;
                } else {
                    /* STACK_PUSH(first, a, depth, limit); */
                    stack[ssize].a = first;
                    stack[ssize].b = a;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    /* STACK_PUSH(c, last, depth, limit); */
                    stack[ssize].a = c;
                    stack[ssize].b = last;
                    stack[ssize].c = depth;
                    stack[ssize].d = limit;
                    ssize += 1;
                    first = b;
                    last = c;
                    depth += 1;
                    limit = ss_ilg(c.offset_from(b) as i32);
                }
            }
        } else {
            limit += 1;
            if (*Td.wrapping_offset(
                (*PA.wrapping_offset(*first as isize)).wrapping_sub(1) as isize,
            ) as i32)
                < v
            {
                first = ss_partition(PA, first, last, depth);
                limit = ss_ilg(last.offset_from(first) as i32);
            }
            depth += 1;
        }
    }
}

/*---------------------------------------------------------------------------*/

/* #if SS_BLOCKSIZE != 0 -> included */

pub unsafe fn ss_blockswap(mut a: *mut i32, mut b: *mut i32, mut n: i32) {
    let mut t: i32;
    while 0 < n {
        t = *a;
        *a = *b;
        *b = t;

        n -= 1;
        a = a.wrapping_offset(1);
        b = b.wrapping_offset(1);
    }
}

pub unsafe fn ss_rotate(mut first: *mut i32, middle: *mut i32, mut last: *mut i32) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;
    let mut l: i32;
    let mut r: i32;

    l = middle.offset_from(first) as i32;
    r = last.offset_from(middle) as i32;
    while (0 < l) && (0 < r) {
        if l == r {
            ss_blockswap(first, middle, l);
            break;
        }
        if l < r {
            a = last.wrapping_offset(-1);
            b = middle.wrapping_offset(-1);
            t = *a;
            loop {
                *a = *b;
                a = a.wrapping_offset(-1);
                *b = *a;
                b = b.wrapping_offset(-1);
                if b < first {
                    *a = t;
                    last = a;
                    r = r.wrapping_sub(l.wrapping_add(1));
                    if r <= l {
                        break;
                    }
                    a = a.wrapping_offset(-1);
                    b = middle.wrapping_offset(-1);
                    t = *a;
                }
            }
        } else {
            a = first;
            b = middle;
            t = *a;
            loop {
                *a = *b;
                a = a.wrapping_offset(1);
                *b = *a;
                b = b.wrapping_offset(1);
                if last <= b {
                    *a = t;
                    first = a.wrapping_offset(1);
                    l = l.wrapping_sub(r.wrapping_add(1));
                    if l <= r {
                        break;
                    }
                    a = a.wrapping_offset(1);
                    b = middle;
                    t = *a;
                }
            }
        }
    }
}

/*---------------------------------------------------------------------------*/

pub unsafe fn ss_inplacemerge(
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
        if *last.wrapping_offset(-1) < 0 {
            x = 1;
            p = PA.wrapping_offset(!*last.wrapping_offset(-1) as isize);
        } else {
            x = 0;
            p = PA.wrapping_offset(*last.wrapping_offset(-1) as isize);
        }
        a = first;
        len = middle.offset_from(first) as i32;
        half = len >> 1;
        r = -1;
        while 0 < len {
            b = a.wrapping_offset(half as isize);
            q = ss_compare(
                T,
                PA.wrapping_offset(if 0 <= *b { *b } else { !*b } as isize),
                p,
                depth,
            );
            if q < 0 {
                a = b.wrapping_offset(1);
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
            last = last.wrapping_offset(-(middle.offset_from(a)));
            middle = a;
            if first == middle {
                break;
            }
        }
        last = last.wrapping_offset(-1);
        if x != 0 {
            loop {
                last = last.wrapping_offset(-1);
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
    let mut t: i32;
    let mut r: i32;

    bufend = buf
        .wrapping_offset(middle.offset_from(first))
        .wrapping_offset(-1);
    ss_blockswap(buf, first, middle.offset_from(first) as i32);

    a = first;
    t = *a;
    b = buf;
    c = middle;
    loop {
        r = ss_compare(
            T,
            PA.wrapping_offset(*b as isize),
            PA.wrapping_offset(*c as isize),
            depth,
        );
        if r < 0 {
            loop {
                *a = *b;
                a = a.wrapping_offset(1);
                if bufend <= b {
                    *bufend = t;
                    return;
                }
                *b = *a;
                b = b.wrapping_offset(1);
                if !(*b < 0) {
                    break;
                }
            }
        } else if r > 0 {
            loop {
                *a = *c;
                a = a.wrapping_offset(1);
                *c = *a;
                c = c.wrapping_offset(1);
                if last <= c {
                    while b < bufend {
                        *a = *b;
                        a = a.wrapping_offset(1);
                        *b = *a;
                        b = b.wrapping_offset(1);
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
                a = a.wrapping_offset(1);
                if bufend <= b {
                    *bufend = t;
                    return;
                }
                *b = *a;
                b = b.wrapping_offset(1);
                if !(*b < 0) {
                    break;
                }
            }

            loop {
                *a = *c;
                a = a.wrapping_offset(1);
                *c = *a;
                c = c.wrapping_offset(1);
                if last <= c {
                    while b < bufend {
                        *a = *b;
                        a = a.wrapping_offset(1);
                        *b = *a;
                        b = b.wrapping_offset(1);
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
    let mut t: i32;
    let mut r: i32;
    let mut x: i32;

    bufend = buf
        .wrapping_offset(last.offset_from(middle))
        .wrapping_offset(-1);
    ss_blockswap(buf, middle, last.offset_from(middle) as i32);

    x = 0;
    if *bufend < 0 {
        p1 = PA.wrapping_offset(!*bufend as isize);
        x |= 1;
    } else {
        p1 = PA.wrapping_offset(*bufend as isize);
    }
    if *middle.wrapping_offset(-1) < 0 {
        p2 = PA.wrapping_offset(!*middle.wrapping_offset(-1) as isize);
        x |= 2;
    } else {
        p2 = PA.wrapping_offset(*middle.wrapping_offset(-1) as isize);
    }
    a = last.wrapping_offset(-1);
    t = *a;
    b = bufend;
    c = middle.wrapping_offset(-1);
    loop {
        r = ss_compare(T, p1, p2, depth);
        if 0 < r {
            if (x & 1) != 0 {
                loop {
                    *a = *b;
                    a = a.wrapping_offset(-1);
                    *b = *a;
                    b = b.wrapping_offset(-1);
                    if !(*b < 0) {
                        break;
                    }
                }
                x ^= 1;
            }
            *a = *b;
            a = a.wrapping_offset(-1);
            if b <= buf {
                *buf = t;
                break;
            }
            *b = *a;
            b = b.wrapping_offset(-1);
            if *b < 0 {
                p1 = PA.wrapping_offset(!*b as isize);
                x |= 1;
            } else {
                p1 = PA.wrapping_offset(*b as isize);
            }
        } else if r < 0 {
            if (x & 2) != 0 {
                loop {
                    *a = *c;
                    a = a.wrapping_offset(-1);
                    *c = *a;
                    c = c.wrapping_offset(-1);
                    if !(*c < 0) {
                        break;
                    }
                }
                x ^= 2;
            }
            *a = *c;
            a = a.wrapping_offset(-1);
            *c = *a;
            c = c.wrapping_offset(-1);
            if c < first {
                while buf < b {
                    *a = *b;
                    a = a.wrapping_offset(-1);
                    *b = *a;
                    b = b.wrapping_offset(-1);
                }
                *a = *b;
                *b = t;
                break;
            }
            if *c < 0 {
                p2 = PA.wrapping_offset(!*c as isize);
                x |= 2;
            } else {
                p2 = PA.wrapping_offset(*c as isize);
            }
        } else {
            if (x & 1) != 0 {
                loop {
                    *a = *b;
                    a = a.wrapping_offset(-1);
                    *b = *a;
                    b = b.wrapping_offset(-1);
                    if !(*b < 0) {
                        break;
                    }
                }
                x ^= 1;
            }
            *a = !*b;
            a = a.wrapping_offset(-1);
            if b <= buf {
                *buf = t;
                break;
            }
            *b = *a;
            b = b.wrapping_offset(-1);
            if (x & 2) != 0 {
                loop {
                    *a = *c;
                    a = a.wrapping_offset(-1);
                    *c = *a;
                    c = c.wrapping_offset(-1);
                    if !(*c < 0) {
                        break;
                    }
                }
                x ^= 2;
            }
            *a = *c;
            a = a.wrapping_offset(-1);
            *c = *a;
            c = c.wrapping_offset(-1);
            if c < first {
                while buf < b {
                    *a = *b;
                    a = a.wrapping_offset(-1);
                    *b = *a;
                    b = b.wrapping_offset(-1);
                }
                *a = *b;
                *b = t;
                break;
            }
            if *b < 0 {
                p1 = PA.wrapping_offset(!*b as isize);
                x |= 1;
            } else {
                p1 = PA.wrapping_offset(*b as isize);
            }
            if *c < 0 {
                p2 = PA.wrapping_offset(!*c as isize);
                x |= 2;
            } else {
                p2 = PA.wrapping_offset(*c as isize);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct SsSwapmergeStackEntry {
    a: *mut i32,
    b: *mut i32,
    c: *mut i32,
    d: i32,
}

impl Default for SsSwapmergeStackEntry {
    fn default() -> Self {
        SsSwapmergeStackEntry {
            a: core::ptr::null_mut(),
            b: core::ptr::null_mut(),
            c: core::ptr::null_mut(),
            d: 0,
        }
    }
}

/* D&C based merge. */
pub unsafe fn ss_swapmerge(
    T: *const u8,
    PA: *const i32,
    mut first: *mut i32,
    mut middle: *mut i32,
    mut last: *mut i32,
    buf: *mut i32,
    bufsize: i32,
    depth: i32,
) {
    /* STACK_SIZE == SS_SMERGE_STACKSIZE */
    let mut stack: [SsSwapmergeStackEntry; SS_SMERGE_STACKSIZE] =
        [Default::default(); SS_SMERGE_STACKSIZE];
    let mut l: *mut i32;
    let mut r: *mut i32;
    let mut lm: *mut i32;
    let mut rm: *mut i32;
    let mut m: i32;
    let mut len: i32;
    let mut half: i32;
    let mut ssize: usize;
    let mut check: i32;
    let mut next: i32;

    check = 0;
    ssize = 0;
    loop {
        if last.offset_from(middle) <= bufsize as isize {
            if (first < middle) && (middle < last) {
                ss_mergebackward(T, PA, first, middle, last, buf, depth);
            }
            /* MERGE_CHECK(first, last, check); */
            if ((check & 1) != 0)
                || (((check & 2) != 0)
                    && (ss_compare(
                        T,
                        PA.wrapping_offset({
                            let g = *first.wrapping_offset(-1);
                            if 0 <= g {
                                g
                            } else {
                                !g
                            }
                        } as isize),
                        PA.wrapping_offset(*first as isize),
                        depth,
                    ) == 0))
            {
                *first = !*first;
            }
            if ((check & 4) != 0)
                && (ss_compare(
                    T,
                    PA.wrapping_offset({
                        let g = *last.wrapping_offset(-1);
                        if 0 <= g {
                            g
                        } else {
                            !g
                        }
                    } as isize),
                    PA.wrapping_offset(*last as isize),
                    depth,
                ) == 0)
            {
                *last = !*last;
            }
            /* STACK_POP(first, middle, last, check); */
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            middle = stack[ssize].b;
            last = stack[ssize].c;
            check = stack[ssize].d;
            continue;
        }

        if middle.offset_from(first) <= bufsize as isize {
            if first < middle {
                ss_mergeforward(T, PA, first, middle, last, buf, depth);
            }
            /* MERGE_CHECK(first, last, check); */
            if ((check & 1) != 0)
                || (((check & 2) != 0)
                    && (ss_compare(
                        T,
                        PA.wrapping_offset({
                            let g = *first.wrapping_offset(-1);
                            if 0 <= g {
                                g
                            } else {
                                !g
                            }
                        } as isize),
                        PA.wrapping_offset(*first as isize),
                        depth,
                    ) == 0))
            {
                *first = !*first;
            }
            if ((check & 4) != 0)
                && (ss_compare(
                    T,
                    PA.wrapping_offset({
                        let g = *last.wrapping_offset(-1);
                        if 0 <= g {
                            g
                        } else {
                            !g
                        }
                    } as isize),
                    PA.wrapping_offset(*last as isize),
                    depth,
                ) == 0)
            {
                *last = !*last;
            }
            /* STACK_POP(first, middle, last, check); */
            if ssize == 0 {
                return;
            }
            ssize -= 1;
            first = stack[ssize].a;
            middle = stack[ssize].b;
            last = stack[ssize].c;
            check = stack[ssize].d;
            continue;
        }

        m = 0;
        len = {
            let d1 = middle.offset_from(first);
            let d2 = last.offset_from(middle);
            if d1 < d2 {
                d1
            } else {
                d2
            }
        } as i32;
        half = len >> 1;
        while 0 < len {
            if ss_compare(
                T,
                PA.wrapping_offset({
                    let g = *middle.wrapping_offset(m.wrapping_add(half) as isize);
                    if 0 <= g {
                        g
                    } else {
                        !g
                    }
                } as isize),
                PA.wrapping_offset({
                    let g = *middle
                        .wrapping_offset(-(m.wrapping_add(half).wrapping_add(1) as isize));
                    if 0 <= g {
                        g
                    } else {
                        !g
                    }
                } as isize),
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
            lm = middle.wrapping_offset(-(m as isize));
            rm = middle.wrapping_offset(m as isize);
            ss_blockswap(lm, middle, m);
            l = middle;
            r = middle;
            next = 0;
            if rm < last {
                if *rm < 0 {
                    *rm = !*rm;
                    if first < lm {
                        loop {
                            l = l.wrapping_offset(-1);
                            if !(*l < 0) {
                                break;
                            }
                        }
                        next |= 4;
                    }
                    next |= 1;
                } else if first < lm {
                    while *r < 0 {
                        r = r.wrapping_offset(1);
                    }
                    next |= 2;
                }
            }

            if l.offset_from(first) <= last.offset_from(r) {
                /* STACK_PUSH(r, rm, last, (next & 3) | (check & 4)); */
                stack[ssize].a = r;
                stack[ssize].b = rm;
                stack[ssize].c = last;
                stack[ssize].d = (next & 3) | (check & 4);
                ssize += 1;
                middle = lm;
                last = l;
                check = (check & 3) | (next & 4);
            } else {
                if ((next & 2) != 0) && (r == middle) {
                    next ^= 6;
                }
                /* STACK_PUSH(first, lm, l, (check & 3) | (next & 4)); */
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
                PA.wrapping_offset({
                    let g = *middle.wrapping_offset(-1);
                    if 0 <= g {
                        g
                    } else {
                        !g
                    }
                } as isize),
                PA.wrapping_offset(*middle as isize),
                depth,
            ) == 0
            {
                *middle = !*middle;
            }
            /* MERGE_CHECK(first, last, check); */
            if ((check & 1) != 0)
                || (((check & 2) != 0)
                    && (ss_compare(
                        T,
                        PA.wrapping_offset({
                            let g = *first.wrapping_offset(-1);
                            if 0 <= g {
                                g
                            } else {
                                !g
                            }
                        } as isize),
                        PA.wrapping_offset(*first as isize),
                        depth,
                    ) == 0))
            {
                *first = !*first;
            }
            if ((check & 4) != 0)
                && (ss_compare(
                    T,
                    PA.wrapping_offset({
                        let g = *last.wrapping_offset(-1);
                        if 0 <= g {
                            g
                        } else {
                            !g
                        }
                    } as isize),
                    PA.wrapping_offset(*last as isize),
                    depth,
                ) == 0)
            {
                *last = !*last;
            }
            /* STACK_POP(first, middle, last, check); */
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
    let mut a: *mut i32 = core::ptr::null_mut();
    /* #if SS_BLOCKSIZE != 0 -> included */
    let mut b: *mut i32;
    let mut middle: *mut i32;
    let mut curbuf: *mut i32;
    let mut j: i32;
    let mut k: i32;
    let mut curbufsize: i32;
    let mut limit: i32 = 0;
    let mut i: i32;

    if lastsuffix != 0 {
        first = first.wrapping_offset(1);
    }

    if (bufsize < SS_BLOCKSIZE)
        && ((bufsize as isize) < last.offset_from(first))
        && ({
            limit = ss_isqrt(last.offset_from(first) as i32);
            bufsize < limit
        })
    {
        if SS_BLOCKSIZE < limit {
            limit = SS_BLOCKSIZE;
        }
        middle = last.wrapping_offset(-(limit as isize));
        buf = middle;
        bufsize = limit;
    } else {
        middle = last;
        limit = 0;
    }
    a = first;
    i = 0;
    while (SS_BLOCKSIZE as isize) < middle.offset_from(a) {
        /* #if SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE -> included */
        ss_mintrosort(T, PA, a, a.wrapping_offset(SS_BLOCKSIZE as isize), depth);
        curbufsize = last.offset_from(a.wrapping_offset(SS_BLOCKSIZE as isize)) as i32;
        curbuf = a.wrapping_offset(SS_BLOCKSIZE as isize);
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
                b.wrapping_offset(-(k as isize)),
                b,
                b.wrapping_offset(k as isize),
                curbuf,
                curbufsize,
                depth,
            );

            b = b.wrapping_offset(-(k as isize));
            k <<= 1;
            j >>= 1;
        }

        a = a.wrapping_offset(SS_BLOCKSIZE as isize);
        i += 1;
    }
    /* #if SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE -> included */
    ss_mintrosort(T, PA, a, middle, depth);
    k = SS_BLOCKSIZE;
    while i != 0 {
        if (i & 1) != 0 {
            ss_swapmerge(
                T,
                PA,
                a.wrapping_offset(-(k as isize)),
                a,
                middle,
                buf,
                bufsize,
                depth,
            );
            a = a.wrapping_offset(-(k as isize));
        }

        k <<= 1;
        i >>= 1;
    }
    if limit != 0 {
        /* #if SS_INSERTIONSORT_THRESHOLD < SS_BLOCKSIZE -> included */
        ss_mintrosort(T, PA, middle, last, depth);
        ss_inplacemerge(T, PA, first, middle, last, depth);
    }

    if lastsuffix != 0 {
        /* Insert last type B* suffix. */
        let mut PAi: [i32; 2] = [0; 2];
        PAi[0] = *PA.wrapping_offset(*first.wrapping_offset(-1) as isize);
        PAi[1] = n.wrapping_sub(2);
        a = first;
        i = *first.wrapping_offset(-1);
        while (a < last)
            && ((*a < 0)
                || (0 < ss_compare(
                    T,
                    &PAi[0] as *const i32,
                    PA.wrapping_offset(*a as isize),
                    depth,
                )))
        {
            *a.wrapping_offset(-1) = *a;
            a = a.wrapping_offset(1);
        }
        *a.wrapping_offset(-1) = i;
    }
}

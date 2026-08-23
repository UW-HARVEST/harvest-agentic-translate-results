//! dictBuilder/divsufsort.c -- "tandem repeat sort" section (C lines 922..1440).
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

/* C pointer subtraction on `int *`, truncated to `int` as the C code does. */
#[inline(always)]
unsafe fn ptr_diff(a: *const i32, b: *const i32) -> i32 {
    a.offset_from(b) as i32
}

/*---------------------------------------------------------------------------*/

/* Simple insertionsort for small size groups. */
pub unsafe fn tr_insertionsort(ISAd: *const i32, first: *mut i32, last: *mut i32) {
    let mut a: *mut i32;
    let mut b: *mut i32;
    let mut t: i32;
    let mut r: i32 = 0;

    a = first.wrapping_offset(1);
    while a < last {
        t = *a;
        b = a.wrapping_offset(-1);
        loop {
            r = (*ISAd.wrapping_offset(t as isize))
                .wrapping_sub(*ISAd.wrapping_offset(*b as isize));
            if !(0 > r) {
                break;
            }
            loop {
                *b.wrapping_offset(1) = *b;
                b = b.wrapping_offset(-1);
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
        *b.wrapping_offset(1) = t;
        a = a.wrapping_offset(1);
    }
}

/*---------------------------------------------------------------------------*/

pub unsafe fn tr_fixdown(ISAd: *const i32, SA: *mut i32, mut i: i32, size: i32) {
    let mut j: i32;
    let mut k: i32 = 0;
    let v: i32;
    let c: i32;
    let mut d: i32;
    let mut e: i32;

    v = *SA.wrapping_offset(i as isize);
    c = *ISAd.wrapping_offset(v as isize);
    loop {
        j = (2i32).wrapping_mul(i).wrapping_add(1);
        if !(j < size) {
            break;
        }
        k = j;
        j = j.wrapping_add(1);
        d = *ISAd.wrapping_offset(*SA.wrapping_offset(k as isize) as isize);
        e = *ISAd.wrapping_offset(*SA.wrapping_offset(j as isize) as isize);
        if d < e {
            k = j;
            d = e;
        }
        if d <= c {
            break;
        }
        /* loop increment: SA[i] = SA[k], i = k */
        *SA.wrapping_offset(i as isize) = *SA.wrapping_offset(k as isize);
        i = k;
    }
    *SA.wrapping_offset(i as isize) = v;
}

/* Simple top-down heapsort. */
pub unsafe fn tr_heapsort(ISAd: *const i32, SA: *mut i32, size: i32) {
    let mut i: i32;
    let mut m: i32;
    let mut t: i32;

    m = size;
    if (size % 2) == 0 {
        m -= 1;
        if *ISAd.wrapping_offset(*SA.wrapping_offset((m / 2) as isize) as isize)
            < *ISAd.wrapping_offset(*SA.wrapping_offset(m as isize) as isize)
        {
            t = *SA.wrapping_offset(m as isize);
            *SA.wrapping_offset(m as isize) = *SA.wrapping_offset((m / 2) as isize);
            *SA.wrapping_offset((m / 2) as isize) = t;
        }
    }

    i = m / 2 - 1;
    while 0 <= i {
        tr_fixdown(ISAd, SA, i, m);
        i -= 1;
    }
    if (size % 2) == 0 {
        t = *SA.wrapping_offset(0);
        *SA.wrapping_offset(0) = *SA.wrapping_offset(m as isize);
        *SA.wrapping_offset(m as isize) = t;
        tr_fixdown(ISAd, SA, 0, m);
    }
    i = m - 1;
    while 0 < i {
        t = *SA.wrapping_offset(0);
        *SA.wrapping_offset(0) = *SA.wrapping_offset(i as isize);
        tr_fixdown(ISAd, SA, 0, i);
        *SA.wrapping_offset(i as isize) = t;
        i -= 1;
    }
}

/*---------------------------------------------------------------------------*/

/* Returns the median of three elements. */
pub unsafe fn tr_median3(
    ISAd: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    mut v3: *mut i32,
) -> *mut i32 {
    let mut t: *mut i32;
    if *ISAd.wrapping_offset(*v1 as isize) > *ISAd.wrapping_offset(*v2 as isize) {
        t = v1;
        v1 = v2;
        v2 = t;
    }
    if *ISAd.wrapping_offset(*v2 as isize) > *ISAd.wrapping_offset(*v3 as isize) {
        if *ISAd.wrapping_offset(*v1 as isize) > *ISAd.wrapping_offset(*v3 as isize) {
            return v1;
        } else {
            return v3;
        }
    }
    v2
}

/* Returns the median of five elements. */
pub unsafe fn tr_median5(
    ISAd: *const i32,
    mut v1: *mut i32,
    mut v2: *mut i32,
    mut v3: *mut i32,
    mut v4: *mut i32,
    mut v5: *mut i32,
) -> *mut i32 {
    let mut t: *mut i32;
    if *ISAd.wrapping_offset(*v2 as isize) > *ISAd.wrapping_offset(*v3 as isize) {
        t = v2;
        v2 = v3;
        v3 = t;
    }
    if *ISAd.wrapping_offset(*v4 as isize) > *ISAd.wrapping_offset(*v5 as isize) {
        t = v4;
        v4 = v5;
        v5 = t;
    }
    if *ISAd.wrapping_offset(*v2 as isize) > *ISAd.wrapping_offset(*v4 as isize) {
        t = v2;
        v2 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if *ISAd.wrapping_offset(*v1 as isize) > *ISAd.wrapping_offset(*v3 as isize) {
        t = v1;
        v1 = v3;
        v3 = t;
    }
    if *ISAd.wrapping_offset(*v1 as isize) > *ISAd.wrapping_offset(*v4 as isize) {
        t = v1;
        v1 = v4;
        v4 = t;
        t = v3;
        v3 = v5;
        v5 = t;
    }
    if *ISAd.wrapping_offset(*v3 as isize) > *ISAd.wrapping_offset(*v4 as isize) {
        return v4;
    }
    v3
}

/* Returns the pivot element. */
pub unsafe fn tr_pivot(ISAd: *const i32, mut first: *mut i32, mut last: *mut i32) -> *mut i32 {
    let mut middle: *mut i32;
    let mut t: i32;

    t = ptr_diff(last, first);
    middle = first.wrapping_offset((t / 2) as isize);

    if t <= 512 {
        if t <= 32 {
            return tr_median3(ISAd, first, middle, last.wrapping_offset(-1));
        } else {
            t >>= 2;
            return tr_median5(
                ISAd,
                first,
                first.wrapping_offset(t as isize),
                middle,
                last.wrapping_offset(-1 - t as isize),
                last.wrapping_offset(-1),
            );
        }
    }
    t >>= 3;
    first = tr_median3(
        ISAd,
        first,
        first.wrapping_offset(t as isize),
        first.wrapping_offset((t << 1) as isize),
    );
    middle = tr_median3(
        ISAd,
        middle.wrapping_offset(-(t as isize)),
        middle,
        middle.wrapping_offset(t as isize),
    );
    last = tr_median3(
        ISAd,
        last.wrapping_offset(-1 - ((t << 1) as isize)),
        last.wrapping_offset(-1 - t as isize),
        last.wrapping_offset(-1),
    );
    tr_median3(ISAd, first, middle, last)
}

/*---------------------------------------------------------------------------*/

pub unsafe fn tr_partition(
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

    b = middle.wrapping_offset(-1);
    loop {
        b = b.wrapping_offset(1);
        if !(b < last) {
            break;
        }
        x = *ISAd.wrapping_offset(*b as isize);
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
            x = *ISAd.wrapping_offset(*b as isize);
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
        x = *ISAd.wrapping_offset(*c as isize);
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
            x = *ISAd.wrapping_offset(*c as isize);
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
            x = *ISAd.wrapping_offset(*b as isize);
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
            x = *ISAd.wrapping_offset(*c as isize);
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
        s = ptr_diff(a, first);
        t = ptr_diff(b, a);
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
        s = ptr_diff(d, c);
        t = ptr_diff(last, d).wrapping_sub(1);
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
        first = first.wrapping_offset(ptr_diff(b, a) as isize);
        last = last.wrapping_offset(-(ptr_diff(d, c) as isize));
    }
    *pa = first;
    *pb = last;
}

pub unsafe fn tr_copy(
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

    v = ptr_diff(b, SA).wrapping_sub(1);
    c = first;
    d = a.wrapping_offset(-1);
    while c <= d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.wrapping_offset(s as isize) == v) {
            d = d.wrapping_offset(1);
            *d = s;
            *ISA.wrapping_offset(s as isize) = ptr_diff(d, SA);
        }
        c = c.wrapping_offset(1);
    }
    c = last.wrapping_offset(-1);
    e = d.wrapping_offset(1);
    d = b;
    while e < d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.wrapping_offset(s as isize) == v) {
            d = d.wrapping_offset(-1);
            *d = s;
            *ISA.wrapping_offset(s as isize) = ptr_diff(d, SA);
        }
        c = c.wrapping_offset(-1);
    }
}

pub unsafe fn tr_partialcopy(
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

    v = ptr_diff(b, SA).wrapping_sub(1);
    lastrank = -1;
    c = first;
    d = a.wrapping_offset(-1);
    while c <= d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.wrapping_offset(s as isize) == v) {
            d = d.wrapping_offset(1);
            *d = s;
            rank = *ISA.wrapping_offset(s.wrapping_add(depth) as isize);
            if lastrank != rank {
                lastrank = rank;
                newrank = ptr_diff(d, SA);
            }
            *ISA.wrapping_offset(s as isize) = newrank;
        }
        c = c.wrapping_offset(1);
    }

    lastrank = -1;
    e = d;
    while first <= e {
        rank = *ISA.wrapping_offset(*e as isize);
        if lastrank != rank {
            lastrank = rank;
            newrank = ptr_diff(e, SA);
        }
        if newrank != rank {
            *ISA.wrapping_offset(*e as isize) = newrank;
        }
        e = e.wrapping_offset(-1);
    }

    lastrank = -1;
    c = last.wrapping_offset(-1);
    e = d.wrapping_offset(1);
    d = b;
    while e < d {
        s = (*c).wrapping_sub(depth);
        if (0 <= s) && (*ISA.wrapping_offset(s as isize) == v) {
            d = d.wrapping_offset(-1);
            *d = s;
            rank = *ISA.wrapping_offset(s.wrapping_add(depth) as isize);
            if lastrank != rank {
                lastrank = rank;
                newrank = ptr_diff(d, SA);
            }
            *ISA.wrapping_offset(s as isize) = newrank;
        }
        c = c.wrapping_offset(-1);
    }
}

pub unsafe fn tr_introsort(
    ISA: *mut i32,
    ISAd: *const i32,
    SA: *mut i32,
    first: *mut i32,
    last: *mut i32,
    budget: *mut trbudget_t,
) {
    /* struct { const int *a; int *b, *c; int d, e; } stack[TR_STACKSIZE]; */
    #[derive(Clone, Copy)]
    struct StackEntry {
        a: *const i32,
        b: *mut i32,
        c: *mut i32,
        d: i32,
        e: i32,
    }
    const STACK_ENTRY_INIT: StackEntry = StackEntry {
        a: core::ptr::null(),
        b: core::ptr::null_mut(),
        c: core::ptr::null_mut(),
        d: 0,
        e: 0,
    };

    let mut ISAd: *const i32 = ISAd;
    let mut first: *mut i32 = first;
    let mut last: *mut i32 = last;

    let mut stack: [StackEntry; TR_STACKSIZE] = [STACK_ENTRY_INIT; TR_STACKSIZE];
    let mut a: *mut i32 = core::ptr::null_mut();
    let mut b: *mut i32 = core::ptr::null_mut();
    let mut c: *mut i32;
    let mut t: i32;
    let mut v: i32;
    let mut x: i32 = 0;
    let incr: i32 = ptr_diff(ISAd, ISA);
    let mut limit: i32;
    let mut next: i32;
    let mut ssize: i32;
    let mut trlink: i32 = -1;

    macro_rules! stack_push5 {
        ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {{
            stack[ssize as usize].a = $a;
            stack[ssize as usize].b = $b;
            stack[ssize as usize].c = $c;
            stack[ssize as usize].d = $d;
            stack[ssize as usize].e = $e;
            ssize += 1;
        }};
    }
    macro_rules! stack_pop5 {
        ($a:ident, $b:ident, $c:ident, $d:ident, $e:ident) => {{
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

    ssize = 0;
    limit = tr_ilg(ptr_diff(last, first));
    loop {
        if limit < 0 {
            if limit == -1 {
                /* tandem repeat partition */
                tr_partition(
                    ISAd.wrapping_offset(-(incr as isize)),
                    first,
                    first,
                    last,
                    &mut a,
                    &mut b,
                    ptr_diff(last, SA).wrapping_sub(1),
                );

                /* update ranks */
                if a < last {
                    c = first;
                    v = ptr_diff(a, SA).wrapping_sub(1);
                    while c < a {
                        *ISA.wrapping_offset(*c as isize) = v;
                        c = c.wrapping_offset(1);
                    }
                }
                if b < last {
                    c = a;
                    v = ptr_diff(b, SA).wrapping_sub(1);
                    while c < b {
                        *ISA.wrapping_offset(*c as isize) = v;
                        c = c.wrapping_offset(1);
                    }
                }

                /* push */
                if 1 < ptr_diff(b, a) {
                    stack_push5!(core::ptr::null(), a, b, 0, 0);
                    stack_push5!(
                        ISAd.wrapping_offset(-(incr as isize)),
                        first,
                        last,
                        -2,
                        trlink
                    );
                    trlink = ssize - 2;
                }
                if ptr_diff(a, first) <= ptr_diff(last, b) {
                    if 1 < ptr_diff(a, first) {
                        stack_push5!(ISAd, b, last, tr_ilg(ptr_diff(last, b)), trlink);
                        last = a;
                        limit = tr_ilg(ptr_diff(a, first));
                    } else if 1 < ptr_diff(last, b) {
                        first = b;
                        limit = tr_ilg(ptr_diff(last, b));
                    } else {
                        stack_pop5!(ISAd, first, last, limit, trlink);
                    }
                } else {
                    if 1 < ptr_diff(last, b) {
                        stack_push5!(ISAd, first, a, tr_ilg(ptr_diff(a, first)), trlink);
                        first = b;
                        limit = tr_ilg(ptr_diff(last, b));
                    } else if 1 < ptr_diff(a, first) {
                        last = a;
                        limit = tr_ilg(ptr_diff(a, first));
                    } else {
                        stack_pop5!(ISAd, first, last, limit, trlink);
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
                stack_pop5!(ISAd, first, last, limit, trlink);
            } else {
                /* sorted partition */
                if 0 <= *first {
                    a = first;
                    loop {
                        *ISA.wrapping_offset(*a as isize) = ptr_diff(a, SA);
                        a = a.wrapping_offset(1);
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
                        a = a.wrapping_offset(1);
                        if !(*a < 0) {
                            break;
                        }
                    }
                    next = if *ISA.wrapping_offset(*a as isize)
                        != *ISAd.wrapping_offset(*a as isize)
                    {
                        tr_ilg(ptr_diff(a, first).wrapping_add(1))
                    } else {
                        -1
                    };
                    a = a.wrapping_offset(1);
                    if a < last {
                        b = first;
                        v = ptr_diff(a, SA).wrapping_sub(1);
                        while b < a {
                            *ISA.wrapping_offset(*b as isize) = v;
                            b = b.wrapping_offset(1);
                        }
                    }

                    /* push */
                    if trbudget_check(budget, ptr_diff(a, first)) != 0 {
                        if ptr_diff(a, first) <= ptr_diff(last, a) {
                            stack_push5!(ISAd, a, last, -3, trlink);
                            ISAd = ISAd.wrapping_offset(incr as isize);
                            last = a;
                            limit = next;
                        } else {
                            if 1 < ptr_diff(last, a) {
                                stack_push5!(
                                    ISAd.wrapping_offset(incr as isize),
                                    first,
                                    a,
                                    next,
                                    trlink
                                );
                                first = a;
                                limit = -3;
                            } else {
                                ISAd = ISAd.wrapping_offset(incr as isize);
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
                            stack_pop5!(ISAd, first, last, limit, trlink);
                        }
                    }
                } else {
                    stack_pop5!(ISAd, first, last, limit, trlink);
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
            let limit_old = limit;
            limit -= 1;
            if limit_old == 0 {
                tr_heapsort(ISAd, first, ptr_diff(last, first));
                a = last.wrapping_offset(-1);
                while first < a {
                    x = *ISAd.wrapping_offset(*a as isize);
                    b = a.wrapping_offset(-1);
                    while (first <= b) && (*ISAd.wrapping_offset(*b as isize) == x) {
                        *b = !*b;
                        b = b.wrapping_offset(-1);
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
        v = *ISAd.wrapping_offset(*first as isize);

        /* partition */
        tr_partition(
            ISAd,
            first,
            first.wrapping_offset(1),
            last,
            &mut a,
            &mut b,
            v,
        );
        if ptr_diff(last, first) != ptr_diff(b, a) {
            next = if *ISA.wrapping_offset(*a as isize) != v {
                tr_ilg(ptr_diff(b, a))
            } else {
                -1
            };

            /* update ranks */
            c = first;
            v = ptr_diff(a, SA).wrapping_sub(1);
            while c < a {
                *ISA.wrapping_offset(*c as isize) = v;
                c = c.wrapping_offset(1);
            }
            if b < last {
                c = a;
                v = ptr_diff(b, SA).wrapping_sub(1);
                while c < b {
                    *ISA.wrapping_offset(*c as isize) = v;
                    c = c.wrapping_offset(1);
                }
            }

            /* push */
            if (1 < ptr_diff(b, a)) && (trbudget_check(budget, ptr_diff(b, a)) != 0) {
                if ptr_diff(a, first) <= ptr_diff(last, b) {
                    if ptr_diff(last, b) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(a, first) {
                            stack_push5!(
                                ISAd.wrapping_offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            stack_push5!(ISAd, b, last, limit, trlink);
                            last = a;
                        } else if 1 < ptr_diff(last, b) {
                            stack_push5!(
                                ISAd.wrapping_offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            first = b;
                        } else {
                            ISAd = ISAd.wrapping_offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if ptr_diff(a, first) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(a, first) {
                            stack_push5!(ISAd, b, last, limit, trlink);
                            stack_push5!(
                                ISAd.wrapping_offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            last = a;
                        } else {
                            stack_push5!(ISAd, b, last, limit, trlink);
                            ISAd = ISAd.wrapping_offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        stack_push5!(ISAd, b, last, limit, trlink);
                        stack_push5!(ISAd, first, a, limit, trlink);
                        ISAd = ISAd.wrapping_offset(incr as isize);
                        first = a;
                        last = b;
                        limit = next;
                    }
                } else {
                    if ptr_diff(a, first) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(last, b) {
                            stack_push5!(
                                ISAd.wrapping_offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            stack_push5!(ISAd, first, a, limit, trlink);
                            first = b;
                        } else if 1 < ptr_diff(a, first) {
                            stack_push5!(
                                ISAd.wrapping_offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            last = a;
                        } else {
                            ISAd = ISAd.wrapping_offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else if ptr_diff(last, b) <= ptr_diff(b, a) {
                        if 1 < ptr_diff(last, b) {
                            stack_push5!(ISAd, first, a, limit, trlink);
                            stack_push5!(
                                ISAd.wrapping_offset(incr as isize),
                                a,
                                b,
                                next,
                                trlink
                            );
                            first = b;
                        } else {
                            stack_push5!(ISAd, first, a, limit, trlink);
                            ISAd = ISAd.wrapping_offset(incr as isize);
                            first = a;
                            last = b;
                            limit = next;
                        }
                    } else {
                        stack_push5!(ISAd, first, a, limit, trlink);
                        stack_push5!(ISAd, b, last, limit, trlink);
                        ISAd = ISAd.wrapping_offset(incr as isize);
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
                        stack_push5!(ISAd, b, last, limit, trlink);
                        last = a;
                    } else if 1 < ptr_diff(last, b) {
                        first = b;
                    } else {
                        stack_pop5!(ISAd, first, last, limit, trlink);
                    }
                } else {
                    if 1 < ptr_diff(last, b) {
                        stack_push5!(ISAd, first, a, limit, trlink);
                        first = b;
                    } else if 1 < ptr_diff(a, first) {
                        last = a;
                    } else {
                        stack_pop5!(ISAd, first, last, limit, trlink);
                    }
                }
            }
        } else {
            if trbudget_check(budget, ptr_diff(last, first)) != 0 {
                limit = tr_ilg(ptr_diff(last, first));
                ISAd = ISAd.wrapping_offset(incr as isize);
            } else {
                if 0 <= trlink {
                    stack[trlink as usize].d = -1;
                }
                stack_pop5!(ISAd, first, last, limit, trlink);
            }
        }
    }
}

/*---------------------------------------------------------------------------*/

/* Tandem repeat sort */
pub unsafe fn trsort(ISA: *mut i32, SA: *mut i32, n: i32, depth: i32) {
    let mut ISAd: *mut i32;
    let mut first: *mut i32;
    let mut last: *mut i32;
    let mut budget: trbudget_t = trbudget_t::default();
    let mut t: i32;
    let mut skip: i32;
    let mut unsorted: i32;

    trbudget_init(&mut budget, tr_ilg(n).wrapping_mul(2) / 3, n);
    /*  trbudget_init(&budget, tr_ilg(n) * 3 / 4, n); */
    ISAd = ISA.wrapping_offset(depth as isize);
    while n.wrapping_neg() < *SA {
        first = SA;
        skip = 0;
        unsorted = 0;
        loop {
            t = *first;
            if t < 0 {
                first = first.wrapping_offset(-(t as isize));
                skip = skip.wrapping_add(t);
            } else {
                if skip != 0 {
                    *first.wrapping_offset(skip as isize) = skip;
                    skip = 0;
                }
                last = SA
                    .wrapping_offset(*ISA.wrapping_offset(t as isize) as isize)
                    .wrapping_offset(1);
                if 1 < ptr_diff(last, first) {
                    budget.count = 0;
                    tr_introsort(ISA, ISAd, SA, first, last, &mut budget);
                    if budget.count != 0 {
                        unsorted = unsorted.wrapping_add(budget.count);
                    } else {
                        skip = ptr_diff(first, last);
                    }
                } else if ptr_diff(last, first) == 1 {
                    skip = -1;
                }
                first = last;
            }
            if !(first < SA.wrapping_offset(n as isize)) {
                break;
            }
        }
        if skip != 0 {
            *first.wrapping_offset(skip as isize) = skip;
        }
        if unsorted == 0 {
            break;
        }
        ISAd = ISAd.wrapping_offset(ptr_diff(ISAd, ISA) as isize);
    }
}

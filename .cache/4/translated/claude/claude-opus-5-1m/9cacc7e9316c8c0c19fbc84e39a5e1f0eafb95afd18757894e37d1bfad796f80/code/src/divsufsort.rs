//! dictBuilder/divsufsort.c -- main entry points (sort_typeBstar / construct_SA /
//! construct_BWT / construct_BWT_indexes / divsufsort / divbwt) and the
//! prototypes from dictBuilder/divsufsort.h.
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
use crate::divsufsort_ss::sssort;
use crate::divsufsort_tr::trsort;
use crate::mem::{free, malloc};

/* `int *` pointer difference, as C's `p - q` on `int *` yields ptrdiff_t which is
then converted to `int` by the surrounding assignment/return. */
#[inline(always)]
fn pdiff(a: *const i32, b: *const i32) -> i32 {
    ((a as isize).wrapping_sub(b as isize) / (core::mem::size_of::<i32>() as isize)) as i32
}

/*---------------------------------------------------------------------------*/

/* Sorts suffixes of type B*. */
pub unsafe fn sort_typeBstar(
    T: *const u8,
    SA: *mut i32,
    bucket_A: *mut i32,
    bucket_B: *mut i32,
    n: i32,
    openMP: i32,
) -> i32 {
    let mut PAb: *mut i32;
    let mut ISAb: *mut i32;
    let mut buf: *mut i32;
    let mut i: i32;
    let mut j: i32;
    let mut k: i32;
    let mut t: i32;
    let mut m: i32;
    let mut bufsize: i32;
    let mut c0: i32;
    let mut c1: i32;

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

    /* Count the number of occurrences of the first one or two characters of each
    type A, B and B* suffix. Moreover, store the beginning position of all
    type B* suffixes into the array SA. */
    i = n.wrapping_sub(1);
    m = n;
    c0 = *T.wrapping_offset(n.wrapping_sub(1) as isize) as i32;
    c1 = 0;
    while 0 <= i {
        /* type A suffix. */
        loop {
            c1 = c0;
            *bucket_A.wrapping_offset(c1 as isize) =
                (*bucket_A.wrapping_offset(c1 as isize)).wrapping_add(1);
            i = i.wrapping_sub(1);
            if !(0 <= i) {
                break;
            }
            c0 = *T.wrapping_offset(i as isize) as i32;
            if !(c0 >= c1) {
                break;
            }
        }
        if 0 <= i {
            /* type B* suffix. */
            *bucket_B.wrapping_offset(((c0 << 8) | c1) as isize) =
                (*bucket_B.wrapping_offset(((c0 << 8) | c1) as isize)).wrapping_add(1);
            m = m.wrapping_sub(1);
            *SA.wrapping_offset(m as isize) = i;
            /* type B suffix. */
            i = i.wrapping_sub(1);
            c1 = c0;
            while (0 <= i)
                && {
                    c0 = *T.wrapping_offset(i as isize) as i32;
                    c0 <= c1
                }
            {
                *bucket_B.wrapping_offset(((c1 << 8) | c0) as isize) =
                    (*bucket_B.wrapping_offset(((c1 << 8) | c0) as isize)).wrapping_add(1);
                i = i.wrapping_sub(1);
                c1 = c0;
            }
        }
    }
    m = n.wrapping_sub(m);
    /*
    note:
      A type B* suffix is lexicographically smaller than a type B suffix that
      begins with the same first two characters.
    */

    /* Calculate the index of start/end point of each bucket. */
    c0 = 0;
    i = 0;
    j = 0;
    while c0 < ALPHABET_SIZE as i32 {
        t = i.wrapping_add(*bucket_A.wrapping_offset(c0 as isize));
        *bucket_A.wrapping_offset(c0 as isize) = i.wrapping_add(j); /* start point */
        i = t.wrapping_add(*bucket_B.wrapping_offset(((c0 << 8) | c0) as isize));
        c1 = c0.wrapping_add(1);
        while c1 < ALPHABET_SIZE as i32 {
            j = j.wrapping_add(*bucket_B.wrapping_offset(((c0 << 8) | c1) as isize));
            *bucket_B.wrapping_offset(((c0 << 8) | c1) as isize) = j; /* end point */
            i = i.wrapping_add(*bucket_B.wrapping_offset(((c1 << 8) | c0) as isize));
            c1 += 1;
        }
        c0 += 1;
    }

    if 0 < m {
        /* Sort the type B* suffixes by their first two characters. */
        PAb = SA
            .wrapping_offset(n as isize)
            .wrapping_offset(-(m as isize));
        ISAb = SA.wrapping_offset(m as isize);
        i = m.wrapping_sub(2);
        while 0 <= i {
            t = *PAb.wrapping_offset(i as isize);
            c0 = *T.wrapping_offset(t as isize) as i32;
            c1 = *T.wrapping_offset(t.wrapping_add(1) as isize) as i32;
            *bucket_B.wrapping_offset(((c0 << 8) | c1) as isize) =
                (*bucket_B.wrapping_offset(((c0 << 8) | c1) as isize)).wrapping_sub(1);
            *SA.wrapping_offset(*bucket_B.wrapping_offset(((c0 << 8) | c1) as isize) as isize) = i;
            i -= 1;
        }
        t = *PAb.wrapping_offset(m.wrapping_sub(1) as isize);
        c0 = *T.wrapping_offset(t as isize) as i32;
        c1 = *T.wrapping_offset(t.wrapping_add(1) as isize) as i32;
        *bucket_B.wrapping_offset(((c0 << 8) | c1) as isize) =
            (*bucket_B.wrapping_offset(((c0 << 8) | c1) as isize)).wrapping_sub(1);
        *SA.wrapping_offset(*bucket_B.wrapping_offset(((c0 << 8) | c1) as isize) as isize) =
            m.wrapping_sub(1);

        /* Sort the type B* substrings using sssort. */
        buf = SA.wrapping_offset(m as isize);
        bufsize = n.wrapping_sub(2i32.wrapping_mul(m));
        c0 = (ALPHABET_SIZE as i32).wrapping_sub(2);
        j = m;
        while 0 < j {
            c1 = (ALPHABET_SIZE as i32).wrapping_sub(1);
            while c0 < c1 {
                i = *bucket_B.wrapping_offset(((c0 << 8) | c1) as isize);
                if 1 < j.wrapping_sub(i) {
                    sssort(
                        T,
                        PAb as *const i32,
                        SA.wrapping_offset(i as isize),
                        SA.wrapping_offset(j as isize),
                        buf,
                        bufsize,
                        2,
                        n,
                        if *SA.wrapping_offset(i as isize) == m.wrapping_sub(1) {
                            1
                        } else {
                            0
                        },
                    );
                }
                j = i;
                c1 -= 1;
            }
            c0 -= 1;
        }

        /* Compute ranks of type B* substrings. */
        i = m.wrapping_sub(1);
        while 0 <= i {
            if 0 <= *SA.wrapping_offset(i as isize) {
                j = i;
                loop {
                    *ISAb.wrapping_offset(*SA.wrapping_offset(i as isize) as isize) = i;
                    i = i.wrapping_sub(1);
                    if !((0 <= i) && (0 <= *SA.wrapping_offset(i as isize))) {
                        break;
                    }
                }
                *SA.wrapping_offset(i.wrapping_add(1) as isize) = i.wrapping_sub(j);
                if i <= 0 {
                    break;
                }
            }
            j = i;
            loop {
                let v = !*SA.wrapping_offset(i as isize);
                *SA.wrapping_offset(i as isize) = v;
                *ISAb.wrapping_offset(v as isize) = j;
                i = i.wrapping_sub(1);
                if !(*SA.wrapping_offset(i as isize) < 0) {
                    break;
                }
            }
            *ISAb.wrapping_offset(*SA.wrapping_offset(i as isize) as isize) = j;
            i -= 1;
        }

        /* Construct the inverse suffix array of type B* suffixes using trsort. */
        trsort(ISAb, SA, m, 1);

        /* Set the sorted order of type B* suffixes. */
        i = n.wrapping_sub(1);
        j = m;
        c0 = *T.wrapping_offset(n.wrapping_sub(1) as isize) as i32;
        while 0 <= i {
            i = i.wrapping_sub(1);
            c1 = c0;
            while (0 <= i)
                && {
                    c0 = *T.wrapping_offset(i as isize) as i32;
                    c0 >= c1
                }
            {
                i = i.wrapping_sub(1);
                c1 = c0;
            }
            if 0 <= i {
                t = i;
                i = i.wrapping_sub(1);
                c1 = c0;
                while (0 <= i)
                    && {
                        c0 = *T.wrapping_offset(i as isize) as i32;
                        c0 <= c1
                    }
                {
                    i = i.wrapping_sub(1);
                    c1 = c0;
                }
                j = j.wrapping_sub(1);
                *SA.wrapping_offset(*ISAb.wrapping_offset(j as isize) as isize) =
                    if (t == 0) || (1 < t.wrapping_sub(i)) { t } else { !t };
            }
        }

        /* Calculate the index of start/end point of each bucket. */
        *bucket_B.wrapping_offset(
            ((((ALPHABET_SIZE as i32) - 1) << 8) | ((ALPHABET_SIZE as i32) - 1)) as isize,
        ) = n; /* end point */
        c0 = (ALPHABET_SIZE as i32).wrapping_sub(2);
        k = m.wrapping_sub(1);
        while 0 <= c0 {
            i = (*bucket_A.wrapping_offset(c0.wrapping_add(1) as isize)).wrapping_sub(1);
            c1 = (ALPHABET_SIZE as i32).wrapping_sub(1);
            while c0 < c1 {
                t = i.wrapping_sub(*bucket_B.wrapping_offset(((c1 << 8) | c0) as isize));
                *bucket_B.wrapping_offset(((c1 << 8) | c0) as isize) = i; /* end point */

                /* Move all type B* suffixes to the correct position. */
                i = t;
                j = *bucket_B.wrapping_offset(((c0 << 8) | c1) as isize);
                while j <= k {
                    *SA.wrapping_offset(i as isize) = *SA.wrapping_offset(k as isize);
                    i = i.wrapping_sub(1);
                    k = k.wrapping_sub(1);
                }
                c1 -= 1;
            }
            *bucket_B.wrapping_offset(((c0 << 8) | c0.wrapping_add(1)) as isize) = i
                .wrapping_sub(*bucket_B.wrapping_offset(((c0 << 8) | c0) as isize))
                .wrapping_add(1); /* start point */
            *bucket_B.wrapping_offset(((c0 << 8) | c0) as isize) = i; /* end point */
            c0 -= 1;
        }
    }

    m
}

/* Constructs the suffix array by using the sorted order of type B* suffixes. */
pub unsafe fn construct_SA(
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
    let mut c2: i32;

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = (ALPHABET_SIZE as i32).wrapping_sub(2);
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.wrapping_offset(
                *bucket_B.wrapping_offset(((c1 << 8) | c1.wrapping_add(1)) as isize) as isize,
            );
            j = SA
                .wrapping_offset(*bucket_A.wrapping_offset(c1.wrapping_add(1) as isize) as isize)
                .wrapping_offset(-1);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    *j = !s;
                    s = s.wrapping_sub(1);
                    c0 = *T.wrapping_offset(s as isize) as i32;
                    if (0 < s) && ((*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *bucket_B.wrapping_offset(((c1 << 8) | c2) as isize) = pdiff(k, SA);
                        }
                        c2 = c0;
                        k = SA.wrapping_offset(
                            *bucket_B.wrapping_offset(((c1 << 8) | c2) as isize) as isize,
                        );
                    }
                    *k = s;
                    k = k.wrapping_offset(-1);
                } else {
                    *j = !s;
                }
                j = j.wrapping_offset(-1);
            }
            c1 -= 1;
        }
    }

    /* Construct the suffix array by using
    the sorted order of type B suffixes. */
    c2 = *T.wrapping_offset(n.wrapping_sub(1) as isize) as i32;
    k = SA.wrapping_offset(*bucket_A.wrapping_offset(c2 as isize) as isize);
    *k = if (*T.wrapping_offset(n.wrapping_sub(2) as isize) as i32) < c2 {
        !n.wrapping_sub(1)
    } else {
        n.wrapping_sub(1)
    };
    k = k.wrapping_offset(1);
    /* Scan the suffix array from left to right. */
    i = SA;
    j = SA.wrapping_offset(n as isize);
    while i < j {
        s = *i;
        if 0 < s {
            s = s.wrapping_sub(1);
            c0 = *T.wrapping_offset(s as isize) as i32;
            if (s == 0) || ((*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32) < c0) {
                s = !s;
            }
            if c0 != c2 {
                *bucket_A.wrapping_offset(c2 as isize) = pdiff(k, SA);
                c2 = c0;
                k = SA.wrapping_offset(*bucket_A.wrapping_offset(c2 as isize) as isize);
            }
            *k = s;
            k = k.wrapping_offset(1);
        } else {
            *i = !s;
        }
        i = i.wrapping_offset(1);
    }
}

/* Constructs the burrows-wheeler transformed string directly
by using the sorted order of type B* suffixes. */
pub unsafe fn construct_BWT(
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
    let mut c2: i32;

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = (ALPHABET_SIZE as i32).wrapping_sub(2);
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.wrapping_offset(
                *bucket_B.wrapping_offset(((c1 << 8) | c1.wrapping_add(1)) as isize) as isize,
            );
            j = SA
                .wrapping_offset(*bucket_A.wrapping_offset(c1.wrapping_add(1) as isize) as isize)
                .wrapping_offset(-1);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    s = s.wrapping_sub(1);
                    c0 = *T.wrapping_offset(s as isize) as i32;
                    *j = !(c0 as i32);
                    if (0 < s) && ((*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *bucket_B.wrapping_offset(((c1 << 8) | c2) as isize) = pdiff(k, SA);
                        }
                        c2 = c0;
                        k = SA.wrapping_offset(
                            *bucket_B.wrapping_offset(((c1 << 8) | c2) as isize) as isize,
                        );
                    }
                    *k = s;
                    k = k.wrapping_offset(-1);
                } else if s != 0 {
                    *j = !s;
                }
                j = j.wrapping_offset(-1);
            }
            c1 -= 1;
        }
    }

    /* Construct the BWTed string by using
    the sorted order of type B suffixes. */
    c2 = *T.wrapping_offset(n.wrapping_sub(1) as isize) as i32;
    k = SA.wrapping_offset(*bucket_A.wrapping_offset(c2 as isize) as isize);
    *k = if (*T.wrapping_offset(n.wrapping_sub(2) as isize) as i32) < c2 {
        !(*T.wrapping_offset(n.wrapping_sub(2) as isize) as i32)
    } else {
        n.wrapping_sub(1)
    };
    k = k.wrapping_offset(1);
    /* Scan the suffix array from left to right. */
    i = SA;
    j = SA.wrapping_offset(n as isize);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            s = s.wrapping_sub(1);
            c0 = *T.wrapping_offset(s as isize) as i32;
            *i = c0;
            if (0 < s) && ((*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32) < c0) {
                s = !(*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32);
            }
            if c0 != c2 {
                *bucket_A.wrapping_offset(c2 as isize) = pdiff(k, SA);
                c2 = c0;
                k = SA.wrapping_offset(*bucket_A.wrapping_offset(c2 as isize) as isize);
            }
            *k = s;
            k = k.wrapping_offset(1);
        } else if s != 0 {
            *i = !s;
        } else {
            orig = i;
        }
        i = i.wrapping_offset(1);
    }

    pdiff(orig, SA)
}

/* Constructs the burrows-wheeler transformed string directly
by using the sorted order of type B* suffixes. */
pub unsafe fn construct_BWT_indexes(
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
    let mut c2: i32;

    let mut r#mod: i32 = n / 8;
    {
        r#mod |= r#mod >> 1;
        r#mod |= r#mod >> 2;
        r#mod |= r#mod >> 4;
        r#mod |= r#mod >> 8;
        r#mod |= r#mod >> 16;
        r#mod >>= 1;

        *num_indexes = (n.wrapping_sub(1) / (r#mod.wrapping_add(1))) as u8;
    }

    if 0 < m {
        /* Construct the sorted order of type B suffixes by using
        the sorted order of type B* suffixes. */
        c1 = (ALPHABET_SIZE as i32).wrapping_sub(2);
        while 0 <= c1 {
            /* Scan the suffix array from right to left. */
            i = SA.wrapping_offset(
                *bucket_B.wrapping_offset(((c1 << 8) | c1.wrapping_add(1)) as isize) as isize,
            );
            j = SA
                .wrapping_offset(*bucket_A.wrapping_offset(c1.wrapping_add(1) as isize) as isize)
                .wrapping_offset(-1);
            k = core::ptr::null_mut();
            c2 = -1;
            while i <= j {
                s = *j;
                if 0 < s {
                    if (s & r#mod) == 0 {
                        *indexes.wrapping_offset(
                            (s / (r#mod.wrapping_add(1))).wrapping_sub(1) as isize,
                        ) = pdiff(j, SA);
                    }

                    s = s.wrapping_sub(1);
                    c0 = *T.wrapping_offset(s as isize) as i32;
                    *j = !(c0 as i32);
                    if (0 < s) && ((*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32) > c0) {
                        s = !s;
                    }
                    if c0 != c2 {
                        if 0 <= c2 {
                            *bucket_B.wrapping_offset(((c1 << 8) | c2) as isize) = pdiff(k, SA);
                        }
                        c2 = c0;
                        k = SA.wrapping_offset(
                            *bucket_B.wrapping_offset(((c1 << 8) | c2) as isize) as isize,
                        );
                    }
                    *k = s;
                    k = k.wrapping_offset(-1);
                } else if s != 0 {
                    *j = !s;
                }
                j = j.wrapping_offset(-1);
            }
            c1 -= 1;
        }
    }

    /* Construct the BWTed string by using
    the sorted order of type B suffixes. */
    c2 = *T.wrapping_offset(n.wrapping_sub(1) as isize) as i32;
    k = SA.wrapping_offset(*bucket_A.wrapping_offset(c2 as isize) as isize);
    if (*T.wrapping_offset(n.wrapping_sub(2) as isize) as i32) < c2 {
        if (n.wrapping_sub(1) & r#mod) == 0 {
            *indexes.wrapping_offset(
                (n.wrapping_sub(1) / (r#mod.wrapping_add(1))).wrapping_sub(1) as isize,
            ) = pdiff(k, SA);
        }
        *k = !(*T.wrapping_offset(n.wrapping_sub(2) as isize) as i32);
        k = k.wrapping_offset(1);
    } else {
        *k = n.wrapping_sub(1);
        k = k.wrapping_offset(1);
    }

    /* Scan the suffix array from left to right. */
    i = SA;
    j = SA.wrapping_offset(n as isize);
    orig = SA;
    while i < j {
        s = *i;
        if 0 < s {
            if (s & r#mod) == 0 {
                *indexes
                    .wrapping_offset((s / (r#mod.wrapping_add(1))).wrapping_sub(1) as isize) =
                    pdiff(i, SA);
            }

            s = s.wrapping_sub(1);
            c0 = *T.wrapping_offset(s as isize) as i32;
            *i = c0;
            if c0 != c2 {
                *bucket_A.wrapping_offset(c2 as isize) = pdiff(k, SA);
                c2 = c0;
                k = SA.wrapping_offset(*bucket_A.wrapping_offset(c2 as isize) as isize);
            }
            if (0 < s) && ((*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32) < c0) {
                if (s & r#mod) == 0 {
                    *indexes.wrapping_offset(
                        (s / (r#mod.wrapping_add(1))).wrapping_sub(1) as isize,
                    ) = pdiff(k, SA);
                }
                *k = !(*T.wrapping_offset(s.wrapping_sub(1) as isize) as i32);
                k = k.wrapping_offset(1);
            } else {
                *k = s;
                k = k.wrapping_offset(1);
            }
        } else if s != 0 {
            *i = !s;
        } else {
            orig = i;
        }
        i = i.wrapping_offset(1);
    }

    pdiff(orig, SA)
}

/*---------------------------------------------------------------------------*/

/*- Function -*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divsufsort(
    T: *const u8,
    SA: *mut core::ffi::c_int,
    n: core::ffi::c_int,
    openMP: core::ffi::c_int,
) -> core::ffi::c_int {
    let bucket_A: *mut i32;
    let bucket_B: *mut i32;
    let mut m: i32;
    let mut err: i32 = 0;

    /* Check arguments. */
    if T.is_null() || SA.is_null() || (n < 0) {
        return -1;
    } else if n == 0 {
        return 0;
    } else if n == 1 {
        *SA.wrapping_offset(0) = 0;
        return 0;
    } else if n == 2 {
        m = if (*T.wrapping_offset(0) as i32) < (*T.wrapping_offset(1) as i32) {
            1
        } else {
            0
        };
        *SA.wrapping_offset((m ^ 1) as isize) = 0;
        *SA.wrapping_offset(m as isize) = 1;
        return 0;
    }

    bucket_A = malloc(BUCKET_A_SIZE * 4) as *mut i32;
    bucket_B = malloc(BUCKET_B_SIZE * 4) as *mut i32;

    /* Suffixsort. */
    if !bucket_A.is_null() && !bucket_B.is_null() {
        m = sort_typeBstar(T, SA, bucket_A, bucket_B, n, openMP);
        construct_SA(T, SA, bucket_A, bucket_B, n, m);
    } else {
        err = -2;
    }

    free(bucket_B as *mut core::ffi::c_void);
    free(bucket_A as *mut core::ffi::c_void);

    err
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divbwt(
    T: *const u8,
    U: *mut u8,
    A: *mut core::ffi::c_int,
    n: core::ffi::c_int,
    num_indexes: *mut u8,
    indexes: *mut core::ffi::c_int,
    openMP: core::ffi::c_int,
) -> core::ffi::c_int {
    let mut B: *mut i32;
    let bucket_A: *mut i32;
    let bucket_B: *mut i32;
    let mut m: i32;
    let mut pidx: i32;
    let mut i: i32;

    /* Check arguments. */
    if T.is_null() || U.is_null() || (n < 0) {
        return -1;
    } else if n <= 1 {
        if n == 1 {
            *U.wrapping_offset(0) = *T.wrapping_offset(0);
        }
        return n;
    }

    B = A;
    if B.is_null() {
        B = malloc((n.wrapping_add(1) as usize).wrapping_mul(4)) as *mut i32;
    }
    bucket_A = malloc(BUCKET_A_SIZE * 4) as *mut i32;
    bucket_B = malloc(BUCKET_B_SIZE * 4) as *mut i32;

    /* Burrows-Wheeler Transform. */
    if !B.is_null() && !bucket_A.is_null() && !bucket_B.is_null() {
        m = sort_typeBstar(T, B, bucket_A, bucket_B, n, openMP);

        if num_indexes.is_null() || indexes.is_null() {
            pidx = construct_BWT(T, B, bucket_A, bucket_B, n, m);
        } else {
            pidx = construct_BWT_indexes(
                T,
                B,
                bucket_A,
                bucket_B,
                n,
                m,
                num_indexes,
                indexes,
            );
        }

        /* Copy to output string. */
        *U.wrapping_offset(0) = *T.wrapping_offset(n.wrapping_sub(1) as isize);
        i = 0;
        while i < pidx {
            *U.wrapping_offset(i.wrapping_add(1) as isize) =
                *B.wrapping_offset(i as isize) as u8;
            i += 1;
        }
        i = i.wrapping_add(1);
        while i < n {
            *U.wrapping_offset(i as isize) = *B.wrapping_offset(i as isize) as u8;
            i += 1;
        }
        pidx = pidx.wrapping_add(1);
    } else {
        pidx = -2;
    }

    free(bucket_B as *mut core::ffi::c_void);
    free(bucket_A as *mut core::ffi::c_void);
    if A.is_null() {
        free(B as *mut core::ffi::c_void);
    }

    pidx
}

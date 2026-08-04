








pub type btac1c_u16 = ::core::ffi::c_ushort;
pub type btac1c_s16 = ::core::ffi::c_short;
pub type btac1c_byte = ::core::ffi::c_uchar;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct btac1c_idxstate_s {
    pub idx: btac1c_u16,
    pub lpred: btac1c_s16,
    pub rpred: btac1c_s16,
    pub tag: btac1c_byte,
    pub bcfcn: btac1c_byte,
    pub bsfcn: btac1c_byte,
    pub usefx: btac1c_byte,
    pub firfx: [[btac1c_s16; 8]; 4],
}
pub type btac1c_idxstate = btac1c_idxstate_s;
unsafe extern "C" fn BTAC1C2_PredictSample(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let mut pred: ::core::ffi::c_int = 0;
    let mut p0: ::core::ffi::c_int = 0;
    let mut p1: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    i = idx;
    match pfcn {
        0 => {
            pred = *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
        }
        1 => {
            pred = 2 as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
        }
        2 => {
            pred = 3 as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                >> 1 as ::core::ffi::c_int;
        }
        3 => {
            pred = 5 as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                >> 2 as ::core::ffi::c_int;
        }
        4 => {
            p0 = *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            p1 = *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            pred = p0 - (p1 >> 1 as ::core::ffi::c_int);
        }
        5 => {
            p0 = *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            p1 = *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            pred = 3 as ::core::ffi::c_int * p0 - p1 >> 2 as ::core::ffi::c_int;
        }
        6 => {
            p0 = *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            p1 = *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            pred = 5 as ::core::ffi::c_int * p0 - p1 >> 3 as ::core::ffi::c_int;
        }
        7 => {
            pred = (18 * psamp.wrapping_offset(((i - 1) & 7) as isize).read()
    - 4 * psamp.wrapping_offset(((i - 2) & 7) as isize).read()
    + 3 * psamp.wrapping_offset(((i - 3) & 7) as isize).read()
    - 2 * psamp.wrapping_offset(((i - 4) & 7) as isize).read()
    + psamp.wrapping_offset(((i - 5) & 7) as isize).read())
    / 16;

        }
        8 => {
            pred = unsafe {
    (72 * *psamp.add(((i - 1) & 7) as usize)
        - 16 * *psamp.add(((i - 2) & 7) as usize)
        + 12 * *psamp.add(((i - 3) & 7) as usize)
        - 8 * *psamp.add(((i - 4) & 7) as usize)
        + 5 * *psamp.add(((i - 5) & 7) as usize)
        - 3 * *psamp.add(((i - 6) & 7) as usize)
        + 3 * *psamp.add(((i - 7) & 7) as usize)
        - *psamp.add(((i - 8) & 7) as usize))
        / 64
};

        }
        9 => {
            pred = (76 * unsafe { *psamp.add(((i - 1) & 7) as usize) }
    - 17 * unsafe { *psamp.add(((i - 2) & 7) as usize) }
    + 10 * unsafe { *psamp.add(((i - 3) & 7) as usize) }
    - 7 * unsafe { *psamp.add(((i - 4) & 7) as usize) }
    + 5 * unsafe { *psamp.add(((i - 5) & 7) as usize) }
    - 4 * unsafe { *psamp.add(((i - 6) & 7) as usize) }
    + 4 * unsafe { *psamp.add(((i - 7) & 7) as usize) }
    - 3 * unsafe { *psamp.add(((i - 8) & 7) as usize) })
    / 64;

        }
        10 => {
            p0 = *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            p1 = *psamp.offset((i - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            pred = 5 as ::core::ffi::c_int * p0 - p1 >> 4 as ::core::ffi::c_int;
        }
        11 => {
            p0 = *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            p1 = *psamp.offset((i - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + *psamp.offset((i - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
            pred = p0 + p1 >> 3 as ::core::ffi::c_int;
        }
        12 | 13 | 14 | 15 => {
            let coeffs = unsafe { &(*ridx).firfx[(pfcn - 12) as usize] };
pred = (0..8)
    .map(|tap| {
        let sample_idx = ((i - 1 - tap as i32) & 7) as isize;
        let sample = unsafe { *psamp.offset(sample_idx) };
        coeffs[tap] as i32 * sample
    })
    .sum::<i32>()
    / 256;

        }
        _ => {
            pred = 0 as ::core::ffi::c_int;
        }
    }
    return pred;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return 2 as ::core::ffi::c_int
        * *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return 3 as ::core::ffi::c_int
        * *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        >> 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    psamp: *mut ::core::ffi::c_int,
    idx: ::core::ffi::c_int,
    pfcn: ::core::ffi::c_int,
    ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let _ = pfcn;
    let _ = ridx;

    let i1 = ((idx - 1) & 7) as isize;
    let i2 = ((idx - 2) & 7) as isize;

    let s1 = *psamp.offset(i1);
    let s2 = *psamp.offset(i2);

    (5 * s1 - s2) >> 2
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn4(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let mut p0: ::core::ffi::c_int = 0;
    let mut p1: ::core::ffi::c_int = 0;
    p0 = *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
    p1 = *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
    return p0 - (p1 >> 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let mut p0: ::core::ffi::c_int = 0;
    let mut p1: ::core::ffi::c_int = 0;
    p0 = *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
    p1 = *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
    return 3 as ::core::ffi::c_int * p0 - p1 >> 2 as ::core::ffi::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    psamp: *mut ::core::ffi::c_int,
    idx: ::core::ffi::c_int,
    pfcn: ::core::ffi::c_int,
    ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let _ = pfcn;
    let _ = ridx;

    let s = ::core::slice::from_raw_parts(psamp as *const ::core::ffi::c_int, 8);
    let i = idx as usize;

    let p0 = s[i.wrapping_sub(1) & 7] + s[i.wrapping_sub(2) & 7];
    let p1 = s[i.wrapping_sub(2) & 7] + s[i.wrapping_sub(3) & 7];
    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    psamp: *mut ::core::ffi::c_int,
    idx: ::core::ffi::c_int,
    pfcn: ::core::ffi::c_int,
    ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let _ = pfcn;
    let _ = ridx;

    let psamp = ::core::slice::from_raw_parts(psamp as *const ::core::ffi::c_int, 8);

    let at = |back: ::core::ffi::c_int| -> ::core::ffi::c_int {
        let pos = ((idx - back) & 7) as usize;
        psamp[pos]
    };

    (18 * at(1) - 4 * at(2) + 3 * at(3) - 2 * at(4) + at(5)) / 16
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    psamp: *mut ::core::ffi::c_int,
    idx: ::core::ffi::c_int,
    pfcn: ::core::ffi::c_int,
    ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let _ = pfcn;
    let _ = ridx;

    let psamp = ::core::slice::from_raw_parts(psamp as *const ::core::ffi::c_int, 8);

    let at = |offset: i32| -> i32 {
        let pos = ((idx - offset) & 7) as usize;
        psamp[pos]
    };

    (72 * at(1)
        - 16 * at(2)
        + 12 * at(3)
        - 8 * at(4)
        + 5 * at(5)
        - 3 * at(6)
        + 3 * at(7)
        - at(8))
        / 64
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return (76 as ::core::ffi::c_int
        * *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 17 as ::core::ffi::c_int
            * *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 10 as ::core::ffi::c_int
            * *psamp.offset((idx - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 7 as ::core::ffi::c_int
            * *psamp.offset((idx - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 5 as ::core::ffi::c_int
            * *psamp.offset((idx - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 4 as ::core::ffi::c_int
            * *psamp.offset((idx - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 4 as ::core::ffi::c_int
            * *psamp.offset((idx - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 3 as ::core::ffi::c_int
            * *psamp.offset((idx - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
        / 64 as ::core::ffi::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    psamp: *mut ::core::ffi::c_int,
    idx: ::core::ffi::c_int,
    pfcn: ::core::ffi::c_int,
    ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let _ = pfcn;
    let _ = ridx;

    let s = |n: i32| -> i32 { *psamp.offset(((idx - n) & 7) as isize) };

    let p0 = s(1) + s(2) + s(3) + s(4);
    let p1 = s(5) + s(6) + s(7) + s(8);

    (5 * p0 - p1) >> 3
}

unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    let mut p0: ::core::ffi::c_int = 0;
    let mut p1: ::core::ffi::c_int = 0;
    p0 = *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
    p1 = *psamp.offset((idx - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + *psamp.offset((idx - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize);
    return p0 + p1 >> 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn BTAC1C2_GetPredictFunc(
    mut pfcn: ::core::ffi::c_int,
) -> *mut ::core::ffi::c_void {
    let mut fcn: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
    match pfcn {
        0 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn0
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        1 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn1
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        2 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn2
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        3 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn3
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        4 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn4
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        5 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn5
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        6 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn6
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        7 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn7
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        8 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn8
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        9 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn9
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        10 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn10
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        11 => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn11
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
        _ => {
            fcn = ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
                >,
                *mut ::core::ffi::c_void,
            >(Some(
                BTAC1C2_PredictSample
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        *mut btac1c_idxstate,
                    ) -> ::core::ffi::c_int,
            ));
        }
    }
    return fcn;
}
#[no_mangle]
pub unsafe extern "C" fn call_predict(mut pfcn: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut fcn: *mut ::core::ffi::c_void = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn0
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        1 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn1
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        2 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn2
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        3 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn3
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        4 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn4
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        5 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn5
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        6 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn6
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        7 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn7
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        8 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn8
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        9 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn9
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        10 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn10
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        11 => {
            result = (fcn
                == ::core::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                    >,
                    *mut ::core::ffi::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn11
                        as unsafe extern "C" fn(
                            *mut ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut btac1c_idxstate,
                        ) -> ::core::ffi::c_int,
                ))) as ::core::ffi::c_int;
        }
        _ => {}
    }
    return result;
}

pub type btac1c_u16 = libc::unix::c_ushort;
pub type btac1c_s16 = libc::unix::c_short;
pub type btac1c_byte = libc::unix::c_uchar;
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
pub type btac1c_idxstate = crate::src::lib::btac1c_idxstate_s;
unsafe extern "C" fn BTAC1C2_PredictSample(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    let mut pred: libc::c_int = 0;
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    let mut i: libc::c_int = 0;
    i = idx;
    match pfcn {
        0 => {
            pred = *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize);
        }
        1 => {
            pred = 2 as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                - *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize);
        }
        2 => {
            pred = 3 as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                - *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                >> 1 as libc::c_int;
        }
        3 => {
            pred = 5 as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                - *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                >> 2 as libc::c_int;
        }
        4 => {
            p0 = *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize);
            p1 = *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize);
            pred = p0 - (p1 >> 1 as libc::c_int);
        }
        5 => {
            p0 = *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize);
            p1 = *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize);
            pred = 3 as libc::c_int * p0 - p1 >> 2 as libc::c_int;
        }
        6 => {
            p0 = *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize);
            p1 = *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize);
            pred = 5 as libc::c_int * p0 - p1 >> 3 as libc::c_int;
        }
        7 => {
            pred = (18 as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                - 4 as libc::c_int
                    * *psamp
                        .offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + 3 as libc::c_int
                    * *psamp
                        .offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize)
                - 2 as libc::c_int
                    * *psamp
                        .offset((i - 4 as libc::c_int & 7 as libc::c_int) as isize)
                + 1 as libc::c_int
                    * *psamp
                        .offset((i - 5 as libc::c_int & 7 as libc::c_int) as isize))
                / 16 as libc::c_int;
        }
        8 => {
            pred = (72 as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                - 16 as libc::c_int
                    * *psamp
                        .offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + 12 as libc::c_int
                    * *psamp
                        .offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize)
                - 8 as libc::c_int
                    * *psamp
                        .offset((i - 4 as libc::c_int & 7 as libc::c_int) as isize)
                + 5 as libc::c_int
                    * *psamp
                        .offset((i - 5 as libc::c_int & 7 as libc::c_int) as isize)
                - 3 as libc::c_int
                    * *psamp
                        .offset((i - 6 as libc::c_int & 7 as libc::c_int) as isize)
                + 3 as libc::c_int
                    * *psamp
                        .offset((i - 7 as libc::c_int & 7 as libc::c_int) as isize)
                - 1 as libc::c_int
                    * *psamp
                        .offset((i - 8 as libc::c_int & 7 as libc::c_int) as isize))
                / 64 as libc::c_int;
        }
        9 => {
            pred = (76 as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                - 17 as libc::c_int
                    * *psamp
                        .offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + 10 as libc::c_int
                    * *psamp
                        .offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize)
                - 7 as libc::c_int
                    * *psamp
                        .offset((i - 4 as libc::c_int & 7 as libc::c_int) as isize)
                + 5 as libc::c_int
                    * *psamp
                        .offset((i - 5 as libc::c_int & 7 as libc::c_int) as isize)
                - 4 as libc::c_int
                    * *psamp
                        .offset((i - 6 as libc::c_int & 7 as libc::c_int) as isize)
                + 4 as libc::c_int
                    * *psamp
                        .offset((i - 7 as libc::c_int & 7 as libc::c_int) as isize)
                - 3 as libc::c_int
                    * *psamp
                        .offset((i - 8 as libc::c_int & 7 as libc::c_int) as isize))
                / 64 as libc::c_int;
        }
        10 => {
            p0 = *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 4 as libc::c_int & 7 as libc::c_int) as isize);
            p1 = *psamp.offset((i - 5 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 6 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 7 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 8 as libc::c_int & 7 as libc::c_int) as isize);
            pred = 5 as libc::c_int * p0 - p1 >> 4 as libc::c_int;
        }
        11 => {
            p0 = *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 4 as libc::c_int & 7 as libc::c_int) as isize);
            p1 = *psamp.offset((i - 5 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 6 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 7 as libc::c_int & 7 as libc::c_int) as isize)
                + *psamp.offset((i - 8 as libc::c_int & 7 as libc::c_int) as isize);
            pred = p0 + p1 >> 3 as libc::c_int;
        }
        12 | 13 | 14 | 15 => {
            pred = ((*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                [0 as libc::c_int as usize] as libc::c_int
                * *psamp.offset((i - 1 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [1 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 2 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [2 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 3 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [3 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 4 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [4 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 5 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [5 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 6 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [6 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 7 as libc::c_int & 7 as libc::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as libc::c_int) as usize]
                    [7 as libc::c_int as usize] as libc::c_int
                    * *psamp
                        .offset((i - 8 as libc::c_int & 7 as libc::c_int) as isize))
                / 256 as libc::c_int;
        }
        _ => {
            pred = 0 as libc::c_int;
        }
    }
    return pred;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn0(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize);
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn1(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return 2 as libc::c_int
        * *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        - *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize);
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn2(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return 3 as libc::c_int
        * *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        - *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        >> 1 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn3(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return 5 as libc::c_int
        * *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        - *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        >> 2 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn4(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    p0 = *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize);
    p1 = *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize);
    return p0 - (p1 >> 1 as libc::c_int);
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn5(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    p0 = *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize);
    p1 = *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize);
    return 3 as libc::c_int * p0 - p1 >> 2 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn6(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    p0 = *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize);
    p1 = *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize);
    return 5 as libc::c_int * p0 - p1 >> 3 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return (18 as libc::c_int
        * *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        - 4 as libc::c_int
            * *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + 3 as libc::c_int
            * *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize)
        - 2 as libc::c_int
            * *psamp.offset((idx - 4 as libc::c_int & 7 as libc::c_int) as isize)
        + 1 as libc::c_int
            * *psamp.offset((idx - 5 as libc::c_int & 7 as libc::c_int) as isize))
        / 16 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return (72 as libc::c_int
        * *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        - 16 as libc::c_int
            * *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + 12 as libc::c_int
            * *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize)
        - 8 as libc::c_int
            * *psamp.offset((idx - 4 as libc::c_int & 7 as libc::c_int) as isize)
        + 5 as libc::c_int
            * *psamp.offset((idx - 5 as libc::c_int & 7 as libc::c_int) as isize)
        - 3 as libc::c_int
            * *psamp.offset((idx - 6 as libc::c_int & 7 as libc::c_int) as isize)
        + 3 as libc::c_int
            * *psamp.offset((idx - 7 as libc::c_int & 7 as libc::c_int) as isize)
        - 1 as libc::c_int
            * *psamp.offset((idx - 8 as libc::c_int & 7 as libc::c_int) as isize))
        / 64 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn9(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    return (76 as libc::c_int
        * *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        - 17 as libc::c_int
            * *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + 10 as libc::c_int
            * *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize)
        - 7 as libc::c_int
            * *psamp.offset((idx - 4 as libc::c_int & 7 as libc::c_int) as isize)
        + 5 as libc::c_int
            * *psamp.offset((idx - 5 as libc::c_int & 7 as libc::c_int) as isize)
        - 4 as libc::c_int
            * *psamp.offset((idx - 6 as libc::c_int & 7 as libc::c_int) as isize)
        + 4 as libc::c_int
            * *psamp.offset((idx - 7 as libc::c_int & 7 as libc::c_int) as isize)
        - 3 as libc::c_int
            * *psamp.offset((idx - 8 as libc::c_int & 7 as libc::c_int) as isize))
        / 64 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn10(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    p0 = *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 4 as libc::c_int & 7 as libc::c_int) as isize);
    p1 = *psamp.offset((idx - 5 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 6 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 7 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 8 as libc::c_int & 7 as libc::c_int) as isize);
    return 5 as libc::c_int * p0 - p1 >> 3 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn11(
    mut psamp: *mut libc::c_int,
    mut idx: libc::c_int,
    mut pfcn: libc::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> libc::c_int {
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    p0 = *psamp.offset((idx - 1 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 2 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 3 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 4 as libc::c_int & 7 as libc::c_int) as isize);
    p1 = *psamp.offset((idx - 5 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 6 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 7 as libc::c_int & 7 as libc::c_int) as isize)
        + *psamp.offset((idx - 8 as libc::c_int & 7 as libc::c_int) as isize);
    return p0 + p1 >> 1 as libc::c_int;
}
unsafe extern "C" fn BTAC1C2_GetPredictFunc(
    mut pfcn: libc::c_int,
) -> *mut libc::c_void {
    let mut fcn: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
    match pfcn {
        0 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn0
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        1 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn1
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        2 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn2
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        3 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn3
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        4 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn4
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        5 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn5
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        6 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn6
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        7 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn7
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        8 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn8
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        9 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn9
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        10 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn10
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        11 => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample_Pfn11
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
        _ => {
            fcn = std::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
                >,
                *mut libc::c_void,
            >(Some(
                BTAC1C2_PredictSample
                    as unsafe extern "C" fn(
                        *mut libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        *mut btac1c_idxstate,
                    ) -> libc::c_int,
            ));
        }
    }
    return fcn;
}
#[no_mangle]
pub unsafe extern "C" fn call_predict(mut pfcn: libc::c_int) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut fcn: *mut libc::c_void = BTAC1C2_GetPredictFunc(pfcn);
    match pfcn {
        0 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn0
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        1 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn1
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        2 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn2
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        3 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn3
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        4 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn4
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        5 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn5
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        6 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn6
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        7 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn7
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        8 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn8
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        9 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn9
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        10 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn10
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        11 => {
            result = (fcn
                == std::mem::transmute::<
                    Option<
                        unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                    >,
                    *mut libc::c_void,
                >(Some(
                    BTAC1C2_PredictSample_Pfn11
                        as unsafe extern "C" fn(
                            *mut libc::c_int,
                            libc::c_int,
                            libc::c_int,
                            *mut btac1c_idxstate,
                        ) -> libc::c_int,
                ))) as libc::c_int;
        }
        _ => {}
    }
    return result;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}


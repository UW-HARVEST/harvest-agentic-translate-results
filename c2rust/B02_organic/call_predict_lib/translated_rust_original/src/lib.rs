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
            pred = (18 as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 4 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 3 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 2 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 1 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
                / 16 as ::core::ffi::c_int;
        }
        8 => {
            pred = (72 as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 16 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 12 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 8 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 5 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 3 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 3 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 1 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
                / 64 as ::core::ffi::c_int;
        }
        9 => {
            pred = (76 as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 17 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 10 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 7 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 5 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 4 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + 4 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                - 3 as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
                / 64 as ::core::ffi::c_int;
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
            pred = ((*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                [0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                * *psamp.offset((i - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [4 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [5 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [6 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
                + (*ridx).firfx[(pfcn - 12 as ::core::ffi::c_int) as usize]
                    [7 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    * *psamp
                        .offset((i - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
                / 256 as ::core::ffi::c_int;
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
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return 5 as ::core::ffi::c_int
        * *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        >> 2 as ::core::ffi::c_int;
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
    return 5 as ::core::ffi::c_int * p0 - p1 >> 3 as ::core::ffi::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn7(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return (18 as ::core::ffi::c_int
        * *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 4 as ::core::ffi::c_int
            * *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 3 as ::core::ffi::c_int
            * *psamp.offset((idx - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 2 as ::core::ffi::c_int
            * *psamp.offset((idx - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 1 as ::core::ffi::c_int
            * *psamp.offset((idx - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
        / 16 as ::core::ffi::c_int;
}
unsafe extern "C" fn BTAC1C2_PredictSample_Pfn8(
    mut psamp: *mut ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut pfcn: ::core::ffi::c_int,
    mut ridx: *mut btac1c_idxstate,
) -> ::core::ffi::c_int {
    return (72 as ::core::ffi::c_int
        * *psamp.offset((idx - 1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 16 as ::core::ffi::c_int
            * *psamp.offset((idx - 2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 12 as ::core::ffi::c_int
            * *psamp.offset((idx - 3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 8 as ::core::ffi::c_int
            * *psamp.offset((idx - 4 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 5 as ::core::ffi::c_int
            * *psamp.offset((idx - 5 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 3 as ::core::ffi::c_int
            * *psamp.offset((idx - 6 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        + 3 as ::core::ffi::c_int
            * *psamp.offset((idx - 7 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize)
        - 1 as ::core::ffi::c_int
            * *psamp.offset((idx - 8 as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as isize))
        / 64 as ::core::ffi::c_int;
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
    return 5 as ::core::ffi::c_int * p0 - p1 >> 3 as ::core::ffi::c_int;
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

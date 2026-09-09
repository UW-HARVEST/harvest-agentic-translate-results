//! Literal, semantics-preserving transliteration of `clevels.h`.
//!
//! Contains the pre-defined compression level table `ZSTD_defaultCParameters`
//! and `ZSTD_MAX_CLEVEL`. Every windowLog/chainLog/hashLog/searchLog/minMatch/
//! targetLength/strategy value is transcribed exactly from the C source; these
//! numbers ARE the compression levels.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::common::zstd_h::{
    ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2, ZSTD_compressionParameters, ZSTD_dfast,
    ZSTD_fast, ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2,
};

pub const ZSTD_MAX_CLEVEL: usize = 22;

/* helper to keep the table rows terse and 1:1 with the C `{ W, C, H, S, L, TL, strat }` rows */
const fn cp(
    windowLog: u32,
    chainLog: u32,
    hashLog: u32,
    searchLog: u32,
    minMatch: u32,
    targetLength: u32,
    strategy: crate::common::zstd_h::ZSTD_strategy,
) -> ZSTD_compressionParameters {
    ZSTD_compressionParameters {
        windowLog,
        chainLog,
        hashLog,
        searchLog,
        minMatch,
        targetLength,
        strategy,
    }
}

/* __attribute__((__unused__)) static const ZSTD_compressionParameters
 *   ZSTD_defaultCParameters[4][ZSTD_MAX_CLEVEL+1] */
pub static ZSTD_defaultCParameters: [[ZSTD_compressionParameters; ZSTD_MAX_CLEVEL + 1]; 4] = [
    /* "default" - for any srcSize > 256 KB */
    /* W,  C,  H,  S,  L, TL, strat */
    [
        cp(19, 12, 13, 1, 6, 1, ZSTD_fast),      /* base for negative levels */
        cp(19, 13, 14, 1, 7, 0, ZSTD_fast),      /* level  1 */
        cp(20, 15, 16, 1, 6, 0, ZSTD_fast),      /* level  2 */
        cp(21, 16, 17, 1, 5, 0, ZSTD_dfast),     /* level  3 */
        cp(21, 18, 18, 1, 5, 0, ZSTD_dfast),     /* level  4 */
        cp(21, 18, 19, 3, 5, 2, ZSTD_greedy),    /* level  5 */
        cp(21, 18, 19, 3, 5, 4, ZSTD_lazy),      /* level  6 */
        cp(21, 19, 20, 4, 5, 8, ZSTD_lazy),      /* level  7 */
        cp(21, 19, 20, 4, 5, 16, ZSTD_lazy2),    /* level  8 */
        cp(22, 20, 21, 4, 5, 16, ZSTD_lazy2),    /* level  9 */
        cp(22, 21, 22, 5, 5, 16, ZSTD_lazy2),    /* level 10 */
        cp(22, 21, 22, 6, 5, 16, ZSTD_lazy2),    /* level 11 */
        cp(22, 22, 23, 6, 5, 32, ZSTD_lazy2),    /* level 12 */
        cp(22, 22, 22, 4, 5, 32, ZSTD_btlazy2),  /* level 13 */
        cp(22, 22, 23, 5, 5, 32, ZSTD_btlazy2),  /* level 14 */
        cp(22, 23, 23, 6, 5, 32, ZSTD_btlazy2),  /* level 15 */
        cp(22, 22, 22, 5, 5, 48, ZSTD_btopt),    /* level 16 */
        cp(23, 23, 22, 5, 4, 64, ZSTD_btopt),    /* level 17 */
        cp(23, 23, 22, 6, 3, 64, ZSTD_btultra),  /* level 18 */
        cp(23, 24, 22, 7, 3, 256, ZSTD_btultra2),/* level 19 */
        cp(25, 25, 23, 7, 3, 256, ZSTD_btultra2),/* level 20 */
        cp(26, 26, 24, 7, 3, 512, ZSTD_btultra2),/* level 21 */
        cp(27, 27, 25, 9, 3, 999, ZSTD_btultra2),/* level 22 */
    ],
    /* for srcSize <= 256 KB */
    /* W,  C,  H,  S,  L,  T, strat */
    [
        cp(18, 12, 13, 1, 5, 1, ZSTD_fast),      /* base for negative levels */
        cp(18, 13, 14, 1, 6, 0, ZSTD_fast),      /* level  1 */
        cp(18, 14, 14, 1, 5, 0, ZSTD_dfast),     /* level  2 */
        cp(18, 16, 16, 1, 4, 0, ZSTD_dfast),     /* level  3 */
        cp(18, 16, 17, 3, 5, 2, ZSTD_greedy),    /* level  4.*/
        cp(18, 17, 18, 5, 5, 2, ZSTD_greedy),    /* level  5.*/
        cp(18, 18, 19, 3, 5, 4, ZSTD_lazy),      /* level  6.*/
        cp(18, 18, 19, 4, 4, 4, ZSTD_lazy),      /* level  7 */
        cp(18, 18, 19, 4, 4, 8, ZSTD_lazy2),     /* level  8 */
        cp(18, 18, 19, 5, 4, 8, ZSTD_lazy2),     /* level  9 */
        cp(18, 18, 19, 6, 4, 8, ZSTD_lazy2),     /* level 10 */
        cp(18, 18, 19, 5, 4, 12, ZSTD_btlazy2),  /* level 11.*/
        cp(18, 19, 19, 7, 4, 12, ZSTD_btlazy2),  /* level 12.*/
        cp(18, 18, 19, 4, 4, 16, ZSTD_btopt),    /* level 13 */
        cp(18, 18, 19, 4, 3, 32, ZSTD_btopt),    /* level 14.*/
        cp(18, 18, 19, 6, 3, 128, ZSTD_btopt),   /* level 15.*/
        cp(18, 19, 19, 6, 3, 128, ZSTD_btultra), /* level 16.*/
        cp(18, 19, 19, 8, 3, 256, ZSTD_btultra), /* level 17.*/
        cp(18, 19, 19, 6, 3, 128, ZSTD_btultra2),/* level 18.*/
        cp(18, 19, 19, 8, 3, 256, ZSTD_btultra2),/* level 19.*/
        cp(18, 19, 19, 10, 3, 512, ZSTD_btultra2),/* level 20.*/
        cp(18, 19, 19, 12, 3, 512, ZSTD_btultra2),/* level 21.*/
        cp(18, 19, 19, 13, 3, 999, ZSTD_btultra2),/* level 22.*/
    ],
    /* for srcSize <= 128 KB */
    /* W,  C,  H,  S,  L,  T, strat */
    [
        cp(17, 12, 12, 1, 5, 1, ZSTD_fast),      /* base for negative levels */
        cp(17, 12, 13, 1, 6, 0, ZSTD_fast),      /* level  1 */
        cp(17, 13, 15, 1, 5, 0, ZSTD_fast),      /* level  2 */
        cp(17, 15, 16, 2, 5, 0, ZSTD_dfast),     /* level  3 */
        cp(17, 17, 17, 2, 4, 0, ZSTD_dfast),     /* level  4 */
        cp(17, 16, 17, 3, 4, 2, ZSTD_greedy),    /* level  5 */
        cp(17, 16, 17, 3, 4, 4, ZSTD_lazy),      /* level  6 */
        cp(17, 16, 17, 3, 4, 8, ZSTD_lazy2),     /* level  7 */
        cp(17, 16, 17, 4, 4, 8, ZSTD_lazy2),     /* level  8 */
        cp(17, 16, 17, 5, 4, 8, ZSTD_lazy2),     /* level  9 */
        cp(17, 16, 17, 6, 4, 8, ZSTD_lazy2),     /* level 10 */
        cp(17, 17, 17, 5, 4, 8, ZSTD_btlazy2),   /* level 11 */
        cp(17, 18, 17, 7, 4, 12, ZSTD_btlazy2),  /* level 12 */
        cp(17, 18, 17, 3, 4, 12, ZSTD_btopt),    /* level 13.*/
        cp(17, 18, 17, 4, 3, 32, ZSTD_btopt),    /* level 14.*/
        cp(17, 18, 17, 6, 3, 256, ZSTD_btopt),   /* level 15.*/
        cp(17, 18, 17, 6, 3, 128, ZSTD_btultra), /* level 16.*/
        cp(17, 18, 17, 8, 3, 256, ZSTD_btultra), /* level 17.*/
        cp(17, 18, 17, 10, 3, 512, ZSTD_btultra),/* level 18.*/
        cp(17, 18, 17, 5, 3, 256, ZSTD_btultra2),/* level 19.*/
        cp(17, 18, 17, 7, 3, 512, ZSTD_btultra2),/* level 20.*/
        cp(17, 18, 17, 9, 3, 512, ZSTD_btultra2),/* level 21.*/
        cp(17, 18, 17, 11, 3, 999, ZSTD_btultra2),/* level 22.*/
    ],
    /* for srcSize <= 16 KB */
    /* W,  C,  H,  S,  L,  T, strat */
    [
        cp(14, 12, 13, 1, 5, 1, ZSTD_fast),      /* base for negative levels */
        cp(14, 14, 15, 1, 5, 0, ZSTD_fast),      /* level  1 */
        cp(14, 14, 15, 1, 4, 0, ZSTD_fast),      /* level  2 */
        cp(14, 14, 15, 2, 4, 0, ZSTD_dfast),     /* level  3 */
        cp(14, 14, 14, 4, 4, 2, ZSTD_greedy),    /* level  4 */
        cp(14, 14, 14, 3, 4, 4, ZSTD_lazy),      /* level  5.*/
        cp(14, 14, 14, 4, 4, 8, ZSTD_lazy2),     /* level  6 */
        cp(14, 14, 14, 6, 4, 8, ZSTD_lazy2),     /* level  7 */
        cp(14, 14, 14, 8, 4, 8, ZSTD_lazy2),     /* level  8.*/
        cp(14, 15, 14, 5, 4, 8, ZSTD_btlazy2),   /* level  9.*/
        cp(14, 15, 14, 9, 4, 8, ZSTD_btlazy2),   /* level 10.*/
        cp(14, 15, 14, 3, 4, 12, ZSTD_btopt),    /* level 11.*/
        cp(14, 15, 14, 4, 3, 24, ZSTD_btopt),    /* level 12.*/
        cp(14, 15, 14, 5, 3, 32, ZSTD_btultra),  /* level 13.*/
        cp(14, 15, 15, 6, 3, 64, ZSTD_btultra),  /* level 14.*/
        cp(14, 15, 15, 7, 3, 256, ZSTD_btultra), /* level 15.*/
        cp(14, 15, 15, 5, 3, 48, ZSTD_btultra2), /* level 16.*/
        cp(14, 15, 15, 6, 3, 128, ZSTD_btultra2),/* level 17.*/
        cp(14, 15, 15, 7, 3, 256, ZSTD_btultra2),/* level 18.*/
        cp(14, 15, 15, 8, 3, 256, ZSTD_btultra2),/* level 19.*/
        cp(14, 15, 15, 8, 3, 512, ZSTD_btultra2),/* level 20.*/
        cp(14, 15, 15, 9, 3, 512, ZSTD_btultra2),/* level 21.*/
        cp(14, 15, 15, 10, 3, 999, ZSTD_btultra2),/* level 22.*/
    ],
];

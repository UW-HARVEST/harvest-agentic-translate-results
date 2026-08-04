// Translation of c_src/src/lib.c to Rust.
//
// The original C is a library (no main, no I/O). To produce the same
// (empty) output, the Rust binary's main does nothing. The library
// functions are translated below for completeness/parity.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]

mod lib_translated {
    pub type btac1c_u16 = u16;
    pub type btac1c_s16 = i16;
    pub type btac1c_byte = u8;

    pub struct Btac1cIdxState {
        pub idx: btac1c_u16,
        pub lpred: btac1c_s16,
        pub rpred: btac1c_s16,
        pub tag: btac1c_byte,
        pub bcfcn: btac1c_byte,
        pub bsfcn: btac1c_byte,
        pub usefx: btac1c_byte,
        pub firfx: [[btac1c_s16; 8]; 4],
    }

    // C-equivalent integer arithmetic helpers — all operations are on i32
    // matching the original int width.

    fn idx_mod(i: i32, off: i32) -> usize {
        // C does (i - off) & 7, where i is int. The mask makes it nonneg.
        ((i - off) & 7) as usize
    }

    pub fn predict_sample(
        psamp: &[i32; 8],
        idx: i32,
        pfcn: i32,
        ridx: &Btac1cIdxState,
    ) -> i32 {
        let i = idx;
        let pred: i32;
        let p0: i32;
        let p1: i32;
        match pfcn {
            0 => {
                pred = psamp[idx_mod(i, 1)];
            }
            1 => {
                pred = 2 * psamp[idx_mod(i, 1)] - psamp[idx_mod(i, 2)];
            }
            2 => {
                pred = (3 * psamp[idx_mod(i, 1)] - psamp[idx_mod(i, 2)]) >> 1;
            }
            3 => {
                pred = (5 * psamp[idx_mod(i, 1)] - psamp[idx_mod(i, 2)]) >> 2;
            }
            4 => {
                p0 = psamp[idx_mod(i, 1)] + psamp[idx_mod(i, 2)];
                p1 = psamp[idx_mod(i, 2)] + psamp[idx_mod(i, 3)];
                pred = p0 - (p1 >> 1);
            }
            5 => {
                p0 = psamp[idx_mod(i, 1)] + psamp[idx_mod(i, 2)];
                p1 = psamp[idx_mod(i, 2)] + psamp[idx_mod(i, 3)];
                pred = (3 * p0 - p1) >> 2;
            }
            6 => {
                p0 = psamp[idx_mod(i, 1)] + psamp[idx_mod(i, 2)];
                p1 = psamp[idx_mod(i, 2)] + psamp[idx_mod(i, 3)];
                pred = (5 * p0 - p1) >> 3;
            }
            7 => {
                pred = (18 * psamp[idx_mod(i, 1)] - 4 * psamp[idx_mod(i, 2)]
                    + 3 * psamp[idx_mod(i, 3)]
                    - 2 * psamp[idx_mod(i, 4)]
                    + 1 * psamp[idx_mod(i, 5)])
                    / 16;
            }
            8 => {
                pred = (72 * psamp[idx_mod(i, 1)] - 16 * psamp[idx_mod(i, 2)]
                    + 12 * psamp[idx_mod(i, 3)]
                    - 8 * psamp[idx_mod(i, 4)]
                    + 5 * psamp[idx_mod(i, 5)]
                    - 3 * psamp[idx_mod(i, 6)]
                    + 3 * psamp[idx_mod(i, 7)]
                    - 1 * psamp[idx_mod(i, 8)])
                    / 64;
            }
            9 => {
                pred = (76 * psamp[idx_mod(i, 1)] - 17 * psamp[idx_mod(i, 2)]
                    + 10 * psamp[idx_mod(i, 3)]
                    - 7 * psamp[idx_mod(i, 4)]
                    + 5 * psamp[idx_mod(i, 5)]
                    - 4 * psamp[idx_mod(i, 6)]
                    + 4 * psamp[idx_mod(i, 7)]
                    - 3 * psamp[idx_mod(i, 8)])
                    / 64;
            }
            10 => {
                p0 = psamp[idx_mod(i, 1)]
                    + psamp[idx_mod(i, 2)]
                    + psamp[idx_mod(i, 3)]
                    + psamp[idx_mod(i, 4)];
                p1 = psamp[idx_mod(i, 5)]
                    + psamp[idx_mod(i, 6)]
                    + psamp[idx_mod(i, 7)]
                    + psamp[idx_mod(i, 8)];
                pred = (5 * p0 - p1) >> 4;
            }
            11 => {
                p0 = psamp[idx_mod(i, 1)]
                    + psamp[idx_mod(i, 2)]
                    + psamp[idx_mod(i, 3)]
                    + psamp[idx_mod(i, 4)];
                p1 = psamp[idx_mod(i, 5)]
                    + psamp[idx_mod(i, 6)]
                    + psamp[idx_mod(i, 7)]
                    + psamp[idx_mod(i, 8)];
                pred = (p0 + p1) >> 3;
            }
            12 | 13 | 14 | 15 => {
                let row = (pfcn - 12) as usize;
                let f = &ridx.firfx[row];
                pred = ((f[0] as i32) * psamp[idx_mod(i, 1)]
                    + (f[1] as i32) * psamp[idx_mod(i, 2)]
                    + (f[2] as i32) * psamp[idx_mod(i, 3)]
                    + (f[3] as i32) * psamp[idx_mod(i, 4)]
                    + (f[4] as i32) * psamp[idx_mod(i, 5)]
                    + (f[5] as i32) * psamp[idx_mod(i, 6)]
                    + (f[6] as i32) * psamp[idx_mod(i, 7)]
                    + (f[7] as i32) * psamp[idx_mod(i, 8)])
                    / 256;
            }
            _ => {
                pred = 0;
            }
        }
        pred
    }

    pub fn predict_sample_pfn0(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        psamp[idx_mod(idx, 1)]
    }

    pub fn predict_sample_pfn1(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        2 * psamp[idx_mod(idx, 1)] - psamp[idx_mod(idx, 2)]
    }

    pub fn predict_sample_pfn2(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        (3 * psamp[idx_mod(idx, 1)] - psamp[idx_mod(idx, 2)]) >> 1
    }

    pub fn predict_sample_pfn3(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        (5 * psamp[idx_mod(idx, 1)] - psamp[idx_mod(idx, 2)]) >> 2
    }

    pub fn predict_sample_pfn4(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        let p0 = psamp[idx_mod(idx, 1)] + psamp[idx_mod(idx, 2)];
        let p1 = psamp[idx_mod(idx, 2)] + psamp[idx_mod(idx, 3)];
        p0 - (p1 >> 1)
    }

    pub fn predict_sample_pfn5(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        let p0 = psamp[idx_mod(idx, 1)] + psamp[idx_mod(idx, 2)];
        let p1 = psamp[idx_mod(idx, 2)] + psamp[idx_mod(idx, 3)];
        (3 * p0 - p1) >> 2
    }

    pub fn predict_sample_pfn6(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        let p0 = psamp[idx_mod(idx, 1)] + psamp[idx_mod(idx, 2)];
        let p1 = psamp[idx_mod(idx, 2)] + psamp[idx_mod(idx, 3)];
        (5 * p0 - p1) >> 3
    }

    pub fn predict_sample_pfn7(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        (18 * psamp[idx_mod(idx, 1)] - 4 * psamp[idx_mod(idx, 2)]
            + 3 * psamp[idx_mod(idx, 3)]
            - 2 * psamp[idx_mod(idx, 4)]
            + 1 * psamp[idx_mod(idx, 5)])
            / 16
    }

    pub fn predict_sample_pfn8(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        (72 * psamp[idx_mod(idx, 1)] - 16 * psamp[idx_mod(idx, 2)]
            + 12 * psamp[idx_mod(idx, 3)]
            - 8 * psamp[idx_mod(idx, 4)]
            + 5 * psamp[idx_mod(idx, 5)]
            - 3 * psamp[idx_mod(idx, 6)]
            + 3 * psamp[idx_mod(idx, 7)]
            - 1 * psamp[idx_mod(idx, 8)])
            / 64
    }

    pub fn predict_sample_pfn9(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        (76 * psamp[idx_mod(idx, 1)] - 17 * psamp[idx_mod(idx, 2)]
            + 10 * psamp[idx_mod(idx, 3)]
            - 7 * psamp[idx_mod(idx, 4)]
            + 5 * psamp[idx_mod(idx, 5)]
            - 4 * psamp[idx_mod(idx, 6)]
            + 4 * psamp[idx_mod(idx, 7)]
            - 3 * psamp[idx_mod(idx, 8)])
            / 64
    }

    pub fn predict_sample_pfn10(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        let p0 = psamp[idx_mod(idx, 1)]
            + psamp[idx_mod(idx, 2)]
            + psamp[idx_mod(idx, 3)]
            + psamp[idx_mod(idx, 4)];
        let p1 = psamp[idx_mod(idx, 5)]
            + psamp[idx_mod(idx, 6)]
            + psamp[idx_mod(idx, 7)]
            + psamp[idx_mod(idx, 8)];
        (5 * p0 - p1) >> 3
    }

    pub fn predict_sample_pfn11(
        psamp: &[i32; 8],
        idx: i32,
        _pfcn: i32,
        _ridx: &Btac1cIdxState,
    ) -> i32 {
        let p0 = psamp[idx_mod(idx, 1)]
            + psamp[idx_mod(idx, 2)]
            + psamp[idx_mod(idx, 3)]
            + psamp[idx_mod(idx, 4)];
        let p1 = psamp[idx_mod(idx, 5)]
            + psamp[idx_mod(idx, 6)]
            + psamp[idx_mod(idx, 7)]
            + psamp[idx_mod(idx, 8)];
        (p0 + p1) >> 1
    }

    /// Tagged identifier for the predict function selected by pfcn — replaces
    /// the C "void* function pointer" pattern with a safe enum.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PredictFn {
        Pfn0,
        Pfn1,
        Pfn2,
        Pfn3,
        Pfn4,
        Pfn5,
        Pfn6,
        Pfn7,
        Pfn8,
        Pfn9,
        Pfn10,
        Pfn11,
        Default,
    }

    pub fn get_predict_func(pfcn: i32) -> PredictFn {
        match pfcn {
            0 => PredictFn::Pfn0,
            1 => PredictFn::Pfn1,
            2 => PredictFn::Pfn2,
            3 => PredictFn::Pfn3,
            4 => PredictFn::Pfn4,
            5 => PredictFn::Pfn5,
            6 => PredictFn::Pfn6,
            7 => PredictFn::Pfn7,
            8 => PredictFn::Pfn8,
            9 => PredictFn::Pfn9,
            10 => PredictFn::Pfn10,
            11 => PredictFn::Pfn11,
            _ => PredictFn::Default,
        }
    }

    pub fn call_predict(pfcn: i32) -> i32 {
        let fcn = get_predict_func(pfcn);
        let result = match pfcn {
            0 => fcn == PredictFn::Pfn0,
            1 => fcn == PredictFn::Pfn1,
            2 => fcn == PredictFn::Pfn2,
            3 => fcn == PredictFn::Pfn3,
            4 => fcn == PredictFn::Pfn4,
            5 => fcn == PredictFn::Pfn5,
            6 => fcn == PredictFn::Pfn6,
            7 => fcn == PredictFn::Pfn7,
            8 => fcn == PredictFn::Pfn8,
            9 => fcn == PredictFn::Pfn9,
            10 => fcn == PredictFn::Pfn10,
            11 => fcn == PredictFn::Pfn11,
            _ => false,
        };
        if result {
            1
        } else {
            0
        }
    }
}

fn main() {
    // The original C source is a library and produces no output.
    // To preserve byte-identical behavior, this binary writes nothing.
    let _ = lib_translated::call_predict(0);
}

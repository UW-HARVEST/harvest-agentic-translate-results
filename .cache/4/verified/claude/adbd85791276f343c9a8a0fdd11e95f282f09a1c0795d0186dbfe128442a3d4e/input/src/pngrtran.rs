//! Translation of `c_src/src/pngrtran.c`

use crate::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct png_dsort {
    pub next: *mut png_dsort,
    pub left: png_byte,
    pub right: png_byte,
}
pub(crate) type png_dsortp = *mut png_dsort;
pub(crate) type png_dsortpp = *mut png_dsortp;

include!("gen/pngrtran_p01.rs");
include!("gen/pngrtran_p02.rs");
include!("gen/pngrtran_p03.rs");
include!("gen/pngrtran_p04.rs");
include!("gen/pngrtran_p05.rs");
include!("gen/pngrtran_p06.rs");
include!("gen/pngrtran_p07.rs");
include!("gen/pngrtran_p08.rs");
include!("gen/pngrtran_p09.rs");
include!("gen/pngrtran_p10.rs");
include!("gen/pngrtran_p11.rs");

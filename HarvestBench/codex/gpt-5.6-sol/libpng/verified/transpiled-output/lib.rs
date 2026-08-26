#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_mut)]

#[macro_use]
extern crate c2rust_bitfields;

pub mod src {
    pub mod png;
    pub mod pngerror;
    pub mod pngget;
    pub mod pngmem;
    pub mod pngpread;
    pub mod pngread;
    pub mod pngrio;
    pub mod pngrtran;
    pub mod pngrutil;
    pub mod pngset;
    pub mod pngtrans;
    pub mod pngwio;
    pub mod pngwrite;
    pub mod pngwtran;
    pub mod pngwutil;
} // mod src

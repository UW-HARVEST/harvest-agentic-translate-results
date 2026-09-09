#![allow(non_snake_case)]

use core::arch::naked_asm;

macro_rules! trampoline {
    ($name:ident, $target:literal) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name() {
            naked_asm!(concat!("jmp ", $target));
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/exports.rs"));

#[unsafe(no_mangle)]
pub static mut g_ZSTD_threading_useless_symbol: core::ffi::c_int = 0;

#[unsafe(no_mangle)]
pub static mut g_debuglevel: core::ffi::c_int = 0;

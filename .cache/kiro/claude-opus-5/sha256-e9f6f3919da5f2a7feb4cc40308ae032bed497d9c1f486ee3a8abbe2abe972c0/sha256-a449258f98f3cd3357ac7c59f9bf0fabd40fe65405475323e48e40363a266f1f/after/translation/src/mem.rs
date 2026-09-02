//! Emulation of the C program's address space, needed because
//! `stb_perlin_noise3_wrap_nonpow2` indexes `stb__perlin_randtab`,
//! `stb__perlin_randtab_grad_idx` and (indirectly) `basis` with values derived
//! from the caller-supplied `*_wrap` arguments. Those indices are *not* clamped
//! by the C, so any `|wrap| > 512` or negative wrap makes it read outside the
//! arrays, and the value it happens to find is part of the observable output.
//!
//! Formally that is undefined behaviour, but it is perfectly deterministic for
//! the binary `CMakeLists.txt` produces: a non-PIE ELF loaded at the fixed
//! address `0x400000`. `procimage.bin` is the byte-exact image of the loaded
//! pages `[0x400000, 0x406000)`, reconstructed from the `PT_LOAD` headers of
//! `c_src/build/driver` (whole pages are mapped from the file, and the `.bss`
//! tail of the RW segment is zeroed). The relevant symbols, from
//! `nm -S c_src/build/driver`:
//!
//! ```text
//! 0x404de8  start of the RW PT_LOAD segment (.init_array/.dynamic/.got/...)
//! 0x405020  __data_start                       (32 bytes of alignment padding)
//! 0x405040  stb__perlin_randtab            512 bytes
//! 0x405240  stb__perlin_randtab_grad_idx   512 bytes
//! 0x405440  basis.0 (the static in stb__perlin_grad)  192 bytes
//! 0x405500  __bss_start .. 0x405508 _end
//! 0x406000  first unmapped page  -> SIGSEGV
//! ```
//!
//! Every page from `0x400000` up to `0x406000` is readable; an access outside
//! that range kills the process with SIGSEGV, exactly as the C program does.

/// First mapped byte of the process image.
const WIN_LO: i64 = 0x40_0000;
/// First unmapped byte after the RW segment's last page.
const WIN_HI: i64 = 0x40_6000;

pub const RANDTAB_ADDR: i64 = 0x40_5040;
pub const GRAD_IDX_ADDR: i64 = 0x40_5240;
pub const BASIS_ADDR: i64 = 0x40_5440;

static IMAGE: &[u8; (WIN_HI - WIN_LO) as usize] = include_bytes!("procimage.bin");

/// Reads one byte of the emulated image, or dies with SIGSEGV like the C does.
#[inline]
pub fn read_u8(addr: i64) -> u8 {
    if addr < WIN_LO || addr >= WIN_HI {
        segv();
    }
    IMAGE[(addr - WIN_LO) as usize]
}

/// Reads a little-endian `float` from the emulated image.
#[inline]
pub fn read_f32(addr: i64) -> f32 {
    let b = [
        read_u8(addr),
        read_u8(addr + 1),
        read_u8(addr + 2),
        read_u8(addr + 3),
    ];
    f32::from_bits(u32::from_le_bytes(b))
}

/// Faults the same way the C process does when it dereferences an unmapped
/// address: the kernel delivers SIGSEGV and nothing has been printed yet.
#[cold]
#[inline(never)]
fn segv() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mov {t}, [{p}]", p = in(reg) 0usize, t = out(reg) _);
    }
    // Not reached: the load above always faults.
    std::process::exit(139);
}

/// C's `%` on `int`, including the `idivl` hardware trap (SIGFPE) that
/// `INT_MIN % -1` raises on x86-64.
#[inline]
pub fn crem_i32(a: i32, b: i32) -> i32 {
    if b == 0 || (b == -1 && a == i32::MIN) {
        sigfpe();
    }
    a.wrapping_rem(b)
}

#[cold]
#[inline(never)]
fn sigfpe() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut a: i32 = i32::MIN;
        core::arch::asm!(
            "cdq",
            "idiv {b:e}",
            inout("eax") a,
            b = in(reg) -1i32,
            out("edx") _,
        );
        core::hint::black_box(a);
    }
    // Not reached: the division above always traps.
    std::process::exit(136);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::{RANDTAB, RANDTAB_GRAD_IDX};

    /// Guards `procimage.bin` against drifting away from the tables that were
    /// transcribed by hand from `c_src/src/stb_perlin.h`.
    #[test]
    fn image_agrees_with_the_transcribed_tables() {
        for (i, expected) in RANDTAB.iter().enumerate() {
            assert_eq!(
                read_u8(RANDTAB_ADDR + i as i64),
                *expected,
                "stb__perlin_randtab[{i}]"
            );
        }
        for (i, expected) in RANDTAB_GRAD_IDX.iter().enumerate() {
            assert_eq!(
                read_u8(GRAD_IDX_ADDR + i as i64),
                *expected,
                "stb__perlin_randtab_grad_idx[{i}]"
            );
        }
    }

    /// `static float basis[12][4]` from `stb__perlin_grad`.
    #[test]
    fn image_agrees_with_the_basis_table() {
        #[rustfmt::skip]
        const BASIS: [[f32; 4]; 12] = [
            [ 1.0,  1.0,  0.0, 0.0],
            [-1.0,  1.0,  0.0, 0.0],
            [ 1.0, -1.0,  0.0, 0.0],
            [-1.0, -1.0,  0.0, 0.0],
            [ 1.0,  0.0,  1.0, 0.0],
            [-1.0,  0.0,  1.0, 0.0],
            [ 1.0,  0.0, -1.0, 0.0],
            [-1.0,  0.0, -1.0, 0.0],
            [ 0.0,  1.0,  1.0, 0.0],
            [ 0.0, -1.0,  1.0, 0.0],
            [ 0.0,  1.0, -1.0, 0.0],
            [ 0.0, -1.0, -1.0, 0.0],
        ];
        for (row, cols) in BASIS.iter().enumerate() {
            for (col, expected) in cols.iter().enumerate() {
                let addr = BASIS_ADDR + (row * 16 + col * 4) as i64;
                assert_eq!(read_f32(addr).to_bits(), expected.to_bits(), "basis[{row}][{col}]");
            }
        }
    }
}
